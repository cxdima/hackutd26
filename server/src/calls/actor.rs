use crate::{
    calls::state::{Leg, Session, terminal},
    config::{Config, ExecutionMode},
    providers::twilio::{self, CreateOutcome, Twilio},
    store::Store,
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, mpsc, oneshot};

#[derive(Clone)]
pub struct App {
    pub config: Arc<Config>,
    pub store: Store,
    pub twilio: Twilio,
    pub sessions: Arc<Mutex<HashMap<String, Handle>>>,
}
#[derive(Clone)]
pub struct Handle {
    pub id: String,
    tx: mpsc::Sender<Envelope>,
    pub audio: tokio::sync::broadcast::Sender<AudioFrame>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPurpose {
    Response,
    MessageClosing,
    ModelFailureClosing,
    PendingSpeechRecovery,
}
#[derive(Clone, Debug)]
pub struct AudioFrame {
    pub generation: u64,
    pub epoch: u64,
    pub payload: Value,
    pub purpose: AudioPurpose,
}
impl AudioFrame {
    pub fn permitted_for(&self, session: &Session) -> bool {
        if self.generation != session.generation_id
            || self.epoch != session.policy_epoch
            || session.inbound_ended
            || session.end_requested
            || session.bridge_committed
        {
            return false;
        }
        match self.purpose {
            AudioPurpose::Response => !session.speech_pending && session.closing_reason.is_none(),
            AudioPurpose::MessageClosing => {
                session.closing_reason.as_deref() == Some("message_saved")
                    && self.payload["type"] == "end"
            }
            AudioPurpose::ModelFailureClosing => {
                session.closing_reason.as_deref() == Some("model_failure")
                    && self.payload["type"] == "end"
            }
            AudioPurpose::PendingSpeechRecovery => {
                session.speech_pending
                    && session.closing_reason.is_none()
                    && self.payload["type"] == "text"
            }
        }
    }
}
pub(crate) struct PendingSpeechEpisode {
    pub(crate) first_started: std::time::Instant,
    pub(crate) deadline: std::time::Instant,
    pub(crate) generation: u64,
    pub(crate) epoch: u64,
    pub(crate) prompted: bool,
}
struct Envelope {
    input: Input,
    reply: oneshot::Sender<anyhow::Result<Value>>,
}
pub enum Input {
    Snapshot,
    GatherClaim {
        nonce: String,
    },
    GatherComplete {
        nonce: String,
        twiml: String,
    },
    Probe {
        contact_id: String,
    },
    Status {
        action_id: String,
        sid: String,
        status: String,
    },
    ProviderCreated {
        action_id: String,
        outcome: CreateOutcome,
    },
    Alert {
        action_id: String,
        sid: String,
    },
    Ack {
        action_id: String,
        sid: String,
        digits: String,
    },
    Keypad {
        sid: String,
        digits: String,
    },
    InboundEnded,
    ConnectRelay {
        sid: String,
        token: String,
    },
    Speech {
        partial: Option<String>,
    },
    SpeechRecovery {
        generation: u64,
        epoch: u64,
    },
    FinalPrompt {
        text: String,
    },
    ModelTool {
        generation: u64,
        epoch: u64,
        version: u64,
        deadline: std::time::Instant,
        tool: crate::providers::nemotron::ModelToolCall,
    },
    ModelFailed {
        generation: u64,
        detail: String,
    },
    Whisper {
        offer_id: String,
        sid: String,
    },
    Choice {
        offer_id: String,
        sid: String,
        digits: String,
    },
    Conference {
        name: String,
        sid: String,
        event: String,
    },
    Cleanup {
        action_id: String,
    },
    BridgeResult {
        ok: bool,
    },
    BridgeTimeout,
    PostCallTool {
        tool: crate::providers::nemotron::ModelToolCall,
    },
    RelayDisconnected,
    ClosingResponse,
    RelayTelemetry {
        event: &'static str,
        elapsed_ms: u64,
        characters: Option<usize>,
    },

