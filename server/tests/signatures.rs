use firstring::providers::signature::validate;

#[test]
fn official_twilio_vector_accepts_exact_url_and_rejects_tampering() {
    let params = [
        ("CallSid", "CA1234567890ABCDE"),
        ("Caller", "+14158675310"),
        ("Digits", "1234"),
        ("From", "+14158675310"),
        ("To", "+18005551212"),
    ]
    .map(|(k, v)| (k.into(), v.into()));
    let url = "https://example.com/myapp.php?foo=1&bar=2";
    let sig = "L/OH5YylLD5NRKLltdqwSvS0BnU=";
    assert!(validate("12345", url, &params, sig));
    assert!(!validate("wrong", url, &params, sig));
    assert!(!validate(
        "12345",
        "https://attacker.example/myapp.php?foo=1&bar=2",
        &params,
        sig
    ));
    assert!(!validate(
        "12345",
        "https://example.com/myapp.php?bar=2&foo=1",
        &params,
        sig
    ));
    let mut changed = params.to_vec();
    changed.push(("NewParameter".into(), "untrusted".into()));
    assert!(!validate("12345", url, &changed, sig));
    assert!(!validate("12345", url, &params, ""));
}
#[test]
fn official_python_sdk_multi_value_vector_sorts_and_deduplicates() {
    let params = [
        ("Sid", "CA123"),
        ("SidAccount", "AC123"),
        ("Digits", "5678"),
        ("Digits", "1234"),
        ("Digits", "1234"),
    ]
    .map(|(k, v)| (k.into(), v.into()));
    assert!(validate(
        "12345",
        "https://mycompany.com/myapp.php?foo=1&bar=2",
        &params,
        "IK+Dwps556ElfBT0I3Rgjkr1wJU="
    ));
}
