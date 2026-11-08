//! Durable, scoped reputation of presented caller numbers; never identity proof.
use crate::{calls::state::Session, config::ExecutionMode, events::Event, store::Store};
use chrono::Duration;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::{Row, SqliteConnection};
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Demo,
    Production,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumberIdentity {
    pub number_key: String,
    pub display_suffix: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub number_key: String,
    pub display_suffix: String,
    pub scope: Scope,
    pub execution_mode: ExecutionMode,
    pub level: String,
    pub household_count: usize,
    pub demo_household_count: usize,
    pub verified_independent_households: usize,
    pub participation_group_count: usize,
    pub corroborating_household_count: usize,
    pub independence: String,
    pub first_reported_at: String,
    pub last_reported_at: String,
    pub expires_at: String,
    pub correction: String,
    pub version: u64,
    /// Stable taxonomy identifiers, not IDs of published/matched capsules.
    pub tactic_ids: Vec<String>,
    #[serde(default)]
    pub scam_categories: Vec<String>,
    #[serde(default = "taxonomy_identifier_kind")]
    pub tactic_identifier_kind: String,
    #[serde(default)]
    pub matched_capsule_ids: Vec<String>,
    pub explanation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportOutcome {
    pub report_id: String,
    pub created: bool,
    pub reputation: Snapshot,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LookupOutcome {
    pub reputation: Option<Snapshot>,
    pub receipt: Event,
}
#[derive(Clone)]
pub struct ReputationService {
    store: Store,
    key: Arc<Vec<u8>>,
    scope: Scope,
    now: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
}
impl ReputationService {
    pub fn new(store: Store, key: &str, scope: Scope) -> anyhow::Result<Self> {
        anyhow::ensure!(
            key.len() >= 32,
            "REPUTATION_HMAC_KEY must contain at least 32 bytes"
        );
        Ok(Self {
            store,
            key: Arc::new(key.as_bytes().to_vec()),
            scope,
            now: Arc::new(Utc::now),
        })
    }
    /// Replay is disabled unless an inert test explicitly injects a service.
    pub fn configured(
        store: Store,
        mode: ExecutionMode,
        key: Option<&str>,
        scope: Option<&str>,
    ) -> anyhow::Result<Option<Self>> {
        if mode == ExecutionMode::Replay {
            return Ok(None);
        }
        let Some(key) = key.filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let scope = match scope {
            Some("demo") => Scope::Demo,
            Some("production") => Scope::Production,
            _ => anyhow::bail!("REPUTATION_SCOPE must explicitly be demo or production"),
        };
        Ok(Some(Self::new(store, key, scope)?))
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn with_clock(mut self, clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>) -> Self {
        self.now = clock;
        self
    }
    pub async fn capture_identity(
        &self,
        session: &Session,
        from: Option<&str>,
    ) -> anyhow::Result<Option<NumberIdentity>> {
        let identity = from.and_then(presented_number).map(|value| {
            let mut mac =
                Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts any key length");
            mac.update(b"firstring:number-reputation:v1\0");
            mac.update(scope_name(self.scope).as_bytes());
            mac.update(b"\0");
            mac.update(value.as_bytes());
            NumberIdentity {
                number_key: mac
                    .finalize()
                    .into_bytes()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
                display_suffix: value[value.len() - 2..].into(),
            }
        });
        let mut tx = self.store.pool.begin().await?;
        // Refuse silent key changes across restarts: that would split the same
        // presented number's history and make prior corrections invisible.
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("valid HMAC key");
        mac.update(b"firstring:reputation:key-check:v1");
        let fingerprint: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        sqlx::query(
            "INSERT OR IGNORE INTO metadata(key,value) VALUES('reputation_hmac_fingerprint_v1',?)",
        )
        .bind(&fingerprint)
        .execute(&mut *tx)
        .await?;
        let stored: String = sqlx::query_scalar(
            "SELECT value FROM metadata WHERE key='reputation_hmac_fingerprint_v1'",
        )
        .fetch_one(&mut *tx)
        .await?;
        anyhow::ensure!(
            stored == fingerprint,
            "Reputation key changed; explicit migration required"
        );
        let number_key = identity.as_ref().map(|i| i.number_key.as_str());
        let suffix = identity.as_ref().map(|i| i.display_suffix.as_str());
        let mut events = vec![];
        if let Some(row) = sqlx::query(
            "SELECT scope,mode,number_key FROM reputation_identities WHERE session_id=?",
        )
        .bind(&session.id)
        .fetch_optional(&mut *tx)
        .await?
        {
            anyhow::ensure!(
                row.get::<String, _>("scope") == scope_name(self.scope)
                    && row.get::<String, _>("mode") == mode_name(session.execution_mode)
                    && row.get::<Option<String>, _>("number_key").as_deref() == number_key,
                "Session number identity is immutable"
            );
        } else {
            sqlx::query("INSERT INTO reputation_identities(session_id,scope,mode,number_key,display_suffix) VALUES(?,?,?,?,?)").bind(&session.id).bind(scope_name(self.scope)).bind(mode_name(session.execution_mode)).bind(number_key).bind(suffix).execute(&mut *tx).await?;
        }
        if events.is_empty() {
            let recorded:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM events WHERE json_extract(data,'$.type')='reputation.identity.captured' AND json_extract(data,'$.session_id')=?)").bind(&session.id).fetch_one(&mut *tx).await?;
            if !recorded {
                events.push(self.event(&mut tx,Some(session),session.execution_mode,"reputation.identity.captured",json!({"scope":self.scope,"has_shared_key":identity.is_some(),"presented_form":if identity.is_none(){"unavailable"}else if from.is_some_and(crate::config::is_e164){"e164"}else{"sip"},"identity_verified":false}),(self.now)()).await?);
            }
        }
        tx.commit().await?;
        self.broadcast(events);
        Ok(identity)
    }
    pub async fn report_protective(
        &self,
        session: &Session,
        source: &Event,
    ) -> anyhow::Result<Option<ReportOutcome>> {
        validate_protection(session, source)?;
        let now = (self.now)();
        let mut tx = self.store.pool.begin().await?;
        let event: Option<String> = sqlx::query_scalar("SELECT data FROM events WHERE sequence=?")
            .bind(source.sequence)
            .fetch_optional(&mut *tx)
            .await?;
        let durable: Event = serde_json::from_str(
            &event.ok_or_else(|| anyhow::anyhow!("Missing durable protective source event"))?,
        )?;
        anyhow::ensure!(
            serde_json::to_value(&durable)? == serde_json::to_value(source)?,
            "Protective source event mismatch"
        );
        let stored: Option<String> = sqlx::query_scalar("SELECT data FROM sessions WHERE id=?")
            .bind(&session.id)
            .fetch_optional(&mut *tx)
            .await?;
        let persisted: Session = serde_json::from_str(
            &stored.ok_or_else(|| anyhow::anyhow!("Protective session is not durable"))?,
        )?;
        validate_protection(&persisted, source)?;
        anyhow::ensure!(
            persisted.turn_version == session.turn_version
                && persisted.policy_epoch == session.policy_epoch
                && persisted.household_id == session.household_id,
            "Protective session changed"
        );
        let Some(identity) = self.identity(&mut tx, session).await? else {
            tx.commit().await?;
            return Ok(None);
        };
        let id = self.entry_id(session.execution_mode, &identity.number_key);
        if let Some(report_id) = sqlx::query_scalar::<_, String>(
            "SELECT report_id FROM reputation_reports WHERE session_id=?",
        )
        .bind(&session.id)
        .fetch_optional(&mut *tx)
        .await?
        {
            let (snapshot, events) = self
                .refresh(&mut tx, &id, session.execution_mode, now)
                .await?;
            tx.commit().await?;
            self.broadcast(events);
            return Ok(Some(ReportOutcome {
                report_id,
                created: false,
                reputation: snapshot,
            }));
        }
        // Derive from the durable accepted state, never caller-supplied report
        // fields or a mutable in-memory argument. This is a category, not a
        // newly published capsule or a lookup/match receipt.
        let category = current_tactic_category(&persisted)
            .ok_or_else(|| anyhow::anyhow!("No accepted current tactic category"))?;
        let tactic_id = categorical_tactic_id(category);
        let prior = self.load(&mut tx, &id).await?;
        let report_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO reputation_reports(report_id,entry_id,session_id,household_id,participation_group,source_event_id,source_event_sequence,source_turn_version,source_policy_epoch,reported_at,expires_at,retracted,scam_category,tactic_id) VALUES(?,?,?,?,?,?,?,?,?,?,?,0,?,?)")
            .bind(&report_id).bind(&id).bind(&session.id).bind(&session.household_id).bind("local_operator")
            .bind(&source.id).bind(source.sequence).bind(session.turn_version as i64).bind(session.policy_epoch as i64)
            .bind(now.to_rfc3339()).bind((now+Duration::days(14)).to_rfc3339()).bind(category).bind(&tactic_id).execute(&mut *tx).await?;
        let mut snapshot = prior.clone().unwrap_or_else(|| Snapshot {
            number_key: identity.number_key,
            display_suffix: identity.display_suffix,
            scope: self.scope,
            execution_mode: session.execution_mode,
            level: "none".into(),
            household_count: 0,
            demo_household_count: 0,
            verified_independent_households: 0,
            participation_group_count: 0,
            corroborating_household_count: 0,
            independence: if self.scope == Scope::Demo {
                "simulated_profiles"
            } else {
                "single_operator_unverified"
            }
            .into(),
            first_reported_at: now.to_rfc3339(),
            last_reported_at: now.to_rfc3339(),
            expires_at: (now + Duration::days(14)).to_rfc3339(),
            correction: "none".into(),
            version: 0,
            tactic_ids: vec![],
            scam_categories: vec![],
            tactic_identifier_kind: taxonomy_identifier_kind(),
            matched_capsule_ids: vec![],
            explanation: String::new(),
        });
        snapshot.last_reported_at = now.to_rfc3339();
        snapshot.expires_at = (now + Duration::days(14)).to_rfc3339();
        if snapshot.correction == "removed" {
            snapshot.correction = "none".into();
        }
        snapshot.version += 1;
        self.recalculate(&mut tx, &id, &mut snapshot, now).await?;
        self.persist(&mut tx, &id, &snapshot).await?;
        let event=self.event(&mut tx,Some(session),session.execution_mode,"reputation.reported",json!({"number_key":snapshot.number_key,"scope":self.scope,"level":snapshot.level,"version":snapshot.version,"household_count":snapshot.household_count,"demo_household_count":snapshot.demo_household_count,"verified_independent_households":0,"report_id":report_id,"source_event_id":source.id,"source_event_sequence":source.sequence,"source_turn_version":session.turn_version,"source_policy_epoch":session.policy_epoch,"scam_category":category,"tactic_id":tactic_id,"tactic_identifier_kind":taxonomy_identifier_kind(),"matched_capsule_ids":[]}),now).await?;
        self.audit(&mut tx,&id,&snapshot,&event,"report",json!({"report_id":report_id,"source_event_id":source.id,"source_event_sequence":source.sequence,"scam_category":category,"tactic_id":tactic_id,"tactic_identifier_kind":taxonomy_identifier_kind()})).await?;
        let mut events = vec![event];
        let before = prior.as_ref().map_or("none", |s| s.level.as_str());
        if before != snapshot.level {
            events.push(
                self.level_event(&mut tx, &snapshot, before, "report", now)
                    .await?,
            );
        }
        tx.commit().await?;
        self.broadcast(events);
        Ok(Some(ReportOutcome {
            report_id,
            created: true,
            reputation: snapshot,
        }))
    }
    pub async fn lookup_for_session(&self, session: &Session) -> anyhow::Result<LookupOutcome> {
        let now = (self.now)();
        let mut tx = self.store.pool.begin().await?;
        let identity = self.identity(&mut tx, session).await?;
        let mut events = vec![];
        let snapshot = if let Some(identity) = identity {
            let id = self.entry_id(session.execution_mode, &identity.number_key);
            if self.load(&mut tx, &id).await?.is_some() {
                let (s, e) = self
                    .refresh(&mut tx, &id, session.execution_mode, now)
                    .await?;
                events.extend(e);
                Some(s)
            } else {
                None
            }
        } else {
            None
        };
        let receipt=self.event(&mut tx,Some(session),session.execution_mode,"reputation.lookup",json!({"scope":self.scope,"household_id":session.household_id,"number_key":snapshot.as_ref().map(|s|&s.number_key),"read_version":snapshot.as_ref().map(|s|s.version),"level":snapshot.as_ref().map_or("none",|s|s.level.as_str()),"source_household_count":snapshot.as_ref().map_or(0,|s|s.household_count),"demo_household_count":snapshot.as_ref().map_or(0,|s|s.demo_household_count),"verified_independent_households":0,"permitted_use":"screening_context_only","identity_verified":false}),now).await?;
        sqlx::query("INSERT INTO reputation_lookups(receipt_id,session_id,data) VALUES(?,?,?)")
            .bind(&receipt.id)
            .bind(&session.id)
            .bind(serde_json::to_string(&receipt)?)
            .execute(&mut *tx)
            .await?;
        events.push(receipt.clone());
        tx.commit().await?;
        self.broadcast(events);
        Ok(LookupOutcome {
            reputation: snapshot,
            receipt,
        })
    }
    /// Authenticated console projection. Reading this does not claim a model used it.
    pub async fn session_view(&self, session: &Session) -> anyhow::Result<Value> {
        let mut tx = self.store.pool.begin().await?;
        let identity = self.identity(&mut tx, session).await?;
        let mut events = vec![];
        let (snapshot, reports) = if let Some(identity) = &identity {
            let id = self.entry_id(session.execution_mode, &identity.number_key);
            if self.load(&mut tx, &id).await?.is_some() {
                let (s, e) = self
                    .refresh(&mut tx, &id, session.execution_mode, (self.now)())
                    .await?;
                events = e;
                let reports:i64=sqlx::query_scalar("SELECT count(*) FROM reputation_reports WHERE entry_id=? AND retracted=0 AND expires_at>?").bind(id).bind((self.now)().to_rfc3339()).fetch_one(&mut *tx).await?;
                (Some(s), reports)
            } else {
                (None, 0)
            }
        } else {
            (None, 0)
        };
        tx.commit().await?;
        self.broadcast(events);
        let level = snapshot.as_ref().map_or("none", |s| s.level.as_str());
        let label = if let Some(identity) = identity {
            format!("Presented number ending {}", identity.display_suffix)
        } else {
            "Presented number unavailable".into()
        };
        let band_label = if level == "none" {
            "No active reports".into()
        } else if self.scope == Scope::Demo {
            format!("Demo {level}; simulated profiles, no verified independence")
        } else {
            format!("{level}; one unverified participation group")
        };
        Ok(
            json!({"version":snapshot.as_ref().map_or(0,|s|s.version),"scope":scope_name(self.scope),"number_label":label,"band":if level=="none"{"no_reports"}else{level},"band_label":band_label,"explanation":snapshot.as_ref().map_or("No active reported-number history. Absence of reports does not prove identity or safety.",|s|s.explanation.as_str()),"independent_households":0,"verified_independent_households":0,"demo_household_count":snapshot.as_ref().map_or(0,|s|s.demo_household_count),"reports":reports,"first_reported_at":snapshot.as_ref().map(|s|&s.first_reported_at),"last_reported_at":snapshot.as_ref().map(|s|&s.last_reported_at),"expires_at":snapshot.as_ref().map(|s|&s.expires_at),"correction":{"state":snapshot.as_ref().map_or("none",|s|s.correction.as_str())},"related_tactic_ids":snapshot.as_ref().map_or_else(Vec::new,|s|s.tactic_ids.clone()),"scam_categories":snapshot.as_ref().map_or_else(Vec::new,|s|s.scam_categories.clone()),"tactic_identifier_kind":taxonomy_identifier_kind(),"matched_capsule_ids":[],"identity_verified":false}),
        )
    }
    pub async fn list(&self, mode: ExecutionMode, limit: usize) -> anyhow::Result<Vec<Snapshot>> {
        let ids:Vec<String>=sqlx::query_scalar("SELECT entry_id FROM number_reputation WHERE scope=? AND mode=? ORDER BY updated_at DESC LIMIT ?").bind(scope_name(self.scope)).bind(mode_name(mode)).bind(limit.clamp(1,100) as i64).fetch_all(&self.store.pool).await?;
        let mut snapshots = vec![];
        for id in ids {
            let mut tx = self.store.pool.begin().await?;
            let (s, e) = self.refresh(&mut tx, &id, mode, (self.now)()).await?;
            tx.commit().await?;
            self.broadcast(e);
            snapshots.push(s);
        }
        Ok(snapshots)
    }
    /// Presented callers with the calls they made and their current reputation.
    /// Read-only: identities are HMAC keys with a two-digit display suffix.
    pub async fn callers(&self, mode: ExecutionMode) -> anyhow::Result<Vec<Value>> {
        let rows = sqlx::query("SELECT session_id,number_key,display_suffix FROM reputation_identities WHERE scope=? AND mode=? AND number_key IS NOT NULL ORDER BY rowid")
            .bind(scope_name(self.scope))
            .bind(mode_name(mode))
            .fetch_all(&self.store.pool)
            .await?;
        let mut order: Vec<String> = vec![];
        let mut groups: std::collections::HashMap<String, (String, Vec<String>)> =
            std::collections::HashMap::new();
        for row in rows {
            let key: String = row.get("number_key");
            let suffix: Option<String> = row.get("display_suffix");
            let entry = groups.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                (suffix.unwrap_or_default(), vec![])
            });
            entry.1.push(row.get("session_id"));
        }
        let now = (self.now)().to_rfc3339();
        let mut items = vec![];
        for key in order {
            let (suffix, sessions) = groups.remove(&key).unwrap_or_default();
            let mut conn = self.store.pool.acquire().await?;
            let snapshot = self.load(&mut conn, &self.entry_id(mode, &key)).await?;
            let reports: i64 = sqlx::query_scalar("SELECT count(*) FROM reputation_reports WHERE entry_id=? AND retracted=0 AND expires_at>?")
                .bind(self.entry_id(mode, &key))
                .bind(&now)
                .fetch_one(&mut *conn)
                .await?;
            let reputation = snapshot.map(|s| json!({"level":s.level,"reports":reports,"scam_categories":s.scam_categories,"tactic_ids":s.tactic_ids,"first_reported_at":s.first_reported_at,"last_reported_at":s.last_reported_at,"expires_at":s.expires_at,"correction":s.correction,"demo_household_count":s.demo_household_count,"verified_independent_households":s.verified_independent_households,"version":s.version,"explanation":s.explanation}));
            items.push(json!({"number_key":key,"display_suffix":suffix,"session_ids":sessions,"reputation":reputation,"identity_verified":false}));
        }
        Ok(items)
    }
    pub async fn detail(&self, mode: ExecutionMode, key: &str) -> anyhow::Result<Option<Value>> {
        let id = self.entry_id(mode, key);
        let mut tx = self.store.pool.begin().await?;
        if self.load(&mut tx, &id).await?.is_none() {
            return Ok(None);
        }
        let (s, e) = self.refresh(&mut tx, &id, mode, (self.now)()).await?;
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT data FROM reputation_audit WHERE entry_id=? ORDER BY version",
        )
        .bind(&id)
        .fetch_all(&mut *tx)
        .await?;
        let audit: Vec<Value> = rows
            .iter()
            .map(|s| serde_json::from_str(s))
            .collect::<Result<_, _>>()?;
        let report_rows=sqlx::query("SELECT report_id,household_id,reported_at,expires_at,retracted,source_event_id,source_turn_version,scam_category,tactic_id FROM reputation_reports WHERE entry_id=? ORDER BY reported_at DESC,report_id LIMIT 100")
            .bind(&id).fetch_all(&mut *tx).await?;
        let reports:Vec<Value>=report_rows.iter().map(|row|json!({"report_id":row.get::<String,_>("report_id"),"household_id":row.get::<String,_>("household_id"),"created_at":row.get::<String,_>("reported_at"),"expires_at":row.get::<String,_>("expires_at"),"retracted":row.get::<i64,_>("retracted")!=0,"source_event_id":row.get::<String,_>("source_event_id"),"source_turn_version":row.get::<i64,_>("source_turn_version"),"scam_category":row.get::<Option<String>,_>("scam_category"),"tactic_id":row.get::<Option<String>,_>("tactic_id"),"tactic_identifier_kind":taxonomy_identifier_kind(),"matched_capsule_ids":[]})).collect();
        tx.commit().await?;
        self.broadcast(e);
        Ok(Some(
            json!({"reputation":s,"audit":audit,"reports":reports}),
        ))
    }
    pub async fn request_correction(
        &self,
        mode: ExecutionMode,
        key: &str,
        reason: &str,
        version: u64,
    ) -> anyhow::Result<Snapshot> {
        anyhow::ensure!(
            matches!(reason, "mistaken" | "spoofed" | "legitimate_business"),
            "Unknown correction reason"
        );
        self.correct(mode, key, Some(reason), "requested", version)
            .await
    }
    pub async fn resolve_correction(
        &self,
        mode: ExecutionMode,
        key: &str,
        outcome: &str,
        version: u64,
    ) -> anyhow::Result<Snapshot> {
        anyhow::ensure!(
            matches!(outcome, "upheld" | "removed"),
            "Unknown correction resolution"
        );
        self.correct(mode, key, None, outcome, version).await
    }
    async fn correct(
        &self,
        mode: ExecutionMode,
        key: &str,
        reason: Option<&str>,
        outcome: &str,
        version: u64,
    ) -> anyhow::Result<Snapshot> {
        let now = (self.now)();
        let id = self.entry_id(mode, key);
        let mut tx = self.store.pool.begin().await?;
        let mut s = self
            .load(&mut tx, &id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Unknown reputation key"))?;
        anyhow::ensure!(
            s.version == version,
            "Reputation version changed; refresh before correction"
        );
        anyhow::ensure!(
            outcome == "requested" || s.correction == "requested",
            "Correction must be requested before resolution"
        );
        let before = s.level.clone();
        s.correction = outcome.into();
        s.version += 1;
        if outcome == "removed" {
            sqlx::query("UPDATE reputation_reports SET retracted=1 WHERE entry_id=?")
                .bind(&id)
                .execute(&mut *tx)
                .await?;
        }
        self.recalculate(&mut tx, &id, &mut s, now).await?;
        self.persist(&mut tx, &id, &s).await?;
        let event=self.event(&mut tx,None,mode,"reputation.correction",json!({"number_key":key,"scope":self.scope,"state":outcome,"version":s.version,"reason":reason,"level":s.level}),now).await?;
        self.audit(
            &mut tx,
            &id,
            &s,
            &event,
            "correction",
            json!({"state":outcome,"reason":reason}),
        )
        .await?;
        let mut events = vec![event];
        if before != s.level {
            events.push(
                self.level_event(&mut tx, &s, &before, "corrected", now)
                    .await?,
            );
        }
        tx.commit().await?;
        self.broadcast(events);
        Ok(s)
    }
    fn entry_id(&self, mode: ExecutionMode, key: &str) -> String {
        format!("{}:{}:{key}", scope_name(self.scope), mode_name(mode))
    }
    async fn identity(
        &self,
        conn: &mut SqliteConnection,
        s: &Session,
    ) -> anyhow::Result<Option<NumberIdentity>> {
        let row=sqlx::query("SELECT number_key,display_suffix FROM reputation_identities WHERE session_id=? AND scope=? AND mode=?").bind(&s.id).bind(scope_name(self.scope)).bind(mode_name(s.execution_mode)).fetch_optional(conn).await?;
        Ok(row.and_then(|r| {
            r.get::<Option<String>, _>("number_key")
                .map(|number_key| NumberIdentity {
                    number_key,
                    display_suffix: r.get("display_suffix"),
                })
        }))
    }
    async fn load(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
    ) -> anyhow::Result<Option<Snapshot>> {
        let data: Option<String> =
            sqlx::query_scalar("SELECT data FROM number_reputation WHERE entry_id=?")
                .bind(id)
                .fetch_optional(conn)
                .await?;
        data.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }
    async fn recalculate(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
        s: &mut Snapshot,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let rows=sqlx::query("SELECT household_id,participation_group,reported_at,scam_category,tactic_id FROM reputation_reports WHERE entry_id=? AND retracted=0 AND expires_at>?").bind(id).bind(now.to_rfc3339()).fetch_all(conn).await?;
        let mut households = BTreeSet::new();
        let mut recent = BTreeSet::new();
        let mut groups = BTreeSet::new();
        let mut recent_groups = BTreeSet::new();
        let mut categories = BTreeSet::new();
        let mut tactic_ids = BTreeSet::new();
        for row in rows {
            if let (Some(category), Some(tactic_id)) = (
                row.get::<Option<String>, _>("scam_category"),
                row.get::<Option<String>, _>("tactic_id"),
            ) && serde_json::from_value::<crate::tools::TacticKind>(json!(&category)).is_ok()
                && tactic_id == categorical_tactic_id(&category)
            {
                categories.insert(category);
                tactic_ids.insert(tactic_id);
            }
            let h: String = row.get("household_id");
            let g: String = row.get("participation_group");
            let at = DateTime::parse_from_rfc3339(row.get::<&str, _>("reported_at"))?;
            if at > now - Duration::days(7) {
                recent.insert(h.clone());
                recent_groups.insert(g.clone());
            }
            households.insert(h);
            groups.insert(g);
        }
        s.scam_categories = categories.into_iter().collect();
        s.tactic_ids = tactic_ids.into_iter().collect();
        s.tactic_identifier_kind = taxonomy_identifier_kind();
        s.matched_capsule_ids.clear();
        s.demo_household_count = if self.scope == Scope::Demo {
            households.len()
        } else {
            0
        };
        s.verified_independent_households = 0;
        s.participation_group_count = groups.len();
        s.household_count = if self.scope == Scope::Demo {
            households.len()
        } else {
            groups.len()
        };
        s.corroborating_household_count = if self.scope == Scope::Demo {
            recent.len()
        } else {
            recent_groups.len()
        };
        let level = if s.household_count == 0 {
            "none"
        } else if s.corroborating_household_count >= 3 {
            "warning"
        } else if s.corroborating_household_count >= 2 {
            "corroborated"
        } else {
            "reported"
        };
        if s.correction != "requested" || rank(level) <= rank(&s.level) {
            s.level = level.into();
        }
        s.explanation=if self.scope==Scope::Demo {"Demo profile reports simulate independent households; verified independence is zero. Presented caller ID can be spoofed. No automatic blocking."}else{"Reports are from one local participation group, not verified independent households. Presented caller ID can be spoofed. No automatic blocking."}.into();
        s.explanation.push_str(" Tactic IDs identify categories, not published capsule matches; legacy reports may have no category.");
        Ok(())
    }
    async fn refresh(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
        mode: ExecutionMode,
        now: DateTime<Utc>,
    ) -> anyhow::Result<(Snapshot, Vec<Event>)> {
        let mut s = self
            .load(conn, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Unknown reputation key"))?;
        let before = serde_json::to_value(&s)?;
        let previous = s.level.clone();
        self.recalculate(conn, id, &mut s, now).await?;
        if serde_json::to_value(&s)? == before {
            return Ok((s, vec![]));
        }
        let expired = previous != s.level
            || before["household_count"] != json!(s.household_count)
            || before["corroborating_household_count"] != json!(s.corroborating_household_count)
            || before["tactic_ids"].as_array().is_some_and(|ids| {
                ids.iter().any(|id| {
                    id.as_str()
                        .is_some_and(|id| !s.tactic_ids.iter().any(|active| active == id))
                })
            });
        let reason = if expired {
            "expired"
        } else {
            "metadata_refreshed"
        };
        s.version += 1;
        self.persist(conn, id, &s).await?;
        let event=self.event(conn,None,mode,"reputation.level_changed",json!({"number_key":s.number_key,"scope":self.scope,"from":previous,"to":s.level,"version":s.version,"reason":reason,"household_count":s.household_count}),now).await?;
        self.audit(
            conn,
            id,
            &s,
            &event,
            if expired {
                "expiry"
            } else {
                "metadata_refresh"
            },
            json!({"from":previous,"to":s.level}),
        )
        .await?;
        Ok((s, vec![event]))
    }
    async fn persist(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
        s: &Snapshot,
    ) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO number_reputation(entry_id,scope,mode,data,updated_at) VALUES(?,?,?,?,?) ON CONFLICT(entry_id) DO UPDATE SET data=excluded.data,updated_at=excluded.updated_at")
            .bind(id).bind(scope_name(self.scope)).bind(mode_name(s.execution_mode)).bind(serde_json::to_string(s)?).bind((self.now)().to_rfc3339()).execute(conn).await?;
        Ok(())
    }
    async fn audit(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
        s: &Snapshot,
        event: &Event,
        action: &str,
        details: Value,
    ) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO reputation_audit(entry_id,version,data) VALUES(?,?,?)").bind(id).bind(s.version as i64).bind(json!({"version":s.version,"event_id":event.id,"event_sequence":event.sequence,"at":event.occurred_at,"action":action,"details":details}).to_string()).execute(conn).await?;
        Ok(())
    }
    async fn level_event(
        &self,
        conn: &mut SqliteConnection,
        s: &Snapshot,
        from: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Event> {
        self.event(conn,None,s.execution_mode,"reputation.level_changed",json!({"number_key":s.number_key,"scope":self.scope,"from":from,"to":s.level,"version":s.version,"reason":reason}),now).await
    }
    #[allow(clippy::too_many_arguments)]
    async fn event(
        &self,
        conn: &mut SqliteConnection,
        session: Option<&Session>,
        mode: ExecutionMode,
        kind: &str,
        payload: Value,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Event> {
        let mut e = Event {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session.map(|s| s.id.clone()),
            leg_role: "system".into(),
            occurred_at: now.to_rfc3339(),
            received_at: now.to_rfc3339(),
            sequence: 0,
            policy_epoch: session.map_or(0, |s| s.policy_epoch),
            execution_mode: mode,
            data_origin: session
                .map_or("synthetic", |s| s.data_origin.as_str())
                .into(),
            kind: kind.into(),
            payload,
        };
        e.sequence = sqlx::query("INSERT INTO events(data) VALUES('')")
            .execute(&mut *conn)
            .await?
            .last_insert_rowid();
        sqlx::query("UPDATE events SET data=? WHERE sequence=?")
            .bind(serde_json::to_string(&e)?)
            .bind(e.sequence)
            .execute(conn)
            .await?;
        Ok(e)
    }
    fn broadcast(&self, events: Vec<Event>) {
        for event in events {
            let _ = self.store.bus.send(event);
        }
    }
}
/// Extract only an exact E.164 SIP user. The URI host is syntax-checked, not
/// trusted as identity. No percent decoding, display-name parsing or guessing.
fn presented_number(value: &str) -> Option<&str> {
    if crate::config::is_e164(value) {
        return Some(value);
    }
    if value.len() > 512 || !value.is_ascii() || value.chars().any(char::is_whitespace) {
        return None;
    }
    let uri = value
        .strip_prefix("sip:")
        .or_else(|| value.strip_prefix("sips:"))?;
    let (number, host) = uri.split_once('@')?;
    if !crate::config::is_e164(number) || host.is_empty() || host.contains(['@', '/', '?', '#']) {
        return None;
    }
    let mut pieces = host.split(';');
    let authority = pieces.next()?;
    for parameter in pieces {
        let mut pair = parameter.splitn(2, '=');
        let name = pair.next()?;
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
            return None;
        }
        if let Some(value) = pair.next()
            && (value.is_empty()
                || !value.bytes().all(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b':' | b'[' | b']')
                }))
        {
            return None;
        }
    }
    let parsed = url::Url::parse(&format!("https://{authority}/")).ok()?;
    if parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
    {
        return None;
    }
    Some(number)
}
fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Demo => "demo",
        Scope::Production => "production",
    }
}
fn mode_name(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Live => "live",
        ExecutionMode::Replay => "replay",
    }
}
fn rank(level: &str) -> u8 {
    match level {
        "reported" => 1,
        "corroborated" => 2,
        "warning" => 3,
        _ => 0,
    }
}
/// New reports require the latest accepted assessment of this turn, never a
/// historical sticky block or a model's unaccepted proposal.
pub fn current_protective_assessment(s: &Session) -> bool {
    if !s.blocked || s.assessed_version != s.turn_version {
        return false;
    }
    let Some(a) = s
        .assessments
        .iter()
        .rev()
        .find(|a| a["turn_version"].as_u64() == Some(s.turn_version))
    else {
        return false;
    };
    let risky = a["observation"]["speech_act"] == "request"
        && a["observation"]["direction"] == "caller_requests_from_sarah"
        && matches!(
            a["observation"]["requested_object"].as_str(),
            Some("authentication_secret" | "remote_access" | "payment_diversion")
        );
    let evidence =
        serde_json::from_value::<Vec<crate::tools::EvidenceRef>>(a["evidence_refs"].clone());
    risky && evidence.is_ok_and(|evidence| s.evidence_valid(&evidence, true).is_ok())
}
fn taxonomy_identifier_kind() -> String {
    "categorical_taxonomy_v1".into()
}
fn categorical_tactic_id(category: &str) -> String {
    format!("firstring:tactic:v1:{category}")
}
fn current_tactic_category(session: &Session) -> Option<&'static str> {
    if !current_protective_assessment(session) {
        return None;
    }
    let assessment = session
        .assessments
        .iter()
        .rev()
        .find(|a| a["turn_version"].as_u64() == Some(session.turn_version))?;
    use crate::tools::{RequestedObject, TacticKind};
    let object = serde_json::from_value::<RequestedObject>(
        assessment["observation"]["requested_object"].clone(),
    )
    .ok()?;
    let tactic = match object {
        RequestedObject::AuthenticationSecret => TacticKind::AuthenticationCodeExtraction,
        RequestedObject::RemoteAccess => TacticKind::RemoteAccessRequest,
        RequestedObject::PaymentDiversion => TacticKind::PaymentDiversion,
        _ => return None,
    };
    Some(match tactic {
        TacticKind::AuthenticationCodeExtraction => "authentication_code_extraction",
        TacticKind::RemoteAccessRequest => "remote_access_request",
        TacticKind::PaymentDiversion => "payment_diversion",
    })
}
fn validate_protection(s: &Session, event: &Event) -> anyhow::Result<()> {
    let m = s
        .message
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No saved protective message"))?;
    anyhow::ensure!(
        s.blocked
            && !s.bridge_committed
            && s.assessed_version == s.turn_version
            && m.turn_version == s.turn_version,
        "No current accepted protective outcome"
    );
    anyhow::ensure!(
        event.kind == "message.recorded"
            && event.session_id.as_deref() == Some(&s.id)
            && event.execution_mode == s.execution_mode
            && event.policy_epoch == s.policy_epoch
            && event.payload["turn_version"].as_u64() == Some(m.turn_version)
            && event.payload["recorded_at"].as_str() == Some(&m.recorded_at),
        "Protective source is not the saved current message"
    );
    anyhow::ensure!(
        current_protective_assessment(s),
        "No accepted current risky assessment"
    );
    Ok(())
}
pub async fn initialize(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS reputation_identities(session_id TEXT PRIMARY KEY,scope TEXT NOT NULL,mode TEXT NOT NULL,number_key TEXT,display_suffix TEXT);
CREATE TABLE IF NOT EXISTS number_reputation(entry_id TEXT PRIMARY KEY,scope TEXT NOT NULL,mode TEXT NOT NULL,data TEXT NOT NULL,updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS reputation_reports(report_id TEXT PRIMARY KEY,entry_id TEXT NOT NULL,session_id TEXT UNIQUE NOT NULL,household_id TEXT NOT NULL,participation_group TEXT NOT NULL,source_event_id TEXT NOT NULL,source_event_sequence INTEGER NOT NULL,source_turn_version INTEGER NOT NULL,source_policy_epoch INTEGER NOT NULL,reported_at TEXT NOT NULL,expires_at TEXT NOT NULL,retracted INTEGER NOT NULL DEFAULT 0,scam_category TEXT,tactic_id TEXT);
CREATE INDEX IF NOT EXISTS reputation_report_active ON reputation_reports(entry_id,retracted,expires_at);
CREATE TABLE IF NOT EXISTS reputation_audit(entry_id TEXT NOT NULL,version INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(entry_id,version));
CREATE TABLE IF NOT EXISTS reputation_lookups(receipt_id TEXT PRIMARY KEY,session_id TEXT NOT NULL,data TEXT NOT NULL);").execute(pool).await?;
    let mut tx = pool.begin().await?;
    let columns = sqlx::query("PRAGMA table_info(reputation_reports)")
        .fetch_all(&mut *tx)
        .await?;
    // Fixed identifiers only; rerunning the migration never resets a ledger,
    // version, correction or unknown legacy category.
    for column in ["scam_category", "tactic_id"] {
        if !columns
            .iter()
            .any(|row| row.get::<&str, _>("name") == column)
        {
            sqlx::query(&format!(
                "ALTER TABLE reputation_reports ADD COLUMN {column} TEXT"
            ))
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
