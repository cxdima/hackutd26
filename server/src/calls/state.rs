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
pub struct Session {
    pub id: String,
    pub household_id: String,
    pub state: String,
    pub policy_epoch: u64,
    pub generation_id: u64,
    pub speech_pending: bool,
    pub inbound_ended: bool,
    pub turns: Vec<Turn>,
    pub current_action: String,
    pub evidence_quote: Option<String>,
    pub offer: Option<Offer>,
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
            state: "screening".into(),
            policy_epoch: 0,
            generation_id: 0,
            speech_pending: false,
            inbound_ended: false,
            turns: vec![],
            current_action: "Listening to the caller".into(),
            evidence_quote: None,
            offer: None,
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
    pub fn speech_started(&mut self) {
        if self.bridge_committed || self.inbound_ended {
            return;
        }
        self.generation_id += 1;
        self.speech_pending = true;
        self.current_action = "Listening; private acceptance on hold".into();
    }
    pub fn commit_turn(&mut self, text: &str) -> String {
        if self.bridge_committed || self.inbound_ended {
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
        self.evidence_quote = Some(evidence[0].quote.clone());
        Ok(())
    }
    pub fn make_offer(
        &mut self,
        summary: &str,
        leg_id: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Offer> {
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
