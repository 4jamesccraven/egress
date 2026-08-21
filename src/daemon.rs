use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

type DaemonError = ();

pub struct Daemon {
    socket: UnixListener,
}

impl Daemon {
    fn new() -> Result<Self, DaemonError> {
        Ok(Self {
            socket: Self::init_connections()?,
        })
    }

    pub async fn run() -> Result<(), DaemonError> {
        let daemon = Self::new()?;

        loop {
            tokio::select! {
                result = daemon.socket.accept() => {
                    let (stream, _) = result.map_err(|_| ())?;
                    daemon.handle_connection(stream).await?;
                }

                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(&self, stream: UnixStream) -> Result<(), DaemonError> {
        let (reader, mut writer) = stream.into_split();

        let mut reader = BufReader::new(reader);
        let mut request = String::new();

        reader.read_line(&mut request).await.unwrap();

        println!("Received: {request:?}");

        writer
            .write_all(b"Request received.\n")
            .await
            .map_err(|_| ())
    }

    fn init_connections() -> Result<UnixListener, DaemonError> {
        let socket_path = if cfg!(debug_assertions) {
            format!("{}/egress.sock", std::env::var("XDG_RUNTIME_DIR").unwrap())
        } else {
            "/run/egress.sock".to_string()
        };

        if std::path::Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path).unwrap();
        }

        UnixListener::bind(socket_path).map_err(|_| ())
    }
}
