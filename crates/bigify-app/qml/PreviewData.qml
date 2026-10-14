pragma Singleton
import QtQuick

// Sample content for the *design preview* (BIGIFY_DESIGN_PREVIEW=1), used to
// review layouts at 1080p/4K without a Spotify account. Names are obviously
// fictional and the UI shows a "Design preview" banner; this data is never
// shown in normal operation.
QtObject {
    function make(kind, i, title, subtitle) {
        return { uri: "preview:" + kind + ":" + i, kind: kind, title: title, subtitle: subtitle,
                 artworkUrl: "", durationMs: kind === "track" ? 150000 + i * 17000 : null,
                 explicit: i % 5 === 3, contextUri: kind === "track" ? "preview:album:1" : null }
    }
    function many(kind, prefix, subtitle, n) {
        const out = []
        for (let i = 1; i <= n; i++) out.push(make(kind, i, prefix + " " + i, subtitle))
        return out
    }

    readonly property var homeSections: [
        { id: "jumpBackIn", title: "Jump back in", layout: "cards",
          items: [make("album", 1, "Sample Album With A Longer Title", "Sample Artist")]
                 .concat(many("playlist", "Sample Playlist", "Playlist · Preview", 7)) },
        { id: "recentTracks", title: "Recently played", layout: "cards", items: many("track", "Sample Track", "Sample Artist", 10) },
        { id: "playlists", title: "Your playlists", layout: "cards", items: many("playlist", "My Playlist", "Playlist · Preview", 10) },
        { id: "topArtists", title: "Your top artists", layout: "cards", items: many("artist", "Sample Artist", "Artist", 10) },
        { id: "albums", title: "Saved albums", layout: "cards", items: many("album", "Sample Album", "Sample Artist", 10) }
    ]

    function searchResults(q) {
        return { query: q, tracks: many("track", "Result Track", "Sample Artist", 6),
                 artists: many("artist", "Result Artist", "Artist", 6),
                 albums: many("album", "Result Album", "Sample Artist", 6),
                 playlists: many("playlist", "Result Playlist", "Playlist · Preview", 6) }
    }

    function library(kind) {
        const map = { playlists: ["playlist", "My Playlist", "Playlist · Preview"],
                      albums: ["album", "Saved Album", "Sample Artist"],
                      artists: ["artist", "Followed Artist", "Artist"],
                      likedSongs: ["track", "Liked Track", "Sample Artist"] }
        const m = map[kind]
        return { total: 24, items: many(m[0], m[1], m[2], 24) }
    }

    function collectionFor(item) {
        return { item: item, description: "Album · 2026 · 12 songs", truncated: false, totalTracks: 12,
                 tracks: many("track", "Sample Track", item.subtitle || "Sample Artist", 12) }
    }

    readonly property var queue: ({ current: make("track", 1, "Sample Track 1", "Sample Artist"),
                                    upcoming: many("track", "Up Next Track", "Sample Artist", 8) })
}
