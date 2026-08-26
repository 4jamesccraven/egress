use crate::error::ConfigError;

use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Who's leaving. Defaults to 'user'.
    #[serde(default = "default_user")]
    pub user_name: String,
    /// A list of Telegram `chat_id`s to receive updates.
    pub targets: Vec<i64>,
    /// The bot token for use with Telegram's API.
    pub telegram_token: String,
    /// How many hours a message should persist
    #[serde(default = "default_expiry")]
    pub expiry_hours: i64,
}

impl Config {
    /// Gets the configuration, panicking if it cannot be loaded.
    ///
    /// Use [`Config::load_config`] when configuration errors need to be handled.
    pub fn get() -> &'static Self {
        Self::load_config().unwrap_or_else(|error| {
            panic!(
                "fatal: Config::get() called before configuration was successfully loaded: {error}"
            );
        })
    }

    /// Loads the user configuration from disk.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::NotFound`] if no configuration file exists,
    /// [`ConfigError::TooMany`] if both TOML and JSON files exist, or
    /// relevant IO/serde errors if the file cannot be read or parsed.
    pub fn load_config() -> Result<&'static Self, ConfigError> {
        if let Some(cfg) = CONFIG.get() {
            return Ok(cfg);
        }

        let cfg_dir = match std::env::var("EGRESS_CONFIG_DIR") {
            Ok(path) => PathBuf::from(path),
            Err(_) => Self::config_dir(),
        };

        let default_path = cfg_dir.join("config.toml");
        let alt_path = default_path.with_extension("json");

        let mut use_toml = true;
        let config_contents = match (default_path.exists(), alt_path.exists()) {
            (true, false) => read_to_string(default_path)?,
            (false, true) => {
                use_toml = false;
                read_to_string(alt_path)?
            }
            (true, true) => return Err(ConfigError::TooMany),
            (false, false) => return Err(ConfigError::NotFound),
        };

        let cfg = if use_toml {
            toml::from_str(&config_contents).map_err(ConfigError::from)
        } else {
            serde_json::from_str(&config_contents).map_err(ConfigError::from)
        }?;

        CONFIG.set(cfg).expect("could not already have been set");
        Ok(CONFIG.get().expect("we just set it"))
    }

    fn config_dir() -> PathBuf {
        if cfg!(debug_assertions) {
            dirs::config_dir()
                .expect("XDG_CONFIG_HOME is not set")
                .join("egress")
        } else {
            PathBuf::from("/etc/egress")
        }
    }
}

const fn default_expiry() -> i64 {
    12
}

fn default_user() -> String {
    "user".into()
}
