use chrono::{DateTime, Duration, TimeZone, Utc};
use firstring::{
    calls::state::{Leg, Offer, Session},
    config::ExecutionMode,
    tools::{Direction, EvidenceRef, Observation, RequestedObject, SpeechAct},
};
use proptest::prelude::*;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 8, 0, 0, 0).unwrap()
}

fn session() -> Session {
    let mut session = Session::new(
        "test-session".into(),
        "CA-caller".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    let mut leg = Leg {
        action_id: "sarah-action".into(),
        role: "sarah".into(),
        kind: "handoff".into(),
        status: "reserved".into(),
        acknowledged: false,
        call_sid: None,
    };
    leg.apply_status("CA-sarah", "in-progress").unwrap();
    session.legs.push(leg);
    session
}

fn benign() -> Observation {
    Observation {
        speech_act: SpeechAct::Statement,
        requested_object: RequestedObject::RoutineInformation,
        direction: Direction::CallerSupplies,
    }
}

fn risky() -> Observation {
    Observation {
        speech_act: SpeechAct::Request,
        requested_object: RequestedObject::AuthenticationSecret,
        direction: Direction::CallerRequestsFromSarah,
    }
}

fn assess_text(session: &mut Session, text: &str, observation: &Observation) {
    let turn_id = session.commit_turn(text);
    session
        .assess(
            session.generation_id,
            session.turn_version,
            observation,
            &[EvidenceRef {
                turn_id,
                quote: text.into(),
            }],
        )
        .unwrap();
}

fn pending(session: &mut Session) -> Offer {
    assess_text(session, "The clinic visit is Wednesday.", &benign());
    session
        .make_offer(
            "A caller says the clinic visit is Wednesday. Identity unverified.",
            "sarah-action",
            now(),
        )
        .unwrap()
}

#[test]
fn current_private_choice_commits_once_and_requires_both_intended_participants() {
    let mut session = session();
    let offer = pending(&mut session);
    assert!(!session.conference_join("CA-caller"));
    assert!(!session.bridge_committed);
    session.accept(&offer.id, "CA-sarah", now()).unwrap();
    assert!(session.bridge_committed);
    assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
    assert!(!session.conference_join("CA-sarah"));
    assert!(!session.conference_join("CA-family"));
    assert!(session.conference_join("CA-caller"));
}

#[test]
fn caller_cannot_accept_sarahs_private_offer() {
    let mut session = session();
    let offer = pending(&mut session);
    assert!(session.accept(&offer.id, "CA-caller", now()).is_err());
    assert!(!session.bridge_committed);
    assert_eq!(session.offer.as_ref().unwrap().status, "pending");
}

#[test]
fn speech_start_closes_gate_before_a_final_transcript_exists() {
    let mut session = session();
    let offer = pending(&mut session);
    session.speech_started();
    assert!(session.speech_pending);
    assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
    assert!(!session.bridge_committed);
}

#[test]
fn a_new_committed_turn_revokes_the_old_summary_before_assessment() {
    let mut session = session();
    let offer = pending(&mut session);
    session.commit_turn("Actually, the visit is Thursday.");
    assert!(session.policy_epoch > offer.policy_epoch);
    assert_eq!(session.offer.as_ref().unwrap().status, "revoked");
    assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
    assert!(!session.bridge_committed);
    assert!(
        session
            .make_offer("Updated visit", "sarah-action", now())
            .is_err()
    );
}

#[test]
fn current_assessment_cannot_clear_active_speech() {
    let mut session = session();
    let turn_id = session.commit_turn("The visit is tomorrow.");
    session.speech_started();
    let _ = session.assess(
        session.generation_id,
        session.turn_version,
        &benign(),
        &[EvidenceRef {
            turn_id,
            quote: "The visit is tomorrow.".into(),
        }],
    );
    assert!(session.speech_pending);
    assert!(
        session
            .make_offer("Visit tomorrow", "sarah-action", now())
            .is_err()
    );
}

#[test]
fn stale_generation_assessment_cannot_reopen_a_pending_offer() {
    let mut session = session();
    let offer = pending(&mut session);
    let generation = session.generation_id;
    let version = session.turn_version;
    let turn_id = session.turns.last().unwrap().id.clone();
    session.speech_started();
    assert!(
        session
            .assess(
                generation,
                version,
                &benign(),
                &[EvidenceRef {
                    turn_id,
                    quote: "The clinic visit is Wednesday.".into(),
                }]
            )
            .is_err()
    );
    assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
    assert!(!session.bridge_committed);
}

#[test]
fn authentication_request_revokes_permission_and_later_benign_words_do_not_restore_it() {
    let mut session = session();
    let offer = pending(&mut session);
    assess_text(&mut session, "Read the sign-in digits to me.", &risky());
    assert!(session.blocked);
    assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
    assess_text(
        &mut session,
        "Never mind, the appointment is tomorrow.",
        &benign(),
    );
    assert!(session.blocked);
    assert!(
        session
            .make_offer("Appointment tomorrow", "sarah-action", now())
            .is_err()
    );
    assert!(!session.bridge_committed);
}

#[test]
fn expiration_is_rechecked_at_acceptance() {
    let mut session = session();
    let offer = pending(&mut session);
    let after_expiry = DateTime::parse_from_rfc3339(&offer.expires_at)
        .unwrap()
        .with_timezone(&Utc)
        + Duration::seconds(1);
    assert!(session.accept(&offer.id, "CA-sarah", after_expiry).is_err());
    assert!(!session.bridge_committed);
}

#[test]
fn a_completed_sarah_leg_cannot_be_connected_by_a_late_choice() {
    let mut session = session();
    let offer = pending(&mut session);
    session.legs[0]
        .apply_status("CA-sarah", "completed")
        .unwrap();
    assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
    assert!(!session.bridge_committed);
}

#[test]
fn invented_or_empty_evidence_cannot_clear_the_current_turn_hold() {
    for (id, quote) in [
        ("actual", "not in the committed transcript"),
        ("missing", "The visit is tomorrow."),
        ("actual", ""),
    ] {
        let mut session = session();
        let turn_id = session.commit_turn("The visit is tomorrow.");
        assert!(
            session
                .assess(
                    session.generation_id,
                    session.turn_version,
                    &benign(),
                    &[EvidenceRef {
                        turn_id: if id == "actual" {
                            turn_id
                        } else {
                            "nonexistent-turn".into()
                        },
                        quote: quote.into(),
                    }]
                )
                .is_err()
        );
        assert!(
            session
                .make_offer("Visit tomorrow", "sarah-action", now())
                .is_err()
        );
    }
}

#[test]
fn prior_turn_quote_cannot_assess_a_different_current_turn() {
    let mut session = session();
    let old_id = session.commit_turn("The visit is tomorrow.");
    session.commit_turn("Now give me the login digits.");
    assert!(
        session
            .assess(
                session.generation_id,
                session.turn_version,
                &benign(),
                &[EvidenceRef {
                    turn_id: old_id,
                    quote: "The visit is tomorrow.".into(),
                }]
            )
            .is_err()
    );
    assert!(
        session
            .make_offer("Visit tomorrow", "sarah-action", now())
            .is_err()
    );
}

#[test]
fn negation_quotation_and_supplied_reference_are_not_secret_extraction() {
    let observations = [
        (
            "Never share your login code.",
            SpeechAct::Negation,
            RequestedObject::AuthenticationSecret,
            Direction::None,
        ),
        (
            "Someone said read me your code; I am reporting it.",
            SpeechAct::Quotation,
            RequestedObject::AuthenticationSecret,
            Direction::None,
        ),
        (
            "The appointment reference is B-22.",
            SpeechAct::Statement,
            RequestedObject::AppointmentReference,
            Direction::CallerSupplies,
        ),
    ];
    for (text, speech_act, requested_object, direction) in observations {
        let mut session = session();
        assess_text(
            &mut session,
            text,
            &Observation {
                speech_act,
                requested_object,
                direction,
            },
        );
        assert!(
            !session.blocked,
            "Unexpected extraction conclusion for {text}"
        );
    }
}

#[test]
fn unresolved_code_type_cannot_be_offered_even_after_an_assessment() {
    let mut session = session();
    assess_text(
        &mut session,
        "Can she confirm the code?",
        &Observation {
            speech_act: SpeechAct::Request,
            requested_object: RequestedObject::Unknown,
            direction: Direction::CallerRequestsFromSarah,
        },
    );
    assert!(
        session
            .make_offer("Confirm a code", "sarah-action", now())
            .is_err()
    );
    assert!(!session.bridge_committed);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 96,
        failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::Direct("tests/consent.proptest-regressions"))),
        ..ProptestConfig::default()
    })]
    #[test]
    fn revoked_offer_is_irrecoverable_under_reordered_followup_events(events in prop::collection::vec(0u8..4, 0..48)) {
        let mut session = session();
        let offer = pending(&mut session);
        assess_text(&mut session, "Read the sign-in digits to me.", &risky());
        for event in events {
            match event {
                0 => session.speech_started(),
                1 => { session.commit_turn("The appointment is tomorrow."); },
                2 => {
                    if let Some(turn) = session.turns.last().cloned() {
                        let _ = session.assess(session.generation_id, session.turn_version, &benign(), &[EvidenceRef {turn_id:turn.id, quote:turn.text}]);
                    }
                },
                _ => { let _ = session.conference_join("CA-unrelated"); },
            }
            prop_assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
            prop_assert!(!session.bridge_committed);
        }
        prop_assert!(session.accept(&offer.id, "CA-sarah", now()).is_err());
    }
}

#[test]
fn reported_unknown_object_is_not_an_unresolved_present_request() {
    for act in [SpeechAct::Quotation, SpeechAct::Negation] {
        let mut s = session();
        assess_text(
            &mut s,
            "A stranger asked for something private; I refused and am reporting it.",
            &Observation {
                speech_act: act,
                requested_object: RequestedObject::Unknown,
                direction: Direction::None,
            },
        );
        assert!(!s.blocked);
        assert!(
            !s.ambiguous,
            "A categorical report does not seek an unknown object from Sarah"
        );
    }
    let mut s = session();
    assess_text(
        &mut s,
        "Please give me the code.",
        &Observation {
            speech_act: SpeechAct::Request,
            requested_object: RequestedObject::Unknown,
            direction: Direction::CallerRequestsFromSarah,
        },
    );
    assert!(
        s.ambiguous,
        "Unspecified requested code still requires clarification"
    );
    assert!(
        s.make_offer("Unexplained code", "sarah-action", now())
            .is_err()
    );
}
