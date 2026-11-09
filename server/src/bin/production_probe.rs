//! Non-dialing reconstruction of synthetic phone contexts through runtime builders and actor.
use anyhow::{Result, ensure};
use firstring::{
    calls::{
        actor::{App, Handle, Input},
        state::{Session, Turn},
    },
    config::{Config, ExecutionMode, ModelSettings},
    providers::nemotron::{
        ModelCapabilities, Nemotron, ToolPhase, append_tool_result, conversation_messages,
        rejected_assessment_for_repair,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
#[derive(Deserialize)]
struct Dataset {
    schema_version: u32,
    data_origin: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    household_id: String,
    profile: Option<firstring::profile::Profile>,
    turns: Vec<Turn>,
    turn_version: u64,
    generation_id: u64,
    policy_epoch: u64,
    expected_action: String,
    expected_blocked: bool,
    semantic_review: String,
}
/// Record that this deployment just completed real native tool cycles, so the
/// console's current-availability check reflects a fresh, passing conversation
/// against exactly the endpoint and model the live server is configured with.
async fn record_current_model_proof(
    probe_url: &str,
    probe_backend: firstring::config::ModelBackend,
    model: &Nemotron,
    receipt: &str,
) -> Result<()> {
    let live = Config::from_env()?;
    let deployment = std::env::var("NEMOTRON_DEPLOYMENT_ID").unwrap_or_default();
    ensure!(
        !deployment.is_empty(),
        "NEMOTRON_DEPLOYMENT_ID is required to record a current-model proof"
    );
    ensure!(
        live.model_url == probe_url
            && live.model_backend == probe_backend
            && live.model == model.requested_model(),
        "The probe did not exercise the live server's configured model endpoint"
    );
    let store = firstring::store::Store::open(&live.database_url).await?;
    let proof = json!({"deployment_id":deployment,"backend":live.model_backend,"model":live.model,"endpoint_sha256":firstring::config::endpoint_fingerprint(&live.model_url),"status":"passed","native_cycle":true,"recorded_at":chrono::Utc::now(),"receipt":receipt});
    store
        .set_metadata("verification.current_model", &proof.to_string())
        .await?;
    println!(
        "{}",
        json!({"recorded":"verification.current_model","deployment_id":proof["deployment_id"],"receipt":receipt})
    );
    Ok(())
}
fn hash(value: &Value) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2 && ["--capture", "--run", "--conversation"].contains(&args[0].as_str()),
        "Usage: production_probe --capture|--run|--conversation OUTPUT.json"
    );
    ensure!(
        !std::path::Path::new(&args[1]).exists(),
        "Do not overwrite prior diagnostic evidence"
    );
    let raw = std::fs::read_to_string("fixtures/production-context-development.json")?;
    let data: Dataset = serde_json::from_str(&raw)?;
    ensure!(
        data.schema_version == 1 && data.data_origin == "synthetic" && data.cases.len() <= 12,
        "Only bounded synthetic development contexts"
    );
    let conversation = args[0] == "--conversation";
    let run = args[0] != "--capture";
    let settings = ModelSettings::from_env()?;
    let max = std::env::var("PRODUCTION_DIAGNOSTIC_MAX_TOKENS")
        .unwrap_or("512".into())
        .parse::<u32>()?;
    let timeout = std::env::var("PRODUCTION_DIAGNOSTIC_TIMEOUT_MS")
        .unwrap_or("3500".into())
        .parse::<u64>()?;
    ensure!(
        !conversation || (max == 512 && timeout == 3500),
        "Conversation mode uses production limits: 512 tokens, 3500ms request and 6000ms per turn"
    );
    let probe_url = settings.url.clone();
    let probe_backend = settings.backend;
    let model = Nemotron::new(settings.key, settings.url, settings.model)
        .with_backend(settings.backend)
        .with_max_output_tokens(max)?
        .with_request_timeout(Duration::from_millis(timeout))?
        .with_synthetic_probe_diagnostics();
    let mut config = Config::from_env()?;
    config.mode = ExecutionMode::Replay;
    config.database_url = "sqlite::memory:".into();
    config.nvidia_key.clear();
    config.contacts.clear();
    config.handoff_enabled = false;
    config.family_enabled = false;
    config.smoke_authorized = false;
    let capabilities = ModelCapabilities::from_config(&config);
    let app = App::new(config).await?;
    if conversation {
        let reports = tokio::time::timeout(
            Duration::from_secs(120),
            run_conversation(&app, &model, &data, &capabilities),
        )
        .await??;
        let report = json!({"kind":"synthetic_consecutive_native_conversation","recorded_at":chrono::Utc::now(),"backend":settings.backend,"requested_model":model.requested_model(),"source_fixture_sha256":format!("{:x}",Sha256::digest(raw.as_bytes())),"execution":"real_inference_with_one_inert_runtime_actor","phone_calls":0,"request_timeout_ms":3500,"turn_timeout_ms":6000,"max_output_tokens":512,"max_requests":8,"diagnostic_override":false,"cases":reports});
        std::fs::write(&args[1], serde_json::to_string_pretty(&report)? + "\n")?;
        println!(
            "{}",
            json!({"receipt":args[1],"cases":reports.len(),"passed":reports.iter().filter(|r|r["passed"]==true).count(),"phone_calls":0})
        );
        ensure!(
            reports.len() == 2 && reports.iter().all(|r| r["passed"] == true),
            "Consecutive conversation failed; inspect preserved receipt"
        );
        if std::env::var("RECORD_CURRENT_MODEL_PROOF").as_deref() == Ok("1") {
            record_current_model_proof(&probe_url, probe_backend, &model, &args[1]).await?;
        }
        return Ok(());
    }
    let mut reports = Vec::new();
    let campaign = Instant::now();
    for case in data.cases {
        if let Ok(filter) = std::env::var("PRODUCTION_DIAGNOSTIC_CASE_PREFIX")
            && !case.id.starts_with(&filter)
        {
            continue;
        }
        ensure!(
            campaign.elapsed() < Duration::from_secs(120),
            "Diagnostic overall deadline reached"
        );
        let s = initial_session(&case);
        let handle = app.restore_synthetic_context(s.clone()).await?;
        let result = run_turn(
            &model,
            &handle,
            &s,
            &capabilities,
            &case,
            run,
            if case.id.starts_with("attempt19") {
                "faithful_pre_assessment_context"
            } else {
                "semantic_prefix_not_phone_lifecycle"
            },
        )
        .await?;
        reports.push(result);
        let report = json!({"kind":"synthetic_production_context_reconstruction","recorded_at":chrono::Utc::now(),"backend":settings.backend,"requested_model":model.requested_model(),"source_fixture_sha256":format!("{:x}",Sha256::digest(raw.as_bytes())),"execution":"real_inference_with_inert_runtime_actor","phone_calls":0,"request_timeout_ms":timeout,"max_output_tokens":max,"diagnostic_override":max!=512||timeout!=3500,"cases":reports});
        std::fs::write(&args[1], serde_json::to_string_pretty(&report)? + "\n")?;
    }
    ensure!(!reports.is_empty(), "No matching development cases");
    println!(
        "{}",
        json!({"receipt":args[1],"cases":reports.len(),"passed":reports.iter().filter(|r|r["passed"]==true).count(),"phone_calls":0})
    );
    ensure!(
        !run || reports.iter().all(|r| r["passed"] == true),
        "Production-context cases failed; inspect receipt"
    );
    Ok(())
}

