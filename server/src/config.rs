use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Live,
    Replay,
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
    pub model_url: String,
    pub model: String,
}
impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
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
            nvidia_key: get("NVIDIA_API_KEY", ""),
            model_url: get(
                "NEMOTRON_CHAT_URL",
                "https://integrate.api.nvidia.com/v1/chat/completions",
            ),
            model: get("NEMOTRON_MODEL", "nvidia/nemotron-3.5-lightning-30b-a3b"),
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
