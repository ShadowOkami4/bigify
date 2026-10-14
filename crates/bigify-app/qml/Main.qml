import QtQuick
import QtQuick.Window
import Bigify

// Application shell: navigation rail, section pages with per-section detail
// stacks, the immersive Now Playing overlay, the queue sheet, sign-in, and
// global controller/media keys.
//
// Focus model (see docs/CONTROLS.md):
//  * Keys go to the focused item first and propagate to parents when
//    unhandled. Lists consume arrows only while they can move, so pressing
//    Left at a row's start bubbles up here and focus moves to the rail.
//  * Every page is a FocusScope, so returning to it restores the last
//    focused element. Overlays remember what had focus and restore it.
Window {
    id: window
    readonly property var dev: JSON.parse(Backend.devJson)
    readonly property var forcedSize: dev.windowSize ? dev.windowSize.split("x").map(Number) : null
    readonly property var settings: JSON.parse(Backend.settingsJson || "{}")

    width: forcedSize ? forcedSize[0] : 1920
    height: forcedSize ? forcedSize[1] : 1080
    visible: true
    visibility: !forcedSize && settings.startFullscreen !== false ? Window.FullScreen : Window.Windowed
    title: "Bigify"
    color: Theme.surface

    Binding { target: Theme; property: "windowHeight"; value: window.height }
    Binding { target: Theme; property: "windowWidth"; value: window.width }

    FocusScope {
        id: app
        anchors.fill: parent
        focus: true

        property string section: "home"
        property bool nowPlayingOpen: false
        property bool queueOpen: false
        property bool everConnected: false
        // Per-section stack of detail pages (album, playlist, artist); Back pops.
        property var detailStacks: ({ home: [], search: [], library: [] })
        readonly property var details: {
            const d = {}
            for (const k in detailStacks) {
                const s = detailStacks[k]
                d[k] = s.length > 0 ? s[s.length - 1] : null
            }
            return d
        }
        property Item focusBeforeOverlay: null

        readonly property var destinations: [
            { id: "home", label: qsTr("Home"), icon: "home" },
            { id: "search", label: qsTr("Search"), icon: "search" },
            { id: "library", label: qsTr("Library"), icon: "library" },
            { id: "nowPlaying", label: qsTr("Playing"), icon: "nowPlaying", overlay: true, bottom: true },
            { id: "settings", label: qsTr("Settings"), icon: "settings", bottom: true }
        ]

        readonly property bool showSignIn: !Backend.designPreview && Backend.authState !== "connected"
            && !(everConnected && (Backend.authState === "reconnecting" || Backend.authState === "connecting"))

        // Push `item` as the section's detail page, or pop with null.
        function setDetail(sectionId, item) {
            const stacks = Object.assign({}, detailStacks)
            const stack = (stacks[sectionId] || []).slice()
            if (item === null) stack.pop()
            else if (stack.length === 0 || stack[stack.length - 1].uri !== item.uri) stack.push(item)
            stacks[sectionId] = stack.slice(-8)
            detailStacks = stacks
        }

        function activate(item, sectionId) {
            if (!item || !item.uri) return
            switch (item.kind) {
            case "album":
            case "playlist":
            case "artist":
                setDetail(sectionId, item)
                break
            case "track":
            case "episode":
                Backend.playUri(item.contextUri || "", item.uri)
                break
            case "collection":
                section = "library"
                break
            default:            // show etc.: play as a context
                Backend.playUri(item.uri, "")
            }
        }

        function playItem(item) {
            if (item.kind === "track" || item.kind === "episode") Backend.playUri(item.contextUri || "", item.uri)
            else Backend.playUri(item.uri, "")
        }

        function rememberFocus() {
            focusBeforeOverlay = window.activeFocusItem
        }
        function restoreFocus() {
            if (focusBeforeOverlay && focusBeforeOverlay.visible) focusBeforeOverlay.forceActiveFocus()
            else pages.forceActiveFocus()
            focusBeforeOverlay = null
        }

        function openNowPlaying() {
            if (nowPlayingOpen) return
            rememberFocus()
            nowPlayingOpen = true
            nowPlaying.forceActiveFocus()
        }
        function closeNowPlaying() {
            nowPlayingOpen = false
            restoreFocus()
        }
        function openQueue() {
            if (queueOpen) return
            if (!nowPlayingOpen) rememberFocus()
            queueOpen = true
        }
        function closeQueue() {
            queueOpen = false
            if (nowPlayingOpen) nowPlaying.forceActiveFocus()
            else restoreFocus()
        }

        function goBack() {
            if (showSignIn) return
            if (queueOpen) return closeQueue()
            if (nowPlayingOpen) return closeNowPlaying()
            if (details[section]) {
                setDetail(section, null)
                pages.forceActiveFocus()
                return
            }
            if (pages.activeFocus || miniPlayer.activeFocus) return rail.forceActiveFocus()
            if (section !== "home") section = "home"
        }

        // Global keys: media/controller shortcuts work from anywhere because
        // unhandled keys bubble up to here.
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_MediaTogglePlayPause:
            case Qt.Key_MediaPlay:
            case Qt.Key_MediaPause:
                if (!event.isAutoRepeat) Backend.togglePlay()
                break
            case Qt.Key_MediaNext:
                if (!event.isAutoRepeat) Backend.next()
                break
            case Qt.Key_MediaPrevious:
                if (!event.isAutoRepeat) Backend.previous()
                break
            case Qt.Key_VolumeUp:
                Backend.stepVolume(0.05)
                break
            case Qt.Key_VolumeDown:
                Backend.stepVolume(-0.05)
                break
            case Qt.Key_F2:
                if (!event.isAutoRepeat) nowPlayingOpen ? closeNowPlaying() : openNowPlaying()
                break
            case Qt.Key_F3:
                if (!event.isAutoRepeat) queueOpen ? closeQueue() : openQueue()
                break
            case Qt.Key_Escape:
            case Qt.Key_Back:
                if (!event.isAutoRepeat) goBack()
                break
            default:
                return
            }
            event.accepted = true
        }

        // ---- Background + ambient tint ---------------------------------------
        Rectangle { anchors.fill: parent; color: Theme.surface }
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.alpha(Theme.primaryContainer, 0.28) }
                GradientStop { position: 0.45; color: Theme.alpha(Theme.surface, 0) }
            }
        }

        NavRail {
            id: rail
            anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
            destinations: app.destinations
            current: app.section
            onNavigate: id => app.section = id
            onEnterContent: id => {
                if (id === "nowPlaying") app.openNowPlaying()
                else pages.forceActiveFocus()
            }
        }

        FocusScope {
            id: contentArea
            anchors { left: rail.right; right: parent.right; top: parent.top; bottom: parent.bottom }
            focus: true

            FocusScope {
                id: pages
                anchors.fill: parent
                focus: true

                SectionPage {
                    sectionId: "home"
                    HomeScreen {
                        id: home
                        focus: true
                        anchors.fill: parent
                        onItemActivated: item => app.activate(item, "home")
                        onPlayRequested: item => app.playItem(item)
                    }
                }
                SectionPage {
                    sectionId: "search"
                    SearchScreen {
                        focus: true
                        anchors.fill: parent
                        onItemActivated: item => app.activate(item, "search")
                    }
                }
                SectionPage {
                    sectionId: "library"
                    LibraryScreen {
                        id: library
                        focus: true
                        anchors.fill: parent
                        onItemActivated: item => app.activate(item, "library")
                    }
                }
                SectionPage {
                    sectionId: "settings"
                    SettingsScreen {
                        focus: true
                        anchors.fill: parent
                        onQuitRequested: Qt.quit()
                        onToastRequested: (level, message) => toast.show(level, message)
                    }
                }

                Keys.onDownPressed: event => {
                    if (miniPlayer.visible) miniPlayer.forceActiveFocus()
                    else event.accepted = false
                }
            }

            MiniPlayer {
                id: miniPlayer
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: Theme.safeH; rightMargin: Theme.safeH; bottomMargin: Theme.space.xl }
                visible: Backend.hasTrack && !app.nowPlayingOpen
                onOpenNowPlaying: app.openNowPlaying()
                Keys.onUpPressed: pages.forceActiveFocus()
            }

            Keys.onLeftPressed: rail.forceActiveFocus()
        }

        NowPlayingScreen {
            id: nowPlaying
            anchors.fill: parent
            z: 10
            opacity: app.nowPlayingOpen ? 1 : 0
            visible: opacity > 0
            enabled: app.nowPlayingOpen
            scale: app.nowPlayingOpen || Theme.reducedMotion ? 1 : 0.96
            Behavior on opacity { NumberAnimation { duration: Theme.motion.fadeLong } }
            Behavior on scale { NumberAnimation { duration: Theme.motion.slow; easing.type: Easing.BezierSpline; easing.bezierCurve: Theme.motion.emphasizedDecelerate } }
            onOpenQueue: app.openQueue()
        }

        QueuePanel {
            id: queuePanel
            anchors.fill: parent
            z: 20
            open: app.queueOpen
            onClosed: app.closeQueue()
        }

        // Connection-lost banner (the UI stays usable while reconnecting).
        Rectangle {
            z: 25
            visible: !app.showSignIn && (Backend.authState === "reconnecting" || Backend.authState === "connecting")
            anchors { top: parent.top; topMargin: Theme.space.l; horizontalCenter: parent.horizontalCenter }
            width: bannerRow.implicitWidth + Theme.space.xxxl
            height: Theme.dp(64)
            radius: height / 2
            color: Theme.tertiaryContainer
            Row {
                id: bannerRow
                anchors.centerIn: parent
                spacing: Theme.space.m
                LoadingIndicator { size: Theme.dp(30); running: parent.visible; color: Theme.tertiaryContainerContent; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Backend.authMessage || qsTr("Reconnecting to Spotify…")
                    color: Theme.tertiaryContainerContent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelLarge
                    font.weight: Font.DemiBold
                }
            }
        }

        SignInScreen {
            id: signIn
            anchors.fill: parent
            z: 30
            visible: app.showSignIn
            onVisibleChanged: if (visible) forceActiveFocus(); else pages.forceActiveFocus()
            Component.onCompleted: if (visible) forceActiveFocus()
            onQuitRequested: Qt.quit()
        }

        Toast {
            id: toast
            z: 40
            anchors { bottom: parent.bottom; bottomMargin: Theme.size.miniPlayer + Theme.space.xxxl }
        }

        Rectangle {
            z: 45
            visible: Backend.designPreview
            anchors { left: parent.left; bottom: parent.bottom; margins: Theme.space.l }
            width: previewText.implicitWidth + Theme.space.xxl
            height: Theme.dp(44)
            radius: height / 2
            color: Theme.tertiaryContainer
            Text {
                id: previewText
                anchors.centerIn: parent
                text: qsTr("Design preview · sample content")
                color: Theme.tertiaryContainerContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall
                font.weight: Font.Bold
            }
        }

        // ---- Keyboard equivalents of controller shortcuts (CONTROLS.md) ----
        Shortcut { sequence: "Ctrl+P"; enabled: !app.showSignIn; onActivated: Backend.togglePlay() }
        Shortcut { sequence: "Ctrl+Right"; enabled: !app.showSignIn; onActivated: Backend.next() }
        Shortcut { sequence: "Ctrl+Left"; enabled: !app.showSignIn; onActivated: Backend.previous() }
        Shortcut { sequence: "Ctrl+Up"; autoRepeat: true; enabled: !app.showSignIn; onActivated: Backend.stepVolume(0.05) }
        Shortcut { sequence: "Ctrl+Down"; autoRepeat: true; enabled: !app.showSignIn; onActivated: Backend.stepVolume(-0.05) }

        // ---- Backend wiring ------------------------------------------------
        Connections {
            target: SavedState
            function onConfirmed(uri, saved, kind) {
                const artist = kind === "artist"
                toast.show("info", saved ? (artist ? qsTr("Following") : qsTr("Added to your library"))
                                         : (artist ? qsTr("Unfollowed") : qsTr("Removed from your library")))
                library.reload()
            }
        }

        property string libraryStatus: "notConfigured"
        Connections {
            target: Backend
            function onLibraryAccessChanged() {
                const status = JSON.parse(Backend.libraryAccess).status
                if (status === "connected" && app.libraryStatus !== "connected") {
                    home.reload()
                    library.reload()
                }
                app.libraryStatus = status
            }
        }

        Connections {
            target: Backend
            function onNotify(level, message) { toast.show(level, message) }
            function onAuthStateChanged() {
                if (Backend.authState === "connected") {
                    const first = !app.everConnected
                    app.everConnected = true
                    home.reload()
                    library.reload()
                    if (first) pages.forceActiveFocus()
                }
            }
        }

        // Unconfirmed requests are reported after a timeout; ticks only
        // while something is pending.
        Timer {
            interval: 1000
            repeat: true
            running: Backend.pending !== "" && !Backend.designPreview
            onTriggered: Backend.expirePending()
        }

        Component.onCompleted: {
            if (Backend.designPreview) {
                Backend.authState = "connected"
                Backend.userName = "Preview"
                Backend.deviceName = "Bigify"
                Backend.hasTrack = true
                Backend.isPlaying = true
                Backend.playbackStatus = "playing"
                Backend.trackUri = "preview:track:0"
                Backend.trackTitle = "Sample Track With A Longer Title"
                Backend.trackArtists = "Sample Artist, Featured Artist"
                Backend.trackAlbum = "Sample Album"
                Backend.durationMs = 214000
                Backend.positionMs = 83000
                Backend.positionSampledAt = Date.now()
                Backend.repeatMode = "context"
                Backend.volume = 0.62
                if (window.dev.inputMode) Backend.inputMode = window.dev.inputMode
                if (window.dev.controller) Backend.controllerFamily = window.dev.controller
                app.everConnected = true
                home.reload()
                library.reload()
            }
            const start = window.dev.startScreen
            if (start === "nowPlaying") app.openNowPlaying()
            else if (start === "queue") { app.openNowPlaying(); app.openQueue() }
            else if (start === "rail") rail.forceActiveFocus()
            else if (start === "artist" && Backend.designPreview) setDetail("home", PreviewData.homeSections[3].items[0])
            else if (start === "album" && Backend.designPreview) setDetail("home", PreviewData.homeSections[4].items[0])
            else if (start && start !== "home") app.section = start
        }
    }

    // A page for one rail section, holding its root screen and an optional
    // album/playlist/artist detail page on top.
    component SectionPage: FocusScope {
        id: page
        required property string sectionId
        default property alias screen: rootHolder.data
        readonly property bool current: app.section === sectionId
        readonly property var detailItem: app.details[sectionId] || null

        anchors.fill: parent
        focus: current
        opacity: current ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: Theme.motion.fade } }

        FocusScope {
            id: rootHolder
            anchors.fill: parent
            visible: !page.detailItem
            focus: !page.detailItem
        }
        Loader {
            anchors.fill: parent
            active: page.detailItem !== null
            focus: active
            sourceComponent: page.detailItem && page.detailItem.kind === "artist" ? artistPage : collectionPage
        }
        Component {
            id: collectionPage
            CollectionScreen {
                item: page.detailItem
                focus: true
                onItemActivated: item => app.activate(item, page.sectionId)
            }
        }
        Component {
            id: artistPage
            ArtistScreen {
                item: page.detailItem
                focus: true
                onItemActivated: item => app.activate(item, page.sectionId)
            }
        }
    }

    // Developer screenshots (BIGIFY_SCREENSHOT=/path.png): capture, then quit.
    Timer {
        running: window.dev.screenshot !== ""
        interval: 3500
        onTriggered: window.contentItem.grabToImage(result => {
            result.saveToFile(window.dev.screenshot)
            Qt.quit()
        })
    }
}
