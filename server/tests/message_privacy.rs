//! Message minimization is separate from semantic assessment and call policy.
use chrono::Utc;
use firstring::{
    calls::state::Session,
    config::ExecutionMode,
    tools::{Direction, EvidenceRef, Observation, RequestedObject, SpeechAct},
};

fn assessed(
    text: &str,
    act: SpeechAct,
    object: RequestedObject,
    direction: Direction,
) -> (Session, Vec<EvidenceRef>) {
    let mut session = Session::new(
        "privacy-fixture".into(),
        "no-provider".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    let id = session.commit_turn(text);
    let evidence = vec![EvidenceRef {
        turn_id: id,
        quote: text.into(),
    }];
    session
        .assess(
            session.generation_id,
            session.turn_version,
            &Observation {
                speech_act: act,
                requested_object: object,
                direction,
            },
            &evidence,
        )
        .unwrap();
    (session, evidence)
}

#[test]
fn ordinary_times_dates_quantities_and_prices_survive_message_storage() {
    for text in [
        "The appointment is Tuesday at 2:30 PM on 11/09; arrive 10 minutes early with a $25 copay.",
        "Delivery is 2026-11-09 at 14:30; there are 12 boxes and 250 items.",
        "The appointment moved to 11/09/2026 at 9 AM.",
    ] {
        let (mut session, evidence) = assessed(
            text,
            SpeechAct::Statement,
            RequestedObject::RoutineInformation,
            Direction::CallerSupplies,
        );
        session.record_message(text, &evidence, Utc::now()).unwrap();
        assert_eq!(session.message.unwrap().summary, text);
    }
}

#[test]
fn phone_and_email_literals_are_redacted_without_losing_adjacent_numeric_facts() {
    let text = "Visit 11/09 at 2:30 PM, allow 10 minutes, $25 copay. Reply at person@example.test or +15550123456.";
    let (mut session, evidence) = assessed(
        text,
        SpeechAct::Statement,
        RequestedObject::RoutineInformation,
        Direction::CallerSupplies,
    );
    session.record_message(text, &evidence, Utc::now()).unwrap();
    let saved = session.message.unwrap().summary;
    for useful in ["11/09", "2:30 PM", "10 minutes", "$25 copay"] {
        assert!(saved.contains(useful), "Lost {useful}: {saved}");
    }
    assert!(!saved.contains('@') && !saved.contains("15550123456"));
    assert_eq!(saved.matches("[redacted]").count(), 2);
    for phone in [
        "(202) 555-0123",
        "+1 202 555 0123",
        "202-555-0123",
        "2025550123",
        "+44 20 7946 0958",
    ] {
        let text = format!("Call {phone} about the 2:30 PM appointment.");
        let (mut session, evidence) = assessed(
            &text,
            SpeechAct::Statement,
            RequestedObject::RoutineInformation,
            Direction::CallerSupplies,
        );
        session
            .record_message(&text, &evidence, Utc::now())
            .unwrap();
        assert_eq!(
            session.message.unwrap().summary,
            "Call [redacted] about the 2:30 PM appointment."
        );
    }
}

#[test]
fn accepted_warning_and_quoted_authentication_report_are_minimized_without_changing_policy() {
    for (act, text, expected) in [
        (
            SpeechAct::Negation,
            "Never share your password or sign-in code.",
            "Caller shared a warning about protecting account information.",
        ),
        (
            SpeechAct::Quotation,
            "Someone told me to share password red-fox-731 and recovery code 684921.",
            "Caller reported a discussion involving sensitive account information.",
        ),
    ] {
        let (mut session, evidence) = assessed(
            text,
            act,
            RequestedObject::AuthenticationSecret,
            Direction::None,
        );
        let blocked = session.blocked;
        let epoch = session.policy_epoch;
        session.record_message(text, &evidence, Utc::now()).unwrap();
        assert_eq!(session.message.as_ref().unwrap().summary, expected);
        assert!(
            !session
                .message
                .as_ref()
                .unwrap()
                .summary
                .contains("red-fox")
                && !session.message.as_ref().unwrap().summary.contains("684921")
        );
        assert_eq!(session.blocked, blocked);
        assert_eq!(session.policy_epoch, epoch);
        assert!(
            !session.blocked,
            "Privacy handling must not classify a warning or report as risky"
        );
    }
}

#[test]
fn actual_sensitive_disclosure_is_never_copied_even_without_a_keyword_in_summary() {
    let text = "The password is red-fox-731.";
    let (mut session, evidence) = assessed(
        text,
        SpeechAct::Statement,
        RequestedObject::AuthenticationSecret,
        Direction::CallerSupplies,
    );
    session
        .record_message("The value is red-fox-731.", &evidence, Utc::now())
        .unwrap();
    assert_eq!(
        session.message.unwrap().summary,
        "Caller discussed sensitive account information; sensitive details omitted."
    );
}

#[test]
fn authentication_details_cannot_be_smuggled_into_an_ordinary_message() {
    let text = "Please reschedule the appointment.";
    for summary in [
        "Caller supplied password correcthorsebatterystaple.",
        "Caller supplied passwords correcthorsebatterystaple.",
        "Caller supplied a recovery code correcthorsebatterystaple.",
    ] {
        let (mut session, evidence) = assessed(
            text,
            SpeechAct::Request,
            RequestedObject::RoutineInformation,
            Direction::CallerRequestsFromSarah,
        );
        assert!(
            session
                .record_message(summary, &evidence, Utc::now())
                .is_err()
        );
        assert!(session.message.is_none());
    }
}

#[test]
fn later_routine_turn_cannot_launder_an_earlier_disclosed_secret() {
    for value in ["684921", "violet-bird"] {
        let text = format!("My authentication password is {value}.");
        let (mut session, _) = assessed(
            &text,
            SpeechAct::Statement,
            RequestedObject::AuthenticationSecret,
            Direction::CallerSupplies,
        );
        let latest = "Please include that in the note.";
        let evidence = vec![EvidenceRef {
            turn_id: session.commit_turn(latest),
            quote: latest.into(),
        }];
        session
            .assess(
                session.generation_id,
                session.turn_version,
                &Observation {
                    speech_act: SpeechAct::Request,
                    requested_object: RequestedObject::RoutineInformation,
                    direction: Direction::CallerRequestsFromSarah,
                },
                &evidence,
            )
            .unwrap();
        session
            .record_message(&format!("The value is {value}."), &evidence, Utc::now())
            .unwrap();
        assert!(!session.message.unwrap().summary.contains(value));
        assert!(
            !session.blocked,
            "Privacy minimization is not a risky-request classification"
        );
    }
}
