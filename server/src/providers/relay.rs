//! ConversationRelay text protocol, independent of the socket and parent actor.
//! Source: https://www.twilio.com/docs/voice/conversationrelay/websocket-messages
use anyhow::{Result, anyhow, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use url::Url;

// Twilio-managed preset TTS, not a voice clone or a separate reasoning agent.
// https://www.twilio.com/docs/voice/conversationrelay/voice-configuration
const TTS_PROVIDER: &str = "ElevenLabs";
const TTS_VOICE: &str = "21m00Tcm4TlvDq8ikWAM-v4";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayInput {
    Setup {
        call_sid: String,
        account_sid: String,
        session_id: String,
        session_token: String,
    },
    Prompt {
        voice_prompt: String,
        last: bool,
        lang: String,
    },
    Interrupt {
        utterance_until_interrupt: String,
        duration_ms: u64,
    },
    Dtmf {
        digit: String,
    },
    Error {
        code: Option<u32>,
        description: String,
    },
    /// Optional diagnostics have no caller-speech semantics.
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum WireInput {
    Setup {
        #[serde(rename = "callSid")]
        call_sid: String,
        #[serde(rename = "accountSid")]
        account_sid: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "customParameters")]
        parameters: SetupParameters,
    },
    Prompt {
        #[serde(rename = "voicePrompt")]
        voice_prompt: String,
        last: bool,
        lang: String,
    },
    Interrupt {
        #[serde(rename = "utteranceUntilInterrupt")]
        utterance_until_interrupt: String,
        #[serde(rename = "durationUntilInterruptMs")]
        duration_ms: u64,
    },
    Dtmf {
        digit: String,
    },
    Error {
        code: Option<u32>,
        description: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct SetupParameters {
    session_token: String,
}

pub fn parse(message: &str) -> Result<RelayInput> {
    ensure!(message.len() <= 32_768, "Relay message exceeds size limit");
    let wire: WireInput =
        serde_json::from_str(message).map_err(|_| anyhow!("invalid Relay message"))?;
    let input = match wire {
        WireInput::Setup {
            call_sid,
            account_sid,
            session_id,
            parameters,
        } => {
            ensure!(
                valid_sid(&call_sid, "CA")
                    && valid_sid(&account_sid, "AC")
                    && valid_sid(&session_id, "VX"),
                "invalid Relay setup identifier"
            );
            ensure!(
                !parameters.session_token.is_empty()
                    && parameters.session_token.len() <= 256
                    && !parameters.session_token.chars().any(char::is_control),
                "missing or invalid Relay session token"
            );
            RelayInput::Setup {
                call_sid,
                account_sid,
                session_id,
                session_token: parameters.session_token,
            }
        }
        WireInput::Prompt {
            voice_prompt,
            last,
            lang,
        } => {
            ensure!(
                voice_prompt.chars().count() <= 4096 && lang.len() <= 32,
                "Relay prompt exceeds size limit"
            );
            RelayInput::Prompt {
                voice_prompt,
                last,
                lang,
            }
        }
        WireInput::Interrupt {
            utterance_until_interrupt,
            duration_ms,
        } => {
            ensure!(
                utterance_until_interrupt.chars().count() <= 4096,
                "Relay interrupt exceeds size limit"
            );
            RelayInput::Interrupt {
                utterance_until_interrupt,
                duration_ms,
            }
        }
        WireInput::Dtmf { digit } => {
            ensure!(
                digit.len() == 1 && digit.chars().all(|c| "0123456789*#ABCD".contains(c)),
                "invalid Relay DTMF digit"
            );
            RelayInput::Dtmf { digit }
        }
        WireInput::Error { code, description } => {
            ensure!(
                description.chars().count() <= 2048,
                "Relay error exceeds size limit"
            );
            RelayInput::Error { code, description }
        }
        // The official docs name clientSpeaking/agentSpeaking but do not publish their
        // envelope or state values. We do not subscribe to them. Diagnostics must
        // neither start nor finish caller speech: doing so can cancel a valid reply
        // or hold consent forever. Only prompt/interrupt drive the caller-speech gate.
        WireInput::Other => RelayInput::Other,
    };
    Ok(input)
}

fn valid_sid(value: &str, prefix: &str) -> bool {
    value.len() == 34
        && value.starts_with(prefix)
        && value[2..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn connect_twiml(base_url: &str, session_token: &str, display_name: &str) -> String {
    let Ok(base) = public_origin(base_url) else {
        return super::twilio::say_end("The assistant is temporarily unavailable.");
    };
    let mut ws = base.clone();
    // HTTPS and WSS share this configured public authority, independent of Host headers.
    if ws.set_scheme("wss").is_err() {
        return "<Response><Hangup/></Response>".into();
    }
    ws.set_path("/voice/relay");
    let mut after = base;
    after.set_path("/voice/after-relay");
    let greeting = format!("I'm {display_name}'s automated assistant. What is the call about?");
    format!(
        "<Response><Connect action=\"{}\" method=\"POST\"><ConversationRelay url=\"{}\" welcomeGreeting=\"{}\" interruptible=\"any\" reportInputDuringAgentSpeech=\"any\" dtmfDetection=\"true\" partialPrompts=\"true\" speechTimeout=\"1000\" language=\"en-US\" transcriptionProvider=\"Deepgram\" speechModel=\"nova-3-general\" ttsProvider=\"{TTS_PROVIDER}\" voice=\"{TTS_VOICE}\"><Parameter name=\"session_token\" value=\"{}\"/></ConversationRelay></Connect></Response>",
        escape(after.as_str()),
        escape(ws.as_str()),
        escape(&greeting),
        escape(session_token)
    )
}

/// The actor supplies a validated short plain-text reply, never raw model content.
pub fn text(reply: &str) -> Value {
    json!({"type":"text","token":reply,"last":true,"interruptible":true,"preemptible":true})
}

pub fn end(reason: &str) -> Value {
    json!({"type":"end","handoffData":json!({"reason":reason}).to_string()})
}

#[derive(Clone, Debug)]
pub struct AudioSource(String);

impl AudioSource {
    pub fn from_server_url(base_url: &str, audio_url: &str) -> Result<Self> {
        let base = public_origin(base_url)?;
        let audio = Url::parse(audio_url).map_err(|_| anyhow!("invalid audio URL"))?;
        ensure!(
            audio.scheme() == "https"
                && audio.origin() == base.origin()
                && audio.username().is_empty()
                && audio.password().is_none()
                && audio.query().is_none()
                && audio.fragment().is_none(),
            "audio must use the configured HTTPS origin"
        );
        let token = audio
            .path()
            .strip_prefix("/audio/")
            .ok_or_else(|| anyhow!("invalid audio route"))?;
        ensure!(
            !token.is_empty()
                && token.len() <= 128
                && token
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_- .".contains(c))
                && !token.contains(' '),
            "invalid audio token"
        );
        Ok(Self(audio.into()))
    }
}

pub fn play(source: &AudioSource) -> Value {
    json!({"type":"play","source":source.0,"loop":1,"interruptible":true,"preemptible":true})
}

fn public_origin(base_url: &str) -> Result<Url> {
    let base = Url::parse(base_url).map_err(|_| anyhow!("invalid public origin"))?;
    ensure!(
        base.scheme() == "https"
            && base.host_str().is_some()
            && base.username().is_empty()
            && base.password().is_none()
            && base.query().is_none()
            && base.fragment().is_none()
            && base.path() == "/",
        "public origin must be a clean HTTPS origin"
    );
    Ok(base)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Value {
        json!({"type":"setup","sessionId":"VX00000000000000000000000000000001","accountSid":"AC00000000000000000000000000000002","callSid":"CA00000000000000000000000000000003","parentCallSid":"","from":"sip:unknown@example.test","to":"sip:firstring@example.test","direction":"inbound","callStatus":"RINGING","customParameters":{"session_token":"opaque-token-for-registered-call"}})
    }

    #[test]
    fn setup_retains_ids_and_opaque_token_for_actor_binding() {
        let input = parse(&setup().to_string()).unwrap();
        assert_eq!(
            input,
            RelayInput::Setup {
                call_sid: "CA00000000000000000000000000000003".into(),
                account_sid: "AC00000000000000000000000000000002".into(),
                session_id: "VX00000000000000000000000000000001".into(),
                session_token: "opaque-token-for-registered-call".into()
            }
        );
        let mut missing = setup();
        missing["customParameters"] = json!({});
        assert!(parse(&missing.to_string()).is_err());
        let mut missing = setup();
        missing["accountSid"] = Value::Null;
        assert!(parse(&missing.to_string()).is_err());
    }

    #[test]
    fn partial_prompt_stays_partial_until_explicit_last_true() {
        let partial =
            parse(r#"{"type":"prompt","voicePrompt":"Please read","lang":"en-US","last":false}"#)
                .unwrap();
        assert_eq!(
            partial,
            RelayInput::Prompt {
                voice_prompt: "Please read".into(),
                last: false,
                lang: "en-US".into()
            }
        );
        let final_turn = parse(r#"{"type":"prompt","voicePrompt":"Please read the sign-in digits.","lang":"en-US","last":true}"#).unwrap();
        assert!(matches!(final_turn, RelayInput::Prompt { last: true, .. }));
        assert!(parse(r#"{"type":"prompt","voicePrompt":"hi","lang":"en-US"}"#).is_err());
        assert!(
            parse(r#"{"type":"prompt","voicePrompt":"hi","lang":"en-US","last":"true"}"#).is_err()
        );
    }

    #[test]
    fn interrupt_and_dtmf_use_relay_fields_not_media_streams_fields() {
        assert_eq!(parse(r#"{"type":"interrupt","utteranceUntilInterrupt":"The appointment is","durationUntilInterruptMs":460}"#).unwrap(), RelayInput::Interrupt {utterance_until_interrupt:"The appointment is".into(),duration_ms:460});
        assert_eq!(
            parse(r#"{"type":"dtmf","digit":"1"}"#).unwrap(),
            RelayInput::Dtmf { digit: "1".into() }
        );
        assert!(parse(r#"{"type":"dtmf","digit":"12"}"#).is_err());
        assert!(parse(r#"{"type":"dtmf","dtmf":{"digit":"1"}}"#).is_err());
    }

    #[test]
    fn optional_diagnostics_have_no_caller_speech_semantics() {
        assert_eq!(
            parse(r#"{"type":"info","name":"clientSpeaking","value":"off"}"#).unwrap(),
            RelayInput::Other
        );
        assert_eq!(
            parse(r#"{"type":"clientSpeaking","value":false}"#).unwrap(),
            RelayInput::Other
        );
        assert_eq!(parse(r#"{"type":"info","name":"configUpdated","value":{"session":{"ttsLanguage":"en-US"}}}"#).unwrap(), RelayInput::Other);
    }

    #[test]
    fn error_can_be_parsed_without_undocumented_numeric_code() {
        assert_eq!(
            parse(r#"{"type":"error","description":"Invalid message received"}"#).unwrap(),
            RelayInput::Error {
                code: None,
                description: "Invalid message received".into()
            }
        );
        assert!(parse("not-json").is_err());
        assert!(parse(&"x".repeat(32769)).is_err());
    }

    #[test]
    fn twiml_uses_secure_route_reports_speech_and_escapes_attributes() {
        let xml = connect_twiml(
            "https://firstring.example",
            "opaque<&\"token",
            "Sarah & Maya",
        );
        assert!(xml.contains("action=\"https://firstring.example/voice/after-relay\""));
        assert!(xml.contains("url=\"wss://firstring.example/voice/relay\""));
        assert!(xml.contains("reportInputDuringAgentSpeech=\"any\""));
        assert!(xml.contains("partialPrompts=\"true\""));
        assert!(xml.contains("speechTimeout=\"1000\""));
        assert!(xml.contains("transcriptionProvider=\"Deepgram\""));
        assert!(xml.contains("speechModel=\"nova-3-general\""));
        assert!(xml.contains("ttsProvider=\"ElevenLabs\""));
        assert!(xml.contains("voice=\"21m00Tcm4TlvDq8ikWAM-v4\""));
        assert!(!xml.contains("speaker-events"));
        assert!(xml.contains("name=\"session_token\" value=\"opaque&lt;&amp;&quot;token\""));
        assert!(xml.contains("Sarah &amp; Maya"));
        assert!(!xml.contains("<Stream"));
        assert!(!xml.contains("opaque<&\"token"));
    }

    #[test]
    fn invalid_connect_origin_fails_closed_without_emitting_a_websocket() {
        let xml = connect_twiml("http://untrusted.invalid", "token", "Sarah");
        assert!(xml.contains("<Hangup"));
        assert!(!xml.contains("ConversationRelay"));
    }

    #[test]
    fn spoken_json_and_end_payload_follow_current_protocol() {
        assert_eq!(
            text("What is Tuesday's update?"),
            json!({"type":"text","token":"What is Tuesday's update?","last":true,"interruptible":true,"preemptible":true})
        );
        let end = end("bridge_authorized");
        assert_eq!(end["type"], "end");
        let handoff: Value = serde_json::from_str(end["handoffData"].as_str().unwrap()).unwrap();
        assert_eq!(handoff, json!({"reason":"bridge_authorized"}));
    }

    #[test]
    fn generated_audio_is_confined_to_our_https_audio_route() {
        let source = AudioSource::from_server_url(
            "https://firstring.example",
            "https://firstring.example/audio/opaque-token.wav",
        )
        .unwrap();
        assert_eq!(
            play(&source),
            json!({"type":"play","source":"https://firstring.example/audio/opaque-token.wav","loop":1,"interruptible":true,"preemptible":true})
        );
        for candidate in [
            "https://untrusted.invalid/audio/a.wav",
            "http://firstring.example/audio/a.wav",
            "https://firstring.example/private/reference.wav",
            "https://user:secret@firstring.example/audio/a.wav",
            "https://firstring.example/audio/a.wav?private=secret",
        ] {
            assert!(AudioSource::from_server_url("https://firstring.example", candidate).is_err());
        }
    }
}
