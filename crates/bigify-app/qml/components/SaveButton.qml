import QtQuick
import Bigify

// Save / Saved toggle for albums, playlists and tracks; Follow / Following
// for artists. Hidden until Spotify has told us the current state.
PillButton {
    id: root
    property string uri: ""
    readonly property bool artist: SavedState.kindOf(uri) === "artist"
    readonly property bool saved: SavedState.isSaved(uri)

    visible: uri !== "" && SavedState.isKnown(uri)
    variant: saved ? "tonal" : "outlined"
    icon: artist ? (saved ? "check" : "add") : (saved ? "heartFilled" : "heart")
    text: artist ? (saved ? qsTr("Following") : qsTr("Follow"))
                 : (saved ? qsTr("Saved") : qsTr("Save"))
    busy: SavedState.isPending(uri)
    onClicked: SavedState.toggle(uri)
}