fn initial_session(case: &Case) -> Session {
    let mut s = Session::new(
        case.id.clone(),
        format!("CA{}", uuid::Uuid::new_v4().simple()),
        case.household_id.clone(),
        ExecutionMode::Replay,
    );
    s.profile = case.profile.clone();
    s.turns = case.turns.clone();
    s.turn_version = case.turn_version;
    s.generation_id = case.generation_id;
    s.policy_epoch = case.policy_epoch;
    s
}
async fn run_turn(
    model: &Nemotron,
    handle: &Handle,
    s: &Session,
    capabilities: &ModelCapabilities,
    case: &Case,
    run: bool,
    lifecycle_scope: &str,
) -> Result<Value> {
    let mut messages = conversation_messages(s, capabilities);
    let first = model.request_payload(&messages, ToolPhase::Assessment);
    let mut result = json!({"id":case.id,"request_sha256":hash(&first),"initial_request":first,"expected_action":case.expected_action,"expected_blocked":case.expected_blocked,"trace":[],"passed":false,"pass_scope":"native_control_flow_and_risk_flag_only","semantic_review_status":"pending_independent_review","semantic_review_rubric":case.semantic_review,"lifecycle_scope":lifecycle_scope,"error":null});
    if run {
        let started = Instant::now();
        let deadline = started + Duration::from_secs(6);
        let mut phase = ToolPhase::Assessment;
        let mut epoch = s.policy_epoch;
        let mut repaired = false;
        let mut last_denial: Option<String> = None;
        let outcome = tokio::time::timeout(Duration::from_secs(6), async {
            for _ in 0..4 {
                let observed = model.next_tool_observed_for(&messages, phase).await;
                let mut receipt = json!({"phase":phase,"observation":observed.observation});
                if observed.result.is_err() {
                    // This executable accepts only the bounded synthetic dataset.
                    // Preserve the exact failed continuation, including native IDs,
                    // for decoder diagnosis without reconstructing it by hand.
                    receipt["failed_synthetic_request"] = model.request_payload(&messages, phase);
                }
                result["trace"].as_array_mut().unwrap().push(receipt);
                let tool = match observed.result {
                    Ok(tool) => tool,
                    Err(error) => {
                        if let Some((call, denial)) =
                            rejected_assessment_for_repair(&error, phase, epoch, &mut repaired)
                        {
                            append_tool_result(&mut messages, &call, &denial);
                            continue;
                        }
                        return Err(error);
                    }
                };
                let outcome = handle
                    .send(Input::ModelTool {
                        generation: s.generation_id,
                        epoch,
                        version: s.turn_version,
                        deadline,
                        tool: tool.clone(),
                    })
                    .await?;
                let trace = result["trace"].as_array_mut().unwrap().last_mut().unwrap();
                trace["tool"] = json!({"id":tool.id,"name":tool.name,"arguments":tool.arguments});
                trace["rust_result"] = outcome.clone();
                append_tool_result(&mut messages, &tool, &outcome);
                epoch = outcome["epoch"].as_u64().unwrap_or(epoch);
                let snapshot: Session =
                    serde_json::from_value(handle.send(Input::Snapshot).await?)?;
                if outcome["ok"] == true && outcome["result"]["stop"] != true {
                    phase = serde_json::from_value(outcome["next_phase"].clone())?;
                }
                if outcome["ok"] == false {
                    let reason = outcome["error"].as_str().unwrap_or("Native tool denied");
                    ensure!(
                        last_denial.as_deref() != Some(reason),
                        "Repeated native validation denial after one repair attempt: {reason}"
                    );
                    last_denial = Some(reason.to_string());
                } else {
                    last_denial = None;
                }
                if outcome["ok"] == true && outcome["result"]["stop"] == true {
                    result["terminal_tool"] = json!(tool.name);
                    result["saved_message"] = json!(snapshot.message);
                    result["blocked"] = json!(snapshot.blocked);
                    result["passed"] = json!(
                        tool.name == case.expected_action
                            && snapshot.blocked == case.expected_blocked
                    );
                    return Ok(());
                }
            }
            anyhow::bail!("No accepted terminal action in four steps")
        })
        .await
        .unwrap_or_else(|_| {
            Err(anyhow::anyhow!(
                "Production six-second turn deadline elapsed"
            ))
        });
        result["complete_turn_ms"] = json!(started.elapsed().as_millis());
        result["within_production_turn_budget"] = json!(started.elapsed() < Duration::from_secs(6));
        result["error"] = json!(outcome.err().map(|error| error.to_string()));
        if result["error"].is_string() {
            result["passed"] = json!(false);
        }
    }
    Ok(result)
}
async fn run_conversation(
    app: &App,
    model: &Nemotron,
    data: &Dataset,
    capabilities: &ModelCapabilities,
) -> Result<Vec<Value>> {
    let first = data
        .cases
        .iter()
        .find(|case| case.id == "attempt19-turn1")
        .ok_or_else(|| anyhow::anyhow!("Missing frozen initial context"))?;
    let second = data
        .cases
        .iter()
        .find(|case| case.id == "attempt19-turn2")
        .ok_or_else(|| anyhow::anyhow!("Missing frozen risk utterance"))?;
    ensure!(
        first.household_id == second.household_id,
        "Conversation household changed"
    );
    let s = initial_session(first);
    let handle = app.restore_synthetic_context(s.clone()).await?;
    let mut initial = run_turn(
        model,
        &handle,
        &s,
        capabilities,
        first,
        true,
        "consecutive_actual_actor_turns",
    )
    .await?;
    initial["conversation_index"] = json!(1);
    // A failed/closed first turn cannot be made into a successful conversation
    // by injecting an invented question or clearing the actor's closing state.
    if initial["passed"] != true || initial["terminal_tool"] != "ask_caller" {
        initial["conversation_stop"] = json!("first_turn_did_not_request_caller_reply");
        return Ok(vec![initial]);
    }
    let text = second
        .turns
        .iter()
        .rev()
        .find(|turn| turn.speaker == "caller")
        .ok_or_else(|| anyhow::anyhow!("Missing next caller words"))?
        .text
        .clone();
    let commit = handle.send(Input::FinalPrompt { text }).await;
    let committed = match commit {
        Ok(value) => value,
        Err(error) => {
            initial["conversation_stop"] =
                json!(format!("Next caller turn was not accepted: {error}"));
            initial["passed"] = json!(false);
            return Ok(vec![initial]);
        }
    };
    let current: Session = serde_json::from_value(handle.send(Input::Snapshot).await?)?;
    if committed["turn_id"].as_str().is_none_or(str::is_empty)
        || current.turn_version != s.turn_version + 1
    {
        initial["conversation_stop"] = json!("actor_did_not_commit_a_fresh_caller_turn");
        initial["passed"] = json!(false);
        return Ok(vec![initial]);
    }
    // Fresh runtime messages include the assistant words actually committed by
    // AskCaller. The frozen second case's earlier assistant text is never used.
    let mut next = run_turn(
        model,
        &handle,
        &current,
        capabilities,
        second,
        true,
        "consecutive_actual_actor_turns",
    )
    .await?;
    next["conversation_index"] = json!(2);
    next["continuation_source"] = json!("actor_committed_assistant_reply_and_new_final_prompt");
    Ok(vec![initial, next])
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, extract::State, routing::post};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::sync::Mutex;
    const QUESTION: &str = "At which entrance and what time should Sarah pick you up?";
    #[derive(Clone)]
    struct LocalProvider {
        requests: Arc<Mutex<Vec<Value>>>,
        count: Arc<AtomicUsize>,
        close_first: bool,
    }
    async fn respond(State(p): State<LocalProvider>, Json(request): Json<Value>) -> Json<Value> {
        let n = p.count.fetch_add(1, Ordering::SeqCst);
        p.requests.lock().await.push(request.clone());
        let context: Value =
            serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
        let turn = context["turns"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .find(|t| t["speaker"] == "caller")
            .unwrap();
        let second = context["policy_state"]["turn_version"] == 2;
        let assessment = request["tools"][0]["function"]["name"] == "record_assessment";
        let refs = json!([{"turn_id":turn["id"],"quote":turn["text"]}]);
        let (name, args) = if assessment {
            let ready = second || p.close_first;
            (
                "record_assessment",
                json!({"observation":{"speech_act":if second{"request"}else{"statement"},"requested_object":if second{"authentication_secret"}else{"routine_information"},"direction":if second{"caller_requests_from_sarah"}else{"caller_supplies"}},"evidence_refs":refs,"understanding":{"version":1,"claimed_identity":null,"purpose":if second{"Obtain a sign-in code"}else{"Arrange caller pickup"},"requested_action":if second{"Ask Sarah for a code"}else{"Ask Sarah to pick caller up"},"supplied_facts":[{"turn_id":turn["id"],"quote":turn["text"]}],"missing_information":if ready{json!([])}else{json!(["Pickup location and time"])},"readiness":if ready{"ready"}else{"needs_information"}}}),
            )
        } else if second || p.close_first {
            (
                "take_message",
                json!({"summary":if second{"Suspicious request blocked; independent verification recommended."}else{"Caller requests pickup."},"evidence_refs":refs}),
            )
        } else {
            (
                "ask_caller",
                json!({"question":QUESTION,"purpose":"clarify_request","evidence_refs":refs}),
            )
        };
        Json(
            json!({"model":"local-probe-test","choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":format!("native_{n}"),"type":"function","function":{"name":name,"arguments":args.to_string()}}]}}]}),
        )
    }
    async fn local(
        close_first: bool,
    ) -> (App, Nemotron, LocalProvider, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chat", listener.local_addr().unwrap());
        let state = LocalProvider {
            requests: Arc::new(Mutex::new(vec![])),
            count: Arc::new(AtomicUsize::new(0)),
            close_first,
        };
        let server = state.clone();
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/chat", post(respond))
                    .with_state(server),
            )
            .await
            .unwrap();
        });
        let mut c = Config::from_env().unwrap();
        c.mode = ExecutionMode::Replay;
        c.database_url = "sqlite::memory:".into();
        c.nvidia_key.clear();
        c.account_sid.clear();
        c.auth_token.clear();
        c.contacts.clear();
        c.handoff_enabled = false;
        c.family_enabled = false;
        c.smoke_authorized = false;
        (
            App::new(c).await.unwrap(),
            Nemotron::new("local-only".into(), url, "local-probe-test".into()),
            state,
            task,
        )
    }
    fn dataset() -> Dataset {
        serde_json::from_str(include_str!(
            "../../../fixtures/production-context-development.json"
        ))
        .unwrap()
    }
    #[tokio::test]
    async fn conversation_uses_actual_question_and_fresh_actor_turn_without_gold_or_dialing() {
        let (app, model, provider, task) = local(false).await;
        let data = dataset();
        let cases = run_conversation(
            &app,
            &model,
            &data,
            &ModelCapabilities::from_config(&app.config),
        )
        .await
        .unwrap();
        assert_eq!(cases.len(), 2);
        assert!(cases.iter().all(|c| c["passed"] == true), "{cases:?}");
        let requests = provider.requests.lock().await;
        assert_eq!(requests.len(), 4);
        let second = &requests[2];
        let text = second["messages"].to_string();
        assert!(text.contains(QUESTION));
        assert!(!text.contains("Sorry, I didn't catch that."));
        assert!(!text.contains("expected_action"));
        assert!(!text.contains("semantic_review"));
        assert!(!text.contains("attempt19-turn2"));
        assert!(!text.contains(&data.cases[1].semantic_review));
        let context: Value =
            serde_json::from_str(second["messages"][1]["content"].as_str().unwrap()).unwrap();
        let caller = context["current_exchange"]["caller_turn"].clone();
        assert_ne!(caller["id"], data.cases[1].turns.last().unwrap().id);
        assert_eq!(caller["text"], data.cases[1].turns.last().unwrap().text);
        assert_eq!(
            context["current_exchange"]["assistant_turn"]["text"],
            QUESTION
        );
        assert_eq!(context["policy_state"]["turn_version"], 2);
        assert_eq!(
            requests[1]["messages"].as_array().unwrap().last().unwrap()["tool_call_id"],
            "native_0"
        );
        assert_eq!(
            requests[3]["messages"].as_array().unwrap().last().unwrap()["tool_call_id"],
            "native_2"
        );
        let actions: i64 = sqlx::query_scalar("SELECT count(*) FROM actions")
            .fetch_one(&app.store.pool)
            .await
            .unwrap();
        assert_eq!(actions, 0);
        assert_eq!(app.store.sessions().await.unwrap().len(), 1);
        task.abort();
    }
    #[tokio::test]
    async fn premature_close_is_reported_without_forcing_the_second_turn() {
        let (app, model, provider, task) = local(true).await;
        let cases = run_conversation(
            &app,
            &model,
            &dataset(),
            &ModelCapabilities::from_config(&app.config),
        )
        .await
        .unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0]["passed"], false);
        assert_eq!(
            cases[0]["conversation_stop"],
            "first_turn_did_not_request_caller_reply"
        );
        assert_eq!(provider.count.load(Ordering::SeqCst), 2);
        let s = app.store.sessions().await.unwrap().pop().unwrap();
        assert_eq!(s.turn_version, 1);
        task.abort();
    }
}
