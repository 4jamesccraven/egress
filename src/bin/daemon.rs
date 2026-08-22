use egress::daemon::Daemon;

#[tokio::main]
async fn main() {
    Daemon::run().await.unwrap()
}
