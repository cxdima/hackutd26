//! One explicitly authorized, inert-until-operator-started synthetic caller canary.
use crate::{
    calls::{
        actor::App,
        state::{Leg, terminal},
    },
    config::ExecutionMode,
    providers::twilio::{self, CreateOutcome},
    store::Store,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use subtle::ConstantTimeEq;

pub const SCENARIO: &str = "clinic_to_login_george";
const KEY: &str = "audio-fixture:owned-number:one-attempt:2026-11-07";
const META: &str = "audio-fixture.run.v1";
const ACTIVE: &str = "audio-fixture.active.v2";
const MAX_ATTEMPTS: usize = 6;
fn run_key(action: &str) -> String {
    format!("audio-fixture.run.v2:{action}")
}
fn default_scenario() -> String {
    SCENARIO.into()
}
fn scenario_clips(id: &str) -> anyhow::Result<&'static [usize]> {
    Ok(match id {
        SCENARIO => &[0, 1],
        "routine_message_george" => &[0, 2],
        "ambiguous_to_login_george" => &[3, 1],
        "ambiguous_to_reference_george" => &[3, 5],
        "explicit_login_george" => &[1],
        "negated_warning_george" => &[4],
        "supplied_reference_george" => &[5],
        "quoted_report_george" => &[6],
        "handoff_changed_george" => &[7, 8],
        "family_pickup_benign_george" => &[9, 10],
        "family_pickup_risky_george" => &[9, 11],
        _ => anyhow::bail!("Unknown audio fixture scenario"),
    })
}
const CACHE: &str = ".tools/audio-fixtures/20261108T035107Z-f7c2c209-a79e-4ae9-bc1a-e8e3b27812c4";
const FAMILY_CACHE: &str =
    ".tools/audio-fixtures/20261108T052514Z-53c19a1c-2bc3-4ce0-8aa7-d16a7615235e";
const CLIPS: [(&str, &str, &str); 12] = [
    (
        "clinic_opening_caller_brisk",
        "003ee739433076ec124e2b68353c6c93b37163e32c6424073f95b4b52978a58f",
        CACHE,
    ),
    (
        "login_request_caller_brisk",
        "fa61bc58fc8af658ced2218965997916da0f5bde009322683a53236c4f9114d7",
        CACHE,
    ),
    (
        "clinic_details_caller_brisk",
        "45d4912041d8cabb6a71329e8e76e18fc2c84bafa36f2f52110045338babfd09",
        CACHE,
    ),
    (
        "ambiguous_code_caller_brisk",
        "9f7ac0e120ac081c966d43e2cbfeccd6a577f5070164a76eef18a5b9243265ae",
        CACHE,
    ),
    (
        "negated_warning_caller_brisk",
        "cd2179fb88031762528526b7d83f10dc373d58738806ca01afba1a1b741d49e2",
        CACHE,
    ),
    (
        "supplied_reference_caller_brisk",
        "9116cfe56ceb304bd96b127adb5ef6360c18270c207bec4520be0afa4a41f2be",
        CACHE,
    ),
    (
        "quoted_report_caller_brisk",
        "aee8680b669acb8d4205a2f8eebaf1f7589d58b7d4c64229979d841f887266b6",
        CACHE,
    ),
    (
        "handoff_opening_caller_brisk",
        "4725f4c8167d1e8039c502334ed54da376a5ae610ad76bcee2ea8238de1d3780",
        CACHE,
    ),
    (
        "handoff_changed_request_caller_brisk",
        "1fa09fb7413a0d1df7b11688223997f5610b9074bfebdea18f042e2bb4d0d29d",
        CACHE,
    ),
    (
        "family_opening_caller_george",
        "18ee6db29e55948535d1fab7eb8cb757d0cf5cfa326d5a84ed065d205765d387",
        FAMILY_CACHE,
    ),
    (
        "family_benign_details_caller_george",
        "1470a7008be863bd48d327c08f26cca3b5d4d3d52d8bee1c4b92f93b02e69994",
        FAMILY_CACHE,
    ),
    (
        "family_risky_details_caller_george",
        "171b514138fdb6c332fae46ec58e37f449cc65df5621f769ffc931521aead451",
        FAMILY_CACHE,
    ),
];

