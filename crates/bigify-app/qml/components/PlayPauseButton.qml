import QtQuick
import Bigify

// The signature control. Material 3 Expressive shape morph: a circle while
// paused, a rounded square while playing. The icon reflects *confirmed*
// state; an unconfirmed request shows the loading indicator around it.
IconButton {
    id: root
    property bool playing: false
    property bool pendingRequest: false

    size: Theme.size.playButton
    variant: "filled"
    icon: playing ? "pause" : "play"
    busy: pendingRequest
    accessibleName: playing ? qsTr("Pause") : qsTr("Play")

    cornerRadius: playing ? size * 0.3 : size / 2
}
