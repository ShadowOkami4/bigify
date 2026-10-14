import QtQuick
import Bigify

// Album or playlist detail: header with artwork and Play, then the tracks.
FocusScope {
    id: root
    property var item: ({})                 // the card that opened this page
    property var detail: null
    property string status: "loading"
    property string errorKind: ""
    property string errorMessage: ""

    readonly property var shown: detail ? detail.item : item
    signal itemActivated(var item)

    property string loadedUri: ""

    function load() {
        if (!item || !item.uri || item.uri === loadedUri) return
        loadedUri = item.uri
        detail = null
        status = "loading"
        if (Backend.designPreview) {
            detail = PreviewData.collectionFor(item)
            status = "ready"
            return
        }
        Backend.loadCollection(item.uri)
    }

    Component.onCompleted: load()
    onItemChanged: load()

    Connections {
        target: Backend
        function onCollectionLoaded(uri, json) {
            if (uri !== root.item.uri) return
            root.detail = JSON.parse(json)
            root.status = "ready"
        }
        function onCollectionFailed(uri, kind, message) {
            if (uri !== root.item.uri) return
            root.errorKind = kind
            root.errorMessage = message
            root.status = "error"
        }
    }

    Item {
        id: header
        anchors { top: parent.top; left: parent.left; right: parent.right; topMargin: Theme.safeV; leftMargin: Theme.safeH; rightMargin: Theme.safeH }
        height: Theme.dp(360)

        Artwork {
            id: art
            width: Theme.dp(360); height: width
            source: root.shown.artworkUrl || ""
            kind: root.shown.kind || "album"
            radius: Theme.shape.extraLargeIncreased
        }
        Column {
            anchors { left: art.right; leftMargin: Theme.space.xxxl; right: parent.right; bottom: art.bottom }
            spacing: Theme.space.m
            Text {
                text: (root.shown.kind === "playlist" ? qsTr("PLAYLIST") : qsTr("ALBUM"))
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
                font.pixelSize: Theme.type.displayMedium
                font.weight: Font.Bold
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: root.shown.subtitle || ""
                color: Theme.surfaceContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.titleMedium
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                visible: text !== ""
                text: root.detail ? root.detail.description : ""
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelLarge
                elide: Text.ElideRight
                maximumLineCount: 2
                wrapMode: Text.WordWrap
            }
            Item { width: 1; height: Theme.space.s }
            Row {
                spacing: Theme.space.l
                PillButton {
                    id: playButton
                    focus: true
                    text: qsTr("Play")
                    icon: "play"
                    busy: /\bload\b/.test(Backend.pending)
                    onClicked: Backend.playUri(root.shown.uri, "")
                    KeyNavigation.down: tracks.count > 0 ? tracks : null
                }
            }
        }
    }

    StatePanel {
        anchors { top: header.bottom; left: parent.left; right: parent.right; bottom: parent.bottom }
        visible: root.status !== "ready" || (root.detail && root.detail.tracks.length === 0)
        state_: root.status === "ready" ? "empty" : root.status
        title: root.status === "error" ? qsTr("Couldn't load this") : (root.status === "ready" ? qsTr("No tracks") : "")
        message: root.status === "error" ? root.errorMessage : ""
        errorKind: root.errorKind
        onRetry: { root.loadedUri = ""; root.load() }
    }

    ListView {
        id: tracks
        anchors { top: header.bottom; topMargin: Theme.space.xxl; left: parent.left; right: parent.right; bottom: parent.bottom }
        leftMargin: Theme.safeH
        rightMargin: Theme.safeH
        bottomMargin: Theme.safeV
        clip: true
        visible: root.status === "ready"
        model: root.detail ? root.detail.tracks : []
        spacing: Theme.space.s
        keyNavigationEnabled: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: Theme.dp(40)
        preferredHighlightEnd: height - Theme.dp(160)
        highlightMoveDuration: Theme.motion.medium
        highlightMoveVelocity: -1
        boundsBehavior: Flickable.StopAtBounds
        reuseItems: true
        KeyNavigation.up: playButton

        delegate: TrackRow {
            required property var modelData
            required property int index
            width: tracks.width - tracks.leftMargin - tracks.rightMargin
            item: modelData
            number: index + 1
            showArtwork: root.shown.kind === "playlist"
            focus: ListView.isCurrentItem
            onActivated: Backend.playUri(root.shown.uri, modelData.uri)
        }

        Keys.onUpPressed: event => {
            if (currentIndex === 0) playButton.forceActiveFocus()
            else decrementCurrentIndex()
        }

        footer: Item {
            width: tracks.width - tracks.leftMargin - tracks.rightMargin
            height: root.detail && root.detail.truncated ? Theme.dp(96) : Theme.dp(24)
            Text {
                visible: root.detail && root.detail.truncated
                anchors.centerIn: parent
                text: qsTr("Showing the first %1 of %2 songs. Play to hear them all.")
                      .arg(root.detail ? root.detail.tracks.length : 0)
                      .arg(root.detail ? root.detail.totalTracks : 0)
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelLarge
            }
        }
    }
}
