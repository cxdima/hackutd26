use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use firstring::{
    calls::actor::{App, Input},
    config::{Config, ExecutionMode},
    http::router,
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

fn inbound_config() -> Config {
    let mut c = Config::from_env().unwrap();
    c.database_url = "sqlite::memory:".into();
    c.public_base_url = "https://voice.example.com".into();
    c.account_sid = format!("AC{}", "0".repeat(32));
    c.auth_token = "test-auth-token".into();
    c.admin_token = "a-long-local-test-admin-token".into();
    c.mode = ExecutionMode::Live;
    c.transport = "probe".into();
    c.photon_number.clear();
    c.sip_username.clear();
    c.sip_password.clear();
    c.nvidia_key.clear();
    c.contacts.clear();
    c
}
async fn get(app: App, path: &str) -> (StatusCode, Value) {
    let response = router(app)
        .oneshot(
            Request::get(path)
                .header("authorization", "Bearer a-long-local-test-admin-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn inbound_probe_is_ready_without_outbound_sip_or_model_credentials() {
    let app = App::new(inbound_config()).await.unwrap();
    let (status, body) = get(app.clone(), "/health/ready").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["capabilities"]["inbound"]["configured"], true);
    assert_eq!(body["capabilities"]["outbound"]["configured"], false);
    assert_eq!(body["capabilities"]["model"]["configured"], false);
    assert_eq!(body["live_verified"], false);
    let (_, snapshot) = get(app, "/api/state").await;
    let capabilities = snapshot["capabilities"].as_array().unwrap();
    assert!(
        capabilities
            .iter()
            .any(|c| c["id"] == "inbound" && c["status"] == "configured")
    );
    assert!(
        capabilities
            .iter()
            .any(|c| c["id"] == "outbound" && c["status"] == "pending")
    );
    assert!(!capabilities.iter().any(|c| c["status"] == "verified"));
}

#[tokio::test]
async fn conversation_readiness_requires_model_but_not_outbound_credentials() {
    let mut c = inbound_config();
    c.transport = "relay".into();
    let (status, body) = get(App::new(c.clone()).await.unwrap(), "/health/ready").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["capabilities"]["inbound"]["configured"], true);
    assert_eq!(body["capabilities"]["relay"]["configured"], true);
    assert_eq!(body["capabilities"]["relay"]["live_verified"], false);
    c.nvidia_key = "test-only-key-never-submitted".into();
    let (status, body) = get(App::new(c).await.unwrap(), "/health/ready").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["capabilities"]["outbound"]["configured"], false);
    assert_eq!(body["capabilities"]["model"]["configured"], true);
    assert_eq!(body["live_verified"], false);
}

#[tokio::test]
async fn outbound_probe_keeps_full_setup_guard_before_reserving() {
    let mut c = inbound_config();
    c.contacts.insert("sarah".into(), "+12025550123".into());
    c.smoke_authorized = true;
    let app = App::new(c).await.unwrap();
    let (actor, _) = app
        .incoming(&format!("CA{}", "1".repeat(32)))
        .await
        .unwrap();
    assert!(
        actor
            .send(Input::Probe {
                contact_id: "sarah".into()
            })
            .await
            .is_err()
    );
    assert!(app.store.actions(&actor.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn model_configuration_does_not_claim_inbound_setup_or_verification() {
    let mut c = inbound_config();
    c.auth_token.clear();
    c.nvidia_key = "test-only-key-never-submitted".into();
    let (status, body) = get(App::new(c).await.unwrap(), "/health/ready").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["capabilities"]["inbound"]["configured"], false);
    assert_eq!(body["capabilities"]["model"]["configured"], true);
    assert_eq!(body["capabilities"]["model"]["live_verified"], false);
}
