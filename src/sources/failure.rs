use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    NotSignedIn,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    kind: FailureKind,
    message: String,
}

impl Failure {
    pub fn said_by_gh(message: String) -> Self {
        let kind = if message.contains("gh auth login") {
            FailureKind::NotSignedIn
        } else {
            FailureKind::Other
        };
        Self { kind, message }
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
