use firstring::{
    calls::actor::{App, Input},
    config::{Config, ExecutionMode},
    providers::nemotron::ModelToolCall,
};
use serde_json::json;
#[tokio::test]
async fn accepted_tool_trace_has_turn_binding_sanitized_arguments_and_actual_result() {
    let mut c = Config::from_env().unwrap();
    c.database_url = "sqlite::memory:".into();
    c.mode = ExecutionMode::Replay;
    let app = App::new(c).await.unwrap();
    let (h, _) = app
        .incoming("CA00000000000000000000000000000011")
        .await
        .unwrap();
    h.send(Input::FinalPrompt {
        text: "Please pass along an appointment reminder.".into(),
    })
    .await
    .unwrap();
    let s = h.send(Input::Snapshot).await.unwrap();
    let arguments = json!({"observation":{"speech_act":"request","requested_object":"routine_information","direction":"caller_requests_from_sarah"},"evidence_refs":[{"turn_id":s["turns"][0]["id"],"quote":"appointment reminder"}]});
    let result = h
        .send(Input::ModelTool {
            generation: s["generation_id"].as_u64().unwrap(),
            epoch: s["policy_epoch"].as_u64().unwrap(),
            version: s["turn_version"].as_u64().unwrap(),
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(2),
            tool: ModelToolCall {
                id: "trace-tool".into(),
                name: "record_assessment".into(),
                arguments,
                assistant_message: json!({}),
            },
        })
        .await
        .unwrap();
    assert_eq!(result["ok"], true);
    let events = app.store.events_after(0).await.unwrap();
    let proposed = events
        .iter()
        .find(|e| e.kind == "agent.tool.proposed")
        .unwrap();
    assert_eq!(proposed.payload["turn_version"], 1);
    assert_eq!(
        proposed.payload["arguments"]["observation"]["requested_object"],
        "routine_information"
    );
    assert!(
        !proposed
            .payload
            .to_string()
            .contains("appointment reminder")
    );
    let outcome = events
        .iter()
        .find(|e| e.kind == "agent.tool.result")
        .unwrap();
    assert_eq!(outcome.payload["ok"], true);
    assert_eq!(outcome.payload["result"]["assessed_version"], 1);
    assert_eq!(outcome.payload["result"]["blocked"], false);
    assert!(outcome.sequence > proposed.sequence);
}
