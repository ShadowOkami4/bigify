import QtQuick
import Bigify

// "Up next" side sheet. Read-only: Spotify's public interfaces let Bigify
// show the queue and add to it, but not reorder or remove items.
FocusScope {
    id: root
    property bool open: false
    property var queue: null
    property string status: "loading"
    property string errorKind: ""
    property string errorMessage: ""
    signal closed()

    visible: sheet.x < parent.width
    function refresh() {
        status = "loading"
        if (Backend.designPreview) {
            queue = PreviewData.queue
            status = "ready"
            return
        }
        Backend.loadQueue()
    }
    onOpenChanged: if (open) { refresh(); list.currentIndex = 0; forceActiveFocus() }

    // Refresh when the track changes while the sheet is open.
    Connections {
        target: Backend
        function onTrackUriChanged() { if (root.open) root.refresh() }
        function onQueueLoaded(json) { root.queue = JSON.parse(json); root.status = "ready" }
        function onQueueFailed(kind, message) { root.errorKind = kind; root.errorMessage = message; root.status = "error" }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.scrim
        opacity: root.open ? 0.5 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.motion.fade } }
        MouseArea { anchors.fill: parent; enabled: root.open; onClicked: root.closed() }
    }

    Rectangle {
        id: sheet
        width: Theme.dp(760)
        height: parent.height
        x: root.open ? parent.width - width : parent.width
        color: Theme.surfaceContainerLow
        radius: Theme.shape.extraExtraLarge
        Behavior on x { NumberAnimation { duration: Theme.motion.slow; easing.type: Easing.BezierSpline; easing.bezierCurve: Theme.motion.emphasizedDecelerate } }

        Text {
            id: header
            anchors { top: parent.top; topMargin: Theme.safeV; left: parent.left; leftMargin: Theme.space.xxxl }
            text: qsTr("Up next")
            color: Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.headlineLarge
            font.weight: Theme.weightDisplay
        }
        Column {
            id: note
            anchors { top: header.bottom; topMargin: Theme.space.s; left: header.left; right: parent.right; rightMargin: Theme.space.xxxl }
            spacing: Theme.space.s
            ButtonHint { action: "more"; label: qsTr("on any song adds it here") }
            Text {
                width: parent.width
                text: qsTr("Reordering isn't available through Spotify's public interfaces.")
                wrapMode: Text.WordWrap
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelMedium
            }
        }

        StatePanel {
            anchors { top: note.bottom; left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.space.xxl }
            visible: root.status !== "ready" || !root.queue || (!root.queue.current && root.queue.upcoming.length === 0)
            state_: root.status === "ready" ? "empty" : root.status
            icon: "queue"
            title: root.status === "error" ? qsTr("Couldn't load the queue") : (root.status === "ready" ? qsTr("The queue is empty") : "")
            message: root.status === "error" ? root.errorMessage : ""
            errorKind: root.errorKind
            onRetry: root.refresh()
        }

        ListView {
            id: list
            anchors { top: note.bottom; topMargin: Theme.space.xxl; left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: Theme.space.xxl; rightMargin: Theme.space.xxl }
            visible: root.status === "ready" && root.queue !== null
            focus: true
            clip: true
            spacing: Theme.space.s
            topMargin: Theme.focusGap + Theme.focusWidth + Theme.dp(4)
            bottomMargin: Theme.safeV
            keyNavigationEnabled: true
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: Theme.dp(40)
            preferredHighlightEnd: height - Theme.dp(200)
            highlightMoveDuration: Theme.motion.medium
            boundsBehavior: Flickable.StopAtBounds
            model: root.queue ? (root.queue.current ? [root.queue.current] : []).concat(root.queue.upcoming) : []
            delegate: FocusScope {
                required property var modelData
                required property int index
                width: list.width
                height: entry.height
                focus: ListView.isCurrentItem
              Column {
                id: entry
                width: parent.width
                spacing: Theme.space.s
                Text {
                    visible: index === 0 && root.queue.current !== null || index === (root.queue.current ? 1 : 0)
                    text: index === 0 && root.queue.current ? qsTr("Now playing") : qsTr("Next")
                    color: Theme.primary
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelLarge
                    font.weight: Font.Bold
                    topPadding: index === 0 ? 0 : Theme.space.l
                }
                TrackRow {
                    width: parent.width
                    focus: true
                    item: modelData
                    showQueueAction: false
                }
              }
            }
        }
    }

    Keys.onEscapePressed: root.closed()
    Keys.onBackPressed: root.closed()
    Keys.onPressed: event => { if (event.key === Qt.Key_F3) { root.closed(); event.accepted = true } }
}
