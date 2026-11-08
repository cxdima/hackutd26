//! Local WebSocket integration only. No provider or model network requests.
use base64::{Engine, engine::general_purpose::STANDARD};
use firstring::{
    calls::actor::{App, Handle, Input},
    config::{Config, ExecutionMode},
    http::router,
    providers::nemotron::ModelToolCall,
};
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha1::Sha1;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

const ACCOUNT: &str = "AC00000000000000000000000000000001";
const CALLER: &str = "CA00000000000000000000000000000002";
const SARAH: &str = "CA00000000000000000000000000000003";
const RELAY: &str = "VX00000000000000000000000000000004";
const ORIGIN: &str = "https://relay.fixture.example";
const TOKEN: &str = "fixture-twilio-auth-token";
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Server {
    app: App,
    address: std::net::SocketAddr,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn server() -> Server {
    let config = Config {
        public_base_url: ORIGIN.into(),
        port: 8080,
        database_url: "sqlite::memory:".into(),
        admin_token: "fixture-operator-token-at-least-24".into(),
        mode: ExecutionMode::Replay,
        transport: "relay".into(),
        account_sid: ACCOUNT.into(),
        auth_token: TOKEN.into(),
        photon_number: String::new(),
        sip_host: "sip.photon.codes".into(),
        sip_port: 5061,
        sip_username: String::new(),
        sip_password: String::new(),
        contacts: BTreeMap::new(),
        smoke_authorized: false,
        handoff_enabled: false,
        family_enabled: false,
        max_call_seconds: 180,
        nvidia_key: String::new(),
        model_url: "http://127.0.0.1:1/not-contacted".into(),
        model: "fixture-model".into(),
    };
    let app = App::new(config).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let routes = router(app.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, routes).await.unwrap();
    });
    Server { app, address, task }
}
fn signature(url: &str) -> String {
    let mut hmac = Hmac::<Sha1>::new_from_slice(TOKEN.as_bytes()).unwrap();
    hmac.update(url.as_bytes());
    STANDARD.encode(hmac.finalize().into_bytes())
}
async fn connect(
    server: &Server,
    path: &str,
    signed_url: Option<&str>,
) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
    let mut request = format!("ws://{}{path}", server.address)
        .into_client_request()
        .unwrap();
    if let Some(url) = signed_url {
        request
            .headers_mut()
            .insert("x-twilio-signature", signature(url).parse().unwrap());
    }
    connect_async(request).await.map(|(socket, _)| socket)
}
fn setup(snapshot: &Value) -> Value {
    json!({"type":"setup","sessionId":RELAY,"accountSid":ACCOUNT,"callSid":CALLER,
        "from":"sip:unknown@fictional.invalid","to":"sip:assistant@fictional.invalid","direction":"inbound","callStatus":"IN_PROGRESS","parentCallSid":"",
        "customParameters":{"session_token":snapshot["relay_token"]}})
}
async fn wait_snapshot(handle: &Handle, predicate: impl Fn(&Value) -> bool) -> Value {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let value = handle.send(Input::Snapshot).await.unwrap();
            if predicate(&value) {
                return value;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("Expected state was not reached by the WebSocket input")
}
async fn wait_event(app: &App, kind: &str) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if app
                .store
                .events_after(0)
                .await
                .unwrap()
                .iter()
                .any(|e| e.kind == kind)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("Expected socket event was not persisted");
}
async fn propose(handle: &Handle, name: &str, args: Value) {
    let snapshot = handle.send(Input::Snapshot).await.unwrap();
    let outcome = handle
        .send(Input::ModelTool {
            generation: snapshot["generation_id"].as_u64().unwrap(),
            epoch: snapshot["policy_epoch"].as_u64().unwrap(),
            version: snapshot["turn_version"].as_u64().unwrap(),
            deadline: Instant::now() + Duration::from_secs(5),
            tool: ModelToolCall {
                id: format!("fixture-{name}"),
                name: name.into(),
                arguments: args,
                assistant_message: json!({}),
            },
        })
        .await
        .unwrap();
    assert_eq!(
        outcome["ok"], true,
        "Fixture proposal was rejected: {outcome}"
    );
}

async fn send_json(socket: &mut Socket, value: Value) {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .unwrap();
}

