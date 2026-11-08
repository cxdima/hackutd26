use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelaySpeechModel {
    Nova3,
    Flux,
}
impl RelaySpeechModel {
    pub fn parse(value: Option<&str>) -> anyhow::Result<Self> {
        match value.unwrap_or("nova-3-general") {
            "nova-3-general" => Ok(Self::Nova3),
            "flux" => Ok(Self::Flux),
            _ => anyhow::bail!("RELAY_SPEECH_MODEL must be nova-3-general or flux"),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nova3 => "nova-3-general",
            Self::Flux => "flux",
        }
    }
}

// One startup snapshot for this single-process demo. Configuration is immutable
// during a call; a different ASR experiment requires an explicit process restart.
static RELAY_SPEECH_MODEL: OnceLock<RelaySpeechModel> = OnceLock::new();
static RELAY_SPEECH_TIMEOUT_MS: OnceLock<u16> = OnceLock::new();
fn parse_relay_speech_timeout(value: Option<&str>) -> anyhow::Result<u16> {
    match value.unwrap_or("1000") {
        "1000" => Ok(1000),
        "600" => Ok(600),
        _ => anyhow::bail!("RELAY_SPEECH_TIMEOUT_MS must be 600 or 1000"),
    }
}
fn install_relay_speech_timeout(slot: &OnceLock<u16>, selected: u16) -> anyhow::Result<()> {
    anyhow::ensure!(
        *slot.get_or_init(|| selected) == selected,
        "RELAY_SPEECH_TIMEOUT_MS changed after startup; restart the process"
    );
    Ok(())
}
pub fn relay_speech_timeout_ms() -> u16 {
    RELAY_SPEECH_TIMEOUT_MS.get().copied().unwrap_or(1000)
}
fn install_relay_speech_model(
    slot: &OnceLock<RelaySpeechModel>,
    selected: RelaySpeechModel,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        *slot.get_or_init(|| selected) == selected,
        "RELAY_SPEECH_MODEL changed after startup; restart the process"
    );
    Ok(())
}
pub fn relay_speech_model() -> RelaySpeechModel {
    RELAY_SPEECH_MODEL
        .get()
        .copied()
        .unwrap_or(RelaySpeechModel::Nova3)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Live,
    Replay,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelBackend {
    NvidiaHosted,
    BrevHosted,
}

/// Selecting a backend never overwrites the other backend's endpoint or key.
pub struct ModelSettings {
    pub backend: ModelBackend,
    pub key: String,
    pub url: String,
    pub model: String,
}
impl ModelSettings {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }
    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let backend = match get("NEMOTRON_BACKEND")
            .as_deref()
            .unwrap_or("nvidia_hosted")
        {
            "nvidia_hosted" => ModelBackend::NvidiaHosted,
            "brev_hosted" => ModelBackend::BrevHosted,
            _ => anyhow::bail!("NEMOTRON_BACKEND must be nvidia_hosted or brev_hosted"),
        };
        let (key, url, model) = match backend {
            ModelBackend::NvidiaHosted => (
                get("NVIDIA_API_KEY").unwrap_or_default(),
                get("NEMOTRON_CHAT_URL").unwrap_or_else(|| {
                    "https://integrate.api.nvidia.com/v1/chat/completions".into()
                }),
                get("NEMOTRON_MODEL")
                    .unwrap_or_else(|| "nvidia/nemotron-3.5-lightning-30b-a3b".into()),
            ),
            ModelBackend::BrevHosted => (
                get("BREV_NEMOTRON_API_KEY").unwrap_or_default(),
                get("BREV_NEMOTRON_CHAT_URL").unwrap_or_default(),
                get("BREV_NEMOTRON_MODEL").unwrap_or_default(),
            ),
        };
        if backend == ModelBackend::BrevHosted {
            anyhow::ensure!(
                !key.trim().is_empty() && !model.trim().is_empty(),
                "Brev backend requires BREV_NEMOTRON_API_KEY and BREV_NEMOTRON_MODEL"
            );
            let endpoint = url::Url::parse(&url)
                .map_err(|_| anyhow::anyhow!("Configure BREV_NEMOTRON_CHAT_URL"))?;
            let loopback = matches!(
                endpoint.host_str(),
                Some("localhost" | "127.0.0.1" | "[::1]")
            );
            anyhow::ensure!(
                (endpoint.scheme() == "https" || (loopback && endpoint.scheme() == "http"))
                    && endpoint.username().is_empty()
                    && endpoint.password().is_none()
                    && endpoint.query().is_none()
                    && endpoint.fragment().is_none(),
                "Brev inference requires HTTPS or a loopback SSH forward, without credentials in its URL"
            );
            anyhow::ensure!(
                endpoint.host_str() != Some("integrate.api.nvidia.com"),
                "Brev backend must use the actual instance endpoint, not hosted NVIDIA inference"
            );
        }
        Ok(Self {
            backend,
            key,
            url,
            model,
        })
    }
}
#[derive(Clone)]
pub struct Config {
    pub public_base_url: String,
    pub port: u16,
    pub database_url: String,
    pub admin_token: String,
    pub mode: ExecutionMode,
    pub transport: String,
    pub account_sid: String,
    pub auth_token: String,
    pub photon_number: String,
    pub sip_host: String,
    pub sip_port: u16,
    pub sip_username: String,
    pub sip_password: String,
    pub contacts: BTreeMap<String, String>,
    pub smoke_authorized: bool,
    pub handoff_enabled: bool,
    pub family_enabled: bool,
    pub max_call_seconds: u32,
    pub nvidia_key: String,
    pub model_backend: ModelBackend,
    pub model_url: String,
    pub model: String,
}
impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let model_settings = ModelSettings::from_env()?;
        let relay_model =
            RelaySpeechModel::parse(std::env::var("RELAY_SPEECH_MODEL").ok().as_deref())?;
        let speech_timeout =
            parse_relay_speech_timeout(std::env::var("RELAY_SPEECH_TIMEOUT_MS").ok().as_deref())?;
        let get = |k: &str, default: &str| std::env::var(k).unwrap_or_else(|_| default.into());
        let mode = match get("DEMO_EXECUTION_MODE", "replay").as_str() {
            "live" => ExecutionMode::Live,
            "replay" => ExecutionMode::Replay,
            _ => anyhow::bail!("DEMO_EXECUTION_MODE must be live or replay"),
        };
        let public_base_url = get("PUBLIC_BASE_URL", "http://127.0.0.1:8080")
            .trim_end_matches('/')
            .to_string();
        let url = url::Url::parse(&public_base_url)?;
        anyhow::ensure!(
            url.path() == "/"
                && url.query().is_none()
                && url.fragment().is_none()
                && url.username().is_empty()
                && url.password().is_none(),
            "PUBLIC_BASE_URL must contain only scheme and host"
        );
        anyhow::ensure!(
            url.scheme() == "https" || (mode == ExecutionMode::Replay && url.scheme() == "http"),
            "Live public URL requires HTTPS"
        );
        anyhow::ensure!(
            get("VOICE_OUTPUT", "relay_native") == "relay_native",
            "Only VOICE_OUTPUT=relay_native is implemented; preset speech is not a clone"
        );
        let transport = get("VOICE_TRANSPORT", "probe");
        anyhow::ensure!(
            ["probe", "relay", "gather"].contains(&transport.as_str()),
            "VOICE_TRANSPORT must be probe, relay or gather"
        );
        let sip_host = get("PHOTON_SIP_HOST", "sip.photon.codes");
        anyhow::ensure!(
            !sip_host.is_empty()
                && sip_host
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-'),
            "Invalid PHOTON_SIP_HOST"
        );
        let mut contacts = BTreeMap::new();
        for (id, key) in [
            ("sarah", "SARAH_TEST_E164"),
            ("family", "FAMILY_TEST_E164"),
            ("clinic", "CLINIC_TEST_E164"),
        ] {
            let value = get(key, "");
            if !value.is_empty() {
                anyhow::ensure!(is_e164(&value), "Invalid enrolled E.164 for {id}");
                contacts.insert(id.into(), value);
            }
        }
        let photon_number = get("PHOTON_NUMBER", "");
        anyhow::ensure!(
            photon_number.is_empty() || is_e164(&photon_number),
            "Invalid PHOTON_NUMBER"
        );
        let account_sid = get("TWILIO_ACCOUNT_SID", "");
        anyhow::ensure!(
            account_sid.is_empty()
                || (account_sid.starts_with("AC")
                    && account_sid.len() == 34
                    && account_sid[2..].bytes().all(|b| b.is_ascii_hexdigit())),
            "Invalid TWILIO_ACCOUNT_SID"
        );
        let admin_token = get("DEMO_ADMIN_TOKEN", "");
        anyhow::ensure!(
            admin_token.is_empty() || admin_token.len() >= 24,
            "DEMO_ADMIN_TOKEN needs at least 24 characters"
        );
        let max_call_seconds = get("MAX_CALL_SECONDS", "180").parse::<u32>()?;
        anyhow::ensure!(
            (15..=600).contains(&max_call_seconds),
            "MAX_CALL_SECONDS outside 15..600"
        );
        install_relay_speech_model(&RELAY_SPEECH_MODEL, relay_model)?;
        install_relay_speech_timeout(&RELAY_SPEECH_TIMEOUT_MS, speech_timeout)?;
        Ok(Self {
            public_base_url,
            port: get("PORT", "8080").parse()?,
            database_url: get("DATABASE_URL", "sqlite:firstring.db"),
            admin_token,
            mode,
            transport,
            account_sid,
            auth_token: get("TWILIO_AUTH_TOKEN", ""),
            photon_number,
            sip_host,
            sip_port: get("PHOTON_SIP_PORT", "5061").parse()?,
            sip_username: get("PHOTON_SIP_USERNAME", ""),
            sip_password: get("PHOTON_SIP_PASSWORD", ""),
            contacts,
            smoke_authorized: get("TEST_CALLS_AUTHORIZED", "false") == "true",
            handoff_enabled: get("HANDOFF_CALLS_ENABLED", "false") == "true",
            family_enabled: get("FAMILY_CALLS_ENABLED", "false") == "true",
            max_call_seconds,
            nvidia_key: model_settings.key,
            model_backend: model_settings.backend,
            model_url: model_settings.url,
            model: model_settings.model,
        })
    }
    pub fn inbound_configured(&self) -> bool {
        !self.account_sid.is_empty()
            && !self.auth_token.is_empty()
            && self.public_base_url.starts_with("https://")
            && !self.public_base_url.contains("YOUR_HOST")
    }
    pub fn outbound_configured(&self) -> bool {
        self.inbound_configured()
            && !self.photon_number.is_empty()
            && !self.sip_username.is_empty()
            && !self.sip_password.is_empty()
            && !self.sip_host.is_empty()
            && self.sip_port != 0
    }
    pub fn relay_configured(&self) -> bool {
        // Local handshake/setup inputs only. Onboarding and live protocol proof
        // cannot be inferred from credential presence.
        self.inbound_configured()
    }
    pub fn model_configured(&self) -> bool {
        !self.nvidia_key.trim().is_empty()
            && !self.model_url.trim().is_empty()
            && !self.model.trim().is_empty()
    }
    pub fn selected_transport_configured(&self) -> bool {
        match self.transport.as_str() {
            "probe" => self.inbound_configured(),
            "relay" => self.relay_configured() && self.model_configured(),
            "gather" => self.inbound_configured() && self.model_configured(),
            _ => false,
        }
    }
    pub fn external_url(&self, path: &str) -> String {
        format!("{}{path}", self.public_base_url)
    }
}
pub fn is_e164(s: &str) -> bool {
    s.starts_with('+')
        && (9..=16).contains(&s.len())
        && s.as_bytes()[1] != b'0'
        && s[1..].bytes().all(|b| b.is_ascii_digit())
}
pub fn endpoint_fingerprint(url: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(url.as_bytes()))
}