#[derive(Clone, Serialize, Deserialize)]
pub struct Run {
    pub leg: Leg,
    #[serde(default = "default_scenario")]
    scenario_id: String,
    #[serde(default)]
    inbound_ended: bool,
    #[serde(default)]
    loop_detected: bool,
    pub expires_at: i64,
    capability: String,
    next_step: u8,
    replies: BTreeMap<u8, String>,
    observations: Vec<Value>,
    #[serde(default)]
    partial_observations: BTreeMap<u8, Value>,
    outcome: String,
    #[serde(default)]
    inbound_sid: Option<String>,
    #[serde(default)]
    recording_sid: Option<String>,
    #[serde(default)]
    recording_duration_seconds: Option<u32>,
    #[serde(default)]
    provider_error_code: Option<u32>,
}
impl Run {
    fn bind(&mut self, sid: &str) -> anyhow::Result<u8> {
        anyhow::ensure!(twilio::valid_call_sid(sid), "Invalid defender leg");
        if self.inbound_sid.as_deref() == Some(sid) {
            return Ok(0);
        }
        if self.inbound_sid.is_some() {
            self.loop_detected = true;
            self.outcome = "duplicate_inbound_loop_stopped".into();
            return Ok(2);
        }
        self.inbound_sid = Some(sid.into());
        Ok(1)
    }
    fn capture_recording(
        &mut self,
        sid: &str,
        recording: &str,
        duration: &str,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            twilio::valid_call_sid(sid)
                && self
                    .leg
                    .call_sid
                    .as_deref()
                    .is_none_or(|known| known == sid),
            "Wrong recording leg"
        );
        anyhow::ensure!(
            recording.starts_with("RE")
                && recording.len() == 34
                && recording[2..].bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid recording identity"
        );
        anyhow::ensure!(
            self.recording_sid
                .as_deref()
                .is_none_or(|known| known == recording),
            "Conflicting recording"
        );
        self.recording_sid = Some(recording.into());
        self.leg.call_sid = Some(sid.into());
        self.recording_duration_seconds = duration.parse().ok();
        Ok(())
    }
    fn new() -> Self {
        Self {
            leg: Leg {
                action_id: uuid::Uuid::new_v4().to_string(),
                role: "fixture_caller".into(),
                kind: "fixture_caller".into(),
                status: "reserved".into(),
                acknowledged: false,
                call_sid: None,
            },
            scenario_id: SCENARIO.into(),
            inbound_ended: false,
            loop_detected: false,
            expires_at: chrono::Utc::now().timestamp() + 300,
            capability: format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
            next_step: 0,
            replies: BTreeMap::new(),
            observations: vec![],
            partial_observations: BTreeMap::new(),
            outcome: "reserved".into(),
            inbound_sid: None,
            recording_sid: None,
            recording_duration_seconds: None,
            provider_error_code: None,
        }
    }
    fn respond(
        &mut self,
        base: &str,
        sid: &str,
        step: u8,
        speech: &str,
        confidence: &str,
    ) -> anyhow::Result<String> {
        anyhow::ensure!(!self.loop_detected, "Fixture loop stopped");
        anyhow::ensure!(twilio::valid_call_sid(sid), "Invalid fixture leg");
        anyhow::ensure!(
            self.leg
                .call_sid
                .as_deref()
                .is_none_or(|known| known == sid),
            "Wrong fixture leg"
        );
        let last_step = scenario_clips(&self.scenario_id)?.len() as u8 + 1;
        anyhow::ensure!(step <= last_step, "Invalid fixture step");
        if let Some(reply) = self.replies.get(&step) {
            if self.inbound_ended {
                return Ok("<Response><Hangup/></Response>".into());
            }
            return Ok(reply.clone());
        }
        anyhow::ensure!(step == self.next_step, "Out-of-order fixture step");
        anyhow::ensure!(
            chrono::Utc::now().timestamp() < self.expires_at
                && (self.inbound_ended || !terminal(&self.leg.status)),
            "Fixture ended or expired"
        );
        if !self.inbound_ended {
            self.leg.apply_status(sid, "in-progress")?;
        }
        let speech = speech.trim();
        let reply = if step > 0 && speech.is_empty() {
            self.outcome = "no_speech_result".into();
            "<Response><Hangup/></Response>".into()
        } else {
            if step > 0 {
                self.observations.push(json!({"step":step,"speech_result":speech.chars().take(2000).collect::<String>(),"confidence":confidence.parse::<f64>().ok(),"received_at":chrono::Utc::now().to_rfc3339(),"origin":"twilio_gather_audio_asr"}));
            }
            if self.inbound_ended {
                self.outcome = if step > 0 {
                    "defender_ended_audio_captured_ungraded"
                } else {
                    "defender_ended_before_audio_result"
                }
                .into();
                "<Response><Hangup/></Response>".into()
            } else if step == last_step {
                self.outcome = "audio_exchange_captured_ungraded".into();
                "<Response><Hangup/></Response>".into()
            } else {
                self.outcome = "awaiting_audio_reply".into();
                let play = if step == 0 {
                    String::new()
                } else {
                    format!(
                        "<Play>{}</Play>",
                        twilio::escape(&format!(
                            "{base}/media/fixture/{}/{}",
                            self.capability,
                            step - 1
                        ))
                    )
                };
                let action = twilio::escape(&format!(
                    "{base}/voice/fixture/{}/{}",
                    self.leg.action_id,
                    step + 1
                ));
                let partial = twilio::escape(&format!(
                    "{base}/voice/fixture-partial/{}/{}",
                    self.leg.action_id,
                    step + 1
                ));
                format!(
                    "<Response><Gather input=\"speech\" language=\"en-US\" timeout=\"12\" speechTimeout=\"2\" actionOnEmptyResult=\"true\" action=\"{action}\" method=\"POST\" partialResultCallback=\"{partial}\" partialResultCallbackMethod=\"POST\">{play}</Gather><Hangup/></Response>"
                )
            }
        };
        self.next_step = if self.inbound_ended || self.outcome == "no_speech_result" {
            last_step + 1
        } else {
            step + 1
        };
        self.replies.insert(step, reply.clone());
        Ok(reply)
    }
    fn public(&self) -> Value {
        json!({"scenario_id":self.scenario_id,"action_id":self.leg.action_id,"execution_mode":"live","data_origin":"synthetic","test_kind":"automated_audio_caller","destination":"owned_firstring_number_only","voice":"George","clip_count":scenario_clips(&self.scenario_id).map_or(0,|clips|clips.len()),"time_limit_seconds":90,"provider_status":self.leg.status,"provider_error_code":self.provider_error_code,"call_sid_bound":self.leg.call_sid.is_some(),"inbound_bound":self.inbound_sid.is_some(),"inbound_ended":self.inbound_ended,"outcome":self.outcome,"next_step":self.next_step,"observations":self.observations,"partial_observations":self.partial_observations,"recording_available":self.recording_sid.is_some(),"recording_duration_seconds":self.recording_duration_seconds,"human_audition":false,"noise_tested":false,"verdict":"ungraded"})
    }
}

