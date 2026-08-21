use egress::daemon::Daemon;

#[tokio::main]
async fn main() {
    println!("initialising daemon…");
    Daemon::run().await.unwrap()
}
