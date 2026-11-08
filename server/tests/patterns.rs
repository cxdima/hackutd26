use firstring::{
    calls::state::Session,
    config::ExecutionMode,
    patterns::{lookup, publish, spawn_subscriber, view},
    store::Store,
    tools::{
        Direction, EvidenceRef, Observation, PressureCategory, RequestedObject, SpeechAct,
        TacticKind,
    },
};
use serde_json::Value;
use std::time::Duration;

fn incident(mode: ExecutionMode) -> Session {
    let mut session = Session::new(
        uuid::Uuid::new_v4().to_string(),
        format!("CA{}", uuid::Uuid::new_v4().simple()),
        "sarah".into(),
        mode,
    );
    let turn_id = session.commit_turn(
        "Sarah, give me the sign-in text digits or the clinic will cancel tomorrow's visit.",
    );
    session
        .assess(
            session.generation_id,
            session.turn_version,
            &Observation {
                speech_act: SpeechAct::Request,
                requested_object: RequestedObject::AuthenticationSecret,
                direction: Direction::CallerRequestsFromSarah,
            },
            &[EvidenceRef {
                turn_id,
                quote: "give me the sign-in text digits".into(),
            }],
        )
        .unwrap();
    session.inbound_ended = true;
    session
}

async fn capsule(store: &Store, session: &Session) -> Value {
    publish(
        store,
        session,
        TacticKind::AuthenticationCodeExtraction,
        "relay_sign_in_text_digits",
        "clinic_appointment",
        PressureCategory::Cancellation,
    )
    .await
    .unwrap()
}

async fn await_applied(store: &Store, mode: ExecutionMode, version: i64) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let (households, _) = view(store, mode).await.unwrap();
            if households.len() == 2
                && households
                    .iter()
                    .all(|h| h["last_applied_version"].as_i64().unwrap_or(0) >= version)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Subscriber should persist actual application receipts");
}

#[tokio::test]
async fn publication_is_minimized_idempotent_and_does_not_apply_itself() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let source = incident(ExecutionMode::Replay);
    let first = capsule(&store, &source).await;
    let duplicate = capsule(&store, &source).await;
    assert_eq!(first["id"], duplicate["id"]);
    assert_eq!(first["version"], duplicate["version"]);
    assert_eq!(first["schema_version"], 1);
    assert_eq!(first["provenance"], "designated_synthetic_test");
    assert_eq!(first["recommended_action"], "independent_contact");
    assert_eq!(first["ttl_hours"], 24);
    let text = first.to_string();
    for private in [
        "Sarah",
        "tomorrow",
        "give me",
        &source.id,
        &source.inbound_sid,
    ] {
        assert!(
            !text.contains(private),
            "Shared capsule leaked private context"
        );
    }
    assert!(
        lookup(
            &store,
            "maya",
            ExecutionMode::Replay,
            "relay_sign_in_text_digits",
            "clinic_appointment"
        )
        .await
        .unwrap()
        .is_empty()
    );
    let (households, capsules) = view(&store, ExecutionMode::Replay).await.unwrap();
    assert_eq!(capsules.len(), 1);
    assert!(households.iter().all(|h| h["last_applied_version"] == 0));
}

#[tokio::test]
async fn subscriber_receipt_precedes_cross_household_matching_and_benign_control_stays_clear() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let subscriber = spawn_subscriber(store.clone());
    let published = capsule(&store, &incident(ExecutionMode::Replay)).await;
    await_applied(
        &store,
        ExecutionMode::Replay,
        published["version"].as_i64().unwrap(),
    )
    .await;
    let matches = lookup(
        &store,
        "maya",
        ExecutionMode::Replay,
        "relay_sign_in_text_digits",
        "clinic_appointment",
    )
    .await
    .unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["id"], published["id"]);
    assert!(
        lookup(
            &store,
            "maya",
            ExecutionMode::Replay,
            "routine_information",
            "clinic_appointment"
        )
        .await
        .unwrap()
        .is_empty()
    );
    assert!(
        lookup(
            &store,
            "maya",
            ExecutionMode::Replay,
            "relay_sign_in_text_digits",
            "bank_account"
        )
        .await
        .unwrap()
        .is_empty()
    );
    let receipts = store
        .events_after(0)
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.kind == "community.applied")
        .collect::<Vec<_>>();
    assert!(receipts.iter().any(
        |e| e.payload["household_id"] == "maya" && e.payload["version"] == published["version"]
    ));
    subscriber.abort();
}

