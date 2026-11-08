//! Trusted callers: numbers a household chooses never to have screened.
//!
//! A trusted call skips the AI entirely: FirstRing greets the caller by name and
//! dials the household's enrolled phone directly. Numbers stay server-side (the
//! API returns only the last four digits), trusted calls are not recorded or
//! reputation-checked, and the pass-through is an event on the call's session.
//! Caller ID can be spoofed, so the console says plainly that a trusted number
//! should be one the household would answer anyway.
use crate::{
    calls::{actor::App, state::Session},
    config::{Config, is_e164},
    providers::twilio::{escape, say_end},
    store::Store,
};
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const HOUSEHOLDS: [&str; 2] = ["sarah", "maya"];
const MAX_ENTRIES: usize = 25;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trusted {
    pub id: String,
    pub name: String,
    pub relation: String,
    pub number: String,
    pub added_at: String,
}

fn key(household: &str) -> String {
    format!("trusted.v1:{household}")
}

fn check_household(household: &str) -> Result<()> {
    ensure!(HOUSEHOLDS.contains(&household), "Unknown household");
    Ok(())
}

/// Normalizes a typed US or international number to E.164.
pub fn normalize(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    // A bare ten-digit number is a US number; anything else is read as given.
    let candidate = if !trimmed.starts_with('+') && digits.len() == 10 {
        format!("+1{digits}")
    } else {
        format!("+{digits}")
    };
    ensure!(
        is_e164(&candidate) && candidate.len() >= 11,
        "Enter a full phone number, for example (512) 555-0123"
    );
    Ok(candidate)
}

fn clean_label(value: &str, field: &str, required: bool) -> Result<String> {
    let value = value.trim();
    ensure!(
        value.chars().count() <= 40,
        "{field} must be 40 characters or fewer"
    );
    ensure!(!required || !value.is_empty(), "{field} is required");
    ensure!(
        value
            .chars()
            .all(|c| c.is_alphabetic() || " '-’.".contains(c)),
        "{field} can use letters, spaces and ordinary name punctuation"
    );
    Ok(value.to_string())
}

pub async fn list(store: &Store, household: &str) -> Result<Vec<Trusted>> {
    check_household(household)?;
    Ok(match store.metadata(&key(household)).await? {
        Some(raw) => serde_json::from_str(&raw)?,
        None => vec![],
    })
}

async fn save(store: &Store, household: &str, entries: &[Trusted]) -> Result<()> {
    store
        .set_metadata(&key(household), &serde_json::to_string(entries)?)
        .await
}

pub async fn add(
    store: &Store,
    household: &str,
    name: &str,
    relation: &str,
    number: &str,
) -> Result<Vec<Trusted>> {
    let mut entries = list(store, household).await?;
    let name = clean_label(name, "Name", true)?;
    let relation = clean_label(relation, "Relationship", false)?;
    let number = normalize(number)?;
    ensure!(
        !entries.iter().any(|e| e.number == number),
        "That number is already on the trusted list"
    );
    ensure!(entries.len() < MAX_ENTRIES, "The trusted list is full");
    entries.push(Trusted {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        relation,
        number,
        added_at: chrono::Utc::now().to_rfc3339(),
    });
    save(store, household, &entries).await?;
    Ok(entries)
}

pub async fn remove(store: &Store, household: &str, id: &str) -> Result<Vec<Trusted>> {
    let mut entries = list(store, household).await?;
    let before = entries.len();
    entries.retain(|e| e.id != id);
    if entries.len() == before {
        bail!("That trusted caller was not found");
    }
    save(store, household, &entries).await?;
    Ok(entries)
}

