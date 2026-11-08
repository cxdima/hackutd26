//! Routes mounted only inside the existing operator-authenticated API router.
use crate::{
    calls::{actor::App, state::Session},
    events::Event,
    reputation::ReputationService,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportRequest {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionRequest {
    reason: CorrectionReason,
    expected_version: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionReason {
    Mistaken,
    Spoofed,
    LegitimateBusiness,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionRequest {
    outcome: Resolution,
    expected_version: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Upheld,
    Removed,
}
fn reply(status: StatusCode, value: Value) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], Json(value)).into_response()
}
fn failure(error: anyhow::Error) -> Response {
    reply(StatusCode::CONFLICT, json!({"error":error.to_string()}))
}
fn service(app: &App) -> Result<Arc<ReputationService>, &'static str> {
    app.reputation
        .clone()
        .ok_or("Number reputation is not configured")
}
async fn load_session(app: &App, id: &str) -> anyhow::Result<Session> {
    anyhow::ensure!(id.len() <= 128, "Invalid session ID");
    let row: Option<String> = sqlx::query_scalar("SELECT data FROM sessions WHERE id=?")
        .bind(id)
        .fetch_optional(&app.store.pool)
        .await?;
    let s: Session =
        serde_json::from_str(&row.ok_or_else(|| anyhow::anyhow!("Session not found"))?)?;
    anyhow::ensure!(
        s.execution_mode == app.config.mode,
        "Session belongs to another execution mode"
    );
    Ok(s)
}
pub async fn list(State(app): State<App>, Query(q): Query<ListQuery>) -> Response {
    let service = match service(&app) {
        Ok(s) => s,
        Err(error) => return reply(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    let limit = q.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return reply(
            StatusCode::BAD_REQUEST,
            json!({"error":"limit must be between 1 and 100"}),
        );
    }
    match service.list(app.config.mode, limit).await {
        Ok(items) => reply(
            StatusCode::OK,
            json!({"scope":service.scope(),"execution_mode":app.config.mode,"items":items}),
        ),
        Err(e) => failure(e),
    }
}
pub async fn callers(State(app): State<App>) -> Response {
    let service = match service(&app) {
        Ok(s) => s,
        Err(error) => return reply(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    match service.callers(app.config.mode).await {
        Ok(items) => reply(
            StatusCode::OK,
            json!({"items":items,"scope":service.scope(),"execution_mode":app.config.mode,"meaning":"Presented caller ID can be spoofed; a number key groups calls, it does not identify a person."}),
        ),
        Err(e) => failure(e),
    }
}
pub async fn detail(State(app): State<App>, Path(key): Path<String>) -> Response {
    let service = match service(&app) {
        Ok(s) => s,
        Err(error) => return reply(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    match service.detail(app.config.mode, &key).await {
        Ok(Some(item)) => {
            let mut view = item["reputation"].clone();
            view["audit"] = item["audit"].clone();
            view["reports"] = item["reports"].clone();
            reply(StatusCode::OK, view)
        }
        Ok(None) => reply(
            StatusCode::NOT_FOUND,
            json!({"error":"Number reputation not found"}),
        ),
        Err(e) => failure(e),
    }
}
pub async fn session_view(State(app): State<App>, Path(id): Path<String>) -> Response {
    let service = match service(&app) {
        Ok(s) => s,
        Err(error) => return reply(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    let s = match load_session(&app, &id).await {
        Ok(s) => s,
        Err(_) => return reply(StatusCode::NOT_FOUND, json!({"error":"Session not found"})),
    };
    match service.session_view(&s).await {
        Ok(view) => reply(StatusCode::OK, view),
        Err(e) => failure(e),
    }
}
pub async fn report(State(app): State<App>, Json(request): Json<ReportRequest>) -> Response {
    let service = match service(&app) {
        Ok(s) => s,
        Err(error) => return reply(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    let result=async{
 let s=load_session(&app,&request.session_id).await?;
 let row:Option<String>=sqlx::query_scalar("SELECT data FROM events WHERE json_extract(data,'$.session_id')=? AND json_extract(data,'$.type')='message.recorded' ORDER BY sequence DESC LIMIT 1").bind(&s.id).fetch_optional(&app.store.pool).await?;
 let event:Event=serde_json::from_str(&row.ok_or_else(||anyhow::anyhow!("No durable saved protective message"))?)?;
 service.report_protective(&s,&event).await
 }.await;
    match result {
        Ok(Some(r)) => reply(
            if r.created {
                StatusCode::ACCEPTED
            } else {
                StatusCode::CONFLICT
            },
            json!({"report_id":r.report_id,"created":r.created,"level":r.reputation.level,"version":r.reputation.version,"scope":r.reputation.scope}),
        ),
        Ok(None) => reply(
            StatusCode::CONFLICT,
            json!({"error":"No presented number was captured for this session"}),
        ),
        Err(e) => failure(e),
    }
}
pub async fn correct(
    State(app): State<App>,
    Path(key): Path<String>,
    Json(request): Json<CorrectionRequest>,
) -> Response {
    let service = match service(&app) {
        Ok(s) => s,
        Err(error) => return reply(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    let reason = match request.reason {
        CorrectionReason::Mistaken => "mistaken",
        CorrectionReason::Spoofed => "spoofed",
        CorrectionReason::LegitimateBusiness => "legitimate_business",
    };
    match service
        .request_correction(app.config.mode, &key, reason, request.expected_version)
        .await
    {
        Ok(s) => reply(
            StatusCode::ACCEPTED,
            json!({"correction":s.correction,"version":s.version,"level":s.level}),
        ),
        Err(e) => failure(e),
    }
}
pub async fn resolve(
    State(app): State<App>,
    Path(key): Path<String>,
    Json(request): Json<ResolutionRequest>,
) -> Response {
    let service = match service(&app) {
        Ok(s) => s,
        Err(error) => return reply(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    let outcome = match request.outcome {
        Resolution::Upheld => "upheld",
        Resolution::Removed => "removed",
    };
    match service
        .resolve_correction(app.config.mode, &key, outcome, request.expected_version)
        .await
    {
        Ok(s) => reply(
            StatusCode::ACCEPTED,
            json!({"correction":s.correction,"version":s.version,"level":s.level}),
        ),
        Err(e) => failure(e),
    }
}
