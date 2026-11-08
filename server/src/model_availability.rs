//! Current deployment availability, separate from historical phone/protocol receipts.
use crate::{config::Config, store::Store};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
#[derive(Clone)]
pub struct ModelAvailability {
    deployment: String,
    until: Option<DateTime<Utc>>,
    cache: Arc<Mutex<Option<(Instant, Value)>>>,
}
impl ModelAvailability {
    pub fn new(deployment: String, until: Option<DateTime<Utc>>) -> Self {
        Self {
            deployment,
            until,
            cache: Arc::new(Mutex::new(None)),
        }
    }
    pub fn from_env() -> Self {
        Self::new(
            std::env::var("NEMOTRON_DEPLOYMENT_ID").unwrap_or_default(),
            std::env::var("NEMOTRON_AVAILABLE_UNTIL")
                .ok()
                .and_then(|s| s.parse().ok()),
        )
    }
    pub async fn view(&self, config: &Config, store: &Store) -> Value {
        let now = Utc::now();
        let expired = self.until.is_some_and(|until| until <= now);
        let mut cache = self.cache.lock().await;
        if !expired
            && let Some((checked, value)) = cache.as_ref()
            && checked.elapsed() < Duration::from_secs(10)
        {
            return value.clone();
        }
        let proof: Value = store
            .metadata("verification.current_model")
            .await
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or(Value::Null);
        let protocol = !self.deployment.is_empty()
            && proof["deployment_id"] == self.deployment
            && proof["backend"] == json!(config.model_backend)
            && proof["model"] == config.model
            && proof["endpoint_sha256"] == crate::config::endpoint_fingerprint(&config.model_url)
            && proof["status"] == "passed"
            && proof["native_cycle"] == true
            && proof["recorded_at"]
                .as_str()
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                .is_some_and(|at| at <= now && now - at < chrono::Duration::hours(8));
        let mut reachable = false;
        let mut reason = if expired {
            "deployment_deadline_passed"
        } else if !config.model_configured() {
            "model_not_configured"
        } else if !protocol {
            "current_native_cycle_unverified"
        } else {
            "model_endpoint_unavailable"
        };
        if !expired && config.model_configured() && protocol {
            if let Ok(mut url) = url::Url::parse(&config.model_url) {
                url.set_path("/v1/models");
                url.set_query(None);
                if let Ok(client) = reqwest::Client::builder()
                    .timeout(Duration::from_millis(800))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    && let Ok(response) =
                        client.get(url).bearer_auth(&config.nvidia_key).send().await
                    && response.status().is_success()
                    && response.content_length().unwrap_or(0) <= 262144
                    && let Ok(bytes) = response.bytes().await
                    && bytes.len() <= 262144
                    && let Ok(body) = serde_json::from_slice::<Value>(&bytes)
                {
                    reachable = body["data"]
                        .as_array()
                        .is_some_and(|models| models.iter().any(|m| m["id"] == config.model));
                }
            }
            if reachable {
                reason = "current_model_loaded_and_native_cycle_verified";
            }
        }
        let value = json!({"available":protocol&&reachable&&!expired,"protocol_current":protocol,"endpoint_reachable":reachable,"deployment_id":self.deployment,"checked_at":now,"available_until":self.until,"reason":reason,"native_cycle_recorded_at":proof["recorded_at"],"native_cycle_receipt":proof["receipt"],"freshness_seconds":10,"meaning":"Authenticated model inventory plus a native cycle from this deployment; not a latency or conversational-quality guarantee"});
        *cache = Some((Instant::now(), value.clone()));
        value
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutionMode, ModelBackend};
    #[tokio::test]
    async fn historical_proof_cannot_verify_new_deployment_or_offline_model() {
        let store = Store::open("sqlite::memory:").await.unwrap();
        let mut config = Config::from_env().unwrap();
        config.mode = ExecutionMode::Live;
        config.model_backend = ModelBackend::BrevHosted;
        config.nvidia_key = "fixture".into();
        config.model_url = "http://127.0.0.1:1/v1/chat/completions".into();
        let monitor = ModelAvailability::new(
            "new-run".into(),
            Some(Utc::now() + chrono::Duration::hours(1)),
        );
        let result = monitor.view(&config, &store).await;
        assert_eq!(result["available"], false);
        assert_eq!(result["protocol_current"], false);
        assert_eq!(result["deployment_id"], "new-run");
    }
    #[tokio::test]
    async fn only_matching_fresh_cycle_and_authenticated_inventory_make_available() {
        use axum::{Json, Router, routing::get};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    "/v1/models",
                    get(|headers: axum::http::HeaderMap| async move {
                        assert_eq!(headers["authorization"], "Bearer fixture-key");
                        Json(json!({"data":[{"id":"fixture-model"}]}))
                    }),
                ),
            )
            .await
            .unwrap();
        });
        let store = Store::open("sqlite::memory:").await.unwrap();
        let mut config = Config::from_env().unwrap();
        config.mode = ExecutionMode::Live;
        config.model_backend = ModelBackend::BrevHosted;
        config.model = "fixture-model".into();
        config.nvidia_key = "fixture-key".into();
        config.model_url = format!("http://{address}/v1/chat/completions");
        let proof = json!({"deployment_id":"this-run","backend":"brev_hosted","model":"fixture-model","endpoint_sha256":crate::config::endpoint_fingerprint(&config.model_url),"status":"passed","native_cycle":true,"recorded_at":Utc::now(),"receipt":"fixture-receipt.json"});
        store
            .set_metadata("verification.current_model", &proof.to_string())
            .await
            .unwrap();
        let monitor = ModelAvailability::new(
            "this-run".into(),
            Some(Utc::now() + chrono::Duration::hours(1)),
        );
        assert_eq!(monitor.view(&config, &store).await["available"], true);
        let changed = ModelAvailability::new(
            "different-run".into(),
            Some(Utc::now() + chrono::Duration::hours(1)),
        );
        assert_eq!(changed.view(&config, &store).await["available"], false);
        server.abort();
        let offline = ModelAvailability::new(
            "this-run".into(),
            Some(Utc::now() + chrono::Duration::hours(1)),
        );
        assert_eq!(offline.view(&config, &store).await["available"], false);
    }
    #[tokio::test]
    async fn expired_deadline_is_offline_even_with_configuration() {
        let store = Store::open("sqlite::memory:").await.unwrap();
        let config = Config::from_env().unwrap();
        let monitor = ModelAvailability::new(
            "expired".into(),
            Some(Utc::now() - chrono::Duration::seconds(1)),
        );
        let result = monitor.view(&config, &store).await;
        assert_eq!(result["available"], false);
        assert_eq!(result["reason"], "deployment_deadline_passed");
    }
}
