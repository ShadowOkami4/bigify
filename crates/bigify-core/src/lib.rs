//! Bigify core: the parts of the application that know nothing about Qt or
//! about how Spotify is reached.
//!
//! * [`model`] — media items, tracks and browse sections shown by the UI.
//! * [`playback`] — the playback state machine that separates *requested*
//!   operations from *confirmed* backend state.
//! * [`command`] / [`event`] — the message contract between the UI and the
//!   playback/Spotify service.
//! * [`settings`] — persisted user preferences.
//! * [`color`] — artwork-derived dynamic color with contrast guarantees.
//! * [`paths`] — XDG directory resolution.

pub mod auth;
pub mod color;
pub mod command;
pub mod error;
pub mod event;
pub mod model;
pub mod paths;
pub mod playback;
pub mod settings;

pub use auth::AuthState;
pub use command::{Command, LibraryKind};
pub use error::{ErrorKind, ServiceError};
pub use event::Event;
