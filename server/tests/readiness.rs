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

#[tokio::test]
async fn partial_relay_proof_explains_observations_without_passing_the_gate() {
    let app = App::new(inbound_config()).await.unwrap();
    app.store
        .set_metadata(
            "verification.relay_phone",
            &serde_json::json!({
                "status":"partial", "source":"manual_relay_rehearsal",
                "recorded_at":"2026-11-08T03:54:33Z",
                "checks":{"signed_setup":true,"caller_transcript":true,"audible_greeting":true}
            })
            .to_string(),
        )
        .await
        .unwrap();
    let (_, body) = get(app, "/health/ready").await;
    assert_eq!(body["evidence"]["relay_phone"]["status"], "pending");
    assert!(
        body["evidence"]["relay_phone"]["detail"]
            .as_str()
            .unwrap()
            .contains("greeting confirmed")
    );
    assert_eq!(body["capabilities"]["relay"]["live_verified"], false);
}

#[tokio::test]
async fn recorded_ingress_failure_is_visible_without_claiming_relay_worked() {
    let app = App::new(inbound_config()).await.unwrap();
    app.store
        .set_metadata(
            "verification.relay_phone",
            &serde_json::json!({
                "status":"failed", "source":"manual_relay_rehearsal",
                "recorded_at":"2026-11-08T03:44:01Z", "error_code":11200
            })
            .to_string(),
        )
        .await
        .unwrap();
    let (_, body) = get(app, "/health/ready").await;
    assert_eq!(body["evidence"]["relay_phone"]["status"], "failed");
    assert!(
        body["evidence"]["relay_phone"]["detail"]
            .as_str()
            .unwrap()
            .contains("voice webhook")
    );
    assert_eq!(body["capabilities"]["relay"]["live_verified"], false);
}

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

#[tokio::test]
async fn recorded_phone_evidence_is_separate_from_configuration_and_relay_proof() {
    let app = App::new(inbound_config()).await.unwrap();
    app.store.set_metadata("verification.inbound_phone", &serde_json::json!({
        "status":"passed", "recorded_at":"2026-11-08T02:42:49Z",
        "source":"manual_inbound_probe", "checks":{
            "signed_inbound":true,"caller_keypad":true,"audible_greeting":true,"terminal_reconciled":true
        }
    }).to_string()).await.unwrap();
    let (_, body) = get(app.clone(), "/health/ready").await;
    assert_eq!(body["capabilities"]["inbound"]["live_verified"], true);
    assert_eq!(body["capabilities"]["relay"]["live_verified"], false);
    assert_eq!(body["live_verified"], true);
    let (_, state) = get(app, "/api/state").await;
    assert_eq!(state["evidence"]["inbound_phone"]["status"], "passed");
    assert_eq!(state["evidence"]["model_protocol"]["status"], "pending");
}

#[tokio::test]
async fn partial_or_malformed_evidence_cannot_claim_a_verified_phone_call() {
    let app = App::new(inbound_config()).await.unwrap();
    app.store.set_metadata("verification.inbound_phone", r#"{"status":"passed","recorded_at":"2026-11-08T02:42:49Z","source":"manual_inbound_probe","checks":{"signed_inbound":true}}"#).await.unwrap();
    let (_, body) = get(app.clone(), "/api/state").await;
    assert_eq!(body["evidence"]["inbound_phone"]["status"], "pending");
    app.store
        .set_metadata("verification.relay_phone", "invalid")
        .await
        .unwrap();
    let (_, body) = get(app, "/health/ready").await;
    assert_eq!(body["capabilities"]["relay"]["live_verified"], false);
}

#[tokio::test]
async fn phone_b_controls_follow_enrollment_authorization_and_durable_global_budget() {
    let mut c = inbound_config();
    c.photon_number = "+12025550100".into();
    c.sip_username = "test".into();
    c.sip_password = "test".into();
    c.smoke_authorized = true;
    let app = App::new(c.clone()).await.unwrap();
    let (_, body) = get(app, "/api/state").await;
    assert_eq!(body["phone_b"]["enrolled"], false);
    assert_eq!(body["phone_b"]["can_request"], false);
    c.contacts.insert("sarah".into(), "+12025550123".into());
    let app = App::new(c).await.unwrap();
    let (_, body) = get(app.clone(), "/api/state").await;
    assert_eq!(body["phone_b"]["can_request"], true);
    let leg = firstring::calls::state::Leg {
        action_id: "reserved-smoke".into(),
        role: "sarah".into(),
        kind: "probe".into(),
        status: "unknown".into(),
        acknowledged: false,
        call_sid: None,
    };
    app.store
        .reserve("session", "smoke", &leg, ExecutionMode::Live)
        .await
        .unwrap();
    let (_, body) = get(app, "/api/state").await;
    assert_eq!(body["phone_b"]["remaining_attempts"], 0);
    assert_eq!(body["phone_b"]["can_request"], false);
}

#[tokio::test]
async fn recorded_relay_failure_is_visible_without_erasing_inbound_proof() {
    let app = App::new(inbound_config()).await.unwrap();
    app.store.set_metadata("verification.relay_phone", r#"{"status":"failed","source":"manual_relay_rehearsal","recorded_at":"2026-11-08T03:19:13Z","error_code":64105}"#).await.unwrap();
    let (_, body) = get(app, "/api/state").await;
    assert_eq!(body["evidence"]["relay_phone"]["status"], "failed");
    assert!(
        body["evidence"]["relay_phone"]["detail"]
            .as_str()
            .unwrap()
            .contains("64105")
    );
}
