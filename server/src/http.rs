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
        .route("/voice/turn", post(turn))
        .route("/voice/whisper/{id}", post(whisper))
        .route("/voice/choice/{id}", post(choice))
        .route("/voice/conference-status", post(conference_status))
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
        .route("/api/session/{id}", get(session))
        .route("/api/events", get(events))
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
        .merge(voice)
        .merge(api)
        .fallback_service(
            tower_http::services::ServeDir::new("web/dist").append_index_html_on_directories(true),
        )
        .with_state(app)
}
async fn admin(State(app): State<App>, req: Request, next: Next) -> Response {
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
    if binding == 1 {
        crate::audio_fixture::watch_inbound(app.clone(), form.field("CallSid").into());
    }
    if snapshot["inbound_ended"] == true {
        return Ok(xml(twilio::say_end(
            "This test session has ended. Please call again.",
        )));
    }
    if app.config.transport == "relay" {
        return Ok(xml(crate::providers::relay::connect_twiml(
            &app.config.public_base_url,
            snapshot["relay_token"].as_str().unwrap_or_default(),
            if snapshot["household_id"] == "maya" {
                "Maya"
            } else {
                "Sarah"
            },
        )));
    }
    if app.config.transport == "gather" {
        return Ok(xml(twilio::speech_gather(
            &app.config.external_url(&format!(
                "/voice/turn?turn_id={}",
                snapshot["relay_token"].as_str().unwrap_or_default()
            )),
            "I'm your automated assistant. What is the call about? Please don't share any sign-in codes or passwords.",
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
    let phone_b = crate::evidence::phone_b(c, &app.store).await?;
    let capabilities = json!([
    {"id":"inbound","status":if c.inbound_configured(){"configured"}else{"pending"},"detail":"Local signing and HTTPS inputs; recorded phone proof is shown separately"},
    {"id":"outbound","status":if c.outbound_configured(){"configured"}else{"pending"},"detail":"Separate Photon SIP credentials and caller number; endpoint enrollment, authorization and real outbound test remain separate gates"},
    {"id":"nemotron","status":if c.model_configured(){"configured"}else{"pending"},"detail":"Local hosted model inputs; protocol evidence and behavior evaluation are separate"},
    {"id":"relay","status":if c.relay_configured(){"configured"}else{"pending"},"detail":"Local signed-WebSocket inputs only; onboarding and real phone protocol test pending"},
    {"id":"smoke_test","status":if c.smoke_authorized{"configured"}else{"pending"},"detail":"Exactly one authorized outbound attempt to Sarah/Phone B"},
    {"id":"family","status":if c.family_enabled{"configured"}else{"disabled"},"detail":"Family calls require separate enrollment and authorization"},
    {"id":"voice_clone","status":"disabled","detail":"Preset assistant voice; no clone endpoint verified"}
    ]);
    Ok(Json(json!({"execution_mode":c.mode,"data_origin":"synthetic","transport":c.transport,"phone_display":if c.photon_number.is_empty(){"Not configured"}else{&c.photon_number},"selected_household":app.store.metadata("selected_household").await?.unwrap_or_else(||"sarah".into()),"capabilities":capabilities,"evidence":evidence,"phone_b":phone_b,"speech":relay::speech_configuration(&c.transport),"sessions":snapshots,"community":community,"tactics":tactics})).into_response())
}
async fn ready(State(app): State<App>) -> Response {
    let c = &app.config;
    let ready = c.selected_transport_configured() && !c.admin_token.is_empty();
    let evidence = crate::evidence::view(&app.store).await.unwrap_or_default();
    let inbound = evidence["inbound_phone"]["status"] == "passed";
    let relay = evidence["relay_phone"]["status"] == "passed";
    let model = evidence["model_protocol"]["status"] == "passed";
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
        Json(json!({"configured":ready,"live_verified":selected,"execution_mode":c.mode,"transport":c.transport,"evidence":evidence,
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
async fn after_relay(State(app): State<App>, Extension(f): Extension<FormData>) -> ApiResult {
    let h = app.by_sid(f.field("CallSid")).await?;
    let s = h.send(Input::Snapshot).await?;
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
        Some("message_saved" | "model_failure")
    ) {
        let closing = h.send(Input::ClosingResponse).await?;
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
    h.send(Input::RelayDisconnected).await?;
    Ok(xml(twilio::say_end(
        "The assistant connection ended. No private choice will connect this call. Goodbye.",
    )))
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
        Some("message_saved" | "model_failure")
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
    if h.send(Input::ConnectRelay {
        sid: call_sid.clone(),
        token: session_token,
    })
    .await
    .is_err()
    {
        tracing::warn!("Relay setup session-token or active-leg binding rejected");
        return;
    }
    let mut audio = h.audio.subscribe();
    relay_observation(&h, started, "setup", None).await;
    loop {
        tokio::select! {biased;
         incoming=socket.recv()=>{
          match incoming {
           Some(Ok(Message::Text(raw)))=>{
            let (command,event,characters)=match relay::parse(&raw){
             Ok(RelayInput::Prompt{voice_prompt,last,..})=>{
               let characters=Some(voice_prompt.chars().count());
               (Some(if last {Input::FinalPrompt{text:voice_prompt}}else{Input::Speech{partial:Some(voice_prompt)}}),if last{"prompt.final"}else{"prompt.partial"},characters)
             },
             Ok(RelayInput::Interrupt{..})=>(Some(Input::Speech{partial:None}),"interrupt",None),
             Ok(RelayInput::Dtmf{digit})=>(Some(Input::Keypad{sid:call_sid.clone(),digits:digit}),"dtmf",None),
             Ok(RelayInput::Error{..})|Err(_)=>{let _=h.send(Input::RelayDisconnected).await;relay_observation(&h,started,"error",None).await;break;},
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
          if frame.permitted_for(&session) {
            if socket.send(Message::Text(frame.payload.to_string().into())).await.is_err(){break;}
            let event=match frame.purpose {
              crate::calls::actor::AudioPurpose::MessageClosing | crate::calls::actor::AudioPurpose::ModelFailureClosing=>"output.end",
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
    let _ = h.send(Input::RelayDisconnected).await;
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
