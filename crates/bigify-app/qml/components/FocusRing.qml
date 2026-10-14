import QtQuick
import Bigify

// High-visibility focus indicator: a thick light ring with a thin dark halo
// inside, so it reads on light artwork, dark surfaces and everything between.
// Place as a child of the focusable item; it follows the parent's shape.
Item {
    id: ring
    property bool shown: parent ? parent.activeFocus : false
    property real radius: Theme.shape.large
    property int gap: Theme.focusGap

    anchors.fill: parent
    anchors.margins: -(gap + Theme.focusWidth)
    opacity: shown ? 1 : 0
    visible: opacity > 0
    z: 100

    Behavior on opacity { NumberAnimation { duration: Theme.motion.fade } }

    Rectangle {
        anchors.fill: parent
        radius: ring.radius + ring.gap + Theme.focusWidth
        color: "transparent"
        border.width: Theme.focusWidth
        border.color: Theme.focusColor
    }
    Rectangle {
        anchors.fill: parent
        anchors.margins: Theme.focusWidth
        radius: ring.radius + ring.gap
        color: "transparent"
        border.width: Math.max(1, Theme.dp(1.5))
        border.color: Theme.alpha(Theme.scrim, 0.55)
    }
}
