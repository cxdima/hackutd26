use firstring::{
    calls::{
        actor::{App, Input},
        state::{Leg, Session},
    },
    config::{Config, ExecutionMode},
    providers::twilio::{CreateOutcome, Twilio},
    store::Store,
};
use proptest::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

fn leg(id: &str) -> Leg {
    Leg {
        action_id: id.into(),
        role: "sarah".into(),
        kind: "probe".into(),
        status: "reserved".into(),
        acknowledged: false,
        call_sid: None,
    }
}

fn config(database_url: String) -> Config {
    Config {
        public_base_url: "https://example.invalid".into(),
        port: 8080,
        database_url,
        admin_token: "fictional-local-test-operator-token".into(),
        mode: ExecutionMode::Replay,
        transport: "probe".into(),
        account_sid: format!("AC{}", "0".repeat(32)),
        auth_token: "fictional-auth-token".into(),
        photon_number: "+12025550124".into(),
        sip_host: "sip.photon.codes".into(),
        sip_port: 5061,
        sip_username: "fictional-sip-user".into(),
        sip_password: "fictional-sip-password".into(),
        contacts: BTreeMap::from([("sarah".into(), "+12025550123".into())]),
        smoke_authorized: true,
        handoff_enabled: false,
        family_enabled: false,
        max_call_seconds: 180,
        nvidia_key: String::new(),
        model_url: "https://integrate.api.nvidia.com/v1/chat/completions".into(),
        model: "nvidia/nemotron-3.5-lightning-30b-a3b".into(),
    }
}

