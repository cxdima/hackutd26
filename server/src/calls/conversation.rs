use super::actor::{Actor, AudioFrame, AudioPurpose, Input, PendingSpeechEpisode};
use crate::{
    calls::state::{Leg, Turn, terminal},
    config::ExecutionMode,
    providers::{
        nemotron::{
            ModelCapabilities, Nemotron, ToolPhase, append_tool_result, conversation_messages,
            next_step_phase, system_prompt,
        },
        twilio,
    },
    tools::{ContactRole, QuestionPurpose, ToolCall},
};
use chrono::Utc;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const MODEL_FAILURE_CLOSING: &str =
    "I can't help with that request. This call won't be connected. Goodbye.";

impl Actor {
    pub(crate) async fn conversation(&mut self, input: Input) -> anyhow::Result<Value> {
        let input = match input {
            Input::ClosingResponseForRelay { token } => {
                anyhow::ensure!(
                    token == self.session.relay_token,
                    "Obsolete Relay closing callback"
                );
                Input::ClosingResponse
            }
            Input::RelaySpeech { token, text, last } => {
                anyhow::ensure!(
                    self.active_relay_token.as_deref() == Some(token.as_str())
                        && token == self.session.relay_token,
                    "Obsolete Relay input"
                );
                if last {
                    Input::FinalPrompt {
                        text: text.unwrap_or_default(),
                    }
                } else {
                    Input::Speech { partial: text }
                }
            }
            other => other,
        };
        match input {
            Input::RelayTelemetry {
                event,
                elapsed_ms,
                characters,
            } => {
                tracing::info!(session_id = %self.session.id, event, elapsed_ms, characters, "Relay transport observation");
                self.emit("caller", "relay.telemetry", json!({"event":event,"elapsed_ms":elapsed_ms,"characters":characters,"heard":false})).await?;
            }
            Input::ClosingResponse => {
                let text = match self.session.closing_reason.as_deref() {
                    Some("message_saved") => self.saved_message_closing(),
                    Some("model_failure") => MODEL_FAILURE_CLOSING,
                    Some("no_message") => "Thanks for calling. Goodbye.",
                    _ => anyhow::bail!("No committed closing response"),
                };
                if self.session.inbound_ended || self.session.end_requested {
                    return Ok(json!({"twiml":"<Response><Hangup/></Response>"}));
                }
                let nonce = self.closing_nonce();
                let url = self
                    .app
                    .config
                    .external_url(&format!("/voice/closing?nonce={nonce}"));
                let twiml = format!(
                    "<Response><Gather input=\"speech\" speechTimeout=\"1\" timeout=\"2\" actionOnEmptyResult=\"true\" action=\"{}\" method=\"POST\"><Say voice=\"Polly.Joanna-Neural\" language=\"en-US\">{}</Say></Gather><Hangup/></Response>",
                    twilio::escape(&url),
                    twilio::escape(text)
                );
                return Ok(json!({"twiml":twiml}));
            }
            Input::ClosingInput { nonce, text } => return self.closing_input(nonce, text).await,
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
                        && !self.session.end_requested
                        && self.session.closing_reason.is_none()
                        && sid == self.session.inbound_sid
                        && token == self.session.relay_token,
                    "Relay session binding rejected"
                );
                self.relay_attached = true;
                self.active_relay_token = Some(token);
                self.emit("caller", "voice.connected", json!({"transport":"relay"}))
                    .await?;
                if let Some(text) = self.pending_relay_prompt.take() {
                    return self.final_prompt(text).await;
                }
            }
            Input::SpeechRecovery { generation, epoch } => {
                return self.recover_pending_speech(generation, epoch).await;
            }
            Input::Speech { partial } => {
                if self.session.closing_reason.is_some()
                    || self.session.inbound_ended
                    || self.session.bridge_committed
                {
                    return Ok(json!({"ignored":"closing"}));
                }
                let old_generation = self.session.generation_id;
                let model_inflight = self
                    .model_task
                    .as_ref()
                    .is_some_and(|task| !task.is_finished());
                self.session.speech_started();
                self.cancel_model();
                if model_inflight {
                    self.emit(
                        "system",
                        "model.cancelled",
                        json!({"generation_id":old_generation,"reason":"caller_speech"}),
                    )
                    .await?;
                }
                if let Some(text) = partial {
                    self.emit("caller", "transcript.partial", json!({"text":text}))
                        .await?;
                }
                self.arm_speech_recovery().await?;
            }
            Input::FinalPrompt { text } => return self.final_prompt(text).await,
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
                        && self.session.closing_reason.is_none()
                        && !self.session.bridge_committed,
                    "Obsolete model tool result"
                );
                let parsed = ToolCall::parse(&tool.name, tool.arguments)?;
                anyhow::ensure!(
                    !matches!(
                        &parsed,
                        ToolCall::RecordAssessment {
                            understanding: None,
                            ..
                        }
                    ),
                    "Native assessment requires current call understanding"
                );
                self.emit("system", "agent.tool.proposed", json!({"name":tool.name,"generation_id":generation,"turn_version":version,"arguments":crate::tools::public_arguments(&parsed)})).await?;
                let result = self.execute_tool(parsed).await;
                match result {
                    Ok(value) => {
                        self.emit("system", "agent.tool.result", json!({"name":tool.name,"generation_id":generation,"turn_version":version,"ok":true,"result":crate::tools::public_result(&value)})).await?;
                        return Ok(
                            json!({"ok":true,"result":value,"epoch":self.session.policy_epoch,"next_phase":next_step_phase(&self.session)}),
                        );
                    }
                    Err(e) => {
                        let detail = e.to_string();
                        self.emit("system", "agent.tool.result", json!({"name":tool.name,"generation_id":generation,"turn_version":version,"ok":false,"result":{"completed":false,"check_action_receipts":true}})).await?;
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
                    && !self.session.end_requested
                    && self.session.closing_reason.is_none()
                {
                    self.emit(
                        "system",
                        "dependency.failed",
                        json!({"provider":"nemotron","detail":detail}),
                    )
                    .await?;
                    if self.session.blocked
                        && self.session.turn_version > 0
                        && self.session.assessed_version == self.session.turn_version
                        && !self.session.speech_pending
                    {
                        // A committed policy assessment already rejected the request.
                        // Do not invent a model action/message when its next request fails.
                        let mut candidate = self.session.clone();
                        candidate.closing_reason = Some("model_failure".into());
                        candidate.state = "closing".into();
                        candidate.current_action =
                            "Risky request blocked; closing after model failure".into();
                        candidate.speech_pending = true;
                        candidate.generation_id += 1;
                        self.app.store.save(&candidate).await?;
                        self.session = candidate;
                        self.clear_speech_recovery();
                        self.cancel_model();
                        self.emit("system", "audio.requested", json!({"text":MODEL_FAILURE_CLOSING,"voice":"preset","transport":"twiml_closing","heard":false})).await?;
                        let _ = self.handle.audio.send(AudioFrame {
                            generation: self.session.generation_id,
                            epoch: self.session.policy_epoch,
                            payload: crate::providers::relay::end("model_failure"),
                            purpose: AudioPurpose::ModelFailureClosing,
                        });
                        return Ok(json!({"closing":true,"reason":"model_failure"}));
                    }
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
                self.clear_speech_recovery();
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
            Input::RelayDisconnected { token } => {
                if self.active_relay_token.as_deref() != Some(token.as_str())
                    || token != self.session.relay_token
                {
                    return Ok(json!({"ignored":"obsolete_relay_disconnect"}));
                }
                self.clear_speech_recovery();
                self.relay_attached = false;
                self.active_relay_token = None;
                self.cancel_model();
                if !self.session.bridge_committed && self.session.closing_reason.is_none() {
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
    fn closing_nonce(&self) -> String {
        format!(
            "{}.{}",
            self.session.relay_token, self.session.generation_id
        )
    }

    async fn closing_input(&mut self, nonce: String, text: String) -> anyhow::Result<Value> {
        if self.session.inbound_ended || self.session.end_requested {
            return Ok(json!({"twiml":"<Response><Hangup/></Response>"}));
        }
        let gather = self.app.config.transport == "gather";
        if gather && let Some(token) = self.session.gather_claims.get(&nonce) {
            anyhow::ensure!(
                token == &self.session.relay_token && self.session.closing_reason.is_none(),
                "Obsolete closing callback"
            );
            // The existing GatherClaim/GatherComplete path owns this turn's
            // response cache, including retries while its model work is pending.
            return Ok(json!({"gather_turn_token":token,"duplicate":true}));
        }
        if let Some(cached) = self.session.gather_replies.get(&nonce) {
            anyhow::ensure!(
                self.session.gather_claims.get(&nonce) == Some(&self.session.relay_token)
                    && self.session.closing_reason.is_none(),
                "Obsolete closing callback"
            );
            return Ok(json!({"twiml":cached,"duplicate":true}));
        }
        anyhow::ensure!(
            nonce == self.closing_nonce()
                && matches!(
                    self.session.closing_reason.as_deref(),
                    Some("message_saved" | "model_failure" | "no_message")
                )
                && !self.session.bridge_committed,
            "Invalid or obsolete closing token"
        );
        anyhow::ensure!(
            text.len() <= 4000 && self.session.turns.len() < 100,
            "Invalid or excessive closing speech"
        );
        self.clear_speech_recovery();
        self.cancel_model();
        if text.trim().is_empty() {
            self.session.end_requested = true;
            self.session.speech_pending = true;
            self.session.current_action =
                "Acknowledgement finished; awaiting call completion".into();
            self.emit(
                "system",
                "closing.completed",
                json!({"speech_received":false,"provider_completion_pending":true}),
            )
            .await?;
            return Ok(json!({"twiml":"<Response><Hangup/></Response>"}));
        }
        let mut candidate = self.session.clone();
        candidate.closing_reason = None;
        candidate.state = "screening".into();
        candidate.current_action = "Caller added information; reconnecting the assistant".into();
        candidate.speech_started();
        candidate.relay_token = uuid::Uuid::new_v4().to_string();
        let response = if gather {
            candidate.gather_token = candidate.relay_token.clone();
            json!({"gather_turn_token":candidate.gather_token})
        } else {
            let twiml = crate::providers::relay::reconnect_twiml(
                &self.app.config.public_base_url,
                &candidate.relay_token,
            );
            candidate
                .gather_replies
                .insert(nonce.clone(), twiml.clone());
            json!({"twiml":twiml})
        };
        candidate
            .gather_claims
            .insert(nonce.clone(), candidate.relay_token.clone());
        self.app.store.save(&candidate).await?;
        self.session = candidate;
        self.relay_attached = false;
        self.active_relay_token = None;
        self.pending_relay_prompt = if gather { None } else { Some(text) };
        self.emit("caller", "closing.interrupted", json!({"speech_received":true,"message_preserved":self.session.message.is_some(),"reconnect_pending":!gather,"transport":self.app.config.transport})).await?;
        Ok(response)
    }

    async fn final_prompt(&mut self, text: String) -> anyhow::Result<Value> {
        anyhow::ensure!(
            !self.session.inbound_ended
                && !self.session.end_requested
                && !self.session.bridge_committed
                && self.session.closing_reason.is_none(),
            "Screening has ended"
        );
        anyhow::ensure!(
            !text.trim().is_empty() && text.len() <= 4000 && self.session.turns.len() < 100,
            "Invalid or excessive caller turn"
        );
        self.clear_speech_recovery();
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
        Ok(
            json!({"turn_id":id,"generation":self.session.generation_id,"epoch":self.session.policy_epoch,"version":self.session.turn_version}),
        )
    }
    fn find_offer(&self, id: &str) -> anyhow::Result<&crate::calls::state::Offer> {
        self.session
            .offer
            .as_ref()
            .filter(|o| o.id == id)
            .or_else(|| self.session.offer_history.iter().find(|o| o.id == id))
            .ok_or_else(|| anyhow::anyhow!("Unknown private offer"))
    }
    pub(crate) fn clear_speech_recovery(&mut self) {
        if let Some(task) = self.speech_recovery_task.take() {
            task.abort();
        }
        self.pending_speech = None;
    }
    async fn arm_speech_recovery(&mut self) -> anyhow::Result<()> {
        if !self.relay_attached || !self.session.speech_pending {
            return Ok(());
        }
        if let Some(task) = self.speech_recovery_task.take() {
            task.abort();
        }
        let now = Instant::now();
        // A prompt alone never starts another timer. New documented caller input
        // after a recovery prompt begins a fresh, independently bounded episode.
        let first = self
            .pending_speech
            .as_ref()
            .filter(|pending| !pending.prompted)
            .map_or(now, |pending| pending.first_started);
        let deadline = (now + Duration::from_secs(4)).min(first + Duration::from_secs(8));
        let generation = self.session.generation_id;
        let epoch = self.session.policy_epoch;
        self.pending_speech = Some(PendingSpeechEpisode {
            first_started: first,
            deadline,
            generation,
            epoch,
            prompted: false,
        });
        if now >= deadline {
            self.recover_pending_speech(generation, epoch).await?;
        } else {
            let handle = self.handle.clone();
            self.speech_recovery_task = Some(tokio::spawn(async move {
                tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
                let _ = handle
                    .send(Input::SpeechRecovery { generation, epoch })
                    .await;
            }));
        }
        Ok(())
    }
    async fn recover_pending_speech(
        &mut self,
        generation: u64,
        epoch: u64,
    ) -> anyhow::Result<Value> {
        let valid = self.pending_speech.as_ref().is_some_and(|pending| {
            !pending.prompted
                && pending.generation == generation
                && pending.epoch == epoch
                && Instant::now() >= pending.deadline
        }) && self.relay_attached
            && self.session.speech_pending
            && !self.session.inbound_ended
            && !self.session.end_requested
            && !self.session.bridge_committed
            && self.session.closing_reason.is_none()
            && self.session.generation_id == generation
            && self.session.policy_epoch == epoch;
        if !valid {
            return Ok(json!({"ignored":"obsolete_or_early_recovery"}));
        }
        self.pending_speech
            .as_mut()
            .expect("validated pending episode")
            .prompted = true;
        self.speech_recovery_task.take();
        self.session.current_action =
            "Please repeat the request; private acceptance remains on hold".into();
        let text = "I didn't catch the end of that. Please repeat your request without sharing passwords or sign-in codes.";
        self.emit("system","speech.recovery.requested",json!({"generation_id":generation,"turn_version":self.session.turn_version,"consent_held":true,"committed_partial":false})).await?;
        self.emit(
            "system",
            "audio.requested",
            json!({"text":text,"voice":"preset","purpose":"pending_speech_recovery","heard":false}),
        )
        .await?;
        let _=self.handle.audio.send(AudioFrame{generation,epoch,purpose:AudioPurpose::PendingSpeechRecovery,payload:json!({"type":"text","token":text,"last":true,"interruptible":true,"preemptible":true})});
        Ok(json!({"recovery_requested":true,"consent_held":true}))
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
        let _=self.handle.audio.send(AudioFrame{generation:self.session.generation_id,epoch:self.session.policy_epoch,purpose:AudioPurpose::Response,payload:json!({"type":"text","token":text,"last":true,"interruptible":true,"preemptible":true})});
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
            let model = Nemotron::new(c.nvidia_key.clone(), c.model_url.clone(), c.model.clone())
                .with_backend(c.model_backend);
            let messages = vec![
                json!({"role":"system","content":system_prompt()}),
                json!({"role":"user","content":json!({"post_call_job":true,"instruction":"The blocked incident has ended. Propose only publish_tactic using categorical fields. No private content or transcript in tool arguments.","turns":turns}).to_string()}),
            ];
            if let Ok(tool) = model.next_tool_for(&messages, ToolPhase::PostCall).await {
                let _ = h.send(Input::PostCallTool { tool }).await;
            }
        });
    }
    fn start_model(&mut self) {
        let s = self.session.clone();
        let h = self.handle.clone();
        let c = self.app.config.clone();
        let store = self.app.store.clone();
        self.model_task = Some(tokio::spawn(async move {
            let deadline = Instant::now() + Duration::from_secs(6);
            let future = async {
                let model =
                    Nemotron::new(c.nvidia_key.clone(), c.model_url.clone(), c.model.clone())
                        .with_backend(c.model_backend);
                let mut messages = conversation_messages(&s, &ModelCapabilities::from_config(&c));
                let mut epoch = s.policy_epoch;
                let mut phase = ToolPhase::Assessment;
                for _ in 0..4 {
                    let observed = model.next_tool_observed_for(&messages, phase).await;
                    // A measured response is not an accepted decision or audible
                    // output. Bind it to its originating generation and turn.
                    let mut observation_session = s.clone();
                    observation_session.policy_epoch = epoch;
                    let _ = store.append(Some(&observation_session), "system", "model.request.completed", json!({"generation_id":s.generation_id,"turn_version":s.turn_version,"boundary":"model_http","heard":false,"observation":observed.observation}), s.execution_mode).await;
                    let tool = observed.result?;
                    let outcome = h
                        .send(Input::ModelTool {
                            generation: s.generation_id,
                            epoch,
                            version: s.turn_version,
                            deadline,
                            tool: tool.clone(),
                        })
                        .await?;
                    epoch = outcome["epoch"].as_u64().unwrap_or(epoch);
                    if outcome["ok"] == true && outcome["result"]["stop"] != true {
                        phase = serde_json::from_value(outcome["next_phase"].clone())?;
                    }
                    append_tool_result(&mut messages, &tool, &outcome);
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
                understanding,
            } => {
                let mut candidate = self.session.clone();
                candidate.assess(
                    candidate.generation_id,
                    candidate.turn_version,
                    &observation,
                    &evidence_refs,
                )?;
                if let Some(value) = understanding {
                    candidate.accept_understanding(value)?;
                }
                self.session = candidate;
                self.emit("system",if self.session.blocked{"policy.denied"}else{"policy.allowed"},json!({"operation":"assessment","observation":observation,"quote":self.session.evidence_quote,"handoff_allowed":!self.session.blocked && !self.session.ambiguous})).await?;
                Ok(
                    json!({"blocked":self.session.blocked,"ambiguous":self.session.ambiguous,"assessed_version":self.session.assessed_version,"understanding":self.session.understanding}),
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
                    "What's the code used for? Please don't read the code itself.".to_string()
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
                summary,
                evidence_refs,
            } => {
                let mut candidate = self.session.clone();
                candidate.record_message(&summary, &evidence_refs, Utc::now())?;
                candidate.closing_reason = Some("message_saved".into());
                candidate.state = "closing".into();
                candidate.current_action = "Message saved; closing the call".into();
                candidate.speech_pending = true;
                candidate.generation_id += 1;
                // Failed storage must not leave a message in actor memory that a
                // later snapshot could accidentally persist or display as saved.
                self.app.store.save(&candidate).await?;
                self.session = candidate;
                self.clear_speech_recovery();
                let message = self.session.message.as_ref().expect("Message was recorded");
                // Neither the event nor caller
                // speech repeats the private summary; only authenticated snapshots expose it.
                self.emit(
                    "system",
                    "message.recorded",
                    json!({
                        "recorded_at":message.recorded_at,
                        "turn_version":message.turn_version,
                        "evidence_turn_ids":message.evidence_turn_ids,
                    }),
                )
                .await?;
                // End Relay before scheduling the goodbye. Its documented protocol
                // has no audio-drained acknowledgement. The signed Connect action
                // returns a speech Gather around the acknowledgement, then Hangup
                // after the bounded grace. Caller corrections can reopen screening.
                self.emit("system", "audio.requested", json!({"text":self.saved_message_closing(),"voice":"preset","transport":"twiml_closing","heard":false})).await?;
                let _ = self.handle.audio.send(AudioFrame {
                    generation: self.session.generation_id,
                    epoch: self.session.policy_epoch,
                    payload: crate::providers::relay::end("message_saved"),
                    purpose: AudioPurpose::MessageClosing,
                });
                Ok(json!({"message_recorded":true,"closing":true,"stop":true}))
            }
            ToolCall::PublishTactic { .. } => {
                anyhow::bail!("Publication runs only in a post-call job")
            }
            ToolCall::EndCall {
                reason,
                evidence_refs,
            } => {
                self.session.require_end_call(&evidence_refs)?;
                let mut candidate = self.session.clone();
                candidate.closing_reason = Some("no_message".into());
                candidate.state = "closing".into();
                candidate.current_action = "Closing politely; no message saved".into();
                candidate.speech_pending = true;
                candidate.generation_id += 1;
                self.app.store.save(&candidate).await?;
                self.session = candidate;
                self.clear_speech_recovery();
                self.emit(
                    "system",
                    "call.closing",
                    json!({"reason":reason,"message_recorded":false}),
                )
                .await?;
                let _ = self.handle.audio.send(AudioFrame {
                    generation: self.session.generation_id,
                    epoch: self.session.policy_epoch,
                    payload: crate::providers::relay::end("no_message"),
                    purpose: AudioPurpose::NoMessageClosing,
                });
                Ok(json!({"closing":true,"message_recorded":false,"stop":true}))
            }
        }
    }

    fn saved_message_closing(&self) -> &'static str {
        if self.session.blocked {
            "I can't help with that request. This call won't be connected. I've saved a brief incident note. Goodbye."
        } else {
            "Your message has been noted. No connection has been made. Goodbye."
        }
    }
}

#[cfg(test)]
mod message_tests {
    use super::*;
    use crate::{
        calls::actor::{App, Handle},
        config::Config,
        providers::nemotron::ModelToolCall,
    };

    async fn prepared_message() -> (App, Handle, Value) {
        let mut config = Config::from_env().unwrap();
        config.mode = ExecutionMode::Replay;
        config.database_url = "sqlite::memory:".into();
        let app = App::new(config).await.unwrap();
        let (handle, _) = app
            .incoming("CA00000000000000000000000000000001")
            .await
            .unwrap();
        let turn = handle
            .send(Input::FinalPrompt {
                text: "Please reschedule the clinic visit on Tuesday.".into(),
            })
            .await
            .unwrap();
        let evidence = json!([{"turn_id":turn["turn_id"],"quote":"Please reschedule the clinic visit on Tuesday."}]);
        let assessment = send_tool(&handle, "record_assessment", json!({"observation":{"speech_act":"request","requested_object":"routine_information","direction":"caller_requests_from_sarah"},"evidence_refs":evidence})).await.unwrap();
        assert_eq!(assessment["ok"], true);
        (app, handle, evidence)
    }

    async fn send_tool(handle: &Handle, name: &str, mut arguments: Value) -> anyhow::Result<Value> {
        if name == "record_assessment" && arguments.get("understanding").is_none() {
            arguments["understanding"] = json!({"version":1,"claimed_identity":null,"purpose":"Leave a routine update","requested_action":"Save the caller update","supplied_facts":[],"missing_information":[],"readiness":"ready"});
        }
        let snapshot = handle.send(Input::Snapshot).await?;
        handle
            .send(Input::ModelTool {
                generation: snapshot["generation_id"].as_u64().unwrap(),
                epoch: snapshot["policy_epoch"].as_u64().unwrap(),
                version: snapshot["turn_version"].as_u64().unwrap(),
                deadline: Instant::now() + Duration::from_secs(5),
                tool: ModelToolCall {
                    id: "message-tool".into(),
                    name: name.into(),
                    arguments,
                    assistant_message: json!({}),
                },
            })
            .await
    }

    #[tokio::test]
    async fn take_message_persists_private_summary_before_claiming_success() {
        let (app, handle, evidence) = prepared_message().await;
        let result = send_tool(&handle, "take_message", json!({"summary":"  Caller asks to reschedule the clinic visit on Tuesday.  ","evidence_refs":evidence})).await.unwrap();
        assert_eq!(result["result"]["message_recorded"], true);
        // Read SQLite directly, before another actor request could save memory.
        let stored = app.store.sessions().await.unwrap().pop().unwrap();
        let message = &stored.public_value()["message"];
        assert_eq!(
            message["summary"],
            "Caller asks to reschedule the clinic visit on Tuesday."
        );
        assert_eq!(message["turn_version"], 1);
        assert_eq!(
            message["evidence_turn_ids"],
            json!([evidence[0]["turn_id"]])
        );
        assert!(
            chrono::DateTime::parse_from_rfc3339(message["recorded_at"].as_str().unwrap()).is_ok()
        );
        let events = app.store.events_after(0).await.unwrap();
        let receipt = events
            .iter()
            .find(|event| event.kind == "message.recorded")
            .expect("A durable message has a receipt");
        let audio = events
            .iter()
            .find(|event| event.kind == "audio.requested")
            .unwrap();
        assert!(receipt.sequence < audio.sequence);
        assert!(!receipt.payload.to_string().contains("clinic visit"));
        assert!(!audio.payload.to_string().contains("clinic visit"));
        assert!(app.store.actions(&handle.id).await.unwrap().is_empty());
        let (households, capsules) = crate::patterns::view(&app.store, ExecutionMode::Replay)
            .await
            .unwrap();
        assert!(capsules.is_empty());
        assert!(
            households
                .iter()
                .all(|household| household["last_applied_version"] == 0)
        );
    }

    #[tokio::test]
    async fn written_out_appointment_time_survives_actual_durable_message_storage() {
        let (app, handle, _) = prepared_message().await;
        let text = "The clinic visit is Friday at three PM.";
        let turn = handle
            .send(Input::FinalPrompt { text: text.into() })
            .await
            .unwrap();
        let evidence = json!([{"turn_id":turn["turn_id"],"quote":text}]);
        let assessment=send_tool(&handle,"record_assessment",json!({"observation":{"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"},"evidence_refs":evidence})).await.unwrap();
        assert_eq!(assessment["ok"], true);
        let summary = "Caller reports the clinic visit is Friday at three PM.";
        let result = send_tool(
            &handle,
            "take_message",
            json!({"summary":summary,"evidence_refs":evidence}),
        )
        .await
        .unwrap();
        assert_eq!(result["result"]["message_recorded"], true);
        let stored = app.store.sessions().await.unwrap().pop().unwrap();
        assert_eq!(stored.message.as_ref().unwrap().summary, summary);
        assert_eq!(stored.closing_reason.as_deref(), Some("message_saved"));
        assert!(app.store.actions(&handle.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn take_message_minimizes_contact_literals() {
        let (app, handle, evidence) = prepared_message().await;
        let result = send_tool(&handle, "take_message", json!({"summary":"Caller asks for a reply at person@example.test or +15550123456.","evidence_refs":evidence})).await.unwrap();
        assert_eq!(result["ok"], true);
        let stored = app.store.sessions().await.unwrap().pop().unwrap();
        assert_eq!(
            stored.public_value()["message"]["summary"],
            "Caller asks for a reply at [redacted] or [redacted]"
        );
    }

    #[tokio::test]
    async fn accepted_risky_assessment_selects_protective_closing_without_repeating_private_details()
     {
        for (object, speech_act, expected) in [
            (
                "authentication_secret",
                "request",
                "I can't help with that request. This call won't be connected. I've saved a brief incident note. Goodbye.",
            ),
            (
                "payment_diversion",
                "request",
                "I can't help with that request. This call won't be connected. I've saved a brief incident note. Goodbye.",
            ),
            (
                "remote_access",
                "request",
                "I can't help with that request. This call won't be connected. I've saved a brief incident note. Goodbye.",
            ),
            (
                "authentication_secret",
                "negation",
                "Your message has been noted. No connection has been made. Goodbye.",
            ),
        ] {
            let (app, handle, _) = prepared_message().await;
            let text = match (speech_act, object) {
                ("negation", _) => "Never share sign-in code 734219 with a caller.",
                (_, "authentication_secret") => {
                    "Please read Sarah's sign-in code to me. The reference is 734219."
                }
                (_, "payment_diversion") => {
                    "Please redirect Sarah's payment to my alternate account. The reference is 734219."
                }
                _ => "Please give me remote access to Sarah's computer. The reference is 734219.",
            };
            let turn = handle
                .send(Input::FinalPrompt { text: text.into() })
                .await
                .unwrap();
            let evidence = json!([{"turn_id":turn["turn_id"],"quote":text}]);
            let assessment = send_tool(&handle,"record_assessment",json!({"observation":{"speech_act":speech_act,"requested_object":object,"direction":"caller_requests_from_sarah"},"evidence_refs":evidence})).await.unwrap();
            assert_eq!(assessment["ok"], true);
            let saved = send_tool(&handle,"take_message",json!({"summary":"Private review note about the caller request.","evidence_refs":evidence})).await.unwrap();
            assert_eq!(saved["result"]["message_recorded"], true);
            let closing = handle.send(Input::ClosingResponse).await.unwrap();
            let xml = closing["twiml"].as_str().unwrap();
            let events = app.store.events_after(0).await.unwrap();
            let audio = events
                .iter()
                .rev()
                .find(|e| e.kind == "audio.requested")
                .unwrap();
            assert_eq!(audio.payload["text"], expected);
            assert!(xml.contains(&twilio::escape(expected)));
            assert!(!xml.contains("734219"));
            assert!(!xml.contains("Private review note"));
            assert!(!xml.contains("trusted contact"));
            assert!(app.store.actions(&handle.id).await.unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn take_message_rejects_authentication_details_before_recording() {
        let (app, handle, evidence) = prepared_message().await;
        for summary in [
            "Caller supplied password correcthorsebatterystaple.",
            "Caller supplied passwords correcthorsebatterystaple.",
            "Caller supplied a recovery code correcthorsebatterystaple.",
        ] {
            let result = send_tool(
                &handle,
                "take_message",
                json!({"summary":summary,"evidence_refs":evidence}),
            )
            .await
            .unwrap();
            assert_eq!(result["ok"], false);
        }
        let after = app.store.sessions().await.unwrap().pop().unwrap();
        assert!(after.message.is_none());
        assert!(
            !serde_json::to_string(&after)
                .unwrap()
                .contains("correcthorsebatterystaple")
        );
    }

    #[tokio::test]
    async fn take_message_never_announces_success_when_durable_write_fails() {
        let (app, handle, evidence) = prepared_message().await;
        let before = handle.send(Input::Snapshot).await.unwrap();
        sqlx::raw_sql("CREATE TRIGGER reject_message BEFORE UPDATE OF data ON sessions WHEN json_extract(NEW.data, '$.message.summary') IS NOT NULL BEGIN SELECT RAISE(FAIL, 'message write rejected'); END;").execute(&app.store.pool).await.unwrap();
        let result = send_tool(
            &handle,
            "take_message",
            json!({"summary":"Caller asks to reschedule.","evidence_refs":evidence}),
        )
        .await;
        assert!(
            result.is_err() || result.as_ref().is_ok_and(|value| value["ok"] == false),
            "A failed persistence must not return message_recorded"
        );
        assert!(
            app.store
                .events_after(0)
                .await
                .unwrap()
                .iter()
                .all(|event| !matches!(
                    event.kind.as_str(),
                    "message.recorded" | "audio.requested"
                ))
        );
        sqlx::raw_sql("DROP TRIGGER reject_message")
            .execute(&app.store.pool)
            .await
            .unwrap();
        let after = handle.send(Input::Snapshot).await.unwrap();
        assert!(
            after["message"].is_null(),
            "A rejected candidate must not survive in actor memory"
        );
        assert_eq!(after["current_action"], before["current_action"]);
        let stored = app.store.sessions().await.unwrap().pop().unwrap();
        assert!(stored.public_value()["message"].is_null());
        assert_eq!(
            stored.current_action,
            before["current_action"].as_str().unwrap()
        );
    }

    #[tokio::test]
    async fn take_message_cannot_finalize_while_new_speech_is_unresolved() {
        let (app, handle, evidence) = prepared_message().await;
        handle
            .send(Input::Speech {
                partial: Some("Actually wait".into()),
            })
            .await
            .unwrap();
        let result = send_tool(
            &handle,
            "take_message",
            json!({"summary":"Caller asks to reschedule.","evidence_refs":evidence}),
        )
        .await
        .unwrap();
        assert_eq!(result["ok"], false);
        assert!(
            app.store
                .sessions()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .public_value()["message"]
                .is_null()
        );
    }
}
