import QtQuick
import QtQuick.Shapes
import Bigify

// Material 3 Expressive wavy progress indicator. The played portion is a
// gentle wave while music plays and flattens when paused; the remaining
// track is a flat line. Focusable for seeking (Left/Right = ±10 s).
Item {
    id: root
    property real value: 0          // 0..1
    property bool playing: false
    property bool seekable: true
    property color activeColor: Theme.primary
    property color trackColor: Theme.alpha(Theme.surfaceContent, 0.22)
    signal seekRequested(real delta)

    implicitHeight: Theme.dp(40)
    activeFocusOnTab: seekable
    Accessible.role: Accessible.Slider
    Accessible.name: qsTr("Playback position")

    property real stroke: Theme.dp(activeFocus ? 8 : 6)
    readonly property real amplitude: (playing && !Theme.reducedMotion) ? Theme.dp(5) : 0
    readonly property real wavelength: Theme.dp(44)
    property real phase: 0
    property real shownAmplitude: amplitude
    Behavior on shownAmplitude { NumberAnimation { duration: Theme.motion.slow; easing.type: Easing.OutCubic } }
    Behavior on stroke { NumberAnimation { duration: Theme.motion.quick } }

    NumberAnimation on phase {
        running: root.playing && root.visible && !Theme.reducedMotion
        from: 0; to: Math.PI * 2
        duration: 1800
        loops: Animation.Infinite
    }

    readonly property real split: Math.max(0, Math.min(1, value)) * width
    readonly property real gap: Theme.dp(8)

    function wavePath(x0, x1, cy, amp, ph) {
        if (x1 <= x0) return ""
        const step = Math.max(2, Theme.dp(3))
        let d = "M" + x0.toFixed(1) + " " + (cy + amp * Math.sin(x0 / wavelength * Math.PI * 2 - ph)).toFixed(2)
        for (let x = x0 + step; x < x1; x += step)
            d += " L" + x.toFixed(1) + " " + (cy + amp * Math.sin(x / wavelength * Math.PI * 2 - ph)).toFixed(2)
        d += " L" + x1.toFixed(1) + " " + (cy + amp * Math.sin(x1 / wavelength * Math.PI * 2 - ph)).toFixed(2)
        return d
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        // Remaining track
        ShapePath {
            strokeColor: root.trackColor
            strokeWidth: root.stroke
            capStyle: ShapePath.RoundCap
            fillColor: "transparent"
            startX: Math.min(root.width, root.split + root.gap + root.stroke / 2)
            startY: root.height / 2
            PathLine { x: root.width - root.stroke / 2; y: root.height / 2 }
        }
        // Played portion
        ShapePath {
            strokeColor: root.activeColor
            strokeWidth: root.stroke
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            fillColor: "transparent"
            PathSvg {
                path: root.split > root.stroke
                      ? root.wavePath(root.stroke / 2, root.split - root.stroke / 2, root.height / 2, root.shownAmplitude, root.phase)
                      : ""
            }
        }
    }

    // Stop indicator at the end of the track (M3).
    Rectangle {
        width: root.stroke; height: width; radius: width / 2
        anchors.verticalCenter: parent.verticalCenter
        x: root.width - width
        color: root.activeColor
    }
    // Handle, shown while focused for seeking.
    Rectangle {
        visible: root.activeFocus
        width: Theme.dp(6); height: Theme.dp(36); radius: width / 2
        anchors.verticalCenter: parent.verticalCenter
        x: root.split - width / 2
        color: root.activeColor
    }

    FocusRing { shown: root.activeFocus; radius: Theme.shape.large; gap: Theme.dp(10) }

    Keys.onLeftPressed: event => { if (seekable) { root.seekRequested(-10000); event.accepted = true } else event.accepted = false }
    Keys.onRightPressed: event => { if (seekable) { root.seekRequested(10000); event.accepted = true } else event.accepted = false }
}
