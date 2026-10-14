import QtQuick
import QtQuick.Effects
import Bigify

// Now Playing — Bigify's signature screen. Artwork-led, immersive, with the
// transport controls front and centre. Shows only confirmed information:
// no progress without a duration, no metadata the engine didn't report.
FocusScope {
    id: root
    signal openQueue()

    readonly property bool hasTrack: Backend.hasTrack
    readonly property bool loading: Backend.playbackStatus === "loading"
    function isPending(id) { return new RegExp("\\b" + id + "\\b").test(Backend.pending) }

    PositionClock { id: clock; active: root.visible }

    // ---- Backdrop: blurred artwork, dimmed for legibility ---------------
    Rectangle { anchors.fill: parent; color: Theme.surface }
    Image {
        id: backdropSource
        anchors.fill: parent
        source: Backend.artworkUrl
        sourceSize: Qt.size(160, 160)   // tiny decode; it is blurred anyway
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
    }
    MultiEffect {
        anchors.fill: parent
        source: backdropSource
        visible: backdropSource.status === Image.Ready
        blurEnabled: true
        blur: 1.0
        blurMax: 64
        saturation: 0.15
        brightness: -0.45
        opacity: 0.85
    }
    // Scrim keeps text contrast independent of artwork brightness.
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: Theme.alpha(Theme.surface, 0.35) }
            GradientStop { position: 0.55; color: Theme.alpha(Theme.surface, 0.75) }
            GradientStop { position: 1.0; color: Theme.alpha(Theme.surface, 0.9) }
        }
    }

    // ---- Empty state ------------------------------------------------------
    StatePanel {
        anchors.fill: parent
        visible: !root.hasTrack
        focus: visible
        state_: "empty"
        icon: "speaker"
        title: qsTr("Nothing playing")
        message: qsTr("Choose music from Home, Search or Library — or open Spotify on your phone, tap the devices icon and pick “%1”.")
                 .arg(Backend.deviceName || "Bigify")
    }

    // ---- Content -------------------------------------------------------------
    FocusScope {
        id: content
        visible: root.hasTrack
        focus: visible
        anchors { fill: parent; leftMargin: Theme.safeH + Theme.dp(40); rightMargin: Theme.safeH + Theme.dp(40); topMargin: Theme.safeV; bottomMargin: Theme.safeV }

        Item {
            id: artFrame
            width: Math.min(Theme.size.nowPlayingArt, content.height - Theme.dp(40))
            height: width
            anchors.verticalCenter: parent.verticalCenter

            // Soft elevation shadow (MultiEffect; Qt 6.8 has no RectangularShadow).
            Rectangle {
                anchors.fill: art
                anchors.margins: Theme.dp(12)
                radius: art.effectiveRadius
                color: Theme.surfaceContainerLowest
                scale: art.scale
                layer.enabled: true
                layer.effect: MultiEffect {
                    shadowEnabled: true
                    shadowBlur: 1.0
                    blurMax: 64
                    shadowVerticalOffset: Theme.dp(28)
                    shadowColor: Theme.alpha(Theme.scrim, 0.7)
                }
            }
            Artwork {
                id: art
                anchors.fill: parent
                source: Backend.artworkUrl
                kind: "track"
                radius: Theme.shape.extraExtraLarge
                // Expressive but restrained: the art settles slightly when paused.
                scale: Backend.isPlaying || Theme.reducedMotion ? 1.0 : 0.94
                Behavior on scale { NumberAnimation { duration: Theme.motion.slow; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
            }
        }

        Column {
            id: info
            anchors { left: artFrame.right; leftMargin: Theme.dp(96); right: parent.right; verticalCenter: parent.verticalCenter }
            spacing: Theme.space.l

            // Status chip: confirmed playback state.
            Rectangle {
                height: Theme.dp(48)
                width: statusRow.implicitWidth + Theme.space.xxl
                radius: height / 2
                color: Theme.alpha(Theme.primaryContainer, 0.85)
                Row {
                    id: statusRow
                    anchors.centerIn: parent
                    spacing: Theme.space.s
                    Equalizer { visible: Backend.isPlaying; size: Theme.dp(22); color: Theme.primaryContainerContent; anchors.verticalCenter: parent.verticalCenter }
                    LoadingIndicator { visible: root.loading; running: visible; size: Theme.dp(26); color: Theme.primaryContainerContent; anchors.verticalCenter: parent.verticalCenter }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.loading ? qsTr("Loading")
                            : Backend.isPlaying ? qsTr("Playing on %1").arg(Backend.deviceName || "Bigify")
                            : qsTr("Paused")
                        color: Theme.primaryContainerContent
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelMedium
                        font.weight: Font.Bold
                    }
                }
            }

            Text {
                width: parent.width
                text: Backend.trackTitle || (root.loading ? qsTr("Loading track…") : "")
                color: Theme.surfaceContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.displayMedium
                font.weight: Font.Bold
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
                lineHeight: 1.05
            }
            Text {
                width: parent.width
                visible: text !== ""
                text: Backend.trackArtists
                color: Theme.surfaceContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.headlineSmall
                font.weight: Font.Medium
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                visible: text !== ""
                text: Backend.trackAlbum
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.titleMedium
                elide: Text.ElideRight
            }

            Item { width: 1; height: Theme.space.xl }

            // Progress — only when the engine reported a duration.
            Column {
                width: parent.width
                spacing: Theme.space.s
                visible: Backend.durationMs > 0
                WavyProgress {
                    id: progress
                    width: parent.width
                    value: clock.fraction
                    playing: Backend.isPlaying
                    onSeekRequested: delta => Backend.seek(Math.max(0, Math.min(Backend.durationMs - 1000, clock.position + delta)))
                    KeyNavigation.down: playPause
                }
                Item {
                    width: parent.width
                    height: Theme.dp(32)
                    Text {
                        text: clock.format(clock.position)
                        color: Theme.surfaceContentVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelLarge
                        font.features: { "tnum": 1 }
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: progress.activeFocus
                        text: root.isPending("seek") ? qsTr("Seeking…") : qsTr("◀ ▶  Seek 10 s")
                        color: Theme.surfaceContentVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelSmall
                    }
                    Text {
                        anchors.right: parent.right
                        text: clock.format(Backend.durationMs)
                        color: Theme.surfaceContentVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelLarge
                        font.features: { "tnum": 1 }
                    }
                }
            }

            Item { width: 1; height: Theme.space.l }

            // Transport
            Row {
                id: transport
                spacing: Theme.space.xxl
                IconButton {
                    id: shuffleButton
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "shuffle"
                    toggle: true
                    checked: Backend.shuffle
                    busy: root.isPending("shuffle")
                    accessibleName: qsTr("Shuffle")
                    onClicked: Backend.toggleShuffle()
                    KeyNavigation.right: prevButton
                    KeyNavigation.up: progress.visible ? progress : null
                    KeyNavigation.down: queueButton
                }
                IconButton {
                    id: prevButton
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "previous"
                    variant: "tonal"
                    size: Theme.dp(96)
                    busy: root.isPending("previous")
                    accessibleName: qsTr("Previous")
                    onClicked: Backend.previous()
                    KeyNavigation.right: playPause
                    KeyNavigation.up: progress.visible ? progress : null
                    KeyNavigation.down: queueButton
                }
                PlayPauseButton {
                    id: playPause
                    focus: true
                    anchors.verticalCenter: parent.verticalCenter
                    playing: Backend.isPlaying
                    pendingRequest: root.isPending("play") || root.isPending("pause")
                    onClicked: Backend.togglePlay()
                    KeyNavigation.right: nextButton
                    KeyNavigation.up: progress.visible ? progress : null
                    KeyNavigation.down: queueButton
                }
                IconButton {
                    id: nextButton
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "next"
                    variant: "tonal"
                    size: Theme.dp(96)
                    busy: root.isPending("next")
                    accessibleName: qsTr("Next")
                    onClicked: Backend.next()
                    KeyNavigation.right: repeatButton
                    KeyNavigation.up: progress.visible ? progress : null
                    KeyNavigation.down: volumeDown
                }
                IconButton {
                    id: repeatButton
                    anchors.verticalCenter: parent.verticalCenter
                    icon: Backend.repeatMode === "track" ? "repeatOne" : "repeat"
                    toggle: true
                    checked: Backend.repeatMode !== "off"
                    busy: root.isPending("repeat")
                    accessibleName: Backend.repeatMode === "track" ? qsTr("Repeat one") : qsTr("Repeat")
                    onClicked: Backend.cycleRepeat()
                    KeyNavigation.up: progress.visible ? progress : null
                    KeyNavigation.down: volumeUp
                }
            }

            Item { width: 1; height: Theme.space.l }

            // Secondary: queue and volume
            Row {
                spacing: Theme.space.xl
                PillButton {
                    id: queueButton
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Up next")
                    icon: "queue"
                    variant: "tonal"
                    height_: Theme.size.buttonSmall
                    onClicked: root.openQueue()
                    KeyNavigation.up: playPause
                    KeyNavigation.right: volumeDown
                }
                Item { width: Theme.space.xl; height: 1 }
                IconButton {
                    id: volumeDown
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "volumeDown"
                    size: Theme.dp(64)
                    accessibleName: qsTr("Volume down")
                    onClicked: Backend.stepVolume(-0.05)
                    KeyNavigation.up: nextButton
                    KeyNavigation.right: volumeUp
                }
                Item {
                    width: Theme.dp(260); height: Theme.dp(64)
                    anchors.verticalCenter: parent.verticalCenter
                    Accessible.role: Accessible.ProgressBar
                    Accessible.name: qsTr("Volume %1 percent").arg(Math.round(Backend.volume * 100))
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width; height: Theme.dp(12); radius: height / 2
                        color: Theme.alpha(Theme.surfaceContent, 0.2)
                        Rectangle {
                            width: Math.max(height, parent.width * Backend.volume)
                            height: parent.height; radius: height / 2
                            color: root.isPending("volume") ? Theme.alpha(Theme.primary, 0.6) : Theme.primary
                            Behavior on width { NumberAnimation { duration: Theme.motion.quick } }
                        }
                    }
                }
                IconButton {
                    id: volumeUp
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "volumeUp"
                    size: Theme.dp(64)
                    accessibleName: qsTr("Volume up")
                    onClicked: Backend.stepVolume(0.05)
                    KeyNavigation.up: repeatButton
                }
            }
        }
    }

    // Controller hint bar
    Text {
        anchors { bottom: parent.bottom; bottomMargin: Theme.dp(24); horizontalCenter: parent.horizontalCenter }
        visible: root.hasTrack
        text: qsTr("X Play/Pause   ·   LB/RB Previous/Next   ·   LT/RT Volume   ·   Select Up next   ·   B Back")
        color: Theme.alpha(Theme.surfaceContentVariant, 0.8)
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelSmall
    }
}
