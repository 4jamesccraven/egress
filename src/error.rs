use std::panic::Location;

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("{}", .0)]
    Config(#[from] ConfigError),
    #[error("could not honor protocol (unimplemented)")]
    NotImplemented,
    #[error("could not honor protocol")]
    InvalidProtocol(#[from] serde_json::Error),
    #[error("could not connect to client")]
    ConnectionFailed,
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

pub trait ExpectExt<T> {
    fn responsible_expect(self, message: &str) -> T;
}

impl<T, E: std::fmt::Debug> ExpectExt<T> for Result<T, E> {
    #[track_caller]
    fn responsible_expect(self, message: &str) -> T {
        let location = Location::caller();

        self.expect(&format!(
            "Fatal: {message}, ({}:{},{})",
            location.file(),
            location.line(),
            location.column()
        ))
    }
}

impl<T> ExpectExt<T> for Option<T> {
    #[track_caller]
    fn responsible_expect(self, message: &str) -> T {
        let location = Location::caller();

        self.expect(&format!(
            "Fatal: {message}, ({}:{},{})",
            location.file(),
            location.line(),
            location.column()
        ))
    }
}
