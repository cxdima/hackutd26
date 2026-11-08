use crate::{
    calls::state::Session,
    config::ExecutionMode,
    events::Event,
    store::Store,
    tools::{PRETEXT_CATEGORIES, PressureCategory, REQUESTED_ACTIONS, TacticKind},
};
use anyhow::{Result, ensure};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
use tokio::{sync::broadcast, task::JoinHandle};

const HOUSEHOLDS: [(&str, &str); 2] = [("sarah", "Sarah"), ("maya", "Maya and Jimmy")];

#[derive(Clone, Serialize, Deserialize)]
struct StoredCapsule {
    execution_mode: ExecutionMode,
    capsule: Value,
}

fn mode_key(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Live => "live",
        ExecutionMode::Replay => "replay",
    }
}

fn categorical(requested_action: &str, pretext: &str) -> Result<()> {
    ensure!(
        REQUESTED_ACTIONS.contains(&requested_action),
        "Unknown requested-action category"
    );
    ensure!(
        PRETEXT_CATEGORIES.contains(&pretext),
        "Unknown pretext category"
    );
    Ok(())
}

/// Publish only after a synthetic incident is assessed and the inbound call ends.
/// The source binding stays in private metadata; shared capsules contain categories
/// and operational receipt metadata only. Reservation, version and event commit once.
pub async fn publish(
    store: &Store,
    source_session: &Session,
    tactic: TacticKind,
    requested_action: &str,
    pretext: &str,
    pressure: PressureCategory,
) -> Result<Value> {
    ensure!(
        source_session.blocked && source_session.inbound_ended,
        "A completed protective incident is required"
    );
    ensure!(
        source_session.data_origin == "synthetic",
        "Automatic publication is limited to designated synthetic incidents"
    );
    ensure!(
        HOUSEHOLDS
            .iter()
            .any(|(id, _)| *id == source_session.household_id),
        "Unknown source household"
    );
    categorical(requested_action, pretext)?;
    let compatible = match tactic {
        TacticKind::AuthenticationCodeExtraction => matches!(
            requested_action,
            "relay_sign_in_text_digits" | "share_authentication_secret"
        ),
        TacticKind::RemoteAccessRequest => requested_action == "install_remote_access",
        TacticKind::PaymentDiversion => {
            matches!(requested_action, "send_payment" | "buy_gift_card")
        }
    };
    ensure!(compatible, "Tactic does not match requested action");
    let mode = source_session.execution_mode;
    let source_key = format!("tactic-source:{}:{}", mode_key(mode), source_session.id);
    let mut tx = store.pool.begin().await?;
    if let Some(id) = sqlx::query_scalar::<_, String>("SELECT value FROM metadata WHERE key=?")
        .bind(&source_key)
        .fetch_optional(&mut *tx)
        .await?
    {
        let stored: String = sqlx::query_scalar("SELECT data FROM tactics WHERE id=?")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        return Ok(serde_json::from_str::<StoredCapsule>(&stored)?.capsule);
    }
    let version: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(version),0)+1 FROM tactics")
        .fetch_one(&mut *tx)
        .await?;
    let id = uuid::Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + Duration::hours(24)).to_rfc3339();
    let capsule = json!({
        "id":id,"version":version,"schema_version":1,"tactic":tactic,
        "requested_action":requested_action,"pretext_category":pretext,
        "pressure_category":pressure,"recommended_action":"independent_contact",
        "provenance":"designated_synthetic_test","ttl_hours":24,"expires_at":expires_at,
    });
    let stored = StoredCapsule {
        execution_mode: mode,
        capsule: capsule.clone(),
    };
    sqlx::query("INSERT INTO tactics(id,version,data,expires_at) VALUES(?,?,?,?)")
        .bind(&id)
        .bind(version)
        .bind(serde_json::to_string(&stored)?)
        .bind(&expires_at)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO metadata(key,value) VALUES(?,?)")
        .bind(&source_key)
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    let event = append_in(&mut tx, "tactic.created", capsule.clone(), mode).await?;
    tx.commit().await?;
    let _ = store.bus.send(event);
    Ok(capsule)
}

/// Lookup is limited to the subscriber's applied version and current execution
/// mode. Caller identity is deliberately absent from this API.
pub async fn lookup(
    store: &Store,
    household: &str,
    mode: ExecutionMode,
    requested_action: &str,
    pretext: &str,
) -> Result<Vec<Value>> {
    ensure!(
        HOUSEHOLDS.iter().any(|(id, _)| *id == household),
        "Unknown household"
    );
    categorical(requested_action, pretext)?;
    if matches!(requested_action, "routine_information" | "unknown") || pretext == "unknown" {
        return Ok(vec![]);
    }
    let applied = applied_version(store, household, mode).await?;
    let mut matches = vec![];
    for stored in unexpired(store).await? {
        let mut capsule = stored.capsule;
        if stored.execution_mode == mode
            && capsule["version"]
                .as_i64()
                .is_some_and(|version| version <= applied)
            && capsule["requested_action"] == requested_action
            && capsule["pretext_category"] == pretext
        {
            capsule["matched_fields"] = json!(["requested_action", "pretext_category"]);
            capsule["household_applied_version"] = json!(applied);
            matches.push(capsule);
        }
    }
    Ok(matches)
}

