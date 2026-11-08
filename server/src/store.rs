use crate::{
    calls::state::{Leg, Session},
    config::ExecutionMode,
    events::Event,
};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::str::FromStr;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Store {
    pub pool: SqlitePool,
    pub bus: broadcast::Sender<Event>,
}
impl Store {
    pub async fn open(url: &str) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(3));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::raw_sql("CREATE TABLE IF NOT EXISTS sessions(id TEXT PRIMARY KEY,inbound_sid TEXT UNIQUE NOT NULL,data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS actions(id TEXT PRIMARY KEY,session_id TEXT NOT NULL,idempotency_key TEXT UNIQUE NOT NULL,kind TEXT NOT NULL,mode TEXT NOT NULL,data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS events(sequence INTEGER PRIMARY KEY AUTOINCREMENT,data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS tactics(id TEXT PRIMARY KEY,version INTEGER UNIQUE NOT NULL,data TEXT NOT NULL,expires_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS household_tactic_versions(household_id TEXT PRIMARY KEY,version INTEGER NOT NULL DEFAULT 0); INSERT OR IGNORE INTO household_tactic_versions VALUES('sarah',0),('maya',0);").execute(&pool).await?;
        let (bus, _) = broadcast::channel(256);
        Ok(Self { pool, bus })
    }
    pub async fn save(&self, s: &Session) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO sessions(id,inbound_sid,data) VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data").bind(&s.id).bind(&s.inbound_sid).bind(serde_json::to_string(s)?).execute(&mut *tx).await?;
        for l in &s.legs {
            sqlx::query("UPDATE actions SET data=? WHERE id=?")
                .bind(serde_json::to_string(l)?)
                .bind(&l.action_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn sessions(&self) -> anyhow::Result<Vec<Session>> {
        let rows = sqlx::query("SELECT data FROM sessions ORDER BY rowid DESC LIMIT 100")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|r| Ok(serde_json::from_str::<Session>(r.get::<&str, _>("data"))?))
            .collect()
    }
    pub async fn actions(&self, session_id: &str) -> anyhow::Result<Vec<Leg>> {
        let rows = sqlx::query("SELECT data FROM actions WHERE session_id=?")
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|r| Ok(serde_json::from_str::<Leg>(r.get::<&str, _>("data"))?))
            .collect()
    }
    /// Reserves before any network effect. A unique key and persistent global smoke
    /// budget ensure a lost response or restart never grants another authorized dial.
    pub async fn reserve(
        &self,
        session: &str,
        key: &str,
        leg: &Leg,
        mode: ExecutionMode,
    ) -> anyhow::Result<(Leg, bool)> {
        let mut tx = self.pool.begin().await?;
        if let Some(row) = sqlx::query("SELECT data FROM actions WHERE idempotency_key=?")
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?
        {
            return Ok((serde_json::from_str(row.get::<&str, _>("data"))?, false));
        }
        if leg.kind == "probe" && mode == ExecutionMode::Live {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM actions WHERE kind='probe' AND mode='live'",
            )
            .fetch_one(&mut *tx)
            .await?;
            anyhow::ensure!(
                count == 0,
                "The single authorized smoke-test attempt is already reserved; reconcile it before requesting new authorization"
            );
        }
        if leg.kind == "fixture_caller" && mode == ExecutionMode::Live {
            crate::audio_fixture::check_budget(&mut tx).await?;
        }
        let mode = if mode == ExecutionMode::Live {
            "live"
        } else {
            "replay"
        };
        sqlx::query("INSERT INTO actions VALUES(?,?,?,?,?,?)")
            .bind(&leg.action_id)
            .bind(session)
            .bind(key)
            .bind(&leg.kind)
            .bind(mode)
            .bind(serde_json::to_string(leg)?)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok((leg.clone(), true))
    }
    pub async fn append(
        &self,
        session: Option<&Session>,
        role: &str,
        kind: &str,
        payload: serde_json::Value,
        mode: ExecutionMode,
    ) -> anyhow::Result<Event> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut event = Event {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session.map(|s| s.id.clone()),
            leg_role: role.into(),
            occurred_at: now.clone(),
            received_at: now,
            sequence: 0,
            policy_epoch: session.map_or(0, |s| s.policy_epoch),
            execution_mode: mode,
            data_origin: "synthetic".into(),
            kind: kind.into(),
            payload,
        };
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query("INSERT INTO events(data) VALUES('')")
            .execute(&mut *tx)
            .await?;
        event.sequence = result.last_insert_rowid();
        sqlx::query("UPDATE events SET data=? WHERE sequence=?")
            .bind(serde_json::to_string(&event)?)
            .bind(event.sequence)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let _ = self.bus.send(event.clone());
        Ok(event)
    }
    pub async fn events_after(&self, sequence: i64) -> anyhow::Result<Vec<Event>> {
        let rows =
            sqlx::query("SELECT data FROM events WHERE sequence>? ORDER BY sequence LIMIT 1000")
                .bind(sequence)
                .fetch_all(&self.pool)
                .await?;
        rows.iter()
            .map(|r| Ok(serde_json::from_str::<Event>(r.get::<&str, _>("data"))?))
            .collect()
    }
    pub async fn set_metadata(&self, key: &str, value: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO metadata VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn metadata(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?)
    }
}
