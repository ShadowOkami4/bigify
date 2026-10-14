import QtQuick
import Bigify

// Small animated bars marking the item that is currently playing. Static
// when paused or with reduced motion.
Row {
    id: root
    property real size: Theme.dp(24)
    property color color: Theme.primary
    readonly property bool animate: Backend.isPlaying && !Theme.reducedMotion && visible

    spacing: size * 0.12
    height: size
    Repeater {
        model: 3
        Rectangle {
            required property int index
            width: root.size * 0.22
            radius: width / 2
            color: root.color
            anchors.bottom: parent.bottom
            height: root.size * 0.5
            SequentialAnimation on height {
                running: root.animate
                loops: Animation.Infinite
                PauseAnimation { duration: index * 120 }
                NumberAnimation { to: root.size; duration: 320; easing.type: Easing.InOutSine }
                NumberAnimation { to: root.size * 0.3; duration: 380; easing.type: Easing.InOutSine }
                NumberAnimation { to: root.size * 0.5; duration: 260; easing.type: Easing.InOutSine }
            }
        }
    }
}
