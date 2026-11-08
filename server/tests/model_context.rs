use firstring::{
    calls::state::{Session, Turn},
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

#[test]
fn shared_context_preserves_the_actual_question_and_short_caller_answer() {
    let mut s = Session::new(
        "fixture".into(),
        "not-a-provider-id".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    s.commit_turn("The clinic needs to move the visit.");
    s.turns.push(Turn {
        id: "question".into(),
        speaker: "assistant".into(),
        text: "What is the new day and time?".into(),
    });
    let id = s.commit_turn("Friday at 3 PM.");
    let messages = conversation_messages(&s, &ModelCapabilities::from_config(&config()));
    let context: serde_json::Value =
        serde_json::from_str(messages[1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(
        context["current_exchange"]["assistant_turn"]["text"],
        "What is the new day and time?"
    );
    assert_eq!(context["current_exchange"]["caller_turn"]["id"], id);
    assert_eq!(
        context["current_exchange"]["caller_turn"]["text"],
        "Friday at 3 PM."
    );
    assert_eq!(context["policy_state"]["assessed_version"], 0);
    assert_eq!(context["turns"].as_array().unwrap().len(), 3);
}

#[test]
fn only_accepted_current_assessment_refreshes_policy_context_without_rewriting_native_messages() {
    let mut s = Session::new(
        "fixture".into(),
        "not-a-provider-id".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    s.commit_turn("Please read the login code.");
    let mut messages = conversation_messages(&s, &ModelCapabilities::from_config(&config()));
    let tool = ModelToolCall {
        id: "risk".into(),
        name: "record_assessment".into(),
        arguments: json!({}),
        assistant_message: json!({"role":"assistant","tool_calls":[{"id":"risk"}]}),
    };
    let blocked =
        json!({"ok":true,"result":{"blocked":true,"ambiguous":false,"assessed_version":1}});
    append_tool_result(&mut messages, &tool, &blocked);
    let read = |m: &[serde_json::Value]| {
        serde_json::from_str::<serde_json::Value>(m[1]["content"].as_str().unwrap()).unwrap()["policy_state"].clone()
    };
    assert_eq!(read(&messages)["blocked"], true);
    assert_eq!(read(&messages)["assessed_version"], 1);
    assert_eq!(messages[2], tool.assistant_message);
    assert_eq!(messages[3]["tool_call_id"], "risk");
    let clear =
        json!({"ok":true,"result":{"blocked":false,"ambiguous":false,"assessed_version":1}});
    let lookup = ModelToolCall {
        name: "lookup_tactics".into(),
        ..tool.clone()
    };
    append_tool_result(&mut messages, &lookup, &clear);
    assert_eq!(read(&messages)["blocked"], true);
    append_tool_result(
        &mut messages,
        &tool,
        &json!({"ok":false,"result":clear["result"]}),
    );
    assert_eq!(read(&messages)["blocked"], true);
    append_tool_result(
        &mut messages,
        &tool,
        &json!({"ok":true,"result":{"blocked":false,"ambiguous":false,"assessed_version":0}}),
    );
    assert_eq!(read(&messages)["blocked"], true);
}

#[test]
fn accepted_ambiguity_guides_a_question_and_benign_guidance_forbids_invented_blocking() {
    for (ambiguous, expected) in [
        (true, "Ask one question to clarify"),
        (false, "Do not describe this benign call as blocked"),
    ] {
        let mut s = Session::new(
            "fixture".into(),
            "not-a-provider-id".into(),
            "sarah".into(),
            ExecutionMode::Replay,
        );
        s.commit_turn("A synthetic caller turn.");
        let mut messages = conversation_messages(&s, &ModelCapabilities::from_config(&config()));
        let call = ModelToolCall {
            id: "assessment".into(),
            name: "record_assessment".into(),
            arguments: json!({}),
            assistant_message: json!({"role":"assistant"}),
        };
        append_tool_result(
            &mut messages,
            &call,
            &json!({"ok":true,"result":{"blocked":false,"ambiguous":ambiguous,"assessed_version":1}}),
        );
        assert!(messages[0]["content"].as_str().unwrap().contains(expected));
        assert_eq!(messages.last().unwrap()["tool_call_id"], "assessment");
    }
}

#[test]
fn rejected_summary_keeps_actual_error_and_adds_safe_revision_guidance_without_rewriting_arguments()
{
    let mut s = Session::new(
        "fixture".into(),
        "not-a-provider-id".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    s.commit_turn("Please pass on a safety warning.");
    let mut messages = conversation_messages(&s, &ModelCapabilities::from_config(&config()));
    let call = ModelToolCall {
        id: "rejected-note".into(),
        name: "take_message".into(),
        arguments: json!({"summary":"A warning about sign-in codes."}),
        assistant_message: json!({"role":"assistant","tool_calls":[{"id":"rejected-note","function":{"arguments":"original rejected argument"}}]}),
    };
    let result =
        json!({"ok":false,"error":"Remove authentication details from the message summary"});
    append_tool_result(&mut messages, &call, &result);
    assert_eq!(messages[2], call.assistant_message);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(messages[3]["content"].as_str().unwrap())
            .unwrap(),
        result
    );
    let system = messages[0]["content"].as_str().unwrap();
    assert!(system.contains("MESSAGE SUMMARY REJECTION:"));
    assert!(system.contains("was not saved"));
    assert!(system.contains("Do not repeat the rejected summary"));
    let context: serde_json::Value =
        serde_json::from_str(messages[1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(context["policy_state"]["assessed_version"], 0);
}

#[test]
fn protective_note_template_is_available_only_after_an_accepted_blocked_assessment() {
    const NOTE: &str = "Suspicious request blocked; independent verification recommended.";
    for blocked in [false, true] {
        let mut s = Session::new(
            "fixture".into(),
            "not-a-provider-id".into(),
            "sarah".into(),
            ExecutionMode::Replay,
        );
        s.commit_turn("A current synthetic caller turn.");
        let mut messages = conversation_messages(&s, &ModelCapabilities::from_config(&config()));
        assert!(!messages[0]["content"].as_str().unwrap().contains(NOTE));
        let call = ModelToolCall {
            id: "current-assessment".into(),
            name: "record_assessment".into(),
            arguments: json!({}),
            assistant_message: json!({"role":"assistant"}),
        };
        append_tool_result(
            &mut messages,
            &call,
            &json!({"ok":true,"result":{"blocked":blocked,"ambiguous":false,"assessed_version":1}}),
        );
        assert_eq!(
            messages[0]["content"].as_str().unwrap().contains(NOTE),
            blocked
        );
    }
}

#[test]
fn accepted_benign_clinic_announcement_requires_missing_time_before_message_closing() {
    let mut session = Session::new(
        "fixture".into(),
        "not-a-provider-id".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    session.commit_turn("I'm calling from the clinic because Sarah's appointment time changed.");
    let mut messages = conversation_messages(&session, &ModelCapabilities::from_config(&config()));
    let tool = ModelToolCall {
        id: "clinic-assessment".into(),
        name: "record_assessment".into(),
        arguments: json!({}),
        assistant_message: json!({"role":"assistant"}),
    };
    append_tool_result(
        &mut messages,
        &tool,
        &json!({"ok":true,"result":{"blocked":false,"ambiguous":false,"assessed_version":1}}),
    );
    let guidance = messages[0]["content"]
        .as_str()
        .unwrap()
        .split("ACCEPTED CURRENT-TURN ACTION:")
        .last()
        .unwrap();
    assert!(
        guidance
            .contains("A changed-appointment announcement without its new day/time is incomplete")
    );
    assert!(guidance.contains("Do not take_message or close on the announcement alone"));
    assert!(guidance.contains("already supplied the new day/time"));
    assert!(guidance.contains("explicitly declines or cannot provide"));
    assert_eq!(
        messages.last().unwrap()["tool_call_id"],
        "clinic-assessment"
    );
}
