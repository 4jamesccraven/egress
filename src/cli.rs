use crate::protocol::{CommandAction, CommandProtocol};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "egressctl",
    long_version = "TODO",
    disable_help_subcommand = true
)]
pub struct ClientCLI {
    #[command(subcommand)]
    pub command: ClientCommands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ClientCommands {
    Status,
    Notify,
    Purge {
        #[arg(long, short)]
        immediate: bool,
    },
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
