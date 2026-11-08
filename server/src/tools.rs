use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    PublishTactic {
        tactic: TacticKind,
        requested_action: String,
        pretext_category: String,
        pressure_category: PressureCategory,
    },
}

impl ToolCall {
    pub fn parse(name: &str, arguments: Value) -> Result<Self> {
        let tool: Self = serde_json::from_value(json!({"name":name,"arguments":arguments}))
            .map_err(|_| anyhow!("unknown tool or invalid tool arguments"))?;
        match &tool {
            Self::RecordAssessment { evidence_refs, .. } => validate_evidence(evidence_refs)?,
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
            "requested_object": enumeration(&["authentication_secret","appointment_reference","payment","payment_diversion","remote_access","routine_information","unknown"]),
            "direction": enumeration(&["caller_requests_from_sarah","caller_supplies","none","unclear"])
        }),
    );
    let evidence = json!({"type":"array","minItems":1,"maxItems":8,"items":object(&["turn_id","quote"],json!({"turn_id":string(96),"quote":string(1024)}))});
    json!([
        function(
            "record_assessment",
            "Required first for every completed caller turn. Describe meaning, direction, quotation and negation using exact caller evidence. This does not authorize bridging.",
            object(
                &["observation", "evidence_refs"],
                json!({"observation":observation,"evidence_refs":evidence})
            )
        ),
        function(
            "ask_caller",
            "Ask one short contextual clarification. Never request credentials, authentication digits or secrets; never reveal private household information.",
            object(
                &["question", "purpose", "evidence_refs"],
                json!({"question":string(240),"purpose":enumeration(&["clarify_request","clarify_code_type","clarify_identity"]),"evidence_refs":evidence})
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
            "Save a brief minimized message without credentials, authentication digits, links or verification claims.",
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
        assert_eq!(schemas.len(), 8);
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
