use crate::config::ExecutionMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Leg {
    pub action_id: String,
    pub role: String,
    pub kind: String,
    pub status: String,
    pub acknowledged: bool,
    pub call_sid: Option<String>,
}
impl Leg {
    pub fn apply_status(&mut self, call_sid: &str, status: &str) -> anyhow::Result<bool> {
        anyhow::ensure!(!call_sid.is_empty(), "Missing leg identity");
        anyhow::ensure!(
            self.call_sid.as_deref().is_none_or(|sid| sid == call_sid),
            "Wrong leg"
        );
        anyhow::ensure!(status_rank(status).is_some(), "Unknown provider status");
        self.call_sid = Some(call_sid.to_string());
        if terminal(&self.status) || status_rank(status) <= status_rank(&self.status) {
            return Ok(false);
        }
        self.status = status.to_string();
        Ok(true)
    }
    pub fn acknowledge(&mut self, call_sid: &str, digits: &str) -> bool {
        if self.call_sid.as_deref() != Some(call_sid)
            || digits != "1"
            || self.acknowledged
            || matches!(
                self.status.as_str(),
                "no-answer" | "busy" | "failed" | "canceled"
            )
        {
            return false;
        }
        self.acknowledged = true;
        true
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Turn {
    pub id: String,
    pub text: String,
    pub speaker: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Offer {
    pub id: String,
    pub summary: String,
    pub status: String,
    pub policy_epoch: u64,
    pub expires_at: String,
    pub leg_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordedMessage {
    pub summary: String,
    pub recorded_at: String,
    pub turn_version: u64,
    pub evidence_turn_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcceptedUnderstanding {
    pub understanding: crate::tools::CallUnderstanding,
    pub generation_id: u64,
    pub turn_version: u64,
    pub policy_epoch: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub household_id: String,
    #[serde(default)]
    pub profile: Option<crate::profile::Profile>,
    pub state: String,
    pub policy_epoch: u64,
    pub generation_id: u64,
    pub speech_pending: bool,
    pub inbound_ended: bool,
    pub turns: Vec<Turn>,
    pub current_action: String,
    pub evidence_quote: Option<String>,
    pub offer: Option<Offer>,
    #[serde(default)]
    pub message: Option<RecordedMessage>,
    #[serde(default)]
    pub understanding: Option<AcceptedUnderstanding>,
    #[serde(default)]
    pub closing_reason: Option<String>,
    pub legs: Vec<Leg>,
    pub execution_mode: ExecutionMode,
    pub data_origin: String,
    pub inbound_sid: String,
    pub relay_token: String,
    pub turn_version: u64,
    pub assessed_version: u64,
    pub blocked: bool,
    pub ambiguous: bool,
    pub bridge_committed: bool,
    pub conference: String,
    pub joined: Vec<String>,
    #[serde(default)]
    pub gather_token: String,
    #[serde(default)]
    pub gather_claims: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub gather_replies: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub end_requested: bool,
    #[serde(default)]
    pub bridge_a_requested: bool,
    #[serde(default)]
    pub assessments: Vec<serde_json::Value>,
    #[serde(default)]
    pub offer_history: Vec<Offer>,
    #[serde(default)]
    pub cleanup_after: std::collections::BTreeMap<String, String>,
}

pub fn terminal(status: &str) -> bool {
    matches!(
        status,
        "completed" | "busy" | "failed" | "no-answer" | "canceled"
    )
}
fn status_rank(status: &str) -> Option<u8> {
    Some(match status {
        "reserved" => 0,
        "unknown" => 1,
        "queued" => 2,
        "initiated" => 3,
        "ringing" => 4,
        "in-progress" => 5,
        "completed" | "busy" | "failed" | "no-answer" | "canceled" => 6,
        _ => return None,
    })
}
impl Session {
    pub fn new(id: String, inbound_sid: String, household_id: String, mode: ExecutionMode) -> Self {
        Self {
            id,
            household_id,
            profile: None,
            state: "screening".into(),
            policy_epoch: 0,
            generation_id: 0,
            speech_pending: false,
            inbound_ended: false,
            turns: vec![],
            current_action: "Listening to the caller".into(),
            evidence_quote: None,
            offer: None,
            message: None,
            understanding: None,
            closing_reason: None,
            legs: vec![],
            execution_mode: mode,
            data_origin: "synthetic".into(),
            inbound_sid,
            relay_token: uuid::Uuid::new_v4().to_string(),
            turn_version: 0,
            assessed_version: 0,
            blocked: false,
            ambiguous: false,
            bridge_committed: false,
            conference: format!("firstring-{}", uuid::Uuid::new_v4()),
            joined: vec![],
            gather_token: String::new(),
            gather_claims: std::collections::BTreeMap::new(),
            gather_replies: std::collections::BTreeMap::new(),
            end_requested: false,
            bridge_a_requested: false,
            assessments: vec![],
            offer_history: vec![],
            cleanup_after: std::collections::BTreeMap::new(),
        }
    }
    pub fn public_value(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).expect("session serializable");
        if let Some(m) = value.as_object_mut() {
            for key in [
                "inbound_sid",
                "relay_token",
                "conference",
                "joined",
                "gather_token",
                "gather_claims",
                "gather_replies",
            ] {
                m.remove(key);
            }
            if let Some(legs) = m.get_mut("legs").and_then(|v| v.as_array_mut()) {
                for leg in legs {
                    leg.as_object_mut().unwrap().remove("call_sid");
                }
            }
        }
        value
    }
}
impl Session {
    pub fn current_understanding(&self) -> Option<&crate::tools::CallUnderstanding> {
        self.understanding
            .as_ref()
            .filter(|value| {
                value.generation_id == self.generation_id
                    && value.turn_version == self.turn_version
                    && value.policy_epoch == self.policy_epoch
                    && self.assessed_version == self.turn_version
                    && !self.speech_pending
            })
            .map(|value| &value.understanding)
    }
    pub fn accept_understanding(
        &mut self,
        value: crate::tools::CallUnderstanding,
    ) -> anyhow::Result<()> {
        value.validate()?;
        for fact in &value.supplied_facts {
            self.evidence_valid(&fact.evidence_refs, false)?;
        }
        if let Some(identity) = &value.claimed_identity {
            anyhow::ensure!(
                self.turns
                    .iter()
                    .any(|turn| turn.speaker == "caller" && turn.text.contains(identity)),
                "Identity claim must quote caller words"
            );
        }
        self.understanding = Some(AcceptedUnderstanding {
            understanding: value,
            generation_id: self.generation_id,
            turn_version: self.turn_version,
            policy_epoch: self.policy_epoch,
        });
        Ok(())
    }
    pub fn require_message_readiness(&self) -> anyhow::Result<()> {
        // Authored legacy replay checkpoints remain usable; a native model result
        // is separately required to carry understanding, even in inert evaluation.
        if self.blocked
            || (self.execution_mode == ExecutionMode::Replay && self.understanding.is_none())
        {
            return Ok(());
        }
        anyhow::ensure!(
            self.current_understanding().is_some_and(|u| matches!(
                u.readiness,
                crate::tools::Readiness::Ready | crate::tools::Readiness::CallerDeclined
            )),
            "Message is not ready: ask for the identified missing information before closing"
        );
        Ok(())
    }
    pub fn require_end_call(&self, evidence: &[crate::tools::EvidenceRef]) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.inbound_ended
                && !self.bridge_committed
                && !self.end_requested
                && self.closing_reason.is_none()
                && !self.blocked
                && !self.speech_pending,
            "Ordinary closing is not authorized in the current state"
        );
        self.evidence_valid(evidence, true)?;
        anyhow::ensure!(
            self.current_understanding()
                .is_some_and(|u| u.readiness == crate::tools::Readiness::NoMessage),
            "Ordinary closing needs a current no-message understanding"
        );
        Ok(())
    }
    pub fn speech_started(&mut self) {
        if self.bridge_committed || self.inbound_ended || self.closing_reason.is_some() {
            return;
        }
        self.generation_id += 1;
        self.speech_pending = true;
        self.current_action = "Listening; private acceptance on hold".into();
    }
    pub fn commit_turn(&mut self, text: &str) -> String {
        if self.bridge_committed || self.inbound_ended || self.closing_reason.is_some() {
            return String::new();
        }
        self.generation_id += 1;
        self.turn_version += 1;
        self.speech_pending = false;
        if let Some(o) = self.offer.as_mut()
            && o.status == "pending"
        {
            o.status = "revoked".into();
            self.policy_epoch += 1;
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.turns.push(Turn {
            id: id.clone(),
            text: text.into(),
            speaker: "caller".into(),
        });
        self.current_action = "Assessing the caller's request".into();
        id
    }
    pub fn evidence_valid(
        &self,
        evidence: &[crate::tools::EvidenceRef],
        require_current: bool,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(!evidence.is_empty(), "Evidence is required");
        let current = self
            .turns
            .iter()
            .rfind(|t| t.speaker == "caller")
            .ok_or_else(|| anyhow::anyhow!("No committed turn"))?;
        for e in evidence {
            anyhow::ensure!(
                !e.quote.trim().is_empty()
                    && self.turns.iter().any(|t| t.speaker == "caller"
                        && t.id == e.turn_id
                        && t.text.contains(&e.quote)),
                "Evidence must quote a committed caller turn"
            );
        }
        anyhow::ensure!(
            !require_current || evidence.iter().any(|e| e.turn_id == current.id),
            "Assessment needs evidence from the current turn"
        );
        Ok(())
    }
    pub fn record_message(
        &mut self,
        summary: &str,
        evidence: &[crate::tools::EvidenceRef],
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.speech_pending
                && !self.inbound_ended
                && !self.bridge_committed
                && self.turn_version > 0
                && self.assessed_version == self.turn_version,
            "Assess the current turn before completing a message"
        );
        self.evidence_valid(evidence, true)?;
        self.require_message_readiness()?;
        // This detects wording requiring privacy minimization, never call risk.
        fn credential_terms(text: &str) -> bool {
            let lower = text.to_lowercase();
            lower.split(|c: char| !c.is_alphabetic()).any(|word| {
                matches!(
                    word.strip_suffix('s').unwrap_or(word),
                    "password" | "passcode" | "otp" | "secret" | "pin"
                )
            }) || [
                "one-time",
                "one time",
                "sign-in",
                "sign in",
                "verification code",
                "authentication code",
                "access token",
                "recovery code",
            ]
            .iter()
            .any(|term| lower.contains(term))
        }
        // Minimize from already accepted conversation semantics, without performing
        // another risk classification. Never copy credential-bearing wording
        // from an authentication discussion into its saved message.
        let observation = self
            .assessments
            .iter()
            .rev()
            .find(|assessment| {
                assessment["observation"]["requested_object"] == "authentication_secret"
                    || (matches!(
                        assessment["observation"]["speech_act"].as_str(),
                        Some("quotation" | "negation")
                    ) && assessment["evidence_refs"]
                        .as_array()
                        .is_some_and(|references| {
                            references.iter().any(|reference| {
                                reference["quote"].as_str().is_some_and(credential_terms)
                            })
                        }))
            })
            .map(|assessment| &assessment["observation"]);
        // A later ordinary turn cannot relabel a previously disclosed credential
        // as a harmless number. Conservatively keep these conversations categorical;
        // this may omit a later benign detail, but does not alter their risk policy.
        let summary = if self.blocked {
            "Suspicious request blocked; independent verification recommended."
        } else if let Some(observation) = observation {
            match observation["speech_act"].as_str() {
                Some("negation") => "Caller shared a warning about protecting account information.",
                Some("quotation") => {
                    "Caller reported a discussion involving sensitive account information."
                }
                _ => "Caller discussed sensitive account information; sensitive details omitted.",
            }
        } else {
            summary
        };
        // Credential material unexpectedly placed in an ordinary message is
        // rejected. Only evidence-backed accepted quotation/negation above can
        // select a categorical note; a model summary cannot invent that context.
        anyhow::ensure!(
            !credential_terms(summary),
            "Remove authentication details from the message summary"
        );
        // A bounded fidelity check, not temporal interpretation: adding an
        // AM/PM qualifier requires positive caller wording in cited evidence.
        // Preserve an unqualified time rather than guessing its day period.
        fn meridiems(text: &str, day_periods: bool) -> u8 {
            let lower = text
                .to_lowercase()
                .replace("a.m.", "am")
                .replace("p.m.", "pm")
                .replace([',', ';', '.', '!', '?'], " | ");
            let words: Vec<_> = lower
                .split(|c: char| !c.is_ascii_alphanumeric() && c != ':' && c != '|')
                .filter(|s| !s.is_empty())
                .collect();
            let clock = |word: &str| {
                word.chars().any(|c| c.is_ascii_digit())
                    || [
                        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
                        "ten", "eleven", "twelve", "fifteen", "thirty", "forty", "fifty",
                    ]
                    .contains(&word)
            };
            let mut mask = 0;
            let mut clause_start = 0;
            for (index, word) in words.iter().enumerate() {
                if *word == "|" {
                    clause_start = index + 1;
                    continue;
                }
                if day_periods
                    && words[index.saturating_sub(3).max(clause_start)..index]
                        .iter()
                        .any(|w| ["not", "never", "no"].contains(w))
                {
                    continue;
                }
                for (suffix, bit) in [("am", 1), ("pm", 2)] {
                    if word.strip_suffix(suffix).is_some_and(|prefix| {
                        if prefix.is_empty() {
                            index > 0 && clock(words[index - 1])
                        } else {
                            clock(prefix)
                        }
                    }) {
                        mask |= bit;
                    }
                }
                if day_periods {
                    mask |= match *word {
                        "morning" => 1,
                        "afternoon" | "evening" | "tonight" => 2,
                        _ => 0,
                    };
                }
            }
            mask
        }
        let supported_periods = evidence.iter().fold(0, |mask, reference| {
            mask | meridiems(&reference.quote, true)
        });
        anyhow::ensure!(
            meridiems(summary, false) & !supported_periods == 0,
            "AM/PM in the message needs explicit caller evidence; preserve the unqualified time"
        );
        // Retain numeric facts such as times, dates, quantities and prices.
        // Contact-shaped literals remain unnecessary: actions resolve only
        // enrolled contacts. A bare long number can be ambiguous, so phone-like
        // sequences are conservatively redacted; this is a bounded shape check.
        fn date(word: &str) -> bool {
            let value = word.trim_matches(|c| matches!(c, ',' | '.' | ';' | '(' | ')'));
            let parts: Vec<_> = value.split(['/', '-']).collect();
            let number = |s: &str| s.parse::<u32>().ok();
            match parts.as_slice() {
                [month, day] if value.contains('/') => {
                    number(month).is_some_and(|v| (1..=12).contains(&v))
                        && number(day).is_some_and(|v| (1..=31).contains(&v))
                }
                [first, second, third] => {
                    let (month, day, year) = if first.len() == 4 {
                        (*second, *third, *first)
                    } else {
                        (*first, *second, *third)
                    };
                    matches!(year.len(), 2 | 4)
                        && number(year).is_some()
                        && number(month).is_some_and(|v| (1..=12).contains(&v))
                        && number(day).is_some_and(|v| (1..=31).contains(&v))
                }
                _ => false,
            }
        }
        fn phone_piece(word: &str) -> bool {
            !date(word)
                && word.chars().any(|c| c.is_ascii_digit())
                && word.chars().all(|c| {
                    c.is_ascii_digit() || matches!(c, '+' | '-' | '(' | ')' | '.' | ',' | ';')
                })
        }
        let words: Vec<_> = summary.split_whitespace().collect();
        let mut minimized = Vec::with_capacity(words.len());
        let mut i = 0;
        while i < words.len() {
            if words[i].contains('@') {
                minimized.push("[redacted]");
                i += 1;
                continue;
            }
            let mut end = i;
            let mut digits = 0;
            let mut marked = false;
            while end < words.len() && end - i < 6 && phone_piece(words[end]) {
                let next = words[end].chars().filter(char::is_ascii_digit).count();
                if digits + next > 15 {
                    break;
                }
                digits += next;
                marked |= words[end].contains(['+', '-', '(', ')']);
                end += 1;
                if words[end - 1].ends_with([',', ';', '.']) {
                    break;
                }
            }
            if digits >= 7 && (marked || digits >= 10) {
                minimized.push("[redacted]");
                i = end;
            } else {
                minimized.push(words[i]);
                i += 1;
            }
        }
        let summary: String = minimized.join(" ").chars().take(360).collect();
        anyhow::ensure!(!summary.is_empty(), "Message summary is empty");
        self.message = Some(RecordedMessage {
            summary,
            recorded_at: now.to_rfc3339(),
            turn_version: self.turn_version,
            evidence_turn_ids: evidence
                .iter()
                .map(|reference| reference.turn_id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
        });
        self.current_action = "Message received; caller identity unverified".into();
        Ok(())
    }
    pub fn assess(
        &mut self,
        generation: u64,
        version: u64,
        observation: &crate::tools::Observation,
        evidence: &[crate::tools::EvidenceRef],
    ) -> anyhow::Result<()> {
        use crate::tools::{Direction, RequestedObject, SpeechAct};
        anyhow::ensure!(
            generation == self.generation_id
                && version == self.turn_version
                && !self.bridge_committed,
            "Stale assessment"
        );
        self.evidence_valid(evidence, true)?;
        self.assessed_version = version;
        self.assessments.push(serde_json::json!({"turn_version":version,"generation_id":generation,"observation":observation,"evidence_refs":evidence}));
        self.ambiguous = observation.speech_act == SpeechAct::Unclear
            || observation.requested_object == RequestedObject::Unknown
            || observation.direction == Direction::Unclear;
        let risky = observation.speech_act == SpeechAct::Request
            && observation.direction == Direction::CallerRequestsFromSarah
            && matches!(
                observation.requested_object,
                RequestedObject::AuthenticationSecret
                    | RequestedObject::RemoteAccess
                    | RequestedObject::PaymentDiversion
            );
        if risky {
            self.blocked = true;
            self.policy_epoch += 1;
            if let Some(o) = self.offer.as_mut()
                && o.status == "pending"
            {
                o.status = "revoked".into();
            }
            self.current_action = "Offer withdrawn. Trusted contact needed".into();
            self.state = "protected".into();
        }
        self.evidence_quote = self
            .turns
            .iter()
            .rfind(|turn| turn.speaker == "caller")
            .and_then(|turn| {
                evidence
                    .iter()
                    .find(|reference| reference.turn_id == turn.id)
            })
            .map(|reference| reference.quote.clone());
        Ok(())
    }
    pub fn make_offer(
        &mut self,
        summary: &str,
        leg_id: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Offer> {
        if self.execution_mode == ExecutionMode::Live || self.understanding.is_some() {
            anyhow::ensure!(
                self.current_understanding()
                    .is_some_and(|u| u.readiness == crate::tools::Readiness::Ready),
                "Private offer needs complete current caller understanding"
            );
        }
        anyhow::ensure!(
            !self.inbound_ended
                && !self.bridge_committed
                && !self.speech_pending
                && !self.blocked
                && !self.ambiguous
                && self.turn_version > 0
                && self.assessed_version == self.turn_version,
            "Handoff is held by policy"
        );
        anyhow::ensure!(
            self.legs.iter().any(|l| l.action_id == leg_id
                && l.role == "sarah"
                && l.kind == "handoff"
                && !terminal(&l.status)),
            "Invalid intended handoff leg"
        );
        anyhow::ensure!(
            !self.offer.as_ref().is_some_and(|o| o.status == "pending"),
            "An offer already exists"
        );
        let o = Offer {
            id: uuid::Uuid::new_v4().to_string(),
            summary: summary.into(),
            status: "pending".into(),
            policy_epoch: self.policy_epoch,
            expires_at: (now + chrono::Duration::seconds(30)).to_rfc3339(),
            leg_id: leg_id.into(),
        };
        if let Some(previous) = self.offer.take() {
            self.offer_history.push(previous);
        }
        self.offer = Some(o.clone());
        self.state = "offer_pending".into();
        self.current_action = format!(
            "Waiting for {}'s choice",
            if self.household_id == "maya" {
                "Maya"
            } else {
                "Sarah"
            }
        );
        Ok(o)
    }
    pub fn accept(
        &mut self,
        offer_id: &str,
        call_sid: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.bridge_committed
                && !self.inbound_ended
                && !self.speech_pending
                && !self.blocked
                && !self.ambiguous
                && self.assessed_version == self.turn_version,
            "Acceptance gate closed"
        );
        let o = self
            .offer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No offer"))?;
        anyhow::ensure!(
            o.id == offer_id && o.status == "pending" && o.policy_epoch == self.policy_epoch,
            "Obsolete offer"
        );
        let expires = chrono::DateTime::parse_from_rfc3339(&o.expires_at)?;
        if now >= expires {
            o.status = "expired".into();
            anyhow::bail!("Offer expired");
        }
        anyhow::ensure!(
            self.legs.iter().any(|l| l.action_id == o.leg_id
                && l.role == "sarah"
                && l.kind == "handoff"
                && l.call_sid.as_deref() == Some(call_sid)
                && !terminal(&l.status)),
            "Wrong or ended private leg"
        );
        o.status = "accepted".into();
        self.bridge_committed = true;
        self.state = "bridge_committed".into();
        self.current_action = "Connecting the approved participants".into();
        Ok(())
    }
    pub fn conference_join(&mut self, call_sid: &str) -> bool {
        if !self.bridge_committed
            || self.state != "bridge_committed"
            || self.end_requested
            || self.inbound_ended
        {
            return false;
        }
        let Some(o) = &self.offer else {
            return false;
        };
        let Some(private) = self
            .legs
            .iter()
            .find(|l| l.action_id == o.leg_id)
            .and_then(|l| l.call_sid.as_deref())
        else {
            return false;
        };
        if call_sid != self.inbound_sid && call_sid != private {
            return false;
        }
        if !self.joined.iter().any(|s| s == call_sid) {
            self.joined.push(call_sid.into());
        }
        if self.joined.iter().any(|s| s == &self.inbound_sid)
            && self.joined.iter().any(|s| s == private)
        {
            self.state = "connected".into();
            self.current_action = "Human call connected. Screening ended".into();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod message_snapshot_tests {
    use super::*;

    #[test]
    fn assessment_display_quotes_current_caller_evidence_and_keeps_prior_context() {
        use crate::tools::{Direction, EvidenceRef, Observation, RequestedObject, SpeechAct};
        let mut session = Session::new(
            "s".into(),
            "ca".into(),
            "sarah".into(),
            ExecutionMode::Replay,
        );
        let earlier = session.commit_turn("I am calling from the clinic.");
        let current = session.commit_turn("Read me the sign-in digits.");
        session.turns.push(Turn {
            id: "assistant".into(),
            text: "One moment.".into(),
            speaker: "assistant".into(),
        });
        let evidence = [
            EvidenceRef {
                turn_id: earlier,
                quote: "clinic".into(),
            },
            EvidenceRef {
                turn_id: current,
                quote: "sign-in digits".into(),
            },
        ];
        let observation = Observation {
            speech_act: SpeechAct::Request,
            requested_object: RequestedObject::AuthenticationSecret,
            direction: Direction::CallerRequestsFromSarah,
        };
        assert!(
            session
                .assess(
                    session.generation_id,
                    session.turn_version,
                    &observation,
                    &evidence[..1]
                )
                .is_err()
        );
        session
            .assess(
                session.generation_id,
                session.turn_version,
                &observation,
                &evidence,
            )
            .unwrap();
        assert_eq!(session.evidence_quote.as_deref(), Some("sign-in digits"));
        assert_eq!(
            session.assessments[0]["evidence_refs"],
            serde_json::to_value(&evidence).unwrap()
        );
        assert!(session.blocked);
        assert_eq!(session.public_value()["evidence_quote"], "sign-in digits");
    }

    #[test]
    fn old_session_snapshots_default_to_no_saved_message() {
        let session = Session::new(
            "s".into(),
            "ca".into(),
            "sarah".into(),
            ExecutionMode::Replay,
        );
        let mut old = serde_json::to_value(session).unwrap();
        old.as_object_mut().unwrap().remove("message");
        let recovered: Session = serde_json::from_value(old).unwrap();
        assert!(recovered.message.is_none());
        assert!(recovered.public_value()["message"].is_null());
    }
}
