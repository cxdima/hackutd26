use crate::{
    calls::{
        actor::{App, Input},
        state::Session,
    },
    providers::{relay, signature, twilio},
};
use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    extract::{
        Path, Query, Request, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    convert::Infallible,
    time::Duration,
};
use subtle::ConstantTimeEq;
#[derive(Clone)]
struct FormData(HashMap<String, String>);
impl FormData {
    fn field(&self, key: &str) -> &str {
        self.0.get(key).map_or("", String::as_str)
    }
}
type ApiResult = Result<Response, ApiError>;
pub struct ApiError(anyhow::Error);
impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::CONFLICT,
            Json(json!({"error":self.0.to_string()})),
        )
            .into_response()
    }
}
pub fn router(app: App) -> Router {
    let voice = Router::new()
        .route("/voice/incoming", post(incoming))
        .route("/voice/probe-choice", post(probe_choice))
        .route("/voice/alert/{id}", post(alert))
        .route("/voice/ack/{id}", post(ack))
        .route("/voice/status", post(status))
        .route("/voice/relay", get(relay_ws))
        .route("/voice/relay/", get(relay_ws))
        .route("/voice/after-relay", post(after_relay))
        .route("/voice/closing", post(closing_input))
        .route("/voice/turn", post(turn))
        .route("/voice/whisper/{id}", post(whisper))
        .route("/voice/choice/{id}", post(choice))
        .route("/voice/conference-status", post(conference_status))
        .route("/voice/recording-status", post(recording_status))
        .route("/voice/clone-recorded", post(clone_recorded))
        .route("/voice/clone-recording", post(clone_recording))
        .route("/voice/fixture/{action}/{step}", post(fixture_turn))
        .route(
            "/voice/fixture-partial/{action}/{step}",
            post(fixture_partial),
        )
        .route(
            "/voice/fixture-recording/{action}",
            post(fixture_recording_receipt),
        )
        .route_layer(middleware::from_fn_with_state(app.clone(), signed));
    let api = Router::new()
        .route("/api/state", get(state))
        .route("/api/session/{id}/recording", get(recording_audio))
        .route("/api/session/{id}/recording/info", get(recording_info))
        .route("/api/voice/clone", get(clone_status))
        .route("/api/voice/clone/arm", post(clone_arm))
        .route("/api/voice/clone/activate", post(clone_activate))
        .route("/api/voice/clone/deactivate", post(clone_deactivate))
        .route("/api/voice/clone/delete", post(clone_delete))
        .route("/api/voice/clone/preview", get(clone_preview))
        .route("/api/reputation", get(crate::reputation_http::list))
        .route(
            "/api/reputation/reports",
            post(crate::reputation_http::report),
        )
        .route(
            "/api/reputation/{number_key}",
            get(crate::reputation_http::detail),
        )
        .route(
            "/api/reputation/{number_key}/corrections",
            post(crate::reputation_http::correct),
        )
        .route(
            "/api/reputation/{number_key}/corrections/resolve",
            post(crate::reputation_http::resolve),
        )
        .route(
            "/api/session/{id}/reputation",
            get(crate::reputation_http::session_view),
        )
        .route("/api/session/{id}", get(session))
        .route(
            "/api/session/{id}/events",
            get(crate::console_http::session_events),
        )
        .route(
            "/api/profile/{household_id}",
            get(profile_get).put(profile_put),
        )
        .route("/api/events", get(events))
        .route("/api/demo/phone-demo", get(phone_demo_status))
        .route("/api/demo/phone-demo/arm", post(phone_demo_arm))
        .route("/api/demo/phone-demo/disarm", post(phone_demo_disarm))
        .route("/api/demo/probe", post(probe))
        .route("/api/demo/replay", post(replay))
        .route("/api/demo/household", post(household))
        .route("/api/demo/end-call", post(end_call))
        .route("/api/demo/reset", post(reset))
        .route(
            "/api/demo/audio-fixture",
            get(fixture_state).post(fixture_start),
        )
        .route("/api/demo/audio-fixture/recording", get(fixture_recording))
        .route("/api/demo/audio-fixture/stop", post(fixture_stop))
        .route_layer(middleware::from_fn_with_state(app.clone(), admin));
    Router::new()
        .route("/health/live", get(|| async { Json(json!({"ok":true})) }))
        .route("/health/ready", get(ready))
        .route("/media/fixture/{capability}/{clip}", get(fixture_audio))
        .route("/media/speech/{capability}", get(speech_audio))
        .route("/media/voice/{capability}", get(voice_media))
        .merge(voice)
        .merge(api)
        .fallback_service(crate::console_http::frontend_service())
        .with_state(app)
}
/// The console opened on this computer (a loopback peer with no proxy or tunnel
/// forwarding headers) is the operator; no token to paste. Requests arriving
/// through the public tunnel always carry Cloudflare forwarding headers and
/// still need the bearer token. `CONSOLE_LOCAL_TRUST=off` disables this.
pub fn local_console(req: &Request) -> bool {
    if std::env::var("CONSOLE_LOCAL_TRUST").as_deref() == Ok("off") {
        return false;
    }
    let loopback = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .is_some_and(|info| info.0.ip().is_loopback());
    let forwarded = [
        "cf-connecting-ip",
        "cf-ray",
        "cdn-loop",
        "x-forwarded-for",
        "x-forwarded-host",
        "forwarded",
        "x-real-ip",
    ]
    .iter()
    .any(|name| req.headers().contains_key(*name));
    loopback && !forwarded
}
async fn admin(State(app): State<App>, req: Request, next: Next) -> Response {
    if local_console(&req) {
        return next.run(req).await;
    }
    let provided = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if app.config.admin_token.is_empty()
        || !bool::from(provided.as_bytes().ct_eq(app.config.admin_token.as_bytes()))
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"Operator authentication required"})),
        )
            .into_response();
    }
    next.run(req).await
}
async fn phone_demo_status(State(app): State<App>) -> ApiResult {
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(crate::phone_demo::status(&app.store, &app.config).await?),
    )
        .into_response())
}
async fn phone_demo_arm(
    State(app): State<App>,
    Json(request): Json<crate::phone_demo::ArmRequest>,
) -> ApiResult {
    let status = crate::phone_demo::arm(&app.store, &app.config, request).await?;
    let _=app.store.append(None,"system","phone_demo.armed",json!({"arm_id":status.arm_id,"session_id":status.session_id,"expires_at":status.expires_at,"scope":status.scope}),app.config.mode).await;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(status)).into_response())
}
async fn phone_demo_disarm(
    State(app): State<App>,
    Json(request): Json<crate::phone_demo::DisarmRequest>,
) -> ApiResult {
    let status = crate::phone_demo::disarm(&app.store, &app.config, &request.arm_id).await?;
    let _ = app
        .store
        .append(
            None,
            "system",
            "phone_demo.disarmed",
            json!({"arm_id":status.arm_id,"scope":status.scope}),
            app.config.mode,
        )
        .await;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(status)).into_response())
}
async fn profile_get(State(app): State<App>, Path(household_id): Path<String>) -> ApiResult {
    let profile = crate::profile::Profile::load(&app.store, &household_id).await?;
    Ok(profile_response(profile))
}
async fn profile_put(
    State(app): State<App>,
    Path(household_id): Path<String>,
    Json(update): Json<crate::profile::ProfileUpdate>,
) -> ApiResult {
    let profile = crate::profile::Profile::save(&app.store, &household_id, update).await?;
    Ok(profile_response(profile))
}
fn profile_response(profile: crate::profile::Profile) -> Response {
    let mut value = serde_json::to_value(profile).expect("Profile consists of JSON scalar fields");
    value["source"] = json!("configured");
    value["applies_to"] = json!("new_calls");
    Json(value).into_response()
}
async fn signed(State(app): State<App>, req: Request, next: Next) -> Response {
    let sig = req
        .headers()
        .get("x-twilio-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if sig.is_empty() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let path = req
        .uri()
        .path_and_query()
        .map_or("/", |p| p.as_str())
        .to_string();
    let (parts, body) = req.into_parts();
    let Ok(bytes) = to_bytes(body, 65536).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let form: Vec<(String, String)> = url::form_urlencoded::parse(&bytes).into_owned().collect();
    let url = app.config.external_url(&path);
    let websocket = parts.method == axum::http::Method::GET
        && (parts.uri.path() == "/voice/relay" || parts.uri.path() == "/voice/relay/");
    // Both signature schemes are restricted to the configured authority and exact
    // request path/query; no Host/proxy headers or unsigned fallback are accepted.
    let valid = signature::validate(&app.config.auth_token, &url, &form, &sig)
        || (websocket
            && signature::validate(
                &app.config.auth_token,
                &url.replacen("https://", "wss://", 1),
                &form,
                &sig,
            ));
    if !valid {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut seen = HashSet::new();
    if form.iter().any(|(k, _)| !seen.insert(k)) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let fields: HashMap<_, _> = form.into_iter().collect();
    if !websocket && fields.get("AccountSid") != Some(&app.config.account_sid) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !websocket
        && parts.uri.path() != "/voice/conference-status"
        && !fields
            .get("CallSid")
            .is_some_and(|s| twilio::valid_call_sid(s))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let mut req = Request::from_parts(parts, Body::from(bytes));
    req.extensions_mut().insert(FormData(fields));
    next.run(req).await
}
fn xml(text: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/xml; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        text,
    )
        .into_response()
}
async fn incoming(State(app): State<App>, Extension(form): Extension<FormData>) -> ApiResult {
    if let Some(twiml) = crate::voice_clone::intercept(&app, form.field("CallSid")).await? {
        return Ok(xml(twiml));
    }
    let binding =
        crate::audio_fixture::bind_inbound(&app, form.field("CallSid"), form.field("From")).await?;
    if binding == 2 {
        let cleanup = app.clone();
        tokio::spawn(async move {
            let _ = crate::audio_fixture::stop(&cleanup, None).await;
        });
        return Ok(xml(twilio::say_end(
            "The automated test detected an unexpected second connection. Goodbye.",
        )));
    }
    let (_, snapshot) = app.incoming(form.field("CallSid")).await?;
    let session: Session = serde_json::from_value(snapshot.clone())?;
    if let Some(reputation) = &app.reputation
        && let Err(error) = reputation
            .capture_identity(&session, Some(form.field("From")))
            .await
    {
        app.store.append(Some(&session), "system", "dependency.failed", json!({"dependency":"reputation","operation":"capture_identity","detail":error.to_string()}), session.execution_mode).await?;
    }
    crate::phone_demo::bind_signed_inbound(&app.store, &app.config, &session).await?;
    crate::call_recording::start(
        app.clone(),
        session.id.clone(),
        form.field("CallSid").into(),
    );
    let _ = crate::voice_clone::refresh(&app).await;
    if binding == 1 {
        crate::audio_fixture::watch_inbound(app.clone(), form.field("CallSid").into());
    }
    if snapshot["inbound_ended"] == true {
        return Ok(xml(twilio::say_end(
            "This test session has ended. Please call again.",
        )));
    }
    if app.config.transport == "relay" {
        let mut profile: crate::profile::Profile =
            serde_json::from_value(snapshot["profile"].clone())?;
        if crate::call_recording::enabled() {
            profile.greeting = format!("{} This call is recorded.", profile.greeting);
        }
        // An activated owner voice speaks the greeting itself once Relay connects.
        let greet = crate::voice_clone::active_voice(&session.household_id).is_none();
        return Ok(xml(crate::providers::relay::connect_twiml_for(
            &app.config.public_base_url,
            snapshot["relay_token"].as_str().unwrap_or_default(),
            &profile,
            greet,
        )));
    }
    if app.config.transport == "gather" {
        return Ok(xml(twilio::speech_gather(
            &app.config.external_url(&format!(
                "/voice/turn?turn_id={}",
                snapshot["relay_token"].as_str().unwrap_or_default()
            )),
            snapshot["profile"]["greeting"]
                .as_str()
                .unwrap_or("Hi, thanks for calling. What’s the call about?"),
        )));
    }
    Ok(xml(twilio::gather(
        &app.config.external_url("/voice/probe-choice"),
        "I'm Sarah's automated assistant. This is a fictional FirstRing transport test. Press 1 to confirm you can hear me.",
        8,
    )))
}
async fn probe_choice(State(app): State<App>, Extension(f): Extension<FormData>) -> ApiResult {
    let h = app.by_sid(f.field("CallSid")).await?;
    let v = h
        .send(Input::Keypad {
            sid: f.field("CallSid").into(),
            digits: f.field("Digits").into(),
        })
        .await?;
    Ok(xml(v["twiml"].as_str().unwrap_or_default().into()))
}
async fn alert(
    State(app): State<App>,
    Path(id): Path<String>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let h = app.by_action(&id).await?;
    let v = h
        .send(Input::Alert {
            action_id: id,
            sid: f.field("CallSid").into(),
        })
        .await?;
    Ok(xml(v["twiml"].as_str().unwrap_or_default().into()))
}
async fn ack(
    State(app): State<App>,
    Path(id): Path<String>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let h = app.by_action(&id).await?;
    let v = h
        .send(Input::Ack {
            action_id: id,
            sid: f.field("CallSid").into(),
            digits: f.field("Digits").into(),
        })
        .await?;
    Ok(xml(v["twiml"].as_str().unwrap_or_default().into()))
}
async fn status(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    if let Some(action) = q.get("action_id") {
        if crate::audio_fixture::status(&app, action, f.field("CallSid"), f.field("CallStatus"))
            .await?
        {
            return Ok(StatusCode::NO_CONTENT.into_response());
        }
        app.by_action(action)
            .await?
            .send(Input::Status {
                action_id: action.clone(),
                sid: f.field("CallSid").into(),
                status: f.field("CallStatus").into(),
            })
            .await?;
    } else if crate::calls::state::terminal(f.field("CallStatus")) {
        if crate::audio_fixture::inbound_status(&app, f.field("CallSid"), f.field("CallStatus"))
            .await
            .is_err()
        {
            tracing::warn!("Fixture terminal receipt persistence failed; batch remains blocked");
        }
        app.by_sid(f.field("CallSid"))
            .await?
            .send(Input::InboundEnded)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureRequest {
    scenario_id: String,
    #[serde(default)]
    request_id: Option<String>,
}
async fn fixture_start(State(app): State<App>, Json(request): Json<FixtureRequest>) -> ApiResult {
    Ok(Json(
        crate::audio_fixture::start_request(
            &app,
            &request.scenario_id,
            request.request_id.as_deref(),
        )
        .await?,
    )
    .into_response())
}
async fn fixture_state(State(app): State<App>) -> ApiResult {
    Ok(Json(crate::audio_fixture::view(&app).await?).into_response())
}
async fn fixture_turn(
    State(app): State<App>,
    Path((action, step)): Path<(String, u8)>,
    Extension(form): Extension<FormData>,
) -> ApiResult {
    Ok(xml(crate::audio_fixture::respond(
        &app,
        &action,
        step,
        form.field("CallSid"),
        form.field("SpeechResult"),
        form.field("Confidence"),
    )
    .await?))
}
async fn fixture_audio(
    State(app): State<App>,
    Path((capability, clip)): Path<(String, usize)>,
) -> Response {
    match crate::audio_fixture::audio(&app, &capability, clip).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "audio/mpeg"),
                (header::CACHE_CONTROL, "private, no-store"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn speech_audio(State(app): State<App>, Path(capability): Path<String>) -> Response {
    if let Some(cache) = &app.speech
        && let Ok(Some(media)) = cache.media_response(&capability).await
    {
        return (
            [
                (header::CONTENT_TYPE, media.mime_type),
                (header::CACHE_CONTROL, "private, no-store"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            media.bytes,
        )
            .into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}
async fn fixture_partial(
    State(app): State<App>,
    Path((action, step)): Path<(String, u8)>,
    Extension(form): Extension<FormData>,
) -> ApiResult {
    crate::audio_fixture::partial(
        &app,
        &action,
        form.field("CallSid"),
        step,
        form.field("UnstableSpeechResult"),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn fixture_recording_receipt(
    State(app): State<App>,
    Path(action): Path<String>,
    Extension(form): Extension<FormData>,
) -> ApiResult {
    crate::audio_fixture::recording_receipt(
        &app,
        &action,
        form.field("CallSid"),
        form.field("RecordingSid"),
        form.field("RecordingStatus"),
        form.field("RecordingDuration"),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn fixture_recording(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult {
    Ok((
        [
            (header::CONTENT_TYPE, "audio/mpeg"),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
        crate::audio_fixture::recording(&app, q.get("action_id").map(String::as_str)).await?,
    )
        .into_response())
}
async fn fixture_stop(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult {
    Ok(
        Json(crate::audio_fixture::stop(&app, q.get("action_id").map(String::as_str)).await?)
            .into_response(),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    session_id: String,
    contact_id: String,
}
async fn probe(State(app): State<App>, Json(p): Json<Probe>) -> ApiResult {
    Ok(Json(
        app.handle(&p.session_id)
            .await?
            .send(Input::Probe {
                contact_id: p.contact_id,
            })
            .await?,
    )
    .into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct End {
    session_id: String,
}
async fn end_call(State(app): State<App>, Json(p): Json<End>) -> ApiResult {
    Ok(Json(app.handle(&p.session_id).await?.send(Input::End).await?).into_response())
}
async fn reset(State(app): State<App>) -> ApiResult {
    let handles: Vec<_> = app.sessions.lock().await.values().cloned().collect();
    for h in handles {
        h.send(Input::End).await?;
    }
    Ok(Json(json!({"end_requested":true,"audit_preserved":true})).into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Household {
    household_id: String,
}
async fn household(State(app): State<App>, Json(p): Json<Household>) -> ApiResult {
    if !["sarah", "maya"].contains(&p.household_id.as_str()) {
        return Err(anyhow::anyhow!("Unknown household").into());
    }
    app.store
        .set_metadata("selected_household", &p.household_id)
        .await?;
    Ok(Json(json!({"selected_household":p.household_id})).into_response())
}
async fn session(State(app): State<App>, Path(id): Path<String>) -> ApiResult {
    let s: Session = serde_json::from_value(app.handle(&id).await?.send(Input::Snapshot).await?)?;
    Ok(Json(s.public_value()).into_response())
}
async fn state(State(app): State<App>) -> ApiResult {
    let snapshots = app
        .store
        .sessions()
        .await?
        .iter()
        .map(Session::public_value)
        .collect::<Vec<_>>();
    let c = &app.config;
    let (community, tactics) = crate::patterns::view(&app.store, c.mode).await?;
    let evidence = crate::evidence::view(&app.store).await?;
    let mut model_backend = crate::evidence::model_backend(c, &app.store).await?;
    model_backend["current"] = app.model_availability.view(c, &app.store).await;
    let phone_b = crate::evidence::phone_b(c, &app.store).await?;
    let phone_demo = crate::phone_demo::status(&app.store, c).await?;
    let selected_household = app
        .store
        .metadata("selected_household")
        .await?
        .unwrap_or_else(|| "sarah".into());
    let profile = crate::profile::Profile::load(&app.store, &selected_household).await?;
    let capabilities = json!([
    {"id":"inbound","status":if c.inbound_configured(){"configured"}else{"pending"},"detail":"Local signing and HTTPS inputs; recorded phone proof is shown separately"},
    {"id":"outbound","status":if c.outbound_configured(){"configured"}else{"pending"},"detail":"Separate Photon SIP credentials and caller number; endpoint enrollment, authorization and real outbound test remain separate gates"},
    {"id":"nemotron","status":if c.model_configured(){"configured"}else{"pending"},"detail":"Selected Nemotron backend inputs; protocol evidence and behavior evaluation are separate"},
    {"id":"relay","status":if c.relay_configured(){"configured"}else{"pending"},"detail":"Local signed-WebSocket inputs only; onboarding and real phone protocol test pending"},
    {"id":"smoke_test","status":if c.smoke_authorized{"configured"}else{"pending"},"detail":"Exactly one authorized outbound attempt to Sarah/Phone B"},
    {"id":"private_handoff_demo","status":if phone_demo.state=="armed" && phone_demo.recipient_enrolled && phone_demo.outbound_configured && c.mode==crate::config::ExecutionMode::Live {"configured"}else{"disabled"},"detail":"One operator arm authorizes only its bound session; independent of the Phone B transport probe"},
    {"id":"handoff","status":if phone_demo.state=="armed" && phone_demo.recipient_enrolled && phone_demo.outbound_configured {"configured"}else{"disabled"},"detail":if !phone_demo.recipient_enrolled {"Physical recipient not enrolled; no private call can start"} else if phone_demo.state=="armed" {"One private phone demo is armed; connection and audible summary remain unverified"} else {"Private calling is not armed; an explicit operator action is required"}},
    {"id":"family","status":if c.family_enabled{"configured"}else{"disabled"},"detail":"Family calls require separate enrollment and authorization"},
    {"id":"reputation","status":if app.reputation.is_some(){"configured"}else{"disabled"},"detail":"Scoped presented-number reports; demo profiles simulate independence, never verify caller identity"},
    {"id":"voice_clone","status":"disabled","detail":"Preset assistant voice; no clone endpoint verified"}
    ]);
    Ok(Json(json!({"execution_mode":c.mode,"data_origin":"synthetic","transport":c.transport,"phone_display":if c.photon_number.is_empty(){"Not configured"}else{&c.photon_number},"selected_household":app.store.metadata("selected_household").await?.unwrap_or_else(||"sarah".into()),"capabilities":capabilities,"evidence":evidence,"model_backend":model_backend,"phone_b":phone_b,"phone_demo":phone_demo,"profile":profile,"speech":relay::speech_configuration_for(&c.transport, &profile),"sessions":snapshots,"community":community,"tactics":tactics})).into_response())
}
async fn ready(State(app): State<App>) -> Response {
    let c = &app.config;
    let ready = c.selected_transport_configured() && !c.admin_token.is_empty();
    let evidence = crate::evidence::view(&app.store).await.unwrap_or_default();
    let inbound = evidence["inbound_phone"]["status"] == "passed";
    let relay = evidence["relay_phone"]["status"] == "passed";
    let mut model_backend = crate::evidence::model_backend(c, &app.store)
        .await
        .unwrap_or_default();
    model_backend["current"] = app.model_availability.view(c, &app.store).await;
    let model = model_backend["current"]["available"] == true;
    let selected = ready
        && c.mode == crate::config::ExecutionMode::Live
        && match c.transport.as_str() {
            "probe" => inbound,
            "relay" => relay && model,
            _ => false,
        };
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(json!({"configured":ready,"live_verified":selected,"execution_mode":c.mode,"transport":c.transport,"evidence":evidence,"model_backend":model_backend,
            "capabilities":{
                "inbound":{"configured":c.inbound_configured(),"live_verified":inbound},
                "outbound":{"configured":c.outbound_configured(),"live_verified":false},
                "relay":{"configured":c.relay_configured(),"live_verified":relay},
                "model":{"configured":c.model_configured(),"live_verified":model}
            },"meaning":"Configuration is current; verification fields refer to separately recorded historical evidence, not an availability guarantee"})),
    )
        .into_response()
}
async fn events(
    State(app): State<App>,
    headers: axum::http::HeaderMap,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let cursor = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let rx = app.store.bus.subscribe();
    let stream = futures_util::stream::unfold(
        (app.store, cursor, VecDeque::new(), rx),
        |(store, mut cursor, mut queue, mut rx)| async move {
            loop {
                if let Some(e) = queue.pop_front() {
                    let e: crate::events::Event = e;
                    cursor = e.sequence;
                    let event = Event::default()
                        .id(e.sequence.to_string())
                        .json_data(&e)
                        .ok()?;
                    return Some((Ok(event), (store, cursor, queue, rx)));
                }
                match store.events_after(cursor).await {
                    Ok(events) if !events.is_empty() => {
                        queue.extend(events);
                        continue;
                    }
                    Ok(_) => {}
                    Err(_) => return None,
                }
                let _ = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await;
            }
        },
    );
    Sse::new(stream).keep_alive(KeepAlive::default().interval(Duration::from_secs(10)))
}
async fn whisper(
    State(app): State<App>,
    Path(id): Path<String>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let v = app
        .by_offer(&id)
        .await?
        .send(Input::Whisper {
            offer_id: id,
            sid: f.field("CallSid").into(),
        })
        .await?;
    Ok(xml(v["twiml"].as_str().unwrap_or_default().into()))
}
async fn choice(
    State(app): State<App>,
    Path(id): Path<String>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let v = app
        .by_offer(&id)
        .await?
        .send(Input::Choice {
            offer_id: id,
            sid: f.field("CallSid").into(),
            digits: f.field("Digits").into(),
        })
        .await?;
    Ok(xml(v["twiml"].as_str().unwrap_or_default().into()))
}
async fn conference_status(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let id = q
        .get("session_id")
        .ok_or_else(|| anyhow::anyhow!("Missing session"))?;
    app.handle(id)
        .await?
        .send(Input::Conference {
            name: f.field("FriendlyName").into(),
            sid: f.field("CallSid").into(),
            event: f.field("StatusCallbackEvent").into(),
        })
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn after_relay(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let h = app.by_sid(f.field("CallSid")).await?;
    let s = h.send(Input::Snapshot).await?;
    let token = q
        .get("relay_token")
        .ok_or_else(|| anyhow::anyhow!("Missing Relay callback token"))?
        .clone();
    if s["relay_token"].as_str() != Some(token.as_str()) {
        return Err(anyhow::anyhow!("Obsolete Relay callback").into());
    }
    let session: Session = serde_json::from_value(s.clone())?;
    let code = f.field("ErrorCode").parse::<u32>().ok();
    let status = match f.field("SessionStatus") {
        "failed" => "failed",
        "completed" => "completed",
        "ended" => "ended",
        _ => "unknown",
    };
    app.store.append(Some(&session), "system", "relay.ended", json!({"error_code":code,"status":status,"duration_seconds":f.field("SessionDuration").parse::<u64>().ok()}),session.execution_mode).await?;
    if matches!(
        s["closing_reason"].as_str(),
        Some("message_saved" | "model_failure" | "no_message")
    ) {
        let closing = h.send(Input::ClosingResponseForRelay { token }).await?;
        return Ok(xml(closing["twiml"]
            .as_str()
            .unwrap_or("<Response><Hangup/></Response>")
            .into()));
    }
    if s["bridge_committed"] == true {
        return Ok(xml(
            "<Response><Pause length=\"10\"/><Hangup/></Response>".into()
        ));
    }
    h.send(Input::RelayDisconnected { token }).await?;
    Ok(xml(twilio::say_end(
        "The assistant connection ended. No private choice will connect this call. Goodbye.",
    )))
}
async fn closing_input(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let nonce = q
        .get("nonce")
        .ok_or_else(|| anyhow::anyhow!("Missing closing token"))?
        .clone();
    let h = app.by_sid(f.field("CallSid")).await?;
    let response = h
        .send(Input::ClosingInput {
            nonce,
            text: f.field("SpeechResult").into(),
        })
        .await?;
    if let Some(token) = response["gather_turn_token"].as_str() {
        // Continue the already signed form through the existing bounded Gather
        // turn handler; closing corrections never upgrade the selected transport.
        return turn(
            State(app),
            Query(HashMap::from([("turn_id".into(), token.into())])),
            Extension(f),
        )
        .await;
    }
    Ok(xml(response["twiml"]
        .as_str()
        .unwrap_or("<Response><Hangup/></Response>")
        .into()))
}
async fn turn(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    let h = app.by_sid(f.field("CallSid")).await?;
    let text = f.field("SpeechResult");
    let nonce = q
        .get("turn_id")
        .ok_or_else(|| anyhow::anyhow!("Missing Gather token"))?
        .clone();
    let claim = h
        .send(Input::GatherClaim {
            nonce: nonce.clone(),
        })
        .await?;
    if claim["fresh"] != true {
        if let Some(cached) = claim["twiml"].as_str() {
            return Ok(xml(cached.into()));
        }
        for _ in 0..35 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let s = h.send(Input::Snapshot).await?;
            if let Some(cached) = s["gather_replies"][&nonce].as_str() {
                return Ok(xml(cached.into()));
            }
        }
        return Ok(xml(twilio::say_end(
            "The previous request is still being handled. No new action will be started. Goodbye.",
        )));
    }
    let next = claim["next_token"].as_str().unwrap_or_default();
    if text.trim().is_empty() {
        let twiml = twilio::say_end("No message received. Goodbye.");
        h.send(Input::GatherComplete {
            nonce,
            twiml: twiml.clone(),
        })
        .await?;
        return Ok(xml(twiml));
    }
    let mut audio = h.audio.subscribe();
    h.send(Input::FinalPrompt { text: text.into() }).await?;
    let reply = tokio::time::timeout(Duration::from_secs(7), audio.recv())
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|f| f.payload["token"].as_str().map(String::from))
        .unwrap_or_else(|| {
            "The assistant is temporarily unavailable. No connection has been made.".into()
        });
    let s = h.send(Input::Snapshot).await?;
    let twiml = if matches!(
        s["closing_reason"].as_str(),
        Some("message_saved" | "model_failure" | "no_message")
    ) {
        h.send(Input::ClosingResponse).await?["twiml"]
            .as_str()
            .unwrap_or("<Response><Hangup/></Response>")
            .to_string()
    } else if s["offer"]["status"] == "pending" {
        // Turn-based fallback: A is paused during private choice, not monitored.
        format!(
            "<Response><Say voice=\"Polly.Joanna-Neural\" language=\"en-US\">{}</Say><Pause length=\"30\"/><Hangup/></Response>",
            twilio::escape(&reply)
        )
    } else if s["blocked"] == true || s["inbound_ended"] == true {
        twilio::say_end(&reply)
    } else {
        twilio::speech_gather(
            &app.config
                .external_url(&format!("/voice/turn?turn_id={next}")),
            &reply,
        )
    };
    h.send(Input::GatherComplete {
        nonce,
        twiml: twiml.clone(),
    })
    .await?;
    Ok(xml(twiml))
}

async fn relay_ws(State(app): State<App>, ws: WebSocketUpgrade) -> Response {
    ws.max_message_size(32768)
        .max_frame_size(32768)
        .on_upgrade(move |socket| relay_socket(app, socket))
}
async fn relay_socket(app: App, mut socket: WebSocket) {
    use crate::providers::relay::{self, RelayInput};
    let started = std::time::Instant::now();
    let setup_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut control_frames = 0_u32;
    let raw = loop {
        let incoming = tokio::time::timeout_at(setup_deadline, socket.recv()).await;
        let frame_kind = match &incoming {
            Ok(Some(Ok(Message::Text(_)))) => "text",
            Ok(Some(Ok(Message::Ping(_)))) => "ping",
            Ok(Some(Ok(Message::Pong(_)))) => "pong",
            Ok(Some(Ok(Message::Close(_)))) => "close",
            Ok(Some(Ok(Message::Binary(_)))) => "binary",
            Ok(Some(Err(_))) => "socket_error",
            Ok(None) => "eof",
            Err(_) => "timeout",
        };
        if control_frames == 0 {
            tracing::info!(frame_kind, "Relay initial authenticated frame kind");
        }
        match incoming {
            Ok(Some(Ok(Message::Text(raw)))) => break raw,
            Ok(Some(Ok(Message::Ping(bytes)))) => {
                control_frames = control_frames.saturating_add(1);
                if !matches!(
                    tokio::time::timeout_at(setup_deadline, socket.send(Message::Pong(bytes)))
                        .await,
                    Ok(Ok(()))
                ) {
                    tracing::warn!(control_frames, "Relay initial pong could not be sent");
                    return;
                }
            }
            Ok(Some(Ok(Message::Pong(_)))) => {
                control_frames = control_frames.saturating_add(1);
            }
            other => {
                let close_code = match other {
                    Ok(Some(Ok(Message::Close(Some(frame))))) => Some(frame.code),
                    _ => None,
                };
                tracing::warn!(
                    frame_kind,
                    close_code,
                    control_frames,
                    "Relay authenticated socket ended before initial text frame"
                );
                return;
            }
        }
    };
    // Only field shapes and binding booleans enter diagnostics. Never log the
    // raw authenticated frame, phone identities or the session-token value.
    let shape: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let message_type = match shape["type"].as_str() {
        Some("setup") => "setup",
        Some("info") => "info",
        Some("error") => "error",
        _ => "other",
    };
    tracing::info!(
        message_type,
        bytes = raw.len(),
        account_present = shape.get("accountSid").is_some(),
        call_present = shape.get("callSid").is_some(),
        session_present = shape.get("sessionId").is_some(),
        custom_parameters_present = shape.get("customParameters").is_some(),
        session_token_present = shape["customParameters"].get("session_token").is_some(),
        error_code = shape["code"].as_u64(),
        "Relay initial authenticated frame shape"
    );
    let Ok(RelayInput::Setup {
        call_sid,
        account_sid,
        session_token,
        ..
    }) = relay::parse(&raw)
    else {
        tracing::warn!(
            reason = relay::parse(&raw)
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "initial frame was not setup".into()),
            "Relay setup rejected"
        );
        return;
    };
    if account_sid != app.config.account_sid {
        tracing::warn!("Relay setup account binding rejected");
        return;
    }
    let Ok(h) = app.by_sid(&call_sid).await else {
        tracing::warn!("Relay setup call binding rejected");
        return;
    };
    // Subscribe before attaching: a queued closing interruption can start its
    // model immediately after setup, and broadcast does not retain old frames.
    let mut audio = h.audio.subscribe();
    if h.send(Input::ConnectRelay {
        sid: call_sid.clone(),
        token: session_token.clone(),
    })
    .await
    .is_err()
    {
        tracing::warn!("Relay setup session-token or active-leg binding rejected");
        return;
    }
    relay_observation(&h, started, "setup", None).await;
    loop {
        tokio::select! {biased;
         incoming=socket.recv()=>{
          match incoming {
           Some(Ok(Message::Text(raw)))=>{
            let (command,event,characters)=match relay::parse(&raw){
             Ok(RelayInput::Prompt{voice_prompt,last,..})=>{
               let characters=Some(voice_prompt.chars().count());
               (Some(Input::RelaySpeech{token:session_token.clone(),text:Some(voice_prompt),last}),if last{"prompt.final"}else{"prompt.partial"},characters)
             },
             Ok(RelayInput::Interrupt{..})=>(Some(Input::RelaySpeech{token:session_token.clone(),text:None,last:false}),"interrupt",None),
             Ok(RelayInput::Dtmf{digit})=>(Some(Input::Keypad{sid:call_sid.clone(),digits:digit}),"dtmf",None),
             Ok(RelayInput::Error{..})|Err(_)=>{let _=h.send(Input::RelayDisconnected{token:session_token.clone()}).await;relay_observation(&h,started,"error",None).await;break;},
             _=>(None,"other",None),
            };
            // Caller policy input must enter the actor before observability can
            // wait on SQLite and allow a concurrent private acceptance to pass.
            if let Some(command)=command{let _=h.send(command).await;}
            relay_observation(&h,started,event,characters).await;
           }
           Some(Ok(Message::Ping(bytes)))=>{if socket.send(Message::Pong(bytes)).await.is_err(){break;}},
           Some(Ok(Message::Close(_)))|None|Some(Err(_))=>break,
           _=>{},
          }
         }
         outgoing=audio.recv()=>{
          let Ok(frame)=outgoing else{break;};
          let Ok(s)=h.send(Input::Snapshot).await else{break;};
          let Ok(session)=serde_json::from_value::<Session>(s) else {break;};
          if session.relay_token != session_token { break; }
          if frame.permitted_for(&session) {
            if socket.send(Message::Text(frame.payload.to_string().into())).await.is_err(){break;}
            let event=match frame.purpose {
              crate::calls::actor::AudioPurpose::MessageClosing | crate::calls::actor::AudioPurpose::ModelFailureClosing | crate::calls::actor::AudioPurpose::NoMessageClosing=>"output.end",
              crate::calls::actor::AudioPurpose::PendingSpeechRecovery=>"output.recovery",
              crate::calls::actor::AudioPurpose::Response=>"output.text",
            };
            relay_observation(&h,started,event,frame.payload["token"].as_str().map(|t|t.chars().count())).await;
          } else {
            relay_observation(&h,started,"output.discarded",None).await;
          }
         }
        }
    }
    let _ = h
        .send(Input::RelayDisconnected {
            token: session_token,
        })
        .await;
    relay_observation(&h, started, "closed", None).await;
}

async fn relay_observation(
    handle: &crate::calls::actor::Handle,
    started: std::time::Instant,
    event: &'static str,
    characters: Option<usize>,
) {
    let _ = handle
        .send(Input::RelayTelemetry {
            event,
            elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            characters,
        })
        .await;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Replay {
    scenario: String,
}
async fn replay(State(app): State<App>, Json(p): Json<Replay>) -> ApiResult {
    Ok(Json(crate::replay::run(&app, &p.scenario).await?).into_response())
}

async fn recording_status(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    if f.field("RecordingStatus") == "completed"
        && let Some(session) = q.get("session_id")
    {
        let seconds = f.field("RecordingDuration").parse().unwrap_or(0);
        crate::call_recording::completed(&app, session, f.field("RecordingSid"), seconds).await?;
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn clone_recorded() -> ApiResult {
    Ok(xml(crate::voice_clone::recorded_twiml()))
}
async fn clone_recording(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
    Extension(f): Extension<FormData>,
) -> ApiResult {
    if f.field("RecordingStatus") == "completed"
        && let Some(household) = q.get("household")
    {
        crate::voice_clone::process_recording(
            app.clone(),
            household.clone(),
            f.field("RecordingSid").into(),
            f.field("RecordingDuration").parse().unwrap_or(0),
        );
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn recording_info(State(app): State<App>, Path(id): Path<String>) -> ApiResult {
    let info = crate::call_recording::info(&app, &id).await?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"available": info.is_some(), "recording": info.map(|v| json!({"duration_seconds": v["duration_seconds"], "channels": v["channels"]}))})),
    )
        .into_response())
}
async fn recording_audio(State(app): State<App>, Path(id): Path<String>) -> Response {
    let Ok(Some(info)) = crate::call_recording::info(&app, &id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"No recording for this call"})),
        )
            .into_response();
    };
    match app
        .twilio
        .fixture_recording(info["recording_sid"].as_str().unwrap_or_default())
        .await
    {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "audio/mpeg"),
                (header::CACHE_CONTROL, "private, no-store"),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloneArm {
    household_id: Option<String>,
    speaker_name: String,
    consent: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloneTarget {
    household_id: Option<String>,
}
async fn clone_household(app: &App, requested: Option<&str>) -> anyhow::Result<String> {
    Ok(match requested {
        Some(id) => id.to_string(),
        None => app
            .store
            .metadata("selected_household")
            .await?
            .unwrap_or_else(|| "sarah".into()),
    })
}
async fn clone_status(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult {
    let household = clone_household(&app, q.get("household").map(String::as_str)).await?;
    let value = crate::voice_clone::load(&app, &household).await?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(crate::voice_clone::view(value.as_ref(), &household)),
    )
        .into_response())
}
async fn clone_arm(State(app): State<App>, Json(p): Json<CloneArm>) -> ApiResult {
    if !p.consent {
        return Err(anyhow::anyhow!("Consent is required").into());
    }
    let household = clone_household(&app, p.household_id.as_deref()).await?;
    let value = crate::voice_clone::arm(&app, &household, &p.speaker_name).await?;
    Ok(Json(crate::voice_clone::view(Some(&value), &household)).into_response())
}
async fn clone_activate(State(app): State<App>, Json(p): Json<CloneTarget>) -> ApiResult {
    let household = clone_household(&app, p.household_id.as_deref()).await?;
    let value = crate::voice_clone::set_active(&app, &household, true).await?;
    Ok(Json(crate::voice_clone::view(Some(&value), &household)).into_response())
}
async fn clone_deactivate(State(app): State<App>, Json(p): Json<CloneTarget>) -> ApiResult {
    let household = clone_household(&app, p.household_id.as_deref()).await?;
    let value = crate::voice_clone::set_active(&app, &household, false).await?;
    Ok(Json(crate::voice_clone::view(Some(&value), &household)).into_response())
}
async fn clone_delete(State(app): State<App>, Json(p): Json<CloneTarget>) -> ApiResult {
    let household = clone_household(&app, p.household_id.as_deref()).await?;
    let value = crate::voice_clone::delete(&app, &household).await?;
    Ok(Json(crate::voice_clone::view(Some(&value), &household)).into_response())
}
async fn clone_preview(
    State(app): State<App>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let result = async {
        let household = clone_household(&app, q.get("household").map(String::as_str)).await?;
        let value = crate::voice_clone::load(&app, &household)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No enrolled voice"))?;
        let voice = value
            .voice_id
            .ok_or_else(|| anyhow::anyhow!("The voice is not ready"))?;
        let profile = crate::profile::Profile::load(&app.store, &household).await?;
        crate::voice_clone::synthesize(
            &voice,
            &format!(
                "Hi, this is FirstRing answering for {}. What's the call about?",
                profile.display_name
            ),
        )
        .await
    }
    .await;
    match result {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "audio/mpeg"),
                (header::CACHE_CONTROL, "private, no-store"),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => (
            StatusCode::CONFLICT,
            Json(json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}
async fn voice_media(Path(capability): Path<String>) -> Response {
    match crate::voice_clone::get_media(&capability) {
        Some(bytes) => (
            [
                (header::CONTENT_TYPE, "audio/mpeg"),
                (header::CACHE_CONTROL, "private, no-store"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
