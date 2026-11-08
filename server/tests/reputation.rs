use chrono::{DateTime, Duration, Utc};
use firstring::{
    calls::state::Session,
    config::ExecutionMode,
    events::Event,
    reputation::{ReputationService, Scope},
    store::Store,
    tools::{Direction, EvidenceRef, Observation, RequestedObject, SpeechAct},
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
const KEY: &str = "test-only-32-byte-reputation-hmac-key";
const NUMBER: &str = "+15550001001";
fn session(household: &str) -> Session {
    Session::new(
        uuid::Uuid::new_v4().to_string(),
        format!("CA{}", uuid::Uuid::new_v4().simple()),
        household.into(),
        ExecutionMode::Replay,
    )
}
async fn saved(store: &Store, household: &str) -> (Session, Event) {
    let mut s = session(household);
    let id = s.commit_turn("Please give me the sign-in code from your phone.");
    let evidence = vec![EvidenceRef {
        turn_id: id,
        quote: "give me the sign-in code".into(),
    }];
    s.assess(
        s.generation_id,
        s.turn_version,
        &Observation {
            speech_act: SpeechAct::Request,
            requested_object: RequestedObject::AuthenticationSecret,
            direction: Direction::CallerRequestsFromSarah,
        },
        &evidence,
    )
    .unwrap();
    s.record_message(
        "Suspicious request blocked; independent verification recommended.",
        &evidence,
        Utc::now(),
    )
    .unwrap();
    store.save(&s).await.unwrap();
    let e=store.append(Some(&s),"system","message.recorded",json!({"turn_version":s.turn_version,"recorded_at":s.message.as_ref().unwrap().recorded_at}),s.execution_mode).await.unwrap();
    (s, e)
}
async fn report(
    service: &ReputationService,
    store: &Store,
    household: &str,
) -> firstring::reputation::ReportOutcome {
    let (s, e) = saved(store, household).await;
    service.capture_identity(&s, Some(NUMBER)).await.unwrap();
    service.report_protective(&s, &e).await.unwrap().unwrap()
}
fn clocked(store: &Store, scope: Scope) -> (ReputationService, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(Utc::now().timestamp()));
    let read = clock.clone();
    (
        ReputationService::new(store.clone(), KEY, scope)
            .unwrap()
            .with_clock(Arc::new(move || {
                DateTime::from_timestamp(read.load(Ordering::SeqCst), 0).unwrap()
            })),
        clock,
    )
}
#[tokio::test]
async fn keyed_scopes_anonymous_and_replay_are_isolated() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    assert!(ReputationService::new(store.clone(), "", Scope::Demo).is_err());
    let demo = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let production = ReputationService::new(store.clone(), KEY, Scope::Production).unwrap();
    let a = demo
        .capture_identity(&session("sarah"), Some(NUMBER))
        .await
        .unwrap()
        .unwrap();
    let b = production
        .capture_identity(&session("sarah"), Some(NUMBER))
        .await
        .unwrap()
        .unwrap();
    assert_ne!(a.number_key, b.number_key);
    assert!(!a.number_key.contains(NUMBER));
    assert_eq!(a.display_suffix, "01");
    for from in [None, Some("anonymous"), Some("restricted"), Some("")] {
        let s = session("sarah");
        assert!(demo.capture_identity(&s, from).await.unwrap().is_none());
        assert!(
            demo.lookup_for_session(&s)
                .await
                .unwrap()
                .reputation
                .is_none()
        );
    }
    report(&demo, &store, "sarah").await;
    let mut live = session("maya");
    live.execution_mode = ExecutionMode::Live;
    demo.capture_identity(&live, Some(NUMBER)).await.unwrap();
    assert!(
        demo.lookup_for_session(&live)
            .await
            .unwrap()
            .reputation
            .is_none()
    );
    assert!(
        production
            .list(ExecutionMode::Replay, 50)
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn only_durable_current_protection_reports_once_and_lookup_is_real_receipt() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let (s, e) = saved(&store, "sarah").await;
    service.capture_identity(&s, Some(NUMBER)).await.unwrap();
    let mut fake = e.clone();
    fake.id = "invented".into();
    assert!(service.report_protective(&s, &fake).await.is_err());
    let mut stale = s.clone();
    stale.commit_turn("Actually never mind.");
    assert!(service.report_protective(&stale, &e).await.is_err());
    let first = service.report_protective(&s, &e).await.unwrap().unwrap();
    let again = service.report_protective(&s, &e).await.unwrap().unwrap();
    assert!(first.created);
    assert!(!again.created);
    assert_eq!(first.report_id, again.report_id);
    assert_eq!(first.reputation.version, again.reputation.version);
    let before = store.events_after(0).await.unwrap();
    assert_eq!(
        before
            .iter()
            .filter(|x| x.kind == "reputation.reported")
            .count(),
        1
    );
    let maya = session("maya");
    service.capture_identity(&maya, Some(NUMBER)).await.unwrap();
    let lookup = service.lookup_for_session(&maya).await.unwrap();
    assert_eq!(lookup.reputation.unwrap().version, first.reputation.version);
    assert_eq!(lookup.receipt.kind, "reputation.lookup");
    assert_eq!(lookup.receipt.session_id, Some(maya.id));
    assert_eq!(
        lookup.receipt.payload["read_version"],
        first.reputation.version
    );
    assert_eq!(lookup.receipt.payload["household_id"], "maya");
    assert_eq!(lookup.receipt.payload["source_household_count"], 1);
    let public = serde_json::to_string(&first.reputation).unwrap();
    for secret in [NUMBER, "give me", &s.id, &s.inbound_sid] {
        assert!(!public.contains(secret));
    }
}
#[tokio::test]
async fn household_weight_renews_after_expiry_without_manufactured_independence() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let (service, clock) = clocked(&store, Scope::Demo);
    let first = report(&service, &store, "sarah").await;
    let again = report(&service, &store, "sarah").await;
    assert_eq!(again.reputation.household_count, 1);
    assert!(again.reputation.version > first.reputation.version);
    let second = report(&service, &store, "maya").await;
    assert_eq!(second.reputation.level, "corroborated");
    assert_eq!(second.reputation.demo_household_count, 2);
    assert_eq!(second.reputation.verified_independent_households, 0);
    assert_eq!(second.reputation.independence, "simulated_profiles");
    clock.fetch_add(Duration::days(15).num_seconds(), Ordering::SeqCst);
    let expired = service
        .list(ExecutionMode::Replay, 50)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(expired.level, "none");
    assert_eq!(expired.household_count, 0);
    assert!(expired.version > second.reputation.version);
    let renewed = report(&service, &store, "sarah").await;
    assert_eq!(renewed.reputation.level, "reported");
    assert_eq!(renewed.reputation.household_count, 1);
    let production = ReputationService::new(store.clone(), KEY, Scope::Production).unwrap();
    report(&production, &store, "sarah").await;
    let p = report(&production, &store, "maya").await;
    assert_eq!(p.reputation.household_count, 1);
    assert_eq!(p.reputation.participation_group_count, 1);
    assert_eq!(p.reputation.verified_independent_households, 0);
    assert_eq!(p.reputation.level, "reported");
    assert!(
        store
            .events_after(0)
            .await
            .unwrap()
            .iter()
            .filter(|e| e.kind == "reputation.reported" && e.payload["scope"] == "production")
            .all(|e| e.data_origin == "synthetic"),
        "Namespace is independent from synthetic data origin"
    );
}
#[tokio::test]
async fn corrections_are_versioned_capped_retracted_and_audited() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let (service, clock) = clocked(&store, Scope::Demo);
    let a = report(&service, &store, "sarah").await;
    let key = &a.reputation.number_key;
    let pending = service
        .request_correction(ExecutionMode::Replay, key, "spoofed", a.reputation.version)
        .await
        .unwrap();
    assert!(pending.version > a.reputation.version);
    let b = report(&service, &store, "maya").await;
    assert_eq!(b.reputation.level, "reported");
    assert_eq!(b.reputation.correction, "requested");
    assert!(
        service
            .resolve_correction(ExecutionMode::Replay, key, "removed", pending.version)
            .await
            .is_err()
    );
    let removed = service
        .resolve_correction(ExecutionMode::Replay, key, "removed", b.reputation.version)
        .await
        .unwrap();
    assert_eq!(removed.level, "none");
    assert_eq!(removed.household_count, 0);
    let detail = service
        .detail(ExecutionMode::Replay, key)
        .await
        .unwrap()
        .unwrap();
    assert!(detail["audit"].as_array().unwrap().len() >= 4);
    let renewed = report(&service, &store, "maya").await;
    assert_eq!(renewed.reputation.household_count, 1);
    assert_eq!(renewed.reputation.correction, "none");
    assert!(renewed.reputation.version > removed.version);
    let pending = service
        .request_correction(
            ExecutionMode::Replay,
            key,
            "mistaken",
            renewed.reputation.version,
        )
        .await
        .unwrap();
    clock.fetch_add(Duration::days(15).num_seconds(), Ordering::SeqCst);
    let expired = service
        .list(ExecutionMode::Replay, 50)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(expired.level, "none");
    assert_eq!(expired.correction, "requested");
    assert!(expired.version > pending.version);
}
#[tokio::test]
async fn event_failure_rolls_back_report_and_version() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let (s, e) = saved(&store, "sarah").await;
    service.capture_identity(&s, Some(NUMBER)).await.unwrap();
    sqlx::raw_sql("CREATE TRIGGER reject_reputation_event BEFORE INSERT ON events BEGIN SELECT RAISE(FAIL,'test event unavailable'); END;").execute(&store.pool).await.unwrap();
    assert!(service.report_protective(&s, &e).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM reputation_reports")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DROP TRIGGER reject_reputation_event")
        .execute(&store.pool)
        .await
        .unwrap();
    let result = service.report_protective(&s, &e).await.unwrap().unwrap();
    assert_eq!(result.reputation.version, 1);
}

