use firstring::calls::state::Leg;
fn leg() -> Leg {
    Leg {
        action_id: "action".into(),
        role: "sarah".into(),
        kind: "probe".into(),
        status: "reserved".into(),
        acknowledged: false,
        call_sid: None,
    }
}
#[test]
fn callback_can_reconcile_lost_create_response_and_terminal_never_regresses() {
    let mut l = leg();
    assert!(l.apply_status("CA1", "completed").unwrap());
    assert_eq!(l.call_sid.as_deref(), Some("CA1"));
    assert!(!l.apply_status("CA1", "ringing").unwrap());
    assert_eq!(l.status, "completed");
    assert!(l.apply_status("CA2", "in-progress").is_err());
    assert!(!l.acknowledged);
}
#[test]
fn correct_leg_keypad_is_only_acknowledgement_source() {
    let mut l = leg();
    l.apply_status("CA1", "in-progress").unwrap();
    assert!(!l.acknowledge("CA2", "1"));
    assert!(!l.acknowledge("CA1", "2"));
    assert!(!l.acknowledged);
    assert!(l.acknowledge("CA1", "1"));
    assert!(l.acknowledged);
    assert!(!l.acknowledge("CA1", "1"));
}
#[test]
fn late_answer_does_not_invent_acknowledgement() {
    let mut l = leg();
    l.apply_status("CA1", "no-answer").unwrap();
    l.apply_status("CA1", "in-progress").unwrap();
    assert_eq!(l.status, "no-answer");
    assert!(!l.acknowledge("CA1", "1"));
    assert!(!l.acknowledged);
}
