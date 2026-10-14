use bigify_core::error::{ErrorKind, ServiceError};
use librespot::core::error::ErrorKind as LsKind;

/// Map a librespot error to a UI-facing error.
///
/// librespot's error text can include request URLs; it never contains access
/// tokens (they travel in headers), so including the kind's description is safe.
pub fn from_librespot(e: librespot::core::Error) -> ServiceError {
    let kind = match e.kind {
        LsKind::Unauthenticated => ErrorKind::Unauthorized,
        LsKind::PermissionDenied => ErrorKind::Unsupported,
        LsKind::NotFound => ErrorKind::NotFound,
        LsKind::ResourceExhausted => ErrorKind::RateLimited,
        LsKind::Unimplemented => ErrorKind::Unsupported,
        LsKind::DeadlineExceeded | LsKind::Unavailable | LsKind::Aborted | LsKind::Cancelled => {
            ErrorKind::Network
        }
        _ => ErrorKind::Internal,
    };
    let message = match kind {
        ErrorKind::Unauthorized => "Spotify rejected the session. Try signing in again.".to_owned(),
        ErrorKind::Unsupported => {
            "Spotify doesn't allow this through the interfaces Bigify uses.".to_owned()
        }
        ErrorKind::NotFound => "This item isn't available.".to_owned(),
        ErrorKind::RateLimited => "Spotify is busy. Try again in a moment.".to_owned(),
        ErrorKind::Network => "Couldn't reach Spotify. Check your connection.".to_owned(),
        _ => format!("Something went wrong ({})", e.kind),
    };
    log::debug!("librespot error mapped to {kind:?}: {e}");
    ServiceError::new(kind, message)
}
