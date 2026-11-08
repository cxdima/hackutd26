use crate::{
    calls::{
        actor::{App, Input},
        state::Session,
    },
    providers::{signature, twilio},
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
        .route_layer(middleware::from_fn_with_state(app.clone(), admin));
    Router::new()
        .route("/health/live", get(|| async { Json(json!({"ok":true})) }))
        .route("/health/ready", get(ready))
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
    let (_, snapshot) = app.incoming(form.field("CallSid")).await?;
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
        app.by_action(action)
            .await?
            .send(Input::Status {
                action_id: action.clone(),
                sid: f.field("CallSid").into(),
                status: f.field("CallStatus").into(),
            })
            .await?;
    } else if crate::calls::state::terminal(f.field("CallStatus")) {
        app.by_sid(f.field("CallSid"))
            .await?
            .send(Input::InboundEnded)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT.into_response())
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
    let configured = c.telephony_configured();
    let (community, tactics) = crate::patterns::view(&app.store, c.mode).await?;
    let capabilities = json!([
    {"id":"telephony","status":if configured{"configured"}else{"pending"},"detail":if configured{"Credentials present; real phone gate not yet verified"}else{"Configure Photon/Twilio credentials and public HTTPS"}},
    {"id":"nemotron","status":if c.nvidia_key.is_empty(){"pending"}else{"configured"},"detail":"Native tool-call access requires a real probe"},
    {"id":"relay","status":"pending","detail":"Onboarding and real WebSocket phone test pending"},
    {"id":"smoke_test","status":if c.smoke_authorized{"configured"}else{"pending"},"detail":"Exactly one authorized outbound attempt to Sarah/Phone B"},
    {"id":"family","status":if c.family_enabled{"configured"}else{"disabled"},"detail":"Family calls require separate enrollment and authorization"},
    {"id":"voice_clone","status":"disabled","detail":"Preset assistant voice; no clone endpoint verified"}
    ]);
    Ok(Json(json!({"execution_mode":c.mode,"data_origin":"synthetic","transport":c.transport,"phone_display":if c.photon_number.is_empty(){"Not configured"}else{&c.photon_number},"selected_household":app.store.metadata("selected_household").await?.unwrap_or_else(||"sarah".into()),"capabilities":capabilities,"sessions":snapshots,"community":community,"tactics":tactics})).into_response())
}
async fn ready(State(app): State<App>) -> Response {
    let ready = app.config.telephony_configured() && !app.config.admin_token.is_empty();
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(json!({"configured":ready,"live_verified":false,"execution_mode":app.config.mode})),
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
    let twiml = if s["offer"]["status"] == "pending" {
        // Turn-based fallback: A is paused during private choice, not monitored.
        format!(
            "<Response><Say>{}</Say><Pause length=\"30\"/><Hangup/></Response>",
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
    let first = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let Ok(Some(Ok(Message::Text(raw)))) = first else {
        return;
    };
    let Ok(RelayInput::Setup {
        call_sid,
        account_sid,
        session_token,
        ..
    }) = relay::parse(&raw)
    else {
        return;
    };
    if account_sid != app.config.account_sid {
        return;
    }
    let Ok(h) = app.by_sid(&call_sid).await else {
        return;
    };
    if h.send(Input::ConnectRelay {
        sid: call_sid.clone(),
        token: session_token,
    })
    .await
    .is_err()
    {
        return;
    }
    let mut audio = h.audio.subscribe();
    loop {
        tokio::select! {biased;
         incoming=socket.recv()=>{
          match incoming {
           Some(Ok(Message::Text(raw)))=>{
            let command=match relay::parse(&raw){
             Ok(RelayInput::Prompt{voice_prompt,last:true,..})=>Some(Input::FinalPrompt{text:voice_prompt}),
             Ok(RelayInput::Prompt{voice_prompt,last:false,..})=>Some(Input::Speech{partial:Some(voice_prompt)}),
             Ok(RelayInput::Interrupt{..})=>Some(Input::Speech{partial:None}),
             Ok(RelayInput::Dtmf{digit})=>Some(Input::Keypad{sid:call_sid.clone(),digits:digit}),
             Ok(RelayInput::Speaker{speaker,started:true}) if speaker=="client"=>Some(Input::Speech{partial:None}),
             Ok(RelayInput::Other{hold:true})=>Some(Input::Speech{partial:None}),
             Ok(RelayInput::Error{..})|Err(_)=>{let _=h.send(Input::RelayDisconnected).await;break;},
             _=>None,
            };
            if let Some(command)=command{let _=h.send(command).await;}
           }
           Some(Ok(Message::Ping(bytes)))=>{if socket.send(Message::Pong(bytes)).await.is_err(){break;}},
           Some(Ok(Message::Close(_)))|None|Some(Err(_))=>break,
           _=>{},
          }
         }
         outgoing=audio.recv()=>{
          let Ok(frame)=outgoing else{break;};
          let Ok(s)=h.send(Input::Snapshot).await else{break;};
          if s["generation_id"].as_u64()==Some(frame.generation) && s["policy_epoch"].as_u64()==Some(frame.epoch) && s["speech_pending"]==false && s["bridge_committed"]==false && socket.send(Message::Text(frame.payload.to_string().into())).await.is_err(){break;}
         }
        }
    }
    let _ = h.send(Input::RelayDisconnected).await;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Replay {
    scenario: String,
}
async fn replay(State(app): State<App>, Json(p): Json<Replay>) -> ApiResult {
    Ok(Json(crate::replay::run(&app, &p.scenario).await?).into_response())
}
