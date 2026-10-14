import QtQuick
import Bigify

// Library: saved playlists, albums, followed artists and Liked Songs.
// Pages of 50 load on demand as focus approaches the end of the grid, so
// large libraries never load (or render) all at once.
FocusScope {
    id: root
    signal itemActivated(var item)

    readonly property var categories: [
        { id: "playlists", label: qsTr("Playlists"), icon: "playlist" },
        { id: "albums", label: qsTr("Albums"), icon: "album" },
        { id: "artists", label: qsTr("Artists"), icon: "artist" },
        { id: "likedSongs", label: qsTr("Liked Songs"), icon: "heart" }
    ]
    property string kind: "playlists"
    property var pages: ({})        // kind -> { status, nextOffset, total, errorKind, errorMessage }
    readonly property var page: pages[kind] || { status: "idle" }

    ListModel { id: playlistsModel }
    ListModel { id: albumsModel }
    ListModel { id: artistsModel }
    ListModel { id: likedModel }
    readonly property var models: ({ playlists: playlistsModel, albums: albumsModel, artists: artistsModel, likedSongs: likedModel })
    readonly property ListModel model: models[kind]

    // ListModel roles are typed by the first row; never append nulls.
    function normalise(it) {
        return {
            uri: it.uri, kind: it.kind, title: it.title || "", subtitle: it.subtitle || "",
            artworkUrl: it.artworkUrl || "", durationMs: it.durationMs || 0,
            explicit: it.explicit === true, contextUri: it.contextUri || ""
        }
    }

    function setPage(k, values) {
        const copy = Object.assign({}, pages)
        copy[k] = Object.assign({}, pages[k] || {}, values)
        pages = copy
    }

    function ensureLoaded() {
        if (!pages[kind] || pages[kind].status === "idle") loadMore(true)
    }

    function loadMore(reset) {
        const p = pages[kind] || {}
        if (p.status === "loading") return
        if (!reset && (p.nextOffset === undefined || p.nextOffset === null)) return
        const offset = reset ? 0 : p.nextOffset
        if (reset) models[kind].clear()
        if (Backend.designPreview) {
            const data = PreviewData.library(kind)
            for (const it of data.items) models[kind].append(normalise(it))
            setPage(kind, { status: "ready", nextOffset: null, total: data.total })
            return
        }
        setPage(kind, { status: "loading" })
        Backend.loadLibrary(kind, offset)
    }

    function reload() {
        for (const c of categories) setPage(c.id, { status: "idle", nextOffset: null })
        ensureLoaded()
    }

    onKindChanged: ensureLoaded()

    Connections {
        target: Backend
        function onLibraryLoaded(k, offset, json) {
            const p = JSON.parse(json)
            const m = root.models[k]
            if (!m) return
            if (offset === 0) m.clear()
            for (const it of p.items) m.append(root.normalise(it))
            const next = p.total !== null && p.offset + p.items.length < p.total && p.items.length > 0
                         ? p.offset + p.items.length : null
            root.setPage(k, { status: "ready", nextOffset: next, total: p.total })
        }
        function onLibraryFailed(k, offset, errorKind, message) {
            root.setPage(k, { status: root.models[k] && root.models[k].count > 0 ? "ready" : "error",
                              errorKind: errorKind, errorMessage: message, nextOffset: null })
        }
    }

    Text {
        id: title
        x: Theme.safeH
        y: Theme.safeV
        text: qsTr("Your Library")
        color: Theme.surfaceContent
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.displaySmall
        font.weight: Theme.weightDisplay
    }

    ListView {
        id: chips
        anchors { top: title.bottom; topMargin: Theme.space.xxl; left: parent.left; right: parent.right }
        leftMargin: Theme.safeH
        height: Theme.dp(80)
        orientation: ListView.Horizontal
        spacing: Theme.space.l
        interactive: false
        focus: true
        model: root.categories
        keyNavigationEnabled: true
        currentIndex: root.categories.findIndex(c => c.id === root.kind)
        delegate: Chip {
            required property var modelData
            required property int index
            anchors.verticalCenter: parent ? parent.verticalCenter : undefined
            text: modelData.label
            icon: modelData.icon
            selected: root.kind === modelData.id
            focus: ListView.isCurrentItem
            onActiveFocusChanged: if (activeFocus) root.kind = modelData.id
            onClicked: root.kind = modelData.id
        }
        KeyNavigation.down: root.kind === "likedSongs" ? trackList : grid
    }

    Text {
        anchors { top: chips.bottom; topMargin: Theme.space.l; left: parent.left; leftMargin: Theme.safeH }
        visible: root.page.total !== undefined && root.page.total !== null && root.page.status === "ready"
        text: root.page.total === 1 ? qsTr("1 item") : qsTr("%1 items").arg(root.page.total)
        color: Theme.surfaceContentVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelLarge
    }

    Item {
        id: content
        anchors { top: chips.bottom; topMargin: Theme.dp(64); left: parent.left; right: parent.right; bottom: parent.bottom }

        StatePanel {
            anchors.fill: parent
            visible: (root.page.status === "loading" && root.model.count === 0)
                     || root.page.status === "error"
                     || (root.page.status === "ready" && root.model.count === 0)
            state_: root.page.status === "ready" ? "empty" : root.page.status
            icon: root.categories.find(c => c.id === root.kind).icon
            title: root.page.status === "error" ? qsTr("Couldn't load your library")
                 : root.page.status === "ready" ? qsTr("Nothing saved here yet") : ""
            message: root.page.status === "error" ? (root.page.errorMessage || "")
                   : root.page.status === "ready" ? qsTr("Things you save in Spotify appear here.") : ""
            errorKind: root.page.errorKind || ""
            onRetry: root.loadMore(true)
        }

        GridView {
            id: grid
            anchors.fill: parent
            anchors.leftMargin: Theme.safeH
            visible: root.kind !== "likedSongs" && root.model.count > 0
            clip: true
            model: root.kind !== "likedSongs" ? root.model : null
            cellWidth: Theme.size.card + Theme.space.xxl
            cellHeight: Theme.size.card + Theme.dp(140)
            topMargin: Theme.dp(16)
            bottomMargin: Theme.safeV + Theme.size.miniPlayer
            keyNavigationEnabled: true
            highlightRangeMode: GridView.ApplyRange
            preferredHighlightBegin: Theme.dp(16)
            preferredHighlightEnd: height - cellHeight - Theme.size.miniPlayer
            highlightMoveDuration: Theme.motion.medium
            boundsBehavior: Flickable.StopAtBounds
            cacheBuffer: cellHeight * 2
            reuseItems: true
            onCurrentIndexChanged: if (currentIndex >= count - 14) root.loadMore(false)
            Keys.onUpPressed: event => {
                if (currentIndex < Math.floor(width / cellWidth)) chips.forceActiveFocus()
                else moveCurrentIndexUp()
            }

            delegate: Item {
                id: cell
                required property var model
                required property int index
                width: grid.cellWidth
                height: grid.cellHeight
                focus: GridView.isCurrentItem
                MediaCard {
                    focus: true
                    item: cell.model
                    onActivated: root.itemActivated(cell.model)
                }
            }
        }

        ListView {
            id: trackList
            anchors.fill: parent
            anchors.leftMargin: Theme.safeH
            anchors.rightMargin: Theme.safeH
            visible: root.kind === "likedSongs" && root.model.count > 0
            clip: true
            model: root.kind === "likedSongs" ? root.model : null
            spacing: Theme.space.s
            bottomMargin: Theme.safeV + Theme.size.miniPlayer
            keyNavigationEnabled: true
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: Theme.dp(40)
            preferredHighlightEnd: height - Theme.dp(260)
            highlightMoveDuration: Theme.motion.medium
            highlightMoveVelocity: -1
            boundsBehavior: Flickable.StopAtBounds
            reuseItems: true
            onCurrentIndexChanged: if (currentIndex >= count - 20) root.loadMore(false)
            Keys.onUpPressed: event => {
                if (currentIndex === 0) chips.forceActiveFocus()
                else decrementCurrentIndex()
            }
            delegate: TrackRow {
                required property var model
                required property int index
                width: trackList.width
                item: model
                focus: ListView.isCurrentItem
                onActivated: Backend.playUri("spotify:collection", model.uri)
            }
            footer: Item {
                width: trackList.width
                height: Theme.dp(100)
                LoadingIndicator {
                    anchors.centerIn: parent
                    visible: root.page.status === "loading"
                    running: visible
                }
            }
        }
    }
}