async fn load_action(store: &Store, action: &str) -> anyhow::Result<Option<Run>> {
    if let Some(raw) = store.metadata(&run_key(action)).await? {
        return Ok(Some(serde_json::from_str(&raw)?));
    }
    let legacy: Option<Run> = store
        .metadata(META)
        .await?
        .map(|raw| serde_json::from_str(&raw))
        .transpose()?;
    Ok(legacy.filter(|run| run.leg.action_id == action))
}
async fn load(store: &Store) -> anyhow::Result<Option<Run>> {
    if let Some(action) = store.metadata(ACTIVE).await? {
        return load_action(store, &action).await;
    }
    let legacy: Option<Run> = store
        .metadata(META)
        .await?
        .map(|raw| serde_json::from_str(&raw))
        .transpose()?;
    if let Some(run) = legacy {
        load_action(store, &run.leg.action_id).await
    } else {
        Ok(None)
    }
}
async fn all_runs(store: &Store) -> anyhow::Result<Vec<Run>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM actions WHERE kind='fixture_caller' AND mode='live' ORDER BY rowid",
    )
    .fetch_all(&store.pool)
    .await?;
    let mut runs = Vec::new();
    for id in ids {
        if let Some(run) = load_action(store, &id).await? {
            runs.push(run);
        }
    }
    Ok(runs)
}
async fn change_action<T>(
    store: &Store,
    action: &str,
    f: impl FnOnce(&mut Run) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let mut tx = store.pool.begin().await?;
    let key = run_key(action);
    let mut raw: Option<String> = sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await?;
    let mut legacy = false;
    if raw.is_none() {
        raw = sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
            .bind(META)
            .fetch_optional(&mut *tx)
            .await?;
        legacy = true;
    }
    let mut run: Run =
        serde_json::from_str(&raw.ok_or_else(|| anyhow::anyhow!("Unknown fixture action"))?)?;
    anyhow::ensure!(run.leg.action_id == action, "Wrong fixture action");
    let result = f(&mut run)?;
    let data = serde_json::to_string(&run)?;
    sqlx::query("INSERT INTO metadata(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(key).bind(&data).execute(&mut *tx).await?;
    if legacy {
        sqlx::query("UPDATE metadata SET value=? WHERE key=?")
            .bind(&data)
            .bind(META)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE actions SET data=? WHERE id=?")
        .bind(serde_json::to_string(&run.leg)?)
        .bind(action)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result)
}
#[cfg(test)]
async fn change<T>(
    store: &Store,
    f: impl FnOnce(&mut Run) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let run = load(store)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No fixture run"))?;
    change_action(store, &run.leg.action_id, f).await
}
/// Called inside Store::reserve's same transaction as the new ledger insertion.
pub(crate) async fn check_budget(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> anyhow::Result<()> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT data FROM actions WHERE kind='fixture_caller' AND mode='live' ORDER BY rowid",
    )
    .fetch_all(&mut **tx)
    .await?;
    anyhow::ensure!(
        rows.len() < MAX_ATTEMPTS,
        "Six-attempt fixture budget exhausted (including failures and retries)"
    );
    let mut failures = Vec::new();
    for raw in rows {
        let leg: Leg = serde_json::from_str(&raw)?;
        anyhow::ensure!(
            terminal(&leg.status),
            "Previous fixture caller is not known terminal; reconcile it first"
        );
        let mut data: Option<String> = sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
            .bind(run_key(&leg.action_id))
            .fetch_optional(&mut **tx)
            .await?;
        if data.is_none() {
            data = sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
                .bind(META)
                .fetch_optional(&mut **tx)
                .await?;
        }
        let run: Run =
            serde_json::from_str(&data.ok_or_else(|| {
                anyhow::anyhow!("Previous fixture metadata needs reconciliation")
            })?)?;
        anyhow::ensure!(
            run.leg.action_id == leg.action_id,
            "Previous fixture metadata needs reconciliation"
        );
        anyhow::ensure!(
            run.inbound_sid.is_none() || run.inbound_ended,
            "Previous defender is not confirmed ended"
        );
        anyhow::ensure!(
            leg.status != "completed" || run.inbound_sid.is_some(),
            "Completed caller lacks correlated defender; reconcile first"
        );
        anyhow::ensure!(
            !run.loop_detected && run.outcome != "duplicate_inbound_loop_stopped",
            "Fixture loop detected; batch stopped"
        );
        failures.push(matches!(
            leg.status.as_str(),
            "failed" | "busy" | "no-answer" | "canceled"
        ));
    }
    anyhow::ensure!(
        failures.len() < 2 || !failures[failures.len() - 2..].iter().all(|failed| *failed),
        "Repeated infrastructure failures; batch stopped"
    );
    Ok(())
}
async fn clip(index: usize) -> anyhow::Result<Vec<u8>> {
    let (name, expected, cache) = CLIPS
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("Unknown fixture clip"))?;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(cache)
        .join(format!("{name}.mp3"));
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|_| anyhow::anyhow!("Cached fixture is missing"))?;
    anyhow::ensure!(
        metadata.is_file() && metadata.len() <= 1_000_000,
        "Invalid cached audio"
    );
    let data = tokio::fs::read(path).await?;
    anyhow::ensure!(
        format!("{:x}", Sha256::digest(&data)) == *expected,
        "Cached audio hash mismatch"
    );
    Ok(data)
}
pub async fn view(app: &App) -> anyhow::Result<Value> {
    let runs = all_runs(&app.store).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM actions WHERE kind='fixture_caller' AND mode='live'",
    )
    .fetch_one(&app.store.pool)
    .await?;
    let mut value = if let Some(run) = load(&app.store).await? {
        run.public()
    } else {
        json!({"scenario_id":SCENARIO,"state":if count==0{"not_started"}else{"reservation_requires_reconciliation"},"destination":"owned_firstring_number_only","voice":"George","clip_count":2,"time_limit_seconds":90,"test_kind":"automated_audio_caller","human_audition":false,"noise_tested":false})
    };
    value["max_attempts"] = json!(MAX_ATTEMPTS);
    value["attempt_count"] = json!(count);
    value["remaining_attempts"] = json!(MAX_ATTEMPTS.saturating_sub(count as usize));
    value["history"] = json!(runs.iter().map(Run::public).collect::<Vec<_>>());
    value["supported_scenarios"] = json!([
        SCENARIO,
        "routine_message_george",
        "ambiguous_to_login_george",
        "ambiguous_to_reference_george",
        "explicit_login_george",
        "negated_warning_george",
        "supplied_reference_george",
        "quoted_report_george",
        "handoff_changed_george",
        "family_pickup_benign_george",
        "family_pickup_risky_george"
    ]);
    value["noise_supported"] = json!(false);
    value["intentional_interrupt_supported"] = json!(false);
    let current = value["scenario_id"].as_str().unwrap_or(SCENARIO);
    let mut cached = true;
    for index in scenario_clips(current)? {
        cached &= clip(*index).await.is_ok();
    }
    value["clips_cached"] = json!(cached);
    Ok(value)
}
async fn public_run(store: &Store, run: &Run) -> anyhow::Result<Value> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM actions WHERE kind='fixture_caller' AND mode='live'",
    )
    .fetch_one(&store.pool)
    .await?;
    let mut value = run.public();
    value["max_attempts"] = json!(MAX_ATTEMPTS);
    value["attempt_count"] = json!(count);
    value["remaining_attempts"] = json!(MAX_ATTEMPTS.saturating_sub(count as usize));
    Ok(value)
}
pub async fn start(app: &App, scenario: &str) -> anyhow::Result<Value> {
    start_request(app, scenario, None).await
}
pub async fn start_request(
    app: &App,
    scenario: &str,
    request_id: Option<&str>,
) -> anyhow::Result<Value> {
    let clips = scenario_clips(scenario)?;
    anyhow::ensure!(app.config.mode == ExecutionMode::Live, "Replay cannot call");
    anyhow::ensure!(
        !app.config.smoke_authorized && !app.config.handoff_enabled && !app.config.family_enabled,
        "Production outbound controls must stay disabled"
    );
    anyhow::ensure!(
        app.config.transport == "relay"
            && app.config.selected_transport_configured()
            && app.config.outbound_configured(),
        "Fixture transport is not configured"
    );
    let key = if let Some(request) = request_id {
        format!(
            "audio-fixture:request:{}",
            uuid::Uuid::parse_str(request)
                .map_err(|_| anyhow::anyhow!("request_id must be an operator UUID"))?
        )
    } else {
        anyhow::ensure!(
            scenario == SCENARIO,
            "New scenarios require an operator request_id"
        );
        KEY.into()
    };
    let existing: Option<String> =
        sqlx::query_scalar("SELECT data FROM actions WHERE idempotency_key=?")
            .bind(&key)
            .fetch_optional(&app.store.pool)
            .await?;
    if let Some(raw) = existing {
        let leg: Leg = serde_json::from_str(&raw)?;
        if let Some(run) = load_action(&app.store, &leg.action_id).await? {
            anyhow::ensure!(
                run.scenario_id == scenario,
                "request_id already belongs to another scenario"
            );
            return public_run(&app.store, &run).await;
        }
        return view(app).await;
    }
    for index in clips {
        clip(*index).await?;
    }
    let mut run = Run::new();
    run.scenario_id = scenario.into();
    let (existing, fresh) = app
        .store
        .reserve("audio-fixture", &key, &run.leg, ExecutionMode::Live)
        .await?;
    if !fresh {
        if let Some(run) = load_action(&app.store, &existing.action_id).await? {
            anyhow::ensure!(
                run.scenario_id == scenario,
                "request_id already belongs to another scenario"
            );
            return public_run(&app.store, &run).await;
        }
        return view(app).await;
    }
    app.store
        .set_metadata(&run_key(&run.leg.action_id), &serde_json::to_string(&run)?)
        .await?;
    if request_id.is_none() {
        app.store
            .set_metadata(META, &serde_json::to_string(&run)?)
            .await?;
    }
    app.store.set_metadata(ACTIVE, &run.leg.action_id).await?;
    app.store.append(None,"fixture_caller","fixture.caller.reserved",json!({"action_id":run.leg.action_id,"scenario_id":scenario,"destination":"owned_firstring_number_only","time_limit_seconds":90}),ExecutionMode::Live).await?;
    let response = public_run(&app.store, &run).await?;
    let app = app.clone();
    tokio::spawn(async move {
        let outcome = app.twilio.create_fixture(&run.leg).await;
        let _ = change_action(&app.store, &run.leg.action_id, |state| {
            match outcome {
                CreateOutcome::Accepted { sid, status } => {
                    state.leg.apply_status(&sid, &status)?;
                }
                CreateOutcome::Rejected { status, code } => {
                    if state.leg.call_sid.is_none() {
                        state.provider_error_code = code;
                        state.leg.status = "failed".into();
                        state.outcome = format!("provider_rejected_{status}");
                    }
                }
                CreateOutcome::Unknown => {
                    if state.leg.call_sid.is_none() {
                        state.leg.status = "unknown".into();
                        state.outcome = "create_outcome_unknown_no_retry".into();
                    }
                }
            }
            Ok(())
        })
        .await;
    });
    Ok(response)
}
pub async fn respond(
    app: &App,
    action: &str,
    step: u8,
    sid: &str,
    speech: &str,
    confidence: &str,
) -> anyhow::Result<String> {
    change_action(&app.store, action, |run| {
        anyhow::ensure!(run.leg.action_id == action, "Wrong fixture action");
        run.respond(&app.config.public_base_url, sid, step, speech, confidence)
    })
    .await
}
pub async fn status(app: &App, action: &str, sid: &str, status: &str) -> anyhow::Result<bool> {
    if load_action(&app.store, action).await?.is_none() {
        return Ok(false);
    }
    change_action(&app.store, action, |run| {
        run.leg.apply_status(sid, status)?;
        Ok(())
    })
    .await?;
    Ok(true)
}
pub async fn partial(
    app: &App,
    action: &str,
    sid: &str,
    step: u8,
    text: &str,
) -> anyhow::Result<()> {
    change_action(&app.store, action, |run| {
        anyhow::ensure!(run.leg.action_id == action && run.leg.call_sid.as_deref() == Some(sid), "Wrong fixture leg");
        anyhow::ensure!((1..=(scenario_clips(&run.scenario_id)?.len() as u8+1)).contains(&step) && step <= run.next_step && chrono::Utc::now().timestamp() < run.expires_at, "Invalid fixture observer step");
        if !text.trim().is_empty() {
            run.partial_observations.insert(step, json!({"step":step,"unstable_speech_result":text.chars().take(2000).collect::<String>(),"is_final":false,"origin":"twilio_gather_audio_asr","received_at":chrono::Utc::now().to_rfc3339()}));
        }
        Ok(())
    }).await
}
pub async fn audio(app: &App, capability: &str, index: usize) -> anyhow::Result<Vec<u8>> {
    let runs = all_runs(&app.store).await?;
    let run = runs
        .iter()
        .find(|run| bool::from(capability.as_bytes().ct_eq(run.capability.as_bytes())))
        .ok_or_else(|| anyhow::anyhow!("Invalid audio capability"))?;
    anyhow::ensure!(
        chrono::Utc::now().timestamp() < run.expires_at,
        "Expired audio capability"
    );
    let selected = scenario_clips(&run.scenario_id)?
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("Unknown fixture clip"))?;
    clip(*selected).await
}
pub async fn inbound_status(app: &App, sid: &str, status: &str) -> anyhow::Result<()> {
    if !terminal(status) {
        return Ok(());
    }
    for run in all_runs(&app.store).await? {
        if run.inbound_sid.as_deref() == Some(sid) {
            change_action(&app.store, &run.leg.action_id, |state| {
                state.inbound_ended = true;
                Ok(())
            })
            .await?;
        }
    }
    Ok(())
}
fn owned_origin(from: &str, number: &str) -> bool {
    !number.is_empty()
        && (from == number
            || from
                .strip_prefix("sip:")
                .or_else(|| from.strip_prefix("sips:"))
                .and_then(|value| value.split_once('@'))
                .is_some_and(|(user, _)| user == number))
}
/// Signed provider ingress + the pending one-attempt reservation correlates this
/// local canary. Caller ID alone is not proof of identity outside this test.
pub async fn bind_inbound(app: &App, sid: &str, from: &str) -> anyhow::Result<u8> {
    if app.config.mode != ExecutionMode::Live || !owned_origin(from, &app.config.photon_number) {
        return Ok(0);
    }
    let Some(run) = load(&app.store).await? else {
        return Ok(0);
    };
    if terminal(&run.leg.status) || chrono::Utc::now().timestamp() >= run.expires_at {
        return Ok(0);
    }
    change_action(&app.store, &run.leg.action_id, |run| {
        if terminal(&run.leg.status) || chrono::Utc::now().timestamp() >= run.expires_at {
            return Ok(0);
        }
        run.bind(sid)
    })
    .await
}
pub fn watch_inbound(app: App, sid: String) {
    tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(90);
        // Provider duration survives app restarts; the local watchdog also cancels
        // actor/model work. Neither path can dial a new number.
        let enforced = app
            .twilio
            .update(&sid, &[("TimeLimit", "90".into())])
            .await
            .is_ok();
        let _=app.store.append(None,"fixture_caller","fixture.defender.bound",json!({"provider_time_limit_applied":enforced,"max_seconds":90,"correlation":"pending_fixture_and_owned_origin"}),ExecutionMode::Live).await;
        tokio::time::sleep_until(deadline).await;
        if let Ok(h) = app.by_sid(&sid).await {
            let _ = h.send(crate::calls::actor::Input::End).await;
        }
    });
}
pub async fn stop(app: &App, action: Option<&str>) -> anyhow::Result<Value> {
    let run = match action {
        Some(id) => load_action(&app.store, id).await?,
        None => load(&app.store).await?,
    }
    .ok_or_else(|| anyhow::anyhow!("No fixture run"))?;
    anyhow::ensure!(
        app.config.mode == ExecutionMode::Live,
        "Replay cannot control provider calls"
    );
    let mut caller_requested = false;
    let mut defender_requested = false;
    if let Some(sid) = run.leg.call_sid.as_deref()
        && !terminal(&run.leg.status)
    {
        caller_requested = app
            .twilio
            .update(sid, &[("Status", "completed".into())])
            .await
            .is_ok();
    }
    if let Some(sid) = run.inbound_sid.as_deref()
        && let Ok(h) = app.by_sid(sid).await
    {
        defender_requested = h.send(crate::calls::actor::Input::End).await.is_ok();
    }
    Ok(
        json!({"caller_stop_accepted":caller_requested,"defender_stop_requested":defender_requested,"completion_requires_status_callback":true}),
    )
}
pub async fn recording_receipt(
    app: &App,
    action: &str,
    sid: &str,
    recording: &str,
    status: &str,
    duration: &str,
) -> anyhow::Result<()> {
    anyhow::ensure!(status == "completed", "Recording is not ready");
    change_action(&app.store, action, |run| {
        anyhow::ensure!(run.leg.action_id == action, "Wrong fixture action");
        run.capture_recording(sid, recording, duration)
    })
    .await
}
pub async fn recording(app: &App, action: Option<&str>) -> anyhow::Result<Vec<u8>> {
    let run = match action {
        Some(id) => load_action(&app.store, id).await?,
        None => load(&app.store).await?,
    }
    .ok_or_else(|| anyhow::anyhow!("No fixture run"))?;
    let recording = run
        .recording_sid
        .ok_or_else(|| anyhow::anyhow!("Recording is not ready"))?;
    app.twilio.fixture_recording(&recording).await
}

