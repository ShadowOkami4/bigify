use serde::Serialize;

/// Connection/authentication lifecycle as reported by the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum AuthState {
    /// Service thread is starting and checking cached credentials.
    Starting,
    /// No usable credentials. `discovery` is true when Bigify is advertised on
    /// the local network so the Spotify app on a phone can sign it in.
    SignedOut { discovery: bool },
    /// A browser sign-in is in progress on this machine.
    BrowserSignIn,
    /// Credentials obtained; establishing the session.
    Connecting,
    /// Session established and Bigify is visible as a Spotify Connect device.
    Connected { user: String, device_name: String },
    /// The connection dropped; reconnecting automatically.
    Reconnecting { attempt: u32 },
    /// Unrecoverable without user action.
    Failed { message: String },
}

impl AuthState {
    /// Stable identifier used by QML.
    pub fn id(&self) -> &'static str {
        match self {
            AuthState::Starting => "starting",
            AuthState::SignedOut { .. } => "signedOut",
            AuthState::BrowserSignIn => "browserSignIn",
            AuthState::Connecting => "connecting",
            AuthState::Connected { .. } => "connected",
            AuthState::Reconnecting { .. } => "reconnecting",
            AuthState::Failed { .. } => "failed",
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, AuthState::Connected { .. })
    }
}
