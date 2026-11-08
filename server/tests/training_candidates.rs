//! Offline authored-data validation. No optimizer, inference, telephony, or live database.
use firstring::{
    calls::{
        actor::{App, Handle, Input},
        state::Session,
    },
    config::{Config, ExecutionMode, ModelBackend},
    providers::nemotron::{
        ModelCapabilities, ModelToolCall, conversation_messages, parse_response,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct Source {
    examples: Vec<Example>,
}
#[derive(Deserialize)]
struct Example {
    id: String,
    family_id: String,
    split: String,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
struct Step {
    caller_text: String,
    assessment: Value,
    next_tool: AuthoredTool,
}
#[derive(Deserialize)]
struct AuthoredTool {
    name: String,
    arguments: Value,
}

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/training-new")
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(directory().join(name)).unwrap()).unwrap()
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn config() -> Config {
    Config {
        public_base_url: "https://training.invalid".into(),
        port: 0,
        database_url: "sqlite::memory:".into(),
        admin_token: "offline-training-validation-only".into(),
        mode: ExecutionMode::Replay,
        transport: "relay".into(),
        account_sid: String::new(),
        auth_token: String::new(),
        photon_number: String::new(),
        sip_host: "sip.invalid".into(),
        sip_port: 5061,
        sip_username: String::new(),
        sip_password: String::new(),
        contacts: BTreeMap::new(),
        smoke_authorized: false,
        handoff_enabled: false,
        family_enabled: false,
        max_call_seconds: 180,
        nvidia_key: String::new(),
        model_backend: ModelBackend::NvidiaHosted,
        model_url: "http://127.0.0.1:1/disabled".into(),
        model: "offline-authored-validation".into(),
    }
}

fn replace_ids(value: &mut Value, ids: &BTreeMap<String, String>) {
    match value {
        Value::String(text) => {
            if let Some(replacement) = ids.get(text) {
                *text = replacement.clone();
            }
            // Model context and native arguments are encoded JSON strings.
            else if let Ok(mut nested) = serde_json::from_str::<Value>(text)
                && (nested.is_object() || nested.is_array())
            {
                replace_ids(&mut nested, ids);
                *text = nested.to_string();
            }
        }
        Value::Array(items) => {
            for item in items {
                replace_ids(item, ids);
            }
        }
        Value::Object(fields) => {
            for item in fields.values_mut() {
                replace_ids(item, ids);
            }
        }
        _ => {}
    }
}
fn native(id: &str, name: &str, arguments: Value) -> ModelToolCall {
    parse_response(json!({"choices":[{"finish_reason":"tool_calls","message":{
        "role":"assistant","content":null,"tool_calls":[{"id":id,"type":"function","function":{"name":name,"arguments":arguments.to_string()}}]
    }}]})).unwrap()
}
async fn snapshot(handle: &Handle) -> Session {
    serde_json::from_value(handle.send(Input::Snapshot).await.unwrap()).unwrap()
}
async fn execute(handle: &Handle, call: ModelToolCall) -> Value {
    let s = snapshot(handle).await;
    handle
        .send(Input::ModelTool {
            generation: s.generation_id,
            epoch: s.policy_epoch,
            version: s.turn_version,
            deadline: Instant::now() + Duration::from_secs(6),
            tool: call,
        })
        .await
        .unwrap()
}

#[test]
fn family_assignment_was_frozen_before_text_and_is_not_label_stratification() {
    let split = read("family-split.json");
    let freeze = read("split-freeze.json");
    let source = read("authored-trajectories.json");
    assert_eq!(
        freeze["sha256"],
        sha(&std::fs::read(directory().join("family-split.json")).unwrap())
    );
    assert_eq!(freeze["scenario_texts_authored"], false);
    assert!(source["authored_at"].as_str().unwrap() >= freeze["frozen_at"].as_str().unwrap());
    let families: BTreeMap<_, _> = split["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["family_id"].as_str().unwrap(),
                f["split"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(families.len(), 7);
    assert_eq!(families.values().filter(|v| **v == "train").count(), 5);
    assert_eq!(families.values().filter(|v| **v == "dev").count(), 2);
    let examples = source["examples"].as_array().unwrap();
    assert_eq!(examples.len(), 12);
    for example in examples {
        assert_eq!(
            families[example["family_id"].as_str().unwrap()],
            example["split"].as_str().unwrap()
        );
    }
}

#[tokio::test]
async fn authored_native_trajectories_match_actual_replay_policy_and_export_causally() {
    validate_authored_trajectories(false).await;
}

#[tokio::test]
#[ignore = "Archived exports use a prior prompt/schema; training is paused while caller fixes proceed"]
async fn frozen_training_exports_match_current_prompt() {
    validate_authored_trajectories(true).await;
}

async fn validate_authored_trajectories(compare_frozen_exports: bool) {
    let source: Source = serde_json::from_value(read("authored-trajectories.json")).unwrap();
    let labels = read("review-labels.json");
    let mut training = Vec::new();
    let mut development = Vec::new();
    let mut receipts = Vec::new();
    let mut row_index = Vec::new();
    for example in source.examples {
        let expected = &labels["labels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["example_id"] == example.id)
            .unwrap()["expected"];
        let app = App::new(config()).await.unwrap();
        let (handle, _) = app.incoming("not-a-provider-call-id").await.unwrap();
        let mut real_ids = BTreeMap::new();
        let mut stable_ids = BTreeMap::new();
        let mut final_messages = Vec::new();
        let mut step_receipts = Vec::new();
        for (index, step) in example.steps.iter().enumerate() {
            handle
                .send(Input::FinalPrompt {
                    text: step.caller_text.clone(),
                })
                .await
                .unwrap();
            let before = snapshot(&handle).await;
            let real = before.turns.last().unwrap().id.clone();
            let stable = format!("turn-{}", index + 1);
            real_ids.insert(stable.clone(), real.clone());
            stable_ids.insert(real, stable);
            let mut assistant_index = 0;
            for turn in &before.turns {
                if turn.speaker == "assistant" {
                    assistant_index += 1;
                    stable_ids.insert(turn.id.clone(), format!("assistant-{assistant_index}"));
                }
            }
            let caps = ModelCapabilities::from_config(&app.config);
            assert!(
                !caps.handoff_available
                    && !caps.family_callback_available
                    && !caps.family_contact_enrolled
            );
            let mut messages = conversation_messages(&before, &caps);
            let original_context: Value =
                serde_json::from_str(messages[1]["content"].as_str().unwrap()).unwrap();
            assert!(original_context["policy_state"]["understanding"].is_null());
            assert!(
                original_context["policy_state"]["assessed_version"]
                    .as_u64()
                    .unwrap()
                    < original_context["policy_state"]["turn_version"]
                        .as_u64()
                        .unwrap()
            );
            let mut args = step.assessment.clone();
            replace_ids(&mut args, &real_ids);
            let assessment = native("call_assessment", "record_assessment", args);
            let result = execute(&handle, assessment.clone()).await;
            assert_eq!(
                result["ok"],
                true,
                "{} step{} assessment: {result}",
                example.id,
                index + 1
            );
            // Preserve causality: do not rewrite earlier inputs with a future accepted assessment.
            messages.push(assessment.assistant_message);
            messages.push(json!({"role":"tool","tool_call_id":"call_assessment","content":result.to_string()}));
            let mut args = step.next_tool.arguments.clone();
            replace_ids(&mut args, &real_ids);
            let next = native("call_next_action", &step.next_tool.name, args);
            let outcome = execute(&handle, next.clone()).await;
            assert_eq!(
                outcome["ok"],
                true,
                "{} step{} action: {outcome}",
                example.id,
                index + 1
            );
            assert_eq!(outcome["result"]["stop"], true);
            messages.push(next.assistant_message);
            final_messages = messages;
            let after = snapshot(&handle).await;
            step_receipts.push(json!({"caller_turn":index+1,"assessment_result":result,"terminal_tool":step.next_tool.name,"actual_tool_result":outcome,"blocked":after.blocked,"stored_summary":after.message.as_ref().map(|m|m.summary.clone()),"closing_reason":after.closing_reason}));
        }
        let final_state = snapshot(&handle).await;
        assert_eq!(
            final_state.blocked,
            expected["blocked"].as_bool().unwrap(),
            "{} blocked",
            example.id
        );
        assert_eq!(
            example.steps.last().unwrap().next_tool.name,
            expected["terminal_tool"]
        );
        assert_eq!(
            serde_json::to_value(
                final_state
                    .understanding
                    .as_ref()
                    .unwrap()
                    .understanding
                    .readiness
            )
            .unwrap(),
            expected["readiness"]
        );
        assert_eq!(
            json!(final_state.message.as_ref().map(|m| m.summary.clone())),
            expected["saved_summary"],
            "{} actual stored summary",
            example.id
        );
        assert_eq!(
            final_state.closing_reason.is_some(),
            expected["closing_requested"].as_bool().unwrap()
        );
        assert!(
            final_state.legs.is_empty()
                && !final_state.inbound_ended
                && !final_state.bridge_committed
        );
        let actions: i64 = sqlx::query_scalar("SELECT count(*) FROM actions")
            .fetch_one(&app.store.pool)
            .await
            .unwrap();
        assert_eq!(actions, 0);
        assert!(
            app.store
                .events_after(0)
                .await
                .unwrap()
                .iter()
                .all(|event| event.kind != "model.request.completed")
        );
        let mut row = json!({"messages":final_messages,"tools":firstring::tools::schemas(),"parallel_tool_calls":false});
        replace_ids(&mut row, &stable_ids);
        let text = row.to_string();
        assert!(!text.contains(&example.id) && !text.contains(&example.family_id));
        assert!(
            !text.contains("review_status")
                && !text.contains("author_rationale")
                && !text.contains("expected_classification")
        );
        let target = if example.split == "train" {
            &mut training
        } else {
            &mut development
        };
        row_index.push(json!({"example_id":example.id,"family_id":example.family_id,"split":example.split,"line":target.len()+1,"row_sha256":sha(text.as_bytes())}));
        target.push(text);
        let mut receipt = json!({"example_id":example.id,"validation":"actual_replay_actor_with_in_memory_sqlite","steps":step_receipts,"actions_reserved":actions,"inference_requests":0,"phone_calls":0,"delivery_verified":false});
        replace_ids(&mut receipt, &stable_ids);
        receipts.push(receipt);
    }
    assert_eq!((training.len(), development.len()), (10, 2));
    if std::env::var("FIRSTRING_EXPORT_TRAINING_CANDIDATES").as_deref() == Ok("1") {
        std::fs::write(
            directory().join("train.jsonl"),
            format!("{}\n", training.join("\n")),
        )
        .unwrap();
        std::fs::write(
            directory().join("dev.jsonl"),
            format!("{}\n", development.join("\n")),
        )
        .unwrap();
        std::fs::write(directory().join("validation-receipts.json"),serde_json::to_string_pretty(&json!({"optimizer_steps":0,"inference_requests":0,"phone_calls":0,"receipts":receipts})).unwrap()+"\n").unwrap();
        std::fs::write(
            directory().join("row-index.json"),
            serde_json::to_string_pretty(&json!({"rows":row_index})).unwrap() + "\n",
        )
        .unwrap();
    } else if compare_frozen_exports {
        for (name, rows) in [("train.jsonl", training), ("dev.jsonl", development)] {
            assert_eq!(
                sha(&std::fs::read(directory().join(name)).unwrap()),
                sha(format!("{}\n", rows.join("\n")).as_bytes()),
                "{name} is archived against a prior prompt/schema; do not regenerate without review"
            );
        }
    }
}

#[tokio::test]
async fn authored_example_cannot_bypass_evidence_or_blocked_question_gates() {
    let source: Source = serde_json::from_value(read("authored-trajectories.json")).unwrap();
    let example = source.examples.iter().find(|e| e.id == "fresh-09").unwrap();
    let app = App::new(config()).await.unwrap();
    let (handle, _) = app.incoming("not-provider-id").await.unwrap();
    handle
        .send(Input::FinalPrompt {
            text: example.steps[0].caller_text.clone(),
        })
        .await
        .unwrap();
    let s = snapshot(&handle).await;
    let turn = s.turns.last().unwrap().id.clone();
    let mut args = example.steps[0].assessment.clone();
    replace_ids(
        &mut args,
        &BTreeMap::from([("turn-1".into(), turn.clone())]),
    );
    let mut invalid = args.clone();
    invalid["evidence_refs"][0]["quote"] = json!("Invented caller statement");
    assert_eq!(
        execute(&handle, native("invalid", "record_assessment", invalid)).await["ok"],
        false
    );
    assert_eq!(
        execute(&handle, native("valid", "record_assessment", args)).await["ok"],
        true
    );
    let ask = native(
        "invalid-next",
        "ask_caller",
        json!({"question":"Could you tell me more?","purpose":"clarify_request","evidence_refs":[{"turn_id":turn,"quote":example.steps[0].caller_text}]}),
    );
    assert_eq!(execute(&handle, ask).await["ok"], false);
    assert!(snapshot(&handle).await.blocked);
}