    End,
}
impl Handle {
    pub async fn send(&self, input: Input) -> anyhow::Result<Value> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Envelope { input, reply: tx })
            .await
            .map_err(|_| anyhow::anyhow!("Session is closed"))?;
        tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .map_err(|_| anyhow::anyhow!("Session busy"))?
            .map_err(|_| anyhow::anyhow!("Session unavailable"))?
    }
}
impl App {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config = Arc::new(config);
        let store = Store::open(&config.database_url).await?;
        let twilio = Twilio::new(config.clone())?;
        let app = Self {
            config,
            store,
            twilio,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };
        for mut s in app.store.sessions().await? {
            s.legs = app.store.actions(&s.id).await?;
            for leg in &mut s.legs {
                if leg.status == "reserved" {
                    leg.status = "unknown".into();
                }
            }
            // Restart never resumes inference or submits an old reservation. Signed callbacks
            // can still reconcile children; fresh consent requires a new inbound session.
            s.inbound_ended = true;
            s.speech_pending = true;
            if let Some(offer) = s.offer.as_mut()
                && offer.status == "pending"
            {
                offer.status = "revoked".into();
                s.policy_epoch += 1;
            }
            s.current_action = "Recovered session; awaiting terminal receipts".into();
            app.store.save(&s).await?;
            app.spawn(s).await;
        }
        std::mem::drop(crate::patterns::spawn_subscriber(app.store.clone()));
        Ok(app)
    }
    async fn spawn(&self, session: Session) -> Handle {
        let (tx, rx) = mpsc::channel(64);
        let (audio, _) = tokio::sync::broadcast::channel(16);
        let handle = Handle {
            id: session.id.clone(),
            tx,
            audio,
        };
        self.sessions
            .lock()
            .await
            .insert(session.id.clone(), handle.clone());
        let actor = Actor {
            app: self.clone(),
            session,
            handle: handle.clone(),
            model_task: None,
            relay_attached: false,
            pending_speech: None,
            speech_recovery_task: None,
        };
        tokio::spawn(actor.run(rx));
        handle
    }
    pub async fn incoming(&self, sid: &str) -> anyhow::Result<(Handle, Value)> {
        let mut registry = self.sessions.lock().await;
        for h in registry.values() {
            let snap = h.send(Input::Snapshot).await?;
            if snap["inbound_sid"] == sid {
                return Ok((h.clone(), snap));
            }
        }
        anyhow::ensure!(registry.len() < 100, "Demo session capacity reached");
        let household = self
            .store
            .metadata("selected_household")
            .await?
            .unwrap_or_else(|| "sarah".into());
        let s = Session::new(
            uuid::Uuid::new_v4().to_string(),
            sid.into(),
            household,
            self.config.mode,
        );
        self.store.save(&s).await?;
        self.store
            .append(
                Some(&s),
                "caller",
                "call.inbound.received",
                json!({"caller_identity":"unverified","transport":self.config.transport}),
                s.execution_mode,
            )
            .await?;
        let value = serde_json::to_value(&s)?;
        let (tx, rx) = mpsc::channel(64);
        let (audio, _) = tokio::sync::broadcast::channel(16);
        let h = Handle {
            id: s.id.clone(),
            tx,
            audio,
        };
        registry.insert(s.id.clone(), h.clone());
        drop(registry);
        let actor = Actor {
            app: self.clone(),
            session: s,
            handle: h.clone(),
            model_task: None,
            relay_attached: false,
            pending_speech: None,
            speech_recovery_task: None,
        };
        tokio::spawn(actor.run(rx));
        let cleanup = h.clone();
        let seconds = self.config.max_call_seconds;
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(u64::from(seconds) + 30)).await;
            let _ = cleanup.send(Input::End).await;
        });
        Ok((h, value))
    }
    pub async fn handle(&self, id: &str) -> anyhow::Result<Handle> {
        self.sessions
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown session"))
    }
    pub async fn by_action(&self, action_id: &str) -> anyhow::Result<Handle> {
        for h in self.sessions.lock().await.values() {
            let s = h.send(Input::Snapshot).await?;
            if s["legs"]
                .as_array()
                .is_some_and(|legs| legs.iter().any(|l| l["action_id"] == action_id))
            {
                return Ok(h.clone());
            }
        }
        anyhow::bail!("Unknown action")
    }
    pub async fn by_offer(&self, offer_id: &str) -> anyhow::Result<Handle> {
        for h in self.sessions.lock().await.values() {
            let s = h.send(Input::Snapshot).await?;
            if s["offer"]["id"] == offer_id
                || s["offer_history"]
                    .as_array()
                    .is_some_and(|offers| offers.iter().any(|o| o["id"] == offer_id))
            {
                return Ok(h.clone());
            }
        }
        anyhow::bail!("Unknown offer")
    }
    pub async fn by_sid(&self, sid: &str) -> anyhow::Result<Handle> {
        for h in self.sessions.lock().await.values() {
            let s = h.send(Input::Snapshot).await?;
            if s["inbound_sid"] == sid {
                return Ok(h.clone());
            }
        }
        anyhow::bail!("Unknown inbound leg")
    }
}
pub(crate) struct Actor {
    pub(crate) app: App,
    pub(crate) session: Session,
    pub(crate) handle: Handle,
    pub(crate) model_task: Option<tokio::task::JoinHandle<()>>,
    pub(crate) relay_attached: bool,
    pub(crate) pending_speech: Option<PendingSpeechEpisode>,
    pub(crate) speech_recovery_task: Option<tokio::task::JoinHandle<()>>,
}
impl Actor {
    async fn run(mut self, mut rx: mpsc::Receiver<Envelope>) {
        while let Some(envelope) = rx.recv().await {
            let snapshot = matches!(&envelope.input, Input::Snapshot);
            let result = self.process(envelope.input).await;
            let result = match self.app.store.save(&self.session).await {
                Ok(()) => result,
                Err(_) => Err(anyhow::anyhow!("Durable session write failed")),
            };
            if !snapshot && result.is_ok() {
                let _ = self.emit("system", "session.updated", json!({})).await;
            }
            let _ = envelope.reply.send(result);
        }
    }
    pub(crate) async fn emit(&self, role: &str, kind: &str, payload: Value) -> anyhow::Result<()> {
        self.app.store.save(&self.session).await?;
        self.app
            .store
            .append(
                Some(&self.session),
                role,
                kind,
                payload,
                self.session.execution_mode,
            )
            .await?;
        Ok(())
    }
    async fn process(&mut self, input: Input) -> anyhow::Result<Value> {
        match input {
            Input::Snapshot => return Ok(serde_json::to_value(&self.session)?),
            Input::GatherClaim { nonce } => {
                if let Some(twiml) = self.session.gather_replies.get(&nonce) {
                    return Ok(json!({"fresh":false,"twiml":twiml}));
                }
                if let Some(next) = self.session.gather_claims.get(&nonce) {
                    return Ok(json!({"fresh":false,"next_token":next}));
                }
                let expected = if self.session.gather_token.is_empty() {
                    &self.session.relay_token
                } else {
                    &self.session.gather_token
                };
                anyhow::ensure!(
                    &nonce == expected
                        && !self.session.inbound_ended
                        && !self.session.bridge_committed
                        && self.session.closing_reason.is_none(),
                    "Invalid or obsolete Gather token"
                );
                let next = uuid::Uuid::new_v4().to_string();
                self.session.gather_token = next.clone();
                self.session.gather_claims.insert(nonce, next.clone());
                return Ok(json!({"fresh":true,"next_token":next}));
            }
            Input::GatherComplete { nonce, twiml } => {
                anyhow::ensure!(
                    self.session.gather_claims.contains_key(&nonce),
                    "Unknown Gather work"
                );
                self.session.gather_replies.entry(nonce).or_insert(twiml);
            }

            Input::Probe { contact_id } => {
                anyhow::ensure!(
                    contact_id == "sarah" && self.app.config.contacts.contains_key(&contact_id),
                    "Contact is not an enrolled smoke-test endpoint"
                );
                anyhow::ensure!(
                    self.session.execution_mode == ExecutionMode::Live,
                    "Replay cannot issue real calls"
                );
                anyhow::ensure!(
                    self.app.config.smoke_authorized,
                    "Smoke test not authorized in local configuration"
                );
                anyhow::ensure!(
                    self.app.config.outbound_configured(),
                    "Outbound Photon SIP setup pending"
                );
                let leg = Leg {
                    action_id: uuid::Uuid::new_v4().to_string(),
                    role: "sarah".into(),
                    kind: "probe".into(),
                    status: "reserved".into(),
                    acknowledged: false,
                    call_sid: None,
                };
                let key = format!("{}:probe", self.session.id);
                let (leg, fresh) = self
                    .app
                    .store
                    .reserve(&self.session.id, &key, &leg, self.session.execution_mode)
                    .await?;
                if fresh {
                    self.session.legs.push(leg.clone());
                    self.app.store.save(&self.session).await?;
                    self.emit(
                        "sarah",
                        "action.reserved",
                        json!({"action_id":leg.action_id,"kind":"probe"}),
                    )
                    .await?;
                    self.submit(leg.clone(), format!("/voice/alert/{}", leg.action_id));
                }
                return Ok(
                    json!({"action_id":leg.action_id,"status":leg.status,"duplicate":!fresh}),
                );
            }
            Input::ProviderCreated { action_id, outcome } => match outcome {
                CreateOutcome::Accepted { sid, status } => {
                    self.status(&action_id, &sid, &status).await?;
                }
                CreateOutcome::Rejected { status, code } => {
                    let l = self.leg_mut(&action_id)?;
                    if !terminal(&l.status) && l.call_sid.is_none() {
                        l.status = "failed".into();
                    }
                    self.emit(
                        "system",
                        "dependency.failed",
                        json!({"action_id":action_id,"provider":"twilio","http_status":status,"provider_error_code":code}),
                    )
                    .await?;
                }
                CreateOutcome::Unknown => {
                    let l = self.leg_mut(&action_id)?;
                    if l.status == "reserved" {
                        l.status = "unknown".into();
                    }
                    self.emit("system","outbound.unknown",json!({"action_id":action_id,"detail":"Create outcome unknown; awaiting signed callback. No retry."})).await?;
                }
            },
            Input::Status {
                action_id,
                sid,
                status,
            } => {
                self.status(&action_id, &sid, &status).await?;
            }
            Input::Alert { action_id, sid } => {
                if self.session.end_requested {
                    return Ok(json!({"twiml":twilio::say_end("This test has ended. Goodbye.")}));
                }
                self.status(&action_id, &sid, "in-progress").await?;
                let l = self.leg_mut(&action_id)?;
                anyhow::ensure!(!terminal(&l.status), "Leg is terminal");
                let text = if l.kind == "probe" {
                    "This is FirstRing's private fictional phone test for Sarah. Press 1 to acknowledge this test message."
                } else {
                    "This is FirstRing, an automated assistant. A caller made a request that needs independent verification. The caller is not connected to you. Press 1 to acknowledge this private alert."
                };
                return Ok(
                    json!({"twiml":twilio::gather(&self.app.config.external_url(&format!("/voice/ack/{action_id}")),text,8)}),
                );
            }
            Input::Ack {
                action_id,
                sid,
                digits,
            } => {
                let l = self.leg_mut(&action_id)?;
                anyhow::ensure!(
                    l.call_sid.as_deref() == Some(sid.as_str()),
                    "Wrong acknowledgement leg"
                );
                let acknowledged = l.acknowledge(&sid, &digits);
                let already_acknowledged = l.acknowledged;
                let role = l.role.clone();
                if acknowledged {
                    self.session.current_action = if role == "family" {
                        "Trusted contact acknowledged the alert".into()
                    } else {
                        "Phone B acknowledged the test".into()
                    };
                    self.emit(
                        &role,
                        "contact.acknowledged",
                        json!({"action_id":action_id}),
                    )
                    .await?;
                }
                return Ok(
                    json!({"twiml":twilio::say_end(if already_acknowledged {"Acknowledged. Thank you. Goodbye."} else {"No acknowledgement recorded. Goodbye."})}),
                );
            }
            Input::Keypad { sid, digits } => {
                anyhow::ensure!(sid == self.session.inbound_sid, "Wrong inbound leg");
                let key = if digits == "1" { "1" } else { "other_or_empty" };
                self.emit("caller", "transport.keypad", json!({"key":key}))
                    .await?;
                return Ok(
                    json!({"twiml":twilio::say_end(if digits=="1" {"FirstRing received your key. This fictional test is complete. Goodbye."}else{"No test acknowledgement received. Goodbye."})}),
                );
            }
            Input::InboundEnded => {
                self.clear_speech_recovery();
                if let Some(task) = self.model_task.take() {
                    task.abort();
                }
                self.session.inbound_ended = true;
                if let Some(offer) = self
                    .session
                    .offer
                    .as_ref()
                    .filter(|o| o.status == "pending")
                    .cloned()
                {
                    self.session.offer.as_mut().unwrap().status = "revoked".into();
                    self.session.policy_epoch += 1;
                    self.revoke_cleanup(&offer.id, &offer.leg_id).await?;
                }
                if self.session.bridge_committed {
                    for leg in &self.session.legs {
                        if leg.kind == "handoff"
                            && let Some(sid) = &leg.call_sid
                        {
                            self.end_leg(sid.clone(), "The other participant left. Goodbye.");
                        }
                    }
                    self.session.state = "ended".into();
                }
                if self.session.closing_reason.is_some() {
                    self.session.state = "ended".into();
                }
                self.session.current_action = if self
                    .session
                    .legs
                    .iter()
                    .any(|l| l.role == "family" && l.acknowledged)
                {
                    "Trusted contact acknowledged the alert".into()
                } else if self
                    .session
                    .legs
                    .iter()
                    .any(|l| l.kind == "probe" && l.acknowledged)
                {
                    "Phone B acknowledged the test".into()
                } else {
                    "Caller ended; child actions remain tracked".into()
                };
                self.emit("caller", "call.ended", json!({})).await?;
                self.start_postcall();
            }
            Input::End => {
                self.clear_speech_recovery();
                if self.session.inbound_ended
                    && self.session.legs.iter().all(|leg| terminal(&leg.status))
                {
                    return Ok(json!({"ignored":"already_terminal"}));
                }
                self.session.end_requested = true;
                self.session.inbound_ended = true;
                self.session.speech_pending = true;
                if let Some(task) = self.model_task.take() {
                    task.abort();
                }
                self.session.policy_epoch += 1;
                if let Some(offer) = self.session.offer.as_mut()
                    && offer.status == "pending"
                {
                    offer.status = "revoked".into();
                }
                self.session.current_action = "End requested; awaiting provider completion".into();
                if self.session.execution_mode == ExecutionMode::Live {
                    let mut sids = vec![self.session.inbound_sid.clone()];
                    sids.extend(
                        self.session
                            .legs
                            .iter()
                            .filter(|l| !terminal(&l.status))
                            .filter_map(|l| l.call_sid.clone()),
                    );
                    for sid in sids {
                        let provider = self.app.twilio.clone();
                        tokio::spawn(async move {
                            let _ = provider
                                .update(&sid, &[("Status", "completed".into())])
                                .await;
                        });
                    }
                }
                self.emit("system", "session.updated", json!({"end_requested":true}))
                    .await?;
            }
            other => return self.conversation(other).await,
        }
        Ok(json!({"ok":true}))
    }
    pub(crate) fn leg_mut(&mut self, id: &str) -> anyhow::Result<&mut Leg> {
        self.session
            .legs
            .iter_mut()
            .find(|l| l.action_id == id)
            .ok_or_else(|| anyhow::anyhow!("Unknown action"))
    }
    pub(crate) fn submit(&self, leg: Leg, path: String) {
        let provider = self.app.twilio.clone();
        let h = self.handle.clone();
        tokio::spawn(async move {
            let outcome = provider.create(&leg, &path).await;
            let _ = h
                .send(Input::ProviderCreated {
                    action_id: leg.action_id,
                    outcome,
                })
                .await;
        });
    }
    pub(crate) async fn status(&mut self, id: &str, sid: &str, status: &str) -> anyhow::Result<()> {
        let l = self.leg_mut(id)?;
        let changed = l.apply_status(sid, status)?;
        let role = l.role.clone();
        if changed {
            let kind = match status {
                "queued" | "initiated" => "outbound.requested",
                "ringing" => "outbound.ringing",
                "in-progress" => "outbound.answered",
                _ => "outbound.ended",
            };
            self.emit(&role, kind, json!({"action_id":id,"status":status}))
                .await?;
        }
        let cleanup = self.session.end_requested
            || self
                .session
                .cleanup_after
                .get(id)
                .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                .is_some_and(|v| chrono::Utc::now() >= v);
        if cleanup && !terminal(status) {
            self.end_leg(sid.to_string(), "This offer was withdrawn. Goodbye.");
        }
        Ok(())
    }
}
