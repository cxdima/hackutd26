//! Operator receipt pagination uses the real auth/router and an isolated in-memory store.
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use firstring::{
    calls::{actor::App, state::Session},
    config::{Config, ExecutionMode},
    http::router,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
const TOKEN: &str = "fixture-console-operator-token";
async fn app() -> App {
    let mut config = Config::from_env().unwrap();
    config.database_url = "sqlite::memory:".into();
    config.mode = ExecutionMode::Replay;
    config.admin_token = TOKEN.into();
    config.nvidia_key.clear();
    config.smoke_authorized = false;
    config.handoff_enabled = false;
    config.family_enabled = false;
    App::new(config).await.unwrap()
}
async fn request(app: &App, path: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::get(path);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router(app.clone())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}
async fn session(app: &App, id: &str) -> Session {
    let s = Session::new(
        id.into(),
        format!("fixture-{id}"),
        "sarah".into(),
        ExecutionMode::Replay,
    );
    app.store.save(&s).await.unwrap();
    s
}
#[tokio::test]
async fn session_receipts_require_auth_and_keep_voice_routes_signed() {
    let app = app().await;
    let s = session(&app, "known").await;
    app.store
        .append(
            Some(&s),
            "system",
            "private.saved",
            json!({"private":"fixture"}),
            ExecutionMode::Replay,
        )
        .await
        .unwrap();
    for token in [None, Some("wrong")] {
        let (status, body) = request(&app, "/api/session/known/events", token).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.get("events").is_none());
    }
    assert_eq!(
        request(&app, "/api/session/known/events", Some(TOKEN))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(request(&app, "/health/live", None).await.0, StatusCode::OK);
    let response = router(app)
        .oneshot(
            Request::post("/voice/incoming")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
#[tokio::test]
async fn session_pages_resume_in_global_sequence_order_without_other_sessions() {
    let app = app().await;
    let (a, b) = (session(&app, "first").await, session(&app, "second").await);
    let mut expected = Vec::new();
    for index in 0..5 {
        let event = app
            .store
            .append(
                Some(&a),
                "system",
                "fixture.receipt",
                json!({"index":index}),
                ExecutionMode::Replay,
            )
            .await
            .unwrap();
        expected.push(event.sequence);
        app.store
            .append(
                Some(&b),
                "system",
                "other.receipt",
                json!({"private":"other"}),
                ExecutionMode::Replay,
            )
            .await
            .unwrap();
        app.store
            .append(
                None,
                "system",
                "global.receipt",
                json!({}),
                ExecutionMode::Replay,
            )
            .await
            .unwrap();
    }
    let mut after = 0;
    let mut actual = Vec::new();
    for (page_index, size) in [2, 2, 1].into_iter().enumerate() {
        let (status, page) = request(
            &app,
            &format!("/api/session/first/events?after={after}&limit=2"),
            Some(TOKEN),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["session_id"], "first");
        assert_eq!(page["events"].as_array().unwrap().len(), size);
        assert_eq!(page["has_more"], page_index < 2);
        for event in page["events"].as_array().unwrap() {
            assert_eq!(event["session_id"], "first");
            actual.push(event["sequence"].as_i64().unwrap());
        }
        let next = page["next_after"].as_i64().unwrap();
        assert!(next > after);
        after = next;
    }
    assert_eq!(actual, expected);
    let (_, empty) = request(
        &app,
        &format!("/api/session/first/events?after={after}&limit=2"),
        Some(TOKEN),
    )
    .await;
    assert_eq!(empty["next_after"], after);
    assert_eq!(empty["events"], json!([]));
    assert_eq!(empty["has_more"], false);
    let event = app
        .store
        .append(
            Some(&a),
            "system",
            "new.receipt",
            json!({}),
            ExecutionMode::Replay,
        )
        .await
        .unwrap();
    let (_, resumed) = request(
        &app,
        &format!("/api/session/first/events?after={after}&limit=2"),
        Some(TOKEN),
    )
    .await;
    assert_eq!(resumed["events"][0]["sequence"], event.sequence);
}
#[tokio::test]
async fn receipt_bounds_unknown_sessions_and_defaults_are_explicit() {
    let app = app().await;
    let s = session(&app, "bounds").await;
    for index in 0..501 {
        app.store
            .append(
                Some(&s),
                "system",
                "fixture",
                json!({"index":index}),
                ExecutionMode::Replay,
            )
            .await
            .unwrap();
    }
    for query in [
        "after=-1",
        "after=no",
        "limit=0",
        "limit=501",
        "limit=65536",
        "before=1",
    ] {
        assert_eq!(
            request(
                &app,
                &format!("/api/session/bounds/events?{query}"),
                Some(TOKEN)
            )
            .await
            .0,
            StatusCode::BAD_REQUEST,
            "{query}"
        );
    }
    let (_, default) = request(&app, "/api/session/bounds/events", Some(TOKEN)).await;
    assert_eq!(default["events"].as_array().unwrap().len(), 100);
    assert_eq!(default["has_more"], true);
    let (_, maximum) = request(&app, "/api/session/bounds/events?limit=500", Some(TOKEN)).await;
    assert_eq!(maximum["events"].as_array().unwrap().len(), 500);
    assert_eq!(maximum["has_more"], true);
    assert_eq!(
        request(&app, "/api/session/unknown/events", Some(TOKEN))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    sqlx::query("DROP TABLE events")
        .execute(&app.store.pool)
        .await
        .unwrap();
    let (status, body) = request(&app, "/api/session/bounds/events", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, json!({"error":"Session receipts unavailable"}));
}
