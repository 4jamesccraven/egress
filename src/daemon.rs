use crate::config::Config;
use crate::database::Database;
use crate::error::DaemonError;
use crate::protocol::{CommandAction, CommandProtocol, ResponseData, ResponseProtocol};
use crate::telegram::{self, TelegramMessage};

use std::path::PathBuf;

use jiff::tz::TimeZone;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// The egress daemon (egressd), Responsible for accepting client requests and coordinating
/// application services.
pub struct Daemon {
    config: &'static Config,
    socket: UnixListener,
    database: Database,
}

impl Daemon {
    // -----------------------------------------------------------------------
    // Daemon Life Cycle
    // -----------------------------------------------------------------------

    /// Initialises a new daemon. Not called externally; use `Daemon::run` instead.
    async fn new() -> Result<Self, DaemonError> {
        Ok(Self {
            config: Config::load_config()?,
            socket: Self::init_socket()?,
            database: Database::new().await?,
        })
    }

    /// Initialises the local UNIX socket for the daemon.
    fn init_socket() -> Result<UnixListener, DaemonError> {
        let socket_path = Self::socket_path();

        if std::path::Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path)?;
        }

        UnixListener::bind(socket_path).map_err(|_| DaemonError::ConnectionFailed)
    }

    /// Runs the Daemon. Listens for socket and http connections.
    pub async fn run() -> Result<(), DaemonError> {
        let daemon = Self::new().await?;
        eprintln!("Started egressd.");

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
                    eprintln!("shutting down");
                    std::fs::remove_file(Self::socket_path())?;
                    break;
                }
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Connection Management
    // -----------------------------------------------------------------------

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

    /// Gets the path to the UNIX socket for the daemon.
    #[inline]
    pub fn socket_path() -> PathBuf {
        if cfg!(debug_assertions) {
            dirs::runtime_dir()
                .map(|p| p.join("egress.sock"))
                .expect("XDG_RUNTIME_DIR not set")
        } else {
            PathBuf::from("/run/egress.sock")
        }
    }

    // -----------------------------------------------------------------------
    // Protocol Handling
    // -----------------------------------------------------------------------

    /// Accepts unvalidated protocol data from a client and validates it before dispatching relevant
    /// subroutines.
    async fn try_process_command(
        &self,
        raw_protocol: &str,
    ) -> Result<ResponseProtocol, DaemonError> {
        // Validate as JSON and as our protocol specifically.
        let request: CommandProtocol =
            serde_json::from_str(raw_protocol).map_err(DaemonError::InvalidProtocol)?;

        let response = match request.action {
            CommandAction::NotifyLeft { source_id } => self.protocol_notify(source_id).await,
            CommandAction::Status => self.protocol_status().await,
            CommandAction::Purge { immediate } => self.protocol_purge(immediate).await,
            CommandAction::GetMessages => self.protocol_get_messages().await,
            _ => todo!(),
        };

        Ok(response)
    }

    /// Handles the `notify_left` protocol.
    async fn protocol_notify(&self, source_id: Option<String>) -> ResponseProtocol {
        _ = source_id; // TODO: use this to customise notifications
        let successful = self.notify_targets(&self.departure_message()).await;

        let total = self.config.targets.len();
        let success = successful == total;

        ResponseData::NotifyLeft {
            success_count: successful,
            total,
        }
        .to_protocol(success)
    }

    /// Handles the `status` protocol.
    async fn protocol_status(&self) -> ResponseProtocol {
        ResponseData::Status {
            text: "egressd is running.".into(),
        }
        .to_protocol(true)
    }

    /// Handles the `purge` protocol.
    async fn protocol_purge(&self, immediate: bool) -> ResponseProtocol {
        let messages = match if immediate {
            self.database.get_all().await
        } else {
            self.database.get_expired(self.config.expiry_hours).await
        } {
            Ok(messages) => messages,
            Err(error) => {
                return ResponseData::Purge {
                    success_count: 0,
                    failure: Vec::new(),
                    error: Some(error.to_string()),
                }
                .to_protocol(false);
            }
        };

        let mut success = 0;
        let mut failure = Vec::new();

        for message in messages {
            let deletion = self.delete_message(message).await;

            if let Err(error) = deletion {
                eprintln!("{error}");
                failure.push((message, error.to_string()).into());
            } else {
                success += 1;
            }
        }

        let overall_success = failure.is_empty();
        ResponseData::Purge {
            success_count: success,
            failure,
            error: None,
        }
        .to_protocol(overall_success)
    }

    async fn protocol_get_messages(&self) -> ResponseProtocol {
        match self.database.get_all().await {
            Ok(messages) => ResponseData::GetMessages { messages }.to_protocol(true),
            Err(error) => {
                eprintln!("could not get messages {error}");
                ResponseData::GetMessages { messages: vec![] }.to_protocol(false)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Application Logic
    // -----------------------------------------------------------------------

    /// Tries to delete from Telegram and from the database in that order, stopping at the first
    /// error.
    async fn delete_message(&self, message: TelegramMessage) -> Result<(), DaemonError> {
        match telegram::delete_message(message.chat_id, message.message_id).await {
            Ok(_) => {
                self.database
                    .delete_message(message.chat_id, message.message_id)
                    .await
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Formats the departure message for the user.
    fn departure_message(&self) -> String {
        let now = jiff::Timestamp::now().to_zoned(TimeZone::system());

        format!(
            "[{}]: {} is departing",
            now.strftime("%A, %B %-d, %Y %H:%M:%S"),
            self.config.user_name
        )
    }

    /// Sends a message to all Telegram chats in the user's config.
    #[must_use]
    async fn notify_targets(&self, text: &str) -> usize {
        let mut success_count = 0;

        for chat_id in &self.config.targets {
            match telegram::send_message(*chat_id, text).await {
                Ok(message_id) => {
                    success_count += 1;
                    if let Err(e) = self.database.record_message(*chat_id, message_id).await {
                        eprintln!("failed to store message: {e}")
                    }
                }
                Err(error) => eprintln!("failed to send message: {error}"),
            }
        }

        success_count
    }
}
