import QtQuick
import Bigify

// Persistent playback bar shown outside Now Playing. Focusable as a whole:
// A opens Now Playing, X toggles play/pause (handled globally).
Item {
    id: root
    signal openNowPlaying()

    height: Theme.size.miniPlayer
    activeFocusOnTab: true
    Accessible.role: Accessible.Button
    Accessible.name: qsTr("Now playing: %1 by %2").arg(Backend.trackTitle).arg(Backend.trackArtists)

    Rectangle {
        id: bg
        anchors.fill: parent
        radius: Theme.shape.extraExtraLarge
        color: Theme.surfaceContainerHigh
        border.width: root.activeFocus ? 0 : Math.max(1, Theme.dp(1))
        border.color: Theme.outlineVariant
        FocusRing { shown: root.activeFocus; radius: bg.radius }
    }

    PositionClock { id: clock; active: root.visible }

    Artwork {
        id: art
        anchors { left: parent.left; leftMargin: Theme.space.l; verticalCenter: parent.verticalCenter }
        width: root.height - Theme.space.l * 2
        height: width
        radius: Theme.shape.extraLarge
        source: Backend.artworkUrl
        kind: "track"
    }

    Column {
        anchors {
            left: art.right; leftMargin: Theme.space.xl
            right: controls.left; rightMargin: Theme.space.xxl
            verticalCenter: parent.verticalCenter
        }
        spacing: Theme.space.xs
        Text {
            width: parent.width
            text: Backend.trackTitle || (Backend.playbackStatus === "loading" ? qsTr("Loading…") : "")
            color: Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.titleMedium
            font.weight: Theme.weightTitle
            elide: Text.ElideRight
        }
        Text {
            width: parent.width
            text: Backend.trackArtists
            color: Theme.surfaceContentVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelLarge
            elide: Text.ElideRight
        }
        // Linear progress (confirmed position, extrapolated while playing).
        // Hidden when the engine reported no duration.
        Row {
            visible: Backend.durationMs > 0
            width: parent.width
            spacing: Theme.space.m
            topPadding: Theme.space.xs
            Text {
                id: elapsed
                anchors.verticalCenter: parent.verticalCenter
                text: clock.format(clock.position)
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall
                font.features: { "tnum": 1 }
            }
            Rectangle {
                id: track
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - elapsed.width - total.width - parent.spacing * 2
                height: Theme.dp(6)
                radius: height / 2
                color: Theme.alpha(Theme.surfaceContent, 0.18)
                Rectangle {
                    height: parent.height
                    radius: parent.radius
                    width: Math.max(height, parent.width * clock.fraction)
                    color: Theme.primary
                }
            }
            Text {
                id: total
                anchors.verticalCenter: parent.verticalCenter
                text: clock.format(Backend.durationMs)
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall
                font.features: { "tnum": 1 }
            }
        }
    }

    Row {
        id: controls
        anchors { right: parent.right; rightMargin: Theme.space.xxl; verticalCenter: parent.verticalCenter }
        spacing: Theme.space.l
        Item {
            width: Theme.dp(64); height: width
            anchors.verticalCenter: parent.verticalCenter
            Rectangle {
                anchors.fill: parent
                radius: Backend.isPlaying ? Theme.shape.large : width / 2
                color: Theme.primary
                Behavior on radius { NumberAnimation { duration: Theme.motion.medium; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
                Icon {
                    anchors.centerIn: parent
                    name: Backend.isPlaying ? "pause" : "play"
                    size: Theme.dp(30)
                    color: Theme.primaryContent
                }
            }
            LoadingIndicator {
                anchors.centerIn: parent
                size: Theme.dp(78)
                running: visible
                visible: /\b(play|pause|load|next|previous)\b/.test(Backend.pending)
            }
        }
        Row {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.activeFocus
            spacing: Theme.space.l
            ButtonHint { action: "select"; label: qsTr("Open") }
            ButtonHint { action: "playPause"; label: qsTr("Play/Pause") }
        }
    }

    Keys.onReturnPressed: root.openNowPlaying()
    Keys.onEnterPressed: root.openNowPlaying()
    MouseArea { anchors.fill: parent; onClicked: root.openNowPlaying() }
}
