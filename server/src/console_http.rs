//! Bounded authenticated receipt reads and static console routing.
use crate::{calls::actor::App, events::Event};
use axum::{
    Extension, Json, Router,
    extract::{Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventQuery {
    after: Option<i64>,
    limit: Option<u16>,
}
/// Must be mounted inside the existing operator-authenticated API router.
pub async fn session_events(
    State(app): State<App>,
    Path(id): Path<String>,
    Query(query): Query<EventQuery>,
) -> Response {
    let after = query.after.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);
    if after < 0 || !(1..=500).contains(&limit) || id.len() > 128 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"after must be nonnegative and limit must be between 1 and 500"})),
        )
            .into_response();
    }
    let read = async {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?)")
            .bind(&id).fetch_one(&app.store.pool).await?;
        if !exists {return Ok::<_,anyhow::Error>(None);}
        let rows: Vec<String> = sqlx::query_scalar("SELECT data FROM events WHERE sequence>? AND json_extract(data,'$.session_id')=? ORDER BY sequence ASC LIMIT ?")
            .bind(after).bind(&id).bind(i64::from(limit)+1).fetch_all(&app.store.pool).await?;
        let has_more = rows.len() > usize::from(limit);
        let events = rows.into_iter().take(usize::from(limit)).map(|row| serde_json::from_str::<Event>(&row)).collect::<Result<Vec<_>,_>>()?;
        let next_after = events.last().map_or(after,|event| event.sequence);
        Ok(Some(json!({"session_id":id,"events":events,"next_after":next_after,"has_more":has_more})))
    }.await;
    match read {
        Ok(Some(page)) => ([(header::CACHE_CONTROL, "no-store")], Json(page)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"Session not found"})),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Session receipts unavailable"})),
        )
            .into_response(),
    }
}

pub fn frontend_service() -> Router {
    frontend_service_at(
        "frontend/dist/firstring-web/browser".into(),
        "web/dist".into(),
    )
}
#[derive(Clone)]
struct LegacyRoot(PathBuf);
fn frontend_service_at(angular: PathBuf, legacy: PathBuf) -> Router {
    let root = if angular.join("index.html").is_file() {
        angular
    } else {
        legacy.clone()
    };
    let index = root.join("index.html");
    let old_console = Router::new()
        .route("/", get(legacy_index))
        .fallback_service(ServeDir::new(&legacy))
        .layer(Extension(LegacyRoot(legacy)));
    Router::new()
        .route_service("/", ServeFile::new(&index))
        .route_service("/console", ServeFile::new(&index))
        .route_service("/console/{*path}", ServeFile::new(&index))
        .nest_service("/legacy-console", old_console)
        // Only the explicit console routes get SPA fallback. Missing APIs,
        // media, scripts and unrelated paths retain their actual HTTP status.
        .fallback_service(ServeDir::new(root).append_index_html_on_directories(false))
        .layer(middleware::from_fn(reserved_paths))
}
async fn legacy_index(Extension(root): Extension<LegacyRoot>) -> Response {
    match tokio::fs::read_to_string(root.0.join("index.html")).await {
        Ok(html) => Html(html.replace("\"/assets/", "\"/legacy-console/assets/")).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn reserved_paths(req: Request, next: Next) -> Response {
    let path = req.uri().path();
    if ["/api", "/voice", "/health"].iter().any(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    }) || path == "/media"
    {
        return (StatusCode::NOT_FOUND, Json(json!({"error":"Not found"}))).into_response();
    }
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|value| value.as_bytes().starts_with(b"text/html"))
    {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    struct Assets(PathBuf);
    impl Assets {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("firstring-console-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(root.join("angular/media")).unwrap();
            std::fs::create_dir_all(root.join("legacy/assets")).unwrap();
            std::fs::write(
                root.join("angular/index.html"),
                "<html>angular entry</html>",
            )
            .unwrap();
            std::fs::write(root.join("angular/main.js"), "console.log('angular')").unwrap();
            std::fs::write(
                root.join("angular/media/ring.webp"),
                b"fictional-image-fixture",
            )
            .unwrap();
            std::fs::write(
                root.join("legacy/index.html"),
                "<html>legacy entry<script src=\"/assets/old.js\"></script></html>",
            )
            .unwrap();
            std::fs::write(root.join("legacy/assets/old.js"), "console.log('legacy')").unwrap();
            Self(root)
        }
        fn router(&self) -> Router {
            frontend_service_at(self.0.join("angular"), self.0.join("legacy"))
        }
    }
    impl Drop for Assets {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    async fn get(router: Router, path: &str) -> (StatusCode, String, Vec<u8>) {
        let response = router
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        (
            status,
            content_type,
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
    }
    #[tokio::test]
    async fn angular_deep_links_refresh_and_legacy_assets_remain_reachable() {
        let assets = Assets::new();
        for path in [
            "/",
            "/console",
            "/console/calls/example",
            "/console/settings",
        ] {
            let (status, kind, body) = get(assets.router(), path).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert!(kind.starts_with("text/html"));
            assert_eq!(body, b"<html>angular entry</html>");
        }
        let (_, _, legacy) = get(assets.router(), "/legacy-console/").await;
        assert!(
            String::from_utf8(legacy)
                .unwrap()
                .contains("/legacy-console/assets/old.js")
        );
        assert_eq!(
            get(assets.router(), "/legacy-console/assets/old.js")
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            get(assets.router(), "/media/ring.webp").await.0,
            StatusCode::OK
        );
    }
    #[tokio::test]
    async fn reserved_missing_routes_and_assets_never_receive_spa_html() {
        let assets = Assets::new();
        for path in [
            "/api",
            "/api/missing",
            "/voice",
            "/voice/missing",
            "/health",
            "/health/missing",
            "/media",
            "/media/missing",
            "/missing.js",
        ] {
            let (status, kind, body) = get(assets.router(), path).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
            assert!(!kind.starts_with("text/html"), "{path}");
            assert!(!String::from_utf8_lossy(&body).contains("entry"));
        }
        let response = assets
            .router()
            .oneshot(
                Request::post("/console/calls/example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
    #[tokio::test]
    async fn absent_angular_build_falls_back_to_existing_console() {
        let assets = Assets::new();
        std::fs::remove_file(assets.0.join("angular/index.html")).unwrap();
        let (status, _, body) = get(assets.router(), "/console/calls/example").await;
        assert_eq!(status, StatusCode::OK);
        assert!(String::from_utf8(body).unwrap().contains("legacy entry"));
        assert_eq!(
            get(assets.router(), "/assets/old.js").await.0,
            StatusCode::OK
        );
    }
}
