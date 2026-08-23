use egress::cli::ClientCLI;
use egress::client::Client;

use clap::Parser;

#[tokio::main]
async fn main() {
    let args = ClientCLI::parse();
    if let Err(error) = Client::run(&args).await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
