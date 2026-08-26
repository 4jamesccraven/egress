use serde::{Deserialize, Serialize};

use crate::telegram::TelegramMessage;

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
        let mut s =
            serde_json::to_string(&self).expect("CommandProtocol can always be serialized as JSON");
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

    /// Gets information about the messages stored in the database.
    /// Note: does _not_ store or list the message text.
    #[serde(rename = "get_messages")]
    GetMessages,

    /// Deletes all managed messages.
    #[serde(rename = "purge")]
    Purge { immediate: bool },

    /// I just like printing the GNU Public License blurb lmao
    #[serde(rename = "license")]
    License,
}

impl From<CommandAction> for CommandProtocol {
    fn from(action: CommandAction) -> Self {
        CommandProtocol {
            protocol_version: PROTOCOL_VER_MAX,
            action,
        }
    }
}

/// JSON protocol that the daemon sends back to clients.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResponseProtocol {
    pub protocol_version: u8,
    pub success: bool,

    #[serde(flatten)]
    pub data: ResponseData,
}

impl ResponseProtocol {
    pub fn to_serialized(self) -> String {
        let mut s = serde_json::to_string(&self)
            .expect("ResponseProtocol can always be serialized as JSON");
        s.push('\n');
        s
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "action")]
pub enum ResponseData {
    NotifyLeft {
        success_count: usize,
        total: usize,
    },
    Status {
        text: String,
    },
    GetMessages {
        messages: Vec<TelegramMessage>,
    },
    Purge {
        /// Number of successfully deleted messages.
        success_count: usize,
        /// Messages that failed to delete and why.
        failure: Vec<PurgeFailure>,
        /// Other errors.
        error: Option<String>,
    },
    License {
        text: String,
    },
    /// For when a protocol that is invalid over HTTP is requested
    HttpNotPermitted,
}

impl ResponseData {
    pub fn to_protocol(self, success: bool) -> ResponseProtocol {
        ResponseProtocol {
            protocol_version: PROTOCOL_VER_MAX,
            success,
            data: self,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurgeFailure {
    pub chat_id: i64,
    pub message_id: i64,
    pub error: String,
}

impl From<(TelegramMessage, String)> for PurgeFailure {
    fn from(value: (TelegramMessage, String)) -> Self {
        Self {
            chat_id: value.0.chat_id,
            message_id: value.0.message_id,
            error: value.1,
        }
    }
}
