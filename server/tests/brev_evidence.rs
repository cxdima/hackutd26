use firstring::{
    calls::state::Session,
    config::{Config, ExecutionMode, ModelBackend, endpoint_fingerprint},
    evidence::model_backend,
    store::Store,
};
use serde_json::{Value, json};

async fn live_cycle() -> (Config, Store, Value) {
    let mut config = Config::from_env().unwrap();
    config.model_backend = ModelBackend::BrevHosted;
    config.model_url = "http://127.0.0.1:18080/v1/chat/completions".into();
    config.model = "nvidia/nemotron-3.5-lightning-30b-a3b".into();
    let store = Store::open("sqlite::memory:").await.unwrap();
    let session = Session::new(
        "live-session".into(),
        "synthetic-provider-leg".into(),
        "sarah".into(),
        ExecutionMode::Live,
    );
    store.save(&session).await.unwrap();
    sqlx::query("INSERT INTO actions VALUES('fixture-action','audio-fixture','test-key','fixture_caller','live','{}')")
        .execute(&store.pool).await.unwrap();
    store
        .set_metadata(
            "audio-fixture.run.v2:fixture-action",
            &json!({"leg":{"action_id":"fixture-action"},"inbound_sid":session.inbound_sid})
                .to_string(),
        )
        .await
        .unwrap();
    let mut ids = Vec::new();
    for (kind, payload) in [
        (
            "model.request.completed",
            json!({"observation":{"backend":"brev_hosted","outcome":"success","requested_model":config.model,"response_model":config.model,"finish_reason":"stop","content_chars":0}}),
        ),
        (
            "agent.tool.result",
            json!({"name":"record_assessment","ok":true,"result":{"assessed_version":1}}),
        ),
        (
            "model.request.completed",
            json!({"observation":{"backend":"brev_hosted","outcome":"success","requested_model":config.model,"response_model":config.model,"finish_reason":"tool_calls","content_chars":0}}),
        ),
        (
            "agent.tool.result",
            json!({"name":"take_message","ok":true,"result":{"message_recorded":true}}),
        ),
    ] {
        let mut payload = payload;
        payload["generation_id"] = json!(7);
        payload["turn_version"] = json!(1);
        ids.push(
            store
                .append(Some(&session), "system", kind, payload, ExecutionMode::Live)
                .await
                .unwrap()
                .id,
        );
    }
    let receipt = json!({"status":"passed","source":"brev_gpu_live_tool_cycle","recorded_at":"2026-11-08T07:50:00Z","model":config.model,"endpoint_sha256":endpoint_fingerprint(&config.model_url),
        "checks":{"loaded_on_gpu":true,"authenticated":true,"native_assessment":true,"native_tool_results":true,"action_selection":true},
        "live_cycle":{"session_id":session.id,"fixture_action_id":"fixture-action","generation_id":7,"turn_version":1,"request_event_ids":[ids[0],ids[2]],"assessment_result_event_id":ids[1],"action_result_event_id":ids[3],"action_name":"take_message"}});
    (config, store, receipt)
}

async fn verified(config: &Config, store: &Store, receipt: &Value) -> bool {
    store
        .set_metadata("verification.brev_model_protocol", &receipt.to_string())
        .await
        .unwrap();
    model_backend(config, store).await.unwrap()["protocol_verified"] == true
}

#[tokio::test]
async fn reviewed_live_cycle_verifies_without_rewriting_failed_preflight() {
    let (config, store, receipt) = live_cycle().await;
    let failure = json!({"status":"failed","source":"brev_gpu_model_probe","reason":"second-context semantic failure"});
    store
        .set_metadata("verification.brev_preflight", &failure.to_string())
        .await
        .unwrap();
    assert!(verified(&config, &store, &receipt).await);
    assert_eq!(
        store
            .metadata("verification.brev_preflight")
            .await
            .unwrap()
            .unwrap(),
        failure.to_string()
    );
}

#[tokio::test]
async fn a_live_receipt_never_verifies_a_different_model_or_endpoint() {
    let (config, store, receipt) = live_cycle().await;
    assert!(verified(&config, &store, &receipt).await);
    let mut changed = config.clone();
    changed.model = "different-model".into();
    assert!(!verified(&changed, &store, &receipt).await);
    changed = config.clone();
    changed.model_url = "http://127.0.0.1:18081/v1/chat/completions".into();
    assert!(!verified(&changed, &store, &receipt).await);
}

#[tokio::test]
async fn unknown_reordered_and_other_generation_evidence_is_not_a_native_cycle() {
    let (config, store, receipt) = live_cycle().await;
    assert!(verified(&config, &store, &receipt).await);
    for (field, replacement) in [
        ("session_id", json!("unknown")),
        ("fixture_action_id", json!("unknown")),
        ("generation_id", json!(8)),
        ("turn_version", json!(2)),
        (
            "request_event_ids",
            json!([
                receipt["live_cycle"]["request_event_ids"][1],
                receipt["live_cycle"]["request_event_ids"][0]
            ]),
        ),
    ] {
        let mut invalid = receipt.clone();
        invalid["live_cycle"][field] = replacement;
        assert!(!verified(&config, &store, &invalid).await, "{field}");
    }
    sqlx::query("UPDATE events SET data=json_set(data,'$.payload.observation.response_model','unmatched') WHERE json_extract(data,'$.type')='model.request.completed'")
        .execute(&store.pool).await.unwrap();
    assert!(!verified(&config, &store, &receipt).await);
}

#[tokio::test]
async fn replay_hosted_or_failed_requests_cannot_be_imported_as_live_gpu_proof() {
    for (path, replacement) in [
        ("$.execution_mode", json!("replay")),
        ("$.payload.observation.backend", json!("nvidia_hosted")),
        ("$.payload.observation.outcome", json!("timeout")),
        ("$.payload.observation.finish_reason", json!("length")),
    ] {
        let (config, store, receipt) = live_cycle().await;
        sqlx::query("UPDATE events SET data=json_set(data,?,json(?)) WHERE json_extract(data,'$.type')='model.request.completed'")
            .bind(path).bind(replacement.to_string()).execute(&store.pool).await.unwrap();
        assert!(!verified(&config, &store, &receipt).await, "{path}");
    }
}
