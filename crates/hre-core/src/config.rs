use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::Path};

/// Engine configuration. Secrets intentionally do not implement Debug.
#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct EngineConfig {
    pub bind: SocketAddr,
    pub database_url: String,
    pub models_dir: String,
    pub salt: String,
    pub api_keys: Vec<String>,
    pub moderator_tokens: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub demo_mode: bool,
    pub retain_media: bool,
    pub night_timezone: String,
    pub night_start_hour: u8,
    pub night_end_hour: u8,
    pub intra_threads: usize,
}
impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], 8080)),
            database_url: "sqlite://target/hre.db?mode=rwc".into(),
            models_dir: "models".into(),
            salt: String::new(),
            api_keys: vec![],
            moderator_tokens: vec![],
            allowed_origins: vec![],
            demo_mode: false,
            retain_media: false,
            night_timezone: "Asia/Kolkata".into(),
            night_start_hour: 22,
            night_end_hour: 5,
            intra_threads: 2,
        }
    }
}
impl EngineConfig {
    /// Loads an optional TOML file, then applies HRE_* environment overrides.
    pub fn load(path: &Path) -> Result<Self, CoreError> {
        let mut config = if path.exists() {
            let text =
                std::fs::read_to_string(path).map_err(|e| CoreError::Config(e.to_string()))?;
            toml::from_str(&text).map_err(|e| CoreError::Config(e.to_string()))?
        } else {
            Self::default()
        };
        if let Ok(value) = env::var("HRE_BIND") {
            config.bind = value
                .parse()
                .map_err(|_| CoreError::Config("HRE_BIND is invalid".into()))?;
        }
        for (key, slot) in [
            ("HRE_DATABASE_URL", &mut config.database_url),
            ("HRE_MODELS_DIR", &mut config.models_dir),
            ("HRE_SALT", &mut config.salt),
        ] {
            if let Ok(value) = env::var(key) {
                *slot = value;
            }
        }
        for (key, slot) in [
            ("HRE_API_KEYS", &mut config.api_keys),
            ("HRE_MODERATOR_TOKENS", &mut config.moderator_tokens),
            ("HRE_ALLOWED_ORIGINS", &mut config.allowed_origins),
        ] {
            if let Ok(value) = env::var(key) {
                *slot = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
        }
        if let Ok(value) = env::var("HRE_DEMO_MODE") {
            config.demo_mode = value
                .parse()
                .map_err(|_| CoreError::Config("HRE_DEMO_MODE must be true or false".into()))?;
        }
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.salt.len() < 32
            || self.api_keys.is_empty()
            || self.moderator_tokens.is_empty()
            || self
                .api_keys
                .iter()
                .chain(&self.moderator_tokens)
                .any(|k| k.trim().is_empty())
        {
            return Err(CoreError::Config(
                "set HRE_SALT (32+ bytes), HRE_API_KEYS and HRE_MODERATOR_TOKENS".into(),
            ));
        }
        if self.night_start_hour > 23 || self.night_end_hour > 23 || self.intra_threads == 0 {
            return Err(CoreError::Config(
                "invalid night hours or intra_threads".into(),
            ));
        }
        Ok(())
    }
}
