use crate::playback::RepeatMode;

/// Saved-content categories shown in the Library screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryKind {
    Playlists,
    Albums,
    Artists,
    LikedSongs,
}

impl LibraryKind {
    pub fn id(self) -> &'static str {
        match self {
            LibraryKind::Playlists => "playlists",
            LibraryKind::Albums => "albums",
            LibraryKind::Artists => "artists",
            LibraryKind::LikedSongs => "likedSongs",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "playlists" => LibraryKind::Playlists,
            "albums" => LibraryKind::Albums,
            "artists" => LibraryKind::Artists,
            "likedSongs" => LibraryKind::LikedSongs,
            _ => return None,
        })
    }
}

/// Requests from the UI to the Spotify service. Commands are fire-and-forget;
/// results arrive later as [`crate::Event`]s.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Start the OAuth flow in the system browser on this machine.
    SignInWithBrowser,
    /// Abandon an in-progress browser sign-in.
    CancelSignIn,
    /// Forget cached credentials and disconnect.
    SignOut,

    Play,
    Pause,
    Next,
    Previous,
    Seek(u32),
    SetShuffle(bool),
    SetRepeat(RepeatMode),
    SetVolume(u16),
    /// Play a context (album, playlist, artist…), optionally starting at a track.
    PlayContext {
        context_uri: String,
        track_uri: Option<String>,
    },
    /// Play an explicit list of tracks starting at `index`.
    PlayTracks {
        uris: Vec<String>,
        index: u32,
    },
    AddToQueue(String),

    LoadHome,
    Search {
        request_id: u32,
        query: String,
    },
    LoadLibrary {
        kind: LibraryKind,
        offset: u32,
    },
    LoadCollection {
        uri: String,
    },
    LoadQueue,
    /// Compute the dynamic color palette for an artwork URL.
    LoadPalette {
        artwork_url: String,
    },

    Shutdown,
}
