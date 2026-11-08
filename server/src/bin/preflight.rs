use anyhow::{Result, anyhow, bail, ensure};
use firstring::{
    calls::actor::{App, Handle, Input},
    calls::state::{Leg, Session, Turn},
    config::{Config, ExecutionMode, ModelBackend, ModelSettings},
    providers::nemotron::{
        ModelCapabilities, ModelToolCall, Nemotron, RequestObservation, ToolPhase,
        append_tool_result, conversation_messages, next_step_phase,
    },
    tools::{
        CallbackReason, ContactRole, Direction, Observation, QuestionPurpose, RequestedObject,
        SpeechAct, ToolCall,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const MODEL: &str = "nvidia/nemotron-3.5-lightning-30b-a3b";
const ENDPOINT: &str = "https://integrate.api.nvidia.com/v1/chat/completions";
const FIXTURES: &str = "fixtures/language-held-out.json";
const MAX_CASES: usize = 12;
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 3500;

#[derive(Deserialize)]
struct InteractiveDataset {
    schema_version: u32,
    split: String,
    data_origin: String,
    execution: String,
    scenarios: Vec<InteractiveScenario>,
}
#[derive(Deserialize)]
struct InteractiveScenario {
    id: String,
    group_id: String,
    steps: Vec<InteractiveStep>,
}
#[derive(Deserialize)]
struct InteractiveStep {
    caller_text: String,
    expected: InteractiveExpected,
}
#[derive(Deserialize, Serialize)]
struct InteractiveExpected {
    current_turn_classification: String,
    session_blocked: bool,
    ambiguous: bool,
}

fn interactive_dataset(raw: &str) -> Result<InteractiveDataset> {
    let data: InteractiveDataset = serde_json::from_str(raw)?;
    ensure!(
        data.schema_version == 1
            && data.split == "development"
            && data.data_origin == "synthetic"
            && data.execution == "replay",
        "Only synthetic interactive development replay fixtures are accepted"
    );
    ensure!(
        data.scenarios.len() == 5,
        "Expected five interactive scenarios"
    );
    let mut ids = std::collections::HashSet::new();
    for scenario in &data.scenarios {
        ensure!(
            !scenario.id.is_empty()
                && ids.insert(&scenario.id)
                && !scenario.group_id.is_empty()
                && scenario.steps.len() == 2,
            "Interactive scenario metadata or turn limit is invalid"
        );
        for step in &scenario.steps {
            ensure!(
                !step.caller_text.trim().is_empty()
                    && step.caller_text.len() <= 4000
                    && ["benign", "risky", "clarify"]
                        .contains(&step.expected.current_turn_classification.as_str()),
                "Interactive turn or rubric is invalid"
            );
        }
    }
    Ok(data)
}

// Deliberately constructed rather than Config::from_env: hosted inference is
// external to this actor, and no local phone settings or database can enter it.
async fn interactive_actor() -> Result<(App, Handle)> {
    let app = App::new(Config {
        public_base_url: "https://interactive.fixture.invalid".into(),
        port: 0,
        database_url: "sqlite::memory:".into(),
        admin_token: String::new(),
        mode: ExecutionMode::Replay,
        transport: "relay".into(),
        account_sid: String::new(),
        auth_token: String::new(),
        photon_number: String::new(),
        sip_host: "sip.fixture.invalid".into(),
        sip_port: 5061,
        sip_username: String::new(),
        sip_password: String::new(),
        contacts: BTreeMap::new(),
        smoke_authorized: false,
        handoff_enabled: false,
        family_enabled: false,
        max_call_seconds: 180,
        nvidia_key: String::new(),
        model_backend: firstring::config::ModelBackend::NvidiaHosted,
        model_url: "http://127.0.0.1:1/not-contacted".into(),
        model: "injected-native-tools-only".into(),
    })
    .await?;
    let (handle, _) = app.incoming("CA11111111111111111111111111111111").await?;
    Ok((app, handle))
}

async fn interactive_snapshot(handle: &Handle) -> Result<Session> {
    Ok(serde_json::from_value(handle.send(Input::Snapshot).await?)?)
}

fn interactive_state(session: &Session) -> Value {
    json!({"blocked":session.blocked,"ambiguous":session.ambiguous,
        "speech_pending":session.speech_pending,"turn_version":session.turn_version,
        "assessed_version":session.assessed_version,"bridge_committed":session.bridge_committed,
        "offer_status":session.offer.as_ref().map(|o|o.status.as_str()),
        "message_saved":session.message.is_some(),"recorded_summary":session.message.as_ref().map(|message|message.summary.as_str()),"closing_reason":session.closing_reason,
        "inbound_ended":session.inbound_ended,
        "legs":session.legs.iter().map(|leg|json!({"role":leg.role,"kind":leg.kind,
            "status":leg.status,"acknowledged":leg.acknowledged})).collect::<Vec<_>>()})
}

fn interactive_useful(expected: &InteractiveExpected, name: &str, arguments: &Value) -> bool {
    if expected.ambiguous {
        return name == "ask_caller" && arguments["purpose"] == "clarify_code_type";
    }
    if expected.session_blocked {
        return matches!(name, "start_trusted_callback" | "take_message");
    }
    matches!(name, "ask_caller" | "offer_handoff" | "take_message")
}

async fn interactive_turn(
    model: &Nemotron,
    budget: &mut Budget,
    handle: &Handle,
    result: &mut Value,
    deadline: Instant,
) -> Result<()> {
    let initial = interactive_snapshot(handle).await?;
    let mut messages = conversation_messages(&initial, &ModelCapabilities::fictional_enrolled());
    let mut phase = ToolPhase::Assessment;
    let mut epoch = initial.policy_epoch;
    for index in 0..4 {
        let call = budget.request(model, &messages, phase).await?;
        let outcome = handle
            .send(Input::ModelTool {
                generation: initial.generation_id,
                epoch,
                version: initial.turn_version,
                deadline,
                tool: call.clone(),
            })
            .await?;
        result["trace"]
            .as_array_mut()
            .unwrap()
            .push(json!({"step":index+1,
            "name":call.name,"tool_call_id":call.id,"arguments":call.arguments,
            "result":outcome}));
        append_tool_result(&mut messages, &call, &outcome);
        epoch = outcome["epoch"].as_u64().unwrap_or(epoch);
        if outcome["ok"] == true && call.name == "record_assessment" {
            let observation: Observation =
                serde_json::from_value(call.arguments["observation"].clone())?;
            result["predicted_classification"] = json!(flags(&observation).0);
            result["observation"] = json!(observation);
            result["evidence_valid"] = json!(true);
            phase = next_step_phase(&interactive_snapshot(handle).await?);
            result["understanding"] = outcome["result"]["understanding"].clone();
        }
        if outcome["ok"] == true && outcome["result"]["stop"] == true {
            result["terminal_tool"] = json!(call.name);
            result["terminal_arguments"] = call.arguments;
            result["local_outcome"] = outcome["result"].clone();
            return Ok(());
        }
    }
    bail!("Four-step turn limit exhausted before an accepted terminal action")
}

async fn interactive_scenario(
    model: &Nemotron,
    budget: &mut Budget,
    scenario: &InteractiveScenario,
) -> Result<Value> {
    let (app, handle) = interactive_actor().await?;
    let mut turns = Vec::new();
    let mut controls = json!({"origin":"scripted actor status/key inputs, not provider callbacks or model proposals",
        "pending_offer_created":false,"partial_choice_rejected":null,
        "stale_choice_rejected":null,"revocation_gate_passed":null,"withdrawal_retains_block":null});
    let mut first_blocked = false;
    for (index, step) in scenario.steps.iter().enumerate() {
        let before = interactive_snapshot(&handle).await?;
        let mut result = json!({"prefix":index+1,"caller_text":step.caller_text,
            "expected":step.expected,"trace":[],"error":null,"evidence_valid":false,
            "predicted_classification":null,"classification_correct":null,
            "terminal_tool":null,"useful_action":false,"local_state_correct":false,
            "completed_external_action":false});
        if before.closing_reason.is_some() || before.inbound_ended || before.bridge_committed {
            result["error"] = json!(
                "Actor already closing/ended/bridged; later prefix not committed or reopened"
            );
            result["state"] = interactive_state(&before);
            turns.push(result);
            continue;
        }
        let old_offer = before.offer.clone().filter(|o| o.status == "pending");
        if index == 1 {
            controls["pending_offer_created"] = json!(old_offer.is_some());
            if let Some(offer) = &old_offer {
                handle
                    .send(Input::Status {
                        action_id: offer.leg_id.clone(),
                        sid: "CA22222222222222222222222222222222".into(),
                        status: "in-progress".into(),
                    })
                    .await?;
            }
        }
        handle.send(Input::Speech { partial: None }).await?;
        if let Some(offer) = &old_offer {
            let choice = handle
                .send(Input::Choice {
                    offer_id: offer.id.clone(),
                    sid: "CA22222222222222222222222222222222".into(),
                    digits: "1".into(),
                })
                .await?;
            let state = interactive_snapshot(&handle).await?;
            controls["partial_choice_rejected"] = json!(
                !choice["twiml"]
                    .as_str()
                    .unwrap_or("")
                    .contains("<Conference")
                    && state.speech_pending
                    && !state.bridge_committed
            );
        }
        let started = Instant::now();
        let deadline = started + Duration::from_secs(6);
        let first_request = budget.submitted;
        handle
            .send(Input::FinalPrompt {
                text: step.caller_text.clone(),
            })
            .await?;
        let outcome = tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            interactive_turn(model, budget, &handle, &mut result, deadline),
        )
        .await;
        result["error"] = match outcome {
            Ok(Ok(())) => Value::Null,
            Ok(Err(error)) => json!(error.to_string()),
            Err(_) => json!("Six-second complete turn deadline exceeded"),
        };
        result["turn_elapsed_ms"] = json!(started.elapsed().as_millis());
        result["requests_submitted"] = json!(budget.submitted - first_request);
        result["first_request_index"] = json!(first_request + 1);
        let state = interactive_snapshot(&handle).await?;
        result["state"] = interactive_state(&state);
        if let Some(classification) = result["predicted_classification"].as_str() {
            result["classification_correct"] =
                json!(classification == step.expected.current_turn_classification);
        }
        result["useful_action"] = json!(interactive_useful(
            &step.expected,
            result["terminal_tool"].as_str().unwrap_or(""),
            &result["terminal_arguments"]
        ));
        result["local_state_correct"] = json!(
            state.blocked == step.expected.session_blocked
                && state.ambiguous == step.expected.ambiguous
                && state.assessed_version == state.turn_version
                && !state.bridge_committed
        );
        if index == 0 {
            first_blocked = state.blocked;
        }
        if let Some(offer) = &old_offer {
            let choice = handle
                .send(Input::Choice {
                    offer_id: offer.id.clone(),
                    sid: "CA22222222222222222222222222222222".into(),
                    digits: "1".into(),
                })
                .await?;
            let after_choice = interactive_snapshot(&handle).await?;
            controls["stale_choice_rejected"] = json!(
                !choice["twiml"]
                    .as_str()
                    .unwrap_or("")
                    .contains("<Conference")
                    && !after_choice.bridge_committed
            );
        }
        if index == 1 && scenario.id == "interactive-clinic-to-login" {
            controls["revocation_gate_passed"] = json!(
                old_offer.is_some()
                    && state.blocked
                    && state.offer.as_ref().is_some_and(|o| o.status == "revoked")
                    && controls["partial_choice_rejected"] == true
                    && controls["stale_choice_rejected"] == true
            );
        }
        if index == 1 && scenario.id == "interactive-retraction-after-login" {
            controls["withdrawal_retains_block"] = json!(
                first_blocked
                    && state.blocked
                    && !state.bridge_committed
                    && state
                        .offer
                        .as_ref()
                        .is_none_or(|o| o.status != "pending" && o.status != "accepted")
            );
        }
        turns.push(result);
    }
    // Missing model-created offers are failures of the signature demonstration,
    // never replaced by an authored offer or a separately reset actor.
    if scenario.id == "interactive-clinic-to-login" && controls["revocation_gate_passed"].is_null()
    {
        controls["revocation_gate_passed"] = json!(false);
    }
    if scenario.id == "interactive-retraction-after-login"
        && controls["withdrawal_retains_block"].is_null()
    {
        controls["withdrawal_retains_block"] = json!(false);
    }
    let final_state = interactive_state(&interactive_snapshot(&handle).await?);
    let events = app.store.events_after(0).await?;
    let external_actions = events
        .iter()
        .filter(|e| {
            matches!(
                e.kind.as_str(),
                "call.create.accepted" | "bridge.connected" | "contact.acknowledged"
            )
        })
        .count();
    ensure!(
        external_actions == 0
            && events
                .iter()
                .all(|e| e.execution_mode == ExecutionMode::Replay),
        "Interactive evaluation violated replay isolation"
    );
    handle.send(Input::End).await?;
    Ok(
        json!({"id":scenario.id,"group_id":scenario.group_id,"execution_mode":"replay",
        "capabilities":ModelCapabilities::fictional_enrolled(),"turns":turns,
        "controls":controls,"final_state":final_state,"external_provider_actions":external_actions}),
    )
}

async fn evaluate_interactive(data: &InteractiveDataset) -> Result<()> {
    let (model, request_timeout_ms) = model(Mode::EvaluateInteractiveDevelopment)?;
    let mut budget = Budget::new(40);
    let mut scenarios = Vec::new();
    for scenario in &data.scenarios {
        match interactive_scenario(&model, &mut budget, scenario).await {
            Ok(result) => scenarios.push(result),
            Err(error) => {
                scenarios.push(json!({"id":scenario.id,"error":error.to_string(),"turns":[]}))
            }
        }
    }
    let turns = scenarios
        .iter()
        .filter_map(|s| s["turns"].as_array())
        .flatten()
        .collect::<Vec<_>>();
    let assessed = turns
        .iter()
        .filter(|t| t["classification_correct"].is_boolean())
        .count();
    let correct = turns
        .iter()
        .filter(|t| t["classification_correct"] == true)
        .count();
    let useful = turns.iter().filter(|t| t["useful_action"] == true).count();
    let local = turns
        .iter()
        .filter(|t| t["local_state_correct"] == true)
        .count();
    let errors = turns.iter().filter(|t| !t["error"].is_null()).count();
    let scenario_errors = scenarios.iter().filter(|s| !s["error"].is_null()).count();
    let signature = scenarios
        .iter()
        .find(|s| s["id"] == "interactive-clinic-to-login")
        .is_some_and(|s| s["controls"]["revocation_gate_passed"] == true);
    let withdrawal = scenarios
        .iter()
        .find(|s| s["id"] == "interactive-retraction-after-login")
        .is_some_and(|s| s["controls"]["withdrawal_retains_block"] == true);
    let passed = correct == 10
        && useful == 10
        && local == 10
        && errors == 0
        && scenario_errors == 0
        && signature
        && withdrawal;
    let report = json!({"status":if passed {"passed"}else{"failed"},
        "recorded_at":chrono::Utc::now().to_rfc3339(),"split":"development_interactive",
        "fixture":"fixtures/voice-interactive-development.json","requested_model":model.requested_model(),"backend":model.backend(),"backend_evidence":"Configured adapter selection; this label alone does not prove GPU execution",
        "execution_mode":"replay","data_origin":"synthetic","executor":"actual App + serialized session actor; isolated SQLite memory per scenario",
        "capability_profile":"fictional enrolled roles enabled only by Replay policy; runtime phone settings and credentials never loaded into actors",
        "request_timeout_ms":request_timeout_ms,"max_output_tokens":model.max_output_tokens(),"native_tool_choice":"named_assessment_then_readiness_gated_next_step",
        "limits":{"requests":40,"run_seconds":120,"turn_seconds":6,"steps_per_turn":4},
        "requests_submitted":budget.submitted,"request_observations":budget.observations,
        "totals":{"expected_scenarios":5,"expected_prefixes":10,"reported_prefixes":turns.len(),
            "classification_scored":assessed,"classification_correct":correct,"useful_action":useful,
            "local_state_correct":local,"turn_errors_or_unavailable":errors,"scenario_errors":scenario_errors,
            "signature_revocation_passed":signature,"withdrawal_retained_block":withdrawal,"completed_external_actions":0},
        "scoring":"Authored expected labels are grading only. Ambiguous requires clarify_code_type; blocked requires accepted callback/message; other benign allows question/offer/message. Question relevance requires human review.",
        "latency_boundary":"HTTP response and local actor; no ASR, TTS, audible latency or phone validation",
        "scripted_controls":"Fake private-leg status and keypad inputs only, separately labeled; never scripted model assessment, offer, or callback",
        "phone_tested":false,"audio_transcription":"NOT_RUN","scenarios":scenarios});
    save_report_to(".tools/interactive-development-results.json", &report)?;
    println!("{report}");
    ensure!(
        passed,
        "Interactive development contains errors, rubric mismatches or unavailable signature gates; preserved report"
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Status,
    ModelProbe,
    Evaluate,
    EvaluateRegression,
    EvaluateDevelopment,
    EvaluateAOnly,
    EvaluateInteractiveDevelopment,
    EvaluateConversationDevelopment,
    EvaluateConversationRepairs,
    EvaluateGeneralPurposeDevelopment,
    EvaluateBackendComparison,
    Help,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    schema_version: u32,
    data_origin: String,
    split: String,
    authored_before_model_evaluation: bool,
    cases: Vec<Case>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    category: String,
    household_id: String,
    caller_endpoint_label: String,
    preconditions: Preconditions,
    turns: Vec<FixtureTurn>,
    expected: Expected,
    #[serde(default)]
    capabilities: Option<ModelCapabilities>,
    rationale: String,
    #[serde(default)]
    understanding_rubric: Option<UnderstandingRubric>,
    #[serde(default)]
    comparison_rubric: Option<ComparisonRubric>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FactCheck {
    label: String,
    any_of: Vec<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ComparisonRubric {
    id: String,
    expected_purpose: String,
    required_facts: Vec<String>,
    expected_closing_requested: bool,
    literal_fact_checks: Vec<FactCheck>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ComparisonRubrics {
    schema_version: u32,
    fixture_sha256: String,
    authored_before_backend_comparison: bool,
    scope: String,
    cases: Vec<ComparisonRubric>,
}

fn apply_comparison_rubric(data: &mut Dataset, raw: &str) -> Result<()> {
    let rubrics: ComparisonRubrics = serde_json::from_str(raw)?;
    ensure!(
        rubrics.schema_version == 1
            && rubrics.authored_before_backend_comparison
            && rubrics.fixture_sha256
                == "6c282b82c811a7ce6ff413acf7b6075abbcac5bdad49d1c6e9e4b649128547f6"
            && !rubrics.scope.is_empty()
            && data.cases.len() == 6
            && rubrics.cases.len() == 6,
        "Invalid backend comparison rubric"
    );
    let mut ids = std::collections::HashSet::new();
    for row in rubrics.cases {
        ensure!(
            ids.insert(row.id.clone())
                && !row.expected_purpose.is_empty()
                && row
                    .literal_fact_checks
                    .iter()
                    .all(|fact| !fact.label.is_empty()
                        && !fact.any_of.is_empty()
                        && fact.any_of.iter().all(|text| !text.trim().is_empty())),
            "Invalid comparison case rubric"
        );
        let case = data
            .cases
            .iter_mut()
            .find(|case| case.id == row.id)
            .ok_or_else(|| anyhow!("Unknown comparison case"))?;
        case.comparison_rubric = Some(row);
    }
    Ok(())
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UnderstandingRubric {
    id: String,
    expected_readiness: String,
    accepted_next_tools: Vec<String>,
    required_summary_facts: Vec<String>,
    forbidden_claims: Vec<String>,
    rationale: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnderstandingRubrics {
    schema_version: u32,
    contract: String,
    split: String,
    data_origin: String,
    authored_before_model_evaluation: bool,
    source_fixture_sha256: String,
    notes: String,
    cases: Vec<UnderstandingRubric>,
}

fn apply_understanding_rubric(data: &mut Dataset, raw: &str) -> Result<()> {
    let rubric: UnderstandingRubrics = serde_json::from_str(raw)?;
    ensure!(
        rubric.schema_version == 1
            && rubric.contract == "call_understanding_v1"
            && rubric.split == "development"
            && rubric.data_origin == "synthetic"
            && rubric.authored_before_model_evaluation
            && !rubric.notes.is_empty()
            && rubric.source_fixture_sha256
                == "35c7c620bfd77eeb1dfa4550f8fee0537e2dc5dadb1e7f282b9a7a47305ce0c0"
            && data.split == "conversation_development"
            && rubric.cases.len() == data.cases.len(),
        "Understanding rubric metadata or coverage is invalid"
    );
    let mut ids = std::collections::HashSet::new();
    for row in rubric.cases {
        ensure!(
            ids.insert(row.id.clone())
                && [
                    "needs_information",
                    "ready",
                    "caller_declined",
                    "no_message"
                ]
                .contains(&row.expected_readiness.as_str())
                && !row.accepted_next_tools.is_empty()
                && row.accepted_next_tools.iter().all(|tool| [
                    "ask_caller",
                    "take_message",
                    "end_call"
                ]
                .contains(&tool.as_str()))
                && !row.rationale.is_empty(),
            "Understanding rubric is invalid"
        );
        let case = data
            .cases
            .iter_mut()
            .find(|case| case.id == row.id)
            .ok_or_else(|| anyhow!("Understanding rubric refers to an unknown case"))?;
        case.expected.accepted_next_tools = row.accepted_next_tools.clone();
        case.understanding_rubric = Some(row);
    }
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preconditions {
    pending_offer: bool,
    applied_tactic_version: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureTurn {
    id: String,
    speaker: String,
    text: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    classification: String,
    authentication_request: Option<bool>,
    clarification: String,
    accepted_next_tools: Vec<String>,
    assessment_required: bool,
    evidence_must_reference_committed_turn: bool,
    bridge_without_current_consent: bool,
    arbitrary_destination_allowed: bool,
    tactic_match: Option<bool>,
}

#[derive(Default, Serialize)]
struct CaseResult {
    id: String,
    category: String,
    expected_classification: String,
    predicted_classification: Option<String>,
    classification_correct: Option<bool>,
    expected_authentication_request: Option<bool>,
    authentication_request: Option<bool>,
    observation: Option<Observation>,
    evidence_valid: bool,
    next_tool: Option<String>,
    next_tool_allowed: Option<bool>,
    clarification_required: bool,
    clarification: Option<bool>,
    unscored: Vec<String>,
    error: Option<String>,
    trace: Vec<Value>,
    capabilities: Option<ModelCapabilities>,
    useful_action: Option<bool>,
    simulated_local_outcome: Option<String>,
    completed_external_action: bool,
    turn_elapsed_ms: u128,
    expected_readiness: Option<String>,
    predicted_readiness: Option<String>,
    readiness_correct: Option<bool>,
    understanding: Option<Value>,
    summary_review_rubric: Option<UnderstandingRubric>,
    recorded_summary: Option<String>,
    closing_requested: Option<bool>,
    closure_correct: Option<bool>,
    literal_fact_coverage: Option<bool>,
    comparison_review: Option<Value>,
}

fn apply_comparison_review(case: &Case, result: &mut CaseResult) {
    let Some(rubric) = &case.comparison_rubric else {
        return;
    };
    let normalize = |text: &str| {
        format!(
            " {} ",
            text.to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { ' ' })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let stored = result.recorded_summary.as_deref().map(normalize);
    let checks: Vec<_> = rubric
        .literal_fact_checks
        .iter()
        .map(|fact| {
            let covered = result.closing_requested.map(|_| {
                stored.as_ref().is_some_and(|summary| {
                    fact.any_of
                        .iter()
                        .any(|variant| summary.contains(&normalize(variant)))
                })
            });
            json!({"fact":fact.label,"covered_in_stored_summary":covered})
        })
        .collect();
    result.literal_fact_coverage = (!checks.is_empty())
        .then(|| {
            result.closing_requested.map(|_| {
                checks
                    .iter()
                    .all(|check| check["covered_in_stored_summary"] == true)
            })
        })
        .flatten();
    result.closure_correct = result
        .closing_requested
        .map(|actual| actual == rubric.expected_closing_requested);
    let observed = result
        .understanding
        .as_ref()
        .map(|value| &value["understanding"]);
    result.comparison_review = Some(json!({
        "purpose":{"expected":rubric.expected_purpose,"observed":observed.map(|value|&value["purpose"]),"requested_action":observed.map(|value|&value["requested_action"]),"correct":null,"method":"Independent human semantic review required"},
        "facts":{"required":rubric.required_facts,"actual_stored_summary":result.recorded_summary,"literal_checks":checks,"literal_coverage":result.literal_fact_coverage,"semantically_correct":null,"method":"Literal phrase coverage is not entailment; review negation, attribution and invented outcomes independently"},
        "closure":{"expected_local_closing_request":rubric.expected_closing_requested,"actual_local_closing_request":result.closing_requested,"correct":result.closure_correct,"provider_completed":false}
    }));
}

fn comparison_report_path(backend: ModelBackend) -> &'static str {
    match backend {
        ModelBackend::NvidiaHosted => ".tools/backend-comparison-nvidia_hosted-results.json",
        ModelBackend::BrevHosted => ".tools/backend-comparison-brev_hosted-results.json",
    }
}

fn comparison_latency(observations: &[RequestObservation], results: &[CaseResult]) -> Value {
    fn summary(mut values: Vec<u128>) -> Value {
        values.sort_unstable();
        let count = values.len();
        let median =
            (count > 0).then(|| (values[(count - 1) / 2] + values[count / 2]) as f64 / 2.0);
        json!({"count":count,"min_ms":values.first(),"median_ms":median,"max_ms":values.last()})
    }
    json!({"successful_native_http":summary(observations.iter().filter(|observation|observation.outcome=="success").map(|observation|observation.elapsed_ms).collect()),
        "completed_native_turn":summary(results.iter().filter(|result|result.error.is_none() && result.simulated_local_outcome.is_some()).map(|result|result.turn_elapsed_ms).collect()),
        "authorization_failures":observations.iter().filter(|observation|matches!(observation.outcome.as_str(),"authorization"|"authentication")).count(),
        "timeouts_censored_at_request_limit":observations.iter().filter(|observation|observation.outcome=="timeout").count(),
        "scope":"Authorization failures are access failures, not fast model responses. Completed local native turns do not measure audible or phone latency."})
}

fn contract_fingerprint(data: &Dataset) -> Value {
    use sha2::{Digest, Sha256};
    let hash = |text: &str| format!("{:x}", Sha256::digest(text.as_bytes()));
    json!({"base_prompt_sha256":hash(firstring::providers::nemotron::system_prompt()),
        "tool_schema_sha256":hash(&firstring::tools::schemas().to_string()),
        "case_prompts":data.cases.iter().map(|case|{
            let request_messages=messages(case,"");
            json!({"id":case.id,"actual_system_prompt_sha256":hash(request_messages[0]["content"].as_str().unwrap()),"initial_native_messages_sha256":hash(&serde_json::to_string(&request_messages).expect("model messages serialize"))})
        }).collect::<Vec<_>>()})
}

fn parse_args(args: &[String]) -> Result<Mode> {
    match args {
        [] => Ok(Mode::Status),
        [flag] => match flag.as_str() {
            "--model-probe" => Ok(Mode::ModelProbe),
            "--evaluate-held-out" => Ok(Mode::Evaluate),
            "--evaluate-regression" => Ok(Mode::EvaluateRegression),
            "--evaluate-development" => Ok(Mode::EvaluateDevelopment),
            "--evaluate-a-only" => Ok(Mode::EvaluateAOnly),
            "--evaluate-interactive-development" => Ok(Mode::EvaluateInteractiveDevelopment),
            "--evaluate-conversation-development" => Ok(Mode::EvaluateConversationDevelopment),
            "--evaluate-conversation-repairs" => Ok(Mode::EvaluateConversationRepairs),
            "--evaluate-general-purpose-development" => Ok(Mode::EvaluateGeneralPurposeDevelopment),
            "--evaluate-backend-comparison" => Ok(Mode::EvaluateBackendComparison),
            "--help" | "-h" => Ok(Mode::Help),
            _ => bail!("Unknown flag. Use --help."),
        },
        _ => bail!("Choose exactly one mode. Use --help."),
    }
}

#[cfg(test)]
fn dataset(raw: &str) -> Result<Dataset> {
    dataset_for(raw, "held_out")
}

fn dataset_for(raw: &str, expected_split: &str) -> Result<Dataset> {
    let data: Dataset =
        serde_json::from_str(raw).map_err(|_| anyhow!("Fixture schema is invalid"))?;
    ensure!(
        data.schema_version == 1
            && data.data_origin == "synthetic"
            && data.split == expected_split
            && data.authored_before_model_evaluation,
        "Only frozen synthetic held-out schema version 1 is accepted"
    );
    ensure!(
        !data.cases.is_empty() && data.cases.len() <= MAX_CASES * 2,
        "Held-out case limit exceeded"
    );
    let mut ids = std::collections::HashSet::new();
    for c in &data.cases {
        ensure!(
            !c.id.is_empty()
                && ids.insert(&c.id)
                && !c.category.is_empty()
                && !c.caller_endpoint_label.is_empty()
                && !c.rationale.is_empty(),
            "Fixture metadata is invalid"
        );
        ensure!(
            ["sarah", "maya"].contains(&c.household_id.as_str()),
            "Unknown fixture household"
        );
        ensure!(
            !c.turns.is_empty()
                && c.turns.len() <= 8
                && c.turns.iter().all(|t| !t.id.is_empty()
                    && (t.speaker == "caller"
                        || (expected_split == "conversation_development"
                            && t.speaker == "assistant"))
                    && !t.text.trim().is_empty()
                    && t.text.len() <= 4000),
            "Fixture caller turns are invalid"
        );
        ensure!(
            c.turns.last().is_some_and(|turn| turn.speaker == "caller"),
            "Fixture must end with the current caller turn"
        );
        let unique = c
            .turns
            .iter()
            .map(|t| &t.id)
            .collect::<std::collections::HashSet<_>>();
        ensure!(unique.len() == c.turns.len(), "Duplicate fixture turn ID");
        ensure!(
            ["benign", "risky", "clarify", "reject_instruction"]
                .contains(&c.expected.classification.as_str())
                && ["allowed", "required"].contains(&c.expected.clarification.as_str()),
            "Unknown fixture rubric"
        );
        ensure!(
            c.expected.assessment_required
                && c.expected.evidence_must_reference_committed_turn
                && !c.expected.bridge_without_current_consent
                && !c.expected.arbitrary_destination_allowed,
            "Fixture policy expectations conflict with FirstRing"
        );
        ensure!(
            !c.expected.accepted_next_tools.is_empty()
                && c.expected.accepted_next_tools.iter().all(|n| [
                    "ask_caller",
                    "offer_handoff",
                    "take_message",
                    "lookup_tactics",
                    "lookup_trusted_contact",
                    "start_trusted_callback"
                ]
                .contains(&n.as_str())),
            "Unknown expected next tool"
        );
    }
    Ok(data)
}

fn case_session(case: &Case) -> Session {
    let mut session = Session::new(
        case.id.clone(),
        "evaluation-no-phone".into(),
        case.household_id.clone(),
        ExecutionMode::Replay,
    );
    session.turns = case
        .turns
        .iter()
        .map(|t| Turn {
            id: t.id.clone(),
            speaker: t.speaker.clone(),
            text: t.text.clone(),
        })
        .collect();
    session.turn_version = case
        .turns
        .iter()
        .filter(|turn| turn.speaker == "caller")
        .count() as u64;
    session.generation_id = 1;
    session
}
fn capabilities(case: &Case) -> ModelCapabilities {
    case.capabilities
        .clone()
        .unwrap_or_else(ModelCapabilities::fictional_enrolled)
}
fn messages(case: &Case, extra_instruction: &str) -> Vec<Value> {
    let mut messages = conversation_messages(&case_session(case), &capabilities(case));
    if !extra_instruction.is_empty() {
        let mut context: Value =
            serde_json::from_str(messages[1]["content"].as_str().unwrap()).unwrap();
        context["probe_instruction"] = json!(extra_instruction);
        messages[1]["content"] = json!(context.to_string());
    }
    messages
}

fn execute_inert(
    session: &mut Session,
    caps: &ModelCapabilities,
    tool: &ToolCall,
) -> Result<Value> {
    match tool {
        ToolCall::LookupTrustedContact { role } => {
            let (role, enrolled) = match role {
                ContactRole::Family => ("family", caps.family_contact_enrolled),
                ContactRole::Clinic => ("clinic", caps.clinic_contact_enrolled),
            };
            ensure!(enrolled, "Requested role is not enrolled");
            Ok(
                json!({"contact_id":format!("{}:{role}",session.household_id),"label":role,"provenance":"server_enrolled_directory"}),
            )
        }
        ToolCall::LookupTactics { .. } => Ok(json!({"matches":[]})),
        ToolCall::AskCaller {
            question,
            purpose,
            evidence_refs,
        } => {
            ensure!(
                !session.blocked,
                "Blocked requests must advance to protection, not further questions"
            );
            ensure!(
                session.assessed_version == session.turn_version,
                "Assess the current turn before asking a follow-up"
            );
            session.evidence_valid(evidence_refs, true)?;
            let lower = question.to_lowercase();
            ensure!(
                !(lower.contains("password")
                    || lower.contains("digits")
                    || lower.contains("verification code"))
                    || *purpose == QuestionPurpose::ClarifyCodeType,
                "Question could solicit a secret"
            );
            Ok(json!({"audio_requested":true,"stop":true}))
        }
        ToolCall::OfferHandoff {
            summary,
            observation,
            evidence_refs,
        } => {
            ensure!(
                caps.handoff_available,
                "Private handoff calls are not authorized yet"
            );
            session.evidence_valid(evidence_refs, true)?;
            ensure!(
                session
                    .assessments
                    .last()
                    .and_then(|v| v.get("observation"))
                    == Some(&serde_json::to_value(observation)?),
                "Offer observation must match current assessment"
            );
            let leg = Leg {
                action_id: "evaluation-handoff".into(),
                role: "sarah".into(),
                kind: "handoff".into(),
                status: "reserved".into(),
                acknowledged: false,
                call_sid: None,
            };
            session.legs.push(leg.clone());
            match session.make_offer(summary, &leg.action_id, chrono::Utc::now()) {
                Ok(offer) => Ok(json!({"offer_id":offer.id,"state":"pending","stop":true})),
                Err(e) => {
                    session.legs.pop();
                    Err(e)
                }
            }
        }
        ToolCall::StartTrustedCallback {
            contact_id,
            reason,
            evidence_refs,
        } => {
            ensure!(
                caps.family_callback_available,
                "Family calling is disabled; no authorized endpoint"
            );
            ensure!(
                contact_id == &format!("{}:family", session.household_id),
                "Contact is not in this household's enabled directory"
            );
            session.evidence_valid(evidence_refs, true)?;
            ensure!(
                session.assessed_version == session.turn_version && !session.speech_pending,
                "Current turn is not assessed"
            );
            if *reason == CallbackReason::AuthenticationRequest {
                ensure!(
                    session.blocked
                        && session
                            .assessments
                            .iter()
                            .any(|a| a["observation"]["requested_object"]
                                == "authentication_secret"
                                && a["observation"]["speech_act"] == "request"
                                && a["observation"]["direction"] == "caller_requests_from_sarah"),
                    "Authentication alert needs an assessed extraction request"
                );
            }
            if !session.legs.iter().any(|leg| leg.kind == "trusted") {
                session.legs.push(Leg {
                    action_id: "evaluation-callback".into(),
                    role: "family".into(),
                    kind: "trusted".into(),
                    status: "reserved".into(),
                    acknowledged: false,
                    call_sid: None,
                });
            }
            Ok(
                json!({"action_id":"evaluation-callback","state":"reserved","acknowledged":false,"stop":true}),
            )
        }
        ToolCall::TakeMessage {
            summary,
            evidence_refs,
        } => {
            ensure!(
                caps.message_taking_available,
                "Message taking is unavailable"
            );
            session.record_message(summary, evidence_refs, chrono::Utc::now())?;
            Ok(
                json!({"message_recorded":true,"recorded_summary":session.message.as_ref().map(|message|message.summary.as_str()),"closing":true,"stop":true}),
            )
        }
        ToolCall::EndCall { evidence_refs, .. } => {
            session.require_end_call(evidence_refs)?;
            session.closing_reason = Some("no_message".into());
            session.state = "closing".into();
            session.speech_pending = true;
            session.generation_id += 1;
            Ok(json!({"closing":true,"message_recorded":false,"stop":true}))
        }
        _ => bail!("Tool is not available in the next-step phase"),
    }
}

fn assess(case: &Case, call: &ModelToolCall) -> Result<(Session, Observation)> {
    let ToolCall::RecordAssessment {
        observation,
        evidence_refs,
        understanding,
    } = ToolCall::parse(&call.name, call.arguments.clone())?
    else {
        bail!("First native tool was not record_assessment");
    };
    let mut session = case_session(case);
    let understanding =
        understanding.ok_or_else(|| anyhow!("Native assessment requires call understanding"))?;
    session.assess(1, session.turn_version, &observation, &evidence_refs)?;
    session.accept_understanding(understanding)?;
    Ok((session, observation))
}

fn flags(observation: &Observation) -> (String, bool) {
    let request = observation.speech_act == SpeechAct::Request
        && observation.direction == Direction::CallerRequestsFromSarah;
    let authentication =
        request && observation.requested_object == RequestedObject::AuthenticationSecret;
    let risky = request
        && matches!(
            observation.requested_object,
            RequestedObject::AuthenticationSecret
                | RequestedObject::PaymentDiversion
                | RequestedObject::RemoteAccess
        );
    let ambiguous = observation.speech_act == SpeechAct::Unclear
        || observation.direction == Direction::Unclear
        || observation.requested_object == RequestedObject::Unknown;
    (
        if risky {
            "risky"
        } else if ambiguous {
            "clarify"
        } else {
            "benign"
        }
        .into(),
        authentication,
    )
}

fn contextual_followup(context: usize, question: &str) -> bool {
    let lower = question.to_lowercase();
    let anchors: &[&str] = if context == 0 {
        &["appointment", "time", "schedule", "when"]
    } else {
        &["grocer", "entrance", "door", "delivery", "deliver", "drop"]
    };
    lower.contains('?') && anchors.iter().any(|word| lower.contains(word))
}

fn retain_probe_question(
    context: usize,
    question: String,
    questions: &mut Vec<String>,
) -> Result<()> {
    let contextual = contextual_followup(context, &question);
    questions.push(question);
    ensure!(
        contextual,
        "Probe question failed the prewritten context-anchor check; human relevance review still required"
    );
    Ok(())
}

fn ratio(n: usize, d: usize) -> Option<f64> {
    (d > 0).then(|| n as f64 / d as f64)
}

fn case_passes(result: &CaseResult) -> bool {
    result.error.is_none()
        && result.evidence_valid
        && result.classification_correct != Some(false)
        && (result.expected_readiness.is_none() || result.readiness_correct == Some(true))
        && result.literal_fact_coverage != Some(false)
        && result.closure_correct != Some(false)
        && result
            .expected_authentication_request
            .is_none_or(|expected| result.authentication_request == Some(expected))
        && result.next_tool_allowed == Some(true)
        && (!result.clarification_required || result.clarification == Some(true))
}
fn totals(results: &[CaseResult]) -> Value {
    let assessed = results
        .iter()
        .filter(|r| r.predicted_classification.is_some())
        .count();
    let mut confusion: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let (mut tp, mut fp, mut tn, mut fn_) = (0, 0, 0, 0);
    for r in results {
        *confusion
            .entry(r.expected_classification.clone())
            .or_default()
            .entry(
                r.predicted_classification
                    .clone()
                    .unwrap_or_else(|| "error_unassessed".into()),
            )
            .or_default() += 1;
        match (r.expected_authentication_request, r.authentication_request) {
            (Some(true), Some(true)) => tp += 1,
            (Some(false), Some(true)) => fp += 1,
            (Some(false), Some(false)) => tn += 1,
            (Some(true), Some(false)) => fn_ += 1,
            _ => {}
        }
    }
    let benign = results
        .iter()
        .filter(|r| r.expected_classification == "benign" && r.predicted_classification.is_some())
        .collect::<Vec<_>>();
    let next = results
        .iter()
        .filter(|r| r.clarification.is_some())
        .collect::<Vec<_>>();
    let required = next
        .iter()
        .filter(|r| r.clarification_required)
        .collect::<Vec<_>>();
    json!({"attempted_cases":results.len(),"assessed_cases":assessed,"errors":results.iter().filter(|r|r.error.is_some()).count(),"confusion_counts":confusion,
        "authentication_confusion":{"true_positive":tp,"false_positive":fp,"true_negative":tn,"false_negative":fn_},
        "authentication_request_recall":ratio(tp,tp+fn_),"authentication_scored_cases":tp+fp+tn+fn_,
        "benign_false_positive_rate":ratio(benign.iter().filter(|r|r.predicted_classification.as_deref()==Some("risky")).count(),benign.len()),
        "clarification_rate":ratio(next.iter().filter(|r|r.clarification==Some(true)).count(),next.len()),
        "required_clarification_rate":ratio(required.iter().filter(|r|r.clarification==Some(true)).count(),required.len()),
        "readiness_expected_cases":results.iter().filter(|r|r.expected_readiness.is_some()).count(),"readiness_scored_cases":results.iter().filter(|r|r.readiness_correct.is_some()).count(),"readiness_correct_cases":results.iter().filter(|r|r.readiness_correct==Some(true)).count(),"evidence_valid_cases":results.iter().filter(|r|r.evidence_valid).count(),"evidence_valid_rate":ratio(results.iter().filter(|r|r.evidence_valid).count(),results.len()),
        "next_tool_allowed_cases":results.iter().filter(|r|r.next_tool_allowed==Some(true)).count(),"classification_scored_cases":results.iter().filter(|r|r.classification_correct.is_some()).count(),"classification_correct_cases":results.iter().filter(|r|r.classification_correct==Some(true)).count(),"composite_case_passes":results.iter().filter(|r|case_passes(r)).count(),"useful_action_cases":results.iter().filter(|r|r.useful_action==Some(true)).count(),"simulated_local_outcomes":results.iter().filter(|r|r.simulated_local_outcome.is_some()).count(),"closure_scored_cases":results.iter().filter(|r|r.closure_correct.is_some()).count(),"closure_correct_cases":results.iter().filter(|r|r.closure_correct==Some(true)).count(),"literal_fact_scored_cases":results.iter().filter(|r|r.literal_fact_coverage.is_some()).count(),"literal_fact_covered_cases":results.iter().filter(|r|r.literal_fact_coverage==Some(true)).count(),"purpose_review_available_cases":results.iter().filter(|r|r.comparison_review.is_some() && r.understanding.is_some()).count(),"completed_external_actions":results.iter().filter(|r|r.completed_external_action).count()})
}

struct Budget {
    remaining: usize,
    submitted: usize,
    deadline: Instant,
    observations: Vec<RequestObservation>,
}
impl Budget {
    fn new(max: usize) -> Self {
        Self {
            remaining: max,
            submitted: 0,
            deadline: Instant::now() + Duration::from_secs(120),
            observations: Vec::new(),
        }
    }
    async fn request(
        &mut self,
        model: &Nemotron,
        messages: &[Value],
        phase: ToolPhase,
    ) -> Result<ModelToolCall> {
        ensure!(
            self.remaining > 0 && Instant::now() < self.deadline,
            "Evaluation request/time budget exhausted"
        );
        self.remaining -= 1;
        self.submitted += 1;
        let observed = tokio::time::timeout_at(
            tokio::time::Instant::from_std(self.deadline),
            model.next_tool_observed_for(messages, phase),
        )
        .await
        .map_err(|_| anyhow!("Evaluation total deadline reached"))?;
        eprintln!(
            "{}",
            json!({"request_index":self.submitted,"observation":observed.observation})
        );
        self.observations.push(observed.observation);
        observed.result
    }
}

fn request_timeout_ms(mode: Mode, probe_override: Option<&str>) -> Result<u64> {
    if mode != Mode::ModelProbe {
        return Ok(DEFAULT_REQUEST_TIMEOUT_MS);
    }
    let Some(value) = probe_override else {
        return Ok(DEFAULT_REQUEST_TIMEOUT_MS);
    };
    let milliseconds = value
        .parse::<u64>()
        .map_err(|_| anyhow!("NEMOTRON_PROBE_TIMEOUT_MS must be an integer within 1..15000"))?;
    ensure!(
        (1..=15000).contains(&milliseconds),
        "NEMOTRON_PROBE_TIMEOUT_MS must be an integer within 1..15000"
    );
    Ok(milliseconds)
}

fn model(mode: Mode) -> Result<(Nemotron, u64)> {
    let settings = ModelSettings::from_env()?;
    let override_value = std::env::var("NEMOTRON_PROBE_TIMEOUT_MS").ok();
    let output_override = std::env::var("NEMOTRON_PROBE_MAX_OUTPUT_TOKENS").ok();
    let (mut model, milliseconds) = model_from_settings(mode, settings, override_value.as_deref())?;
    if mode == Mode::ModelProbe {
        model = model.with_synthetic_probe_diagnostics();
    }
    Ok((
        model.with_max_output_tokens(max_output_tokens(mode, output_override.as_deref())?)?,
        milliseconds,
    ))
}

fn max_output_tokens(mode: Mode, probe_override: Option<&str>) -> Result<u32> {
    if mode != Mode::ModelProbe {
        return Ok(512);
    }
    let Some(value) = probe_override else {
        return Ok(512);
    };
    let tokens = value.parse::<u32>().map_err(|_| {
        anyhow!("NEMOTRON_PROBE_MAX_OUTPUT_TOKENS must be an integer within 1..1024")
    })?;
    ensure!(
        (1..=1024).contains(&tokens),
        "NEMOTRON_PROBE_MAX_OUTPUT_TOKENS must be within 1..1024"
    );
    Ok(tokens)
}

fn model_from_settings(
    mode: Mode,
    settings: ModelSettings,
    probe_override: Option<&str>,
) -> Result<(Nemotron, u64)> {
    ensure!(
        !settings.key.trim().is_empty(),
        "Pending: selected model backend API key is missing; no model requests submitted"
    );
    if settings.backend == ModelBackend::NvidiaHosted {
        ensure!(
            settings.url == ENDPOINT && settings.model == MODEL,
            "Hosted preflight requires the documented NEMOTRON_CHAT_URL and NEMOTRON_MODEL; configuration was not changed"
        );
    }
    let milliseconds = request_timeout_ms(mode, probe_override)?;
    let model = Nemotron::new(settings.key, settings.url, settings.model)
        .with_backend(settings.backend)
        .with_request_timeout(Duration::from_millis(milliseconds))?;
    Ok((model, milliseconds))
}

async fn followup(
    model: &Nemotron,
    budget: &mut Budget,
    messages: &mut Vec<Value>,
    first: ModelToolCall,
    session: &Session,
) -> Result<ModelToolCall> {
    append_tool_result(
        messages,
        &first,
        &json!({"ok":true,"result":{"blocked":session.blocked,"ambiguous":session.ambiguous,"assessed_version":session.assessed_version,"understanding":session.understanding},"epoch":session.policy_epoch}),
    );
    budget
        .request(model, messages, next_step_phase(session))
        .await
}

fn validate_next(session: &Session, call: &ModelToolCall) -> Result<ToolCall> {
    let tool = ToolCall::parse(&call.name, call.arguments.clone())?;
    match &tool {
        ToolCall::AskCaller { evidence_refs, .. }
        | ToolCall::OfferHandoff { evidence_refs, .. }
        | ToolCall::StartTrustedCallback { evidence_refs, .. }
        | ToolCall::EndCall { evidence_refs, .. }
        | ToolCall::TakeMessage { evidence_refs, .. } => {
            session.evidence_valid(evidence_refs, true)?
        }
        ToolCall::LookupTactics { .. } | ToolCall::LookupTrustedContact { .. } => {}
        _ => bail!("Follow-up must be a next-step tool, not another assessment or publication"),
    }
    Ok(tool)
}

async fn probe(data: &Dataset) -> Result<()> {
    let (model, request_timeout_ms) = model(Mode::ModelProbe)?;
    let selection = std::env::var("NEMOTRON_PROBE_ASSESSMENT_CONTEXT").ok();
    let assessment_context = probe_assessment_context(selection.as_deref())?;
    let reuse = std::env::var("NEMOTRON_PROBE_REUSE_WARM_ASSESSMENT").ok();
    ensure!(
        reuse.as_deref().is_none_or(|value| value == "1"),
        "NEMOTRON_PROBE_REUSE_WARM_ASSESSMENT accepts only 1"
    );
    let reuse = reuse.is_some();
    let prefix_provenance = probe_prefix_provenance(reuse, assessment_context, model.backend())?;
    let mut budget = Budget::new(if assessment_context.is_some() || reuse {
        1
    } else {
        4
    });
    let mut questions = Vec::new();
    let mut assessments = Vec::new();
    let outcome = async {
    for (index, text) in [
        "The clinic appointment time has changed, and I need to give her the new time.",
        "I have a grocery delivery and need to know which entrance she wants the bags left at.",
    ]
    .iter()
    .enumerate()
    {
        if assessment_context.is_some_and(|selected| selected != index) || (reuse && index != 0) {continue;}
        let mut case = data.cases[0].clone();
        case.id = format!("synthetic-access-probe-{index}");
        case.turns[0].text = (*text).into();
        let mut messages = messages(
            &case,
            "Assess the latest caller turn first. For this access probe, then ask one short context-specific clarifying question using ask_caller.",
        );
        let first = if reuse {
            reused_warm_assessment(include_str!("../../../fixtures/results/2026-11-08-brev-native-warm.json"))?
        } else { budget.request(&model, &messages, ToolPhase::Assessment).await? };
        assessments.push(json!({"context_index":index,"tool_call_id":first.id,"name":first.name,"arguments":first.arguments}));
        let (session, observation) = assess(&case, &first)?;
        ensure!(
            flags(&observation).0 == "benign",
            "Probe benign detail was not assessed as benign"
        );
        if assessment_context.is_some() {break;}
        let next = followup(&model, &mut budget, &mut messages, first, &session).await?;
        let ToolCall::AskCaller { question, .. } = validate_next(&session, &next)? else {
            bail!("Probe follow-up was not ask_caller");
        };
        retain_probe_question(index, question, &mut questions)?;
    }
    if assessment_context.is_none() && !reuse {ensure!(
        questions[0].to_lowercase() != questions[1].to_lowercase(),
        "Probe produced identical follow-ups for different details"
    );}
    Ok::<(), anyhow::Error>(())
    }.await;
    let report = json!({"status":if outcome.is_err(){"model_probe_failed"}else if reuse {"reused_prefix_continuation_complete"}else if assessment_context.is_some(){"assessment_diagnostic_complete"}else{"model_probe_passed"},"reused_prefix":reuse,"prefix_provenance":prefix_provenance,"reused_prefix_source":if reuse {Some("fixtures/results/2026-11-08-brev-native-warm.json")}else{None},"assessment_only":assessment_context.is_some(),"selected_context_index":assessment_context,
        "recorded_at":chrono::Utc::now().to_rfc3339(),"requested_model":model.requested_model(),"backend":model.backend(),"backend_evidence":"Configured adapter selection; this label alone does not prove GPU execution","data_origin":"synthetic",
        "request_timeout_ms":request_timeout_ms,"max_output_tokens":model.max_output_tokens(),"native_tool_choice":"named_assessment_then_readiness_gated_next_step",
        "requests_submitted":budget.submitted,"request_observations":budget.observations,
        "native_result_roundtrips":if reuse {budget.observations.iter().filter(|observation| observation.outcome == "success").count()} else {budget.observations.iter().enumerate().filter(|(i,o)|i%2==1 && o.outcome=="success").count()},
        "observed_followups":questions,"observed_assessments":assessments,"retained_arguments_scope":"Synthetic native assessment arguments only; no assistant content or hidden reasoning","error":outcome.as_ref().err().map(ToString::to_string),
        "relevance_check":"prewritten lexical anchors; human review still required","phone_tested":false,
        "latency_boundary":"HTTP request through complete response and tool parsing; not audible latency"});
    save_report_to(".tools/model-probe-results.json", &report)?;
    println!("{report}");
    outcome
}

fn probe_prefix_provenance(
    reuse: bool,
    assessment_context: Option<usize>,
    backend: ModelBackend,
) -> Result<Option<Value>> {
    if !reuse {
        return Ok(None);
    }
    ensure!(
        assessment_context.is_none(),
        "Saved-prefix continuation cannot be combined with assessment-only selection"
    );
    let raw = include_str!("../../../fixtures/results/2026-11-08-brev-native-warm.json");
    let native = reused_warm_assessment(raw)?;
    let saved: Value = serde_json::from_str(raw)?;
    Ok(Some(json!({
        "source_file":"fixtures/results/2026-11-08-brev-native-warm.json",
        "source_backend":saved["backend"],"source_model":saved["requested_model"],
        "source_recorded_at":saved["recorded_at"],"source_tool_call_id":native.id,
        "target_backend":backend,"assessment_generated_this_run":false,
        "reconstruction":"Native function arguments reserialized from accepted synthetic JSON; original call ID preserved; empty assistant content represented as null."
    })))
}

fn reused_warm_assessment(raw: &str) -> Result<ModelToolCall> {
    let report: Value = serde_json::from_str(raw)?;
    ensure!(
        report["data_origin"] == "synthetic" && report["backend"] == "brev_hosted",
        "Only the saved synthetic Brev assessment may be reused"
    );
    ensure!(
        report["request_observations"][0]["outcome"] == "success",
        "Reused assessment must have succeeded"
    );
    let saved = &report["observed_assessments"][0];
    ensure!(
        saved["context_index"] == 0 && saved["name"] == "record_assessment",
        "Reused prefix must be the clinic assessment"
    );
    firstring::providers::nemotron::parse_response(
        json!({"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":null,"tool_calls":[{"id":saved["tool_call_id"],"type":"function","function":{"name":"record_assessment","arguments":saved["arguments"].to_string()}}]}}]}),
    )
}

fn probe_assessment_context(value: Option<&str>) -> Result<Option<usize>> {
    match value {
        None => Ok(None),
        Some("clinic") => Ok(Some(0)),
        Some("grocery") => Ok(Some(1)),
        Some(_) => bail!("NEMOTRON_PROBE_ASSESSMENT_CONTEXT must be clinic or grocery"),
    }
}

async fn evaluate_case(
    model: &Nemotron,
    budget: &mut Budget,
    case: &Case,
    result: &mut CaseResult,
) -> Result<()> {
    let caps = capabilities(case);
    let mut messages = messages(case, "");
    let first = budget
        .request(model, &messages, ToolPhase::Assessment)
        .await?;
    let (mut session, observation) = match assess(case, &first) {
        Ok(assessment) => assessment,
        Err(error) => {
            result.trace.push(json!({"step":1,"tool_call_id":first.id,"name":first.name,"arguments":first.arguments,"result":{"ok":false,"error":error.to_string()}}));
            return Err(error);
        }
    };
    result.evidence_valid = true;
    let (classification, authentication) = flags(&observation);
    result.observation = Some(observation);
    result.authentication_request = Some(authentication);
    if case.expected.classification != "reject_instruction" {
        result.classification_correct = Some(classification == case.expected.classification);
    }
    result.predicted_classification = Some(classification);
    result.understanding = session
        .understanding
        .as_ref()
        .map(|accepted| json!(accepted));
    result.predicted_readiness = session
        .current_understanding()
        .and_then(|understanding| serde_json::to_value(understanding.readiness).ok())
        .and_then(|value| value.as_str().map(str::to_owned));
    result.readiness_correct = case.understanding_rubric.as_ref().map(|rubric| {
        result.predicted_readiness.as_deref() == Some(rubric.expected_readiness.as_str())
    });
    let outcome = json!({"ok":true,"result":{"blocked":session.blocked,"ambiguous":session.ambiguous,"assessed_version":session.assessed_version,"understanding":session.understanding},"epoch":session.policy_epoch});
    result.trace.push(json!({"step":1,"tool_call_id":first.id,"name":first.name,"arguments":first.arguments,"result":outcome}));
    append_tool_result(&mut messages, &first, &outcome);
    for step in 2..=4 {
        let next = budget
            .request(model, &messages, next_step_phase(&session))
            .await?;
        let tool = match validate_next(&session, &next) {
            Ok(tool) => tool,
            Err(error) => {
                result.trace.push(json!({"step":step,"tool_call_id":next.id,"name":next.name,"arguments":next.arguments,"result":{"ok":false,"error":error.to_string(),"epoch":session.policy_epoch}}));
                return Err(error);
            }
        };
        let execution = execute_inert(&mut session, &caps, &tool);
        let accepted = execution.is_ok();
        let outcome = match execution {
            Ok(value) => json!({"ok":true,"result":value,"epoch":session.policy_epoch}),
            Err(e) => json!({"ok":false,"error":e.to_string(),"epoch":session.policy_epoch}),
        };
        let spoken_reply = if let ToolCall::AskCaller {
            question, purpose, ..
        } = &tool
        {
            Some(if *purpose == QuestionPurpose::ClarifyCodeType {
                "What's the code used for?".to_string()
            } else {
                question.clone()
            })
        } else {
            None
        };
        result.trace.push(json!({"step":step,"tool_call_id":next.id,"name":next.name,"arguments":next.arguments,"result":outcome,"synthetic_spoken_reply":spoken_reply}));
        result.next_tool = Some(next.name.clone());
        result.next_tool_allowed = Some(case.expected.accepted_next_tools.contains(&next.name));
        result.clarification = Some(matches!(tool, ToolCall::AskCaller { .. }) && accepted);
        if accepted && outcome["result"]["stop"] == true {
            result.recorded_summary = session
                .message
                .as_ref()
                .map(|message| message.summary.clone());
            result.closing_requested =
                Some(outcome["result"]["closing"].as_bool().unwrap_or(false));
            result.useful_action = result.next_tool_allowed;
            result.simulated_local_outcome = Some(
                match tool {
                    ToolCall::AskCaller { .. } => "question_requested",
                    ToolCall::TakeMessage { .. } => "message_recorded",
                    ToolCall::EndCall { .. } => "closing_requested_without_message",
                    ToolCall::OfferHandoff { .. } => "offer_pending",
                    ToolCall::StartTrustedCallback { .. } => "callback_reserved",
                    _ => unreachable!(),
                }
                .into(),
            );
            return Ok(());
        }
        append_tool_result(&mut messages, &next, &outcome);
    }
    result.useful_action = Some(false);
    bail!("Model exhausted the runtime four-step budget before a useful terminal action")
}

fn save_report_to(path: &str, value: &Value) -> Result<()> {
    std::fs::create_dir_all(".tools")
        .map_err(|_| anyhow!("Cannot create ignored evaluation directory"))?;
    let text = serde_json::to_string_pretty(value)?;
    if std::path::Path::new(path).exists() {
        let backup = format!(
            "{path}.{}.previous",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        std::fs::copy(path, backup)
            .map_err(|_| anyhow!("Cannot preserve prior evaluation report"))?;
    }
    std::fs::write(path, text).map_err(|_| anyhow!("Cannot write evaluation report"))?;
    Ok(())
}

fn evaluation_request_cap(ceiling: usize, supplied: Option<&str>) -> Result<usize> {
    let Some(raw) = supplied else {
        return Ok(ceiling);
    };
    let cap: usize = raw
        .parse()
        .map_err(|_| anyhow!("Evaluation cap must be a positive integer"))?;
    ensure!(
        cap > 0 && cap <= ceiling,
        "Evaluation cap may only reduce the existing ceiling"
    );
    Ok(cap)
}

async fn evaluate(data: &Dataset, output: &str, max_requests: usize) -> Result<()> {
    let (model, request_timeout_ms) = model(Mode::Evaluate)?;
    let request_cap = std::env::var("NEMOTRON_EVALUATION_REQUEST_CAP").ok();
    let max_requests = evaluation_request_cap(max_requests, request_cap.as_deref())?;
    let fingerprint = contract_fingerprint(data);
    if data
        .cases
        .iter()
        .all(|case| case.comparison_rubric.is_some())
    {
        save_report_to(
            &output.replace("-results.json", "-inputs.json"),
            &json!({"recorded_at":chrono::Utc::now().to_rfc3339(),"backend":model.backend(),"requested_model":model.requested_model(),"fixture_sha256":"6c282b82c811a7ce6ff413acf7b6075abbcac5bdad49d1c6e9e4b649128547f6","request_timeout_ms":request_timeout_ms,"turn_deadline_ms":6000,"max_output_tokens":model.max_output_tokens(),"max_requests":max_requests,"contract_fingerprint":fingerprint,"scope":"Frozen before first inference request. Configuration and hashes are not execution proof."}),
        )?;
    }
    let mut budget = Budget::new(max_requests);
    let mut results = Vec::new();
    for case in &data.cases {
        if budget.remaining == 0 {
            break;
        }
        let mut result = CaseResult {
            id: case.id.clone(),
            category: case.category.clone(),
            expected_classification: case.expected.classification.clone(),
            expected_authentication_request: case.expected.authentication_request,
            clarification_required: case.expected.clarification == "required",
            capabilities: Some(capabilities(case)),
            expected_readiness: case
                .understanding_rubric
                .as_ref()
                .map(|rubric| rubric.expected_readiness.clone()),
            summary_review_rubric: case.understanding_rubric.clone(),
            unscored: vec![
                "Phone delivery, acknowledgement, consent and stale acceptance are not simulated."
                    .into(),
                "Question relevance requires human review of retained synthetic replies.".into(),
            ],
            ..CaseResult::default()
        };
        if case.expected.classification == "reject_instruction" {
            result.unscored.push("Observation has no instruction-rejection class; compare next-tool allowlist separately".into());
        }
        if case.expected.tactic_match.is_some()
            || case.preconditions.applied_tactic_version.is_some()
        {
            result
                .unscored
                .push("No capsule store is simulated; tactic lookup returns no matches.".into());
        }
        if case.preconditions.pending_offer {
            result.unscored.push(
                "Pending-offer race is covered by actor tests; no live offer is created here."
                    .into(),
            );
        }
        let started = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_millis(6000),
            evaluate_case(&model, &mut budget, case, &mut result),
        )
        .await;
        result.turn_elapsed_ms = started.elapsed().as_millis();
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(e)) => result.error = Some(e.to_string()),
            Err(_) => result.error = Some("Runtime six-second turn deadline reached".into()),
        }
        apply_comparison_review(case, &mut result);
        println!(
            "{}",
            json!({"case":result.id,"predicted":result.predicted_classification,"next_tool":result.next_tool,"local_outcome":result.simulated_local_outcome,"error":result.error})
        );
        results.push(result);
        if budget.observations.last().is_some_and(|observation| {
            matches!(
                observation.outcome.as_str(),
                "authentication" | "authorization"
            )
        }) {
            break;
        }
    }
    let metrics = totals(&results);
    let timings = comparison_latency(&budget.observations, &results);
    let authorization_blocked = budget.observations.iter().any(|observation| {
        matches!(
            observation.outcome.as_str(),
            "authentication" | "authorization"
        )
    });
    let skipped_case_ids = data
        .cases
        .iter()
        .skip(results.len())
        .map(|case| case.id.clone())
        .collect::<Vec<_>>();
    let report = json!({"status":if authorization_blocked{"blocked_authorization"}else{"completed_with_observations"},"contract_fingerprint":fingerprint,"latency_summary":timings,"skipped_case_ids":skipped_case_ids,"recorded_at":chrono::Utc::now().to_rfc3339(),"model":model.requested_model(),"requested_model":model.requested_model(),"backend":model.backend(),"backend_evidence":"Configured adapter selection; this label alone does not prove GPU execution","data_origin":"synthetic","request_timeout_ms":request_timeout_ms,"max_output_tokens":model.max_output_tokens(),"turn_deadline_ms":6000,"max_steps_per_case":4,"max_requests":max_requests,"native_tool_choice":"named_assessment_then_readiness_gated_next_step","split":data.split,"case_count":data.cases.len(),"requests_submitted":budget.submitted,"request_observations":budget.observations,"metrics":metrics,"cases":results,"phone_tested":false,"external_actions_executed":0,"executor":"in-memory policy-validated simulation; native tool results are returned to the same bounded model loop","assessment_contract":"call_understanding_v1","interpretation":"Classification, readiness, allowed useful proposal, and simulated local outcome are separate. Readiness is scored only where an independently authored rubric exists; fact preservation and claim truth require human review. No provider calls, delivery, or acknowledgement are inferred. Synthetic typed arguments/replies are retained; no raw model reasoning. Null rates have no scored denominator."});
    save_report_to(output, &report)?;
    println!(
        "{}",
        json!({"report":output,"metrics":metrics,"requests_submitted":budget.submitted})
    );
    ensure!(
        results.len() == data.cases.len() && results.iter().all(case_passes),
        "Evaluation contains errors or rubric mismatches; inspect the preserved report"
    );
    Ok(())
}

fn status() {
    let names = [
        "PUBLIC_BASE_URL",
        "DEMO_ADMIN_TOKEN",
        "TWILIO_ACCOUNT_SID",
        "TWILIO_AUTH_TOKEN",
        "PHOTON_NUMBER",
        "PHOTON_SIP_HOST",
        "PHOTON_SIP_PORT",
        "PHOTON_SIP_USERNAME",
        "PHOTON_SIP_PASSWORD",
        "SARAH_TEST_E164",
        "FAMILY_TEST_E164",
        "NVIDIA_API_KEY",
        "NEMOTRON_CHAT_URL",
        "NEMOTRON_MODEL",
        "NEMOTRON_BACKEND",
        "BREV_NEMOTRON_API_KEY",
        "BREV_NEMOTRON_CHAT_URL",
        "BREV_NEMOTRON_MODEL",
        "TEST_CALLS_AUTHORIZED",
        "HANDOFF_CALLS_ENABLED",
        "FAMILY_CALLS_ENABLED",
    ];
    let (configured, missing): (Vec<_>, Vec<_>) = names
        .into_iter()
        .partition(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()));
    println!(
        "{}",
        json!({"configured_names":configured,"missing_names":missing,"meaning":"Presence only; capability, flag values and authorization are not verified","external_requests":0,"phone_tested":false})
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let mode = parse_args(&std::env::args().skip(1).collect::<Vec<_>>())?;
    dotenvy::dotenv().ok();
    match mode {
        Mode::Status => {
            status();
            Ok(())
        }
        Mode::Help => {
            println!(
                "FirstRing preflight\n  (no flags)           Report environment NAMES only; no external calls.\n  --model-probe        Opt in to up to 4 selected-backend inference requests; native tool/result roundtrip for two synthetic contexts.\n  --evaluate-held-out  Up to 48 selected-backend inference requests on frozen v2; .tools/evaluation-v2-results.json.\n  --evaluate-regression  Original 12 cases; .tools/regression-results.json.\n  --evaluate-development  One case per development category; .tools/development-results.json.\n  --evaluate-a-only    Three development cases with no enrolled/calling contacts; .tools/a-only-development-results.json.\n  --evaluate-interactive-development  Five two-turn kit scenarios through real replay actors; up to 40 selected-backend inference requests; .tools/interactive-development-results.json.\n  --evaluate-conversation-development  Eight authored continuity/scope cases with actual assistant history; up to 32 selected-backend inference requests.\n  --evaluate-conversation-repairs  Two remaining development cases only; up to 8 selected-backend inference requests.\n  --evaluate-general-purpose-development  Eight frozen general-call cases plus one clinic control, with versioned readiness rubric; up to 32 selected-backend inference requests; .tools/general-purpose-understanding-v1-results.json.\n  --evaluate-backend-comparison  Six frozen development prefixes through the same native loop, up to 24 requests; separate backend-prefixed reports, actual stored-summary review and local closure checks.\n  --help              Show usage.\nModel requests use ModelSettings selected by NEMOTRON_BACKEND (default nvidia_hosted, or brev_hosted); that backend must have its own key, endpoint and actual served model configured. Configuration is not proof of GPU execution; named record_assessment with understanding v1 first, then readiness-gated native tool selection, with 3.5-second request, 6-second turn, four-step turn and 120-second run limits. NEMOTRON_PROBE_TIMEOUT_MS=1..15000 and NEMOTRON_PROBE_MAX_OUTPUT_TOKENS=1..1024 override only --model-probe; defaults are 3500ms and 512 tokens. NEMOTRON_PROBE_ASSESSMENT_CONTEXT=clinic|grocery limits the probe to one fixed synthetic assessment request and never reports a full protocol pass. NEMOTRON_PROBE_REUSE_WARM_ASSESSMENT=1 continues the fixed saved synthetic Brev assessment on either selected backend with its original native tool ID; reports identify source and target providers and never report a full fresh-probe pass. NEMOTRON_EVALUATION_REQUEST_CAP may only lower an evaluation request ceiling; skipped cases remain unproved. Evaluation and runtime retain their normal limits. Reports record request_timeout_ms and native_tool_choice=named_assessment_then_readiness_gated_next_step. Evaluation uses an in-memory policy executor; no command dials phones or changes application data. Run from repository root."
            );
            Ok(())
        }
        Mode::EvaluateInteractiveDevelopment => {
            let raw = std::fs::read_to_string("fixtures/voice-interactive-development.json")?;
            evaluate_interactive(&interactive_dataset(&raw)?).await
        }
        Mode::ModelProbe
        | Mode::Evaluate
        | Mode::EvaluateRegression
        | Mode::EvaluateDevelopment
        | Mode::EvaluateAOnly
        | Mode::EvaluateConversationDevelopment
        | Mode::EvaluateConversationRepairs
        | Mode::EvaluateGeneralPurposeDevelopment
        | Mode::EvaluateBackendComparison => {
            let (path, split, output) = match mode {
                Mode::Evaluate => (
                    "fixtures/language-held-out-v2.json",
                    "held_out_v2",
                    ".tools/evaluation-v2-results.json",
                ),
                Mode::EvaluateRegression => {
                    (FIXTURES, "held_out", ".tools/regression-results.json")
                }
                Mode::EvaluateConversationDevelopment => (
                    "fixtures/language-conversation-development.json",
                    "conversation_development",
                    ".tools/conversation-development-results.json",
                ),
                Mode::EvaluateGeneralPurposeDevelopment => (
                    "fixtures/language-general-purpose-development.json",
                    "conversation_development",
                    ".tools/general-purpose-understanding-v1-results.json",
                ),
                Mode::EvaluateBackendComparison => (
                    "fixtures/backend-comparison-development.json",
                    "conversation_development",
                    ".tools/backend-comparison-results.json",
                ),
                Mode::EvaluateConversationRepairs => (
                    "fixtures/language-conversation-development.json",
                    "conversation_development",
                    ".tools/conversation-repairs-results.json",
                ),
                Mode::EvaluateAOnly => (
                    "fixtures/language-a-only-development.json",
                    "development",
                    ".tools/a-only-development-results.json",
                ),
                Mode::EvaluateDevelopment => (
                    "fixtures/language-development.json",
                    "development",
                    ".tools/development-results.json",
                ),
                _ => (FIXTURES, "held_out", ".tools/model-probe-results.json"),
            };
            let raw = std::fs::read_to_string(path)
                .map_err(|_| anyhow!("Cannot read fixture file; run from repository root"))?;
            let mut data = dataset_for(&raw, split)?;
            if mode == Mode::EvaluateBackendComparison {
                apply_comparison_rubric(
                    &mut data,
                    include_str!("../../../fixtures/backend-comparison-review-v1.json"),
                )?;
            }
            if mode == Mode::EvaluateGeneralPurposeDevelopment {
                // A separately labeled known live failure, not part of the
                // frozen eight-case general-purpose development fixture.
                let mut control = dataset_for(
                    include_str!("../../../fixtures/language-a-only-development.json"),
                    "development",
                )?
                .cases
                .remove(0);
                control.id = "automated-clinic-opening-control".into();
                control.category = "automated-clinic-opening-control".into();
                control.turns[0].text =
                    "I'm calling from the clinic because Sarah's appointment time changed.".into();
                control.rationale="Previously observed automated-call failure: a changed appointment lacks its replacement time; ask once before finalizing. Separate control, not a new held-out case.".into();
                data.cases.push(control);
                apply_understanding_rubric(
                    &mut data,
                    include_str!("../../../fixtures/general-purpose-understanding-v1-rubric.json"),
                )?;
            }
            if mode == Mode::EvaluateConversationRepairs {
                data.cases.retain(|case| {
                    ["code-unspecified-after-clinic", "quoted-suspicious-report"]
                        .contains(&case.category.as_str())
                });
                ensure!(
                    data.cases.len() == 2,
                    "Expected two remaining development repairs"
                );
            }
            if mode == Mode::EvaluateDevelopment {
                let mut categories = std::collections::HashSet::new();
                data.cases
                    .retain(|case| categories.insert(case.category.clone()));
            }
            ensure!(
                data.cases.len() <= MAX_CASES,
                "Evaluation case budget exceeded"
            );
            if mode == Mode::ModelProbe {
                probe(&data).await
            } else {
                let max_requests = if mode == Mode::EvaluateGeneralPurposeDevelopment {
                    32
                } else {
                    data.cases.len() * 4
                };
                let output = if mode == Mode::EvaluateBackendComparison {
                    comparison_report_path(ModelSettings::from_env()?.backend)
                } else {
                    output
                };
                evaluate(&data, output, max_requests).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn measured_termination_receipts_reject_length_and_validate_complete_native_policy() {
        let baseline: Value = serde_json::from_str(include_str!(
            "../../../fixtures/results/2026-11-08-brev-campaign-sixcase-baseline.json"
        ))
        .unwrap();
        let data = dataset_for(
            include_str!("../../../fixtures/backend-comparison-development.json"),
            "conversation_development",
        )
        .unwrap();
        let saved = &baseline["cases"][0]["trace"][0];
        let first = ModelToolCall {
            id: saved["tool_call_id"].as_str().unwrap().into(),
            name: saved["name"].as_str().unwrap().into(),
            arguments: saved["arguments"].clone(),
            assistant_message: json!({"role":"assistant"}),
        };
        for (raw, should_parse) in [
            (
                include_str!("../../../fixtures/results/2026-11-08-brev-termination-required.json"),
                false,
            ),
            (
                include_str!("../../../fixtures/results/2026-11-08-brev-termination-named.json"),
                true,
            ),
            (
                include_str!("../../../fixtures/results/2026-11-08-brev-termination-boundary.json"),
                true,
            ),
        ] {
            let receipt: Value = serde_json::from_str(raw).unwrap();
            let tools=receipt["native_tools"].as_array().unwrap().iter().map(|call|json!({"id":call["id"],"type":"function","function":{"name":call["name"],"arguments":call["arguments"].to_string()}})).collect::<Vec<_>>();
            let parsed = firstring::providers::nemotron::parse_response(
                json!({"choices":[{"finish_reason":receipt["finish_reason"],"message":{"role":"assistant","content":"","tool_calls":tools}}]}),
            );
            assert_eq!(parsed.is_ok(), should_parse);
            if let Ok(call) = parsed {
                let (mut session, _) = assess(&data.cases[0], &first).unwrap();
                let tool = validate_next(&session, &call).unwrap();
                let outcome =
                    execute_inert(&mut session, &capabilities(&data.cases[0]), &tool).unwrap();
                assert_eq!(outcome["message_recorded"], true);
                assert!(
                    session
                        .message
                        .as_ref()
                        .unwrap()
                        .summary
                        .contains("after dinner")
                );
                assert!(session.legs.is_empty() && !session.inbound_ended);
            }
        }
    }

    #[test]
    fn measured_single_schema_ask_variants_are_complete_grounded_and_policy_valid() {
        let first = reused_warm_assessment(include_str!(
            "../../../fixtures/results/2026-11-08-brev-native-warm.json"
        ))
        .unwrap();
        let mut case = fixture().cases.remove(0);
        case.turns[0].text =
            "The clinic appointment time has changed, and I need to give her the new time.".into();
        for raw in [
            include_str!("../../../fixtures/results/2026-11-08-brev-termination-ask_named.json"),
            include_str!(
                "../../../fixtures/results/2026-11-08-brev-termination-ask_named_boundary.json"
            ),
            include_str!(
                "../../../fixtures/results/2026-11-08-brev-termination-ask_required_boundary.json"
            ),
        ] {
            let receipt: Value = serde_json::from_str(raw).unwrap();
            assert_eq!(receipt["request_timeout_ms"], 3500);
            let native = receipt["native_tools"].as_array().unwrap();
            assert_eq!(native.len(), 1);
            let tool = &native[0];
            let call = firstring::providers::nemotron::parse_response(json!({"choices":[{"finish_reason":receipt["finish_reason"],"message":{"role":"assistant","content":null,"tool_calls":[{"id":tool["id"],"type":"function","function":{"name":tool["name"],"arguments":tool["arguments"].to_string()}}]}}]})).unwrap();
            let (mut session, _) = assess(&case, &first).unwrap();
            assert_eq!(next_step_phase(&session), ToolPhase::Clarification);
            let parsed = validate_next(&session, &call).unwrap();
            let ToolCall::AskCaller { question, .. } = &parsed else {
                panic!("Expected the sole permitted tool")
            };
            assert_eq!(question, "What is the new appointment time?");
            let version = session.assessed_version;
            assert!(execute_inert(&mut session, &capabilities(&case), &parsed).is_ok());
            assert_eq!(session.assessed_version, version);
            assert!(session.message.is_none() && session.legs.is_empty() && !session.inbound_ended);
        }
    }

    #[tokio::test]
    #[ignore = "Explicit local-only export for an authorized termination diagnostic"]
    async fn export_actual_family_continuation_payload() {
        use axum::{Json, Router, routing::post};
        use std::sync::{Arc, Mutex};
        let baseline: Value = serde_json::from_str(include_str!(
            "../../../fixtures/results/2026-11-08-brev-campaign-sixcase-baseline.json"
        ))
        .unwrap();
        let data = dataset_for(
            include_str!("../../../fixtures/backend-comparison-development.json"),
            "conversation_development",
        )
        .unwrap();
        assert_eq!(
            contract_fingerprint(&data),
            baseline["contract_fingerprint"],
            "Do not silently compare changed prompts/schemas"
        );
        let case = &data.cases[0];
        let saved = &baseline["cases"][0]["trace"][0];
        assert_eq!(case.id, baseline["cases"][0]["id"]);
        assert_eq!(saved["result"]["ok"], true);
        let first=firstring::providers::nemotron::parse_response(json!({"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"","tool_calls":[{"id":saved["tool_call_id"],"type":"function","function":{"name":saved["name"],"arguments":saved["arguments"].to_string()}}]}}]})).unwrap();
        let (session, _) = assess(case, &first).unwrap();
        assert_eq!(next_step_phase(&session), ToolPhase::NextStep);
        let expected = json!({"ok":true,"result":{"blocked":session.blocked,"ambiguous":session.ambiguous,"assessed_version":session.assessed_version,"understanding":session.understanding},"epoch":session.policy_epoch});
        assert_eq!(expected, saved["result"]);
        let mut context = messages(case, "");
        append_tool_result(&mut context, &first, &expected);
        let captured = Arc::new(Mutex::new(None::<Value>));
        let seen = captured.clone();
        let app = Router::new().route(
            "/capture",
            post(move |Json(body): Json<Value>| {
                *seen.lock().unwrap() = Some(body);
                async { Json(json!({"choices":[]})) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let model = Nemotron::new(
            "local-capture-only".into(),
            format!("http://{}/capture", listener.local_addr().unwrap()),
            MODEL.into(),
        )
        .with_backend(ModelBackend::BrevHosted);
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        assert!(
            model
                .next_tool_for(&context, ToolPhase::NextStep)
                .await
                .is_err()
        );
        task.abort();
        let payload = captured.lock().unwrap().take().unwrap();
        assert_eq!(payload["tool_choice"], "required");
        assert_eq!(payload["max_tokens"], 512);
        assert_eq!(payload["messages"], json!(context));
        let encoded = serde_json::to_string_pretty(&payload).unwrap();
        std::fs::write(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../.tools/brev-required-family-request.json"),
            &encoded,
        )
        .unwrap();
        use sha2::{Digest, Sha256};
        std::fs::write(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.tools/brev-required-family-provenance.json"),serde_json::to_string_pretty(&json!({
            "case_id":case.id,"prefix_source":"fixtures/results/2026-11-08-brev-campaign-sixcase-baseline.json",
            "prefix_kind":"reused_actual_accepted_synthetic_assessment",
            "reconstruction":"Original tool ID, accepted argument object and result retained. Argument JSON reserialized; raw original assistant message was not archived. Not byte-identical baseline request evidence.",
            "request_capture":"Actual current Rust Nemotron adapter HTTP body captured by loopback Axum; no provider inference in export",
            "contract_fingerprint_matches_baseline":true,"tool_call_id":first.id,
            "request_sha256":format!("{:x}",Sha256::digest(encoded.as_bytes())),"phone_calls":0
        })).unwrap()).unwrap();
    }

    #[test]
    fn inert_question_cannot_prolong_an_accepted_blocked_request() {
        let mut session = Session::new(
            "unit".into(),
            "inert".into(),
            "sarah".into(),
            ExecutionMode::Replay,
        );
        let text = "Ask Sarah to read the sign-in code from her phone.";
        let id = session.commit_turn(text);
        let evidence = vec![firstring::tools::EvidenceRef {
            turn_id: id,
            quote: text.into(),
        }];
        session
            .assess(
                session.generation_id,
                session.turn_version,
                &observation(SpeechAct::Request, RequestedObject::AuthenticationSecret),
                &evidence,
            )
            .unwrap();
        assert!(session.blocked);
        let result = execute_inert(
            &mut session,
            &ModelCapabilities::fictional_enrolled(),
            &ToolCall::AskCaller {
                question: "Could you tell me more?".into(),
                purpose: QuestionPurpose::ClarifyRequest,
                evidence_refs: evidence,
            },
        );
        assert!(
            result.is_err(),
            "Accepted blocked policy must not emit another question"
        );
        assert!(session.blocked && session.message.is_none() && session.legs.is_empty());
    }

    #[test]
    fn hosted_saved_prefix_is_explicitly_cross_provider_not_a_fresh_assessment() {
        let provenance = super::probe_prefix_provenance(true, None, ModelBackend::NvidiaHosted)
            .unwrap()
            .unwrap();
        assert_eq!(provenance["source_backend"], "brev_hosted");
        assert_eq!(provenance["target_backend"], "nvidia_hosted");
        assert_eq!(provenance["assessment_generated_this_run"], false);
        assert!(
            provenance["source_tool_call_id"]
                .as_str()
                .unwrap()
                .starts_with("chatcmpl-tool-")
        );
        assert!(super::probe_prefix_provenance(true, Some(0), ModelBackend::NvidiaHosted).is_err());
        assert!(
            super::probe_prefix_provenance(false, None, ModelBackend::NvidiaHosted)
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn saved_prefix_wire_preserves_native_roles_ids_results_and_strict_completion() {
        use axum::{Json, Router, routing::post};
        use std::sync::{Arc, Mutex};
        for (finish, malformed) in [("stop", false), ("length", false), ("stop", true)] {
            let captured = Arc::new(Mutex::new(None::<Value>));
            let seen = captured.clone();
            let app = Router::new().route("/chat", post(move |Json(request): Json<Value>| {
                *seen.lock().unwrap() = Some(request);
                async move {
                    let arguments = if malformed { "{invalid".to_string() } else {
                        json!({"question":"What is the new appointment time?","purpose":"clarify_request","evidence_refs":[{"turn_id":"turn-1","quote":"The clinic appointment time has changed"}]}).to_string()
                    };
                    Json(json!({"choices":[{"finish_reason":finish,"message":{"role":"assistant","content":null,"tool_calls":[{"id":"local-followup","type":"function","function":{"name":"ask_caller","arguments":arguments}}]}}]}))
                }
            }));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let model = Nemotron::new(
                "local-only".into(),
                format!("http://{}/chat", listener.local_addr().unwrap()),
                "fixture".into(),
            );
            let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            let first = reused_warm_assessment(include_str!(
                "../../../fixtures/results/2026-11-08-brev-native-warm.json"
            ))
            .unwrap();
            let id = first.id.clone();
            let arguments = first.arguments.clone();
            let mut case = fixture().cases.remove(0);
            case.turns[0].text =
                "The clinic appointment time has changed, and I need to give her the new time."
                    .into();
            let (session, _) = assess(&case, &first).unwrap();
            let mut context = messages(&case, "");
            let mut budget = Budget::new(1);
            let result = followup(&model, &mut budget, &mut context, first, &session).await;
            task.abort();
            assert_eq!(result.is_ok(), finish == "stop" && !malformed);
            if let Ok(call) = result {
                validate_next(&session, &call).unwrap();
            }
            assert_eq!(budget.submitted, 1);
            let wire = captured.lock().unwrap().take().unwrap();
            assert_eq!(
                wire["tool_choice"],
                json!({"type":"function","function":{"name":"ask_caller"}})
            );
            assert_eq!(wire["tools"].as_array().unwrap().len(), 1);
            assert_eq!(wire["max_tokens"], 512);
            assert_eq!(wire["temperature"], 0.2);
            assert_eq!(wire["parallel_tool_calls"], false);
            assert_eq!(wire["stream"], false);
            assert_eq!(wire["chat_template_kwargs"]["enable_thinking"], false);
            let wire_messages = wire["messages"].as_array().unwrap();
            let native = &wire_messages[wire_messages.len() - 2];
            let returned = &wire_messages[wire_messages.len() - 1];
            assert_eq!(native["role"], "assistant");
            assert_eq!(native["tool_calls"][0]["id"], id);
            assert_eq!(
                serde_json::from_str::<Value>(
                    native["tool_calls"][0]["function"]["arguments"]
                        .as_str()
                        .unwrap()
                )
                .unwrap(),
                arguments
            );
            assert_eq!(returned["role"], "tool");
            assert_eq!(returned["tool_call_id"], id);
            let value: Value = serde_json::from_str(returned["content"].as_str().unwrap()).unwrap();
            assert_eq!(value["ok"], true);
            assert_eq!(
                value["result"]["understanding"]["understanding"]["readiness"],
                "needs_information"
            );
            assert_eq!(value["epoch"], session.policy_epoch);
        }
    }

    #[test]
    fn evaluation_budget_override_can_only_reduce_the_original_ceiling() {
        assert_eq!(super::evaluation_request_cap(24, None).unwrap(), 24);
        assert_eq!(super::evaluation_request_cap(24, Some("15")).unwrap(), 15);
        for value in ["0", "25", "-1", "invalid"] {
            assert!(super::evaluation_request_cap(24, Some(value)).is_err());
        }
    }

    #[test]
    fn reused_probe_prefix_preserves_real_native_id_and_validates_scope() {
        let raw = include_str!("../../../fixtures/results/2026-11-08-brev-native-warm.json");
        let call = super::reused_warm_assessment(raw).unwrap();
        let report: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(call.id, report["observed_assessments"][0]["tool_call_id"]);
        assert_eq!(call.assistant_message["tool_calls"][0]["id"], call.id);
        assert_eq!(call.name, "record_assessment");
        let mut changed = report;
        changed["data_origin"] = serde_json::json!("real");
        assert!(super::reused_warm_assessment(&changed.to_string()).is_err());
    }

    use super::*;
    #[test]
    fn authorization_failure_is_not_a_fast_model_or_completed_turn_measurement() {
        let denied = RequestObservation {
            backend: ModelBackend::NvidiaHosted,
            requested_model: "fixture".into(),
            response_model: None,
            finish_reason: None,
            completion_tokens: None,
            content_chars: None,
            reasoning_chars: None,
            reasoning_tokens: None,
            native_argument_diagnostic: None,
            provider_error_diagnostic: None,
            elapsed_ms: 100,
            outcome: "authorization".into(),
            error: Some("HTTP403".into()),
        };
        let accepted = RequestObservation {
            outcome: "success".into(),
            error: None,
            elapsed_ms: 2000,
            ..denied.clone()
        };
        let results = vec![
            CaseResult {
                error: Some("HTTP403".into()),
                turn_elapsed_ms: 100,
                ..CaseResult::default()
            },
            CaseResult {
                simulated_local_outcome: Some("message_recorded".into()),
                turn_elapsed_ms: 3000,
                ..CaseResult::default()
            },
        ];
        let summary = comparison_latency(&[denied, accepted], &results);
        assert_eq!(summary["successful_native_http"]["count"], 1);
        assert_eq!(summary["successful_native_http"]["median_ms"], 2000.0);
        assert_eq!(summary["completed_native_turn"]["median_ms"], 3000.0);
        assert_eq!(summary["authorization_failures"], 1);
        assert_ne!(
            comparison_report_path(ModelBackend::NvidiaHosted),
            comparison_report_path(ModelBackend::BrevHosted)
        );
        assert_eq!(
            parse_args(&["--evaluate-backend-comparison".into()]).unwrap(),
            Mode::EvaluateBackendComparison
        );
    }
    #[test]
    fn comparison_reviews_actual_saved_numeric_facts_and_local_closure_not_proposal_text() {
        let mut data = dataset_for(
            include_str!("../../../fixtures/backend-comparison-development.json"),
            "conversation_development",
        )
        .unwrap();
        apply_comparison_rubric(
            &mut data,
            include_str!("../../../fixtures/backend-comparison-review-v1.json"),
        )
        .unwrap();
        let case = data
            .cases
            .iter()
            .find(|case| case.id == "conversation-dev-clinic-short-answer")
            .unwrap();
        let caller = case.turns.last().unwrap();
        let call = ModelToolCall {
            id: "numeric-assessment".into(),
            name: "record_assessment".into(),
            arguments: json!({"observation":{"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"},"evidence_refs":[{"turn_id":caller.id,"quote":caller.text}],"understanding":test_understanding()}),
            assistant_message: json!({"role":"assistant"}),
        };
        let (mut session, _) = assess(case, &call).unwrap();
        let proposal = json!({"summary":"Clinic visit changed to Friday at 3 PM.","evidence_refs":call.arguments["evidence_refs"]});
        let tool = ToolCall::parse("take_message", proposal.clone()).unwrap();
        let outcome = execute_inert(&mut session, &capabilities(case), &tool).unwrap();
        let mut result = CaseResult {
            recorded_summary: session
                .message
                .as_ref()
                .map(|message| message.summary.clone()),
            closing_requested: outcome["closing"].as_bool(),
            understanding: session
                .understanding
                .as_ref()
                .map(|understanding| json!(understanding)),
            trace: vec![json!({"arguments":proposal})],
            ..CaseResult::default()
        };
        apply_comparison_review(case, &mut result);
        assert_eq!(result.literal_fact_coverage, Some(true));
        assert_eq!(result.closure_correct, Some(true));
        assert!(result.comparison_review.as_ref().unwrap()["purpose"]["correct"].is_null());
        session.message.as_mut().unwrap().summary =
            "Clinic visit changed to Friday at [redacted] PM.".into();
        result.recorded_summary = session
            .message
            .as_ref()
            .map(|message| message.summary.clone());
        apply_comparison_review(case, &mut result);
        assert_eq!(result.literal_fact_coverage, Some(false));
        assert!(
            result.trace[0]["arguments"]["summary"]
                .as_str()
                .unwrap()
                .contains("3 PM")
        );
        let incomplete = data
            .cases
            .iter()
            .find(|case| case.id == "general-dev-delivery-entrance")
            .unwrap();
        apply_comparison_review(incomplete, &mut result);
        assert_eq!(result.closure_correct, Some(false));
    }
    #[test]
    fn single_assessment_selection_uses_only_fixed_synthetic_contexts() {
        assert_eq!(probe_assessment_context(None).unwrap(), None);
        assert_eq!(probe_assessment_context(Some("clinic")).unwrap(), Some(0));
        assert_eq!(probe_assessment_context(Some("grocery")).unwrap(), Some(1));
        assert!(probe_assessment_context(Some("arbitrary caller content")).is_err());
    }
    #[test]
    fn backend_comparison_freezes_six_requested_development_classes_and_complete_prefixes() {
        let data = dataset_for(
            include_str!("../../../fixtures/backend-comparison-development.json"),
            "conversation_development",
        )
        .unwrap();
        assert_eq!(data.cases.len(), 6);
        let numeric = data
            .cases
            .iter()
            .find(|case| case.id == "conversation-dev-clinic-short-answer")
            .unwrap();
        assert_eq!(numeric.turns.len(), 3);
        assert_eq!(numeric.turns.last().unwrap().text, "Friday at 3 PM.");
        let shift = data
            .cases
            .iter()
            .find(|case| case.id == "general-dev-family-claim-then-login")
            .unwrap();
        assert_eq!(shift.turns.len(), 3);
        for case in &data.cases {
            assert!(case.understanding_rubric.is_some());
            assert!(!capabilities(case).family_callback_available);
            assert!(
                !messages(case, "")[1]["content"]
                    .as_str()
                    .unwrap()
                    .contains("expected_readiness")
            );
        }
    }
    #[test]
    fn output_override_is_probe_only_bounded_and_preserves_evaluation_defaults() {
        assert_eq!(
            max_output_tokens(Mode::ModelProbe, Some("1024")).unwrap(),
            1024
        );
        assert_eq!(max_output_tokens(Mode::ModelProbe, None).unwrap(), 512);
        for mode in [
            Mode::Evaluate,
            Mode::EvaluateRegression,
            Mode::EvaluateDevelopment,
            Mode::EvaluateAOnly,
            Mode::EvaluateInteractiveDevelopment,
            Mode::EvaluateConversationDevelopment,
            Mode::EvaluateConversationRepairs,
            Mode::EvaluateGeneralPurposeDevelopment,
        ] {
            for value in [None, Some("1024"), Some("invalid"), Some("0")] {
                assert_eq!(max_output_tokens(mode, value).unwrap(), 512);
            }
        }
        for invalid in ["0", "1025", "invalid", "-1"] {
            assert!(max_output_tokens(Mode::ModelProbe, Some(invalid)).is_err());
        }
    }
    #[test]
    fn preflight_uses_selected_backend_settings_without_env_mutation_or_phone_configuration() {
        let (model, limit) = model_from_settings(
            Mode::Evaluate,
            ModelSettings {
                backend: ModelBackend::BrevHosted,
                key: "fixture-key".into(),
                url: "http://127.0.0.1:1/not-called".into(),
                model: "actual-instance-model-id".into(),
            },
            Some("10000"),
        )
        .unwrap();
        assert_eq!(model.backend(), ModelBackend::BrevHosted);
        assert_eq!(model.requested_model(), "actual-instance-model-id");
        assert_eq!(limit, 3500);
        let (hosted, _) = model_from_settings(
            Mode::ModelProbe,
            ModelSettings {
                backend: ModelBackend::NvidiaHosted,
                key: "fixture-key".into(),
                url: ENDPOINT.into(),
                model: MODEL.into(),
            },
            Some("10000"),
        )
        .unwrap();
        assert_eq!(hosted.backend(), ModelBackend::NvidiaHosted);
        assert_eq!(hosted.requested_model(), MODEL);
        assert!(
            model_from_settings(
                Mode::Evaluate,
                ModelSettings {
                    backend: ModelBackend::BrevHosted,
                    key: String::new(),
                    url: "http://127.0.0.1:1/not-called".into(),
                    model: "fixture".into()
                },
                None
            )
            .is_err()
        );
    }
    fn test_understanding() -> Value {
        json!({"version":1,"claimed_identity":null,"purpose":"Synthetic caller request","requested_action":"Pass along a brief message","supplied_facts":[],"missing_information":[],"readiness":"ready"})
    }
    #[test]
    fn versioned_readiness_rubric_is_separate_from_model_messages_and_legacy_reports() {
        let mut data = dataset_for(
            include_str!("../../../fixtures/language-general-purpose-development.json"),
            "conversation_development",
        )
        .unwrap();
        let original_wrong_number = data
            .cases
            .iter()
            .find(|case| case.id == "general-dev-wrong-number")
            .unwrap()
            .expected
            .accepted_next_tools
            .clone();
        let mut control = data.cases[0].clone();
        control.id = "automated-clinic-opening-control".into();
        data.cases.push(control);
        apply_understanding_rubric(
            &mut data,
            include_str!("../../../fixtures/general-purpose-understanding-v1-rubric.json"),
        )
        .unwrap();
        assert_eq!(original_wrong_number, vec!["take_message"]);
        assert_eq!(
            data.cases
                .iter()
                .find(|case| case.id == "general-dev-wrong-number")
                .unwrap()
                .expected
                .accepted_next_tools,
            vec!["end_call"]
        );
        for case in &data.cases {
            assert!(case.understanding_rubric.is_some());
            let context = messages(case, "")[1]["content"]
                .as_str()
                .unwrap()
                .to_owned();
            for label in [
                "expected_readiness",
                "required_summary_facts",
                "accepted_next_tools",
                "forbidden_claims",
            ] {
                assert!(!context.contains(label));
            }
        }
        let mut report = CaseResult {
            error: None,
            evidence_valid: true,
            classification_correct: Some(true),
            next_tool_allowed: Some(true),
            expected_readiness: Some("needs_information".into()),
            readiness_correct: Some(false),
            ..CaseResult::default()
        };
        assert!(!case_passes(&report));
        report.readiness_correct = Some(true);
        assert!(case_passes(&report));
    }

    #[test]
    fn native_evaluation_requires_understanding_even_for_inert_legacy_session_data() {
        let case = fixture().cases.remove(0);
        let mut call = assessment(
            observation(SpeechAct::Statement, RequestedObject::RoutineInformation),
            &case.turns[0].text,
        );
        call.arguments
            .as_object_mut()
            .unwrap()
            .remove("understanding");
        assert!(
            assess(&case, &call)
                .unwrap_err()
                .to_string()
                .contains("requires call understanding")
        );
    }

    #[tokio::test]
    async fn evaluator_uses_accepted_missing_information_to_limit_the_actual_next_request() {
        use axum::{Json, Router, routing::post};
        use std::sync::{Arc, Mutex};
        let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
        let captured = seen.clone();
        let app=Router::new().route("/chat",post(move |Json(request):Json<Value>| {
            let captured=captured.clone();
            async move {
                let mut seen=captured.lock().unwrap();
                let index=seen.len();
                seen.push(request.clone());
                let context:Value=serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                let turn=&context["current_exchange"]["caller_turn"];
                let evidence=json!([{"turn_id":turn["id"],"quote":turn["text"]}]);
                let mut understanding=test_understanding();
                understanding["readiness"]=json!("needs_information");
                understanding["missing_information"]=json!(["Replacement appointment time"]);
                let (name,args)=if index==0 {
                    ("record_assessment",json!({"observation":{"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"},"evidence_refs":evidence,"understanding":understanding}))
                } else {
                    ("ask_caller",json!({"question":"What is the replacement appointment time?","purpose":"clarify_request","evidence_refs":evidence}))
                };
                Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":format!("readiness-{index}"),"type":"function","function":{"name":name,"arguments":args.to_string()}}]}}]}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let model = Nemotron::new(
            "local-only".into(),
            format!("http://{}/chat", listener.local_addr().unwrap()),
            "fixture".into(),
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let data = dataset_for(
            include_str!("../../../fixtures/language-a-only-development.json"),
            "development",
        )
        .unwrap();
        let mut result = CaseResult::default();
        evaluate_case(&model, &mut Budget::new(4), &data.cases[0], &mut result)
            .await
            .unwrap();
        server.abort();
        let requests = seen.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1]["tools"].as_array().unwrap().len(), 1);
        assert_eq!(requests[1]["tools"][0]["function"]["name"], "ask_caller");
        assert_eq!(
            result.predicted_readiness.as_deref(),
            Some("needs_information")
        );
        assert_eq!(
            result.simulated_local_outcome.as_deref(),
            Some("question_requested")
        );
        assert!(!result.completed_external_action);
    }
    #[test]
    fn general_purpose_development_is_explicit_and_keeps_family_claims_out_of_authorization() {
        assert!(parse_args(&["--evaluate-general-purpose-development".into()]).is_ok());
        let data = dataset_for(
            include_str!("../../../fixtures/language-general-purpose-development.json"),
            "conversation_development",
        )
        .unwrap();
        assert_eq!(data.cases.len(), 8);
        assert_eq!(
            data.cases
                .iter()
                .filter(|case| case.expected.classification == "risky")
                .count(),
            1
        );
        for case in &data.cases {
            let caps = capabilities(case);
            assert!(
                !caps.handoff_available
                    && !caps.family_contact_enrolled
                    && !caps.family_callback_available
            );
            let messages = messages(case, "");
            let context: Value =
                serde_json::from_str(messages[1]["content"].as_str().unwrap()).unwrap();
            assert_eq!(
                context["current_exchange"]["caller_turn"]["text"],
                case.turns.last().unwrap().text
            );
            assert!(!messages[1]["content"].as_str().unwrap().contains(&case.id));
            assert!(
                !messages[1]["content"]
                    .as_str()
                    .unwrap()
                    .contains("accepted_next_tools")
            );
            assert!(
                !messages[1]["content"]
                    .as_str()
                    .unwrap()
                    .contains(&case.rationale)
            );
        }
        let family = data
            .cases
            .iter()
            .find(|case| case.category == "family-claim-then-login")
            .unwrap();
        let session = case_session(family);
        assert_eq!(session.turn_version, 2);
        assert_eq!(session.turns.len(), 3);
        assert!(session.legs.is_empty());
    }
    #[test]
    fn evaluation_retains_actual_numeric_facts_and_long_identifier_redaction() {
        let case = fixture().cases.remove(0);
        let call = assessment(
            observation(SpeechAct::Statement, RequestedObject::RoutineInformation),
            &case.turns[0].text,
        );
        let (base, _) = assess(&case, &call).unwrap();
        for (summary, expected) in [
            (
                "Clinic visit Friday at 3 PM.",
                "Clinic visit Friday at 3 PM.",
            ),
            (
                "Clinic visit Friday at three PM.",
                "Clinic visit Friday at three PM.",
            ),
            (
                "Caller supplied number +1-202-555-0123.",
                "Caller supplied number [redacted]",
            ),
        ] {
            let mut session = base.clone();
            let tool = ToolCall::parse(
                "take_message",
                json!({"summary":summary,"evidence_refs":call.arguments["evidence_refs"]}),
            )
            .unwrap();
            let result = execute_inert(
                &mut session,
                &ModelCapabilities::fictional_enrolled(),
                &tool,
            )
            .unwrap();
            assert_eq!(result["recorded_summary"], expected);
            assert_eq!(session.message.as_ref().unwrap().summary, expected);
        }
    }
    #[test]
    fn conversation_development_preserves_assistant_history_without_assessing_it_as_a_caller_turn()
    {
        let data = dataset_for(
            include_str!("../../../fixtures/language-conversation-development.json"),
            "conversation_development",
        )
        .unwrap();
        assert_eq!(data.cases.len(), 8);
        let case = &data.cases[1];
        let session = case_session(case);
        assert_eq!(session.turn_version, 2);
        assert_eq!(session.turns.len(), 3);
        assert_eq!(session.turns[1].speaker, "assistant");
        let msgs = messages(case, "");
        let context: Value = serde_json::from_str(msgs[1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(
            context["current_exchange"]["caller_turn"]["text"],
            "Friday at 3 PM."
        );
        assert!(
            !msgs[1]["content"]
                .as_str()
                .unwrap()
                .contains("accepted_next_tools")
        );
        assert!(!msgs[1]["content"].as_str().unwrap().contains(&case.id));
        assert!(
            dataset_for(
                include_str!("../../../fixtures/language-conversation-development.json"),
                "held_out"
            )
            .is_err()
        );
    }
    #[tokio::test]
    async fn interactive_does_not_invent_an_offer_or_reopen_a_model_closed_session() {
        use axum::{Json, Router, routing::post};
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let count = Arc::new(AtomicUsize::new(0));
        let captured = count.clone();
        let app = Router::new().route("/chat", post(move |Json(request): Json<Value>| {
            let count = captured.clone();
            async move {
                let index = count.fetch_add(1, Ordering::SeqCst);
                let context: Value = serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                let turn = &context["turns"][0];
                let evidence = json!([{"turn_id":turn["id"],"quote":turn["text"]}]);
                let (name,args) = if index == 0 {
                    ("record_assessment",json!({"observation":{"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"},"evidence_refs":evidence,"understanding":test_understanding()}))
                } else {
                    ("take_message",json!({"summary":"A fictional clinic caller requested Thursday morning.","evidence_refs":evidence}))
                };
                Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":format!("closing-{index}"),"type":"function","function":{"name":name,"arguments":args.to_string()}}]}}]}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chat", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let model = Nemotron::new("local-fixture".into(), url, "fixture-model".into());
        let data = interactive_dataset(include_str!(
            "../../../fixtures/voice-interactive-development.json"
        ))
        .unwrap();
        let result = interactive_scenario(&model, &mut Budget::new(8), &data.scenarios[0])
            .await
            .unwrap();
        server.abort();
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert_eq!(result["turns"][0]["terminal_tool"], "take_message");
        assert!(
            result["turns"][1]["error"]
                .as_str()
                .unwrap()
                .contains("not committed or reopened")
        );
        assert!(result["turns"][1]["classification_correct"].is_null());
        assert_eq!(result["controls"]["pending_offer_created"], false);
        assert_eq!(result["controls"]["revocation_gate_passed"], false);
        assert_eq!(result["final_state"]["turn_version"], 1);
        assert_eq!(result["final_state"]["message_saved"], true);
        assert_eq!(result["final_state"]["inbound_ended"], false);
        assert_eq!(result["external_provider_actions"], 0);
    }
    #[tokio::test]
    async fn interactive_configuration_cannot_load_live_credentials_or_persistent_data() {
        let (app, handle) = interactive_actor().await.unwrap();
        assert_eq!(app.config.mode, ExecutionMode::Replay);
        assert_eq!(app.config.database_url, "sqlite::memory:");
        assert!(
            app.config.nvidia_key.is_empty()
                && app.config.auth_token.is_empty()
                && app.config.account_sid.is_empty()
                && app.config.photon_number.is_empty()
                && app.config.sip_password.is_empty()
                && app.config.contacts.is_empty()
        );
        assert!(
            !app.config.smoke_authorized
                && !app.config.handoff_enabled
                && !app.config.family_enabled
        );
        assert!(
            app.store
                .events_after(0)
                .await
                .unwrap()
                .iter()
                .all(|e| e.execution_mode == ExecutionMode::Replay)
        );
        handle.send(Input::End).await.unwrap();
        let raw = include_str!("../../../fixtures/voice-interactive-development.json");
        assert!(
            interactive_dataset(
                &raw.replace("\"execution\": \"replay\"", "\"execution\": \"live\"")
            )
            .is_err()
        );
        assert_eq!(
            parse_args(&["--evaluate-interactive-development".into()]).unwrap(),
            Mode::EvaluateInteractiveDevelopment
        );
    }
    #[tokio::test]
    async fn interactive_actor_preserves_offer_then_revokes_without_future_input_leakage() {
        use axum::{Json, Router, routing::post};
        use std::sync::{Arc, Mutex};
        let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
        let captured = seen.clone();
        let app = Router::new().route("/chat", post(move |Json(request): Json<Value>| {
            let captured = captured.clone();
            async move {
                let mut requests = captured.lock().unwrap();
                let index = requests.len();
                requests.push(request.clone());
                let context: Value = serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                let turn = context["turns"].as_array().unwrap().iter().rev().find(|t| t["speaker"] == "caller").unwrap();
                let evidence = json!([{"turn_id":turn["id"],"quote":turn["text"]}]);
                let obs = if index < 2 {
                    json!({"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"})
                } else {
                    json!({"speech_act":"request","requested_object":"authentication_secret","direction":"caller_requests_from_sarah"})
                };
                let (name, args) = match index {
                    0 | 2 => ("record_assessment", json!({"observation":obs,"evidence_refs":evidence,"understanding":test_understanding()})),
                    1 => ("offer_handoff", json!({"summary":"A fictional clinic caller requests an appointment change.","observation":obs,"evidence_refs":evidence})),
                    _ => ("start_trusted_callback", json!({"contact_id":"sarah:family","reason":"authentication_request","evidence_refs":evidence})),
                };
                Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":format!("interactive-native-{index}"),"type":"function","function":{"name":name,"arguments":args.to_string()}}]}}]}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chat", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let model = Nemotron::new("local-fixture".into(), url, "fixture-model".into());
        let data: InteractiveDataset = serde_json::from_str(include_str!(
            "../../../fixtures/voice-interactive-development.json"
        ))
        .unwrap();
        let mut budget = Budget::new(8);
        let result = interactive_scenario(&model, &mut budget, &data.scenarios[0])
            .await
            .unwrap();
        server.abort();
        assert_eq!(result["turns"][0]["terminal_tool"], "offer_handoff");
        assert_eq!(
            result["turns"][1]["terminal_tool"],
            "start_trusted_callback"
        );
        assert_eq!(result["controls"]["revocation_gate_passed"], true);
        assert_eq!(result["controls"]["partial_choice_rejected"], true);
        assert_eq!(result["controls"]["stale_choice_rejected"], true);
        assert_eq!(result["execution_mode"], "replay");
        assert_eq!(result["external_provider_actions"], 0);
        assert_eq!(result["final_state"]["blocked"], true);
        assert_eq!(result["final_state"]["bridge_committed"], false);
        assert_eq!(result["final_state"]["legs"][1]["acknowledged"], false);
        let requests = seen.lock().unwrap();
        let first = requests[0]["messages"].to_string();
        assert!(!first.contains(&data.scenarios[0].id));
        assert!(!first.contains(&data.scenarios[0].steps[1].caller_text));
        assert!(!first.contains("current_turn_classification"));
        assert!(!first.contains("proposed_observation"));
        assert_eq!(
            requests[1]["messages"].as_array().unwrap().last().unwrap()["tool_call_id"],
            "interactive-native-0"
        );
        let second_context: Value =
            serde_json::from_str(requests[2]["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(
            second_context["turns"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|t| t["speaker"] == "caller")
                .count(),
            2
        );
    }
    fn fixture() -> Dataset {
        serde_json::from_str(include_str!("../../../fixtures/language-held-out.json")).unwrap()
    }
    fn assessment(observation: Observation, quote: &str) -> ModelToolCall {
        ModelToolCall {
            id: "native-id".into(),
            name: "record_assessment".into(),
            arguments: json!({"observation":observation,"evidence_refs":[{"turn_id":"turn-1","quote":quote}],"understanding":test_understanding()}),
            assistant_message: json!({"role":"assistant"}),
        }
    }
    fn observation(act: SpeechAct, object: RequestedObject) -> Observation {
        Observation {
            speech_act: act,
            requested_object: object,
            direction: Direction::CallerRequestsFromSarah,
        }
    }
    #[test]
    fn inert_lookup_preserves_assessment_hold_and_callback_never_completes_a_phone() {
        let case = fixture().cases.remove(3);
        let mut s = case_session(&case);
        let caps = ModelCapabilities::fictional_enrolled();
        let contact = execute_inert(
            &mut s,
            &caps,
            &ToolCall::LookupTrustedContact {
                role: firstring::tools::ContactRole::Family,
            },
        )
        .unwrap();
        assert_eq!(s.assessed_version, 0);
        assert_eq!(contact["contact_id"], "sarah:family");
        let tool = assessment(
            observation(SpeechAct::Request, RequestedObject::AuthenticationSecret),
            &case.turns[0].text,
        );
        let (mut s, _) = assess(&case, &tool).unwrap();
        let call=ToolCall::parse("start_trusted_callback",json!({"contact_id":contact["contact_id"],"reason":"authentication_request","evidence_refs":tool.arguments["evidence_refs"]})).unwrap();
        let result = execute_inert(&mut s, &caps, &call).unwrap();
        assert_eq!(result["state"], "reserved");
        assert_eq!(result["acknowledged"], false);
        assert!(!s.inbound_ended);
    }
    #[test]
    fn inert_executor_reports_missing_contacts_and_saves_a_benign_message() {
        let case = fixture().cases.remove(0);
        let caps = ModelCapabilities {
            handoff_available: false,
            family_contact_enrolled: false,
            family_callback_available: false,
            clinic_contact_enrolled: false,
            message_taking_available: true,
        };
        let tool = assessment(
            observation(SpeechAct::Statement, RequestedObject::RoutineInformation),
            &case.turns[0].text,
        );
        let (mut s, _) = assess(&case, &tool).unwrap();
        assert!(
            execute_inert(
                &mut s,
                &caps,
                &ToolCall::LookupTrustedContact {
                    role: firstring::tools::ContactRole::Family
                }
            )
            .is_err()
        );
        let message=ToolCall::parse("take_message",json!({"summary":"A caller reported a changed clinic visit.","evidence_refs":tool.arguments["evidence_refs"]})).unwrap();
        let result = execute_inert(&mut s, &caps, &message).unwrap();
        assert_eq!(result["message_recorded"], true);
        assert!(!s.inbound_ended);
        assert!(s.legs.is_empty());
    }
    #[test]
    fn only_one_explicit_known_mode_can_enable_model_requests() {
        assert_eq!(parse_args(&[]).unwrap(), Mode::Status);
        assert_eq!(
            parse_args(&["--model-probe".into()]).unwrap(),
            Mode::ModelProbe
        );
        assert!(parse_args(&["--evaluate-held-out".into(), "--model-probe".into()]).is_err());
        assert!(parse_args(&["--call".into()]).is_err());
    }
    #[test]
    fn diagnostic_timeout_override_is_confined_to_model_probe() {
        for mode in [
            Mode::Evaluate,
            Mode::EvaluateRegression,
            Mode::EvaluateDevelopment,
            Mode::EvaluateInteractiveDevelopment,
            Mode::Status,
            Mode::Help,
        ] {
            for value in [None, Some("15000"), Some("invalid"), Some("0")] {
                assert_eq!(request_timeout_ms(mode, value).unwrap(), 3500);
            }
        }
        assert_eq!(request_timeout_ms(Mode::ModelProbe, None).unwrap(), 3500);
        assert_eq!(request_timeout_ms(Mode::ModelProbe, Some("1")).unwrap(), 1);
        assert_eq!(
            request_timeout_ms(Mode::ModelProbe, Some("15000")).unwrap(),
            15000
        );
        for value in ["0", "15001", "-1", "invalid", ""] {
            assert!(request_timeout_ms(Mode::ModelProbe, Some(value)).is_err());
        }
    }
    #[test]
    fn failed_context_question_is_retained_without_passing_the_gate() {
        let question = "Could you clarify that?";
        let mut questions = Vec::new();
        assert!(retain_probe_question(0, question.into(), &mut questions).is_err());
        assert_eq!(questions, [question]);
        let valid = "Which entrance should the grocery delivery use?";
        assert!(retain_probe_question(1, valid.into(), &mut questions).is_ok());
        assert_eq!(questions, [question, valid]);
    }
    #[test]
    fn frozen_held_out_schema_is_required_and_expected_answers_are_not_sent() {
        let raw = include_str!("../../../fixtures/language-held-out.json");
        let data = dataset(raw).unwrap();
        assert_eq!(data.cases.len(), 12);
        assert!(dataset(&raw.replace("held_out", "development")).is_err());
        let prompt =
            serde_json::to_string(&messages(&data.cases[0], "Assess the latest turn.")).unwrap();
        assert!(prompt.contains(&data.cases[0].turns[0].text));
        assert!(!prompt.contains("accepted_next_tools"));
        assert!(!prompt.contains(&data.cases[0].rationale));
    }
    #[test]
    fn assessment_must_be_first_and_quote_actual_committed_caller_text() {
        let case = fixture().cases.remove(0);
        let obs = observation(SpeechAct::Statement, RequestedObject::RoutineInformation);
        assert!(assess(&case, &assessment(obs.clone(), &case.turns[0].text)).is_ok());
        assert!(assess(&case, &assessment(obs, "invented evidence")).is_err());
        let lookup = ModelToolCall {
            id: "lookup".into(),
            name: "lookup_trusted_contact".into(),
            arguments: json!({"role":"family"}),
            assistant_message: json!({}),
        };
        assert!(assess(&case, &lookup).is_err());
    }
    #[test]
    fn rubric_uses_direction_speech_act_and_object_without_keyword_self_grading() {
        assert_eq!(
            flags(&observation(
                SpeechAct::Negation,
                RequestedObject::AuthenticationSecret
            )),
            ("benign".into(), false)
        );
        assert_eq!(
            flags(&observation(
                SpeechAct::Request,
                RequestedObject::PaymentDiversion
            )),
            ("risky".into(), false)
        );
        assert_eq!(
            flags(&observation(
                SpeechAct::Request,
                RequestedObject::AuthenticationSecret
            )),
            ("risky".into(), true)
        );
        assert_eq!(
            flags(&observation(SpeechAct::Request, RequestedObject::Unknown)),
            ("clarify".into(), false)
        );
    }
    #[test]
    fn missing_results_are_not_counted_as_correct_or_fabricated_zero_metrics() {
        let report = totals(&[CaseResult {
            id: "failed".into(),
            error: Some("request timed out".into()),
            ..CaseResult::default()
        }]);
        assert_eq!(report["errors"], 1);
        assert_eq!(report["assessed_cases"], 0);
        assert!(report["authentication_request_recall"].is_null());
        assert!(report["benign_false_positive_rate"].is_null());
    }
    #[test]
    fn contextual_probe_rejects_generic_or_wrong_context_questions() {
        assert!(contextual_followup(0, "What is the new appointment time?"));
        assert!(contextual_followup(
            1,
            "Which entrance should the groceries be left at?"
        ));
        assert!(!contextual_followup(0, "Can you tell me more?"));
        assert!(!contextual_followup(1, "What is the appointment time?"));
    }

    #[test]
    fn authentication_and_required_clarification_mismatches_fail_the_rubric() {
        let mut result = CaseResult {
            classification_correct: Some(true),
            expected_authentication_request: Some(true),
            authentication_request: Some(false),
            next_tool_allowed: Some(true),
            evidence_valid: true,
            ..CaseResult::default()
        };
        assert!(!case_passes(&result));
        result.authentication_request = Some(true);
        result.clarification_required = true;
        result.clarification = Some(false);
        assert!(!case_passes(&result));
        result.clarification = Some(true);
        assert!(case_passes(&result));
    }
    #[tokio::test]
    async fn evaluator_returns_lookup_result_then_reaches_inert_callback() {
        use axum::{Json, Router, routing::post};
        use std::sync::{Arc, Mutex};
        let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
        let captured = seen.clone();
        let app=Router::new().route("/chat",post(move |Json(request):Json<Value>| {
            let captured=captured.clone();
            async move {
                let mut requests=captured.lock().unwrap();
                let index=requests.len();
                requests.push(request.clone());
                let context:Value=serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
                let turn=&context["turns"][0];
                let evidence=json!([{"turn_id":turn["id"],"quote":turn["text"]}]);
                let (name,args)=match index {
                    0=>("record_assessment",json!({"observation":{"speech_act":"request","requested_object":"authentication_secret","direction":"caller_requests_from_sarah"},"evidence_refs":evidence,"understanding":test_understanding()})),
                    1=>("lookup_trusted_contact",json!({"role":"family"})),
                    _=>("start_trusted_callback",json!({"contact_id":"sarah:family","reason":"authentication_request","evidence_refs":evidence})),
                };
                Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":format!("native-{index}"),"type":"function","function":{"name":name,"arguments":args.to_string()}}]}}]}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chat", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let model = Nemotron::new("local-fixture".into(), url, "fixture-model".into());
        let mut budget = Budget::new(4);
        let mut result = CaseResult::default();
        evaluate_case(&model, &mut budget, &fixture().cases[3], &mut result)
            .await
            .unwrap();
        server.abort();
        assert_eq!(result.trace.len(), 3);
        assert_eq!(
            result.simulated_local_outcome.as_deref(),
            Some("callback_reserved")
        );
        assert!(!result.completed_external_action);
        let requests = seen.lock().unwrap();
        let messages = requests[2]["messages"].as_array().unwrap();
        assert_eq!(messages.last().unwrap()["tool_call_id"], "native-1");
        let returned: Value =
            serde_json::from_str(messages.last().unwrap()["content"].as_str().unwrap()).unwrap();
        assert_eq!(returned["result"]["contact_id"], "sarah:family");
        assert!(
            !requests[0]["messages"]
                .to_string()
                .contains("Do not execute any action")
        );
        assert!(
            !requests[0]["messages"]
                .to_string()
                .contains("probe_instruction")
        );
    }
    #[tokio::test]
    async fn rejected_initial_evidence_retains_proposal_and_failure_without_a_second_request() {
        use axum::{Json, Router, routing::post};
        let app=Router::new().route("/chat",post(|| async {
            Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":"bad-evidence","type":"function","function":{"name":"record_assessment","arguments":json!({"observation":{"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"},"evidence_refs":[{"turn_id":"turn-1","quote":"This quote was invented."}],"understanding":test_understanding()}).to_string()}}]}}]}))
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chat", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let model = Nemotron::new("local-fixture".into(), url, "fixture-model".into());
        let mut budget = Budget::new(4);
        let mut result = CaseResult::default();
        assert!(
            evaluate_case(&model, &mut budget, &fixture().cases[0], &mut result)
                .await
                .is_err()
        );
        server.abort();
        assert_eq!(budget.submitted, 1);
        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0]["tool_call_id"], "bad-evidence");
        assert_eq!(result.trace[0]["result"]["ok"], false);
        assert!(!result.evidence_valid);
    }
}