// A pong is a processing barrier: the server awaits each preceding actor input.
async fn socket_barrier(socket: &mut Socket) {
    let marker = b"activity-barrier";
    socket
        .send(Message::Ping(marker.to_vec().into()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match socket.next().await.unwrap().unwrap() {
                Message::Pong(bytes) if bytes.as_ref() == marker => return,
                Message::Text(_) | Message::Pong(_) => {}
                other => panic!("Socket closed before processing barrier: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
}

async fn assessed_socket(server: &Server) -> (Handle, Socket, Value, Value) {
    let (handle, snapshot) = server.app.incoming(CALLER).await.unwrap();
    let mut socket = connect(
        server,
        "/voice/relay",
        Some("https://relay.fixture.example/voice/relay"),
    )
    .await
    .unwrap();
    send_json(&mut socket, setup(&snapshot)).await;
    wait_event(&server.app, "voice.connected").await;
    let text = "The clinic visit is Wednesday at ten.";
    send_json(
        &mut socket,
        json!({"type":"prompt","voicePrompt":text,"lang":"en-US","last":true}),
    )
    .await;
    let committed = wait_snapshot(&handle, |s| s["turn_version"] == 1).await;
    let evidence = json!([{"turn_id":committed["turns"][0]["id"],"quote":text}]);
    let observation = json!({"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"});
    propose(
        &handle,
        "record_assessment",
        json!({"observation":observation,"evidence_refs":evidence}),
    )
    .await;
    (handle, socket, evidence, observation)
}

async fn assistant_activity(socket: &mut Socket) {
    // Defensive fixtures, not asserted provider captures: the optional speaker
    // event envelope is unverified. None of these is a caller prompt/interrupt.
    for message in [
        json!({"type":"info","name":"agentSpeaking","value":"on"}),
        json!({"type":"info","name":"agentSpeaking","value":"off"}),
        json!({"type":"agentSpeaking","value":true}),
        json!({"type":"info","name":"tokensPlayed","value":"The assistant reply"}),
        json!({"type":"info","name":"configUpdated","value":{"session":{"ttsLanguage":"en-US"}}}),
    ] {
        send_json(socket, message).await;
    }
    socket_barrier(socket).await;
}

#[tokio::test]
async fn assistant_activity_cannot_cancel_a_current_reply() {
    let server = server().await;
    let (handle, mut socket, evidence, _) = assessed_socket(&server).await;
    let before = handle.send(Input::Snapshot).await.unwrap();
    assistant_activity(&mut socket).await;
    // The response was prepared before the assistant diagnostics arrived.
    let outcome = handle.send(Input::ModelTool {
        generation: before["generation_id"].as_u64().unwrap(),
        epoch: before["policy_epoch"].as_u64().unwrap(),
        version: before["turn_version"].as_u64().unwrap(),
        deadline: Instant::now() + Duration::from_secs(5),
        tool: ModelToolCall {
            id: "reply-prepared-before-diagnostics".into(),
            name: "ask_caller".into(),
            arguments: json!({"question":"Does Sarah need to bring anything to the visit?","purpose":"clarify_request","evidence_refs":evidence}),
            assistant_message: json!({}),
        },
    }).await;
    assert!(
        outcome.is_ok(),
        "Assistant diagnostics invalidated a current model response: {outcome:?}"
    );
    assert_eq!(outcome.unwrap()["ok"], true);
    let reply = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let Message::Text(reply) = reply else {
        panic!("Expected caller text, received {reply:?}")
    };
    let reply: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(
        reply["token"],
        "Does Sarah need to bring anything to the visit?"
    );
    let after = handle.send(Input::Snapshot).await.unwrap();
    assert_eq!(after["generation_id"], before["generation_id"]);
    assert_eq!(after["speech_pending"], false);
}

#[tokio::test]
async fn assistant_activity_cannot_indefinitely_hold_private_consent() {
    let server = server().await;
    let (handle, mut socket, evidence, observation) = assessed_socket(&server).await;
    propose(&handle, "offer_handoff", json!({"summary":"The caller reports a clinic appointment. Identity unverified.","observation":observation,"evidence_refs":evidence})).await;
    let before = handle.send(Input::Snapshot).await.unwrap();
    handle
        .send(Input::Status {
            action_id: before["offer"]["leg_id"].as_str().unwrap().into(),
            sid: SARAH.into(),
            status: "in-progress".into(),
        })
        .await
        .unwrap();
    assistant_activity(&mut socket).await;
    let choice = handle
        .send(Input::Choice {
            offer_id: before["offer"]["id"].as_str().unwrap().into(),
            sid: SARAH.into(),
            digits: "1".into(),
        })
        .await
        .unwrap();
    assert!(
        choice["twiml"].as_str().unwrap().contains("<Conference"),
        "Assistant diagnostics held valid consent: {choice}"
    );
    let after = handle.send(Input::Snapshot).await.unwrap();
    assert_eq!(after["bridge_committed"], true);
    assert_eq!(after["speech_pending"], false);
}

#[tokio::test]
async fn diagnostics_cannot_reopen_a_hold_from_documented_caller_speech() {
    for speech in [
        json!({"type":"prompt","voicePrompt":"Actually, before you connect","lang":"en-US","last":false}),
        json!({"type":"interrupt","utteranceUntilInterrupt":"Please hold","durationUntilInterruptMs":460}),
    ] {
        let server = server().await;
        let (handle, mut socket, evidence, observation) = assessed_socket(&server).await;
        propose(&handle, "offer_handoff", json!({"summary":"The caller reports a clinic appointment. Identity unverified.","observation":observation,"evidence_refs":evidence})).await;
        let before = handle.send(Input::Snapshot).await.unwrap();
        handle
            .send(Input::Status {
                action_id: before["offer"]["leg_id"].as_str().unwrap().into(),
                sid: SARAH.into(),
                status: "in-progress".into(),
            })
            .await
            .unwrap();
        send_json(&mut socket, speech).await;
        send_json(
            &mut socket,
            json!({"type":"info","name":"clientSpeaking","value":"off"}),
        )
        .await;
        assistant_activity(&mut socket).await;
        let choice = handle
            .send(Input::Choice {
                offer_id: before["offer"]["id"].as_str().unwrap().into(),
                sid: SARAH.into(),
                digits: "1".into(),
            })
            .await
            .unwrap();
        assert!(!choice["twiml"].as_str().unwrap().contains("<Conference"));
        let after = handle.send(Input::Snapshot).await.unwrap();
        assert_eq!(after["bridge_committed"], false);
        assert_eq!(after["speech_pending"], true);
    }
}

#[test]
fn relay_subscribes_only_to_verified_caller_input_contracts() {
    let twiml = firstring::providers::relay::connect_twiml(ORIGIN, "opaque-token", "Sarah");
    assert!(twiml.contains("partialPrompts=\"true\""));
    assert!(twiml.contains("reportInputDuringAgentSpeech=\"any\""));
    assert!(
        !twiml.contains("speaker-events"),
        "Unverified speaker event envelope must remain unsubscribed"
    );
}

#[tokio::test]
async fn websocket_upgrade_requires_signature_for_configured_origin_and_exact_query() {
    let server = server().await;
    for (path, url) in [
        ("/voice/relay", None),
        (
            "/voice/relay?nonce=changed",
            Some("https://relay.fixture.example/voice/relay?nonce=original"),
        ),
        ("/voice/relay", Some("https://attacker.invalid/voice/relay")),
    ] {
        let error = connect(&server, path, url).await.unwrap_err();
        match error {
            tokio_tungstenite::tungstenite::Error::Http(response) => {
                assert_eq!(response.status(), 403)
            }
            other => panic!("Expected rejected upgrade, received {other}"),
        }
    }
    assert!(server.app.store.sessions().await.unwrap().is_empty());
    // Transport Host is localhost, yet only the configured public URL is signed.
    let mut accepted = connect(
        &server,
        "/voice/relay?nonce=exact",
        Some("https://relay.fixture.example/voice/relay?nonce=exact"),
    )
    .await
    .unwrap();
    accepted.close(None).await.unwrap();
}

#[tokio::test]
async fn invalid_setup_binding_never_emits_voice_connected() {
    for invalid in ["token", "account", "call", "relay_session"] {
        let server = server().await;
        let (_, snapshot) = server.app.incoming(CALLER).await.unwrap();
        let mut socket = connect(
            &server,
            "/voice/relay",
            Some("https://relay.fixture.example/voice/relay"),
        )
        .await
        .unwrap();
        let mut message = setup(&snapshot);
        match invalid {
            "token" => message["customParameters"]["session_token"] = json!("wrong-token"),
            "account" => message["accountSid"] = json!("AC00000000000000000000000000000999"),
            "call" => message["callSid"] = json!("CA00000000000000000000000000000999"),
            _ => message["sessionId"] = json!("not-a-relay-session-id"),
        }
        socket
            .send(Message::Text(message.to_string().into()))
            .await
            .unwrap();
        let closed = tokio::time::timeout(Duration::from_secs(2), socket.next())
            .await
            .expect("Invalid setup should terminate this socket");
        assert!(!matches!(closed, Some(Ok(Message::Text(_)))));
        assert!(
            !server
                .app
                .store
                .events_after(0)
                .await
                .unwrap()
                .iter()
                .any(|e| e.kind == "voice.connected"),
            "Invalid {invalid} emitted a connected receipt"
        );
    }
}

#[tokio::test]
async fn real_socket_turn_boundaries_keep_keypad_private_and_disconnect_revokes_offer() {
    let server = server().await;
    let (handle, snapshot) = server.app.incoming(CALLER).await.unwrap();
    let mut socket = connect(
        &server,
        "/voice/relay",
        Some("https://relay.fixture.example/voice/relay"),
    )
    .await
    .unwrap();
    socket
        .send(Message::Text(setup(&snapshot).to_string().into()))
        .await
        .unwrap();
    wait_event(&server.app, "voice.connected").await;
    socket
        .send(Message::Text(
            json!({"type":"prompt","voicePrompt":"The clinic visit","lang":"en-US","last":false})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let partial = wait_snapshot(&handle, |s| s["speech_pending"] == true).await;
    assert!(partial["turns"].as_array().unwrap().is_empty());
    let text = "The clinic visit is Wednesday at ten.";
    socket
        .send(Message::Text(
            json!({"type":"prompt","voicePrompt":text,"lang":"en-US","last":true})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let committed = wait_snapshot(&handle, |s| s["turn_version"] == 1).await;
    assert_eq!(committed["turns"].as_array().unwrap().len(), 1);
    assert_eq!(committed["turns"][0]["text"], text);
    assert_eq!(committed["speech_pending"], false);
    assert_eq!(committed["assessed_version"], 0);
    let evidence = json!([{"turn_id":committed["turns"][0]["id"],"quote":text}]);
    let observation = json!({"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"});
    propose(
        &handle,
        "record_assessment",
        json!({"observation":observation,"evidence_refs":evidence}),
    )
    .await;
    propose(&handle,"offer_handoff",json!({"summary":"A caller says the clinic visit is Wednesday at ten. Identity unverified.","observation":observation,"evidence_refs":evidence})).await;
    let caller_audio = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .expect("Validated caller audio was not sent over the socket")
        .expect("Socket closed before caller audio")
        .expect("Socket audio failed");
    let Message::Text(caller_audio) = caller_audio else {
        panic!("Expected a ConversationRelay text message");
    };
    let caller_audio: Value = serde_json::from_str(&caller_audio).unwrap();
    assert_eq!(caller_audio["type"], "text");
    assert_eq!(caller_audio["last"], true);
    assert!(
        !caller_audio["token"]
            .as_str()
            .unwrap()
            .contains("Wednesday"),
        "Sarah's private summary leaked onto caller audio"
    );
    let offer = handle.send(Input::Snapshot).await.unwrap();
    handle
        .send(Input::Status {
            action_id: offer["offer"]["leg_id"].as_str().unwrap().into(),
            sid: SARAH.into(),
            status: "in-progress".into(),
        })
        .await
        .unwrap();
    socket
        .send(Message::Text(
            json!({"type":"dtmf","digit":"1"}).to_string().into(),
        ))
        .await
        .unwrap();
    wait_event(&server.app, "transport.keypad").await;
    let after_key = handle.send(Input::Snapshot).await.unwrap();
    assert_eq!(after_key["legs"][0]["acknowledged"], false);
    assert_eq!(after_key["offer"]["status"], "pending");
    assert_eq!(after_key["bridge_committed"], false);
    assert!(
        !server
            .app
            .store
            .events_after(0)
            .await
            .unwrap()
            .iter()
            .any(|e| e.kind == "contact.acknowledged")
    );
    socket.close(None).await.unwrap();
    let disconnected = wait_snapshot(&handle, |s| s["offer"]["status"] == "revoked").await;
    assert_eq!(disconnected["speech_pending"], true);
    assert_eq!(disconnected["bridge_committed"], false);
}

#[tokio::test]
async fn duplicate_setup_cannot_displace_the_attached_socket() {
    let server = server().await;
    let (handle, snapshot) = server.app.incoming(CALLER).await.unwrap();
    let mut first = connect(
        &server,
        "/voice/relay",
        Some("https://relay.fixture.example/voice/relay"),
    )
    .await
    .unwrap();
    first
        .send(Message::Text(setup(&snapshot).to_string().into()))
        .await
        .unwrap();
    wait_event(&server.app, "voice.connected").await;
    let mut duplicate = connect(
        &server,
        "/voice/relay",
        Some("https://relay.fixture.example/voice/relay"),
    )
    .await
    .unwrap();
    duplicate
        .send(Message::Text(setup(&snapshot).to_string().into()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), duplicate.next())
        .await
        .unwrap();
    first.send(Message::Text(json!({"type":"prompt","voicePrompt":"The original socket is still active.","lang":"en-US","last":true}).to_string().into())).await.unwrap();
    wait_snapshot(&handle, |s| s["turn_version"] == 1).await;
    let connected = server
        .app
        .store
        .events_after(0)
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.kind == "voice.connected")
        .count();
    assert_eq!(connected, 1);
    first.close(None).await.unwrap();
}