#[cfg(test)]
mod tests {
    use super::*;
    const SID: &str = "CA11111111111111111111111111111111";
    #[tokio::test]
    async fn family_scenarios_share_opening_select_their_continuation_and_stay_replay_inert() {
        let app = app().await;
        let public = view(&app).await.unwrap();
        for (scenario, continuation, expected_hash) in [
            (
                "family_pickup_benign_george",
                "family_benign_details_caller_george",
                "1470a7008be863bd48d327c08f26cca3b5d4d3d52d8bee1c4b92f93b02e69994",
            ),
            (
                "family_pickup_risky_george",
                "family_risky_details_caller_george",
                "171b514138fdb6c332fae46ec58e37f449cc65df5621f769ffc931521aead451",
            ),
        ] {
            let selected = scenario_clips(scenario).expect("Family scenario must be allowlisted");
            assert_eq!(selected.len(), 2);
            assert_eq!(CLIPS[selected[0]].0, "family_opening_caller_george");
            assert_eq!(
                CLIPS[selected[0]].1,
                "18ee6db29e55948535d1fab7eb8cb757d0cf5cfa326d5a84ed065d205765d387"
            );
            assert_eq!(CLIPS[selected[1]].0, continuation);
            assert_eq!(CLIPS[selected[1]].1, expected_hash);
            // The ignored cache is optional in a clean checkout. When present,
            // exercise the actual loader so a wrong directory or byte hash fails.
            let cached_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../.tools/audio-fixtures/20261108T052514Z-53c19a1c-2bc3-4ce0-8aa7-d16a7615235e/manifest.json");
            if cached_manifest.is_file() {
                assert_eq!(
                    format!("{:x}", Sha256::digest(clip(selected[0]).await.unwrap())),
                    "18ee6db29e55948535d1fab7eb8cb757d0cf5cfa326d5a84ed065d205765d387"
                );
                assert_eq!(
                    format!("{:x}", Sha256::digest(clip(selected[1]).await.unwrap())),
                    expected_hash
                );
            }
            assert!(
                public["supported_scenarios"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value == scenario)
            );
            let mut run = Run::new();
            run.scenario_id = scenario.into();
            assert_eq!(run.public()["clip_count"], 2);
            assert!(
                !run.respond("https://fixture.example", SID, 0, "", "")
                    .unwrap()
                    .contains("<Play>")
            );
            for step in 1..=2 {
                let reply = run
                    .respond("https://fixture.example", SID, step, "actual reply", "0.9")
                    .unwrap();
                assert!(reply.contains(&format!("/{}</Play>", step - 1)));
            }
            assert_eq!(
                run.respond("https://fixture.example", SID, 3, "goodbye", "0.9")
                    .unwrap(),
                "<Response><Hangup/></Response>"
            );
            assert!(
                run.respond("https://fixture.example", SID, 4, "extra", "1")
                    .is_err()
            );
            let error = start_request(&app, scenario, Some(&uuid::Uuid::new_v4().to_string()))
                .await
                .unwrap_err();
            assert!(error.to_string().contains("Replay cannot call"));
        }
        let actions: i64 = sqlx::query_scalar("SELECT count(*) FROM actions")
            .fetch_one(&app.store.pool)
            .await
            .unwrap();
        assert_eq!(actions, 0);
    }
    #[tokio::test]
    async fn batch_cannot_start_while_an_old_reservation_is_unknown() {
        let app = app().await;
        reserved(&app).await;
        assert!(
            app.store
                .reserve(
                    "audio-fixture",
                    "different-operator-key",
                    &Run::new().leg,
                    ExecutionMode::Live
                )
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn simultaneous_operator_requests_reserve_only_one_live_attempt() {
        let app = app().await;
        let first = Run::new();
        let second = Run::new();
        let (a, b) = tokio::join!(
            app.store.reserve(
                "audio-fixture",
                "concurrent-a",
                &first.leg,
                ExecutionMode::Live
            ),
            app.store.reserve(
                "audio-fixture",
                "concurrent-b",
                &second.leg,
                ExecutionMode::Live
            )
        );
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM actions WHERE kind='fixture_caller'")
                .fetch_one(&app.store.pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }
    async fn seed_terminal(app: &App, key: &str, status: &str, inbound_ended: bool) -> Run {
        let mut run = Run::new();
        run.leg.status = status.into();
        if status == "completed" {
            run.leg.call_sid = Some(SID.into());
            run.inbound_sid = Some("CA22222222222222222222222222222222".into());
            run.inbound_ended = inbound_ended;
        } else {
            run.outcome = "provider_rejected_401".into();
        }
        app.store
            .reserve("audio-fixture", key, &run.leg, ExecutionMode::Live)
            .await
            .unwrap();
        app.store
            .set_metadata(
                &run_key(&run.leg.action_id),
                &serde_json::to_string(&run).unwrap(),
            )
            .await
            .unwrap();
        run
    }
    #[tokio::test]
    async fn six_attempts_including_terminal_calls_exhaust_durable_budget() {
        let app = app().await;
        for n in 0..6 {
            seed_terminal(&app, &format!("operator-{n}"), "completed", true).await;
        }
        assert!(
            app.store
                .reserve(
                    "audio-fixture",
                    "seventh",
                    &Run::new().leg,
                    ExecutionMode::Live
                )
                .await
                .is_err()
        );
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM actions WHERE kind='fixture_caller'")
                .fetch_one(&app.store.pool)
                .await
                .unwrap();
        assert_eq!(count, 6);
        assert_eq!(view(&app).await.unwrap()["remaining_attempts"], 0);
        assert!(
            !app.store
                .reserve(
                    "audio-fixture",
                    "operator-0",
                    &Run::new().leg,
                    ExecutionMode::Live
                )
                .await
                .unwrap()
                .1
        );
    }
    #[tokio::test]
    async fn next_attempt_waits_for_defender_terminal_receipt_not_stop_request() {
        let app = app().await;
        seed_terminal(&app, "first", "completed", false).await;
        assert!(
            app.store
                .reserve(
                    "audio-fixture",
                    "second",
                    &Run::new().leg,
                    ExecutionMode::Live
                )
                .await
                .is_err()
        );
        inbound_status(&app, "CA22222222222222222222222222222222", "in-progress")
            .await
            .unwrap();
        assert!(
            app.store
                .reserve(
                    "audio-fixture",
                    "second",
                    &Run::new().leg,
                    ExecutionMode::Live
                )
                .await
                .is_err()
        );
        inbound_status(&app, "CA22222222222222222222222222222222", "completed")
            .await
            .unwrap();
        assert!(
            app.store
                .reserve(
                    "audio-fixture",
                    "second",
                    &Run::new().leg,
                    ExecutionMode::Live
                )
                .await
                .unwrap()
                .1
        );
    }
    #[tokio::test]
    async fn two_infrastructure_failures_stop_before_a_third_reservation() {
        let app = app().await;
        seed_terminal(&app, "one", "failed", false).await;
        seed_terminal(&app, "two", "failed", false).await;
        assert!(
            app.store
                .reserve(
                    "audio-fixture",
                    "three",
                    &Run::new().leg,
                    ExecutionMode::Live
                )
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn old_callbacks_and_recordings_update_only_their_action() {
        let app = app().await;
        let old = seed_terminal(&app, "first", "completed", true).await;
        let current = Run::new();
        app.store
            .reserve("audio-fixture", "second", &current.leg, ExecutionMode::Live)
            .await
            .unwrap();
        app.store
            .set_metadata(
                &run_key(&current.leg.action_id),
                &serde_json::to_string(&current).unwrap(),
            )
            .await
            .unwrap();
        app.store
            .set_metadata(ACTIVE, &current.leg.action_id)
            .await
            .unwrap();
        status(&app, &old.leg.action_id, SID, "ringing")
            .await
            .unwrap();
        recording_receipt(
            &app,
            &old.leg.action_id,
            SID,
            "RE11111111111111111111111111111111",
            "completed",
            "10",
        )
        .await
        .unwrap();
        let current = load(&app.store).await.unwrap().unwrap();
        assert_eq!(current.leg.status, "reserved");
        assert!(current.recording_sid.is_none());
        let old = load_action(&app.store, &old.leg.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(old.leg.status, "completed");
        assert!(old.recording_sid.is_some());
    }
    #[tokio::test]
    async fn legacy_and_operator_request_replays_do_not_select_the_current_run() {
        let mut app = app().await;
        let first = reserved(&app).await;
        change_action(&app.store, &first.leg.action_id, |run| {
            run.leg.status = "failed".into();
            run.outcome = "provider_rejected_401".into();
            Ok(())
        })
        .await
        .unwrap();
        let request = uuid::Uuid::new_v4();
        let key = format!("audio-fixture:request:{request}");
        let mut second = seed_terminal(&app, &key, "failed", false).await;
        second.scenario_id = "explicit_login_george".into();
        app.store
            .set_metadata(
                &run_key(&second.leg.action_id),
                &serde_json::to_string(&second).unwrap(),
            )
            .await
            .unwrap();
        app.store
            .set_metadata(ACTIVE, &second.leg.action_id)
            .await
            .unwrap();
        let mut config = (*app.config).clone();
        config.mode = ExecutionMode::Live;
        config.nvidia_key = "fixture-no-network".into();
        app.config = std::sync::Arc::new(config);
        // Provider adapter deliberately remains replay even if a regression submitted.
        assert_eq!(
            start_request(&app, SCENARIO, None).await.unwrap()["action_id"],
            first.leg.action_id
        );
        assert_eq!(
            start_request(&app, "explicit_login_george", Some(&request.to_string()))
                .await
                .unwrap()["action_id"],
            second.leg.action_id
        );
        assert!(
            start_request(&app, SCENARIO, Some(&request.to_string()))
                .await
                .is_err()
        );
        assert!(
            start_request(&app, SCENARIO, Some("not-a-uuid"))
                .await
                .is_err()
        );
        assert_eq!(view(&app).await.unwrap()["attempt_count"], 2);
    }
    #[test]
    fn single_clip_scripts_end_after_one_play_and_never_invent_followup_audio() {
        let mut run = Run::new();
        run.scenario_id = "negated_warning_george".into();
        assert!(
            !run.respond("https://fixture.example", SID, 0, "", "")
                .unwrap()
                .contains("<Play>")
        );
        assert!(
            run.respond("https://fixture.example", SID, 1, "Hello", "1")
                .unwrap()
                .contains("<Play>")
        );
        assert_eq!(
            run.respond(
                "https://fixture.example",
                SID,
                2,
                "Message noted. Goodbye.",
                "1"
            )
            .unwrap(),
            "<Response><Hangup/></Response>"
        );
        assert!(
            run.respond("https://fixture.example", SID, 3, "unexpected", "1")
                .is_err()
        );
    }
    #[test]
    fn defender_ending_records_actual_final_audio_without_playing_the_next_clip() {
        let mut run = Run::new();
        run.respond("https://fixture.example", SID, 0, "", "")
            .unwrap();
        let first = run
            .respond(
                "https://fixture.example",
                SID,
                1,
                "I'm Sarah's assistant.",
                "1",
            )
            .unwrap();
        assert!(first.contains("<Play>"));
        run.inbound_ended = true;
        run.leg.status = "completed".into();
        let end = run
            .respond(
                "https://fixture.example",
                SID,
                2,
                "Your message has been noted. Goodbye.",
                "0.9",
            )
            .unwrap();
        assert_eq!(end, "<Response><Hangup/></Response>");
        assert_eq!(run.observations.len(), 2);
        assert_eq!(
            run.observations[1]["speech_result"],
            "Your message has been noted. Goodbye."
        );
        assert_eq!(run.outcome, "defender_ended_audio_captured_ungraded");
        assert_eq!(
            run.respond("https://fixture.example", SID, 2, "duplicate", "0")
                .unwrap(),
            end
        );
        assert_eq!(run.observations.len(), 2);
        assert!(
            !run.respond("https://fixture.example", SID, 1, "old", "0")
                .unwrap()
                .contains("<Play>")
        );
    }
    #[test]
    fn one_defender_leg_and_recording_are_bound_without_looping() {
        let mut run = Run::new();
        assert_eq!(run.bind(SID).unwrap(), 1);
        assert_eq!(run.bind(SID).unwrap(), 0);
        assert_eq!(run.bind("CA22222222222222222222222222222222").unwrap(), 2);
        assert!(run.loop_detected);
        assert!(
            run.respond("https://fixture.example", SID, 0, "", "")
                .is_err()
        );
        run.leg.call_sid = Some(SID.into());
        assert!(
            run.capture_recording(
                "CA22222222222222222222222222222222",
                "RE11111111111111111111111111111111",
                "10"
            )
            .is_err()
        );
        run.capture_recording(SID, "RE11111111111111111111111111111111", "10")
            .unwrap();
        assert_eq!(run.recording_duration_seconds, Some(10));
        assert!(
            run.capture_recording(SID, "RE22222222222222222222222222222222", "10")
                .is_err()
        );
    }
    async fn app() -> App {
        use crate::config::Config;
        App::new(Config {
            public_base_url: "https://fixture.example".into(),
            port: 8080,
            database_url: "sqlite::memory:".into(),
            admin_token: "fixture-admin-at-least-24-chars".into(),
            mode: ExecutionMode::Replay,
            transport: "relay".into(),
            account_sid: format!("AC{}", "0".repeat(32)),
            auth_token: "fixture-auth".into(),
            photon_number: "+15555550101".into(),
            sip_host: "sip.example.invalid".into(),
            sip_port: 5061,
            sip_username: "fixture".into(),
            sip_password: "fixture".into(),
            contacts: BTreeMap::new(),
            smoke_authorized: false,
            handoff_enabled: false,
            family_enabled: false,
            max_call_seconds: 180,
            nvidia_key: String::new(),
            model_backend: crate::config::ModelBackend::NvidiaHosted,
            model_url: "http://127.0.0.1:1/not-contacted".into(),
            model: "fixture".into(),
        })
        .await
        .unwrap()
    }
    async fn reserved(app: &App) -> Run {
        let run = Run::new();
        assert!(
            app.store
                .reserve("audio-fixture", KEY, &run.leg, ExecutionMode::Live)
                .await
                .unwrap()
                .1
        );
        app.store
            .set_metadata(META, &serde_json::to_string(&run).unwrap())
            .await
            .unwrap();
        run
    }
    #[tokio::test]
    async fn fixture_reservation_is_one_use_without_consuming_sarah_probe() {
        let app = app().await;
        let run = reserved(&app).await;
        assert!(
            !app.store
                .reserve("audio-fixture", KEY, &Run::new().leg, ExecutionMode::Live)
                .await
                .unwrap()
                .1
        );
        let probes: i64 = sqlx::query_scalar("SELECT count(*) FROM actions WHERE kind='probe'")
            .fetch_one(&app.store.pool)
            .await
            .unwrap();
        assert_eq!(probes, 0);
        assert_eq!(
            load(&app.store).await.unwrap().unwrap().leg.action_id,
            run.leg.action_id
        );
        assert!(
            start(&app, SCENARIO).await.is_err(),
            "Replay must remain inert"
        );
        assert!(matches!(
            app.twilio.create_fixture(&run.leg).await,
            CreateOutcome::Rejected { status: 503, .. }
        ));
    }
    #[tokio::test]
    async fn provider_fixture_form_pins_owned_number_and_recording_duration() {
        let app = app().await;
        let run = Run::new();
        let form = app.twilio.fixture_call_form(&run.leg);
        let field = |key: &str| {
            form.iter()
                .find(|(name, _)| name == key)
                .unwrap()
                .1
                .as_str()
        };
        assert_eq!(
            field("To"),
            "sip:+15555550101@sip.example.invalid:5061;transport=tls;secure=true"
        );
        assert_eq!(field("From"), app.config.photon_number);
        assert_eq!(field("TimeLimit"), "90");
        assert_eq!(field("Record"), "true");
        assert_eq!(field("RecordingChannels"), "dual");
        assert!(field("Url").ends_with(&format!("/voice/fixture/{}/0", run.leg.action_id)));
        assert!(field("StatusCallback").ends_with(&format!("action_id={}", run.leg.action_id)));
        assert!(!form.iter().any(|(key, _)| key == "Twiml"));
    }
    #[tokio::test]
    async fn signed_status_reconciles_lost_create_and_never_regresses() {
        let app = app().await;
        let run = reserved(&app).await;
        change(&app.store, |r| {
            r.leg.status = "unknown".into();
            Ok(())
        })
        .await
        .unwrap();
        assert!(
            status(&app, &run.leg.action_id, SID, "ringing")
                .await
                .unwrap()
        );
        assert!(
            status(&app, &run.leg.action_id, SID, "initiated")
                .await
                .unwrap()
        );
        assert_eq!(
            load(&app.store).await.unwrap().unwrap().leg.status,
            "ringing"
        );
        status(&app, &run.leg.action_id, SID, "completed")
            .await
            .unwrap();
        status(&app, &run.leg.action_id, SID, "ringing")
            .await
            .unwrap();
        assert_eq!(
            load(&app.store).await.unwrap().unwrap().leg.status,
            "completed"
        );
        assert!(
            status(
                &app,
                &run.leg.action_id,
                "CA22222222222222222222222222222222",
                "completed"
            )
            .await
            .is_err()
        );
        assert!(!status(&app, "unrelated", SID, "ringing").await.unwrap());
    }
    #[tokio::test]
    async fn capability_audio_requires_valid_unexpired_exact_selected_clip() {
        let app = app().await;
        let run = reserved(&app).await;
        assert!(audio(&app, "bad-capability", 0).await.is_err());
        assert!(audio(&app, &run.capability, 2).await.is_err());
        // Cached data is optional for an isolated checkout; validation itself is enforced above.
        if let Ok(data) = audio(&app, &run.capability, 0).await {
            assert_eq!(format!("{:x}", Sha256::digest(data)), CLIPS[0].1);
        }
        change(&app.store, |r| {
            r.expires_at = 0;
            Ok(())
        })
        .await
        .unwrap();
        assert!(audio(&app, &run.capability, 0).await.is_err());
        assert!(
            !view(&app)
                .await
                .unwrap()
                .to_string()
                .contains(&run.capability)
        );
    }
    #[tokio::test]
    async fn partial_audio_receipt_never_advances_or_claims_final() {
        let app = app().await;
        let run = reserved(&app).await;
        respond(&app, &run.leg.action_id, 0, SID, "", "")
            .await
            .unwrap();
        partial(&app, &run.leg.action_id, SID, 1, "I'm Sarah's assistant")
            .await
            .unwrap();
        let saved = load(&app.store).await.unwrap().unwrap();
        assert_eq!(saved.next_step, 1);
        assert!(saved.observations.is_empty());
        assert_eq!(saved.partial_observations[&1]["is_final"], false);
        assert!(
            partial(&app, &run.leg.action_id, SID, 3, "future")
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn fixture_http_requires_operator_and_signed_provider_requests() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;
        let app = app().await;
        let router = crate::http::router(app.clone());
        let request = |method: &str, path: &str, body: &str, auth: bool| {
            let mut builder = Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json");
            if auth {
                builder = builder.header(
                    "authorization",
                    format!("Bearer {}", app.config.admin_token),
                );
            }
            builder.body(Body::from(body.to_owned())).unwrap()
        };
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "POST",
                    "/api/demo/audio-fixture",
                    r#"{"scenario_id":"clinic_to_login_george"}"#,
                    false
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "POST",
                    "/api/demo/audio-fixture",
                    r#"{"scenario_id":"clinic_to_login_george","number":"+15555550123"}"#,
                    true
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "POST",
                    "/api/demo/audio-fixture",
                    r#"{"scenario_id":"clinic_to_login_george"}"#,
                    true
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            router
                .oneshot(request("POST", "/voice/fixture/no-action/0", "", false))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM actions")
            .fetch_one(&app.store.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
    #[test]
    fn greeting_then_two_audio_turns_are_bound_cached_and_no_more() {
        let mut run = Run::new();
        assert!(
            run.respond("https://fixture.example", SID, 2, "early", "1")
                .is_err()
        );
        let greeting = run
            .respond("https://fixture.example", SID, 0, "", "")
            .unwrap();
        assert!(greeting.contains("<Gather"));
        assert!(!greeting.contains("<Play>"));
        let first = run
            .respond(
                "https://fixture.example",
                SID,
                1,
                "I am Sarah's assistant.",
                "0.9",
            )
            .unwrap();
        assert!(first.contains("<Play>"));
        assert_eq!(
            first,
            run.respond("https://fixture.example", SID, 1, "duplicate", "0")
                .unwrap()
        );
        assert!(
            run.respond(
                "https://fixture.example",
                "CA22222222222222222222222222222222",
                2,
                "bad",
                "1"
            )
            .is_err()
        );
        let second = run
            .respond(
                "https://fixture.example",
                SID,
                2,
                "What time is the appointment?",
                "0.9",
            )
            .unwrap();
        assert!(second.contains("<Play>"));
        let end = run
            .respond(
                "https://fixture.example",
                SID,
                3,
                "I cannot help obtain sign-in codes. Goodbye.",
                "0.9",
            )
            .unwrap();
        assert_eq!(end, "<Response><Hangup/></Response>");
        assert_eq!(run.observations.len(), 3);
        assert!(
            run.respond("https://fixture.example", SID, 4, "extra", "1")
                .is_err()
        );
    }
    #[test]
    fn no_speech_stops_without_playing_or_claiming_audio_recognition() {
        let mut run = Run::new();
        run.respond("https://fixture.example", SID, 0, "", "")
            .unwrap();
        assert_eq!(
            run.respond("https://fixture.example", SID, 1, "", "")
                .unwrap(),
            "<Response><Hangup/></Response>"
        );
        assert_eq!(run.outcome, "no_speech_result");
        assert!(run.observations.is_empty());
    }
}
