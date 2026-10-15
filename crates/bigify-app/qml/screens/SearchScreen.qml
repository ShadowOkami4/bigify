import QtQuick
import Bigify

// Search: query field + D-pad keyboard on the left, results on the right.
// Typing is debounced; stale responses are ignored; the query and results
// survive leaving and returning to the screen.
FocusScope {
    id: root
    signal itemActivated(var item)

    property string query: ""
    property int requestId: 0
    property var results: null
    property string status: "idle"   // idle | loading | ready | error
    property string errorKind: ""
    property string errorMessage: ""

    readonly property var groups: results ? [
        { id: "tracks", title: qsTr("Songs"), items: results.tracks, layout: "tracks" },
        { id: "artists", title: qsTr("Artists"), items: results.artists, layout: "cards" },
        { id: "albums", title: qsTr("Albums"), items: results.albums, layout: "cards" },
        { id: "playlists", title: qsTr("Playlists"), items: results.playlists, layout: "cards" }
    ].filter(g => g.items && g.items.length > 0) : []

    onQueryChanged: debounce.restart()

    Timer {
        id: debounce
        interval: 450
        onTriggered: root.runSearch()
    }

    function runSearch() {
        const q = root.query.trim()
        if (q.length === 0) {
            root.results = null
            root.status = "idle"
            return
        }
        if (Backend.designPreview) {
            root.results = PreviewData.searchResults(q)
            root.status = "ready"
            return
        }
        root.status = "loading"
        root.requestId = Backend.search(q)
    }

    Connections {
        target: Backend
        function onSearchFinished(id, json) {
            if (id !== root.requestId) return
            root.results = JSON.parse(json)
            root.status = "ready"
        }
        function onSearchFailed(id, kind, message) {
            if (id !== root.requestId) return
            root.errorKind = kind
            root.errorMessage = message
            root.status = "error"
        }
    }

    // Developer start screen: sample results with focus on the first one.
    function previewResults() {
        query = "sample"
        runSearch()
        focusResults()
    }

    readonly property bool hasResults: root.status === "ready" && root.groups.length > 0
    function focusResults() {
        if (!hasResults) return false
        resultsList.forceActiveFocus()
        return true
    }

    // Hardware keyboards (and Steam's on-screen keyboard) type directly.
    // Y (gamepad) or Tab jumps from the keyboard straight to the results.
    Keys.onPressed: event => {
        if ((event.key === Qt.Key_Menu || event.key === Qt.Key_Tab) && keyboard.activeFocus) {
            event.accepted = root.focusResults()
        } else if (event.key === Qt.Key_Backspace) {
            root.query = root.query.slice(0, -1)
            event.accepted = true
        } else if (event.text && event.text.length === 1 && event.text >= " "
                   && !(event.modifiers & (Qt.ControlModifier | Qt.AltModifier))) {
            root.query += event.text
            event.accepted = true
        }
    }

    Column {
        id: inputColumn
        x: Theme.safeH
        y: Theme.safeV
        width: keyboard.implicitWidth
        spacing: Theme.space.xxl

        Text {
            text: qsTr("Search")
            color: Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.displaySmall
            font.weight: Theme.weightDisplay
        }

        Rectangle {
            id: field
            width: parent.width
            height: Theme.dp(88)
            radius: height / 2
            color: Theme.surfaceContainerHigh
            border.width: keyboard.activeFocus ? Theme.dp(3) : 0
            border.color: Theme.primary
            Accessible.role: Accessible.EditableText
            Accessible.name: qsTr("Search query")
            Row {
                anchors { left: parent.left; leftMargin: Theme.space.xxl; verticalCenter: parent.verticalCenter }
                spacing: Theme.space.l
                Icon { name: "search"; size: Theme.dp(34); color: Theme.surfaceContentVariant; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: field.width - Theme.dp(140)
                    text: root.query.length > 0 ? root.query : qsTr("Songs, artists, albums")
                    color: root.query.length > 0 ? Theme.surfaceContent : Theme.surfaceContentVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.titleMedium
                    elide: Text.ElideLeft
                }
            }
            Rectangle {
                // Caret
                visible: keyboard.activeFocus
                anchors { right: parent.right; rightMargin: Theme.space.xxl; verticalCenter: parent.verticalCenter }
                width: Theme.dp(3); height: Theme.dp(36)
                color: Theme.primary
                SequentialAnimation on opacity {
                    running: keyboard.activeFocus && !Theme.reducedMotion
                    loops: Animation.Infinite
                    NumberAnimation { to: 0; duration: 500 }
                    NumberAnimation { to: 1; duration: 500 }
                }
            }
        }

        OnScreenKeyboard {
            id: keyboard
            focus: true
            onTyped: t => root.query += t
            onBackspace: root.query = root.query.slice(0, -1)
            onCleared: root.query = ""
            Keys.onRightPressed: event => { event.accepted = root.focusResults() }
        }

        Text {
            width: parent.width
            text: qsTr("Spotify returns up to 10 results per category.")
            color: Theme.surfaceContentVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall
            wrapMode: Text.WordWrap
            visible: root.status === "ready"
        }

        ButtonHint {
            visible: root.hasResults && keyboard.activeFocus
            action: "results"
            label: qsTr("Go to results")
        }
    }

    Item {
        id: resultsArea
        // Room for the focus lift and ring of the first card/row.
        readonly property real inset: Theme.focusGap + Theme.focusWidth + Theme.dp(16)
        anchors { left: inputColumn.right; leftMargin: Theme.space.xxxl - inset; right: parent.right; top: parent.top; bottom: parent.bottom }

        StatePanel {
            anchors.fill: parent
            visible: root.status !== "ready" || root.groups.length === 0
            state_: root.status === "loading" ? "loading" : (root.status === "error" ? "error" : "empty")
            icon: "search"
            title: root.status === "idle" ? qsTr("Find something to play")
                 : root.status === "error" ? qsTr("Search failed")
                 : root.status === "ready" ? qsTr("No results for “%1”").arg(root.query) : ""
            message: root.status === "idle" ? qsTr("Type with the keyboard on the left, or with a physical keyboard.")
                   : root.status === "error" ? root.errorMessage : ""
            errorKind: root.errorKind
            onRetry: root.runSearch()
        }

        ListView {
            id: resultsList
            anchors.fill: parent
            anchors.topMargin: Theme.safeV
            visible: root.hasResults
            clip: true
            model: root.groups
            spacing: Theme.space.xxxl
            keyNavigationEnabled: true
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: 0
            preferredHighlightEnd: height * 0.4
            highlightMoveDuration: Theme.motion.slow
            highlightMoveVelocity: -1
            boundsBehavior: Flickable.StopAtBounds
            bottomMargin: Theme.safeV + Theme.size.miniPlayer

            Keys.onLeftPressed: keyboard.forceActiveFocus()

            delegate: Loader {
                id: groupLoader
                required property var modelData
                width: resultsList.width
                focus: ListView.isCurrentItem
                sourceComponent: modelData.layout === "tracks" ? tracksComponent : cardsComponent

                Component {
                    id: cardsComponent
                    Shelf {
                        focus: true
                        title: groupLoader.modelData.title
                        items: groupLoader.modelData.items
                        cardWidth: Theme.dp(220)
                        leftInset: resultsArea.inset
                        onItemActivated: (item, i) => root.itemActivated(item)
                    }
                }
                Component {
                    id: tracksComponent
                    FocusScope {
                        focus: true
                        height: tracksHeader.height + Theme.space.l + trackList.height
                        Text {
                            id: tracksHeader
                            x: resultsArea.inset
                            text: groupLoader.modelData.title
                            color: parent.activeFocus ? Theme.surfaceContent : Theme.surfaceContentVariant
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.headlineSmall
                            font.weight: Theme.weightDisplay
                        }
                        ListView {
                            id: trackList
                            anchors { top: tracksHeader.bottom; topMargin: Theme.space.l; left: parent.left; leftMargin: resultsArea.inset; right: parent.right; rightMargin: Theme.safeH }
                            focus: true
                            interactive: false
                            height: Math.min(count, 5) * (Theme.size.trackRow + spacing)
                            spacing: Theme.space.s
                            model: groupLoader.modelData.items.slice(0, 5)
                            keyNavigationEnabled: true
                            delegate: TrackRow {
                                required property var modelData
                                width: trackList.width
                                item: modelData
                                focus: ListView.isCurrentItem
                                onActivated: root.itemActivated(modelData)
                            }
                        }
                    }
                }
            }
        }
    }
}
