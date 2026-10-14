use crate::auth::AuthState;
use crate::color::Scheme;
use crate::command::LibraryKind;
use crate::error::ServiceResult;
use crate::model::{ArtistDetail, CollectionDetail, Page, QueueSnapshot, SearchResults, Section};
use crate::playback::PlayerUpdate;

/// Notifications from the Spotify service to the UI.
#[derive(Debug, Clone)]
pub enum Event {
    Auth(AuthState),
    Player(PlayerUpdate),
    Home(ServiceResult<Vec<Section>>),
    Search {
        request_id: u32,
        result: ServiceResult<SearchResults>,
    },
    Library {
        kind: LibraryKind,
        offset: u32,
        result: ServiceResult<Page>,
    },
    Collection {
        uri: String,
        result: ServiceResult<CollectionDetail>,
    },
    Artist {
        uri: String,
        result: ServiceResult<ArtistDetail>,
    },
    Queue(ServiceResult<QueueSnapshot>),
    Palette {
        artwork_url: String,
        scheme: Box<Scheme>,
    },
    /// Confirmed library state for these URIs.
    Saved {
        states: Vec<(String, bool)>,
    },
    /// Saving or removing failed; the UI drops its pending indicator.
    SaveFailed {
        uri: String,
        message: String,
    },
    /// Library access through the user's own Spotify app. `status` is
    /// "notConfigured", "setup", "connected" or "error".
    LibraryAccess {
        status: String,
        message: String,
    },
    /// A command failed without a more specific event to carry the error.
    CommandFailed {
        message: String,
    },
}
