import QtQuick
import QtQuick.Effects
import Bigify

// Lyrics for Now Playing. Synced lyrics follow the confirmed playback
// position: the current line is highlighted and kept in view. Plain lyrics
// can be scrolled with Up/Down when focused.
FocusScope {
    id: root
    property var lyrics: null
    property string status: "idle"      // idle | loading | ready | error
    property string message: ""
    property real position: 0           // ms, from PositionClock

    readonly property bool synced: !!lyrics && lyrics.synced
    readonly property var lines: lyrics ? lyrics.lines : []
    readonly property bool hasLines: lines.length > 0

    // Last line whose timestamp has been reached (binary search; lines are
    // sorted). A small lead makes the highlight feel on time.
    readonly property int current: {
        if (!synced || !hasLines) return -1
        const t = position + 250
        let lo = 0, hi = lines.length - 1, found = -1
        while (lo <= hi) {
            const mid = (lo + hi) >> 1
            if (lines[mid].timeMs <= t) { found = mid; lo = mid + 1 } else hi = mid - 1
        }
        return found
    }

    activeFocusOnTab: !synced && hasLines

    // Soft fade at the top and bottom edges instead of a hard clip.
    Item {
        id: fadeMask
        anchors.fill: list
        visible: false
        layer.enabled: true
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0.0; color: "transparent" }
                GradientStop { position: 0.14; color: "black" }
                GradientStop { position: 0.86; color: "black" }
                GradientStop { position: 1.0; color: "transparent" }
            }
        }
    }
    ListView {
        id: list
        anchors.fill: parent
        clip: true
        // Drawn through the fade mask above.
        layer.enabled: true
        layer.effect: MultiEffect {
            maskEnabled: true
            maskSource: fadeMask
            maskThresholdMin: 0.5
            maskSpreadAtMin: 1.0
        }
        visible: root.status === "ready" && root.hasLines
        model: root.lines
        spacing: Theme.space.l
        interactive: !root.synced
        focus: true
        keyNavigationEnabled: !root.synced
        // Synced lyrics follow playback; plain lyrics follow the D-pad.
        Binding on currentIndex {
            when: root.synced
            value: Math.max(0, root.current)
        }
        highlightFollowsCurrentItem: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: height * 0.3
        preferredHighlightEnd: height * 0.3 + Theme.dp(80)
        highlightMoveDuration: Theme.reducedMotion ? 0 : 450
        highlightMoveVelocity: -1
        boundsBehavior: Flickable.StopAtBounds
        topMargin: height * 0.3
        bottomMargin: height * 0.5

        delegate: Text {
            required property var modelData
            required property int index
            readonly property bool isCurrent: root.synced && index === root.current
            width: list.width
            text: modelData.text === "" ? "♪" : modelData.text
            wrapMode: Text.WordWrap
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.headlineSmall
            font.weight: isCurrent ? Font.Bold : Font.DemiBold
            color: !root.synced ? Theme.surfaceContent
                 : isCurrent ? Theme.surfaceContent
                 : Theme.alpha(Theme.surfaceContentVariant, index < root.current ? 0.45 : 0.75)
            Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
        }
    }

    Column {
        anchors.centerIn: parent
        width: parent.width
        spacing: Theme.space.l
        visible: !list.visible
        LoadingIndicator {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.status === "loading"
            running: visible
            size: Theme.dp(56)
        }
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            visible: root.status !== "loading"
            text: root.lyrics && root.lyrics.instrumental ? qsTr("Instrumental — no lyrics")
                : root.message !== "" ? root.message : qsTr("No lyrics found for this song.")
            color: Theme.surfaceContentVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.titleMedium
        }
    }

    Text {
        anchors { right: parent.right; bottom: parent.bottom }
        visible: root.status === "ready"
        text: qsTr("Lyrics: %1").arg(root.lyrics ? root.lyrics.source : "")
        color: Theme.alpha(Theme.surfaceContentVariant, 0.8)
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelSmall
    }

    FocusRing { shown: root.activeFocus && !root.synced; radius: Theme.shape.large }
}
