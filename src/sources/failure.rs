use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    ProgramMissing,
    RateLimited,
    NotSignedIn,
    Offline,
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

    pub fn program_missing(message: String) -> Self {
        Self {
            kind: FailureKind::ProgramMissing,
            message,
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

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
