//! Owner-voice enrollment is explicit, consented and time-boxed; an armed
//! enrollment takes the next call with a caller-only recording.
use firstring::{
    calls::actor::App,
    config::{Config, ExecutionMode},
    voice_clone,
};

fn config() -> Config {
    let mut c = Config::from_env().unwrap();
    c.database_url = "sqlite::memory:".into();
    c.public_base_url = "https://voice.example.com".into();
    c.account_sid = format!("AC{}", "0".repeat(32));
    c.auth_token = "test-auth-token".into();
    c.admin_token = "a-long-local-test-admin-token".into();
    c.mode = ExecutionMode::Live;
    c.transport = "relay".into();
    c.contacts.clear();
    c
}

#[tokio::test]
async fn enrollment_is_consented_time_boxed_and_takes_one_call() {
    // SAFETY: this test binary sets the key once before any other use.
    unsafe { std::env::set_var("ELEVENLABS_API_KEY", "test-key-not-real") };
    let app = App::new(config()).await.unwrap();
    assert!(voice_clone::intercept(&app, "CA1").await.unwrap().is_none());
    assert!(
        voice_clone::arm(&app, "sarah", "").await.is_err(),
        "name required"
    );
    assert!(voice_clone::arm(&app, "nobody", "Dima").await.is_err());
    let armed = voice_clone::arm(&app, "sarah", "Dima").await.unwrap();
    assert_eq!(armed.state, "armed");
    let view = voice_clone::view(Some(&armed), "sarah");
    assert_eq!(view["state"], "armed");
    assert_eq!(view["identity_verified"], false);
    let twiml = voice_clone::intercept(&app, "CA1")
        .await
        .unwrap()
        .expect("armed call");
    assert!(twiml.contains("<Record"));
    assert!(twiml.contains("/voice/clone-recording?household=sarah"));
    // The arm is consumed: the next call is screened normally.
    assert!(voice_clone::intercept(&app, "CA2").await.unwrap().is_none());
    // While that setup call is recording, a second setup is refused...
    assert!(voice_clone::arm(&app, "sarah", "Dima").await.is_err());
    // ...but a setup call that hung up before the tone never produces a recording.
    // After a few minutes it reads as failed and the owner can simply try again.
    let key = "voice_clone.v1:sarah";
    let mut stored: serde_json::Value =
        serde_json::from_str(&app.store.metadata(key).await.unwrap().unwrap()).unwrap();
    assert_eq!(stored["state"], "recording");
    stored["updated_at"] =
        serde_json::json!((chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339());
    app.store
        .set_metadata(key, &stored.to_string())
        .await
        .unwrap();
    let stalled: voice_clone::Enrollment = serde_json::from_value(stored).unwrap();
    let view = voice_clone::view(Some(&stalled), "sarah");
    assert_eq!(view["state"], "failed");
    assert!(view["error"].as_str().unwrap().contains("Try again"));
    assert_eq!(
        voice_clone::arm(&app, "sarah", "Dima").await.unwrap().state,
        "armed"
    );
    assert!(voice_clone::active_voice("sarah").is_none());
    assert!(
        voice_clone::set_active(&app, "sarah", true).await.is_err(),
        "no voice yet"
    );
}

#[test]
fn synthesized_clips_are_short_lived_capabilities() {
    let id = voice_clone::put_media(vec![1, 2, 3]);
    assert_eq!(id.len(), 64);
    assert_eq!(voice_clone::get_media(&id), Some(vec![1, 2, 3]));
    assert_eq!(voice_clone::get_media("unknown"), None);
}
