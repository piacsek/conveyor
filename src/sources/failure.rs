use std::any::Any;
use std::{fmt, io};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    ProgramMissing,
    RateLimited,
    NotSignedIn,
    Offline,
    TimedOut,
    Crashed,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    kind: FailureKind,
    message: String,
}

impl Failure {
    pub fn said_by_gh(message: String) -> Self {
        let kind = if message.to_lowercase().contains("rate limit") {
            FailureKind::RateLimited
        } else if message.contains("gh auth login") || message.contains("HTTP 401") {
            FailureKind::NotSignedIn
        } else if message.contains("error connecting to") || message.contains("dial tcp") {
            FailureKind::Offline
        } else {
            FailureKind::Other
        };
        Self { kind, message }
    }

    pub fn timed_out(message: String) -> Self {
        Self {
            kind: FailureKind::TimedOut,
            message,
        }
    }

    pub fn crashed(payload: &(dyn Any + Send)) -> Self {
        let said = crate::crash::said(payload);
        Self {
            kind: FailureKind::Crashed,
            message: format!("crashed: {said}"),
        }
    }

    pub fn kind(&self) -> FailureKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn reworded(self, message: String) -> Self {
        Self { message, ..self }
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self {
            kind: FailureKind::Other,
            message,
        }
    }
}

impl From<io::Error> for Failure {
    fn from(err: io::Error) -> Self {
        let kind = if err.kind() == io::ErrorKind::NotFound {
            FailureKind::ProgramMissing
        } else {
            FailureKind::Other
        };
        Self {
            kind,
            message: err.to_string(),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
