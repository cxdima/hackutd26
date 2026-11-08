//! The console on this computer needs no token; tunnelled requests still do.
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use firstring::{
    calls::actor::App,
    config::{Config, ExecutionMode},
    http::router,
};
use std::net::SocketAddr;
use tower::ServiceExt;

fn config() -> Config {
    let mut c = Config::from_env().unwrap();
    c.database_url = "sqlite::memory:".into();
    c.public_base_url = "https://voice.example.com".into();
    c.account_sid = format!("AC{}", "0".repeat(32));
    c.auth_token = "test-auth-token".into();
    c.admin_token = "a-long-local-test-admin-token".into();
    c.mode = ExecutionMode::Replay;
    c.transport = "probe".into();
    c.photon_number.clear();
    c.contacts.clear();
    c
}

async fn status(app: &App, peer: &str, headers: &[(&str, &str)]) -> StatusCode {
    let mut req = Request::get("/api/state");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let mut req = req.body(Body::empty()).unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    router(app.clone()).oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn loopback_console_needs_no_token_but_tunnelled_requests_do() {
    let app = App::new(config()).await.unwrap();
    assert_eq!(status(&app, "127.0.0.1:50000", &[]).await, StatusCode::OK);
    assert_eq!(status(&app, "[::1]:50000", &[]).await, StatusCode::OK);
    // cloudflared connects from loopback but always adds forwarding headers.
    for header in ["cf-connecting-ip", "cf-ray", "x-forwarded-for"] {
        assert_eq!(
            status(&app, "127.0.0.1:50000", &[(header, "203.0.113.9")]).await,
            StatusCode::UNAUTHORIZED,
            "{header}"
        );
    }
    assert_eq!(
        status(&app, "192.168.1.20:50000", &[]).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status(
            &app,
            "203.0.113.9:443",
            &[("authorization", "Bearer a-long-local-test-admin-token")]
        )
        .await,
        StatusCode::OK
    );
}

#[test]
fn edge_viewer_reads_only_public_paths() {
    use axum::http::Method;
    use firstring::http::edge_readable;
    for path in [
        "/api/state",
        "/api/events",
        "/api/callers",
        "/api/reputation",
        "/api/reputation/88f51446ef92d0b4",
        "/api/session/7e3cea7b-9f00-40df-be94-75e9fb9e9c37",
        "/api/session/7e3cea7b-9f00-40df-be94-75e9fb9e9c37/events",
        "/api/session/7e3cea7b-9f00-40df-be94-75e9fb9e9c37/reputation",
        "/api/demo/phone-demo",
        "/api/profile/sarah",
    ] {
        assert!(edge_readable(&Method::GET, path), "{path}");
        assert!(
            !edge_readable(&Method::POST, path),
            "writes are never edge-readable: {path}"
        );
    }
    for path in [
        "/api/session/abc/recording",
        "/api/session/abc/recording/info",
        "/api/voice/clone",
        "/api/voice/clone/preview",
        "/api/trusted",
        "/api/demo/phone-demo/arm",
        "/api/demo/reset",
        "/api/session/../state",
        "/api/reputation/a/corrections",
    ] {
        assert!(!edge_readable(&Method::GET, path), "{path}");
    }
}

#[test]
fn edge_viewer_requires_the_exact_long_secret() {
    use axum::{body::Body, http::Request};
    use firstring::http::edge_viewer;
    let secret = "fixture-edge-secret-0123456789";
    let with = |value: &str| {
        Request::get("/api/state")
            .header("x-firstring-edge", value)
            .body(Body::empty())
            .unwrap()
    };
    assert!(edge_viewer(&with(secret), secret));
    assert!(!edge_viewer(&with("wrong-secret-0123456789abcdef"), secret));
    assert!(!edge_viewer(&with(secret), "short"));
    let post = Request::post("/api/state")
        .header("x-firstring-edge", secret)
        .body(Body::empty())
        .unwrap();
    assert!(!edge_viewer(&post, secret));
    let none = Request::get("/api/state").body(Body::empty()).unwrap();
    assert!(!edge_viewer(&none, secret));
}