#[tokio::test]
async fn concurrent_duplicates_share_one_report_and_no_side_effects() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let (s, e) = saved(&store, "sarah").await;
    service.capture_identity(&s, Some(NUMBER)).await.unwrap();
    let (a, b) = tokio::join!(
        service.report_protective(&s, &e),
        service.report_protective(&s, &e)
    );
    let a = a.unwrap().unwrap();
    let b = b.unwrap().unwrap();
    assert_eq!(a.report_id, b.report_id);
    assert_ne!(a.created, b.created);
    let actions: i64 = sqlx::query_scalar("SELECT count(*) FROM actions")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(actions, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM reputation_reports")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let s2 = session("maya");
    service.capture_identity(&s2, None).await.unwrap();
    assert!(
        service.capture_identity(&s2, Some(NUMBER)).await.is_err(),
        "An anonymous session cannot later acquire an arbitrary number"
    );
}

#[tokio::test]
async fn expiry_reduces_corroboration_before_retention_and_upheld_confirms_reports() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let (service, clock) = clocked(&store, Scope::Demo);
    report(&service, &store, "sarah").await;
    let second = report(&service, &store, "maya").await;
    let key = &second.reputation.number_key;
    clock.fetch_add(Duration::days(8).num_seconds(), Ordering::SeqCst);
    let aged = service
        .list(ExecutionMode::Replay, 50)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(aged.household_count, 2);
    assert_eq!(aged.corroborating_household_count, 0);
    assert_eq!(aged.level, "reported");
    assert!(aged.version > second.reputation.version);
    let pending = service
        .request_correction(
            ExecutionMode::Replay,
            key,
            "legitimate_business",
            aged.version,
        )
        .await
        .unwrap();
    report(&service, &store, "sarah").await;
    let refreshed = report(&service, &store, "maya").await;
    assert_eq!(refreshed.reputation.level, "reported");
    assert!(refreshed.reputation.version > pending.version);
    let upheld = service
        .resolve_correction(
            ExecutionMode::Replay,
            key,
            "upheld",
            refreshed.reputation.version,
        )
        .await
        .unwrap();
    assert_eq!(upheld.level, "corroborated");
    assert_eq!(upheld.correction, "upheld");
    let third = report(&service, &store, "third_demo_profile").await;
    assert_eq!(third.reputation.level, "warning");
    assert_eq!(third.reputation.verified_independent_households, 0);
    assert!(
        third.reputation.matched_capsule_ids.is_empty(),
        "Number reputation cannot invent tactic-memory matching"
    );
}