fn suffix(number: &str) -> String {
    number
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// What the console sees: never the full number.
pub fn view(household: &str, entries: &[Trusted], config: &Config) -> Value {
    json!({
        "household_id": household,
        "items": entries.iter().map(|e| json!({
            "id": e.id,
            "name": e.name,
            "relation": e.relation,
            "number_label": format!("ending {}", suffix(&e.number)),
            "added_at": e.added_at,
        })).collect::<Vec<_>>(),
        "connects_to_enrolled_phone": config.contacts.contains_key("sarah"),
        "meaning": "Calls from these numbers skip AI screening and ring straight through. Caller ID can be faked, so only add numbers you would answer anyway.",
    })
}

pub async fn find(store: &Store, household: &str, from: &str) -> Result<Option<Trusted>> {
    let Ok(number) = normalize(from) else {
        return Ok(None);
    };
    Ok(list(store, household)
        .await?
        .into_iter()
        .find(|e| e.number == number))
}

/// TwiML for a trusted caller, plus the receipt on the call's session.
pub async fn passthrough(app: &App, session: &Session, caller: &Trusted) -> Result<String> {
    let household = session
        .profile
        .as_ref()
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| session.household_id.clone());
    let c = &app.config;
    // The enrolled phone is the household's own; only Sarah's line has one in this demo.
    let target = c.contacts.get("sarah").cloned();
    let outcome = match &target {
        Some(t) if *t == caller.number => "caller_is_enrolled_phone",
        Some(_) if c.photon_number.is_empty() || c.sip_username.is_empty() => "no_outbound_route",
        Some(_) => "dialing",
        None => "no_enrolled_phone",
    };
    app.store
        .append(
            Some(session),
            "system",
            "trusted.passthrough",
            json!({"session_id":session.id,"household_id":session.household_id,"name":caller.name,"relation":caller.relation,"number_label":format!("ending {}", suffix(&caller.number)),"outcome":outcome,"screened":false,"recorded":false}),
            session.execution_mode,
        )
        .await?;
    let first = caller
        .name
        .split_whitespace()
        .next()
        .unwrap_or(&caller.name);
    Ok(match (outcome, target) {
        ("dialing", Some(target)) => format!(
            "<Response><Say voice=\"Polly.Joanna-Neural\" language=\"en-US\">{}</Say><Dial answerOnBridge=\"true\" timeout=\"25\" timeLimit=\"{}\" callerId=\"{}\" action=\"{}\" method=\"POST\"><Sip username=\"{}\" password=\"{}\">sip:{}@{}:{};transport=tls;secure=true</Sip></Dial></Response>",
            escape(&format!(
                "Hi {first}. You're on {household}'s trusted list, so FirstRing is putting you straight through."
            )),
            c.max_call_seconds,
            escape(&c.photon_number),
            escape(&c.external_url(&format!("/voice/trusted-done?session_id={}", session.id))),
            escape(&c.sip_username),
            escape(&c.sip_password),
            escape(&target),
            escape(&c.sip_host),
            c.sip_port,
        ),
        ("caller_is_enrolled_phone", _) => say_end(&format!(
            "Hi {first}. You're on {household}'s trusted list, and you're calling from the phone FirstRing rings for {household}, so there's no one else to connect. Goodbye."
        )),
        _ => say_end(&format!(
            "Hi {first}. You're on {household}'s trusted list, but there's no phone set up to connect you right now. Please try again later. Goodbye."
        )),
    })
}

/// Receipt for the end of a trusted pass-through dial.
pub async fn completed(app: &App, session_id: &str, status: &str, seconds: &str) -> Result<()> {
    ensure!(session_id.len() <= 64, "Invalid session ID");
    let raw: Option<String> = sqlx::query_scalar("SELECT data FROM sessions WHERE id=?")
        .bind(session_id)
        .fetch_optional(&app.store.pool)
        .await?;
    let Some(raw) = raw else {
        bail!("Session not found");
    };
    let session: Session = serde_json::from_str(&raw)?;
    app.store
        .append(
            Some(&session),
            "system",
            "trusted.passthrough.completed",
            json!({"session_id":session.id,"dial_status":status,"duration_seconds":seconds.parse::<u64>().ok()}),
            session.execution_mode,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbers_normalize_to_e164() {
        assert_eq!(normalize("(512) 555-0123").unwrap(), "+15125550123");
        assert_eq!(normalize("1 512 555 0123").unwrap(), "+15125550123");
        assert_eq!(normalize("+44 20 7946 0958").unwrap(), "+442079460958");
        assert!(normalize("911").is_err());
        assert!(normalize("").is_err());
    }
    #[test]
    fn labels_reject_markup() {
        assert!(clean_label("Jimmy", "Name", true).is_ok());
        assert!(clean_label("Mary-Jo O’Neil", "Name", true).is_ok());
        assert!(clean_label("<b>", "Name", true).is_err());
        assert!(clean_label("", "Name", true).is_err());
        assert!(clean_label("", "Relationship", false).is_ok());
    }
}
