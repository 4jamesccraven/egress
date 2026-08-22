use crate::daemon::DaemonError;

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
            .get_or_init(|| Self::from_xdg_config().expect("CONFIG ERROR"))
            .clone()
    }

    fn from_xdg_config() -> Result<Self, DaemonError> {
        let cfg_dir = std::env::var("EGRESS_CONFIG_DIR")
            .map(PathBuf::from)
            .map_err(|_| ())
            .or(config_dir().map(|p| p.join("egress")).ok_or(()))?;

        let default_path = cfg_dir.join("config.toml");
        let alt_path = default_path.with_extension("json");

        let mut use_toml = true;
        let config_contents = match (default_path.exists(), alt_path.exists()) {
            (true, false) => read_to_string(default_path).map_err(|_| ())?,
            (false, true) => {
                use_toml = false;
                read_to_string(alt_path).map_err(|_| ())?
            }
            _ => return Err(()),
        };

        if use_toml {
            toml::from_str(&config_contents).map_err(|_| ())
        } else {
            serde_json::from_str(&config_contents).map_err(|_| ())
        }
    }
}