#[cfg(test)]
mod model_settings_tests {
    use super::*;
    #[test]
    fn relay_speech_model_default_selection_and_invalid_values_are_bounded() {
        assert_eq!(
            RelaySpeechModel::parse(None).unwrap(),
            RelaySpeechModel::Nova3
        );
        assert_eq!(
            RelaySpeechModel::parse(Some("nova-3-general"))
                .unwrap()
                .as_str(),
            "nova-3-general"
        );
        assert_eq!(
            RelaySpeechModel::parse(Some("flux")).unwrap().as_str(),
            "flux"
        );
        for invalid in ["", "Flux", "nova-2", "flux\" voice=\"other", " flux"] {
            assert!(RelaySpeechModel::parse(Some(invalid)).is_err());
        }
    }

    #[test]
    fn startup_speech_selection_is_immutable_until_process_restart() {
        let slot = std::sync::OnceLock::new();
        install_relay_speech_model(&slot, RelaySpeechModel::Flux).unwrap();
        install_relay_speech_model(&slot, RelaySpeechModel::Flux).unwrap();
        assert!(install_relay_speech_model(&slot, RelaySpeechModel::Nova3).is_err());
        assert_eq!(slot.get(), Some(&RelaySpeechModel::Flux));
    }
    #[test]
    fn speech_timeout_experiment_is_bounded_and_immutable() {
        assert_eq!(parse_relay_speech_timeout(None).unwrap(), 1000);
        assert_eq!(parse_relay_speech_timeout(Some("600")).unwrap(), 600);
        for invalid in ["0", "599", "601", "-1", "10000", "600ms"] {
            assert!(parse_relay_speech_timeout(Some(invalid)).is_err());
        }
        let slot = OnceLock::new();
        install_relay_speech_timeout(&slot, 600).unwrap();
        install_relay_speech_timeout(&slot, 600).unwrap();
        assert!(install_relay_speech_timeout(&slot, 1000).is_err());
        assert_eq!(slot.get(), Some(&600));
    }

