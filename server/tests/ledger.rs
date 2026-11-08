use firstring::{calls::state::Leg, config::ExecutionMode, store::Store};
#[tokio::test]
async fn reservation_is_unique_and_smoke_authorization_cannot_be_spent_twice() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let l = Leg {
        action_id: "a1".into(),
        role: "sarah".into(),
        kind: "probe".into(),
        status: "reserved".into(),
        acknowledged: false,
        call_sid: None,
    };
    assert!(
        store
            .reserve("s1", "s1:probe", &l, ExecutionMode::Live)
            .await
            .unwrap()
            .1
    );
    let mut other = l.clone();
    other.action_id = "a2".into();
    let (saved, fresh) = store
        .reserve("s1", "s1:probe", &other, ExecutionMode::Live)
        .await
        .unwrap();
    assert!(!fresh);
    assert_eq!(saved.action_id, "a1");
    assert!(
        store
            .reserve("s2", "s2:probe", &other, ExecutionMode::Live)
            .await
            .is_err()
    );
}
