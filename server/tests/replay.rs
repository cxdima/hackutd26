use firstring::{
    calls::actor::App,
    config::{Config, ExecutionMode},
    replay,
};
async fn app() -> App {
    let mut c = Config::from_env().unwrap();
    c.database_url = "sqlite::memory:".into();
    c.mode = ExecutionMode::Replay;
    App::new(c).await.unwrap()
}
#[tokio::test]
async fn revocation_replay_exercises_actor_and_subscriber_without_any_bridge() {
    let a = app().await;
    let out = replay::run(&a, "revocation").await.unwrap();
    assert_eq!(out["execution_mode"], "replay");
    let events = a.store.events_after(0).await.unwrap();
    assert!(events.iter().any(|e| e.kind == "handoff.revoked"));
    assert!(events.iter().any(|e| e.kind == "handoff.stale_rejected"));
    assert!(events.iter().any(|e| e.kind == "contact.acknowledged"));
    assert!(!events.iter().any(|e| e.kind == "bridge.connected"));
    assert!(
        events
            .iter()
            .all(|e| e.execution_mode == ExecutionMode::Replay)
    );
    for _ in 0..30 {
        let (c, _) = firstring::patterns::view(&a.store, ExecutionMode::Replay)
            .await
            .unwrap();
        if c.iter()
            .all(|h| h["last_applied_version"].as_i64().unwrap_or(0) > 0)
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("subscriber did not apply publication");
}
#[tokio::test]
async fn benign_replay_connects_only_two_expected_participants_and_no_family() {
    let a = app().await;
    replay::run(&a, "benign").await.unwrap();
    let events = a.store.events_after(0).await.unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| e.kind == "bridge.connected")
            .count(),
        1
    );
    assert!(!events.iter().any(|e| e.kind == "contact.acknowledged"));
    assert!(!events.iter().any(|e| e.leg_role == "family"));
}
#[tokio::test]
async fn new_number_benign_control_remains_clear_after_shared_memory() {
    let a = app().await;
    replay::run(&a, "revocation").await.unwrap();
    let out = replay::run(&a, "benign_new_number").await.unwrap();
    let session = a
        .store
        .sessions()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.id == out["session_id"])
        .unwrap();
    assert!(!session.blocked);
    assert!(session.legs.is_empty());
}
