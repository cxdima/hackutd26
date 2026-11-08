use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_SUPPLIED_FACTS: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub turn_id: String,
    pub quote: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub speech_act: SpeechAct,
    pub requested_object: RequestedObject,
    pub direction: Direction,
}

/// Open-ended interpretation; caller identity and facts remain claims, not verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallUnderstanding {
    pub version: u8,
    pub claimed_identity: Option<String>,
    pub purpose: String,
    pub requested_action: String,
    pub supplied_facts: Vec<GroundedFact>,
    pub missing_information: Vec<String>,
    pub readiness: Readiness,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedFact {
    pub text: String,
    pub evidence_refs: Vec<EvidenceRef>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    NeedsInformation,
    Ready,
    CallerDeclined,
    NoMessage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    WrongNumber,
    CallerFinished,
    NoMessage,
}

impl CallUnderstanding {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "Unsupported understanding version");
        if let Some(identity) = &self.claimed_identity {
            validate_speech(identity, 120)?;
        }
        validate_speech(&self.purpose, 180)?;
        validate_speech(&self.requested_action, 180)?;
        ensure!(
            self.supplied_facts.len() <= MAX_SUPPLIED_FACTS && self.missing_information.len() <= 3,
            "Understanding exceeds bounds"
        );
        for fact in &self.supplied_facts {
            validate_speech(&fact.text, 180)?;
            validate_evidence(&fact.evidence_refs)?;
            ensure!(
                fact.evidence_refs
                    .iter()
                    .any(|r| r.quote.contains(&fact.text)),
                "Facts must retain literal caller wording"
            );
        }
        for missing in &self.missing_information {
            validate_speech(missing, 160)?;
        }
        ensure!(
            !matches!(self.readiness, Readiness::Ready | Readiness::NoMessage)
                || self.missing_information.is_empty(),
            "Ready or no-message understanding cannot have missing information"
        );
        ensure!(
            self.readiness != Readiness::NeedsInformation || !self.missing_information.is_empty(),
            "Clarification must identify missing information"
        );
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechAct {
    Request,
    Statement,
    Negation,
    Quotation,
    Unclear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedObject {
    AuthenticationSecret,
    AppointmentReference,
    Payment,
    PaymentDiversion,
    RemoteAccess,
    RoutineInformation,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    CallerRequestsFromSarah,
    CallerSupplies,
    None,
    Unclear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionPurpose {
    ClarifyRequest,
    ClarifyCodeType,
    ClarifyIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactRole {
    Family,
    Clinic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallbackReason {
    AuthenticationRequest,
    IdentityCheck,
    FamilyHelp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TacticKind {
    AuthenticationCodeExtraction,
    RemoteAccessRequest,
    PaymentDiversion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureCategory {
    Cancellation,
    Urgency,
    Threat,
    None,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "name",
    content = "arguments",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ToolCall {
    RecordAssessment {
        observation: Observation,
        evidence_refs: Vec<EvidenceRef>,
        #[serde(default)]
        understanding: Option<CallUnderstanding>,
    },
    AskCaller {
        question: String,
        purpose: QuestionPurpose,
        evidence_refs: Vec<EvidenceRef>,
    },
    LookupTrustedContact {
        role: ContactRole,
    },
    LookupTactics {
        requested_action: String,
        pretext_category: String,
    },
    OfferHandoff {
        summary: String,
        observation: Observation,
        evidence_refs: Vec<EvidenceRef>,
    },
    StartTrustedCallback {
        contact_id: String,
        reason: CallbackReason,
        evidence_refs: Vec<EvidenceRef>,
    },
    TakeMessage {
        summary: String,
        evidence_refs: Vec<EvidenceRef>,
    },
    EndCall {
        reason: EndReason,
        evidence_refs: Vec<EvidenceRef>,
    },
    PublishTactic {
        tactic: TacticKind,
        requested_action: String,
        pretext_category: String,
        pressure_category: PressureCategory,
    },
}

impl ToolCall {
    pub fn parse(name: &str, mut arguments: Value) -> Result<Self> {
        // The native wire format cites each fact once. Derive display text from
        // that literal quote; the actor still validates its actual caller turn.
        // Keep the old internal shape readable for persisted/replay fixtures.
        if name == "record_assessment"
            && let Some(facts) = arguments
                .pointer_mut("/understanding/supplied_facts")
                .and_then(Value::as_array_mut)
        {
            for fact in facts {
                if fact.get("text").is_none() {
                    let evidence: EvidenceRef = serde_json::from_value(fact.clone())
                        .map_err(|_| anyhow!("invalid supplied fact source"))?;
                    *fact = json!({"text":evidence.quote,"evidence_refs":[evidence]});
                }
            }
        }
        let tool: Self = serde_json::from_value(json!({"name":name,"arguments":arguments}))
            .map_err(|_| anyhow!("unknown tool or invalid tool arguments"))?;
        match &tool {
            Self::RecordAssessment {
                evidence_refs,
                understanding,
                ..
            } => {
                validate_evidence(evidence_refs)?;
                if let Some(value) = understanding {
                    value.validate()?;
                }
            }
            Self::EndCall { evidence_refs, .. } => validate_evidence(evidence_refs)?,
            Self::AskCaller {
                question,
                evidence_refs,
                ..
            } => {
                validate_speech(question, 240)?;
                validate_evidence(evidence_refs)?;
            }
            Self::OfferHandoff {
                summary,
                evidence_refs,
                ..
            }
            | Self::TakeMessage {
                summary,
                evidence_refs,
            } => {
                validate_speech(summary, 360)?;
                validate_evidence(evidence_refs)?;
            }
            Self::StartTrustedCallback {
                contact_id,
                evidence_refs,
                ..
            } => {
                ensure!(
                    !contact_id.is_empty()
                        && contact_id.len() <= 96
                        && contact_id
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || "_:-".contains(c)),
                    "invalid contact identifier"
                );
                validate_evidence(evidence_refs)?;
            }
            Self::LookupTactics {
                requested_action,
                pretext_category,
            }
            | Self::PublishTactic {
                requested_action,
                pretext_category,
                ..
            } => {
                ensure!(
                    REQUESTED_ACTIONS.contains(&requested_action.as_str()),
                    "unsupported tactic action category"
                );
                ensure!(
                    PRETEXT_CATEGORIES.contains(&pretext_category.as_str()),
                    "unsupported pretext category"
                );
            }
            Self::LookupTrustedContact { .. } => {}
        }
        Ok(tool)
    }
}

/// A public proposal summary retains validated categories and omits private text.
pub fn public_arguments(tool: &ToolCall) -> Value {
    match tool {
        ToolCall::RecordAssessment { observation, .. }
        | ToolCall::OfferHandoff { observation, .. } => json!({"observation":observation}),
        ToolCall::AskCaller {
            question, purpose, ..
        } => json!({"purpose":purpose,"question_characters":question.chars().count()}),
        ToolCall::LookupTrustedContact { role } => json!({"role":role}),
        ToolCall::LookupTactics {
            requested_action,
            pretext_category,
        } => json!({"requested_action":requested_action,"pretext_category":pretext_category}),
        ToolCall::StartTrustedCallback { reason, .. } => json!({"reason":reason}),
        ToolCall::TakeMessage { summary, .. } => {
            json!({"summary_characters":summary.chars().count()})
        }
        ToolCall::EndCall { reason, .. } => json!({"reason":reason}),
        ToolCall::PublishTactic {
            tactic,
            requested_action,
            pretext_category,
            pressure_category,
        } => {
            json!({"tactic":tactic,"requested_action":requested_action,"pretext_category":pretext_category,"pressure_category":pressure_category})
        }
    }
}

/// Bound the public outcome to policy-owned booleans/counts. A requested action
/// never becomes a completed or acknowledged call in this projection.
pub fn public_result(value: &Value) -> Value {
    let mut fields = serde_json::Map::new();
    for key in [
        "blocked",
        "ambiguous",
        "assessed_version",
        "audio_requested",
        "message_recorded",
        "closing",
        "stop",
        "acknowledged",
        "summary_characters",
    ] {
        if value[key].is_boolean() || value[key].is_u64() {
            fields.insert(key.into(), value[key].clone());
        }
    }
    for (key, source) in [
        ("contact_available", "contact_id"),
        ("offer_created", "offer_id"),
        ("action_requested", "action_id"),
    ] {
        if value.get(source).is_some() {
            fields.insert(key.into(), json!(true));
        }
    }
    if let Some(matches) = value["matches"].as_array() {
        fields.insert("tactic_matches".into(), json!(matches.len()));
    }
    if value["state"].as_str().is_some_and(|s| {
        matches!(
            s,
            "reserved"
                | "requested"
                | "unknown"
                | "ringing"
                | "in-progress"
                | "completed"
                | "failed"
                | "no-answer"
                | "busy"
                | "canceled"
        )
    }) {
        fields.insert("state".into(), value["state"].clone());
    }
    Value::Object(fields)
}

/// A closed categorical vocabulary prevents names, numbers and quotes entering shared memory.
pub const REQUESTED_ACTIONS: &[&str] = &[
    "relay_sign_in_text_digits",
    "share_authentication_secret",
    "install_remote_access",
    "send_payment",
    "buy_gift_card",
    "routine_information",
    "unknown",
];
pub const PRETEXT_CATEGORIES: &[&str] = &[
    "clinic_appointment",
    "bank_account",
    "government",
    "tech_support",
    "delivery",
    "family_emergency",
    "other",
    "unknown",
];

fn validate_evidence(refs: &[EvidenceRef]) -> Result<()> {
    ensure!(
        !refs.is_empty() && refs.len() <= 8,
        "evidence references required within bounds"
    );
    for item in refs {
        ensure!(
            !item.turn_id.is_empty() && item.turn_id.len() <= 96,
            "invalid evidence turn identifier"
        );
        ensure!(
            !item.quote.trim().is_empty() && item.quote.chars().count() <= 1024,
            "invalid evidence quote length"
        );
    }
    // Existence, exact substring, role and current turn are checked by the owning actor.
    Ok(())
}

fn validate_speech(text: &str, max_chars: usize) -> Result<()> {
    ensure!(
        !text.trim().is_empty() && text.chars().count() <= max_chars,
        "invalid spoken text length"
    );
    ensure!(
        !text.chars().any(|c| c.is_control() || c == '<' || c == '>'),
        "spoken text must be plain text"
    );
    let lower = text.to_ascii_lowercase();
    ensure!(
        !lower.contains("://") && !lower.contains("www."),
        "spoken text cannot contain links"
    );
    Ok(())
}

pub fn schemas() -> Value {
    let observation = object(
        &["speech_act", "requested_object", "direction"],
        json!({
            "speech_act": enumeration(&["request","statement","negation","quotation","unclear"]),
            "requested_object": {"type":"string","enum":["authentication_secret","appointment_reference","payment","payment_diversion","remote_access","routine_information","unknown"],"description":"What information/action is actually sought. Unexplained digits/code sought from the household are unknown even after benign context; account access makes them authentication_secret. Classify a warning's object as the warned-about object while speech_act negation preserves its benign meaning."},
            "direction": {"type":"string","enum":["caller_requests_from_sarah","caller_supplies","none","unclear"],"description":"Flow of information or requested action, not speaker identity. Caller gives an update/reference: caller_supplies. Caller seeks Sarah's secret/payment/action: caller_requests_from_sarah. Quoting a stranger or warning not to share: none. 'Please tell Sarah my train time' supplies information; it does not request a secret from Sarah."}
        }),
    );
    let evidence = json!({"type":"array","minItems":1,"maxItems":8,"items":object(&["turn_id","quote"],json!({"turn_id":string(96),"quote":string(1024)}))});
    let understanding = object(
        &[
            "version",
            "claimed_identity",
            "purpose",
            "requested_action",
            "supplied_facts",
            "missing_information",
            "readiness",
        ],
        json!({
            "version":{"type":"integer","enum":[1]},
            "claimed_identity":{"type":["string","null"],"maxLength":120,"description":"Copy one continuous exact substring of caller words, preserving punctuation. A name alone is enough; never concatenate a name and affiliation from separate phrases. Use null if absent. This is an unverified claim."},
            "purpose":string(180), "requested_action":{"type":"string","maxLength":180,"description":"Who should do what for whom. Preserve the caller's roles: I/me/my refer to the caller, not the household person. Do not reverse the requested actor and recipient."},
            "supplied_facts":{"type":"array","maxItems":MAX_SUPPLIED_FACTS,"description":"Optional literal caller evidence, each quoted once. Never paraphrase. Use [] for sensitive content.","items":object(&["turn_id","quote"],json!({"turn_id":string(96),"quote":string(180)}))},
            "missing_information":{"type":"array","maxItems":3,"items":string(160),"description":"Essential unanswered details for the actual caller purpose. Must be [] for ready or no_message; a caller declining a message does not owe more information. Do not invent mandatory intake."},
            "readiness":{"type":"string","enum":["needs_information","ready","caller_declined","no_message"],"description":"needs_information for an unresolved request for assistance or incomplete opening. ready for a complete notice/update, a clear correction replacing an earlier supplied detail, or a fully understood risky action. Reuse the earlier purpose when the caller corrects one value; do not ask them to repeat or confirm that value. Unspecified AM/PM alone does not make a supplied time incomplete. Never mark an unresolved assistance request ready merely because its words can be saved. A request to arrange a ride, delivery, visit or collection is still an opening when its practical when/where detail is absent; ask for the most useful missing detail. A complete update or explicit callback message does not need extra intake. Unknown code purpose needs_information; established account-access extraction ready for protection, never seek its value."}
        }),
    );
    json!([
        function(
            "record_assessment",
            "Required first for every completed caller turn. Describe meaning, direction, quotation and negation using exact caller evidence. This does not authorize bridging.",
            object(
                &["observation", "evidence_refs", "understanding"],
                json!({"observation":observation,"evidence_refs":evidence,"understanding":understanding})
            )
        ),
        function(
            "ask_caller",
            "Ask one short contextual clarification. If asked whether you are the household person or who is answering, first briefly say you are their automated assistant, then ask the relevant question. Never request credentials, authentication digits or secrets; never reveal private household information.",
            object(
                &["question", "purpose", "evidence_refs"],
                json!({"question":{"type":"string","maxLength":240,"description":"The actual brief spoken reply to the current caller. Follow accepted missing information and server guidance. Preserve who does what for whom; reuse known facts instead of asking again. Do not add an identity introduction or request unless the actual current exchange needs one."},"purpose":enumeration(&["clarify_request","clarify_code_type","clarify_identity"]),"evidence_refs":evidence})
            )
        ),
        function(
            "end_call",
            "End politely without saving a message, only after a current no_message understanding (for example an explicit wrong number). No delivery, verification or contact claim.",
            object(
                &["reason", "evidence_refs"],
                json!({"reason":enumeration(&["wrong_number","caller_finished","no_message"]),"evidence_refs":evidence})
            )
        ),
        function(
            "lookup_trusted_contact",
            "Read the enrolled household directory. Returns an opaque contact identifier, never a caller-provided number.",
            object(&["role"], json!({"role":enumeration(&["family","clinic"])}))
        ),
        function(
            "lookup_tactics",
            "Read scoped unexpired categorical tactic memory. Match requested action and pretext, never caller identity. Does not clear a turn's assessment hold.",
            object(
                &["requested_action", "pretext_category"],
                json!({"requested_action":enumeration(REQUESTED_ACTIONS),"pretext_category":enumeration(PRETEXT_CATEGORIES)})
            )
        ),
        function(
            "offer_handoff",
            "Propose a private, revocable summary and keypad offer only after the current benign assessment. Never bridge directly. Identity remains unverified.",
            object(
                &["summary", "observation", "evidence_refs"],
                json!({"summary":string(360),"observation":observation,"evidence_refs":evidence})
            )
        ),
        function(
            "start_trusted_callback",
            "Request one independent call to a directory contact. Rust authorizes and deduplicates it. A request is not an answer or acknowledgement.",
            object(
                &["contact_id", "reason", "evidence_refs"],
                json!({"contact_id":string(96),"reason":enumeration(&["authentication_request","identity_check","family_help"]),"evidence_refs":evidence})
            )
        ),
        function(
            "take_message",
            "Ends the caller conversation after saving a finalized brief message; this is not a draft or intermediate note. First ask for essential missing details relevant to the caller's actual purpose. Finalize when those details are supplied, or the caller explicitly declines further detail or wants to end. Never include credentials, authentication digits, links or verification claims.",
            object(
                &["summary", "evidence_refs"],
                json!({"summary":string(360),"evidence_refs":evidence})
            )
        ),
        function(
            "publish_tactic",
            "POST-CALL ONLY: propose categorical tactic memory for a fictional designated incident. No names, numbers, transcripts, credentials or recordings. Rust sets provenance and expiry.",
            object(
                &[
                    "tactic",
                    "requested_action",
                    "pretext_category",
                    "pressure_category"
                ],
                json!({"tactic":enumeration(&["authentication_code_extraction","remote_access_request","payment_diversion"]),"requested_action":enumeration(REQUESTED_ACTIONS),"pretext_category":enumeration(PRETEXT_CATEGORIES),"pressure_category":enumeration(&["cancellation","urgency","threat","none","unknown"])})
            )
        )
    ])
}

fn string(max: u32) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}
fn enumeration(values: &[&str]) -> Value {
    json!({"type":"string","enum":values})
}
fn object(required: &[&str], properties: Value) -> Value {
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}
fn function(name: &str, description: &str, parameters: Value) -> Value {
    json!({"type":"function","function":{"name":name,"description":description,"parameters":parameters}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assessment() -> Value {
        json!({"observation":{"speech_act":"request","requested_object":"authentication_secret","direction":"caller_requests_from_sarah"},"evidence_refs":[{"turn_id":"turn-3","quote":"read the sign-in digits"}]})
    }

    #[test]
    fn current_evidence_can_be_parsed_without_accepting_control_fields() {
        let args = assessment();
        assert!(matches!(
            ToolCall::parse("record_assessment", args.clone()).unwrap(),
            ToolCall::RecordAssessment { .. }
        ));
        let mut injected = args;
        injected["session_id"] = json!("attacker-session");
        assert!(ToolCall::parse("record_assessment", injected).is_err());
    }

    #[test]
    fn missing_observation_fields_and_invented_enums_are_rejected() {
        let mut args = assessment();
        args["observation"]
            .as_object_mut()
            .unwrap()
            .remove("direction");
        assert!(ToolCall::parse("record_assessment", args).is_err());
        let mut args = assessment();
        args["observation"]["speech_act"] = json!("verified_identity");
        assert!(ToolCall::parse("record_assessment", args).is_err());
    }

    #[test]
    fn callback_cannot_smuggle_a_destination_or_url() {
        let mut args = json!({"contact_id":"family-sarah","reason":"family_help","evidence_refs":[{"turn_id":"t1","quote":"please help"}]});
        assert!(ToolCall::parse("start_trusted_callback", args.clone()).is_ok());
        args["number"] = json!("+12025550123");
        assert!(ToolCall::parse("start_trusted_callback", args).is_err());
        assert!(ToolCall::parse("dial", json!({"number":"+12025550123"})).is_err());
    }

    #[test]
    fn nested_evidence_and_empty_evidence_fail_closed() {
        let mut args = assessment();
        args["evidence_refs"][0]["url"] = json!("https://untrusted.invalid");
        assert!(ToolCall::parse("record_assessment", args).is_err());
        let mut args = assessment();
        args["evidence_refs"] = json!([]);
        assert!(ToolCall::parse("record_assessment", args).is_err());
    }

    #[test]
    fn speech_text_cannot_include_markup_or_arbitrary_links() {
        let mut args = json!({"question":"What is the appointment reference for?","purpose":"clarify_code_type","evidence_refs":[{"turn_id":"t2","quote":"appointment code"}]});
        assert!(ToolCall::parse("ask_caller", args.clone()).is_ok());
        args["question"] = json!("<audio src=\"https://untrusted.invalid\"/>");
        assert!(ToolCall::parse("ask_caller", args).is_err());
    }

    #[test]
    fn publication_accepts_categories_and_rejects_personal_text() {
        let mut args = json!({"tactic":"authentication_code_extraction","requested_action":"relay_sign_in_text_digits","pretext_category":"clinic_appointment","pressure_category":"cancellation"});
        assert!(ToolCall::parse("publish_tactic", args.clone()).is_ok());
        args["pretext_category"] = json!("Sarah appointment Tuesday at 2 +12025550123");
        assert!(ToolCall::parse("publish_tactic", args).is_err());
    }

    #[test]
    fn offered_schema_has_required_observation_and_no_dial_parameters() {
        let schemas = schemas();
        let schemas = schemas.as_array().unwrap();
        assert_eq!(schemas.len(), 9);
        for schema in schemas {
            let parameters = &schema["function"]["parameters"];
            assert_eq!(parameters["additionalProperties"], false);
            assert!(parameters["properties"].get("number").is_none());
            assert!(parameters["properties"].get("url").is_none());
        }
        let handoff = schemas
            .iter()
            .find(|s| s["function"]["name"] == "offer_handoff")
            .unwrap();
        assert_eq!(
            handoff["function"]["parameters"]["required"],
            json!(["summary", "observation", "evidence_refs"])
        );
    }
}
