//! One operator-armed private handoff; independent of the existing smoke probe.
use crate::config::ExecutionMode;
use crate::{
    calls::state::{Leg, Session},
    config::Config,
    store::Store,
};
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::SqliteConnection;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmRequest {
    pub contact_id: String,
    pub session_id: Option<String>,
    pub ttl_seconds: Option<u16>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisarmRequest {
    pub arm_id: String,
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub state: String,
    pub arm_id: Option<String>,
    pub session_id: Option<String>,
    pub household_id: Option<String>,
    pub contact_id: &'static str,
    pub expires_at: Option<String>,
    pub action_id: Option<String>,
    pub remaining_attempts: u8,
    pub recipient_enrolled: bool,
    pub outbound_configured: bool,
    pub can_arm: bool,
    pub can_disarm: bool,
    pub scope: &'static str,
    pub probe_budget_unchanged: bool,
    pub private_summary_provider: &'static str,
    pub private_summary_voice: &'static str,
    pub private_summary_source: &'static str,
}
const KEY: &str = "phone_demo.v1";
const USED_KEY: &str = "phone_demo.attempts_used";
/// Demo-day allowance of private handoff attempts (`PHONE_DEMO_MAX_ATTEMPTS`, 1–30).
/// Defaults to the original single attempt; each reserved attempt is counted durably.
fn max_attempts() -> u32 {
    std::env::var("PHONE_DEMO_MAX_ATTEMPTS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| (1..=30).contains(n))
        .unwrap_or(1)
}
async fn used(connection: &mut SqliteConnection) -> Result<u32> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
        .bind(USED_KEY)
        .fetch_optional(connection)
        .await?;
    Ok(raw.and_then(|v| v.parse().ok()).unwrap_or(0))
}
#[derive(Clone, Serialize, Deserialize)]
struct Arm {
    id: String,
    session_id: Option<String>,
    household_id: String,
    recipient_fingerprint: String,
    armed_at: DateTime<Utc>,
    /// Global receipt boundary captured in the same transaction as operator arming.
    /// Older persisted arms lack this proof and cannot authorize a handoff.
    #[serde(default)]
    bind_after_sequence: Option<i64>,
    expires_at: DateTime<Utc>,
    state: String,
    action_id: Option<String>,
}
fn recipient(config: &Config) -> Option<&str> {
    config
        .contacts
        .get("sarah")
        .map(String::as_str)
        .filter(|number| crate::config::is_e164(number))
}
fn configured(config: &Config) -> bool {
    config.mode == ExecutionMode::Live
        && config.outbound_configured()
        && recipient(config).is_some()
        && !config.admin_token.is_empty()
}
fn fingerprint(config: &Config) -> Option<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(config.admin_token.as_bytes()).ok()?;
    mac.update(b"FirstRing private handoff recipient v1:");
    mac.update(recipient(config)?.as_bytes());
    Some(
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}
fn active(arm: &Arm) -> bool {
    arm.state == "armed" && arm.action_id.is_none() && arm.expires_at > Utc::now()
}
fn active_session(session: &Session) -> bool {
    session.execution_mode == ExecutionMode::Live
        && !session.inbound_ended
        && !session.end_requested
        && !session.bridge_committed
        && session.closing_reason.is_none()
}
fn matches(arm: &Arm, config: &Config, session: &Session) -> bool {
    configured(config)
        && arm.bind_after_sequence.is_some()
        && active(arm)
        && active_session(session)
        && arm.session_id.as_deref() == Some(session.id.as_str())
        && arm.household_id == session.household_id
        && fingerprint(config).as_deref() == Some(arm.recipient_fingerprint.as_str())
}
async fn read(connection: &mut SqliteConnection) -> Result<Option<Arm>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
        .bind(KEY)
        .fetch_optional(connection)
        .await?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(Into::into))
        .transpose()
}
async fn write(connection: &mut SqliteConnection, arm: &Arm) -> Result<()> {
    sqlx::query("INSERT INTO metadata(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(KEY).bind(serde_json::to_string(arm)?).execute(connection).await?;
    Ok(())
}
fn view(config: &Config, arm: Option<&Arm>, used: u32) -> Status {
    let consumed = arm.is_some_and(|value| value.action_id.is_some());
    // A legacy record consumed before counting existed still counts as one attempt.
    let used = used.max(u32::from(consumed));
    let remaining = max_attempts().saturating_sub(used);
    let armed = arm.is_some_and(active);
    Status {
        state: arm
            .map_or("unarmed", |value| {
                if consumed {
                    "reserved"
                } else if value.state == "armed" && !armed {
                    "expired"
                } else {
                    value.state.as_str()
                }
            })
            .into(),
        arm_id: arm.map(|value| value.id.clone()),
        session_id: arm.and_then(|value| value.session_id.clone()),
        household_id: arm.map(|value| value.household_id.clone()),
        contact_id: "sarah",
        expires_at: arm.map(|value| value.expires_at.to_rfc3339()),
        action_id: arm.and_then(|value| value.action_id.clone()),
        remaining_attempts: u8::try_from(remaining).unwrap_or(u8::MAX),
        recipient_enrolled: recipient(config).is_some(),
        outbound_configured: config.outbound_configured(),
        can_arm: configured(config) && !armed && remaining > 0,
        can_disarm: armed,
        scope: "one_private_handoff",
        probe_budget_unchanged: true,
        private_summary_provider: "Amazon Polly",
        private_summary_voice: "Joanna-Neural",
        private_summary_source: "configured",
    }
}
pub async fn status(store: &Store, config: &Config) -> Result<Status> {
    let mut connection = store.pool.acquire().await?;
    let arm = read(&mut connection).await?;
    let used = used(&mut connection).await?;
    Ok(view(config, arm.as_ref(), used))
}
pub async fn arm(store: &Store, config: &Config, request: ArmRequest) -> Result<Status> {
    ensure!(
        configured(config),
        "Live outbound setup and an enrolled Sarah recipient are required"
    );
    ensure!(
        request.contact_id == "sarah",
        "Only the enrolled Sarah recipient is allowed"
    );
    let ttl = request.ttl_seconds.unwrap_or(300);
    ensure!(
        (60..=600).contains(&ttl),
        "Arm expiry must be between 60 and 600 seconds"
    );
    let mut tx = store.pool.begin().await?;
    let attempts = used(&mut tx).await?;
    if let Some(previous) = read(&mut tx).await? {
        let spent = attempts.max(u32::from(previous.action_id.is_some()));
        ensure!(
            spent < max_attempts(),
            "The private-handoff allowance is used up; reconcile it without retrying"
        );
        ensure!(
            !active(&previous),
            "A private-handoff arm is already active; disarm it before replacing it"
        );
    }
    let household_id = if let Some(id) = &request.session_id {
        let raw: Option<String> = sqlx::query_scalar("SELECT data FROM sessions WHERE id=?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        let session: Session =
            serde_json::from_str(&raw.ok_or_else(|| anyhow::anyhow!("Session not found"))?)?;
        ensure!(
            active_session(&session),
            "An active live session is required"
        );
        session.household_id
    } else {
        sqlx::query_scalar("SELECT value FROM metadata WHERE key='selected_household'")
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or_else(|| "sarah".into())
    };
    ensure!(
        ["sarah", "maya"].contains(&household_id.as_str()),
        "Unknown household"
    );
    let boundary: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sequence),0) FROM events")
        .fetch_one(&mut *tx)
        .await?;
    let now = Utc::now();
    let value = Arm {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: request.session_id,
        household_id,
        recipient_fingerprint: fingerprint(config).expect("configured recipient"),
        armed_at: now,
        bind_after_sequence: Some(boundary),
        expires_at: now + chrono::Duration::seconds(i64::from(ttl)),
        state: "armed".into(),
        action_id: None,
    };
    write(&mut tx, &value).await?;
    tx.commit().await?;
    Ok(view(config, Some(&value), attempts))
}
pub async fn disarm(store: &Store, config: &Config, arm_id: &str) -> Result<Status> {
    let mut tx = store.pool.begin().await?;
    let mut value = read(&mut tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No private handoff is armed"))?;
    ensure!(
        value.id == arm_id,
        "The arm has changed; refresh before disarming"
    );
    ensure!(
        value.action_id.is_none(),
        "The attempt is already reserved; disarming cannot cancel or refund it"
    );
    value.state = "disarmed".into();
    write(&mut tx, &value).await?;
    let attempts = used(&mut tx).await?;
    tx.commit().await?;
    Ok(view(config, Some(&value), attempts))
}
/// Call only after signed inbound validation; synthetic/replay sessions never bind.
pub async fn bind_signed_inbound(
    store: &Store,
    config: &Config,
    session: &Session,
) -> Result<bool> {
    if !configured(config) || !active_session(session) {
        return Ok(false);
    }
    let mut tx = store.pool.begin().await?;
    let Some(mut value) = read(&mut tx).await? else {
        return Ok(false);
    };
    if !active(&value)
        || value.session_id.is_some()
        || value.household_id != session.household_id
        || fingerprint(config).as_deref() != Some(value.recipient_fingerprint.as_str())
    {
        return Ok(false);
    }
    let Some(boundary) = value.bind_after_sequence else {
        return Ok(false);
    };
    let first_inbound: Option<i64> = sqlx::query_scalar(
        "SELECT MIN(sequence) FROM events WHERE json_extract(data,'$.session_id')=? AND json_extract(data,'$.type')='call.inbound.received' AND json_extract(data,'$.execution_mode')='live'",
    ).bind(&session.id).fetch_one(&mut *tx).await?;
    if first_inbound.is_none_or(|sequence| sequence <= boundary) {
        return Ok(false);
    }
    value.session_id = Some(session.id.clone());
    write(&mut tx, &value).await?;
    tx.commit().await?;
    Ok(true)
}
pub async fn available_for(store: &Store, config: &Config, session: &Session) -> Result<bool> {
    let mut connection = store.pool.acquire().await?;
    Ok(read(&mut connection)
        .await?
        .is_some_and(|value| matches(&value, config, session)))
}
/// Consumes the fixed durable grant and reserves the side effect in one transaction.
pub async fn reserve_handoff(
    store: &Store,
    config: &Config,
    session: &Session,
    key: &str,
    leg: &Leg,
) -> Result<(Leg, bool)> {
    ensure!(
        session.execution_mode == ExecutionMode::Live,
        "Replay cannot reserve a real private handoff"
    );
    ensure!(
        leg.kind == "handoff"
            && leg.role == "sarah"
            && leg.status == "reserved"
            && leg.call_sid.is_none()
            && !leg.acknowledged,
        "Invalid private-handoff reservation"
    );
    let mut tx = store.pool.begin().await?;
    let previous: Option<String> =
        sqlx::query_scalar("SELECT data FROM actions WHERE idempotency_key=? AND session_id=?")
            .bind(key)
            .bind(&session.id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(previous) = previous {
        return Ok((serde_json::from_str(&previous)?, false));
    }
    let mut value = read(&mut tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Private handoff is not armed"))?;
    ensure!(
        matches(&value, config, session),
        "Private handoff is unarmed, expired, consumed or bound to another recipient/session"
    );
    sqlx::query(
        "INSERT INTO actions(id,session_id,idempotency_key,kind,mode,data) VALUES(?,?,?,?,?,?)",
    )
    .bind(&leg.action_id)
    .bind(&session.id)
    .bind(key)
    .bind("handoff")
    .bind("live")
    .bind(serde_json::to_string(leg)?)
    .execute(&mut *tx)
    .await?;
    value.action_id = Some(leg.action_id.clone());
    value.state = "reserved".into();
    write(&mut tx, &value).await?;
    let spent = used(&mut tx).await? + 1;
    sqlx::query("INSERT INTO metadata(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(USED_KEY)
        .bind(spent.to_string())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((leg.clone(), true))
}

/// Readiness for a new originating fixture; no call or grant consumption occurs.
pub async fn awaiting_signed_inbound(store: &Store, config: &Config) -> Result<bool> {
    if !configured(config) {
        return Ok(false);
    }
    let mut connection = store.pool.acquire().await?;
    let Some(arm) = read(&mut connection).await? else {
        return Ok(false);
    };
    let household: Option<String> =
        sqlx::query_scalar("SELECT value FROM metadata WHERE key='selected_household'")
            .fetch_optional(&mut *connection)
            .await?;
    Ok(active(&arm)
        && arm.bind_after_sequence.is_some()
        && arm.session_id.is_none()
        && arm.household_id == household.as_deref().unwrap_or("sarah")
        && fingerprint(config).as_deref() == Some(arm.recipient_fingerprint.as_str()))
}
