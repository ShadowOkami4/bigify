import QtQuick
import QtQuick.Effects
import Bigify

// Rounded artwork with a tonal placeholder. When no image is available the
// placeholder shows the item's kind icon instead of guessing at artwork.
Item {
    id: root
    property string source: ""
    property string kind: "album"
    property real radius: Theme.shape.large
    property bool circular: false
    // Decode at display size (×2 headroom) to bound memory on big libraries.
    property int decodeSize: Math.max(width, 64) * 2

    readonly property bool ready: image.status === Image.Ready
    readonly property real effectiveRadius: circular ? width / 2 : radius

    Rectangle {
        id: placeholder
        anchors.fill: parent
        radius: root.effectiveRadius
        color: Theme.surfaceContainerHighest
        opacity: root.ready ? 0 : 1
        Behavior on opacity { NumberAnimation { duration: Theme.motion.fade } }

        Icon {
            anchors.centerIn: parent
            size: Math.min(parent.width, parent.height) * 0.36
            color: Theme.surfaceContentVariant
            name: ({ "artist": "artist", "playlist": "playlist", "track": "nowPlaying",
                     "episode": "nowPlaying", "collection": "heart" })[root.kind] || "album"
        }
    }

    Image {
        id: image
        anchors.fill: parent
        source: root.source
        sourceSize.width: root.decodeSize
        sourceSize.height: root.decodeSize
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
        visible: false
        layer.enabled: true
    }

    Item {
        id: mask
        anchors.fill: parent
        visible: false
        layer.enabled: true
        layer.smooth: true
        Rectangle {
            anchors.fill: parent
            radius: root.effectiveRadius
            color: "black"
            antialiasing: true
        }
    }

    MultiEffect {
        anchors.fill: parent
        source: image
        maskEnabled: true
        maskSource: mask
        maskThresholdMin: 0.5
        maskSpreadAtMin: 1.0
        opacity: root.ready ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: Theme.motion.fadeLong } }
    }
}
