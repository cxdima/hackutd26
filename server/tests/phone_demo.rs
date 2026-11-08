use firstring::{
    calls::state::{Leg, Session},
    config::{Config, ExecutionMode},
    phone_demo::{self, ArmRequest},
    store::Store,
};
use serde_json::json;
fn config() -> Config {
    let mut c = Config::from_env().unwrap();
    c.mode = ExecutionMode::Live;
    c.admin_token = "test-admin-arming-key".into();
    c.public_base_url = "https://fixture.example".into();
    c.account_sid = "AC00000000000000000000000000000000".into();
    c.auth_token = "fixture-auth".into();
    c.photon_number = "+12025550100".into();
    c.sip_username = "fixture-user".into();
    c.sip_password = "fixture-password".into();
    c.contacts.insert("sarah".into(), "+12025550101".into());
    c.handoff_enabled = false;
    c.smoke_authorized = false;
    c.family_enabled = false;
    c
}
fn call(id: &str) -> Session {
    Session::new(
        id.into(),
        format!("CA{id}"),
        "sarah".into(),
        ExecutionMode::Live,
    )
}
fn leg(id: &str) -> Leg {
    Leg {
        action_id: id.into(),
        role: "sarah".into(),
        kind: "handoff".into(),
        status: "reserved".into(),
        acknowledged: false,
        call_sid: None,
    }
}
fn arm_request(session: Option<&str>) -> ArmRequest {
    ArmRequest {
        contact_id: "sarah".into(),
        session_id: session.map(str::to_owned),
        ttl_seconds: Some(300),
    }
}
#[tokio::test]
async fn handoff_grant_is_bound_one_use_atomic_and_does_not_spend_probe() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let c = config();
    let s = call("first");
    store.save(&s).await.unwrap();
    assert!(!phone_demo::available_for(&store, &c, &s).await.unwrap());
    let armed = phone_demo::arm(&store, &c, arm_request(Some(&s.id)))
        .await
        .unwrap();
    assert_eq!(armed.state, "armed");
    assert_eq!(armed.remaining_attempts, 1);
    assert!(phone_demo::available_for(&store, &c, &s).await.unwrap());
    assert!(
        !phone_demo::available_for(&store, &c, &call("other"))
            .await
            .unwrap()
    );
    let (first_leg, second_leg) = (leg("a"), leg("b"));
    let (a, b) = tokio::join!(
        phone_demo::reserve_handoff(&store, &c, &s, "first:handoff:0", &first_leg),
        phone_demo::reserve_handoff(&store, &c, &s, "first:handoff:0", &second_leg)
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.1, b.1);
    assert_eq!(a.0.action_id, b.0.action_id);
    assert_eq!(store.actions(&s.id).await.unwrap().len(), 1);
    assert!(!phone_demo::available_for(&store, &c, &s).await.unwrap());
    assert_eq!(
        phone_demo::status(&store, &c)
            .await
            .unwrap()
            .remaining_attempts,
        0
    );
    assert!(
        phone_demo::arm(&store, &c, arm_request(Some(&s.id)))
            .await
            .is_err()
    );
    assert!(
        phone_demo::reserve_handoff(&store, &c, &s, "first:handoff:1", &leg("new"))
            .await
            .is_err()
    );
    let mut probe = leg("probe");
    probe.kind = "probe".into();
    assert!(
        store
            .reserve(&s.id, "first:probe", &probe, ExecutionMode::Live)
            .await
            .unwrap()
            .1
    );
}
#[tokio::test]
async fn next_signed_session_binding_is_single_and_expiry_disarm_cannot_authorize() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let c = config();
    let armed = phone_demo::arm(&store, &c, arm_request(None))
        .await
        .unwrap();
    assert!(
        phone_demo::awaiting_signed_inbound(&store, &c)
            .await
            .unwrap()
    );
    let mut replay = call("replay");
    replay.execution_mode = ExecutionMode::Replay;
    assert!(
        !phone_demo::bind_signed_inbound(&store, &c, &replay)
            .await
            .unwrap()
    );
    let wrong = Session::new(
        "wrong".into(),
        "CAwrong".into(),
        "maya".into(),
        ExecutionMode::Live,
    );
    assert!(
        !phone_demo::bind_signed_inbound(&store, &c, &wrong)
            .await
            .unwrap()
    );
    let first = call("first");
    store.save(&first).await.unwrap();
    store
        .append(
            Some(&first),
            "caller",
            "call.inbound.received",
            json!({}),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    assert!(
        phone_demo::bind_signed_inbound(&store, &c, &first)
            .await
            .unwrap()
    );
    assert!(
        !phone_demo::bind_signed_inbound(&store, &c, &call("second"))
            .await
            .unwrap()
    );
    assert!(
        !phone_demo::awaiting_signed_inbound(&store, &c)
            .await
            .unwrap()
    );
    assert!(
        phone_demo::disarm(&store, &c, "obsolete-arm")
            .await
            .is_err()
    );
    phone_demo::disarm(&store, &c, armed.arm_id.as_deref().unwrap())
        .await
        .unwrap();
    assert!(!phone_demo::available_for(&store, &c, &first).await.unwrap());
    let next = phone_demo::arm(&store, &c, arm_request(None))
        .await
        .unwrap();
    assert_ne!(next.arm_id, armed.arm_id);
    let raw = store.metadata("phone_demo.v1").await.unwrap().unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["expires_at"] = json!("2000-01-01T00:00:00Z");
    store
        .set_metadata("phone_demo.v1", &value.to_string())
        .await
        .unwrap();
    assert!(
        !phone_demo::bind_signed_inbound(&store, &c, &call("late"))
            .await
            .unwrap()
    );
    assert_eq!(
        phone_demo::status(&store, &c).await.unwrap().state,
        "expired"
    );
}
#[tokio::test]
async fn replay_missing_or_changed_recipient_and_extra_payload_are_rejected() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let c = config();
    let s = call("first");
    store.save(&s).await.unwrap();
    let mut replay = c.clone();
    replay.mode = ExecutionMode::Replay;
    assert!(
        phone_demo::arm(&store, &replay, arm_request(None))
            .await
            .is_err()
    );
    let mut missing = c.clone();
    missing.contacts.remove("sarah");
    assert!(
        phone_demo::arm(&store, &missing, arm_request(None))
            .await
            .is_err()
    );
    assert!(
        serde_json::from_value::<ArmRequest>(
            json!({"contact_id":"sarah","url":"https://evil.test"})
        )
        .is_err()
    );
    phone_demo::arm(&store, &c, arm_request(Some(&s.id)))
        .await
        .unwrap();
    let mut changed = c.clone();
    changed
        .contacts
        .insert("sarah".into(), "+12025550199".into());
    assert!(
        !phone_demo::available_for(&store, &changed, &s)
            .await
            .unwrap()
    );
    assert!(
        phone_demo::reserve_handoff(&store, &changed, &s, "first:handoff", &leg("changed"))
            .await
            .is_err()
    );
    assert!(store.actions(&s.id).await.unwrap().is_empty());
}
#[tokio::test]
async fn failed_reservation_does_not_consume_arm_and_restart_retains_it() {
    let path = std::env::temp_dir().join(format!("firstring-arm-{}.db", uuid::Uuid::new_v4()));
    let url = format!("sqlite://{}", path.display());
    let store = Store::open(&url).await.unwrap();
    let c = config();
    let s = call("first");
    store.save(&s).await.unwrap();
    phone_demo::arm(&store, &c, arm_request(Some(&s.id)))
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER reject_handoff BEFORE INSERT ON actions BEGIN SELECT RAISE(FAIL,'fixture write failure'); END").execute(&store.pool).await.unwrap();
    assert!(
        phone_demo::reserve_handoff(&store, &c, &s, "first:handoff", &leg("failed"))
            .await
            .is_err()
    );
    assert!(phone_demo::available_for(&store, &c, &s).await.unwrap());
    store.pool.close().await;
    drop(store);
    let reopened = Store::open(&url).await.unwrap();
    assert_eq!(
        phone_demo::status(&reopened, &c).await.unwrap().state,
        "armed"
    );
    reopened.pool.close().await;
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn operator_api_arms_and_disarms_without_dialing_or_exposing_recipient() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use firstring::{calls::actor::App, http::router};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let mut c = config();
    c.database_url = "sqlite::memory:".into();
    c.nvidia_key.clear();
    let app = App::new(c.clone()).await.unwrap();
    let body = json!({"contact_id":"sarah"});
    let unauthorized = router(app.clone())
        .oneshot(
            Request::post("/api/demo/phone-demo/arm")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    let arm = router(app.clone())
        .oneshot(
            Request::post("/api/demo/phone-demo/arm")
                .header("authorization", format!("Bearer {}", c.admin_token))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(arm.status(), StatusCode::OK);
    let raw = arm.into_body().collect().await.unwrap().to_bytes();
    let arm: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(arm["state"], "armed");
    assert_eq!(arm["probe_budget_unchanged"], true);
    assert!(!String::from_utf8_lossy(&raw).contains("12025550101"));
    assert!(arm.get("recipient_fingerprint").is_none());
    let state = router(app.clone())
        .oneshot(
            Request::get("/api/state")
                .header("authorization", format!("Bearer {}", c.admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&state.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(state["phone_demo"]["arm_id"], arm["arm_id"]);
    let disarm = router(app.clone())
        .oneshot(
            Request::post("/api/demo/phone-demo/disarm")
                .header("authorization", format!("Bearer {}", c.admin_token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"arm_id":arm["arm_id"]}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disarm.status(), StatusCode::OK);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM actions")
        .fetch_one(&app.store.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn legacy_global_flag_cannot_authorize_an_unarmed_actor_handoff() {
    use firstring::{
        calls::actor::{App, Input},
        providers::nemotron::ModelToolCall,
    };
    let mut c = config();
    c.database_url = "sqlite::memory:".into();
    c.nvidia_key.clear();
    c.handoff_enabled = true;
    let app = App::new(c).await.unwrap();
    let (handle, _) = app
        .incoming("CA00000000000000000000000000000001")
        .await
        .unwrap();
    let text =
        "Sarah's appointment is Monday at two. I would like to discuss transportation with her.";
    let turn = handle
        .send(Input::FinalPrompt { text: text.into() })
        .await
        .unwrap();
    let refs = json!([{"turn_id":turn["turn_id"],"quote":text}]);
    let observation = json!({"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"});
    for (name, arguments) in [
        (
            "record_assessment",
            json!({"observation":observation,"evidence_refs":refs,"understanding":{"version":1,"claimed_identity":null,"purpose":"Discuss transportation","requested_action":"Speak with Sarah","supplied_facts":[],"missing_information":[],"readiness":"ready"}}),
        ),
        (
            "offer_handoff",
            json!({"summary":"Caller asks Sarah about transportation for Monday's appointment.","observation":observation,"evidence_refs":refs}),
        ),
    ] {
        let snapshot = handle.send(Input::Snapshot).await.unwrap();
        let result = handle
            .send(Input::ModelTool {
                generation: snapshot["generation_id"].as_u64().unwrap(),
                epoch: snapshot["policy_epoch"].as_u64().unwrap(),
                version: snapshot["turn_version"].as_u64().unwrap(),
                deadline: std::time::Instant::now() + std::time::Duration::from_secs(6),
                tool: ModelToolCall {
                    id: name.into(),
                    name: name.into(),
                    arguments,
                    assistant_message: json!({"role":"assistant","content":null}),
                },
            })
            .await
            .unwrap();
        assert_eq!(result["ok"], name == "record_assessment");
        if name == "offer_handoff" {
            assert!(result["error"].as_str().unwrap().contains("not armed"));
        }
    }
    assert!(app.store.actions(&handle.id).await.unwrap().is_empty());
    assert!(
        handle.send(Input::Snapshot).await.unwrap()["legs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn next_call_arm_rejects_duplicate_old_ingress_and_legacy_missing_boundary() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let c = config();
    let old = call("older");
    store.save(&old).await.unwrap();
    store
        .append(
            Some(&old),
            "caller",
            "call.inbound.received",
            json!({}),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    phone_demo::arm(&store, &c, arm_request(None))
        .await
        .unwrap();
    // Even another receipt for an old session must not replace its first inbound.
    store
        .append(
            Some(&old),
            "caller",
            "call.inbound.received",
            json!({}),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    // A retry of the same signed webhook reuses this old session and its receipt.
    assert!(
        !phone_demo::bind_signed_inbound(&store, &c, &old)
            .await
            .unwrap()
    );
    assert!(!phone_demo::available_for(&store, &c, &old).await.unwrap());
    let fresh = call("fresh");
    store.save(&fresh).await.unwrap();
    store
        .append(
            Some(&fresh),
            "caller",
            "call.inbound.received",
            json!({}),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    assert!(
        phone_demo::bind_signed_inbound(&store, &c, &fresh)
            .await
            .unwrap()
    );
    assert!(phone_demo::available_for(&store, &c, &fresh).await.unwrap());
    let mut raw: serde_json::Value =
        serde_json::from_str(&store.metadata("phone_demo.v1").await.unwrap().unwrap()).unwrap();
    raw.as_object_mut().unwrap().remove("bind_after_sequence");
    store
        .set_metadata("phone_demo.v1", &raw.to_string())
        .await
        .unwrap();
    assert!(!phone_demo::available_for(&store, &c, &fresh).await.unwrap());
    raw["session_id"] = serde_json::Value::Null;
    store
        .set_metadata("phone_demo.v1", &raw.to_string())
        .await
        .unwrap();
    assert!(
        !phone_demo::awaiting_signed_inbound(&store, &c)
            .await
            .unwrap()
    );
    assert!(
        !phone_demo::bind_signed_inbound(&store, &c, &fresh)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn explicit_active_session_arm_is_not_restricted_to_future_inbound_receipts() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let c = config();
    let old = call("existing");
    store.save(&old).await.unwrap();
    store
        .append(
            Some(&old),
            "caller",
            "call.inbound.received",
            json!({}),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    phone_demo::arm(&store, &c, arm_request(Some(&old.id)))
        .await
        .unwrap();
    assert!(phone_demo::available_for(&store, &c, &old).await.unwrap());
}
