//! The actual launcher must enable configured reputation, not only injected test Apps.
use std::process::{Child, Command, Stdio};
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[tokio::test]
async fn live_server_startup_loads_explicit_reputation_configuration() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_firstring"))
            .env("PORT", port.to_string())
            .env("DATABASE_URL", "sqlite::memory:")
            .env("PUBLIC_BASE_URL", "https://fixture.example")
            .env("DEMO_EXECUTION_MODE", "live")
            .env("VOICE_TRANSPORT", "probe")
            .env("DEMO_ADMIN_TOKEN", "test-startup-operator-token-long")
            .env(
                "REPUTATION_HMAC_KEY",
                "test-startup-stable-reputation-key-32bytes",
            )
            .env("REPUTATION_SCOPE", "demo")
            .env("NEMOTRON_BACKEND", "nvidia_hosted")
            .env("NVIDIA_API_KEY", "")
            .env("TWILIO_AUTH_TOKEN", "")
            .env("TWILIO_ACCOUNT_SID", "")
            .env("PHOTON_SIP_PASSWORD", "")
            .env("ELEVENLABS_API_KEY", "")
            .env("HANDOFF_CALLS_ENABLED", "false")
            .env("FAMILY_CALLS_ENABLED", "false")
            .env("TEST_CALLS_AUTHORIZED", "false")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/reputation");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let response = loop {
        assert!(std::time::Instant::now() < deadline, "Startup deadline");
        if let Ok(response) = client
            .get(&url)
            .bearer_auth("test-startup-operator-token-long")
            .send()
            .await
        {
            break response;
        }
        assert!(
            server.0.try_wait().unwrap().is_none(),
            "Server exited before readiness"
        );
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    };
    assert_eq!(response.status(), 200);
    let value: serde_json::Value = response.json().await.unwrap();
    assert_eq!(value["scope"], "demo");
    assert_eq!(value["items"], serde_json::json!([]));
}
