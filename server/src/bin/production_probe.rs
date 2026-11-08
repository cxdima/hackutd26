//! Non-dialing reconstruction of synthetic phone contexts through runtime builders and actor.
use anyhow::{Result, ensure};
use firstring::{
    calls::{
        actor::{App, Input},
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
fn hash(value: &Value) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2 && ["--capture", "--run"].contains(&args[0].as_str()),
        "Usage: production_probe --capture|--run OUTPUT.json"
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
    let run = args[0] == "--run";
    let settings = ModelSettings::from_env()?;
    let max = std::env::var("PRODUCTION_DIAGNOSTIC_MAX_TOKENS")
        .unwrap_or("512".into())
        .parse::<u32>()?;
    let timeout = std::env::var("PRODUCTION_DIAGNOSTIC_TIMEOUT_MS")
        .unwrap_or("3500".into())
        .parse::<u64>()?;
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
        let mut s = Session::new(
            case.id.clone(),
            format!("CA{}", uuid::Uuid::new_v4().simple()),
            case.household_id,
            ExecutionMode::Replay,
        );
        s.profile = case.profile;
        s.turns = case.turns;
        s.turn_version = case.turn_version;
        s.generation_id = case.generation_id;
        s.policy_epoch = case.policy_epoch;
        let handle = app.restore_synthetic_context(s.clone()).await?;
        let mut messages = conversation_messages(&s, &capabilities);
        let first = model.request_payload(&messages, ToolPhase::Assessment);
        let mut result = json!({"id":case.id,"request_sha256":hash(&first),"initial_request":first,"expected_action":case.expected_action,"expected_blocked":case.expected_blocked,"trace":[],"passed":false,"pass_scope":"native_control_flow_and_risk_flag_only","semantic_review_status":"pending_independent_review","semantic_review_rubric":case.semantic_review,"lifecycle_scope":if case.id.starts_with("attempt19") {"faithful_pre_assessment_context"} else {"semantic_prefix_not_phone_lifecycle"},"error":null});
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
                    result["trace"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"phase":phase,"observation":observed.observation}));
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
                    trace["tool"] =
                        json!({"id":tool.id,"name":tool.name,"arguments":tool.arguments});
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
            result["within_production_turn_budget"] =
                json!(started.elapsed() < Duration::from_secs(6));
            result["error"] = json!(outcome.err().map(|error| error.to_string()));
            if result["error"].is_string() {
                result["passed"] = json!(false);
            }
        }
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
