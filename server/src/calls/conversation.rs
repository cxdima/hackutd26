use super::actor::{Actor, AudioFrame, Input};
use crate::{
    calls::state::{Leg, Turn, terminal},
    config::ExecutionMode,
    providers::{
        nemotron::{Nemotron, system_prompt},
        twilio,
    },
    tools::{ContactRole, QuestionPurpose, ToolCall},
};
use chrono::Utc;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

impl Actor {
    pub(crate) async fn conversation(&mut self, input: Input) -> anyhow::Result<Value> {
        match input {
            Input::PostCallTool { tool } => {
                anyhow::ensure!(
                    self.session.inbound_ended && self.session.blocked,
                    "No qualifying ended incident"
                );
                let ToolCall::PublishTactic {
                    tactic,
                    requested_action,
                    pretext_category,
                    pressure_category,
                } = ToolCall::parse(&tool.name, tool.arguments)?
                else {
                    anyhow::bail!("Post-call job only publishes a minimized tactic");
                };
                return crate::patterns::publish(
                    &self.app.store,
                    &self.session,
                    tactic,
                    &requested_action,
                    &pretext_category,
                    pressure_category,
                )
                .await;
            }
            Input::ConnectRelay { sid, token } => {
                anyhow::ensure!(
                    !self.relay_attached
                        && !self.session.inbound_ended
                        && sid == self.session.inbound_sid
                        && token == self.session.relay_token,
                    "Relay session binding rejected"
                );
                self.relay_attached = true;
                self.emit("caller", "voice.connected", json!({"transport":"relay"}))
                    .await?;
            }
            Input::Speech { partial } => {
                self.session.speech_started();
                self.cancel_model();
                if let Some(text) = partial {
                    self.emit("caller", "transcript.partial", json!({"text":text}))
                        .await?;
                }
            }
            Input::FinalPrompt { text } => {
                anyhow::ensure!(
                    !self.session.inbound_ended && !self.session.bridge_committed,
                    "Screening has ended"
                );
                anyhow::ensure!(
                    !text.trim().is_empty() && text.len() <= 4000 && self.session.turns.len() < 100,
                    "Invalid or excessive caller turn"
                );
                self.cancel_model();
                let previous = self
                    .session
                    .offer
                    .as_ref()
                    .filter(|o| o.status == "pending")
                    .cloned();
                let id = self.session.commit_turn(&text);
                self.emit(
                    "caller",
                    "transcript.final",
                    json!({"turn_id":id,"text":text}),
                )
                .await?;
                if let Some(offer) = previous {
                    self.revoke_cleanup(&offer.id, &offer.leg_id).await?;
                }
                self.app.store.save(&self.session).await?;
                if self.session.execution_mode == ExecutionMode::Live {
                    self.start_model();
                }
                return Ok(
                    json!({"turn_id":id,"generation":self.session.generation_id,"epoch":self.session.policy_epoch,"version":self.session.turn_version}),
                );
            }
            Input::ModelTool {
                generation,
                epoch,
                version,
                deadline,
                tool,
            } => {
                anyhow::ensure!(
                    Instant::now() <= deadline
                        && generation == self.session.generation_id
                        && epoch == self.session.policy_epoch
                        && version == self.session.turn_version
                        && !self.session.inbound_ended
                        && !self.session.bridge_committed,
                    "Obsolete model tool result"
                );
                self.emit("system", "agent.tool.proposed", json!({"name":tool.name}))
                    .await?;
                let parsed = ToolCall::parse(&tool.name, tool.arguments)?;
                let result = self.execute_tool(parsed).await;
                match result {
                    Ok(value) => {
                        return Ok(
                            json!({"ok":true,"result":value,"epoch":self.session.policy_epoch}),
                        );
                    }
                    Err(e) => {
                        let detail = e.to_string();
                        self.emit(
                            "system",
                            "policy.denied",
                            json!({"tool":tool.name,"detail":detail}),
                        )
                        .await?;
                        return Ok(
                            json!({"ok":false,"error":detail,"epoch":self.session.policy_epoch}),
                        );
                    }
                }
            }
            Input::ModelFailed { generation, detail } => {
                if generation == self.session.generation_id
                    && !self.session.bridge_committed
                    && !self.session.inbound_ended
                {
                    self.emit(
                        "system",
                        "dependency.failed",
                        json!({"provider":"nemotron","detail":detail}),
                    )
                    .await?;
                    self.session.current_action =
                        if self.session.legs.iter().any(|l| !terminal(&l.status)) {
                            "Connection pending; tracking the existing action".into()
                        } else {
                            "Conversation unavailable; message-taking fallback".into()
                        };
                    self.speak("I'm having a temporary problem. I won't connect this call. Please leave a brief message without any passwords or sign-in codes.").await?;
                }
            }
            Input::Whisper { offer_id, sid } => {
                let offer = self.find_offer(&offer_id)?.clone();
                self.status(&offer.leg_id, &sid, "in-progress").await?;
                if offer.status != "pending"
                    || offer.policy_epoch != self.session.policy_epoch
                    || self.session.end_requested
                    || Utc::now() >= chrono::DateTime::parse_from_rfc3339(&offer.expires_at)?
                {
                    return Ok(
                        json!({"twiml":twilio::say_end("This private offer was withdrawn. You will not be connected. Goodbye.")}),
                    );
                }
                let text = format!(
                    "FirstRing, your automated assistant. {} Caller identity is unverified. Press 1 to connect, or 2 to decline.",
                    offer.summary
                );
                return Ok(
                    json!({"twiml":twilio::gather(&self.app.config.external_url(&format!("/voice/choice/{offer_id}")),&text,20)}),
                );
            }
            Input::Choice {
                offer_id,
                sid,
                digits,
            } => {
                let offer = self.find_offer(&offer_id)?.clone();
                anyhow::ensure!(
                    self.session.legs.iter().any(|l| l.action_id == offer.leg_id
                        && l.call_sid.as_deref() == Some(sid.as_str())),
                    "Wrong private choice leg"
                );
                if digits == "1"
                    && offer.status == "accepted"
                    && self.session.bridge_committed
                    && !self.session.end_requested
                    && !self.session.inbound_ended
                    && matches!(
                        self.session.state.as_str(),
                        "bridge_committed" | "connected"
                    )
                {
                    let cb = self.app.config.external_url(&format!(
                        "/voice/conference-status?session_id={}",
                        self.session.id
                    ));
                    return Ok(
                        json!({"twiml":twilio::conference(&self.session.conference,&cb,self.app.config.max_call_seconds),"duplicate":true}),
                    );
                }
                self.emit("sarah","handoff.choice",json!({"offer_id":offer_id,"choice":if digits=="1"{"accept"}else{"decline_or_empty"}})).await?;
                if digits != "1" {
                    if let Some(o) = self
                        .session
                        .offer
                        .as_mut()
                        .filter(|o| o.id == offer_id && o.status == "pending")
                    {
                        o.status = "declined".into();
                    }
                    self.session.current_action = "Sarah declined or did not choose".into();
                    return Ok(json!({"twiml":twilio::say_end("No connection made. Goodbye.")}));
                }
                if self.session.accept(&offer_id, &sid, Utc::now()).is_err() {
                    self.emit(
                        "sarah",
                        "handoff.stale_rejected",
                        json!({"offer_id":offer_id}),
                    )
                    .await?;
                    return Ok(
                        json!({"twiml":twilio::say_end("That choice is no longer valid. The offer was withdrawn or is on hold. You will not be connected. Goodbye.")}),
                    );
                }
                self.cancel_model();
                self.app.store.save(&self.session).await?;
                // Return Sarah's conference TwiML in the choice response. Move A only after
                // the signed participant-join proves Sarah is in the room. This avoids racing
                // a Calls API update against this very webhook's TwiML response.
                let cb = self.app.config.external_url(&format!(
                    "/voice/conference-status?session_id={}",
                    self.session.id
                ));
                let room = twilio::conference(
                    &self.session.conference,
                    &cb,
                    self.app.config.max_call_seconds,
                );
                let h = self.handle.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    let _ = h.send(Input::BridgeTimeout).await;
                });
                return Ok(json!({"twiml":room}));
            }
            Input::Conference { name, sid, event } => {
                anyhow::ensure!(name == self.session.conference, "Wrong conference");
                if event == "participant-join" && self.session.conference_join(&sid) {
                    self.emit(
                        "system",
                        "bridge.connected",
                        json!({"participants":2,"screening":"ended"}),
                    )
                    .await?;
                }
                let private = self
                    .session
                    .offer
                    .as_ref()
                    .and_then(|o| self.session.legs.iter().find(|l| l.action_id == o.leg_id))
                    .and_then(|l| l.call_sid.clone());
                if event == "participant-join"
                    && self.session.bridge_committed
                    && self.session.state == "bridge_committed"
                    && !self.session.end_requested
                    && !self.session.inbound_ended
                    && private.as_deref() == Some(&sid)
                    && !self.session.bridge_a_requested
                {
                    self.session.bridge_a_requested = true;
                    self.app.store.save(&self.session).await?;
                    if self.session.execution_mode == ExecutionMode::Live {
                        let provider = self.app.twilio.clone();
                        let h = self.handle.clone();
                        let caller = self.session.inbound_sid.clone();
                        let cb = self.app.config.external_url(&format!(
                            "/voice/conference-status?session_id={}",
                            self.session.id
                        ));
                        let room = twilio::conference(
                            &self.session.conference,
                            &cb,
                            self.app.config.max_call_seconds,
                        );
                        let sid = sid.clone();
                        tokio::spawn(async move {
                            let ok = provider.update(&caller, &[("Twiml", room)]).await.is_ok();
                            if !ok {
                                let _ = provider
                                    .update(&sid, &[("Status", "completed".into())])
                                    .await;
                                let _ = provider
                                    .update(&caller, &[("Status", "completed".into())])
                                    .await;
                            }
                            let _ = h.send(Input::BridgeResult { ok }).await;
                        });
                    }
                }
                if event == "participant-leave" && self.session.joined.iter().any(|s| s == &sid) {
                    self.session.current_action = "Human call ended".into();
                    self.session.state = "ended".into();
                    for other in self
                        .session
                        .joined
                        .clone()
                        .into_iter()
                        .filter(|s| s != &sid)
                    {
                        self.end_leg(other, "The other participant left. Goodbye.");
                    }
                }
            }
            Input::Cleanup { action_id } => {
                if let Some(sid) = self
                    .session
                    .legs
                    .iter()
                    .find(|l| l.action_id == action_id && !terminal(&l.status))
                    .and_then(|l| l.call_sid.clone())
                {
                    self.end_leg(
                        sid,
                        "This private offer was withdrawn. You will not be connected. Goodbye.",
                    );
                }
            }
            Input::BridgeTimeout => {
                if self.session.bridge_committed && self.session.state != "connected" {
                    self.session.state = "bridge_failed".into();
                    self.session.current_action = "Bridge timed out; ending both legs".into();
                    self.end_leg(
                        self.session.inbound_sid.clone(),
                        "Connection could not be completed. Goodbye.",
                    );
                    for l in &self.session.legs {
                        if l.kind == "handoff"
                            && let Some(sid) = &l.call_sid
                        {
                            self.end_leg(
                                sid.clone(),
                                "Connection could not be completed. Goodbye.",
                            );
                        }
                    }
                    self.emit(
                        "system",
                        "dependency.failed",
                        json!({"operation":"bridge_timeout"}),
                    )
                    .await?;
                }
            }
            Input::BridgeResult { ok } => {
                if !ok {
                    self.session.state = "bridge_failed".into();
                    self.session.current_action =
                        "Connection failed; ending the incomplete bridge".into();
                    self.emit(
                        "system",
                        "dependency.failed",
                        json!({"provider":"twilio","operation":"bridge"}),
                    )
                    .await?;
                }
            }
            Input::RelayDisconnected => {
                self.relay_attached = false;
                self.cancel_model();
                if !self.session.bridge_committed {
                    self.session.speech_started();
                    let old = self
                        .session
                        .offer
                        .as_ref()
                        .filter(|o| o.status == "pending")
                        .cloned();
                    if let Some(o) = old {
                        self.session.offer.as_mut().unwrap().status = "revoked".into();
                        self.session.policy_epoch += 1;
                        self.revoke_cleanup(&o.id, &o.leg_id).await?;
                    }
                    self.emit(
                        "system",
                        "dependency.failed",
                        json!({"provider":"relay","detail":"Socket disconnected; consent held"}),
                    )
                    .await?;
                }
            }
            _ => anyhow::bail!("Unsupported conversation command"),
        }
        Ok(json!({"ok":true}))
    }
    fn find_offer(&self, id: &str) -> anyhow::Result<&crate::calls::state::Offer> {
        self.session
            .offer
            .as_ref()
            .filter(|o| o.id == id)
            .or_else(|| self.session.offer_history.iter().find(|o| o.id == id))
            .ok_or_else(|| anyhow::anyhow!("Unknown private offer"))
    }
    pub(crate) fn cancel_model(&mut self) {
        if let Some(task) = self.model_task.take() {
            task.abort();
        }
    }
    pub(crate) fn end_leg(&self, sid: String, message: &str) {
        if self.session.execution_mode == ExecutionMode::Live {
            let provider = self.app.twilio.clone();
            let xml = twilio::say_end(message);
            tokio::spawn(async move {
                let _ = provider.update(&sid, &[("Twiml", xml)]).await;
            });
        }
    }
    pub(crate) async fn revoke_cleanup(
        &mut self,
        offer_id: &str,
        leg_id: &str,
    ) -> anyhow::Result<()> {
        self.session.cleanup_after.insert(
            leg_id.into(),
            (Utc::now() + chrono::Duration::seconds(5)).to_rfc3339(),
        );
        self.emit(
            "sarah",
            "handoff.revoked",
            json!({"offer_id":offer_id,"stale_key_window_seconds":5}),
        )
        .await?;
        let h = self.handle.clone();
        let action_id = leg_id.into();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let _ = h.send(Input::Cleanup { action_id }).await;
        });
        Ok(())
    }
    async fn speak(&mut self, text: &str) -> anyhow::Result<()> {
        self.session.turns.push(Turn {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.into(),
            speaker: "assistant".into(),
        });
        self.emit(
            "system",
            "audio.requested",
            json!({"text":text,"voice":"preset","heard":false}),
        )
        .await?;
        let _=self.handle.audio.send(AudioFrame{generation:self.session.generation_id,epoch:self.session.policy_epoch,payload:json!({"type":"text","token":text,"last":true,"interruptible":true,"preemptible":true})});
        Ok(())
    }
    pub(crate) fn start_postcall(&self) {
        if !self.session.blocked
            || !self.session.inbound_ended
            || self.session.execution_mode != ExecutionMode::Live
            || self.app.config.nvidia_key.is_empty()
        {
            return;
        }
        let h = self.handle.clone();
        let c = self.app.config.clone();
        let turns = self.session.turns.clone();
        tokio::spawn(async move {
            let model = Nemotron::new(c.nvidia_key.clone(), c.model_url.clone(), c.model.clone());
            let messages = vec![
                json!({"role":"system","content":system_prompt()}),
                json!({"role":"user","content":json!({"post_call_job":true,"instruction":"The blocked incident has ended. Propose only publish_tactic using categorical fields. No private content or transcript in tool arguments.","turns":turns}).to_string()}),
            ];
            if let Ok(tool) = model.next_tool(&messages).await {
                let _ = h.send(Input::PostCallTool { tool }).await;
            }
        });
    }
    fn start_model(&mut self) {
        let s = self.session.clone();
        let h = self.handle.clone();
        let c = self.app.config.clone();
        self.model_task = Some(tokio::spawn(async move {
            let deadline = Instant::now() + Duration::from_secs(6);
            let future = async {
                let model =
                    Nemotron::new(c.nvidia_key.clone(), c.model_url.clone(), c.model.clone());
                let mut messages = vec![
                    json!({"role":"system","content":system_prompt()}),
                    json!({"role":"user","content":serde_json::to_string(&json!({"household":s.household_id,"turns":s.turns,"instruction":"Assess the latest caller turn, then choose a useful next tool."}))?}),
                ];
                let mut epoch = s.policy_epoch;
                for _ in 0..4 {
                    let tool = model.next_tool(&messages).await?;
                    let assistant = tool.assistant_message.clone();
                    let id = tool.id.clone();
                    let outcome = h
                        .send(Input::ModelTool {
                            generation: s.generation_id,
                            epoch,
                            version: s.turn_version,
                            deadline,
                            tool,
                        })
                        .await?;
                    epoch = outcome["epoch"].as_u64().unwrap_or(epoch);
                    messages.push(assistant);
                    messages.push(
                        json!({"role":"tool","tool_call_id":id,"content":outcome.to_string()}),
                    );
                    if outcome["result"]["stop"] == true {
                        return Ok::<_, anyhow::Error>(());
                    }
                }
                anyhow::bail!("Turn tool-step budget exhausted")
            };
            let result =
                tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), future).await;
            let detail = match result {
                Ok(Ok(())) => return,
                Ok(Err(e)) => e.to_string(),
                Err(_) => "Total turn deadline reached".into(),
            };
            let _ = h
                .send(Input::ModelFailed {
                    generation: s.generation_id,
                    detail,
                })
                .await;
        }));
    }
    async fn execute_tool(&mut self, tool: ToolCall) -> anyhow::Result<Value> {
        match tool {
            ToolCall::RecordAssessment {
                observation,
                evidence_refs,
            } => {
                self.session.assess(
                    self.session.generation_id,
                    self.session.turn_version,
                    &observation,
                    &evidence_refs,
                )?;
                self.emit("system",if self.session.blocked{"policy.denied"}else{"policy.allowed"},json!({"operation":"assessment","observation":observation,"quote":self.session.evidence_quote,"handoff_allowed":!self.session.blocked && !self.session.ambiguous})).await?;
                Ok(
                    json!({"blocked":self.session.blocked,"ambiguous":self.session.ambiguous,"assessed_version":self.session.assessed_version}),
                )
            }
            ToolCall::AskCaller {
                question,
                purpose,
                evidence_refs,
            } => {
                anyhow::ensure!(
                    self.session.assessed_version == self.session.turn_version,
                    "Assess the current turn before asking a follow-up"
                );
                self.session.evidence_valid(&evidence_refs, true)?;
                let question = if purpose == QuestionPurpose::ClarifyCodeType {
                    "Is that an appointment reference, or a code used to sign in? Please don't read the code itself.".to_string()
                } else {
                    question
                };
                let lower = question.to_lowercase();
                anyhow::ensure!(
                    !(lower.contains("password")
                        || lower.contains("digits")
                        || lower.contains("verification code"))
                        || purpose == QuestionPurpose::ClarifyCodeType,
                    "Question could solicit a secret"
                );
                self.session.current_action = "Waiting for the caller’s answer".into();
                self.speak(&question).await?;
                Ok(json!({"audio_requested":true,"stop":true}))
            }
            ToolCall::LookupTrustedContact { role } => {
                let role = match role {
                    ContactRole::Family => "family",
                    ContactRole::Clinic => "clinic",
                };
                anyhow::ensure!(
                    self.session.execution_mode == ExecutionMode::Replay
                        || self.app.config.contacts.contains_key(role),
                    "Requested role is not enrolled"
                );
                Ok(
                    json!({"contact_id":format!("{}:{role}",self.session.household_id),"label":role,"provenance":"server_enrolled_directory"}),
                )
            }
            ToolCall::LookupTactics {
                requested_action,
                pretext_category,
            } => {
                let matches = crate::patterns::lookup(
                    &self.app.store,
                    &self.session.household_id,
                    self.session.execution_mode,
                    &requested_action,
                    &pretext_category,
                )
                .await?;
                if !matches.is_empty() {
                    self.emit("system","tactic.matched",json!({"matches":matches,"matched_fields":["requested_action","pretext_category"]})).await?;
                }
                Ok(json!({"matches":matches}))
            }
            ToolCall::OfferHandoff {
                summary,
                observation,
                evidence_refs,
            } => {
                self.session.evidence_valid(&evidence_refs, true)?;
                let current = self
                    .session
                    .assessments
                    .last()
                    .and_then(|v| v.get("observation"));
                anyhow::ensure!(
                    current == Some(&serde_json::to_value(&observation)?),
                    "Offer observation must match current assessment"
                );
                anyhow::ensure!(
                    self.session.execution_mode == ExecutionMode::Replay
                        || self.app.config.handoff_enabled,
                    "Private handoff calls are not authorized yet"
                );
                let leg = Leg {
                    action_id: uuid::Uuid::new_v4().to_string(),
                    role: "sarah".into(),
                    kind: "handoff".into(),
                    status: "reserved".into(),
                    acknowledged: false,
                    call_sid: None,
                };
                self.session.legs.push(leg.clone());
                let offer = match self
                    .session
                    .make_offer(&summary, &leg.action_id, Utc::now())
                {
                    Ok(o) => o,
                    Err(e) => {
                        self.session.legs.pop();
                        return Err(e);
                    }
                };
                let key = format!("{}:handoff:{}", self.session.id, self.session.policy_epoch);
                let reserve = self
                    .app
                    .store
                    .reserve(&self.session.id, &key, &leg, self.session.execution_mode)
                    .await;
                if !matches!(&reserve, Ok((_, true))) {
                    self.session.legs.retain(|l| l.action_id != leg.action_id);
                    self.session.offer = self.session.offer_history.pop();
                    self.session.current_action =
                        "Handoff attempt already reserved or unavailable".into();
                    anyhow::bail!("This handoff could not reserve a fresh action");
                }
                self.app.store.save(&self.session).await?;
                self.emit(
                    "sarah",
                    "handoff.offered",
                    json!({"offer_id":offer.id,"summary":summary,"expires_at":offer.expires_at}),
                )
                .await?;
                if self.session.execution_mode == ExecutionMode::Live {
                    self.submit(leg, format!("/voice/whisper/{}", offer.id));
                }
                self.speak("I'm asking privately whether this call can be connected. Please stay on the line.").await?;
                Ok(json!({"offer_id":offer.id,"state":"pending","stop":true}))
            }
            ToolCall::StartTrustedCallback {
                contact_id,
                reason,
                evidence_refs,
            } => {
                if reason == crate::tools::CallbackReason::AuthenticationRequest {
                    anyhow::ensure!(
                        self.session.blocked
                            && self.session.assessments.iter().any(
                                |a| a["observation"]["requested_object"] == "authentication_secret"
                                    && a["observation"]["speech_act"] == "request"
                                    && a["observation"]["direction"]
                                        == "caller_requests_from_sarah"
                            ),
                        "Authentication alert needs an assessed extraction request"
                    );
                }
                self.session.evidence_valid(&evidence_refs, true)?;
                anyhow::ensure!(
                    self.session.assessed_version == self.session.turn_version
                        && !self.session.speech_pending,
                    "Current turn is not assessed"
                );
                anyhow::ensure!(
                    contact_id == format!("{}:family", self.session.household_id),
                    "Contact is not in this household's enabled directory"
                );
                anyhow::ensure!(
                    self.session.execution_mode == ExecutionMode::Replay
                        || self.app.config.family_enabled,
                    "Family calling is disabled; no authorized endpoint"
                );
                if let Some(o) = self
                    .session
                    .offer
                    .as_mut()
                    .filter(|o| o.status == "pending")
                {
                    o.status = "revoked".into();
                    self.session.policy_epoch += 1;
                    let id = o.id.clone();
                    let leg = o.leg_id.clone();
                    self.revoke_cleanup(&id, &leg).await?;
                }
                let leg = Leg {
                    action_id: uuid::Uuid::new_v4().to_string(),
                    role: "family".into(),
                    kind: "trusted".into(),
                    status: "reserved".into(),
                    acknowledged: false,
                    call_sid: None,
                };
                let key = format!("{}:trusted:family", self.session.id);
                let (leg, fresh) = self
                    .app
                    .store
                    .reserve(&self.session.id, &key, &leg, self.session.execution_mode)
                    .await?;
                if fresh {
                    self.session.legs.push(leg.clone());
                    self.app.store.save(&self.session).await?;
                    self.emit(
                        "family",
                        "action.reserved",
                        json!({"action_id":leg.action_id}),
                    )
                    .await?;
                    if self.session.execution_mode == ExecutionMode::Live {
                        self.submit(leg.clone(), format!("/voice/alert/{}", leg.action_id));
                    }
                }
                self.session.current_action = "Independent contact requested".into();
                self.speak("I won't connect this call. An independently enrolled trusted contact is being requested.").await?;
                self.end_leg(self.session.inbound_sid.clone(),"I cannot help obtain an authentication secret. This call will not be connected. Goodbye.");
                Ok(
                    json!({"action_id":leg.action_id,"state":leg.status,"acknowledged":leg.acknowledged,"stop":true}),
                )
            }
            ToolCall::TakeMessage {
                summary: _,
                evidence_refs,
            } => {
                anyhow::ensure!(
                    self.session.assessed_version == self.session.turn_version,
                    "Assess the current turn before completing a message"
                );
                self.session.evidence_valid(&evidence_refs, true)?;
                self.session.current_action = "Message received; caller identity unverified".into();
                self.speak("Your message has been noted. No connection has been made. Goodbye.")
                    .await?;
                Ok(json!({"message_recorded":true,"stop":true}))
            }
            ToolCall::PublishTactic { .. } => {
                anyhow::bail!("Publication runs only in a post-call job")
            }
        }
    }
}
