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
}

impl Into<CommandProtocol> for ClientCommands {
    fn into(self) -> CommandProtocol {
        match self {
            ClientCommands::Status => CommandAction::Status.to_protocol(),
            ClientCommands::Notify => CommandAction::NotifyLeft { source_id: None }.to_protocol(),
        }
    }
}
