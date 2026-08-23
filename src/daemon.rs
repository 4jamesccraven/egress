use crate::config::Config;
use crate::error::{DaemonError, ExpectExt};
use crate::protocol::{CommandAction, CommandProtocol, PROTOCOL_VER_MAX, ResponseProtocol};
use crate::telegram;

use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub struct Daemon {
    config: Config,
    socket: UnixListener,
}

impl Daemon {
    /// Initialises a new daemon. Not called externally; use `Daemon::run` instead.
    fn new() -> Result<Self, DaemonError> {
        Ok(Self {
            config: Config::load_config()?,
            socket: Self::init_connections()?,
        })
    }

    /// Initialises the socket and http server.
    fn init_connections() -> Result<UnixListener, DaemonError> {
        let socket_path = Self::socket_path();

        if std::path::Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path).responsible_expect("unable to remove stale socket");
        }

        UnixListener::bind(socket_path).map_err(|_| DaemonError::ConnectionFailed)
    }

    /// Runs the Daemon. Listens for socket and http connections.
    pub async fn run() -> Result<(), DaemonError> {
        let daemon = Self::new()?;
        eprintln!("egressd is running");

        loop {
            tokio::select! {
                result = daemon.socket.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            daemon.handle_sock(stream).await?;
                        }
                        Err(error) => {
                            eprintln!("Warning: could not accept Unix socket connection: {error}");
                        }
                    }
                }

                _ = tokio::signal::ctrl_c() => {
                    std::fs::remove_file(Self::socket_path()).responsible_expect("could not remove socket");
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

        // If reading fails, emit a warning and resume listening.
        if let Err(error) = reader.read_line(&mut request).await {
            eprintln!("warning: could not read from socket: {error}");
            return Ok(());
        }

        let response = self.try_process_command(&request).await?.to_serialized();

        // Same as above if writing fails.
        if let Err(error) = writer.write_all(response.as_bytes()).await {
            eprintln!("warning: could not write to socket: {error}");
        }

        Ok(())
    }

    /// Accepts unvalidated protocol data from a client and validates it before dispatching relevant
    /// subroutines.
    async fn try_process_command(
        &self,
        raw_protocol: &str,
    ) -> Result<ResponseProtocol, DaemonError> {
        // Validate as JSON and as our protocol specifically.
        let request: CommandProtocol =
            serde_json::from_str(raw_protocol).map_err(|e| DaemonError::InvalidProtocol(e))?;

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
            _ => todo!(),
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
            dirs::runtime_dir()
                .map(|p| p.join("egress.sock"))
                .responsible_expect("XDG_RUNTIME_DIR not set")
        } else {
            PathBuf::from("/run/egress.sock")
        }
    }
}
