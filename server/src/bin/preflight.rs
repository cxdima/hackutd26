use anyhow::{Result, anyhow, bail, ensure};
use firstring::{
    calls::state::{Session, Turn},
    config::ExecutionMode,
    providers::nemotron::{ModelToolCall, Nemotron, RequestObservation, system_prompt},
    tools::{Direction, Observation, RequestedObject, SpeechAct, ToolCall},
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
const OUTPUT: &str = ".tools/evaluation-results.json";
const MAX_CASES: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Status,
    ModelProbe,
    Evaluate,
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
    rationale: String,
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
}

fn parse_args(args: &[String]) -> Result<Mode> {
    match args {
        [] => Ok(Mode::Status),
        [flag] => match flag.as_str() {
            "--model-probe" => Ok(Mode::ModelProbe),
            "--evaluate-held-out" => Ok(Mode::Evaluate),
            "--help" | "-h" => Ok(Mode::Help),
            _ => bail!("Unknown flag. Use --help."),
        },
        _ => bail!("Choose exactly one mode. Use --help."),
    }
}

fn dataset(raw: &str) -> Result<Dataset> {
    let data: Dataset =
        serde_json::from_str(raw).map_err(|_| anyhow!("Fixture schema is invalid"))?;
    ensure!(
        data.schema_version == 1
            && data.data_origin == "synthetic"
            && data.split == "held_out"
            && data.authored_before_model_evaluation,
        "Only frozen synthetic held-out schema version 1 is accepted"
    );
    ensure!(
        !data.cases.is_empty() && data.cases.len() <= MAX_CASES,
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
                    && t.speaker == "caller"
                    && !t.text.trim().is_empty()
                    && t.text.len() <= 4000),
            "Fixture caller turns are invalid"
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

fn messages(case: &Case, instruction: &str) -> Vec<Value> {
    vec![
        json!({"role":"system","content":system_prompt()}),
        json!({"role":"user","content":json!({"household":case.household_id,"turns":case.turns,"preconditions":case.preconditions,"instruction":instruction}).to_string()}),
    ]
}

fn assess(case: &Case, call: &ModelToolCall) -> Result<(Session, Observation)> {
    let ToolCall::RecordAssessment {
        observation,
        evidence_refs,
    } = ToolCall::parse(&call.name, call.arguments.clone())?
    else {
        bail!("First native tool was not record_assessment");
    };
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
    session.turn_version = case.turns.len() as u64;
    session.generation_id = 1;
    session.assess(1, session.turn_version, &observation, &evidence_refs)?;
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

fn ratio(n: usize, d: usize) -> Option<f64> {
    (d > 0).then(|| n as f64 / d as f64)
}

fn case_passes(result: &CaseResult) -> bool {
    result.error.is_none()
        && result.evidence_valid
        && result.classification_correct != Some(false)
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
        "evidence_valid_cases":results.iter().filter(|r|r.evidence_valid).count(),"evidence_valid_rate":ratio(results.iter().filter(|r|r.evidence_valid).count(),results.len()),
        "next_tool_allowed_cases":results.iter().filter(|r|r.next_tool_allowed==Some(true)).count(),"classification_scored_cases":results.iter().filter(|r|r.classification_correct.is_some()).count()})
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
    async fn request(&mut self, model: &Nemotron, messages: &[Value]) -> Result<ModelToolCall> {
        ensure!(
            self.remaining > 0 && Instant::now() < self.deadline,
            "Evaluation request/time budget exhausted"
        );
        self.remaining -= 1;
        self.submitted += 1;
        let observed = tokio::time::timeout_at(
            tokio::time::Instant::from_std(self.deadline),
            model.next_tool_observed(messages),
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

fn model() -> Result<Nemotron> {
    let key = std::env::var("NVIDIA_API_KEY").unwrap_or_default();
    ensure!(
        !key.trim().is_empty(),
        "Pending: NVIDIA_API_KEY is missing; no model requests submitted"
    );
    let url = std::env::var("NEMOTRON_CHAT_URL").unwrap_or_else(|_| ENDPOINT.into());
    let model = std::env::var("NEMOTRON_MODEL").unwrap_or_else(|_| MODEL.into());
    ensure!(
        url == ENDPOINT && model == MODEL,
        "This preflight requires the documented NEMOTRON_CHAT_URL and NEMOTRON_MODEL; configuration was not changed"
    );
    Ok(Nemotron::new(key, url, model))
}

async fn followup(
    model: &Nemotron,
    budget: &mut Budget,
    messages: &mut Vec<Value>,
    first: ModelToolCall,
    session: &Session,
) -> Result<ModelToolCall> {
    messages.push(first.assistant_message);
    messages.push(json!({"role":"tool","tool_call_id":first.id,"content":json!({"ok":true,"result":{"blocked":session.blocked,"ambiguous":session.ambiguous,"assessed_version":session.assessed_version},"execution":"evaluation_no_actions"}).to_string()}));
    budget.request(model, messages).await
}

fn validate_next(session: &Session, call: &ModelToolCall) -> Result<ToolCall> {
    let tool = ToolCall::parse(&call.name, call.arguments.clone())?;
    match &tool {
        ToolCall::AskCaller { evidence_refs, .. }
        | ToolCall::OfferHandoff { evidence_refs, .. }
        | ToolCall::StartTrustedCallback { evidence_refs, .. }
        | ToolCall::TakeMessage { evidence_refs, .. } => {
            session.evidence_valid(evidence_refs, true)?
        }
        ToolCall::LookupTactics { .. } | ToolCall::LookupTrustedContact { .. } => {}
        _ => bail!("Follow-up must be a next-step tool, not another assessment or publication"),
    }
    Ok(tool)
}

async fn probe(data: &Dataset) -> Result<()> {
    let model = model()?;
    let mut budget = Budget::new(4);
    let mut questions = Vec::new();
    let outcome = async {
    for (index, text) in [
        "The clinic appointment time has changed, and I need to give her the new time.",
        "I have a grocery delivery and need to know which entrance she wants the bags left at.",
    ]
    .iter()
    .enumerate()
    {
        let mut case = data.cases[0].clone();
        case.id = format!("synthetic-access-probe-{index}");
        case.turns[0].text = (*text).into();
        let mut messages = messages(
            &case,
            "Assess the latest caller turn first. For this access probe, then ask one short context-specific clarifying question using ask_caller.",
        );
        let first = budget.request(&model, &messages).await?;
        let (session, observation) = assess(&case, &first)?;
        ensure!(
            flags(&observation).0 == "benign",
            "Probe benign detail was not assessed as benign"
        );
        let next = followup(&model, &mut budget, &mut messages, first, &session).await?;
        let ToolCall::AskCaller { question, .. } = validate_next(&session, &next)? else {
            bail!("Probe follow-up was not ask_caller");
        };
        ensure!(
            contextual_followup(index, &question),
            "Probe question failed the prewritten context-anchor check; human relevance review still required"
        );
        questions.push(question);
    }
    ensure!(
        questions[0].to_lowercase() != questions[1].to_lowercase(),
        "Probe produced identical follow-ups for different details"
    );
    Ok::<(), anyhow::Error>(())
    }.await;
    let report = json!({"status":if outcome.is_ok(){"model_probe_passed"}else{"model_probe_failed"},
        "recorded_at":chrono::Utc::now().to_rfc3339(),"requested_model":MODEL,"data_origin":"synthetic",
        "requests_submitted":budget.submitted,"request_observations":budget.observations,
        "native_result_roundtrips":budget.observations.iter().enumerate().filter(|(i,o)|i%2==1 && o.outcome=="success").count(),
        "distinct_contextual_followups":questions,"error":outcome.as_ref().err().map(ToString::to_string),
        "relevance_check":"prewritten lexical anchors; human review still required","phone_tested":false,
        "latency_boundary":"HTTP request through complete response and tool parsing; not audible latency"});
    save_report_to(".tools/model-probe-results.json", &report)?;
    println!("{report}");
    outcome
}

async fn evaluate_case(
    model: &Nemotron,
    budget: &mut Budget,
    case: &Case,
    result: &mut CaseResult,
) -> Result<()> {
    let mut messages = messages(
        case,
        "Assess the latest caller turn, then choose a useful next tool. Do not execute any action.",
    );
    let first = budget.request(model, &messages).await?;
    let (session, observation) = assess(case, &first)?;
    result.evidence_valid = true;
    let (classification, authentication) = flags(&observation);
    result.observation = Some(observation);
    result.authentication_request = Some(authentication);
    if case.expected.classification != "reject_instruction" {
        result.classification_correct = Some(classification == case.expected.classification);
    }
    result.predicted_classification = Some(classification);
    let next = followup(model, budget, &mut messages, first, &session).await?;
    let tool = validate_next(&session, &next)?;
    result.clarification = Some(matches!(tool, ToolCall::AskCaller { .. }));
    result.next_tool_allowed = Some(case.expected.accepted_next_tools.contains(&next.name));
    result.next_tool = Some(next.name);
    Ok(())
}

fn save_report(value: &Value) -> Result<()> {
    save_report_to(OUTPUT, value)
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

async fn evaluate(data: &Dataset) -> Result<()> {
    let model = match model() {
        Ok(m) => m,
        Err(e) => {
            save_report(
                &json!({"status":"pending","reason":e.to_string(),"data_origin":"synthetic","case_count":data.cases.len(),"requests_submitted":0,"metrics":null,"cases":[],"phone_tested":false}),
            )?;
            return Err(e);
        }
    };
    let mut budget = Budget::new(MAX_CASES * 2);
    let mut results = Vec::new();
    for case in &data.cases {
        let mut result=CaseResult {id:case.id.clone(),category:case.category.clone(),expected_classification:case.expected.classification.clone(),expected_authentication_request:case.expected.authentication_request,clarification_required:case.expected.clarification=="required",unscored:vec!["Live consent, stale acceptance and phone side effects require protocol/phone tests".into()],..CaseResult::default()};
        if case.expected.classification == "reject_instruction" {
            result.unscored.push("Observation has no instruction-rejection class; compare next-tool allowlist separately".into());
        }
        if case.expected.tactic_match.is_some() {
            result.unscored.push("Tactic receipt/match requires an applied capsule and is outside this model-only probe".into());
        }
        if let Err(e) = evaluate_case(&model, &mut budget, case, &mut result).await {
            result.error = Some(e.to_string());
        }
        println!(
            "{}",
            json!({"case":result.id,"predicted":result.predicted_classification,"next_tool":result.next_tool,"error":result.error})
        );
        results.push(result);
    }
    let metrics = totals(&results);
    let report = json!({"status":"completed_with_observations","recorded_at":chrono::Utc::now().to_rfc3339(),"model":MODEL,"data_origin":"synthetic","split":"held_out","case_count":data.cases.len(),"requests_submitted":budget.submitted,"request_observations":budget.observations,"metrics":metrics,"cases":results,"phone_tested":false,"tool_actions_executed":0,"interpretation":"Prewritten rubric compared with observed native tool fields; fixtures and model self-report are not phone verification. Null rates have no scored denominator."});
    save_report(&report)?;
    println!(
        "{}",
        json!({"report":OUTPUT,"metrics":metrics,"requests_submitted":budget.submitted})
    );
    ensure!(
        results.iter().all(case_passes),
        "Held-out run contains errors or rubric mismatches; inspect evaluation-results.json"
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
                "FirstRing preflight\n  (no flags)           Report environment NAMES only; no external calls.\n  --model-probe        Opt in to up to 4 NVIDIA requests; native tool/result roundtrip for two synthetic contexts.\n  --evaluate-held-out  Opt in to up to 24 NVIDIA requests across <=12 frozen cases; save ignored .tools/evaluation-results.json.\n  --help              Show usage.\nModel requests require NVIDIA_API_KEY and the documented endpoint/model; 3.5-second request and 120-second run limits. No command dials phones or executes tools. Run from repository root."
            );
            Ok(())
        }
        Mode::ModelProbe | Mode::Evaluate => {
            let raw = std::fs::read_to_string(FIXTURES).map_err(|_| {
                anyhow!("Cannot read fixtures/language-held-out.json; run from repository root")
            })?;
            let data = dataset(&raw)?;
            if mode == Mode::ModelProbe {
                probe(&data).await
            } else {
                evaluate(&data).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Dataset {
        serde_json::from_str(include_str!("../../../fixtures/language-held-out.json")).unwrap()
    }
    fn assessment(observation: Observation, quote: &str) -> ModelToolCall {
        ModelToolCall {
            id: "native-id".into(),
            name: "record_assessment".into(),
            arguments: json!({"observation":observation,"evidence_refs":[{"turn_id":"turn-1","quote":quote}]}),
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
}
