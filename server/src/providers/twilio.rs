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
    Rejected { status: u16 },
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
            return CreateOutcome::Rejected { status: 503 };
        }
        let allowed = match leg.kind.as_str() {
            "probe" => leg.role == "sarah" && c.smoke_authorized,
            "handoff" => leg.role == "sarah" && c.handoff_enabled,
            "trusted" => leg.role == "family" && c.family_enabled,
            _ => false,
        };
        if !allowed {
            return CreateOutcome::Rejected { status: 403 };
        }
        let Some(target) = c.contacts.get(&leg.role) else {
            return CreateOutcome::Rejected { status: 403 };
        };
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
            .form(&self.call_form(leg, target, twiml_path))
            .send()
            .await
        else {
            return CreateOutcome::Unknown;
        };
        let status = response.status();
        if status.is_client_error() {
            return CreateOutcome::Rejected {
                status: status.as_u16(),
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
