use firstring::{
    calls::actor::App,
    config::{Config, ExecutionMode},
    profile::{Profile, ProfileUpdate},
    providers::nemotron::{ModelCapabilities, conversation_messages},
    providers::relay,
};
use serde_json::json;

#[tokio::test]
async fn incoming_snapshots_profile_for_voice_and_model_without_mutating_an_existing_call() {
    let mut c = Config::from_env().unwrap();
    c.mode = ExecutionMode::Replay;
    c.database_url = "sqlite::memory:".into();
    let app = App::new(c).await.unwrap();
    let (handle, first) = app
        .incoming("CA11111111111111111111111111111111")
        .await
        .unwrap();
    assert_eq!(first["profile"]["voice_id"], firstring::profile::JESSICA);
    let update: ProfileUpdate=serde_json::from_value(json!({"display_name":"Jordan","greeting":"Hello, what can I help with?","tts_provider":"ElevenLabs","voice_id":firstring::profile::GEORGE,"locale":"en-US","timezone":"America/New_York"})).unwrap();
    Profile::save(&app.store, "sarah", update).await.unwrap();
    let (_, next) = app
        .incoming("CA22222222222222222222222222222222")
        .await
        .unwrap();
    assert_eq!(next["profile"]["display_name"], "Jordan");
    let old = handle
        .send(firstring::calls::actor::Input::Snapshot)
        .await
        .unwrap();
    assert_eq!(old["profile"]["display_name"], "Sarah");
    let s = serde_json::from_value(next).unwrap();
    let messages = conversation_messages(&s, &ModelCapabilities::from_config(&app.config));
    let context: serde_json::Value =
        serde_json::from_str(messages[1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(context["profile"]["display_name"], "Jordan");
}

#[test]
fn default_relay_greeting_is_natural_and_does_not_warn_about_secrets() {
    let xml = relay::connect_twiml("https://fixture.invalid", "token", "Sarah");
    assert!(xml.contains("Hi, thanks for calling."));
    assert!(xml.contains(firstring::profile::JESSICA));
    assert!(!xml.contains("automated assistant"));
    assert!(!xml.contains("password"));
}
