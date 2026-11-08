use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::collections::{BTreeMap, BTreeSet};

/// Twilio form signatures include the exact external URL (query included), followed
/// by sorted decoded keys and sorted unique values. Never derive this URL from Host.
/// Conformance vectors come from Twilio's official request validator and SDK tests.
pub fn validate(token: &str, url: &str, form: &[(String, String)], signature: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let Ok(bytes) = STANDARD.decode(signature) else {
        return false;
    };
    let mut mac =
        Hmac::<Sha1>::new_from_slice(token.as_bytes()).expect("HMAC accepts arbitrary key length");
    mac.update(url.as_bytes());
    let mut params: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (key, value) in form {
        params.entry(key).or_default().insert(value);
    }
    for (key, values) in params {
        for value in values {
            mac.update(key.as_bytes());
            mac.update(value.as_bytes());
        }
    }
    mac.verify_slice(&bytes).is_ok()
}
