use egress::daemon::Daemon;

#[tokio::main]
async fn main() {
    if let Err(error) = Daemon::run().await {
        eprintln!("fatal: unable to start egressd: {error}");
        std::process::exit(1);
    }
}
