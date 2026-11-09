//! Owner voice enrollment over a real phone call, and replies spoken in that voice.
//!
//! Flow: the operator arms enrollment with explicit consent; the next signed
//! inbound call hears a short instruction, the caller reads the passage shown on
//! the console after the tone, and Twilio records only the caller. The recording
//! is sent to ElevenLabs Instant Voice Cloning. Once the operator activates it,
//! FirstRing's Relay replies for that household are synthesized with the clone
//! and played into the call. Nothing here grants authority: a familiar voice is
//! never proof of identity.
use crate::calls::actor::App;
use anyhow::{Context, Result, anyhow, ensure};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const API: &str = "https://api.elevenlabs.io/v1";
/// Spoken from the console screen; about 45 seconds at a natural pace.
pub const PASSAGE: &str = "Hi, this is my voice for FirstRing. When someone I don't know calls, FirstRing answers first, listens to what they want, and takes a clear message. If a caller asks for a code, a password, my Social Security number or money in an unusual way, FirstRing politely says no and lets me know. My family and friends can still reach me, and I stay in charge of who I talk to. Today is a good day to make phone scams a little harder to repeat.";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Enrollment {
    pub household_id: String,
    /// armed | recording | processing | ready | active | failed | revoked
    pub state: String,
    pub speaker_name: String,
    pub consented_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub voice_id: Option<String>,
    pub sample_seconds: Option<u32>,
    pub error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

fn key(household: &str) -> String {
    format!("voice_clone.v1:{household}")
}
fn api_key() -> Result<String> {
    std::env::var("ELEVENLABS_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("ELEVENLABS_API_KEY is not configured"))
}
fn household_ok(id: &str) -> Result<()> {
    ensure!(["sarah", "maya"].contains(&id), "Unknown household");
    Ok(())
}

/// Households whose calls currently speak in an activated clone (voice id).
fn active_map() -> &'static Mutex<HashMap<String, String>> {
    static ACTIVE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    ACTIVE.get_or_init(Default::default)
}
pub fn active_voice(household: &str) -> Option<String> {
    active_map().lock().ok()?.get(household).cloned()
}

pub async fn load(app: &App, household: &str) -> Result<Option<Enrollment>> {
    Ok(match app.store.metadata(&key(household)).await? {
        Some(raw) => Some(serde_json::from_str(&raw)?),
        None => None,
    })
}
async fn save(app: &App, value: &Enrollment) -> Result<()> {
    app.store
        .set_metadata(&key(&value.household_id), &serde_json::to_string(value)?)
        .await?;
    let mut map = active_map()
        .lock()
        .map_err(|_| anyhow!("voice map poisoned"))?;
    match (&value.state[..], &value.voice_id) {
        ("active", Some(id)) => {
            map.insert(value.household_id.clone(), id.clone());
        }
        _ => {
            map.remove(&value.household_id);
        }
    }
    Ok(())
}
/// Reload activation state after a restart.
pub async fn refresh(app: &App) -> Result<()> {
    for household in ["sarah", "maya"] {
        if let Some(value) = load(app, household).await? {
            let mut map = active_map()
                .lock()
                .map_err(|_| anyhow!("voice map poisoned"))?;
            if value.state == "active"
                && let Some(id) = &value.voice_id
            {
                map.insert(household.into(), id.clone());
            } else {
                map.remove(household);
            }
        }
    }
    Ok(())
}

/// A setup call that ended before the tone never produces a recording callback.
/// After a few minutes such an enrollment counts as failed so the owner can retry.
fn stalled(v: &Enrollment, now: DateTime<Utc>) -> bool {
    (v.state == "recording" && now - v.updated_at > Duration::minutes(3))
        || (v.state == "processing" && now - v.updated_at > Duration::minutes(5))
}
const STALLED_ERROR: &str = "The setup call ended before a usable recording was made. Try again.";

pub fn view(value: Option<&Enrollment>, household: &str) -> Value {
    let now = Utc::now();
    let state = value.map_or("none".to_string(), |v| {
        if v.state == "armed" && v.expires_at.is_some_and(|e| e <= now) {
            "expired".into()
        } else if stalled(v, now) {
            "failed".into()
        } else {
            v.state.clone()
        }
    });
    let error = value.and_then(|v| {
        if stalled(v, now) {
            Some(STALLED_ERROR.to_string())
        } else {
            v.error.clone()
        }
    });
    json!({
        "household_id": household,
        "state": state,
        "speaker_name": value.map(|v| v.speaker_name.clone()),
        "expires_at": value.and_then(|v| v.expires_at),
        "voice_ready": value.is_some_and(|v| v.voice_id.is_some() && ["ready","active"].contains(&v.state.as_str())),
        "active": value.is_some_and(|v| v.state == "active"),
        "sample_seconds": value.and_then(|v| v.sample_seconds),
        "error": error,
        "passage": PASSAGE,
        "provider": "ElevenLabs Instant Voice Cloning",
        "provider_configured": api_key().is_ok(),
        "identity_verified": false,
    })
}

pub async fn arm(app: &App, household: &str, speaker: &str) -> Result<Enrollment> {
    household_ok(household)?;
    api_key()?;
    let speaker = speaker.trim();
    ensure!(
        (2..=60).contains(&speaker.chars().count())
            && speaker
                .chars()
                .all(|c| c.is_alphabetic() || " '-’.".contains(c)),
        "Enter the speaker's name"
    );
    if let Some(previous) = load(app, household).await? {
        ensure!(
            !["recording", "processing"].contains(&previous.state.as_str())
                || stalled(&previous, Utc::now()),
            "A recording is already being processed"
        );
    }
    let now = Utc::now();
    let value = Enrollment {
        household_id: household.into(),
        state: "armed".into(),
        speaker_name: speaker.into(),
        consented_at: now,
        expires_at: Some(now + Duration::minutes(10)),
        voice_id: load(app, household).await?.and_then(|v| v.voice_id),
        sample_seconds: None,
        error: None,
        updated_at: now,
    };
    save(app, &value).await?;
    Ok(value)
}

/// Signed inbound only: an armed enrollment takes this call instead of screening.
pub async fn intercept(app: &App, call_sid: &str) -> Result<Option<String>> {
    for household in ["sarah", "maya"] {
        let Some(mut value) = load(app, household).await? else {
            continue;
        };
        if value.state != "armed" || value.expires_at.is_none_or(|e| e <= Utc::now()) {
            continue;
        }
        value.state = "recording".into();
        value.updated_at = Utc::now();
        save(app, &value).await?;
        let done = app
            .config
            .external_url(&format!("/voice/clone-recorded?household={household}"));
        let status = app
            .config
            .external_url(&format!("/voice/clone-recording?household={household}"));
        let _ = call_sid;
        return Ok(Some(format!(
            "<Response><Say voice=\"Polly.Joanna-Neural\">This is FirstRing voice setup. After the tone, please read the passage on your screen in your normal speaking voice. Press the pound key when you finish.</Say><Record maxLength=\"90\" timeout=\"6\" finishOnKey=\"#\" playBeep=\"true\" trim=\"trim-silence\" action=\"{}\" method=\"POST\" recordingStatusCallback=\"{}\" recordingStatusCallbackMethod=\"POST\" recordingStatusCallbackEvent=\"completed\"/><Say voice=\"Polly.Joanna-Neural\">Thank you. Goodbye.</Say></Response>",
            escape(&done),
            escape(&status)
        )));
    }
    Ok(None)
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
pub fn recorded_twiml() -> String {
    "<Response><Say voice=\"Polly.Joanna-Neural\">Thank you. FirstRing is preparing your voice now. It will be ready on the dashboard in about a minute. Goodbye.</Say><Hangup/></Response>".into()
}

/// Twilio recording callback: fetch the caller-only recording and create the clone.
pub fn process_recording(app: App, household: String, recording_sid: String, seconds: u32) {
    tokio::spawn(async move {
        let result = async {
            household_ok(&household)?;
            let mut value = load(&app, &household)
                .await?
                .ok_or_else(|| anyhow!("No enrollment"))?;
            ensure!(value.state == "recording", "Enrollment is not recording");
            value.state = "processing".into();
            value.sample_seconds = Some(seconds);
            value.updated_at = Utc::now();
            save(&app, &value).await?;
            ensure!(
                seconds >= 15,
                "The recording was {seconds} seconds; read the whole passage (at least 15 seconds)"
            );
            let audio = app.twilio.fixture_recording(&recording_sid).await?;
            let voice_id = create_clone(&value.speaker_name, audio).await?;
            if let Some(old) = value.voice_id.replace(voice_id)
                && Some(&old) != value.voice_id.as_ref()
            {
                let _ = delete_clone(&old).await;
            }
            value.state = "ready".into();
            value.error = None;
            value.updated_at = Utc::now();
            save(&app, &value).await
        }
        .await;
        if let Err(error) = result
            && let Ok(Some(mut value)) = load(&app, &household).await
        {
            value.state = "failed".into();
            value.error = Some(error.to_string());
            value.updated_at = Utc::now();
            let _ = save(&app, &value).await;
            tracing::warn!(%error, "voice enrollment failed");
        }
    });
}

pub async fn set_active(app: &App, household: &str, active: bool) -> Result<Enrollment> {
    household_ok(household)?;
    let mut value = load(app, household)
        .await?
        .ok_or_else(|| anyhow!("No enrolled voice"))?;
    ensure!(
        value.voice_id.is_some() && ["ready", "active"].contains(&value.state.as_str()),
        "The voice is not ready"
    );
    value.state = if active { "active" } else { "ready" }.into();
    value.updated_at = Utc::now();
    save(app, &value).await?;
    Ok(value)
}

pub async fn delete(app: &App, household: &str) -> Result<Enrollment> {
    household_ok(household)?;
    let mut value = load(app, household)
        .await?
        .ok_or_else(|| anyhow!("No enrolled voice"))?;
    if let Some(id) = value.voice_id.take() {
        delete_clone(&id).await?;
    }
    value.state = "revoked".into();
    value.expires_at = None;
    value.updated_at = Utc::now();
    save(app, &value).await?;
    Ok(value)
}

async fn create_clone(speaker: &str, audio: Vec<u8>) -> Result<String> {
    let key = api_key()?;
    let boundary = format!("firstring-{}", uuid::Uuid::new_v4().simple());
    let mut body: Vec<u8> = Vec::with_capacity(audio.len() + 1024);
    let mut field = |name: &str, value: &str| {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    };
    field("name", &format!("FirstRing · {speaker}"));
    field(
        "description",
        "Consented owner voice recorded on a FirstRing setup call.",
    );
    field("remove_background_noise", "true");
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"sample.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(&audio);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = reqwest::Client::new()
        .post(format!("{API}/voices/add"))
        .header("xi-api-key", key)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .timeout(std::time::Duration::from_secs(60))
        .body(body)
        .send()
        .await
        .context("Voice provider unreachable")?;
    let status = response.status();
    let value: Value = response.json().await.unwrap_or(Value::Null);
    ensure!(
        status.is_success(),
        "Voice provider rejected the sample ({}): {}",
        status.as_u16(),
        value["detail"]["message"]
            .as_str()
            .or(value["detail"].as_str())
            .unwrap_or("no detail")
    );
    value["voice_id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Voice provider returned no voice id"))
}

async fn delete_clone(voice_id: &str) -> Result<()> {
    ensure!(
        voice_id.len() <= 64 && voice_id.chars().all(|c| c.is_ascii_alphanumeric()),
        "Invalid voice id"
    );
    let response = reqwest::Client::new()
        .delete(format!("{API}/voices/{voice_id}"))
        .header("xi-api-key", api_key()?)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    ensure!(
        response.status().is_success() || response.status().as_u16() == 404,
        "Voice deletion failed ({})",
        response.status().as_u16()
    );
    Ok(())
}

/// Low-latency synthesis in the cloned voice.
pub async fn synthesize(voice_id: &str, text: &str) -> Result<Vec<u8>> {
    ensure!(
        voice_id.len() <= 64 && voice_id.chars().all(|c| c.is_ascii_alphanumeric()),
        "Invalid voice id"
    );
    ensure!(
        !text.trim().is_empty() && text.chars().count() <= 600,
        "Bounded speech only"
    );
    let response = reqwest::Client::new()
        .post(format!(
            "{API}/text-to-speech/{voice_id}?output_format=mp3_22050_32"
        ))
        .header("xi-api-key", api_key()?)
        .timeout(std::time::Duration::from_secs(8))
        .json(&json!({"text": text, "model_id": "eleven_flash_v2_5",
            "voice_settings": {"stability": 0.5, "similarity_boost": 0.85}}))
        .send()
        .await
        .context("Voice synthesis unreachable")?;
    ensure!(
        response.status().is_success(),
        "Voice synthesis failed ({})",
        response.status().as_u16()
    );
    Ok(response.bytes().await?.to_vec())
}

/// Short-lived capability URLs for synthesized clips (served at /media/voice/{id}).
type MediaCache = Mutex<HashMap<String, (Vec<u8>, std::time::Instant)>>;
fn media() -> &'static MediaCache {
    static MEDIA: OnceLock<MediaCache> = OnceLock::new();
    MEDIA.get_or_init(Default::default)
}
pub fn put_media(bytes: Vec<u8>) -> String {
    let id = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    if let Ok(mut map) = media().lock() {
        map.retain(|_, (_, at)| at.elapsed() < std::time::Duration::from_secs(900));
        map.insert(id.clone(), (bytes, std::time::Instant::now()));
    }
    id
}
pub fn get_media(id: &str) -> Option<Vec<u8>> {
    let map = media().lock().ok()?;
    map.get(id)
        .filter(|(_, at)| at.elapsed() < std::time::Duration::from_secs(900))
        .map(|(bytes, _)| bytes.clone())
}
