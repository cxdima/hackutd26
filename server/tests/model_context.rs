use firstring::{
    calls::state::Session,
    config::{Config, ExecutionMode},
    providers::nemotron::{
        ModelCapabilities, ModelToolCall, append_tool_result, conversation_messages,
    },
};
use serde_json::json;
use std::collections::BTreeMap;

fn config() -> Config {
    Config {
        public_base_url: "https://fixture.invalid".into(),
        port: 8080,
        database_url: "sqlite::memory:".into(),
        admin_token: "fixture".into(),
        mode: ExecutionMode::Live,
        transport: "relay".into(),
        account_sid: "ACfixture".into(),
        auth_token: "fixture".into(),
        photon_number: "+12025550111".into(),
        sip_host: "fixture.invalid".into(),
        sip_port: 5061,
        sip_username: "fixture".into(),
        sip_password: "fixture".into(),
        contacts: BTreeMap::new(),
        smoke_authorized: false,
        handoff_enabled: false,
        family_enabled: false,
        max_call_seconds: 180,
        nvidia_key: String::new(),
        model_url: "http://127.0.0.1:1".into(),
        model: "fixture".into(),
    }
}

#[test]
fn capabilities_require_enrollment_authorization_and_outbound_configuration() {
    let mut c = config();
    let unavailable = ModelCapabilities::from_config(&c);
    assert!(!unavailable.handoff_available);
    assert!(!unavailable.family_callback_available);
    assert!(unavailable.message_taking_available);
    c.handoff_enabled = true;
    c.family_enabled = true;
    assert!(!ModelCapabilities::from_config(&c).handoff_available);
    c.contacts.insert("sarah".into(), "+12025550122".into());
    c.contacts.insert("family".into(), "+12025550133".into());
    assert!(ModelCapabilities::from_config(&c).handoff_available);
    assert!(ModelCapabilities::from_config(&c).family_callback_available);
    c.sip_password.clear();
    assert!(!ModelCapabilities::from_config(&c).family_callback_available);
    assert!(
        !serde_json::to_string(&ModelCapabilities::from_config(&c))
            .unwrap()
            .contains("1202555")
    );
}

#[test]
fn shared_messages_expose_capabilities_and_preserve_native_result_identity() {
    let mut s = Session::new(
        "fixture".into(),
        "not-a-provider-id".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    s.commit_turn("Please tell her the delivery is here.");
    let mut messages = conversation_messages(&s, &ModelCapabilities::from_config(&config()));
    assert_eq!(messages.len(), 2);
    let text = messages[1]["content"].as_str().unwrap();
    let context: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(context["capabilities"]["handoff_available"], false);
    assert_eq!(
        context["turns"][0]["text"],
        "Please tell her the delivery is here."
    );
    assert!(!text.contains("Do not execute any action"));
    let assistant = json!({"role":"assistant","tool_calls":[{"id":"native-42","type":"function","function":{"name":"lookup_trusted_contact","arguments":"{\"role\":\"family\"}"}}]});
    let tool = ModelToolCall {
        id: "native-42".into(),
        name: "lookup_trusted_contact".into(),
        arguments: json!({"role":"family"}),
        assistant_message: assistant.clone(),
    };
    append_tool_result(
        &mut messages,
        &tool,
        &json!({"ok":false,"error":"Requested role is not enrolled"}),
    );
    assert_eq!(messages[2], assistant);
    assert_eq!(messages[3]["tool_call_id"], "native-42");
}
