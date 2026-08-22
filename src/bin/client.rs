use egress::cli::ClientCLI;
use egress::client::Client;

use clap::Parser;

#[tokio::main]
async fn main() {
    let args = ClientCLI::parse();
    Client::run(&args).await.unwrap();
}
