use crate::error::{ConfigError, ExpectExt};

use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::OnceLock;

use dirs::config_dir;
use serde::{Deserialize, Serialize};

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// A list of Telegram `chat_id`s to receive updates.
    pub targets: Vec<u64>,
    /// The bot token for use with Telegram's API.
    pub telegram_token: String,
}

impl Config {
    pub fn get() -> Self {
        CONFIG
            .get_or_init(|| match Self::load_config() {
                Ok(cfg) => cfg,
                Err(error) => {
                    eprintln!("fatal: Config::get() called before configuration was successfully loaded: {error}");
                    std::process::exit(1);
                }
            })
            .clone()
    }

    pub fn load_config() -> Result<Self, ConfigError> {
        let cfg_dir = match std::env::var("EGRESS_CONFIG_DIR") {
            Ok(path) => PathBuf::from(path),
            Err(_) => config_dir()
                .responsible_expect("XDG_CONFIG_HOME is not set")
                .join("egress"),
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

        if use_toml {
            toml::from_str(&config_contents).map_err(ConfigError::from)
        } else {
            serde_json::from_str(&config_contents).map_err(ConfigError::from)
        }
    }
}
