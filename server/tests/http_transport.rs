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
use sha1::Sha1;
use tower::ServiceExt;
async fn app() -> App {
    let mut c = Config::from_env().unwrap();
    c.database_url = "sqlite::memory:".into();
    c.public_base_url = "https://demo.example".into();
    c.auth_token = "test-token".into();
    c.account_sid = "AC00000000000000000000000000000000".into();
    c.admin_token = "a-long-local-test-admin-token".into();
    c.mode = ExecutionMode::Replay;
    c.transport = "probe".into();
    App::new(c).await.unwrap()
}
fn signed(path: &str, body: &str) -> Request<Body> {
    let mut mac = Hmac::<Sha1>::new_from_slice(b"test-token").unwrap();
    mac.update(format!("https://demo.example{path}").as_bytes());
    let mut params: Vec<_> = url::form_urlencoded::parse(body.as_bytes()).collect();
    params.sort();
    for (k, v) in params {
        mac.update(k.as_bytes());
        mac.update(v.as_bytes());
    }
    Request::post(path)
        .header("content-type", "application/x-www-form-urlencoded")
        .header(
            "x-twilio-signature",
            STANDARD.encode(mac.finalize().into_bytes()),
        )
        .body(Body::from(body.to_string()))
        .unwrap()
}
#[tokio::test]
async fn unsigned_ingress_is_rejected_before_session_creation() {
    let a = app().await;
    let r = router(a.clone())
        .oneshot(
            Request::post("/voice/incoming")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert!(a.store.sessions().await.unwrap().is_empty());
}
#[tokio::test]
async fn signed_inbound_greets_and_keypad_stays_on_its_own_leg() {
    let a = app().await;
    let body = "AccountSid=AC00000000000000000000000000000000&CallSid=CA00000000000000000000000000000001&From=%2B12025550123";
    let r = router(a.clone())
        .oneshot(signed("/voice/incoming", body))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.contains("automated assistant"));
    assert!(xml.contains("/voice/probe-choice"));
    let wrong = "AccountSid=AC00000000000000000000000000000000&CallSid=CA00000000000000000000000000000002&Digits=1";
    let r = router(a.clone())
        .oneshot(signed("/voice/probe-choice", wrong))
        .await
        .unwrap();
    assert!(!r.status().is_success());
    let right = format!("{body}&Digits=1");
    let r = router(a.clone())
        .oneshot(signed("/voice/probe-choice", &right))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert!(
        a.store
            .events_after(0)
            .await
            .unwrap()
            .iter()
            .any(|e| e.kind == "transport.keypad")
    );
}
#[tokio::test]
async fn api_requires_operator_auth_and_replay_cannot_dial() {
    let a = app().await;
    let (h, _) = a
        .incoming("CA00000000000000000000000000000001")
        .await
        .unwrap();
    let request = || {
        Request::post("/api/demo/probe")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"session_id":h.id,"contact_id":"sarah"}).to_string(),
            ))
            .unwrap()
    };
    let r = router(a.clone()).oneshot(request()).await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let mut req = request();
    req.headers_mut().insert(
        "authorization",
        "Bearer a-long-local-test-admin-token".parse().unwrap(),
    );
    let r = router(a.clone()).oneshot(req).await.unwrap();
    assert!(!r.status().is_success());
    assert!(a.store.actions(&h.id).await.unwrap().is_empty());
}
