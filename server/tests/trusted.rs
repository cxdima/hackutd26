//! Trusted callers: an operator-managed list per household whose calls skip AI
//! screening and ring straight through to the household's enrolled phone.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use firstring::{
    calls::actor::App,
    config::{Config, ExecutionMode},
    http::router,
};
use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha1::Sha1;
use tower::ServiceExt;
const TOKEN: &str = "fixture-trusted-operator-token";

async fn app(enrolled: bool) -> App {
    let mut c = Config::from_env().unwrap();
    c.mode = ExecutionMode::Live;
    c.database_url = "sqlite::memory:".into();
    c.public_base_url = "https://trusted.fixture.example".into();
    c.account_sid = "AC00000000000000000000000000000000".into();
    c.auth_token = "fixture-token".into();
    c.admin_token = TOKEN.into();
    c.transport = "relay".into();
    c.photon_number = "+15550009999".into();
    c.sip_host = "sip.fixture.example".into();
    c.sip_username = "fixture-user".into();
    c.sip_password = "fixture-pass".into();
    c.nvidia_key.clear();
    c.contacts.clear();
    if enrolled {
        c.contacts.insert("sarah".into(), "+15550004444".into());
    }
    c.smoke_authorized = false;
    c.handoff_enabled = false;
    c.family_enabled = false;
    App::new(c).await.unwrap()
}

fn incoming(sid: &str, from: &str) -> Request<Body> {
    let body = format!(
        "AccountSid=AC00000000000000000000000000000000&CallSid={sid}&From={}",
        url::form_urlencoded::byte_serialize(from.as_bytes()).collect::<String>()
    );
    let mut m = Hmac::<Sha1>::new_from_slice(b"fixture-token").unwrap();
    m.update(b"https://trusted.fixture.example/voice/incoming");
    let mut p: Vec<_> = url::form_urlencoded::parse(body.as_bytes()).collect();
    p.sort();
    for (k, v) in p {
        m.update(k.as_bytes());
        m.update(v.as_bytes());
    }
    Request::post("/voice/incoming")
        .header("content-type", "application/x-www-form-urlencoded")
        .header(
            "x-twilio-signature",
            STANDARD.encode(m.finalize().into_bytes()),
        )
        .body(Body::from(body))
        .unwrap()
}

async fn send(app: &App, request: Request<Body>) -> (StatusCode, String) {
    let response = router(app.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn api(
    app: &App,
    method: &str,
    path: &str,
    body: Option<Value>,
    auth: bool,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    if auth {
        req = req.header("authorization", format!("Bearer {TOKEN}"));
    }
    if body.is_some() {
        req = req.header("content-type", "application/json");
    }
    let (status, text) = send(
        app,
        req.body(Body::from(body.map_or(String::new(), |b| b.to_string())))
            .unwrap(),
    )
    .await;
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

#[tokio::test]
async fn trusted_caller_skips_ai_and_rings_the_enrolled_phone() {
    let app = app(true).await;
    assert_eq!(
        api(&app, "GET", "/api/trusted?household=sarah", None, false)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, added) = api(
        &app,
        "POST",
        "/api/trusted",
        Some(json!({"household_id":"sarah","name":"Jimmy","relation":"Son","number":"(555) 000-1234"})),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let entry = &added["items"][0];
    assert_eq!(entry["name"], "Jimmy");
    assert_eq!(entry["number_label"], "ending 1234");
    assert!(
        !added.to_string().contains("5550001234"),
        "full number never returned"
    );

    // The trusted number: no Relay, no AI; a greeting by name and a direct dial.
    let (status, twiml) = send(
        &app,
        incoming("CA00000000000000000000000000000011", "+15550001234"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!twiml.contains("ConversationRelay"), "{twiml}");
    assert!(twiml.contains("Jimmy"), "{twiml}");
    assert!(twiml.contains("<Dial"), "{twiml}");
    assert!(
        twiml.contains("sip:+15550004444@sip.fixture.example"),
        "{twiml}"
    );
    assert!(
        !twiml.contains("Record"),
        "trusted calls are not recorded: {twiml}"
    );

    // Everyone else is still screened.
    let (_, screened) = send(
        &app,
        incoming("CA00000000000000000000000000000012", "+15550007777"),
    )
    .await;
    assert!(screened.contains("ConversationRelay"), "{screened}");

    // The pass-through is a receipt on the call's own session, without the number.
    let receipts: Vec<String> = sqlx::query_scalar(
        "SELECT data FROM events WHERE json_extract(data,'$.type')='trusted.passthrough'",
    )
    .fetch_all(&app.store.pool)
    .await
    .unwrap();
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0].contains("Jimmy") && receipts[0].contains("session_id"));
    assert!(!receipts[0].contains("5550001234"));

    // Removing the entry restores screening for that number.
    let id = entry["id"].as_str().unwrap();
    let (status, after) = api(
        &app,
        "POST",
        "/api/trusted/remove",
        Some(json!({"household_id":"sarah","id":id})),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after["items"], json!([]));
    let (_, again) = send(
        &app,
        incoming("CA00000000000000000000000000000013", "+15550001234"),
    )
    .await;
    assert!(again.contains("ConversationRelay"), "{again}");
}

#[tokio::test]
async fn trusted_list_rejects_bad_input_and_needs_an_enrolled_phone_to_connect() {
    let app = app(false).await;
    for bad in [
        json!({"household_id":"sarah","name":"Jimmy","relation":"Son","number":"12"}),
        json!({"household_id":"sarah","name":"<script>","relation":"Son","number":"+15550001234"}),
        json!({"household_id":"nobody","name":"Jimmy","relation":"Son","number":"+15550001234"}),
    ] {
        assert_ne!(
            api(&app, "POST", "/api/trusted", Some(bad.clone()), true)
                .await
                .0,
            StatusCode::OK,
            "{bad}"
        );
    }
    let (status, _) = api(
        &app,
        "POST",
        "/api/trusted",
        Some(
            json!({"household_id":"sarah","name":"Jimmy","relation":"Son","number":"+15550001234"}),
        ),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // Duplicate numbers are refused.
    assert_ne!(
        api(
            &app,
            "POST",
            "/api/trusted",
            Some(json!({"household_id":"sarah","name":"Jim","relation":"Son","number":"+1 555 000 1234"})),
            true
        )
        .await
        .0,
        StatusCode::OK
    );
    // No enrolled phone: the trusted caller is told plainly, never screened by the AI.
    let (_, twiml) = send(
        &app,
        incoming("CA00000000000000000000000000000021", "+15550001234"),
    )
    .await;
    assert!(!twiml.contains("ConversationRelay"), "{twiml}");
    assert!(!twiml.contains("<Dial"), "{twiml}");
    assert!(twiml.contains("Jimmy"), "{twiml}");
}