    fn settings(values: &[(&str, &str)]) -> anyhow::Result<ModelSettings> {
        ModelSettings::from_lookup(|key| {
            values
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        })
    }
    #[test]
    fn backend_selection_keeps_hosted_fallback_and_separate_authentication() {
        let hosted = settings(&[
            ("NVIDIA_API_KEY", "hosted-test"),
            ("BREV_NEMOTRON_API_KEY", "instance-test"),
        ])
        .unwrap();
        assert_eq!(hosted.backend, ModelBackend::NvidiaHosted);
        assert_eq!(hosted.key, "hosted-test");
        let brev = settings(&[
            ("NEMOTRON_BACKEND", "brev_hosted"),
            ("NVIDIA_API_KEY", "hosted-test"),
            ("BREV_NEMOTRON_API_KEY", "instance-test"),
            (
                "BREV_NEMOTRON_CHAT_URL",
                "http://127.0.0.1:18080/v1/chat/completions",
            ),
            ("BREV_NEMOTRON_MODEL", "actual-checkpoint"),
        ])
        .unwrap();
        assert_eq!(brev.backend, ModelBackend::BrevHosted);
        assert_eq!(brev.key, "instance-test");
        assert_eq!(brev.model, "actual-checkpoint");
    }
    #[test]
    fn brev_requires_its_own_key_and_protected_endpoint() {
        let base = [
            ("NEMOTRON_BACKEND", "brev_hosted"),
            ("BREV_NEMOTRON_MODEL", "checkpoint"),
            ("NVIDIA_API_KEY", "hosted-test"),
        ];
        for endpoint in [
            "http://198.51.100.1/v1/chat/completions",
            "https://key:secret@example.test/v1/chat/completions",
            "https://integrate.api.nvidia.com/v1/chat/completions",
        ] {
            let mut v = base.to_vec();
            v.extend([
                ("BREV_NEMOTRON_API_KEY", "instance-test"),
                ("BREV_NEMOTRON_CHAT_URL", endpoint),
            ]);
            assert!(settings(&v).is_err());
        }
        let mut missing = base.to_vec();
        missing.push((
            "BREV_NEMOTRON_CHAT_URL",
            "http://127.0.0.1:18080/v1/chat/completions",
        ));
        assert!(
            settings(&missing).is_err(),
            "Hosted key must never silently authenticate to the instance"
        );
    }
}