/// Return the current subscriber receipts and publishable capsules, respectively.
pub async fn view(store: &Store, mode: ExecutionMode) -> Result<(Vec<Value>, Vec<Value>)> {
    let mut households = vec![];
    for (id, display_name) in HOUSEHOLDS {
        households.push(json!({"household_id":id,"display_name":display_name,"last_applied_version":applied_version(store,id,mode).await?}));
    }
    let capsules = unexpired(store)
        .await?
        .into_iter()
        .filter(|stored| stored.execution_mode == mode)
        .map(|stored| stored.capsule)
        .collect();
    Ok((households, capsules))
}

async fn applied_version(store: &Store, household: &str, mode: ExecutionMode) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT version FROM household_tactic_versions WHERE household_id=?",
    )
    .bind(format!("{}:{household}", mode_key(mode)))
    .fetch_optional(&store.pool)
    .await?
    .unwrap_or(0))
}

async fn unexpired(store: &Store) -> Result<Vec<StoredCapsule>> {
    let rows = sqlx::query("SELECT data,expires_at FROM tactics ORDER BY version")
        .fetch_all(&store.pool)
        .await?;
    let now = Utc::now();
    let mut active = vec![];
    for row in rows {
        let expires_at: &str = row.get("expires_at");
        if DateTime::parse_from_rfc3339(expires_at)? <= now {
            continue;
        }
        let mut stored: StoredCapsule = serde_json::from_str(row.get::<&str, _>("data"))?;
        // The database expiry is authoritative, including an operator's early expiry.
        stored.capsule["expires_at"] = json!(expires_at);
        active.push(stored);
    }
    Ok(active)
}

/// A separate subscriber applies stored capsules and emits durable acknowledgements.
/// It subscribes before recovery, then scans again on events/lag or a bounded timer,
/// so startup or a lost in-process notification cannot falsely imply application.
pub fn spawn_subscriber(store: Store) -> JoinHandle<()> {
    let mut receiver = store.bus.subscribe();
    tokio::spawn(async move {
        let mut recovery_tick = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            if store.pool.is_closed() {
                break;
            }
            if apply_pending(&store).await.is_err() {
                tracing::warn!("Community subscriber application pending after storage failure");
            }
            loop {
                tokio::select! {
                    _ = recovery_tick.tick() => break,
                    message = receiver.recv() => match message {
                        Ok(event) if event.kind=="tactic.created" => break,
                        Ok(_) => {},
                        Err(broadcast::error::RecvError::Lagged(_)) => break,
                        Err(broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        }
    })
}

async fn apply_pending(store: &Store) -> Result<()> {
    for stored in unexpired(store).await? {
        let mode = stored.execution_mode;
        let version = stored.capsule["version"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("Invalid capsule version"))?;
        for (household, _) in HOUSEHOLDS {
            let key = format!("{}:{household}", mode_key(mode));
            let mut tx = store.pool.begin().await?;
            let current = sqlx::query_scalar::<_, i64>(
                "SELECT version FROM household_tactic_versions WHERE household_id=?",
            )
            .bind(&key)
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(0);
            if current >= version {
                continue;
            }
            // Recheck expiry inside the transaction, rather than trusting the scan.
            let expiry: String = sqlx::query_scalar("SELECT expires_at FROM tactics WHERE id=?")
                .bind(stored.capsule["id"].as_str().unwrap_or_default())
                .fetch_one(&mut *tx)
                .await?;
            if DateTime::parse_from_rfc3339(&expiry)? <= Utc::now() {
                continue;
            }
            sqlx::query("INSERT INTO household_tactic_versions(household_id,version) VALUES(?,?) ON CONFLICT(household_id) DO UPDATE SET version=excluded.version")
                .bind(&key).bind(version).execute(&mut *tx).await?;
            let event = append_in(&mut tx, "community.applied", json!({"household_id":household,"version":version,"capsule_id":stored.capsule["id"]}), mode).await?;
            tx.commit().await?;
            let _ = store.bus.send(event);
        }
    }
    Ok(())
}

/// Event and changed memory commit in one SQLite transaction. Broadcasting occurs
/// afterwards; SSE can recover this persisted receipt even when delivery is lost.
async fn append_in(
    tx: &mut Transaction<'_, Sqlite>,
    kind: &str,
    payload: Value,
    mode: ExecutionMode,
) -> Result<Event> {
    let now = Utc::now().to_rfc3339();
    let mut event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: None,
        leg_role: "system".into(),
        occurred_at: now.clone(),
        received_at: now,
        sequence: 0,
        policy_epoch: 0,
        execution_mode: mode,
        data_origin: "synthetic".into(),
        kind: kind.into(),
        payload,
    };
    event.sequence = sqlx::query("INSERT INTO events(data) VALUES('')")
        .execute(&mut **tx)
        .await?
        .last_insert_rowid();
    sqlx::query("UPDATE events SET data=? WHERE sequence=?")
        .bind(serde_json::to_string(&event)?)
        .bind(event.sequence)
        .execute(&mut **tx)
        .await?;
    Ok(event)
}