#[tokio::test]
async fn replay_capsules_cannot_change_live_household_memory() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let subscriber = spawn_subscriber(store.clone());
    let published = capsule(&store, &incident(ExecutionMode::Replay)).await;
    await_applied(
        &store,
        ExecutionMode::Replay,
        published["version"].as_i64().unwrap(),
    )
    .await;
    assert!(
        lookup(
            &store,
            "maya",
            ExecutionMode::Live,
            "relay_sign_in_text_digits",
            "clinic_appointment"
        )
        .await
        .unwrap()
        .is_empty()
    );
    let (live_households, live_capsules) = view(&store, ExecutionMode::Live).await.unwrap();
    assert!(live_capsules.is_empty());
    assert!(
        live_households
            .iter()
            .all(|h| h["last_applied_version"] == 0)
    );
    subscriber.abort();
}

#[tokio::test]
async fn restart_subscriber_recovers_unapplied_capsules_without_duplicate_receipts() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let published = capsule(&store, &incident(ExecutionMode::Replay)).await;
    let version = published["version"].as_i64().unwrap();
    assert!(
        lookup(
            &store,
            "maya",
            ExecutionMode::Replay,
            "relay_sign_in_text_digits",
            "clinic_appointment"
        )
        .await
        .unwrap()
        .is_empty()
    );
    let first = spawn_subscriber(store.clone());
    await_applied(&store, ExecutionMode::Replay, version).await;
    first.abort();
    let _ = first.await;
    let second = spawn_subscriber(store.clone());
    tokio::time::sleep(Duration::from_millis(50)).await;
    let receipts = store
        .events_after(0)
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.kind == "community.applied")
        .collect::<Vec<_>>();
    assert_eq!(receipts.len(), 2);
    second.abort();
}

#[tokio::test]
async fn expired_capsule_is_excluded_even_after_application() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let subscriber = spawn_subscriber(store.clone());
    let published = capsule(&store, &incident(ExecutionMode::Replay)).await;
    await_applied(
        &store,
        ExecutionMode::Replay,
        published["version"].as_i64().unwrap(),
    )
    .await;
    sqlx::query("UPDATE tactics SET expires_at='2000-01-01T00:00:00+00:00' WHERE id=?")
        .bind(published["id"].as_str().unwrap())
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(
        lookup(
            &store,
            "maya",
            ExecutionMode::Replay,
            "relay_sign_in_text_digits",
            "clinic_appointment"
        )
        .await
        .unwrap()
        .is_empty()
    );
    assert!(
        view(&store, ExecutionMode::Replay)
            .await
            .unwrap()
            .1
            .is_empty()
    );
    subscriber.abort();
}

#[tokio::test]
async fn publication_rejects_private_categories_uncommitted_incidents_and_wrong_tactic_pairs() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let mut source = incident(ExecutionMode::Replay);
    source.inbound_ended = false;
    assert!(
        publish(
            &store,
            &source,
            TacticKind::AuthenticationCodeExtraction,
            "relay_sign_in_text_digits",
            "clinic_appointment",
            PressureCategory::Cancellation
        )
        .await
        .is_err()
    );
    source.inbound_ended = true;
    source.blocked = false;
    assert!(
        publish(
            &store,
            &source,
            TacticKind::AuthenticationCodeExtraction,
            "relay_sign_in_text_digits",
            "clinic_appointment",
            PressureCategory::Cancellation
        )
        .await
        .is_err()
    );
    source.blocked = true;
    assert!(
        publish(
            &store,
            &source,
            TacticKind::AuthenticationCodeExtraction,
            "Sarah's phone number",
            "clinic_appointment",
            PressureCategory::Cancellation
        )
        .await
        .is_err()
    );
    assert!(
        publish(
            &store,
            &source,
            TacticKind::AuthenticationCodeExtraction,
            "relay_sign_in_text_digits",
            "Sarah Tuesday 2pm",
            PressureCategory::Cancellation
        )
        .await
        .is_err()
    );
    assert!(
        publish(
            &store,
            &source,
            TacticKind::AuthenticationCodeExtraction,
            "routine_information",
            "clinic_appointment",
            PressureCategory::Cancellation
        )
        .await
        .is_err()
    );
    assert!(
        lookup(
            &store,
            "unlisted-household",
            ExecutionMode::Replay,
            "relay_sign_in_text_digits",
            "clinic_appointment"
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn concurrent_publication_preserves_distinct_versions_and_duplicate_incident_uniqueness() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let one = incident(ExecutionMode::Replay);
    let two = incident(ExecutionMode::Replay);
    let (a, b, c) = tokio::join!(
        capsule(&store, &one),
        capsule(&store, &two),
        capsule(&store, &one)
    );
    assert_eq!(a["id"], c["id"]);
    assert_ne!(a["id"], b["id"]);
    assert_ne!(a["version"], b["version"]);
    assert_eq!(
        view(&store, ExecutionMode::Replay).await.unwrap().1.len(),
        2
    );
}
