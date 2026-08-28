use crate::protocol::{CommandAction, CommandProtocol};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "egressctl",
    version = "0.1.0",
    about = "command the egressd, the surveillance daemon",
    disable_help_subcommand = true
)]
pub struct ClientCLI {
    #[command(subcommand)]
    pub command: ClientCommands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ClientCommands {
    /// Get the status of egressd
    Status,
    /// Have egressd notify its targets
    Notify,
    /// Remove expired messages
    Purge {
        /// Remove the message regardless of whether it's expired or not.
        #[arg(long, short)]
        immediate: bool,
    },
    /// View stored messages.
    Messages,
}

impl From<ClientCommands> for CommandProtocol {
    fn from(val: ClientCommands) -> Self {
        let action = match val {
            ClientCommands::Status => CommandAction::Status,
            ClientCommands::Notify => CommandAction::NotifyLeft { source_id: None },
            ClientCommands::Purge { immediate } => CommandAction::Purge { immediate },
            ClientCommands::Messages => CommandAction::GetMessages,
        };

        action.into()
    }
}
