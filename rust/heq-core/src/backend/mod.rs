pub mod snapshot;

pub use snapshot::Snapshot;

use std::fmt;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    pub connected: bool,
    pub message: String,
}

impl Status {
    pub fn ok(message: impl Into<String>) -> Self {
        Status {
            connected: true,
            message: message.into(),
        }
    }

    pub fn offline(message: impl Into<String>) -> Self {
        Status {
            connected: false,
            message: message.into(),
        }
    }
}

#[derive(Debug)]
pub enum BackendError {
    NotAvailable(String),
    Io(std::io::Error),
    Protocol(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::NotAvailable(m) => write!(f, "{}", m),
            BackendError::Io(e) => write!(f, "{}", e),
            BackendError::Protocol(m) => write!(f, "{}", m),
        }
    }
}

impl std::error::Error for BackendError {}

impl From<std::io::Error> for BackendError {
    fn from(e: std::io::Error) -> Self {
        BackendError::Io(e)
    }
}

pub trait Backend {
    fn name(&self) -> &str;

    fn apply(&mut self, snapshot: &Snapshot) -> Result<(), BackendError>;

    fn status(&self) -> Status;
}
