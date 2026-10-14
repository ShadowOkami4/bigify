import QtQuick
import Bigify

// One track in a list: artwork or index, title/artists, explicit badge,
// duration. The currently playing track is marked; the Y / Menu key adds the
// track to the queue.
Item {
    id: root
    property var item: ({})
    property int number: 0
    property bool showArtwork: true
    property bool showQueueAction: true
    readonly property bool isCurrent: item && item.uri === Backend.trackUri && Backend.hasTrack
    signal activated()

    height: Theme.size.trackRow
    activeFocusOnTab: true
    Accessible.role: Accessible.ListItem
    Accessible.name: (item.title || "") + ", " + (item.subtitle || "")

    function formatTime(ms) {
        if (!ms && ms !== 0) return ""
        const s = Math.floor(ms / 1000)
        return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0")
    }

    Rectangle {
        id: bg
        anchors.fill: parent
        radius: Theme.shape.extraLarge
        color: root.activeFocus ? Theme.surfaceContainerHighest
             : (root.isCurrent ? Theme.alpha(Theme.primaryContainer, 0.45) : "transparent")
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
        FocusRing { shown: root.activeFocus; radius: bg.radius; gap: Theme.dp(2) }
    }

    Row {
        id: lead
        anchors { left: parent.left; leftMargin: Theme.space.l; verticalCenter: parent.verticalCenter }
        spacing: Theme.space.l
        Item {
            width: root.showArtwork ? Theme.dp(68) : Theme.dp(48)
            height: Theme.dp(68)
            Artwork {
                visible: root.showArtwork
                anchors.fill: parent
                source: root.item.artworkUrl || ""
                kind: "track"
                radius: Theme.shape.medium
            }
            Text {
                visible: !root.showArtwork && !root.isCurrent
                anchors.centerIn: parent
                text: root.number > 0 ? root.number : ""
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.titleSmall
            }
            Rectangle {
                visible: root.isCurrent
                anchors.centerIn: parent
                width: Theme.dp(44); height: width; radius: width / 2
                color: Theme.primary
                Equalizer { anchors.centerIn: parent; color: Theme.primaryContent; size: Theme.dp(22) }
            }
        }
    }

    Column {
        anchors {
            left: lead.right; leftMargin: Theme.space.xl
            right: trail.left; rightMargin: Theme.space.xl
            verticalCenter: parent.verticalCenter
        }
        spacing: Theme.space.xs
        Text {
            width: parent.width
            text: root.item.title || ""
            color: root.isCurrent ? Theme.primary : Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.titleSmall
            font.weight: Theme.weightTitle
            elide: Text.ElideRight
        }
        Row {
            width: parent.width
            spacing: Theme.space.s
            Rectangle {
                visible: root.item.explicit === true
                width: Theme.dp(26); height: width; radius: Theme.shape.extraSmall
                color: Theme.alpha(Theme.surfaceContentVariant, 0.3)
                anchors.verticalCenter: parent.verticalCenter
                Text {
                    anchors.centerIn: parent
                    text: "E"
                    color: Theme.surfaceContent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall * 0.85
                    font.weight: Font.Bold
                }
                Accessible.name: qsTr("Explicit")
            }
            Text {
                width: parent.width - (root.item.explicit ? Theme.dp(34) : 0)
                text: root.item.subtitle || ""
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelMedium
                elide: Text.ElideRight
            }
        }
    }

    Row {
        id: trail
        anchors { right: parent.right; rightMargin: Theme.space.xl; verticalCenter: parent.verticalCenter }
        spacing: Theme.space.xl
        ButtonHint {
            visible: root.activeFocus && root.showQueueAction
            anchors.verticalCenter: parent.verticalCenter
            action: "more"
            label: qsTr("Add to queue")
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.formatTime(root.item.durationMs)
            color: Theme.surfaceContentVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelMedium
            font.features: { "tnum": 1 }
        }
    }

    Keys.onReturnPressed: root.activated()
    Keys.onEnterPressed: root.activated()
    Keys.onPressed: event => {
        const more = event.key === Qt.Key_Menu
            || (event.key === Qt.Key_Q && event.modifiers === Qt.NoModifier)
        if (more && root.showQueueAction && root.item.uri) {
            Backend.addToQueue(root.item.uri)
            event.accepted = true
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: { root.forceActiveFocus(); root.activated() }
    }
}
