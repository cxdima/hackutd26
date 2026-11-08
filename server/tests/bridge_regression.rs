use chrono::Utc;
use firstring::{
    calls::{
        actor::{App, Handle, Input},
        state::{Leg, Session},
    },
    config::{Config, ExecutionMode},
    providers::nemotron::ModelToolCall,
    tools::{Direction, EvidenceRef, Observation, RequestedObject, SpeechAct},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const CALLER: &str = "CA11111111111111111111111111111111";
const SARAH: &str = "CA22222222222222222222222222222222";
const CALLER_TEXT: &str = "The clinic visit is Wednesday.";

fn benign() -> Observation {
    Observation {
        speech_act: SpeechAct::Statement,
        requested_object: RequestedObject::RoutineInformation,
        direction: Direction::CallerSupplies,
    }
}

fn committed_session() -> Session {
    let mut s = Session::new(
        "bridge-test".into(),
        CALLER.into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    s.legs.push(Leg {
        action_id: "sarah-action".into(),
        role: "sarah".into(),
        kind: "handoff".into(),
        status: "in-progress".into(),
        acknowledged: false,
        call_sid: Some(SARAH.into()),
    });
    let turn_id = s.commit_turn(CALLER_TEXT);
    s.assess(
        s.generation_id,
        s.turn_version,
        &benign(),
        &[EvidenceRef {
            turn_id,
            quote: CALLER_TEXT.into(),
        }],
    )
    .unwrap();
    let offer = s
        .make_offer(
            "The clinic visit is Wednesday. Identity unverified.",
            "sarah-action",
            Utc::now(),
        )
        .unwrap();
    s.accept(&offer.id, SARAH, Utc::now()).unwrap();
    s
}

#[test]
fn late_joins_cannot_resurrect_terminal_or_ending_bridges() {
    for terminal in ["ended", "bridge_failed"] {
        let mut s = committed_session();
        assert!(!s.conference_join(SARAH));
        s.state = terminal.into();
        assert!(!s.conference_join(CALLER), "late join revived {terminal}");
        assert_eq!(s.state, terminal);
    }
    for inbound_ended in [true, false] {
        let mut s = committed_session();
        assert!(!s.conference_join(SARAH));
        s.inbound_ended = inbound_ended;
        s.end_requested = !inbound_ended;
        assert!(!s.conference_join(CALLER));
        assert_ne!(s.state, "connected");
    }
}

fn config() -> Config {
    Config {
        public_base_url: "https://example.invalid".into(),
        port: 8080,
        database_url: "sqlite::memory:".into(),
        admin_token: "fictional-operator-token-for-tests".into(),
        mode: ExecutionMode::Replay,
        transport: "relay".into(),
        account_sid: format!("AC{}", "0".repeat(32)),
        auth_token: "fictional-auth-token".into(),
        photon_number: "+12025550124".into(),
        sip_host: "sip.photon.codes".into(),
        sip_port: 5061,
        sip_username: "fictional-user".into(),
        sip_password: "fictional-password".into(),
        contacts: BTreeMap::new(),
        smoke_authorized: false,
        handoff_enabled: false,
        family_enabled: false,
        max_call_seconds: 180,
        nvidia_key: String::new(),
        model_backend: firstring::config::ModelBackend::NvidiaHosted,
        model_url: "https://integrate.api.nvidia.com/v1/chat/completions".into(),
        model: "nvidia/nemotron-3.5-lightning-30b-a3b".into(),
    }
}

async fn tool(h: &Handle, name: &str, arguments: Value) -> Value {
    let snapshot = h.send(Input::Snapshot).await.unwrap();
    h.send(Input::ModelTool {
        generation: snapshot["generation_id"].as_u64().unwrap(),
        epoch: snapshot["policy_epoch"].as_u64().unwrap(),
        version: snapshot["turn_version"].as_u64().unwrap(),
        deadline: Instant::now() + Duration::from_secs(10),
        tool: ModelToolCall {
            id: "fixture-native-tool-id".into(),
            name: name.into(),
            arguments,
            assistant_message: json!({"role":"assistant"}),
        },
    })
    .await
    .unwrap()
}

async fn pending_actor() -> (App, Handle, String) {
    let app = App::new(config()).await.unwrap();
    let (h, _) = app.incoming(CALLER).await.unwrap();
    let turn = h
        .send(Input::FinalPrompt {
            text: CALLER_TEXT.into(),
        })
        .await
        .unwrap();
    let evidence = json!([{"turn_id":turn["turn_id"],"quote":CALLER_TEXT}]);
    let assessment = tool(
        &h,
        "record_assessment",
        json!({"observation":benign(),"evidence_refs":evidence,"understanding":{"version":1,"claimed_identity":null,"purpose":"Share the clinic visit day","requested_action":"Pass along the Wednesday visit","supplied_facts":[{"text":"Wednesday","evidence_refs":evidence}],"missing_information":[],"readiness":"ready"}}),
    )
    .await;
    assert_eq!(assessment["ok"], true);
    let offer = tool(&h, "offer_handoff", json!({"summary":"The clinic visit is Wednesday. Identity unverified.","observation":benign(),"evidence_refs":evidence})).await;
    assert_eq!(offer["ok"], true, "{offer}");
    let snap = h.send(Input::Snapshot).await.unwrap();
    h.send(Input::Status {
        action_id: snap["offer"]["leg_id"].as_str().unwrap().into(),
        sid: SARAH.into(),
        status: "in-progress".into(),
    })
    .await
    .unwrap();
    (app, h, snap["offer"]["id"].as_str().unwrap().into())
}

async fn accept(h: &Handle, offer_id: &str) -> Value {
    h.send(Input::Choice {
        offer_id: offer_id.into(),
        sid: SARAH.into(),
        digits: "1".into(),
    })
    .await
    .unwrap()
}

async fn conference(h: &Handle, sid: &str, event: &str) {
    let s = h.send(Input::Snapshot).await.unwrap();
    h.send(Input::Conference {
        name: s["conference"].as_str().unwrap().into(),
        sid: sid.into(),
        event: event.into(),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn retried_private_acceptance_returns_same_room_without_new_action() {
    let (app, h, offer_id) = pending_actor().await;
    let first = accept(&h, &offer_id).await;
    assert!(first["twiml"].as_str().unwrap().contains("<Conference"));
    let duplicate = accept(&h, &offer_id).await;
    assert_eq!(
        duplicate["twiml"], first["twiml"],
        "webhook retry must not hang up the approved participant"
    );
    assert_eq!(app.store.actions(&h.id).await.unwrap().len(), 1);
    assert_eq!(
        app.store
            .events_after(0)
            .await
            .unwrap()
            .iter()
            .filter(|e| e.kind == "handoff.offered")
            .count(),
        1
    );
    let snapshot = h.send(Input::Snapshot).await.unwrap();
    assert_eq!(
        snapshot["bridge_a_requested"], false,
        "A moves only after B's signed join"
    );
}

#[tokio::test]
async fn delayed_private_join_after_timeout_cannot_move_caller() {
    let (_, h, offer_id) = pending_actor().await;
    accept(&h, &offer_id).await;
    h.send(Input::BridgeTimeout).await.unwrap();
    conference(&h, SARAH, "participant-join").await;
    let snapshot = h.send(Input::Snapshot).await.unwrap();
    assert_eq!(snapshot["state"], "bridge_failed");
    assert_eq!(snapshot["bridge_a_requested"], false);
    assert!(snapshot["joined"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn reordered_join_after_participant_leave_does_not_show_connected_again() {
    let (app, h, offer_id) = pending_actor().await;
    accept(&h, &offer_id).await;
    conference(&h, SARAH, "participant-join").await;
    conference(&h, CALLER, "participant-join").await;
    conference(&h, SARAH, "participant-leave").await;
    conference(&h, SARAH, "participant-join").await;
    let snapshot = h.send(Input::Snapshot).await.unwrap();
    assert_eq!(snapshot["state"], "ended");
    assert_eq!(
        app.store
            .events_after(0)
            .await
            .unwrap()
            .iter()
            .filter(|e| e.kind == "bridge.connected")
            .count(),
        1
    );
}

#[tokio::test]
async fn caller_hangup_withdraws_the_pending_private_prompt() {
    let (app, h, offer) = pending_actor().await;
    h.send(Input::InboundEnded).await.unwrap();
    let state = h.send(Input::Snapshot).await.unwrap();
    assert_eq!(state["offer"]["status"], "revoked");
    let response = h
        .send(Input::Whisper {
            offer_id: offer,
            sid: SARAH.into(),
        })
        .await
        .unwrap();
    assert!(!response["twiml"].as_str().unwrap().contains("<Gather"));
    assert!(
        app.store
            .events_after(0)
            .await
            .unwrap()
            .iter()
            .any(|e| e.kind == "handoff.revoked")
    );
}

#[tokio::test]
async fn parent_still_reconciles_the_independent_child_after_caller_hangup() {
    let (_, h, _) = pending_actor().await;
    let s = h.send(Input::Snapshot).await.unwrap();
    let caller_turn = s["turns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["speaker"] == "caller")
        .unwrap();
    let response=tool(&h,"start_trusted_callback",json!({"contact_id":"sarah:family","reason":"family_help","evidence_refs":[{"turn_id":caller_turn["id"],"quote":CALLER_TEXT}]})).await;
    assert_eq!(response["ok"], true);
    let action = response["result"]["action_id"]
        .as_str()
        .unwrap()
        .to_string();
    h.send(Input::InboundEnded).await.unwrap();
    let family = format!("CA{}", "f".repeat(32));
    h.send(Input::Status {
        action_id: action.clone(),
        sid: family.clone(),
        status: "in-progress".into(),
    })
    .await
    .unwrap();
    h.send(Input::Ack {
        action_id: action,
        sid: family,
        digits: "1".into(),
    })
    .await
    .unwrap();
    let s = h.send(Input::Snapshot).await.unwrap();
    assert_eq!(s["inbound_ended"], true);
    assert!(
        s["legs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["role"] == "family" && l["acknowledged"] == true)
    );
    assert_eq!(s["bridge_committed"], false);
}
