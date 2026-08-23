use crate::error::ExpectExt;

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VER_MAX: u8 = 1;

/// Describes JSON protocol that is passed via sockets and http connections.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandProtocol {
    pub protocol_version: u8,

    #[serde(flatten)]
    pub action: CommandAction,
}

impl CommandProtocol {
    pub fn to_serialized(self) -> String {
        let mut s = serde_json::to_string(&self).responsible_expect("should be infallible");
        s.push('\n');
        s
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "action")]
pub enum CommandAction {
    /// Have Egress send departure info.
    #[serde(rename = "notify_left")]
    NotifyLeft { source_id: Option<String> },

    /// Query egressd's status.
    #[serde(rename = "status")]
    Status,

    /// I just like printing the GNU Public License blurb lmao
    #[serde(rename = "license")]
    License,
}

impl CommandAction {
    pub fn to_protocol(&self) -> CommandProtocol {
        CommandProtocol {
            protocol_version: PROTOCOL_VER_MAX,
            action: self.to_owned(),
        }
    }
}

/// JSON protocol that the daemon sends back to clients.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResponseProtocol {
    pub protocol_version: u8,
    pub success: bool,
    pub text: String,
}

impl ResponseProtocol {
    pub fn to_serialized(self) -> String {
        let mut s = serde_json::to_string(&self).responsible_expect("should be infallible");
        s.push('\n');
        s
    }
}
