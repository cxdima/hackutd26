//! Operator-reviewed evidence receipts, separate from configuration and replay.
//! The import helper records observations; credential presence cannot create proof.
use crate::{
    config::{Config, ExecutionMode},
    store::Store,
};
use serde_json::{Value, json};

async fn receipt(
    store: &Store,
    key: &str,
    source: &str,
    checks: &[&str],
    detail: &str,
) -> anyhow::Result<Value> {
    let stored = store.metadata(&format!("verification.{key}")).await?;
    let value: Value = stored
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    let dated = value["recorded_at"]
        .as_str()
        .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok());
    let passed = value["status"] == "passed"
        && value["source"] == source
        && dated
        && checks.iter().all(|name| value["checks"][name] == true);
    if key == "relay_phone"
        && value["status"] == "partial"
        && value["source"] == source
        && dated
        && ["signed_setup", "caller_transcript", "audible_greeting"]
            .iter()
            .all(|name| value["checks"][name] == true)
    {
        return Ok(
            json!({"status":"pending","recorded_at":value["recorded_at"],"source":source,
            "detail":"Signed Relay setup, caller transcription and audible greeting confirmed. A completed model reply, interruption and clean closing still need phone proof."}),
        );
    }
    if key == "relay_phone"
        && value["status"] == "failed"
        && value["source"] == source
        && dated
        && value["error_code"]
            .as_u64()
            .is_some_and(|code| (64101..=64120).contains(&code) || code == 39001 || code == 11200)
    {
        let detail = if value["error_code"] == 11200 {
            "Last recorded Relay attempt failed to retrieve the voice webhook (Twilio 11200). A successful audible conversation is still unverified.".to_string()
        } else {
            format!(
                "Last recorded Relay attempt failed with Twilio error {}. A successful audible conversation is still unverified.",
                value["error_code"]
            )
        };
        return Ok(
            json!({"status":"failed","recorded_at":value["recorded_at"],"source":source,
            "detail":detail}),
        );
    }
    let mut result = json!({"status":if passed {"passed"} else {"pending"},
        "detail":if passed {detail} else {"Required observations have not all been recorded; configuration is not proof."},
        "recorded_at":if passed {value["recorded_at"].clone()} else {Value::Null}});
    if passed {
        result["source"] = json!(source);
    }
    Ok(result)
}

pub async fn view(store: &Store) -> anyhow::Result<Value> {
    Ok(json!({
        "model_protocol":receipt(store,"model_protocol","hosted_model_probe",&["authenticated","expected_model","native_tool_results"],"Recorded hosted NVIDIA native tool/result probe passed. This is protocol evidence, not a behavioral quality score.").await?,
        "inbound_phone":receipt(store,"inbound_phone","manual_inbound_probe",&["signed_inbound","caller_keypad","audible_greeting","terminal_reconciled"],"Recorded real Photon/Twilio inbound greeting, caller keypad and completed call; audible greeting confirmed by the caller.").await?,
        "relay_phone":receipt(store,"relay_phone","manual_relay_rehearsal",&["signed_setup","caller_transcript","native_model_reply","audible_reply","interrupt","obsolete_response_rejected","clean_hangup"],"Recorded signed Relay conversation, native model reply, caller-confirmed audio, interruption/cancellation and clean hangup.").await?
    }))
}

pub async fn phone_b(config: &Config, store: &Store) -> anyhow::Result<Value> {
    let used: i64 =
        sqlx::query_scalar("SELECT count(*) FROM actions WHERE kind='probe' AND mode='live'")
            .fetch_one(&store.pool)
            .await?;
    let enrolled = config.contacts.contains_key("sarah");
    let enabled = config.outbound_configured();
    let authorized = config.smoke_authorized;
    let remaining = usize::from(used == 0);
    let reason = if used > 0 {
        "The one attempt is reserved or spent; reconcile its outcome."
    } else if !enrolled {
        "Designate a separate consenting Sarah / Phone B endpoint."
    } else if config.mode != ExecutionMode::Live {
        "Replay cannot place calls."
    } else if !enabled {
        "Outbound SIP configuration is incomplete."
    } else if !authorized {
        "Outbound smoke calling is disabled in this runtime."
    } else {
        "One reserved-before-submit smoke attempt is available."
    };
    Ok(
        json!({"enrolled":enrolled,"enabled":enabled,"authorized":authorized,
        "remaining_attempts":remaining,"can_request":enrolled && enabled && authorized && remaining == 1 && config.mode == ExecutionMode::Live,
        "reason":reason}),
    )
}
