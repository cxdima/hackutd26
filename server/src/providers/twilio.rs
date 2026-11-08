use crate::{
    calls::state::Leg,
    config::{Config, ExecutionMode},
};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct Twilio {
    pub config: Arc<Config>,
    client: reqwest::Client,
}
#[derive(Debug)]
pub enum CreateOutcome {
    Accepted { sid: String, status: String },
    Rejected { status: u16, code: Option<u32> },
    Unknown,
}
#[derive(Deserialize)]
struct CreatedCall {
    sid: String,
    status: String,
}
impl Twilio {
    pub fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
    pub fn call_form(&self, leg: &Leg, target: &str, twiml_path: &str) -> Vec<(String, String)> {
        let c = &self.config;
        let mut form = vec![
            (
                "To".into(),
                format!(
                    "sip:{target}@{}:{};transport=tls;secure=true",
                    c.sip_host, c.sip_port
                ),
            ),
            ("From".into(), c.photon_number.clone()),
            ("SipAuthUsername".into(), c.sip_username.clone()),
            ("SipAuthPassword".into(), c.sip_password.clone()),
            ("Url".into(), c.external_url(twiml_path)),
            ("Method".into(), "POST".into()),
            (
                "StatusCallback".into(),
                c.external_url(&format!("/voice/status?action_id={}", leg.action_id)),
            ),
            ("StatusCallbackMethod".into(), "POST".into()),
            ("Timeout".into(), "20".into()),
            ("TimeLimit".into(), c.max_call_seconds.to_string()),
        ];
        for event in ["initiated", "ringing", "answered", "completed"] {
            form.push(("StatusCallbackEvent".into(), event.into()));
        }
        form
    }
    pub async fn create(&self, leg: &Leg, twiml_path: &str) -> CreateOutcome {
        let c = &self.config;
        if c.mode != ExecutionMode::Live || !c.outbound_configured() {
            return CreateOutcome::Rejected {
                status: 503,
                code: None,
            };
        }
        let allowed = match leg.kind.as_str() {
            "probe" => leg.role == "sarah" && c.smoke_authorized,
            "handoff" => leg.role == "sarah" && c.handoff_enabled,
            "trusted" => leg.role == "family" && c.family_enabled,
            _ => false,
        };
        if !allowed {
            return CreateOutcome::Rejected {
                status: 403,
                code: None,
            };
        }
        let Some(target) = c.contacts.get(&leg.role) else {
            return CreateOutcome::Rejected {
                status: 403,
                code: None,
            };
        };
        self.submit(self.call_form(leg, target, twiml_path)).await
    }
    /// Dedicated one-attempt test harness; it cannot select a contact or a number.
    /// The harness persists its global reservation before calling this method.
    pub(crate) async fn create_fixture(&self, leg: &Leg) -> CreateOutcome {
        let c = &self.config;
        if c.mode != ExecutionMode::Live || !c.outbound_configured() {
            return CreateOutcome::Rejected {
                status: 503,
                code: None,
            };
        }
        if leg.kind != "fixture_caller"
            || leg.role != "fixture_caller"
            || c.smoke_authorized
            || c.handoff_enabled
            || c.family_enabled
        {
            return CreateOutcome::Rejected {
                status: 403,
                code: None,
            };
        }
        self.submit(self.fixture_call_form(leg)).await
    }
    pub(crate) fn fixture_call_form(&self, leg: &Leg) -> Vec<(String, String)> {
        let c = &self.config;
        let mut form = self.call_form(
            leg,
            &c.photon_number,
            &format!("/voice/fixture/{}/0", leg.action_id),
        );
        for (key, value) in &mut form {
            if key == "TimeLimit" {
                *value = c.max_call_seconds.min(90).to_string();
            }
        }
        form.extend([
            ("Record".into(), "true".into()),
            ("RecordingChannels".into(), "dual".into()),
            ("RecordingTrack".into(), "both".into()),
            (
                "RecordingStatusCallback".into(),
                c.external_url(&format!("/voice/fixture-recording/{}", leg.action_id)),
            ),
            ("RecordingStatusCallbackMethod".into(), "POST".into()),
            ("RecordingStatusCallbackEvent".into(), "completed".into()),
        ]);
        form
    }
    pub(crate) async fn fixture_recording(&self, recording: &str) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(
            self.config.mode == ExecutionMode::Live,
            "Replay cannot retrieve provider media"
        );
        anyhow::ensure!(
            recording.starts_with("RE")
                && recording.len() == 34
                && recording[2..].bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid recording identity"
        );
        let mut response = self
            .client
            .get(format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}/Recordings/{recording}.mp3",
                self.config.account_sid
            ))
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Recording fetch failed"))?;
        anyhow::ensure!(
            response.status().is_success(),
            "Recording unavailable ({})",
            response.status().as_u16()
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| anyhow::anyhow!("Recording transfer failed"))?
        {
            anyhow::ensure!(
                bytes.len() + chunk.len() <= 10_000_000,
                "Recording exceeds bounded size"
            );
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
    async fn submit(&self, form: Vec<(String, String)>) -> CreateOutcome {
        let c = &self.config;
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Calls.json",
            c.account_sid
        );
        // No automatic retry: a timeout/5xx may have created a call. The reserved ID in
        // StatusCallback reconciles it even if the response never arrives.
        let Ok(response) = self
            .client
            .post(url)
            .basic_auth(&c.account_sid, Some(&c.auth_token))
            .form(&form)
            .send()
            .await
        else {
            return CreateOutcome::Unknown;
        };
        let status = response.status();
        if status.is_client_error() {
            #[derive(Deserialize)]
            struct ErrorCode {
                code: u32,
            }
            let code = response
                .json::<ErrorCode>()
                .await
                .ok()
                .map(|error| error.code);
            if let Some(code) = code {
                tracing::warn!(
                    http_status = status.as_u16(),
                    provider_error_code = code,
                    "Twilio call request rejected"
                );
            }
            return CreateOutcome::Rejected {
                status: status.as_u16(),
                code,
            };
        }
        if !status.is_success() {
            return CreateOutcome::Unknown;
        }
        match response.json::<CreatedCall>().await {
            Ok(call) if valid_call_sid(&call.sid) => CreateOutcome::Accepted {
                sid: call.sid,
                status: call.status,
            },
            _ => CreateOutcome::Unknown,
        }
    }
    pub async fn update(&self, sid: &str, fields: &[(&str, String)]) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.config.mode == ExecutionMode::Live,
            "Replay cannot update provider calls"
        );
        anyhow::ensure!(
            !self.config.account_sid.is_empty() && !self.config.auth_token.is_empty(),
            "Twilio control credentials are not configured"
        );
        anyhow::ensure!(valid_call_sid(sid), "Invalid CallSid");
        let response = self
            .client
            .post(format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}/Calls/{sid}.json",
                self.config.account_sid
            ))
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(fields)
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Provider update outcome unknown"))?;
        anyhow::ensure!(
            response.status().is_success(),
            "Provider update failed ({})",
            response.status().as_u16()
        );
        Ok(())
    }
}
pub fn valid_call_sid(sid: &str) -> bool {
    sid.starts_with("CA") && sid.len() == 34 && sid[2..].bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn say_end(text: &str) -> String {
    format!(
        "<Response><Say voice=\"Polly.Joanna-Neural\" language=\"en-US\">{}</Say><Hangup/></Response>",
        escape(text)
    )
}
pub fn gather(url: &str, text: &str, timeout: u32) -> String {
    format!(
        "<Response><Gather input=\"dtmf\" numDigits=\"1\" timeout=\"{timeout}\" actionOnEmptyResult=\"true\" action=\"{}\" method=\"POST\"><Say voice=\"Polly.Joanna-Neural\" language=\"en-US\">{}</Say></Gather><Hangup/></Response>",
        escape(url),
        escape(text)
    )
}
pub fn speech_gather(url: &str, text: &str) -> String {
    format!(
        "<Response><Gather input=\"speech\" speechTimeout=\"auto\" timeout=\"8\" actionOnEmptyResult=\"true\" action=\"{}\" method=\"POST\"><Say voice=\"Polly.Joanna-Neural\" language=\"en-US\">{}</Say></Gather><Say voice=\"Polly.Joanna-Neural\" language=\"en-US\">No input received. Goodbye.</Say><Hangup/></Response>",
        escape(url),
        escape(text)
    )
}
pub fn conference(name: &str, callback: &str, max_seconds: u32) -> String {
    format!(
        "<Response><Dial timeLimit=\"{max_seconds}\"><Conference beep=\"false\" startConferenceOnEnter=\"true\" endConferenceOnExit=\"true\" maxParticipants=\"2\" statusCallback=\"{}\" statusCallbackMethod=\"POST\" statusCallbackEvent=\"join leave end\">{}</Conference></Dial></Response>",
        escape(callback),
        escape(name)
    )
}
