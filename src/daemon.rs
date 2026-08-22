use crate::config::Config;
use crate::error::ResultExt;
use crate::protocol::{CommandAction, CommandProtocol, PROTOCOL_VER_MAX, ResponseProtocol};
use crate::telegram;

use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub type DaemonError = ();

pub struct Daemon {
    config: Config,
    socket: UnixListener,
}

impl Daemon {
    /// Initialises a new daemon. Not called externally; use `Daemon::run` instead.
    fn new() -> Result<Self, DaemonError> {
        Ok(Self {
            config: Config::get(),
            socket: Self::init_connections()?,
        })
    }

    /// Initialises the socket and http server.
    fn init_connections() -> Result<UnixListener, DaemonError> {
        let socket_path = Self::socket_path();

        if std::path::Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path).unwrap();
        }

        UnixListener::bind(socket_path).map_err(|_| ())
    }

    /// Runs the Daemon. Listens for socket and http connections.
    pub async fn run() -> Result<(), DaemonError> {
        let daemon = Self::new()?;

        loop {
            tokio::select! {
                result = daemon.socket.accept() => {
                    let (stream, _) = result.map_err(|_| ())?;
                    daemon.handle_sock(stream).await?;
                }

                _ = tokio::signal::ctrl_c() => {
                    std::fs::remove_file(Self::socket_path()).unwrap();
                    break;
                }
            }
        }

        Ok(())
    }

    /// Accepts connection from the daemon's UNIX socket.
    async fn handle_sock(&self, stream: UnixStream) -> Result<(), DaemonError> {
        let (reader, mut writer) = stream.into_split();

        let mut reader = BufReader::new(reader);
        let mut request = String::new();

        reader.read_line(&mut request).await.unwrap();
        let response = self.try_process_command(&request).await?;

        let mut to_client =
            serde_json::to_string(&response).responsible_expect("failed to deserialise response");
        to_client.push_str("\n");

        writer.write_all(to_client.as_bytes()).await.map_err(|_| ())
    }

    /// Accepts unvalidated protocol data from a client and validates it before dispatching relevant
    /// subroutines.
    async fn try_process_command(
        &self,
        raw_protocol: &str,
    ) -> Result<ResponseProtocol, DaemonError> {
        let request: CommandProtocol = serde_json::from_str(raw_protocol).map_err(|_| ())?;

        match request.action {
            CommandAction::Status => Ok(ResponseProtocol {
                protocol_version: PROTOCOL_VER_MAX,
                success: true,
                text: "egressd is running.".into(),
            }),
            CommandAction::NotifyLeft { source_id: _ } => {
                self.notify_targets("The user left.").await;
                Ok(ResponseProtocol {
                    protocol_version: PROTOCOL_VER_MAX,
                    success: true,
                    text: "message sent (I think).".to_string(),
                })
            }
            a => {
                eprintln!("Action \"{a:?}\" is not yet implemented");
                Err(())
            }
        }
    }

    /// Sends a message to all Telegram chats in the user's config.
    async fn notify_targets(&self, text: &str) {
        for chat in &self.config.targets {
            telegram::send_message(*chat, text).await;
        }
    }

    /// Gets the path to the UNIX socket for the daemon.
    pub fn socket_path() -> PathBuf {
        if cfg!(debug_assertions) {
            dirs::runtime_dir().map(|p| p.join("egress.sock")).unwrap()
        } else {
            PathBuf::from("/run/egress.sock")
        }
    }
}
