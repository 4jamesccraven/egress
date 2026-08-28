use crate::config::Config;
use crate::database::Database;
use crate::error::DaemonError;
use crate::protocol::{CommandAction, CommandProtocol, ResponseData, ResponseProtocol};
use crate::telegram::{self, TelegramMessage};

use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Form, State};
use jiff::ToSpan;
use jiff::tz::TimeZone;
use serde::Deserialize;
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

    /// Initializes a new daemon. Not called externally; use `Daemon::run` instead.
    async fn new() -> Result<Self, DaemonError> {
        Ok(Self {
            config: Config::load_config()?,
            socket: Self::init_socket()?,
            database: Database::new().await?,
        })
    }

    /// Initializes the local UNIX socket for the daemon.
    fn init_socket() -> Result<UnixListener, DaemonError> {
        let socket_path = Self::socket_path();

        if std::path::Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path)?;
        }

        UnixListener::bind(socket_path).map_err(|_| DaemonError::ConnectionFailed)
    }

    /// Runs the Daemon. Listens for socket and http connections.
    pub async fn run() -> Result<(), DaemonError> {
        // Initialize the daemon
        let daemon = Arc::new(Self::new().await?);
        eprintln!("Started egressd.");

        // Run a pre-emptive purge to remove expired messages, and create a timer to do that on a
        // set interval.
        daemon.auto_purge().await;
        let purge_timer = tokio::time::sleep(Self::until_next_purge());
        tokio::pin!(purge_timer);

        // Initialize the http router and a tcp listener.
        let router = Self::http_router(daemon.clone()).await;
        let listener = tokio::net::TcpListener::bind("0.0.0.0:50925").await?;
        let http_server = axum::serve(listener, router).into_future();
        tokio::pin!(http_server);

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

                result = &mut http_server => {
                    result?;
                    break;
                }

                _ = &mut purge_timer => {
                    daemon.auto_purge().await;

                    // Reset the timer
                    purge_timer.as_mut().reset(
                        tokio::time::Instant::now() + Self::until_next_purge()
                    );
                }

                _ = tokio::signal::ctrl_c() => {
                    eprintln!("\nshutting down");
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
                .expect("XDG_RUNTIME_DIR not se&t")
        } else {
            PathBuf::from("/run/egress.sock")
        }
    }

    async fn http_router(daemon: Arc<Self>) -> axum::Router {
        use axum::routing::post;
        axum::Router::new()
            .route("/notify", post(Self::http_notify))
            .with_state(daemon)
    }

    async fn http_notify(
        State(daemon): State<Arc<Self>>,
        Form(request): Form<HttpNotifyRequest>,
    ) -> Json<ResponseProtocol> {
        let _ = request.protocol_version;

        Json(daemon.protocol_notify(request.source_id).await)
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
        let admin_success_text = if success { "Success" } else { "Failure" };

        // Attempt to notify the admin.
        _ = telegram::send_message(
            self.config.admin_target,
            &format!("Notification {admin_success_text}: {successful} messages sent"),
        )
        .await;

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

    /// Performs the purge action internally, without an external client.
    ///
    /// See [`Daemon::protocol_purge`] for more info.
    async fn auto_purge(&self) {
        let response = self.protocol_purge(false).await;
        if !response.success {
            eprintln!("warning: could not purge db: {}", response.to_serialized());
            return;
        }
        match response.data {
            ResponseData::Purge {
                success_count,
                failure: _,
                error: _,
            } => match success_count {
                0 => eprintln!("purge info: no expired messages to purge."),
                1 => eprintln!("purge info: one expired message purged."),
                2.. => eprintln!("purge info: {success_count} expired messages purged."),
            },
            _ => unreachable!(),
        }
    }

    /// Calculates the time until the next database purge.
    ///
    /// Purges occur every sixth hour of the day (i.e., 00:00, 06:00, 12:00, 18:00)
    fn until_next_purge() -> std::time::Duration {
        let now = jiff::Zoned::now();

        let next_hour = ((now.hour() / 6) + 1) * 6;

        let next = if next_hour < 24 {
            now.with()
                .hour(next_hour)
                .minute(0)
                .second(0)
                .nanosecond(0)
                .build()
                .expect("valid purge time")
        } else {
            now.start_of_day()
                .expect("current day has a start")
                .checked_add(1.day())
                .expect("next day is representable")
        };

        std::time::Duration::try_from(now.duration_until(&next))
            .expect("next purge time is after now")
    }
}

#[derive(Deserialize)]
struct HttpNotifyRequest {
    protocol_version: u8,
    source_id: Option<String>,
}
