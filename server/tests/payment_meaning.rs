use firstring::{
    calls::state::Session,
    config::ExecutionMode,
    tools::{EvidenceRef, Observation},
};
use serde_json::json;
fn assessed(object: &str, text: &str) -> Session {
    let mut s = Session::new(
        "test".into(),
        "caller".into(),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    let id = s.commit_turn(text);
    let observation:Observation=serde_json::from_value(json!({"speech_act":"request","requested_object":object,"direction":"caller_requests_from_sarah"})).unwrap();
    s.assess(
        s.generation_id,
        s.turn_version,
        &observation,
        &[EvidenceRef {
            turn_id: id,
            quote: text.into(),
        }],
    )
    .unwrap();
    s
}
#[test]
fn ordinary_copay_request_is_not_payment_diversion() {
    let s = assessed(
        "payment",
        "Please bring your usual copay to the clinic desk.",
    );
    assert!(!s.blocked);
}
#[test]
fn diverted_payment_request_has_a_distinct_protective_observation() {
    let s = assessed(
        "payment_diversion",
        "Buy a gift card and read its number to me.",
    );
    assert!(s.blocked);
}