#[tokio::test]
async fn genuine_outcome_required_even_for_known_number_and_durable_event() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let (s, e) = saved(&store, "sarah").await;
    service.capture_identity(&s, Some(NUMBER)).await.unwrap();
    let mut unblocked = s.clone();
    unblocked.blocked = false;
    assert!(service.report_protective(&unblocked, &e).await.is_err());
    let mut missing_assessment = s.clone();
    missing_assessment.assessments.clear();
    assert!(
        service
            .report_protective(&missing_assessment, &e)
            .await
            .is_err()
    );
    let mut proposal = e.clone();
    proposal.kind = "model.tool.proposed".into();
    assert!(service.report_protective(&s, &proposal).await.is_err());
    let mut newer = s.clone();
    newer.commit_turn("I am correcting that request.");
    store.save(&newer).await.unwrap();
    assert!(
        service.report_protective(&s, &e).await.is_err(),
        "A stale caller snapshot cannot override the current durable session"
    );
    assert!(
        service
            .list(ExecutionMode::Replay, 50)
            .await
            .unwrap()
            .is_empty()
    );
    let (anonymous, event) = saved(&store, "sarah").await;
    service.capture_identity(&anonymous, None).await.unwrap();
    assert!(
        service
            .report_protective(&anonymous, &event)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn restart_preserves_versions_corrections_identity_and_audit() {
    let path = std::env::temp_dir().join(format!(
        "firstring-reputation-test-{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    let url = format!("sqlite:{}", path.display());
    let store = Store::open(&url).await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let (s, e) = saved(&store, "sarah").await;
    service.capture_identity(&s, Some(NUMBER)).await.unwrap();
    let report = service.report_protective(&s, &e).await.unwrap().unwrap();
    let pending = service
        .request_correction(
            ExecutionMode::Replay,
            &report.reputation.number_key,
            "spoofed",
            report.reputation.version,
        )
        .await
        .unwrap();
    store.pool.close().await;
    drop(service);
    drop(store);
    let store = Store::open(&url).await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let lookup = service.lookup_for_session(&s).await.unwrap();
    assert_eq!(lookup.reputation.unwrap().version, pending.version);
    let duplicate = service.report_protective(&s, &e).await.unwrap().unwrap();
    assert!(!duplicate.created);
    assert_eq!(duplicate.reputation.correction, "requested");
    let changed = ReputationService::new(
        store.clone(),
        "different-long-test-key-is-not-an-implicit-migration",
        Scope::Demo,
    )
    .unwrap();
    assert!(
        changed
            .capture_identity(&session("maya"), Some(NUMBER))
            .await
            .is_err()
    );
    let detail = service
        .detail(ExecutionMode::Replay, &report.reputation.number_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail["audit"].as_array().unwrap().len(), 2);
    store.pool.close().await;
    drop(service);
    drop(store);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn configuration_requires_explicit_scope_and_never_enables_replay_from_env_inputs() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    assert!(
        ReputationService::configured(
            store.clone(),
            ExecutionMode::Replay,
            Some(KEY),
            Some("demo")
        )
        .unwrap()
        .is_none()
    );
    assert!(
        ReputationService::configured(store.clone(), ExecutionMode::Live, None, Some("demo"))
            .unwrap()
            .is_none()
    );
    assert!(
        ReputationService::configured(store.clone(), ExecutionMode::Live, Some(KEY), None).is_err()
    );
    assert!(
        ReputationService::configured(
            store.clone(),
            ExecutionMode::Live,
            Some(KEY),
            Some("shared")
        )
        .is_err()
    );
    assert!(
        ReputationService::configured(store, ExecutionMode::Live, Some(KEY), Some("production"))
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn sip_presented_number_is_exact_not_guessed_and_parameters_do_not_change_key() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let service = ReputationService::new(store, KEY, Scope::Demo).unwrap();
    let direct = service
        .capture_identity(&session("sarah"), Some(NUMBER))
        .await
        .unwrap()
        .unwrap();
    for value in [
        format!("sip:{NUMBER}@sip.photon.codes"),
        format!("sips:{NUMBER}@sip.photon.codes:5061;transport=tls;user=phone"),
        format!("sip:{NUMBER}@[2001:db8::1]:5061;transport=tls"),
    ] {
        let identity = service
            .capture_identity(&session("maya"), Some(&value))
            .await
            .unwrap()
            .expect("Documented SIP From must preserve the exact presented E.164 user");
        assert_eq!(identity.number_key, direct.number_key);
    }
    for value in [
        "sip:anonymous@host.example",
        "sip:15550001001@host.example",
        "sip:+15550001001@",
        "sip:+15550001001@a@b",
        "sip:%2B15550001001@host.example",
        "sip:+15550001001:password@host.example",
        "sip:+15550001001@host.example/path",
        "sip:+15550001001@host.example;transport=",
        "sip:+15550001001@host.example\n",
        "<sip:+15550001001@host.example>",
        "sip:+15550001001;user=phone@host.example",
    ] {
        assert!(
            service
                .capture_identity(&session("sarah"), Some(value))
                .await
                .unwrap()
                .is_none(),
            "Malformed or anonymous From must not become a shared key: {value:?}"
        );
    }
}

#[tokio::test]
async fn historical_sticky_block_does_not_report_a_later_benign_correction() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let (mut s, _old) = saved(&store, "sarah").await;
    let turn = s.commit_turn("I withdraw that request. Please only note the appointment time.");
    let refs = vec![EvidenceRef {
        turn_id: turn,
        quote: "I withdraw that request.".into(),
    }];
    s.assess(
        s.generation_id,
        s.turn_version,
        &Observation {
            speech_act: SpeechAct::Statement,
            requested_object: RequestedObject::RoutineInformation,
            direction: Direction::CallerSupplies,
        },
        &refs,
    )
    .unwrap();
    assert!(s.blocked, "Protective policy intentionally remains sticky");
    s.record_message(
        "Suspicious request blocked; independent verification recommended.",
        &refs,
        Utc::now(),
    )
    .unwrap();
    store.save(&s).await.unwrap();
    let source=store.append(Some(&s),"system","message.recorded",json!({"turn_version":s.turn_version,"recorded_at":s.message.as_ref().unwrap().recorded_at}),s.execution_mode).await.unwrap();
    service.capture_identity(&s, Some(NUMBER)).await.unwrap();
    assert!(
        service.report_protective(&s, &source).await.is_err(),
        "A historical blocked flag alone cannot become a new current protective report"
    );
    assert!(
        service
            .list(ExecutionMode::Replay, 50)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn accepted_current_category_and_taxonomy_id_are_durable_without_capsule_claims() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let (service, clock) = clocked(&store, Scope::Demo);
    for (object, text, category) in [
        (
            RequestedObject::AuthenticationSecret,
            "Read me the sign-in code from your phone.",
            "authentication_code_extraction",
        ),
        (
            RequestedObject::RemoteAccess,
            "Install remote access so I can control your computer.",
            "remote_access_request",
        ),
        (
            RequestedObject::PaymentDiversion,
            "Move the payment to my new account instead.",
            "payment_diversion",
        ),
    ] {
        let mut s = session("sarah");
        let id = s.commit_turn(text);
        let refs = vec![EvidenceRef {
            turn_id: id,
            quote: text.into(),
        }];
        s.assess(
            s.generation_id,
            s.turn_version,
            &Observation {
                speech_act: SpeechAct::Request,
                requested_object: object,
                direction: Direction::CallerRequestsFromSarah,
            },
            &refs,
        )
        .unwrap();
        s.record_message(
            "Suspicious request blocked; independent verification recommended.",
            &refs,
            Utc::now(),
        )
        .unwrap();
        store.save(&s).await.unwrap();
        let source=store.append(Some(&s),"system","message.recorded",json!({"turn_version":s.turn_version,"recorded_at":s.message.as_ref().unwrap().recorded_at}),s.execution_mode).await.unwrap();
        service.capture_identity(&s, Some(NUMBER)).await.unwrap();
        let result = service
            .report_protective(&s, &source)
            .await
            .unwrap()
            .unwrap();
        let public = serde_json::to_value(&result.reputation).unwrap();
        let tactic_id = format!("firstring:tactic:v1:{category}");
        assert!(
            public["scam_categories"]
                .as_array()
                .is_some_and(|items| items.contains(&json!(category)))
        );
        assert!(result.reputation.tactic_ids.contains(&tactic_id));
        let stored: (String, String) = sqlx::query_as(
            "SELECT scam_category,tactic_id FROM reputation_reports WHERE report_id=?",
        )
        .bind(&result.report_id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
        assert_eq!(stored, (category.into(), tactic_id.clone()));
        let view = service.session_view(&s).await.unwrap();
        assert_eq!(view["tactic_identifier_kind"], "categorical_taxonomy_v1");
        assert!(
            view["related_tactic_ids"]
                .as_array()
                .unwrap()
                .contains(&json!(tactic_id))
        );
        assert_eq!(view["matched_capsule_ids"], json!([]));
        let detail = service
            .detail(ExecutionMode::Replay, &result.reputation.number_key)
            .await
            .unwrap()
            .unwrap();
        let report = detail["reports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["report_id"] == result.report_id)
            .unwrap();
        assert_eq!(report["scam_category"], category);
        assert_eq!(report["tactic_identifier_kind"], "categorical_taxonomy_v1");
        let event = store
            .events_after(0)
            .await
            .unwrap()
            .into_iter()
            .find(|e| e.kind == "reputation.reported" && e.payload["report_id"] == result.report_id)
            .unwrap();
        assert_eq!(event.payload["scam_category"], category);
        assert_eq!(event.payload["tactic_id"], tactic_id);
    }
    let capsules: i64 = sqlx::query_scalar("SELECT count(*) FROM tactics")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(
        capsules, 0,
        "Categorical identifiers are not published or matched capsules"
    );
    clock.fetch_add(Duration::days(15).num_seconds(), Ordering::SeqCst);
    let expired = service
        .list(ExecutionMode::Replay, 50)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert!(expired.tactic_ids.is_empty());
    assert_eq!(
        serde_json::to_value(expired).unwrap()["scam_categories"],
        json!([])
    );
}

#[tokio::test]
async fn additive_category_migration_preserves_unknown_legacy_report_without_guessing() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    sqlx::raw_sql("DROP TABLE reputation_reports;
CREATE TABLE reputation_reports(report_id TEXT PRIMARY KEY,entry_id TEXT NOT NULL,session_id TEXT UNIQUE NOT NULL,household_id TEXT NOT NULL,participation_group TEXT NOT NULL,source_event_id TEXT NOT NULL,source_event_sequence INTEGER NOT NULL,source_turn_version INTEGER NOT NULL,source_policy_epoch INTEGER NOT NULL,reported_at TEXT NOT NULL,expires_at TEXT NOT NULL,retracted INTEGER NOT NULL DEFAULT 0);
INSERT INTO reputation_reports VALUES('legacy-report','legacy-key','legacy-session','sarah','local_operator','legacy-event',17,2,1,'2026-11-08T00:00:00Z','2026-11-22T00:00:00Z',0);").execute(&store.pool).await.unwrap();
    firstring::reputation::initialize(&store.pool)
        .await
        .unwrap();
    firstring::reputation::initialize(&store.pool)
        .await
        .unwrap();
    let row:(String,i64,Option<String>,Option<String>)=sqlx::query_as("SELECT session_id,source_event_sequence,scam_category,tactic_id FROM reputation_reports WHERE report_id='legacy-report'").fetch_one(&store.pool).await.unwrap();
    assert_eq!(row, ("legacy-session".into(), 17, None, None));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM reputation_reports")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn legacy_snapshot_metadata_refresh_is_versioned_without_claiming_expiry() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let service = ReputationService::new(store.clone(), KEY, Scope::Demo).unwrap();
    let result = report(&service, &store, "sarah").await;
    let mut legacy = serde_json::to_value(&result.reputation).unwrap();
    for field in [
        "scam_categories",
        "tactic_identifier_kind",
        "matched_capsule_ids",
    ] {
        legacy.as_object_mut().unwrap().remove(field);
    }
    legacy["tactic_ids"] = json!([]);
    legacy["explanation"] = json!("Legacy explanation without category metadata.");
    sqlx::query("UPDATE number_reputation SET data=?")
        .bind(legacy.to_string())
        .execute(&store.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE reputation_reports SET scam_category=NULL,tactic_id=NULL")
        .execute(&store.pool)
        .await
        .unwrap();
    let refreshed = service
        .list(ExecutionMode::Replay, 50)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(refreshed.level, "reported");
    assert_eq!(refreshed.household_count, 1);
    assert_eq!(refreshed.version, result.reputation.version + 1);
    assert!(refreshed.scam_categories.is_empty());
    let event = store
        .events_after(0)
        .await
        .unwrap()
        .into_iter()
        .rfind(|e| e.kind == "reputation.level_changed")
        .unwrap();
    assert_eq!(event.payload["reason"], "metadata_refreshed");
    assert_eq!(
        service
            .list(ExecutionMode::Replay, 50)
            .await
            .unwrap()
            .pop()
            .unwrap()
            .version,
        refreshed.version,
        "Idempotent metadata refresh cannot manufacture repeated versions"
    );
}
