use firstring::{
    calls::{
        actor::{App, Input},
        state::Session,
    },
    config::{Config, ExecutionMode},
};
#[tokio::test]
async fn context_restore_preserves_real_turn_ids_and_refuses_live_execution() {
    let mut config = Config::from_env().unwrap();
    config.mode = ExecutionMode::Replay;
    config.database_url = "sqlite::memory:".into();
    config.nvidia_key.clear();
    let app = App::new(config).await.unwrap();
    let mut context = Session::new(
        "synthetic-context".into(),
        "CA00000000000000000000000000000000".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    let turn = context.commit_turn("My train arrives at eight thirty.");
    let mut live = context.clone();
    live.execution_mode = ExecutionMode::Live;
    assert!(app.restore_synthetic_context(live).await.is_err());
    let handle = app.restore_synthetic_context(context).await.unwrap();
    let result = handle.send(Input::Snapshot).await.unwrap();
    assert_eq!(result["turns"][0]["id"], turn);
    assert_eq!(result["inbound_ended"], false);
    assert_eq!(result["turn_version"], 1);
}
