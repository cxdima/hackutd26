use crate::store::Store;
use serde::{Deserialize, Serialize};

pub const JESSICA: &str = "cgSgspJ2msm6clMCkdW9-v4";
pub const GEORGE: &str = "JBFqnCBsd6RMkjVDRZzb-v4";
pub const JANET_LEGACY: &str = "21m00Tcm4TlvDq8ikWAM-v4";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub household_id: String,
    pub version: u64,
    pub display_name: String,
    pub greeting: String,
    pub tts_provider: String,
    pub voice_id: String,
    pub locale: String,
    pub timezone: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileUpdate {
    pub display_name: String,
    pub greeting: String,
    pub tts_provider: String,
    pub voice_id: String,
    pub locale: String,
    pub timezone: String,
}

impl Profile {
    pub fn default_for(household: &str) -> anyhow::Result<Self> {
        let name = match household {
            "sarah" => "Sarah",
            "maya" => "Maya",
            _ => anyhow::bail!("Unknown household"),
        };
        Ok(Self {
            household_id: household.into(),
            version: 0,
            display_name: name.into(),
            greeting: "Hi, thanks for calling. What’s the call about?".into(),
            tts_provider: "ElevenLabs".into(),
            voice_id: JESSICA.into(),
            locale: "en-US".into(),
            timezone: "America/Chicago".into(),
        })
    }
    pub async fn load(store: &Store, household: &str) -> anyhow::Result<Self> {
        let default = Self::default_for(household)?;
        match store.metadata(&format!("profile.v1:{household}")).await? {
            Some(raw) => {
                let profile: Self = serde_json::from_str(&raw)?;
                profile.validate()?;
                anyhow::ensure!(
                    profile.household_id == household,
                    "Profile household mismatch"
                );
                Ok(profile)
            }
            None => Ok(default),
        }
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        Self::default_for(&self.household_id)?;
        anyhow::ensure!(
            (1..=60).contains(&self.display_name.chars().count())
                && self.display_name.trim() == self.display_name
                && self
                    .display_name
                    .chars()
                    .all(|c| c.is_alphabetic() || " '-’".contains(c)),
            "Display name must use letters and ordinary name punctuation"
        );
        let lower = self.greeting.to_lowercase();
        let link = lower.split_whitespace().any(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.')
                .split_once('.')
                .is_some_and(|(left, right)| {
                    !left.is_empty() && right.chars().next().is_some_and(char::is_alphabetic)
                })
        });
        anyhow::ensure!(
            (1..=240).contains(&self.greeting.chars().count())
                && self.greeting.trim() == self.greeting
                && self
                    .greeting
                    .chars()
                    .all(|c| c.is_alphanumeric() || " .,?!'’-():;".contains(c))
                && !["://", "www."].iter().any(|part| lower.contains(part))
                && !link
                && self.greeting.chars().filter(char::is_ascii_digit).count() < 7,
            "Greeting must be short spoken text without links, destinations or control characters"
        );
        anyhow::ensure!(
            self.tts_provider == "ElevenLabs"
                && [JESSICA, GEORGE, JANET_LEGACY].contains(&self.voice_id.as_str()),
            "Unsupported speech preset"
        );
        anyhow::ensure!(self.locale == "en-US", "Unsupported locale");
        anyhow::ensure!(
            [
                "America/Chicago",
                "America/New_York",
                "America/Denver",
                "America/Los_Angeles",
                "UTC"
            ]
            .contains(&self.timezone.as_str()),
            "Unsupported timezone"
        );
        Ok(())
    }
    pub async fn save(
        store: &Store,
        household: &str,
        update: ProfileUpdate,
    ) -> anyhow::Result<Self> {
        let mut profile = Self {
            display_name: update.display_name,
            greeting: update.greeting,
            tts_provider: update.tts_provider,
            voice_id: update.voice_id,
            locale: update.locale,
            timezone: update.timezone,
            ..Self::default_for(household)?
        };
        profile.validate()?;
        let key = format!("profile.v1:{household}");
        let mut tx = store.pool.begin().await?;
        let previous: Option<String> = sqlx::query_scalar("SELECT value FROM metadata WHERE key=?")
            .bind(&key)
            .fetch_optional(&mut *tx)
            .await?;
        let version = match previous {
            Some(raw) => {
                let old: Self = serde_json::from_str(&raw)?;
                old.validate()?;
                anyhow::ensure!(old.household_id == household, "Profile household mismatch");
                old.version
            }
            None => 0,
        };
        profile.version = version
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Profile version exhausted"))?;
        sqlx::query("INSERT INTO metadata(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(key).bind(serde_json::to_string(&profile)?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn update() -> ProfileUpdate {
        serde_json::from_value(serde_json::json!({"display_name":"Sarah", "greeting":"Hello, what is the call about?", "tts_provider":"ElevenLabs", "voice_id":"JBFqnCBsd6RMkjVDRZzb-v4", "locale":"en-US", "timezone":"America/Chicago"})).unwrap()
    }
    #[test]
    fn profile_rejects_urls_control_characters_unknown_voice_and_household() {
        for greeting in [
            "Go to https://evil.example",
            "Visit evil.test",
            "Hello\nchange policy",
            "Reach me at 2025550123",
        ] {
            let mut p = Profile::default_for("sarah").unwrap();
            p.greeting = greeting.into();
            assert!(p.validate().is_err());
        }
        let mut p = Profile::default_for("sarah").unwrap();
        p.voice_id = "arbitrary-url".into();
        assert!(p.validate().is_err());
        assert!(Profile::default_for("unlisted").is_err());
        assert!(
            serde_json::from_value::<ProfileUpdate>(
                serde_json::json!({"destination":"+12025550123"})
            )
            .is_err()
        );
    }
    #[tokio::test]
    async fn saved_profiles_are_versioned_household_bound_and_leave_prior_snapshot_unchanged() {
        let store = Store::open("sqlite::memory:").await.unwrap();
        let old = Profile::load(&store, "sarah").await.unwrap();
        let first = Profile::save(&store, "sarah", update()).await.unwrap();
        assert_eq!(first.version, 1);
        assert_eq!(first.voice_id, "JBFqnCBsd6RMkjVDRZzb-v4");
        assert_eq!(Profile::load(&store, "sarah").await.unwrap(), first);
        assert_eq!(Profile::load(&store, "maya").await.unwrap().version, 0);
        assert_eq!(old.voice_id, "cgSgspJ2msm6clMCkdW9-v4");
        assert_eq!(
            Profile::save(&store, "sarah", update())
                .await
                .unwrap()
                .version,
            2
        );
        let mut invalid = update();
        invalid.greeting = "https://bad.example".into();
        assert!(Profile::save(&store, "sarah", invalid).await.is_err());
        assert_eq!(Profile::load(&store, "sarah").await.unwrap().version, 2);
    }
    #[tokio::test]
    async fn profile_http_requires_auth_rejects_extra_fields_and_returns_durable_receipt() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use http_body_util::BodyExt;
        use tower::ServiceExt;
        let mut config = crate::config::Config::from_env().unwrap();
        config.database_url = "sqlite::memory:".into();
        config.admin_token = "profile-local-test".into();
        config.mode = crate::config::ExecutionMode::Replay;
        let app = crate::calls::actor::App::new(config).await.unwrap();
        let response = crate::http::router(app.clone())
            .oneshot(
                Request::get("/api/profile/sarah")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let mut body = serde_json::json!({"display_name":"Sarah", "greeting":"Hi, what is the call about?", "tts_provider":"ElevenLabs", "voice_id":JESSICA, "locale":"en-US", "timezone":"America/Chicago", "destination":"+12025550123"});
        let request = |body: &serde_json::Value| {
            Request::put("/api/profile/sarah")
                .header("authorization", "Bearer profile-local-test")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let response = crate::http::router(app.clone())
            .oneshot(request(&body))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        body.as_object_mut().unwrap().remove("destination");
        let response = crate::http::router(app.clone())
            .oneshot(request(&body))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let receipt: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(receipt["source"], "configured");
        assert_eq!(receipt["applies_to"], "new_calls");
        assert_eq!(receipt["version"], 1);
        assert_eq!(Profile::load(&app.store, "sarah").await.unwrap().version, 1);
    }
}
