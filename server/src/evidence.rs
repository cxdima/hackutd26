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
        let detail = if ["native_model_reply", "audible_reply", "clean_hangup"]
            .iter()
            .all(|name| value["checks"][name] == true)
        {
            "Live NVIDIA replies and automatic goodbye confirmed. Deliberate interruption and obsolete-response rejection still need phone proof."
        } else {
            "Signed Relay setup, caller transcription and audible greeting confirmed. A completed model reply, interruption and clean closing still need phone proof."
        };
        return Ok(
            json!({"status":"pending","recorded_at":value["recorded_at"],"source":source,
            "detail":detail}),
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

/// A reviewed GPU receipt must also reference an actual ordered live cycle.
/// Endpoint/GPU/authentication provenance comes from the operator receipt; local
/// events establish the same session, generation, model and accepted results.
async fn live_brev_cycle(store: &Store, value: &Value, model: &str) -> anyhow::Result<bool> {
    let cycle = &value["live_cycle"];
    let (Some(session_id), Some(action_id), Some(generation), Some(turn)) = (
        cycle["session_id"].as_str(),
        cycle["fixture_action_id"].as_str(),
        cycle["generation_id"].as_u64(),
        cycle["turn_version"].as_u64(),
    ) else {
        return Ok(false);
    };
    if turn == 0 {
        return Ok(false);
    }
    let session: Option<String> = sqlx::query_scalar("SELECT data FROM sessions WHERE id=?")
        .bind(session_id)
        .fetch_optional(&store.pool)
        .await?;
    let session: Value = session
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or(Value::Null);
    if session["execution_mode"] != "live" {
        return Ok(false);
    }
    let action_exists: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM actions WHERE id=? AND kind='fixture_caller' AND mode='live'",
    )
    .bind(action_id)
    .fetch_one(&store.pool)
    .await?;
    let run: Value = store
        .metadata(&format!("audio-fixture.run.v2:{action_id}"))
        .await?
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or(Value::Null);
    if action_exists != 1
        || run["leg"]["action_id"] != action_id
        || !session["inbound_sid"].is_string()
        || run["inbound_sid"] != session["inbound_sid"]
    {
        return Ok(false);
    }
    let Some(requests) = cycle["request_event_ids"]
        .as_array()
        .filter(|v| v.len() == 2)
    else {
        return Ok(false);
    };
    let mut events = Vec::with_capacity(4);
    for id in [
        &requests[0],
        &cycle["assessment_result_event_id"],
        &requests[1],
        &cycle["action_result_event_id"],
    ] {
        let Some(id) = id.as_str() else {
            return Ok(false);
        };
        let raw: Option<String> = sqlx::query_scalar("SELECT data FROM events WHERE json_extract(data,'$.id')=? AND json_extract(data,'$.session_id')=? LIMIT 1")
            .bind(id).bind(session_id).fetch_optional(&store.pool).await?;
        let event: Value = raw
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or(Value::Null);
        if event["execution_mode"] != "live"
            || event["payload"]["generation_id"] != generation
            || event["payload"]["turn_version"] != turn
            || event["sequence"].as_i64().is_none()
        {
            return Ok(false);
        }
        events.push(event);
    }
    if !events
        .windows(2)
        .all(|pair| pair[0]["sequence"].as_i64() < pair[1]["sequence"].as_i64())
    {
        return Ok(false);
    }
    for event in [&events[0], &events[2]] {
        let observation = &event["payload"]["observation"];
        if event["type"] != "model.request.completed"
            || observation["backend"] != "brev_hosted"
            || observation["outcome"] != "success"
            || observation["requested_model"] != model
            || observation["response_model"] != model
            || observation["content_chars"] != 0
            || !matches!(
                observation["finish_reason"].as_str(),
                Some("stop" | "tool_calls")
            )
        {
            return Ok(false);
        }
    }
    let assessment = &events[1];
    let action = &events[3];
    Ok(assessment["type"] == "agent.tool.result"
        && assessment["payload"]["name"] == "record_assessment"
        && assessment["payload"]["ok"] == true
        && assessment["payload"]["result"]["assessed_version"] == turn
        && action["type"] == "agent.tool.result"
        && action["payload"]["ok"] == true
        && action["payload"]["name"] == cycle["action_name"]
        && matches!(
            cycle["action_name"].as_str(),
            Some(
                "ask_caller"
                    | "take_message"
                    | "end_call"
                    | "offer_handoff"
                    | "call_trusted_contact"
            )
        ))
}

/// Historical hosted receipts never verify a different deployment. Brev proof
/// additionally requires the operator's GPU-loading/native-cycle receipt.
pub async fn model_backend(config: &Config, store: &Store) -> anyhow::Result<Value> {
    use crate::config::ModelBackend;
    let (label, protocol_verified) = match config.model_backend {
        ModelBackend::NvidiaHosted => (
            "NVIDIA-hosted Nemotron",
            config.model_url == "https://integrate.api.nvidia.com/v1/chat/completions"
                && config.model == "nvidia/nemotron-3.5-lightning-30b-a3b"
                && view(store).await?["model_protocol"]["status"] == "passed",
        ),
        ModelBackend::BrevHosted => {
            let stored: Value = store
                .metadata("verification.brev_model_protocol")
                .await?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(Value::Null);
            let source_valid = stored["source"] == "brev_gpu_model_probe"
                || (stored["source"] == "brev_gpu_live_tool_cycle"
                    && live_brev_cycle(store, &stored, &config.model).await?);
            let passed = stored["status"] == "passed"
                && source_valid
                && stored["model"] == config.model
                && stored["endpoint_sha256"]
                    == crate::config::endpoint_fingerprint(&config.model_url)
                && stored["recorded_at"]
                    .as_str()
                    .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
                && [
                    "loaded_on_gpu",
                    "authenticated",
                    "native_assessment",
                    "native_tool_results",
                    "action_selection",
                ]
                .iter()
                .all(|c| stored["checks"][c] == true);
            ("Brev-hosted Nemotron", passed)
        }
    };
    Ok(
        json!({"backend":config.model_backend,"label":label,"requested_model":config.model,"configured":config.model_configured(),"protocol_verified":protocol_verified,"detail":if protocol_verified {"Recorded native tool-cycle evidence; behavioral quality and live phone proof remain separate."} else {"Configuration selected; this backend's native tool cycle has not been verified."}}),
    )
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
