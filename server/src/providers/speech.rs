//! Bounded, fixed-phrase ElevenLabs speech for signed call-control TwiML.
//! Media capabilities carry no operator credential and expire after five minutes.
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use url::Url;

const API: &str = "https://api.elevenlabs.io/v1/text-to-speech";
const CACHE_TTL: u64 = 86_400;
const CAPABILITY_TTL: u64 = 300;
const MAX_AUDIO: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum AudioFormat {
    #[default]
    Mp3,
    Wav,
}
impl AudioFormat {
    fn parse(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("mp3") {
            "mp3" => Ok(Self::Mp3),
            "wav" => Ok(Self::Wav),
            _ => Err(anyhow!("CLOSING_AUDIO_FORMAT must be mp3 or wav")),
        }
    }
    fn mime_type(self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
        }
    }
}

pub struct SpeechMedia {
    pub bytes: Vec<u8>,
    pub mime_type: &'static str,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StaticPhrase {
    NormalSaved,
    ProtectiveSaved,
    ProtectedModelFailure,
    NoMessage,
}
impl StaticPhrase {
    pub const ALL: [Self; 4] = [
        Self::NormalSaved,
        Self::ProtectiveSaved,
        Self::ProtectedModelFailure,
        Self::NoMessage,
    ];
    pub fn text(self) -> &'static str {
        match self {
            Self::NormalSaved => "Thanks, I’ve saved your message. Goodbye.",
            Self::ProtectiveSaved => "I can’t help with that request. I’ve saved a note. Goodbye.",
            Self::ProtectedModelFailure => "I can’t help with that request. Goodbye.",
            Self::NoMessage => "Thanks for calling. Goodbye.",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct VoiceProfile {
    pub provider: &'static str,
    pub model_id: &'static str,
    pub voice_id: &'static str,
}

/// Twilio's -v4 suffix maps to ElevenLabs' eleven_v4, not a voice clone.
/// https://www.twilio.com/docs/voice/conversationrelay/voice-configuration
/// Exact IDs/model access were read back from ElevenLabs on 2026-11-08.
pub fn voice_profile(relay_voice: &str) -> Result<VoiceProfile> {
    let voice_id = match relay_voice {
        "cgSgspJ2msm6clMCkdW9-v4" => "cgSgspJ2msm6clMCkdW9",
        "JBFqnCBsd6RMkjVDRZzb-v4" => "JBFqnCBsd6RMkjVDRZzb",
        // Legacy ID is retained for compatibility; current API labels it Janet.
        "21m00Tcm4TlvDq8ikWAM-v4" => "21m00Tcm4TlvDq8ikWAM",
        _ => return Err(anyhow!("Unverified static-speech voice configuration")),
    };
    Ok(VoiceProfile {
        provider: "ElevenLabs",
        model_id: "eleven_v4",
        voice_id,
    })
}

/// Do not log `url`: it is an expiring bearer capability. Use `evidence()`.
#[derive(Clone)]
pub struct PreparedAudio {
    pub url: String,
    pub expires_at: u64,
    pub phrase: StaticPhrase,
    pub text: &'static str,
    pub profile: VoiceProfile,
    pub sha256: String,
    pub cache_hit: bool,
    audio_format: AudioFormat,
    requested_audio_format: AudioFormat,
    duration_ms: Option<u64>,
    source_sha256: String,
    format_fallback: Option<&'static str>,
}
impl PreparedAudio {
    pub fn evidence(&self) -> Value {
        json!({"provider":self.profile.provider,"model_id":self.profile.model_id,
            "voice_id":self.profile.voice_id,"phrase":self.phrase,"sha256":self.sha256,
            "cache_hit":self.cache_hit,"expires_at":self.expires_at,
            "audio_format":self.audio_format,"requested_audio_format":self.requested_audio_format,
            "mime_type":self.audio_format.mime_type(),"duration_ms":self.duration_ms,
            "source_sha256":self.source_sha256,"format_fallback":self.format_fallback,
            "generated_audio_available":true,"audible_playback_verified":false})
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CacheEntry {
    phrase: StaticPhrase,
    relay_voice: String,
    created_at: u64,
    sha256: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    cache_key: String,
    expires_at: u64,
    #[serde(default)]
    format: AudioFormat,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WavEntry {
    source_sha256: String,
    sha256: String,
    duration_ms: u64,
}

pub struct SpeechCache {
    root: PathBuf,
    origin: Url,
    key: Option<String>,
    client: reqwest::Client,
    lock: Mutex<()>,
    endpoint: String,
    format: AudioFormat,
}
impl SpeechCache {
    #[cfg(test)]
    pub(crate) async fn seed_fixture(&self, phrase: StaticPhrase, voice: &str) -> Result<()> {
        let bytes = [b"ID3".as_slice(), &[0u8; 40]].concat();
        self.store_audio(phrase, voice, &bytes, now()).await?;
        Ok(())
    }
    /// Construction is inert: no provider request or filesystem mutation.
    pub fn new(root: PathBuf, public_origin: &str, key: Option<String>) -> Result<Self> {
        let mut origin =
            Url::parse(public_origin).map_err(|_| anyhow!("Invalid speech media origin"))?;
        ensure!(
            origin.scheme() == "https"
                && origin.host_str().is_some()
                && origin.username().is_empty()
                && origin.password().is_none()
                && origin.query().is_none()
                && origin.fragment().is_none(),
            "Speech media requires the configured HTTPS origin"
        );
        origin.set_path("/");
        Ok(Self {
            root,
            origin,
            key: key.filter(|s| !s.trim().is_empty()),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            lock: Mutex::new(()),
            endpoint: API.into(),
            format: AudioFormat::parse(std::env::var("CLOSING_AUDIO_FORMAT").ok().as_deref())?,
        })
    }

    /// Caller must authorize live speech synthesis; replay code must not call this.
    /// Whole operation is bounded, including contention. No automatic retries.
    pub async fn prepare(&self, phrase: StaticPhrase, relay_voice: &str) -> Result<PreparedAudio> {
        tokio::time::timeout(
            Duration::from_secs(2),
            self.prepare_at(phrase, relay_voice, now()),
        )
        .await
        .map_err(|_| anyhow!("Static speech preparation timed out"))?
    }
    async fn prepare_at(
        &self,
        phrase: StaticPhrase,
        relay_voice: &str,
        at: u64,
    ) -> Result<PreparedAudio> {
        let profile = voice_profile(relay_voice)?;
        let cache_key = cache_key(phrase, relay_voice)?;
        let _lock = self.lock.lock().await;
        let existing = self.read_audio(&cache_key, at).await?;
        let cache_hit = existing.is_some();
        let entry = if let Some((entry, _)) = existing {
            entry
        } else {
            ensure!(
                self.format != AudioFormat::Wav,
                "WAV experiment requires an existing MP3 source"
            );
            let key = self
                .key
                .as_ref()
                .ok_or_else(|| anyhow!("ELEVENLABS_API_KEY unavailable for static speech"))?;
            let mut response = self
                .client
                .post(format!("{}/{}", self.endpoint, profile.voice_id))
                .query(&[("output_format", "mp3_44100_128")])
                .header("xi-api-key", key)
                .json(&json!({"text":phrase.text(),"model_id":profile.model_id}))
                .send()
                .await
                .map_err(|_| anyhow!("Static speech provider transport failed"))?;
            ensure!(
                response.status().is_success(),
                "Static speech provider HTTP {}",
                response.status().as_u16()
            );
            ensure!(
                response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.starts_with("audio/mpeg")),
                "Static speech provider returned an unexpected format"
            );
            ensure!(
                response
                    .content_length()
                    .is_none_or(|length| length <= MAX_AUDIO as u64),
                "Static speech exceeds size limit"
            );
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| anyhow!("Static speech audio transfer failed"))?
            {
                ensure!(
                    bytes.len() + chunk.len() <= MAX_AUDIO,
                    "Static speech exceeds size limit"
                );
                bytes.extend_from_slice(&chunk);
            }
            self.store_audio(phrase, relay_voice, &bytes, at).await?
        };
        // Offline derivatives only: never convert or regenerate speech in a call.
        // A missing/invalid variant keeps the already-validated same-voice MP3.
        let wav = if self.format == AudioFormat::Wav {
            self.read_wav(&cache_key, &entry.sha256)
                .await
                .unwrap_or(None)
        } else {
            None
        };
        let (format, sha256, duration_ms, format_fallback) = if let Some((wav, _)) = wav {
            (AudioFormat::Wav, wav.sha256, Some(wav.duration_ms), None)
        } else {
            (
                AudioFormat::Mp3,
                entry.sha256.clone(),
                None,
                (self.format == AudioFormat::Wav).then_some("wav_unavailable_using_cached_mp3"),
            )
        };
        let capability = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let expires_at = at
            .saturating_add(CAPABILITY_TTL)
            .min(entry.created_at.saturating_add(CACHE_TTL));
        let cap = Capability {
            cache_key,
            expires_at,
            format,
        };
        write_private(
            &self
                .root
                .join("capabilities")
                .join(format!("{capability}.json")),
            &serde_json::to_vec(&cap)?,
        )
        .await?;
        let extension = if format == AudioFormat::Wav {
            ".wav"
        } else {
            ""
        };
        let url = self
            .origin
            .join(&format!("/media/speech/{capability}{extension}"))?
            .to_string();
        Ok(PreparedAudio {
            url,
            expires_at,
            phrase,
            text: phrase.text(),
            profile,
            sha256,
            cache_hit,
            audio_format: format,
            requested_audio_format: self.format,
            duration_ms,
            source_sha256: entry.sha256,
            format_fallback,
        })
    }
    async fn store_audio(
        &self,
        phrase: StaticPhrase,
        voice: &str,
        audio: &[u8],
        at: u64,
    ) -> Result<CacheEntry> {
        ensure!(
            valid_mp3(audio),
            "Static speech response was not bounded MP3 audio"
        );
        let key = cache_key(phrase, voice)?;
        let entry = CacheEntry {
            phrase,
            relay_voice: voice.into(),
            created_at: at,
            sha256: hash(audio),
        };
        write_private(&self.root.join("audio").join(format!("{key}.mp3")), audio).await?;
        write_private(
            &self.root.join("audio").join(format!("{key}.json")),
            &serde_json::to_vec(&entry)?,
        )
        .await?;
        Ok(entry)
    }
    async fn read_audio(&self, key: &str, at: u64) -> Result<Option<(CacheEntry, Vec<u8>)>> {
        if !hex_key(key) {
            return Ok(None);
        }
        let Some(metadata) =
            read_bounded(&self.root.join("audio").join(format!("{key}.json")), 4096).await?
        else {
            return Ok(None);
        };
        let Ok(entry) = serde_json::from_slice::<CacheEntry>(&metadata) else {
            return Ok(None);
        };
        if entry.created_at > at
            || at >= entry.created_at.saturating_add(CACHE_TTL)
            || cache_key(entry.phrase, &entry.relay_voice).ok().as_deref() != Some(key)
        {
            return Ok(None);
        }
        let Some(audio) = read_bounded(
            &self.root.join("audio").join(format!("{key}.mp3")),
            MAX_AUDIO,
        )
        .await?
        else {
            return Ok(None);
        };
        if !valid_mp3(&audio) || hash(&audio) != entry.sha256 {
            return Ok(None);
        }
        Ok(Some((entry, audio)))
    }
    pub async fn media(&self, capability: &str) -> Result<Option<Vec<u8>>> {
        self.media_at(capability, now()).await
    }
    async fn media_at(&self, capability: &str, at: u64) -> Result<Option<Vec<u8>>> {
        Ok(self
            .media_response_at(capability, at)
            .await?
            .map(|media| media.bytes))
    }
    pub async fn media_response(&self, capability: &str) -> Result<Option<SpeechMedia>> {
        self.media_response_at(capability, now()).await
    }
    async fn read_wav(
        &self,
        key: &str,
        source_sha256: &str,
    ) -> Result<Option<(WavEntry, Vec<u8>)>> {
        let Some(metadata) = read_bounded(
            &self.root.join("audio").join(format!("{key}.wav.json")),
            4096,
        )
        .await?
        else {
            return Ok(None);
        };
        let Ok(entry) = serde_json::from_slice::<WavEntry>(&metadata) else {
            return Ok(None);
        };
        let Some(bytes) = read_bounded(
            &self.root.join("audio").join(format!("{key}.wav")),
            MAX_AUDIO,
        )
        .await?
        else {
            return Ok(None);
        };
        if entry.source_sha256 != source_sha256
            || hash(&bytes) != entry.sha256
            || wav_duration_ms(&bytes) != Some(entry.duration_ms)
        {
            return Ok(None);
        }
        Ok(Some((entry, bytes)))
    }
    async fn media_response_at(&self, capability: &str, at: u64) -> Result<Option<SpeechMedia>> {
        let (capability, format) = if let Some(key) = capability.strip_suffix(".wav") {
            (key, AudioFormat::Wav)
        } else {
            (capability, AudioFormat::Mp3)
        };
        if !hex_key(capability) {
            return Ok(None);
        }
        let Some(bytes) = read_bounded(
            &self
                .root
                .join("capabilities")
                .join(format!("{capability}.json")),
            4096,
        )
        .await?
        else {
            return Ok(None);
        };
        let Ok(cap) = serde_json::from_slice::<Capability>(&bytes) else {
            return Ok(None);
        };
        if at >= cap.expires_at
            || cap.expires_at > at.saturating_add(CAPABILITY_TTL)
            || cap.format != format
        {
            return Ok(None);
        }
        let Some((source, audio)) = self.read_audio(&cap.cache_key, at).await? else {
            return Ok(None);
        };
        let bytes = if format == AudioFormat::Wav {
            let Some((_, wav)) = self.read_wav(&cap.cache_key, &source.sha256).await? else {
                return Ok(None);
            };
            wav
        } else {
            audio
        };
        Ok(Some(SpeechMedia {
            bytes,
            mime_type: format.mime_type(),
        }))
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex_key(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn valid_mp3(bytes: &[u8]) -> bool {
    (32..=MAX_AUDIO).contains(&bytes.len())
        && (bytes.starts_with(b"ID3") || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0))
}
/// Strict RIFF PCM16, mono 8kHz. Metadata chunks are allowed, malformed/duplicate
/// chunks and compressed WAV variants are not. No duration is guessed from MP3.
fn wav_duration_ms(bytes: &[u8]) -> Option<u64> {
    if !(44..=MAX_AUDIO).contains(&bytes.len())
        || &bytes[..4] != b"RIFF"
        || &bytes[8..12] != b"WAVE"
    {
        return None;
    }
    let u32_at = |at: usize| -> Option<u32> {
        Some(u32::from_le_bytes(
            bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
        ))
    };
    if usize::try_from(u32_at(4)?).ok()?.checked_add(8)? != bytes.len() {
        return None;
    }
    let mut at = 12usize;
    let mut format = false;
    let mut data = None;
    while at < bytes.len() {
        let tag = bytes.get(at..at.checked_add(4)?)?;
        let size = usize::try_from(u32_at(at + 4)?).ok()?;
        let start = at.checked_add(8)?;
        let end = start.checked_add(size)?;
        let chunk = bytes.get(start..end)?;
        if tag == b"fmt " {
            if format
                || size != 16
                || chunk[..2] != 1u16.to_le_bytes()
                || chunk[2..4] != 1u16.to_le_bytes()
                || chunk[4..8] != 8000u32.to_le_bytes()
                || chunk[8..12] != 16000u32.to_le_bytes()
                || chunk[12..14] != 2u16.to_le_bytes()
                || chunk[14..16] != 16u16.to_le_bytes()
            {
                return None;
            }
            format = true;
        } else if tag == b"data" {
            if data.is_some() || size == 0 || size % 2 != 0 {
                return None;
            }
            data = Some(size as u64);
        }
        at = end.checked_add(size % 2)?;
    }
    if !format || at != bytes.len() {
        return None;
    }
    Some(data?.checked_mul(1000)? / 16000)
}
fn cache_key(phrase: StaticPhrase, relay_voice: &str) -> Result<String> {
    let profile = voice_profile(relay_voice)?;
    Ok(hash(
        json!({"version":1,"voice":profile.voice_id,"model":profile.model_id,
        "text":phrase.text(),"format":"mp3_44100_128","voice_settings":"provider_defaults"})
        .to_string()
        .as_bytes(),
    ))
}
async fn read_bounded(path: &Path, limit: usize) -> Result<Option<Vec<u8>>> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(anyhow!("Static speech cache read failed")),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit as u64 {
        return Ok(None);
    }
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|_| anyhow!("Static speech cache read failed"))?;
    Ok((bytes.len() <= limit).then_some(bytes))
}
async fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Invalid static speech cache path"))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|_| anyhow!("Static speech cache directory failed"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(|_| anyhow!("Static speech cache permissions failed"))?;
    }
    let temporary = path.with_extension(format!("{}.partial", uuid::Uuid::new_v4()));
    use tokio::io::AsyncWriteExt;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .await
        .map_err(|_| anyhow!("Static speech cache write failed"))?;
    file.write_all(bytes)
        .await
        .map_err(|_| anyhow!("Static speech cache write failed"))?;
    file.sync_all()
        .await
        .map_err(|_| anyhow!("Static speech cache write failed"))?;
    tokio::fs::rename(&temporary, path)
        .await
        .map_err(|_| anyhow!("Static speech cache publication failed"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm_wav() -> Vec<u8> {
        let data = vec![0u8; 16000];
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&(36u32 + data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8000u32.to_le_bytes());
        bytes.extend_from_slice(&16000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend(data);
        bytes
    }

    #[test]
    fn wav_switch_defaults_off_and_pcm_validation_rejects_wrong_or_truncated_audio() {
        assert_eq!(AudioFormat::parse(None).unwrap(), AudioFormat::Mp3);
        assert_eq!(AudioFormat::parse(Some("wav")).unwrap(), AudioFormat::Wav);
        assert!(AudioFormat::parse(Some("arbitrary")).is_err());
        let bytes = pcm_wav();
        assert_eq!(wav_duration_ms(&bytes), Some(1000));
        for (offset, replacement) in [(0, b'X'), (20, 3), (22, 2), (24, 0), (34, 8)] {
            let mut invalid = bytes.clone();
            invalid[offset] = replacement;
            assert!(wav_duration_ms(&invalid).is_none(), "offset {offset}");
        }
        assert!(wav_duration_ms(&bytes[..bytes.len() - 1]).is_none());
        assert!(wav_duration_ms(&vec![0; MAX_AUDIO + 1]).is_none());
    }

    #[tokio::test]
    async fn wav_variant_keeps_original_provenance_capability_expiry_and_correct_mime() {
        let root = directory();
        let mut cache = SpeechCache::new(root.clone(), "https://first.example", None).unwrap();
        let phrase = StaticPhrase::NormalSaved;
        let voice = "cgSgspJ2msm6clMCkdW9-v4";
        let source = [b"ID3".as_slice(), &[0u8; 40]].concat();
        cache
            .store_audio(phrase, voice, &source, 1000)
            .await
            .unwrap();
        let old_mp3 = cache.prepare_at(phrase, voice, 1001).await.unwrap();
        let key = cache_key(phrase, voice).unwrap();
        let wav = pcm_wav();
        write_private(&root.join("audio").join(format!("{key}.wav")), &wav)
            .await
            .unwrap();
        write_private(
            &root.join("audio").join(format!("{key}.wav.json")),
            &serde_json::to_vec(
                &json!({"source_sha256":hash(&source),"sha256":hash(&wav),"duration_ms":1000}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
        cache.format = AudioFormat::Wav;
        let prepared = cache.prepare_at(phrase, voice, 1001).await.unwrap();
        assert!(prepared.url.ends_with(".wav"));
        assert_eq!(prepared.evidence()["mime_type"], "audio/wav");
        assert_eq!(prepared.evidence()["duration_ms"], 1000);
        assert_eq!(prepared.evidence()["source_sha256"], hash(&source));
        assert_eq!(prepared.evidence()["sha256"], hash(&wav));
        assert_eq!(prepared.evidence()["audible_playback_verified"], false);
        let capability = prepared.url.rsplit('/').next().unwrap();
        let media = cache
            .media_response_at(capability, 1300)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(media.mime_type, "audio/wav");
        assert_eq!(media.bytes, wav);
        assert!(
            cache
                .media_response_at(capability, 1301)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            cache
                .media_response_at(capability.trim_end_matches(".wav"), 1002)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            cache
                .media_at(old_mp3.url.rsplit('/').next().unwrap(), 1002)
                .await
                .unwrap(),
            Some(source)
        );
        tokio::fs::write(
            root.join("audio").join(format!("{key}.wav")),
            b"invalid RIFF",
        )
        .await
        .unwrap();
        assert!(
            cache
                .media_response_at(capability, 1002)
                .await
                .unwrap()
                .is_none()
        );
        let fallback = cache.prepare_at(phrase, voice, 1002).await.unwrap();
        assert_eq!(fallback.evidence()["audio_format"], "mp3");
        assert_eq!(
            fallback.evidence()["format_fallback"],
            "wav_unavailable_using_cached_mp3"
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn wav_mode_without_source_never_attempts_provider_generation() {
        let root = directory();
        let mut cache = SpeechCache::new(
            root.clone(),
            "https://first.example",
            Some("never-submit".into()),
        )
        .unwrap();
        cache.format = AudioFormat::Wav;
        cache.endpoint = "http://127.0.0.1:1/must-not-contact".into();
        let error = cache
            .prepare_at(StaticPhrase::NoMessage, "cgSgspJ2msm6clMCkdW9-v4", 1000)
            .await
            .err()
            .unwrap()
            .to_string();
        assert_eq!(error, "WAV experiment requires an existing MP3 source");
        assert!(!root.exists());
    }

    fn directory() -> PathBuf {
        std::env::temp_dir().join(format!("firstring-speech-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn relay_v4_maps_to_same_direct_model_and_only_verified_voices() {
        let profile = voice_profile("cgSgspJ2msm6clMCkdW9-v4").unwrap();
        assert_eq!(profile.model_id, "eleven_v4");
        assert_eq!(profile.voice_id, "cgSgspJ2msm6clMCkdW9");
        assert!(voice_profile("https://attacker.test/audio").is_err());
        assert!(voice_profile("unlisted-v4").is_err());
    }

    #[tokio::test]
    async fn persistent_cache_reuses_audio_without_key_and_capability_expires() {
        let root = directory();
        let cache = SpeechCache::new(root.clone(), "https://first.example", None).unwrap();
        let phrase = StaticPhrase::NormalSaved;
        let voice = "cgSgspJ2msm6clMCkdW9-v4";
        let audio = [b"ID3".as_slice(), &[0u8; 40]].concat();
        cache
            .store_audio(phrase, voice, &audio, 1000)
            .await
            .unwrap();
        let prepared = cache.prepare_at(phrase, voice, 1001).await.unwrap();
        assert!(
            prepared
                .url
                .starts_with("https://first.example/media/speech/")
        );
        assert_eq!(prepared.expires_at, 1301);
        assert_eq!(prepared.text, phrase.text());
        let capability = prepared.url.rsplit('/').next().unwrap();
        let restarted = SpeechCache::new(root.clone(), "https://first.example", None).unwrap();
        assert_eq!(
            restarted.media_at(capability, 1300).await.unwrap(),
            Some(audio)
        );
        assert_eq!(restarted.media_at(capability, 1301).await.unwrap(), None);
        assert_eq!(restarted.media_at("../private", 1001).await.unwrap(), None);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn missing_key_never_attempts_synthesis_and_expired_cache_cannot_extend_itself() {
        let root = directory();
        let cache = SpeechCache::new(root.clone(), "https://first.example", None).unwrap();
        assert!(
            cache
                .prepare_at(StaticPhrase::NoMessage, "cgSgspJ2msm6clMCkdW9-v4", 1000)
                .await
                .is_err()
        );
        assert!(!root.exists());
        let audio = [b"ID3".as_slice(), &[0u8; 40]].concat();
        cache
            .store_audio(
                StaticPhrase::NoMessage,
                "cgSgspJ2msm6clMCkdW9-v4",
                &audio,
                1000,
            )
            .await
            .unwrap();
        assert!(
            cache
                .prepare_at(
                    StaticPhrase::NoMessage,
                    "cgSgspJ2msm6clMCkdW9-v4",
                    1000 + CACHE_TTL
                )
                .await
                .is_err()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[test]
    fn generated_media_origin_must_be_configured_https_without_credentials() {
        for origin in [
            "http://first.example",
            "https://user:secret@first.example",
            "https://first.example/?secret=1",
        ] {
            assert!(SpeechCache::new(directory(), origin, None).is_err());
        }
    }

    #[tokio::test]
    async fn corrupt_cached_audio_is_not_served() {
        let root = directory();
        let cache = SpeechCache::new(root.clone(), "https://first.example", None).unwrap();
        let phrase = StaticPhrase::ProtectiveSaved;
        let voice = "cgSgspJ2msm6clMCkdW9-v4";
        cache
            .store_audio(
                phrase,
                voice,
                &[b"ID3".as_slice(), &[0u8; 40]].concat(),
                1000,
            )
            .await
            .unwrap();
        let prepared = cache.prepare_at(phrase, voice, 1001).await.unwrap();
        tokio::fs::write(
            root.join("audio")
                .join(format!("{}.mp3", cache_key(phrase, voice).unwrap())),
            b"not-audio",
        )
        .await
        .unwrap();
        assert_eq!(
            cache
                .media_at(prepared.url.rsplit('/').next().unwrap(), 1002)
                .await
                .unwrap(),
            None
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    async fn provider(
        status: u16,
        body: Vec<u8>,
        delay: Duration,
    ) -> (
        String,
        tokio::sync::oneshot::Receiver<String>,
        tokio::task::JoinHandle<()>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (sent, received) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0u8; 2048];
                let count = stream.read(&mut chunk).await.unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..count]);
                if let Some(index) = request.windows(4).position(|x| x == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..index]).to_lowercase();
                    let size = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    if request.len() >= index + 4 + size {
                        break;
                    }
                }
            }
            let _ = sent.send(String::from_utf8(request).unwrap());
            tokio::time::sleep(delay).await;
            let header = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: audio/mpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes()).await;
            let _ = stream.write_all(&body).await;
        });
        (format!("http://{address}"), received, task)
    }

    #[tokio::test]
    async fn direct_request_uses_same_voice_model_and_redacts_provider_error() {
        let (endpoint, request, task) = provider(
            503,
            b"private-provider-body-and-key".to_vec(),
            Duration::ZERO,
        )
        .await;
        let root = directory();
        let mut cache = SpeechCache::new(
            root,
            "https://first.example",
            Some("fixture-secret-key".into()),
        )
        .unwrap();
        cache.endpoint = endpoint;
        let error = cache
            .prepare(StaticPhrase::NoMessage, "cgSgspJ2msm6clMCkdW9-v4")
            .await
            .err()
            .unwrap()
            .to_string();
        assert_eq!(error, "Static speech provider HTTP 503");
        let request = request.await.unwrap();
        assert!(request.starts_with("POST /cgSgspJ2msm6clMCkdW9?output_format=mp3_44100_128 "));
        let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(
            body,
            json!({"text":StaticPhrase::NoMessage.text(),"model_id":"eleven_v4"})
        );
        task.await.unwrap();
    }

    #[tokio::test]
    async fn provider_stall_is_bounded_by_two_seconds() {
        let (endpoint, _, task) = provider(
            200,
            [b"ID3".as_slice(), &[0u8; 40]].concat(),
            Duration::from_secs(4),
        )
        .await;
        let mut cache = SpeechCache::new(
            directory(),
            "https://first.example",
            Some("fixture-key".into()),
        )
        .unwrap();
        cache.endpoint = endpoint;
        let started = std::time::Instant::now();
        assert!(
            cache
                .prepare(StaticPhrase::NoMessage, "cgSgspJ2msm6clMCkdW9-v4")
                .await
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_millis(2800));
        task.abort();
    }
}
