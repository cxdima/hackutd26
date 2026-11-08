//! Bounded actor recovery from documented incomplete caller speech. No providers.
use firstring::{
    calls::actor::{App, AudioFrame, Handle, Input},
    config::{Config, ExecutionMode},
    providers::nemotron::ModelToolCall,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
const REPEAT: &str = "I didn't catch the end of that. Please repeat your request without sharing passwords or sign-in codes.";
async fn setup() -> (App, Handle, broadcast::Receiver<AudioFrame>) {
    let config = Config {
        public_base_url: "https://recovery.fixture.example".into(),
        port: 8080,
        database_url: "sqlite::memory:".into(),
        admin_token: "fixture-token-at-least-24-characters".into(),
        mode: ExecutionMode::Replay,
        transport: "relay".into(),
        account_sid: String::new(),
        auth_token: String::new(),
        photon_number: String::new(),
        sip_host: "sip.photon.codes".into(),
        sip_port: 5061,
        sip_username: String::new(),
        sip_password: String::new(),
        contacts: BTreeMap::new(),
        smoke_authorized: false,
        handoff_enabled: false,
        family_enabled: false,
        max_call_seconds: 180,
        nvidia_key: String::new(),
        model_url: "http://127.0.0.1:1/never-contacted".into(),
        model: "fixture".into(),
    };
    let app = App::new(config).await.unwrap();
    let (h, s) = app
        .incoming("CA00000000000000000000000000000001")
        .await
        .unwrap();
    let audio = h.audio.subscribe();
    h.send(Input::ConnectRelay {
        sid: s["inbound_sid"].as_str().unwrap().into(),
        token: s["relay_token"].as_str().unwrap().into(),
    })
    .await
    .unwrap();
    (app, h, audio)
}
async fn tool(h: &Handle, s: &Value, name: &str, arguments: Value) -> anyhow::Result<Value> {
    h.send(Input::ModelTool {
        generation: s["generation_id"].as_u64().unwrap(),
        epoch: s["policy_epoch"].as_u64().unwrap(),
        version: s["turn_version"].as_u64().unwrap(),
        deadline: Instant::now() + Duration::from_secs(20),
        tool: ModelToolCall {
            id: "fixture-tool".into(),
            name: name.into(),
            arguments,
            assistant_message: json!({}),
        },
    })
    .await
}
async fn final_assessed(h: &Handle, text: &str) -> (Value, Value) {
    let turn = h
        .send(Input::FinalPrompt { text: text.into() })
        .await
        .unwrap();
    let s = h.send(Input::Snapshot).await.unwrap();
    let evidence = json!([{"turn_id":turn["turn_id"],"quote":text}]);
    assert_eq!(tool(h,&s,"record_assessment",json!({"observation":{"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"},"evidence_refs":evidence})).await.unwrap()["ok"],true);
    (h.send(Input::Snapshot).await.unwrap(), evidence)
}
async fn recovery(audio: &mut broadcast::Receiver<AudioFrame>) -> AudioFrame {
    tokio::time::timeout(Duration::from_secs(6), audio.recv())
        .await
        .expect("Incomplete speech must produce a bounded repeat request")
        .unwrap()
}
#[tokio::test]
async fn completed_turn_then_abandoned_partial_requests_repeat_without_committing_or_acting() {
    let (app, h, mut audio) = setup().await;
    let (_, _) = final_assessed(&h, "The appointment moved to Friday.").await;
    h.send(Input::Speech {
        partial: Some("And before you".into()),
    })
    .await
    .unwrap();
    let before = h.send(Input::Snapshot).await.unwrap();
    assert_eq!(
        h.send(Input::SpeechRecovery {
            generation: before["generation_id"].as_u64().unwrap(),
            epoch: before["policy_epoch"].as_u64().unwrap()
        })
        .await
        .unwrap()["ignored"],
        "obsolete_or_early_recovery"
    );
    let frame = recovery(&mut audio).await;
    assert_eq!(frame.payload["token"], REPEAT);
    let after = h.send(Input::Snapshot).await.unwrap();
    assert_eq!(after["speech_pending"], true);
    assert_eq!(after["turn_version"], before["turn_version"]);
    assert_eq!(after["turns"], before["turns"]);
    assert_eq!(frame.generation, after["generation_id"].as_u64().unwrap());
    assert_eq!(
        h.send(Input::SpeechRecovery {
            generation: frame.generation,
            epoch: frame.epoch
        })
        .await
        .unwrap()["ignored"],
        "obsolete_or_early_recovery"
    );
    assert!(app.store.actions(&h.id).await.unwrap().is_empty());
    assert!(
        !app.store
            .events_after(0)
            .await
            .unwrap()
            .iter()
            .any(|e| e.kind == "model.request.completed")
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(250), audio.recv())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn repeated_partials_have_an_absolute_deadline_and_new_input_can_start_one_new_episode() {
    let (_, h, mut audio) = setup().await;
    let start = Instant::now();
    h.send(Input::Speech {
        partial: Some("Still speaking".into()),
    })
    .await
    .unwrap();
    for _ in 0..3 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        h.send(Input::Speech {
            partial: Some("Still speaking more".into()),
        })
        .await
        .unwrap();
    }
    let first = recovery(&mut audio).await;
    assert!(start.elapsed() < Duration::from_secs(10));
    assert_eq!(first.payload["token"], REPEAT);
    let next = Instant::now();
    h.send(Input::Speech {
        partial: Some("Let me try again".into()),
    })
    .await
    .unwrap();
    let second = recovery(&mut audio).await;
    assert!(next.elapsed() >= Duration::from_millis(3900));
    assert!(second.generation > first.generation);
    assert_eq!(h.send(Input::Snapshot).await.unwrap()["turn_version"], 0);
}
#[tokio::test]
async fn late_final_cancels_old_recovery_and_old_model_completion() {
    let (_, h, mut audio) = setup().await;
    let (old, evidence) = final_assessed(&h, "The visit has changed.").await;
    h.send(Input::Speech {
        partial: Some("Friday".into()),
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let (_, _) = final_assessed(&h, "It is Friday at nine.").await;
    assert!(tool(&h,&old,"ask_caller",json!({"question":"When is the visit?","purpose":"clarify_request","evidence_refs":evidence})).await.is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(4500), audio.recv())
            .await
            .is_err()
    );
    assert_eq!(
        h.send(Input::Snapshot).await.unwrap()["speech_pending"],
        false
    );
}
#[tokio::test]
async fn interrupt_followed_by_final_allows_current_reply_and_suppresses_old_recovery() {
    let (_, h, mut audio) = setup().await;
    h.send(Input::Speech { partial: None }).await.unwrap();
    let (current, evidence) = final_assessed(&h, "The new visit is Wednesday.").await;
    assert_eq!(tool(&h,&current,"ask_caller",json!({"question":"What time on Wednesday?","purpose":"clarify_request","evidence_refs":evidence})).await.unwrap()["ok"],true);
    let frame = audio.recv().await.unwrap();
    assert_eq!(frame.payload["token"], "What time on Wednesday?");
    assert!(
        tokio::time::timeout(Duration::from_millis(4500), audio.recv())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn hangup_while_speech_pending_suppresses_recovery() {
    let (_, h, mut audio) = setup().await;
    h.send(Input::Speech {
        partial: Some("Wait".into()),
    })
    .await
    .unwrap();
    h.send(Input::InboundEnded).await.unwrap();
    let terminal = h.send(Input::Snapshot).await.unwrap();
    h.send(Input::End).await.unwrap();
    assert_eq!(
        h.send(Input::Snapshot).await.unwrap()["current_action"],
        terminal["current_action"],
        "Late max-call timer must not resurrect a completed call"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(4500), audio.recv())
            .await
            .is_err()
    );
    assert_eq!(
        h.send(Input::Snapshot).await.unwrap()["inbound_ended"],
        true
    );
}

#[test]
fn only_current_dedicated_recovery_audio_can_cross_an_unresolved_speech_hold() {
    use firstring::calls::{actor::AudioPurpose, state::Session};
    let mut s = Session::new(
        "fixture".into(),
        "no-provider".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    s.speech_started();
    let frame = AudioFrame {
        generation: s.generation_id,
        epoch: s.policy_epoch,
        purpose: AudioPurpose::PendingSpeechRecovery,
        payload: json!({"type":"text","token":REPEAT,"last":true}),
    };
    assert!(frame.permitted_for(&s));
    let mut ordinary = frame.clone();
    ordinary.purpose = AudioPurpose::Response;
    assert!(!ordinary.permitted_for(&s));
    for mode in 0..6 {
        let mut changed = s.clone();
        match mode {
            0 => changed.generation_id += 1,
            1 => changed.policy_epoch += 1,
            2 => changed.inbound_ended = true,
            3 => changed.bridge_committed = true,
            4 => changed.closing_reason = Some("message_saved".into()),
            _ => changed.speech_pending = false,
        };
        assert!(!frame.permitted_for(&changed));
    }
}
#[tokio::test]
async fn late_end_still_settles_nonterminal_children_after_caller_hangup() {
    let (_, h, _) = setup().await;
    let (current, evidence) = final_assessed(&h, "The clinic visit changed to Friday.").await;
    let observation = json!({"speech_act":"statement","requested_object":"routine_information","direction":"caller_supplies"});
    assert_eq!(tool(&h,&current,"offer_handoff",json!({"summary":"Clinic visit Friday.","observation":observation,"evidence_refs":evidence})).await.unwrap()["ok"],true);
    h.send(Input::InboundEnded).await.unwrap();
    h.send(Input::End).await.unwrap();
    assert_eq!(
        h.send(Input::Snapshot).await.unwrap()["end_requested"],
        true
    );
}
