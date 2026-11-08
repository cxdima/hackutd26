use anyhow::{Result, anyhow, bail, ensure};
use firstring::{
    calls::state::{Leg, Session, Turn},
    config::ExecutionMode,
    providers::nemotron::{
        ModelCapabilities, ModelToolCall, Nemotron, RequestObservation, ToolPhase,
        append_tool_result, conversation_messages,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Status,
    ModelProbe,
    Evaluate,
    EvaluateRegression,
    EvaluateDevelopment,
    EvaluateAOnly,
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
    session.turn_version = case.turns.len() as u64;
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
            Ok(json!({"message_recorded":true,"closing":true,"stop":true}))
        }
        _ => bail!("Tool is not available in the next-step phase"),
    }
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
        "next_tool_allowed_cases":results.iter().filter(|r|r.next_tool_allowed==Some(true)).count(),"classification_scored_cases":results.iter().filter(|r|r.classification_correct.is_some()).count(),"classification_correct_cases":results.iter().filter(|r|r.classification_correct==Some(true)).count(),"composite_case_passes":results.iter().filter(|r|case_passes(r)).count(),"useful_action_cases":results.iter().filter(|r|r.useful_action==Some(true)).count(),"simulated_local_outcomes":results.iter().filter(|r|r.simulated_local_outcome.is_some()).count(),"completed_external_actions":results.iter().filter(|r|r.completed_external_action).count()})
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
    let override_value = std::env::var("NEMOTRON_PROBE_TIMEOUT_MS").ok();
    let milliseconds = request_timeout_ms(mode, override_value.as_deref())?;
    let model =
        Nemotron::new(key, url, model).with_request_timeout(Duration::from_millis(milliseconds))?;
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
        &json!({"ok":true,"result":{"blocked":session.blocked,"ambiguous":session.ambiguous,"assessed_version":session.assessed_version},"epoch":session.policy_epoch}),
    );
    budget.request(model, messages, ToolPhase::NextStep).await
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
    let (model, request_timeout_ms) = model(Mode::ModelProbe)?;
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
        let first = budget.request(&model, &messages, ToolPhase::Assessment).await?;
        let (session, observation) = assess(&case, &first)?;
        ensure!(
            flags(&observation).0 == "benign",
            "Probe benign detail was not assessed as benign"
        );
        let next = followup(&model, &mut budget, &mut messages, first, &session).await?;
        let ToolCall::AskCaller { question, .. } = validate_next(&session, &next)? else {
            bail!("Probe follow-up was not ask_caller");
        };
        retain_probe_question(index, question, &mut questions)?;
    }
    ensure!(
        questions[0].to_lowercase() != questions[1].to_lowercase(),
        "Probe produced identical follow-ups for different details"
    );
    Ok::<(), anyhow::Error>(())
    }.await;
    let report = json!({"status":if outcome.is_ok(){"model_probe_passed"}else{"model_probe_failed"},
        "recorded_at":chrono::Utc::now().to_rfc3339(),"requested_model":MODEL,"data_origin":"synthetic",
        "request_timeout_ms":request_timeout_ms,"native_tool_choice":"named_assessment_then_required_next_step",
        "requests_submitted":budget.submitted,"request_observations":budget.observations,
        "native_result_roundtrips":budget.observations.iter().enumerate().filter(|(i,o)|i%2==1 && o.outcome=="success").count(),
        "observed_followups":questions,"error":outcome.as_ref().err().map(ToString::to_string),
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
    let outcome = json!({"ok":true,"result":{"blocked":session.blocked,"ambiguous":session.ambiguous,"assessed_version":session.assessed_version},"epoch":session.policy_epoch});
    result.trace.push(json!({"step":1,"tool_call_id":first.id,"name":first.name,"arguments":first.arguments,"result":outcome}));
    append_tool_result(&mut messages, &first, &outcome);
    for step in 2..=4 {
        let next = budget
            .request(model, &messages, ToolPhase::NextStep)
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
                "Is that an appointment reference, or a code used to sign in? Please don't read the code itself.".to_string()
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
            result.useful_action = result.next_tool_allowed;
            result.simulated_local_outcome = Some(
                match tool {
                    ToolCall::AskCaller { .. } => "question_requested",
                    ToolCall::TakeMessage { .. } => "message_recorded",
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

async fn evaluate(data: &Dataset, output: &str) -> Result<()> {
    let (model, request_timeout_ms) = model(Mode::Evaluate)?;
    let mut budget = Budget::new(MAX_CASES * 4);
    let mut results = Vec::new();
    for case in &data.cases {
        let mut result = CaseResult {
            id: case.id.clone(),
            category: case.category.clone(),
            expected_classification: case.expected.classification.clone(),
            expected_authentication_request: case.expected.authentication_request,
            clarification_required: case.expected.clarification == "required",
            capabilities: Some(capabilities(case)),
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
        println!(
            "{}",
            json!({"case":result.id,"predicted":result.predicted_classification,"next_tool":result.next_tool,"local_outcome":result.simulated_local_outcome,"error":result.error})
        );
        results.push(result);
    }
    let metrics = totals(&results);
    let report = json!({"status":"completed_with_observations","recorded_at":chrono::Utc::now().to_rfc3339(),"model":MODEL,"data_origin":"synthetic","request_timeout_ms":request_timeout_ms,"turn_deadline_ms":6000,"max_steps_per_case":4,"native_tool_choice":"named_assessment_then_required_next_step","split":data.split,"case_count":data.cases.len(),"requests_submitted":budget.submitted,"request_observations":budget.observations,"metrics":metrics,"cases":results,"phone_tested":false,"external_actions_executed":0,"executor":"in-memory policy-validated simulation; native tool results are returned to the same bounded model loop","interpretation":"Classification, allowed useful proposal, and simulated local outcome are separate. No provider calls, delivery, or acknowledgement are inferred. Synthetic typed arguments/replies are retained; no raw model reasoning. Null rates have no scored denominator."});
    save_report_to(output, &report)?;
    println!(
        "{}",
        json!({"report":output,"metrics":metrics,"requests_submitted":budget.submitted})
    );
    ensure!(
        results.iter().all(case_passes),
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
                "FirstRing preflight\n  (no flags)           Report environment NAMES only; no external calls.\n  --model-probe        Opt in to up to 4 NVIDIA requests; native tool/result roundtrip for two synthetic contexts.\n  --evaluate-held-out  Up to 48 NVIDIA requests on frozen v2; .tools/evaluation-v2-results.json.\n  --evaluate-regression  Original 12 cases; .tools/regression-results.json.\n  --evaluate-development  One case per development category; .tools/development-results.json.\n  --evaluate-a-only    Three development cases with no enrolled/calling contacts; .tools/a-only-development-results.json.\n  --help              Show usage.\nModel requests require NVIDIA_API_KEY and the documented endpoint/model; named record_assessment first, then required next-step tool selection, with 3.5-second request, 6-second turn, four-step turn and 120-second run limits. NEMOTRON_PROBE_TIMEOUT_MS=1..15000 overrides only --model-probe; held-out evaluation and runtime retain their normal deadlines. Reports record request_timeout_ms and native_tool_choice=named_assessment_then_required_next_step. Evaluation uses an in-memory policy executor; no command dials phones or changes application data. Run from repository root."
            );
            Ok(())
        }
        Mode::ModelProbe
        | Mode::Evaluate
        | Mode::EvaluateRegression
        | Mode::EvaluateDevelopment
        | Mode::EvaluateAOnly => {
            let (path, split, output) = match mode {
                Mode::Evaluate => (
                    "fixtures/language-held-out-v2.json",
                    "held_out_v2",
                    ".tools/evaluation-v2-results.json",
                ),
                Mode::EvaluateRegression => {
                    (FIXTURES, "held_out", ".tools/regression-results.json")
                }
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
                evaluate(&data, output).await
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
                    0=>("record_assessment",json!({"observation":{"speech_act":"request","requested_object":"authentication_secret","direction":"caller_requests_from_sarah"},"evidence_refs":evidence})),
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
            Json(json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":"bad-evidence","type":"function","function":{"name":"record_assessment","arguments":json!({"observation":{"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"},"evidence_refs":[{"turn_id":"turn-1","quote":"This quote was invented."}]}).to_string()}}]}}]}))
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
