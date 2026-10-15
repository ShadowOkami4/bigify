import QtQuick
import QtQuick.Effects
import Bigify

// Now Playing — Bigify's signature screen. Artwork-led, immersive, with the
// transport controls front and centre. Shows only confirmed information:
// no progress without a duration, no metadata the engine didn't report.
//
// Immersive mode: after a few seconds without input while music plays, the
// controls fade away, leaving artwork, title and progress (or lyrics). The
// first navigation key only wakes the UI; media keys keep working.
FocusScope {
    id: root
    signal openQueue()

    // ---- Immersive mode -------------------------------------------------
    property bool immersive: false
    property Item focusBeforeImmersive: null
    readonly property real chromeOpacity: immersive ? 0 : 1

    function enterImmersive() {
        if (immersive || !root.activeFocus || !root.hasTrack) return
        focusBeforeImmersive = root.Window.activeFocusItem
        immersive = true
        waker.forceActiveFocus()
    }
    function exitImmersive() {
        if (!immersive) return
        immersive = false
        if (focusBeforeImmersive && focusBeforeImmersive.visible) focusBeforeImmersive.forceActiveFocus()
        else playPause.forceActiveFocus()
        focusBeforeImmersive = null
        idleTimer.restart()
    }

    Timer {
        id: idleTimer
        interval: 6000
        running: root.enabled && root.activeFocus && Backend.isPlaying && !root.immersive
        onTriggered: root.enterImmersive()
    }
    Connections {
        target: Backend
        function onInputSerialChanged() { if (!root.immersive) idleTimer.restart() }
        function onIsPlayingChanged() { if (!Backend.isPlaying) root.exitImmersive() }
    }
    onEnabledChanged: if (!enabled) { immersive = false; focusBeforeImmersive = null }

    // Holds focus while immersive: navigation keys wake the UI and are
    // consumed; everything else (media keys, volume, lyrics) passes through.
    Item {
        id: waker
        Keys.onPressed: event => {
            const wake = [Qt.Key_Up, Qt.Key_Down, Qt.Key_Left, Qt.Key_Right, Qt.Key_Return,
                          Qt.Key_Enter, Qt.Key_Space, Qt.Key_Escape, Qt.Key_Back]
            if (wake.indexOf(event.key) >= 0) {
                root.exitImmersive()
                event.accepted = true
            } else {
                event.accepted = false
            }
        }
    }
    MouseArea {
        anchors.fill: parent
        z: 100
        enabled: root.immersive
        hoverEnabled: true
        property point start
        property bool tracking: false
        onEnabledChanged: tracking = false
        onPositionChanged: mouse => {
            if (!tracking) { start = Qt.point(mouse.x, mouse.y); tracking = true; return }
            if (Math.abs(mouse.x - start.x) + Math.abs(mouse.y - start.y) > 12) root.exitImmersive()
        }
        onPressed: root.exitImmersive()
    }

    // ---- Lyrics ------------------------------------------------------------
    property bool lyricsOpen: false
    property var lyrics: null
    property string lyricsStatus: "idle"
    property string lyricsMessage: ""
    property string lyricsFor: ""

    function toggleLyrics() {
        lyricsOpen = !lyricsOpen
        ensureLyrics()
    }
    function ensureLyrics() {
        if (!lyricsOpen || Backend.trackUri === "" || lyricsFor === Backend.trackUri) return
        lyricsFor = Backend.trackUri
        lyrics = null
        lyricsMessage = ""
        lyricsStatus = "loading"
        if (Backend.designPreview) {
            lyrics = PreviewData.lyrics
            lyricsStatus = "ready"
            return
        }
        Backend.loadLyrics()
    }
    Connections {
        target: Backend
        function onTrackUriChanged() { root.ensureLyrics() }
        function onLyricsLoaded(uri, json) {
            if (uri !== root.lyricsFor) return
            root.lyrics = JSON.parse(json)
            root.lyricsStatus = "ready"
        }
        function onLyricsFailed(uri, kind, message) {
            if (uri !== root.lyricsFor) return
            root.lyricsMessage = message
            root.lyricsStatus = "error"
        }
    }

    Keys.onPressed: event => {
        const plain = event.modifiers === Qt.NoModifier
        if (event.key === Qt.Key_Menu || (plain && (event.key === Qt.Key_L || event.key === Qt.Key_Q))) {
            root.toggleLyrics()
            event.accepted = true
        }
    }

    readonly property bool hasTrack: Backend.hasTrack
    readonly property bool loading: Backend.playbackStatus === "loading"
    function isPending(id) { return new RegExp("\\b" + id + "\\b").test(Backend.pending) }

    PositionClock { id: clock; active: root.visible }
    // Real sessions check the Liked state when the track changes (bridge);
    // the design preview has no backend to ask.
    Component.onCompleted: if (Backend.designPreview) SavedState.check([Backend.trackUri])

    // ---- Backdrop: blurred, zoomed artwork, dimmed for legibility --------
    Rectangle { anchors.fill: parent; color: Theme.surface }
    Image {
        id: backdropImage
        anchors.fill: parent
        source: Backend.artworkUrl
        sourceSize: Qt.size(160, 160)   // tiny decode; it is blurred anyway
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
    }
    // An Image's own texture is the decoded 160 px bitmap, not the cropped,
    // screen-filling rendering, so render it into a texture first (small:
    // it is blurred anyway).
    ShaderEffectSource {
        id: backdropSource
        anchors.fill: parent
        sourceItem: backdropImage
        textureSize: Qt.size(320, 320 * height / Math.max(1, width))
        smooth: true
        visible: false
    }
    MultiEffect {
        anchors.fill: parent
        scale: 1.15                     // zoomed in, hides blurred edges
        source: backdropSource
        visible: backdropImage.status === Image.Ready
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
                scale: Theme.reducedMotion ? 1.0 : (!Backend.isPlaying ? 0.94 : (root.immersive ? 1.04 : 1.0))
                Behavior on scale { NumberAnimation { duration: Theme.motion.slow; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
            }
        }

        Column {
            id: info
            anchors { left: artFrame.right; leftMargin: Theme.dp(96); right: parent.right; verticalCenter: parent.verticalCenter }
            spacing: Theme.space.l

            // Status chip: confirmed playback state.
            Rectangle {
                opacity: root.chromeOpacity
                Behavior on opacity { NumberAnimation { duration: Theme.motion.fadeLong * 2 } }
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
                visible: root.lyricsOpen
                text: Backend.trackTitle + (Backend.trackArtists ? "  ·  " + Backend.trackArtists : "")
                color: Theme.surfaceContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.titleLarge
                font.weight: Font.Bold
                elide: Text.ElideRight
            }
            LyricsView {
                id: lyricsView
                visible: root.lyricsOpen
                width: parent.width
                height: Math.max(Theme.dp(240), content.height - Theme.dp(650))
                lyrics: root.lyrics
                status: root.lyricsStatus
                message: root.lyricsMessage
                position: clock.position
                KeyNavigation.down: progress.visible ? progress : playPause
            }

            Text {
                width: parent.width
                visible: !root.lyricsOpen
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
                visible: text !== "" && !root.lyricsOpen
                text: Backend.trackArtists
                color: Theme.surfaceContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.headlineSmall
                font.weight: Font.Medium
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                visible: text !== "" && !root.lyricsOpen
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
                    KeyNavigation.up: lyricsView.visible && lyricsView.activeFocusOnTab ? lyricsView : null
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
                    ButtonHint {
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: progress.activeFocus
                        action: "seek"
                        label: root.isPending("seek") ? qsTr("Seeking…") : qsTr("Seek 10 s")
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
                opacity: root.chromeOpacity
                Behavior on opacity { NumberAnimation { duration: Theme.motion.fadeLong * 2 } }
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
                    KeyNavigation.down: likeButton.visible ? likeButton : queueButton
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
                    KeyNavigation.down: lyricsButton
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
                    KeyNavigation.down: lyricsButton
                }
            }

            Item { width: 1; height: Theme.space.l }

            // Secondary: like, queue and lyrics. Volume has no on-screen control
            // (like Spotify's TV app): the device's volume keys, Ctrl+Up/Down or
            // the phone change it, and the volume indicator shows the result.
            Row {
                spacing: Theme.space.xl
                opacity: root.chromeOpacity
                Behavior on opacity { NumberAnimation { duration: Theme.motion.fadeLong * 2 } }
                IconButton {
                    id: likeButton
                    anchors.verticalCenter: parent.verticalCenter
                    visible: SavedState.isKnown(Backend.trackUri)
                    icon: SavedState.isSaved(Backend.trackUri) ? "heartFilled" : "heart"
                    toggle: true
                    checked: SavedState.isSaved(Backend.trackUri)
                    busy: SavedState.isPending(Backend.trackUri)
                    size: Theme.dp(64)
                    accessibleName: checked ? qsTr("Remove from Liked Songs") : qsTr("Add to Liked Songs")
                    onClicked: SavedState.toggle(Backend.trackUri)
                    KeyNavigation.up: shuffleButton
                    KeyNavigation.right: queueButton
                }
                PillButton {
                    id: queueButton
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Up next")
                    icon: "queue"
                    variant: "tonal"
                    height_: Theme.size.buttonSmall
                    onClicked: root.openQueue()
                    KeyNavigation.up: playPause
                    KeyNavigation.left: likeButton.visible ? likeButton : null
                    KeyNavigation.right: lyricsButton
                }
                PillButton {
                    id: lyricsButton
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.lyricsOpen ? qsTr("Hide lyrics") : qsTr("Lyrics")
                    icon: "lyrics"
                    variant: root.lyricsOpen ? "filled" : "tonal"
                    height_: Theme.size.buttonSmall
                    onClicked: root.toggleLyrics()
                    KeyNavigation.up: nextButton
                    KeyNavigation.left: queueButton
                }
            }
        }
    }

    // Button hints (controller glyphs or keyboard keys, whichever is in use)
    Row {
        anchors { bottom: parent.bottom; bottomMargin: Theme.dp(22); horizontalCenter: parent.horizontalCenter }
        visible: root.hasTrack
        spacing: Theme.space.xxl
        opacity: root.chromeOpacity
        Behavior on opacity { NumberAnimation { duration: Theme.motion.fadeLong * 2 } }
        ButtonHint { action: "playPause"; label: qsTr("Play/Pause") }
        ButtonHint { action: "lyrics"; label: root.lyricsOpen ? qsTr("Hide lyrics") : qsTr("Lyrics") }
        ButtonHint { action: "prevNext"; label: qsTr("Previous/Next") }
        ButtonHint { action: "skip10"; label: qsTr("−10 s / +10 s") }
        ButtonHint { action: "upNext"; label: qsTr("Up next") }
        ButtonHint { action: "back"; label: qsTr("Back") }
    }
}
