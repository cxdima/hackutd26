//! Opt-in recording of screened calls (`CALL_RECORDING=on`) so the owner can
//! listen back in the console. Dual channel: caller and FirstRing.
use crate::{calls::actor::App, calls::state::Session, config::ExecutionMode};
use anyhow::Result;
use serde_json::{Value, json};

pub fn enabled() -> bool {
    std::env::var("CALL_RECORDING").as_deref() == Ok("on")
}
fn key(session_id: &str) -> String {
    format!("recording.v1:{session_id}")
}

/// Starts a recording shortly after the call is connected (Twilio requires an in-progress call).
pub fn start(app: App, session_id: String, call_sid: String) {
    if !enabled() || app.config.mode != ExecutionMode::Live || call_sid.is_empty() {
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        let callback = app
            .config
            .external_url(&format!("/voice/recording-status?session_id={session_id}"));
        if let Err(error) = app.twilio.start_recording(&call_sid, &callback).await {
            tracing::warn!(%error, "call recording did not start");
        }
    });
}

pub async fn completed(
    app: &App,
    session_id: &str,
    recording_sid: &str,
    seconds: u32,
) -> Result<()> {
    anyhow::ensure!(session_id.len() <= 128, "Invalid session");
    app.store
        .set_metadata(
            &key(session_id),
            &json!({"recording_sid": recording_sid, "duration_seconds": seconds,
                "channels": 2, "completed_at": chrono::Utc::now()})
            .to_string(),
        )
        .await?;
    let raw: Option<String> = sqlx::query_scalar("SELECT data FROM sessions WHERE id=?")
        .bind(session_id)
        .fetch_optional(&app.store.pool)
        .await?;
    if let Some(raw) = raw {
        let session: Session = serde_json::from_str(&raw)?;
        app.store
            .append(
                Some(&session),
                "system",
                "recording.completed",
                json!({"duration_seconds": seconds, "channels": 2}),
                session.execution_mode,
            )
            .await?;
    }
    Ok(())
}

pub async fn info(app: &App, session_id: &str) -> Result<Option<Value>> {
    Ok(match app.store.metadata(&key(session_id)).await? {
        Some(raw) => Some(serde_json::from_str(&raw)?),
        None => None,
    })
}
