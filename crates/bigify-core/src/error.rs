use serde::Serialize;

/// Coarse error categories the UI uses to choose wording and recovery actions.
///
/// The distinction between [`ErrorKind::Unsupported`] (will never work with
/// the current integration) and transient kinds such as
/// [`ErrorKind::Network`] (retrying may help) is deliberate: the UI must not
/// offer "Retry" for something that cannot succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// Connectivity problem; retrying later may succeed.
    Network,
    /// The session or token was rejected; the user may need to sign in again.
    Unauthorized,
    /// The requested item does not exist or is not available in this market.
    NotFound,
    /// Spotify is throttling requests.
    RateLimited,
    /// The feature is not available through the interfaces Bigify uses.
    Unsupported,
    /// The service is not ready (e.g. not signed in yet).
    NotReady,
    /// Malformed response or unexpected internal failure.
    Internal,
}

impl ErrorKind {
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            ErrorKind::Network | ErrorKind::RateLimited | ErrorKind::Internal
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct ServiceError {
    pub kind: ErrorKind,
    /// Human-readable, already free of sensitive data.
    pub message: String,
}

impl ServiceError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unsupported, message)
    }

    pub fn not_ready() -> Self {
        Self::new(ErrorKind::NotReady, "Not connected to Spotify yet")
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;