#[tokio::test]
async fn duplicate_reservation_returns_the_original_action_after_unknown_outcome() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    let (mut original, fresh) = store
        .reserve(
            "session-1",
            "session-1:probe",
            &leg("action-1"),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    assert!(fresh);
    original.status = "unknown".into();
    let mut session = Session::new(
        "session-1".into(),
        "CA-caller".into(),
        "sarah".into(),
        ExecutionMode::Live,
    );
    session.legs.push(original);
    store.save(&session).await.unwrap();
    let (duplicate, fresh) = store
        .reserve(
            "session-1",
            "session-1:probe",
            &leg("action-would-redial"),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    assert!(!fresh);
    assert_eq!(duplicate.action_id, "action-1");
    assert_eq!(duplicate.status, "unknown");
    assert_eq!(store.actions("session-1").await.unwrap().len(), 1);
}

#[tokio::test]
async fn the_single_live_smoke_budget_spans_distinct_sessions() {
    let store = Store::open("sqlite::memory:").await.unwrap();
    assert!(
        store
            .reserve(
                "session-1",
                "session-1:probe",
                &leg("action-1"),
                ExecutionMode::Live
            )
            .await
            .unwrap()
            .1
    );
    assert!(
        store
            .reserve(
                "session-2",
                "session-2:probe",
                &leg("action-2"),
                ExecutionMode::Live
            )
            .await
            .is_err()
    );
    assert!(store.actions("session-2").await.unwrap().is_empty());
}

#[tokio::test]
async fn replay_probe_never_reserves_an_external_action_even_with_credentials_present() {
    let app = App::new(config("sqlite::memory:".into())).await.unwrap();
    let (handle, _) = app.incoming("CA-caller").await.unwrap();
    assert!(
        handle
            .send(Input::Probe {
                contact_id: "sarah".into()
            })
            .await
            .is_err()
    );
    assert!(app.store.actions(&handle.id).await.unwrap().is_empty());
    assert!(matches!(
        app.twilio
            .create(&leg("replay-action"), "/voice/alert/replay-action")
            .await,
        CreateOutcome::Rejected { .. }
    ));
}

#[tokio::test]
async fn unlisted_contact_is_rejected_before_any_reservation() {
    let app = App::new(config("sqlite::memory:".into())).await.unwrap();
    let (handle, _) = app.incoming("CA-caller").await.unwrap();
    assert!(
        handle
            .send(Input::Probe {
                contact_id: "attacker-number".into()
            })
            .await
            .is_err()
    );
    assert!(app.store.actions(&handle.id).await.unwrap().is_empty());
}

#[test]
fn public_snapshot_redacts_ids_while_durable_serialization_preserves_leg_binding() {
    let mut session = Session::new(
        "session-1".into(),
        "CA-private-caller".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    let mut bound = leg("action-1");
    bound.apply_status("CA-private-sarah", "ringing").unwrap();
    session.legs.push(bound);
    let public = session.public_value();
    assert!(public.get("relay_token").is_none());
    assert!(public.get("inbound_sid").is_none());
    assert!(public.get("conference").is_none());
    assert!(public["legs"][0].get("call_sid").is_none());
    let recovered: Session =
        serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap();
    assert_eq!(
        recovered.legs[0].call_sid.as_deref(),
        Some("CA-private-sarah")
    );
}

#[test]
fn calls_api_form_carries_reserved_action_id_and_all_progress_events() {
    let provider = Twilio::new(Arc::new(config("sqlite::memory:".into()))).unwrap();
    let form = provider.call_form(
        &leg("action-reserved"),
        "+12025550123",
        "/voice/alert/action-reserved",
    );
    assert!(form.contains(&(
        "To".into(),
        "sip:+12025550123@sip.photon.codes:5061;transport=tls;secure=true".into()
    )));
    assert!(form.contains(&(
        "StatusCallback".into(),
        "https://example.invalid/voice/status?action_id=action-reserved".into()
    )));
    assert!(form.contains(&(
        "Url".into(),
        "https://example.invalid/voice/alert/action-reserved".into()
    )));
    let progress: Vec<_> = form
        .iter()
        .filter(|(key, _)| key == "StatusCallbackEvent")
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(progress, ["initiated", "ringing", "answered", "completed"]);
    let encoded = serde_urlencoded::to_string(&form).unwrap();
    assert!(encoded.contains("%2B12025550123"));
    assert!(!encoded.contains("To=sip:+"));
}

#[tokio::test]
async fn restart_recovers_an_unknown_action_without_granting_a_new_reservation() {
    let path = std::env::temp_dir().join(format!("firstring-recovery-{}.db", uuid::Uuid::new_v4()));
    let database = format!("sqlite:{}", path.display());
    let store = Store::open(&database).await.unwrap();
    let (mut original, _) = store
        .reserve(
            "session-1",
            "session-1:probe",
            &leg("action-1"),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    original.status = "unknown".into();
    let mut session = Session::new(
        "session-1".into(),
        "CA-caller".into(),
        "sarah".into(),
        ExecutionMode::Live,
    );
    session.legs.push(original);
    store.save(&session).await.unwrap();
    store.pool.close().await;
    let recovered = Store::open(&database).await.unwrap();
    let mut action = recovered.actions("session-1").await.unwrap().remove(0);
    assert_eq!(action.status, "unknown");
    action.apply_status("CA-reconciled", "completed").unwrap();
    let mut session = recovered.sessions().await.unwrap().remove(0);
    session.legs = vec![action];
    recovered.save(&session).await.unwrap();
    let (same, fresh) = recovered
        .reserve(
            "session-1",
            "session-1:probe",
            &leg("action-redial"),
            ExecutionMode::Live,
        )
        .await
        .unwrap();
    assert!(!fresh);
    assert_eq!(same.call_sid.as_deref(), Some("CA-reconciled"));
    assert_eq!(same.status, "completed");
    assert!(
        recovered
            .reserve(
                "session-2",
                "session-2:probe",
                &leg("action-other"),
                ExecutionMode::Live
            )
            .await
            .is_err()
    );
    recovered.pool.close().await;
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn a_leg_bound_after_operator_end_cannot_receive_a_new_private_gather() {
    let path =
        std::env::temp_dir().join(format!("firstring-late-bind-{}.db", uuid::Uuid::new_v4()));
    let database = format!("sqlite:{}", path.display());
    let store = Store::open(&database).await.unwrap();
    let (action, _) = store
        .reserve(
            "session-1",
            "session-1:probe",
            &leg("action-1"),
            ExecutionMode::Replay,
        )
        .await
        .unwrap();
    let mut session = Session::new(
        "session-1".into(),
        "CA-caller".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    session.legs.push(action);
    store.save(&session).await.unwrap();
    store.pool.close().await;
    let app = App::new(config(database)).await.unwrap();
    let handle = app.handle("session-1").await.unwrap();
    handle.send(Input::End).await.unwrap();
    handle
        .send(Input::Status {
            action_id: "action-1".into(),
            sid: "CA-late".into(),
            status: "in-progress".into(),
        })
        .await
        .unwrap();
    let response = handle
        .send(Input::Alert {
            action_id: "action-1".into(),
            sid: "CA-late".into(),
        })
        .await
        .unwrap();
    let twiml = response["twiml"].as_str().unwrap();
    assert!(
        !twiml.contains("<Gather"),
        "An operator-ended action must not start a new private prompt"
    );
    assert!(twiml.contains("<Hangup"));
    app.store.pool.close().await;
    std::fs::remove_file(path).unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 96,
        failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::Direct("tests/regression.proptest-regressions"))),
        ..ProptestConfig::default()
    })]
    #[test]
    fn terminal_callback_is_absorbing_and_never_invents_acknowledgement(
        terminal_index in 0usize..5,
        later in prop::collection::vec(0usize..9, 0..64),
    ) {
        let terminals = ["completed", "busy", "failed", "no-answer", "canceled"];
        let statuses = ["queued", "initiated", "ringing", "in-progress", "completed", "busy", "failed", "no-answer", "canceled"];
        let expected = terminals[terminal_index];
        let mut leg = leg("action-1");
        leg.apply_status("CA-bound", expected).unwrap();
        for index in later {
            leg.apply_status("CA-bound", statuses[index]).unwrap();
            prop_assert_eq!(&leg.status, expected);
            prop_assert!(!leg.acknowledged);
            prop_assert!(leg.apply_status("CA-wrong", statuses[index]).is_err());
            prop_assert_eq!(leg.call_sid.as_deref(), Some("CA-bound"));
        }
    }
}
