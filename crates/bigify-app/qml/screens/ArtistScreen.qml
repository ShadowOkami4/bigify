import QtQuick
import Bigify

// Artist page: portrait and Play, popular tracks, albums, singles & EPs.
// Rows are a vertical list like Home, so Up/Down move between them and each
// row remembers its position.
FocusScope {
    id: root
    property var item: ({})
    property var detail: null
    property string status: "loading"
    property string errorKind: ""
    property string errorMessage: ""
    property string loadedUri: ""
    signal itemActivated(var item)

    readonly property var shown: detail ? detail.item : item
    readonly property var rows: {
        const r = [{ kind: "header" }]
        if (!detail) return r
        if (detail.topTracks.length > 0) r.push({ kind: "tracks", title: qsTr("Popular"), items: detail.topTracks })
        if (detail.albums.length > 0) r.push({ kind: "cards", title: qsTr("Albums"), items: detail.albums })
        if (detail.singles.length > 0) r.push({ kind: "cards", title: qsTr("Singles & EPs"), items: detail.singles })
        return r
    }

    function load() {
        if (!item || !item.uri || item.uri === loadedUri) return
        loadedUri = item.uri
        detail = null
        status = "loading"
        if (Backend.designPreview) {
            detail = PreviewData.artistFor(item)
            status = "ready"
            return
        }
        Backend.loadArtist(item.uri)
    }
    function retry() { loadedUri = ""; load() }

    Component.onCompleted: load()
    onItemChanged: load()

    Connections {
        target: Backend
        function onArtistLoaded(uri, json) {
            if (uri !== root.item.uri) return
            root.detail = JSON.parse(json)
            root.status = "ready"
        }
        function onArtistFailed(uri, kind, message) {
            if (uri !== root.item.uri) return
            root.errorKind = kind
            root.errorMessage = message
            root.status = "error"
        }
    }

    ListView {
        id: list
        anchors.fill: parent
        anchors.topMargin: Theme.safeV
        focus: true
        clip: true
        model: root.rows
        spacing: Theme.space.xxxl
        keyNavigationEnabled: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: 0
        preferredHighlightEnd: height * 0.45
        highlightMoveDuration: Theme.motion.slow
        highlightMoveVelocity: -1
        boundsBehavior: Flickable.StopAtBounds
        bottomMargin: Theme.safeV + Theme.size.miniPlayer
        cacheBuffer: Theme.dp(1200)

        delegate: Loader {
            id: rowLoader
            required property var modelData
            required property int index
            width: list.width
            focus: ListView.isCurrentItem
            sourceComponent: modelData.kind === "header" ? headerComponent
                           : modelData.kind === "tracks" ? tracksComponent : shelfComponent

            Component {
                id: headerComponent
                FocusScope {
                    focus: true
                    height: Theme.dp(320)
                    Artwork {
                        id: portrait
                        x: Theme.safeH
                        width: Theme.dp(300); height: width
                        circular: true
                        kind: "artist"
                        source: root.shown.artworkUrl || ""
                    }
                    Column {
                        anchors { left: portrait.right; leftMargin: Theme.space.xxxl; right: parent.right; rightMargin: Theme.safeH; verticalCenter: portrait.verticalCenter }
                        spacing: Theme.space.l
                        Text {
                            text: qsTr("ARTIST")
                            color: Theme.primary
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.labelLarge
                            font.weight: Font.Bold
                            font.letterSpacing: Theme.dp(2)
                        }
                        Text {
                            width: parent.width
                            text: root.shown.title || ""
                            color: Theme.surfaceContent
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.displayLarge
                            font.weight: Font.Bold
                            elide: Text.ElideRight
                        }
                        Row {
                            spacing: Theme.space.l
                            PillButton {
                                focus: true
                                text: qsTr("Play")
                                icon: "play"
                                busy: /\bload\b/.test(Backend.pending)
                                onClicked: Backend.playUri(root.shown.uri, "")
                            }
                            LoadingIndicator {
                                visible: root.status === "loading"
                                running: visible
                                size: Theme.dp(48)
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }
                }
            }
            Component {
                id: tracksComponent
                FocusScope {
                    focus: true
                    height: tracksHeader.height + Theme.space.l + trackList.height
                    Text {
                        id: tracksHeader
                        x: Theme.safeH
                        text: rowLoader.modelData.title
                        color: parent.activeFocus ? Theme.surfaceContent : Theme.surfaceContentVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.headlineSmall
                        font.weight: Theme.weightDisplay
                    }
                    ListView {
                        id: trackList
                        anchors { top: tracksHeader.bottom; topMargin: Theme.space.l; left: parent.left; right: parent.right; leftMargin: Theme.safeH; rightMargin: Theme.safeH }
                        focus: true
                        interactive: false
                        height: count * (Theme.size.trackRow + spacing)
                        spacing: Theme.space.s
                        model: rowLoader.modelData.items
                        keyNavigationEnabled: true
                        delegate: TrackRow {
                            required property var modelData
                            required property int index
                            width: trackList.width
                            item: modelData
                            number: index + 1
                            focus: ListView.isCurrentItem
                            // Plays the artist context starting at this track.
                            onActivated: Backend.playUri(root.shown.uri, modelData.uri)
                        }
                    }
                }
            }
            Component {
                id: shelfComponent
                Shelf {
                    focus: true
                    title: rowLoader.modelData.title
                    items: rowLoader.modelData.items
                    leftInset: Theme.safeH
                    onItemActivated: (item, i) => root.itemActivated(item)
                }
            }
        }
    }

    StatePanel {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; top: parent.top; topMargin: Theme.dp(420) }
        visible: root.status === "error" || (root.status === "ready" && root.rows.length === 1)
        state_: root.status === "error" ? "error" : "empty"
        icon: "artist"
        title: root.status === "error" ? qsTr("Couldn't load this artist") : qsTr("No music to show")
        message: root.status === "error" ? root.errorMessage : ""
        errorKind: root.errorKind
        onRetry: root.retry()
    }
}
