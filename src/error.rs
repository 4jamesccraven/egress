#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("{0}")]
    Config(#[from] ConfigError),

    #[error("could not honor protocol (unimplemented)")]
    NotImplemented,

    #[error("could not honor protocol")]
    InvalidProtocol(#[from] serde_json::Error),

    #[error("could not connect to client")]
    ConnectionFailed,

    #[error("{0}")]
    DatabaseFailure(#[from] sqlx::Error),

    #[error("{0}")]
    MigrationFailure(#[from] sqlx::migrate::MigrateError),

    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Telegram(#[from] TelegramError),
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read configuration")]
    ReadFailure(#[from] std::io::Error),

    #[error("config not found")]
    NotFound,

    #[error("cannot have config.toml and config.json simultaneously")]
    TooMany,

    #[error("could not parse config.json")]
    InvalidJSON(#[from] serde_json::Error),

    #[error("could not parse config.toml")]
    InvalidTOML(#[from] toml::de::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("{0}")]
    Network(#[from] reqwest::Error),

    #[error("malformed response: {0}")]
    MalformedResponse(#[from] serde_json::Error),

    #[error("{0}")]
    Api(String),

    #[error("unknown error")]
    Unknown,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("could not connect to the daemon: {0}")]
    ConnectionFailed(#[source] std::io::Error),

    #[error("no response from daemon")]
    NoResponse,

    #[error("unable to read from socket: {0}")]
    ReadFailure(#[source] std::io::Error),

    #[error("unintelligible response from daemon: {0}")]
    InvalidResponse(#[from] serde_json::Error),

    #[error("unable to write to socket: {0}")]
    WriteFailure(#[source] std::io::Error),
}
