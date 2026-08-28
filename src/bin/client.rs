use egress::cli::ClientCLI;
use egress::client::Client;

use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;

#[tokio::main]
async fn main() {
    CompleteEnv::with_factory(ClientCLI::command).complete();

    let args = ClientCLI::parse();

    match Client::run(&args).await {
        Ok(response_ok) => {
            if !response_ok {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
