import QtQuick
import Bigify

// Artwork card for albums, playlists, artists and tracks. Focus lifts the
// card (scale + ring) and brightens the title; artists use circular art.
Item {
    id: root
    property var item: ({})
    property real cardWidth: Theme.size.card
    property bool playing: item && item.uri && (item.uri === Backend.trackUri)
    signal activated()

    width: cardWidth
    height: art.height + Theme.space.l + titleText.height + Theme.space.xs + subtitleText.height
    activeFocusOnTab: true

    Accessible.role: Accessible.Button
    Accessible.name: (item.title || "") + (item.subtitle ? ", " + item.subtitle : "")

    readonly property bool isArtist: item.kind === "artist"

    Item {
        id: lift
        width: parent.width
        height: art.height
        scale: root.activeFocus ? Theme.focusScale : 1.0
        transformOrigin: Item.Bottom
        Behavior on scale { NumberAnimation { duration: Theme.motion.medium; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }

        Artwork {
            id: art
            width: root.cardWidth
            height: width
            source: root.item.artworkUrl || ""
            kind: root.item.kind || "album"
            circular: root.isArtist
            radius: Theme.shape.extraLarge
        }

        // Now-playing badge
        Rectangle {
            visible: root.playing
            anchors { right: art.right; bottom: art.bottom; margins: Theme.space.m }
            width: Theme.dp(52); height: width; radius: width / 2
            color: Theme.primary
            Equalizer { anchors.centerIn: parent; color: Theme.primaryContent; size: Theme.dp(26) }
        }

        FocusRing {
            anchors.fill: art
            shown: root.activeFocus
            radius: art.effectiveRadius
        }
    }

    Text {
        id: titleText
        anchors { top: lift.bottom; topMargin: Theme.space.l; left: parent.left; right: parent.right }
        horizontalAlignment: root.isArtist ? Text.AlignHCenter : Text.AlignLeft
        text: root.item.title || ""
        color: root.activeFocus ? Theme.surfaceContent : Theme.alpha(Theme.surfaceContent, 0.92)
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.titleSmall
        font.weight: root.activeFocus ? Font.Bold : Theme.weightTitle
        elide: Text.ElideRight
        maximumLineCount: 1
    }
    Text {
        id: subtitleText
        anchors { top: titleText.bottom; topMargin: Theme.space.xs; left: parent.left; right: parent.right }
        horizontalAlignment: titleText.horizontalAlignment
        text: root.item.subtitle || " "
        color: Theme.surfaceContentVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelMedium
        elide: Text.ElideRight
        maximumLineCount: 1
    }

    Keys.onReturnPressed: root.activated()
    Keys.onEnterPressed: root.activated()

    MouseArea {
        anchors.fill: parent
        onClicked: { root.forceActiveFocus(); root.activated() }
    }
}
