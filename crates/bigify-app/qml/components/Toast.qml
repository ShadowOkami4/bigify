import QtQuick
import Bigify

// Snackbar for transient feedback. Messages queue so none are lost; repeated
// identical messages collapse.
Item {
    id: root
    property var queue: []
    property string current: ""
    property string level: "info"

    anchors.horizontalCenter: parent.horizontalCenter
    width: Math.min(Theme.dp(1100), bubble.implicitWidth)
    height: bubble.height
    opacity: current !== "" ? 1 : 0
    visible: opacity > 0
    Behavior on opacity { NumberAnimation { duration: Theme.motion.fade } }

    function show(level, message) {
        if (message === current || queue.some(m => m.message === message)) return
        queue.push({ level: level, message: message })
        if (current === "") next()
    }
    function next() {
        const m = queue.shift()
        if (!m) { current = ""; return }
        level = m.level
        current = m.message
        timer.restart()
    }

    Timer {
        id: timer
        interval: root.level === "error" ? 5000 : 3000
        onTriggered: root.next()
    }

    Rectangle {
        id: bubble
        width: parent.width
        implicitWidth: row.implicitWidth + Theme.space.xxl * 2
        height: Theme.dp(84)
        radius: Theme.shape.extraLarge
        color: Theme.surfaceContent
        Row {
            id: row
            anchors.centerIn: parent
            spacing: Theme.space.l
            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: root.level === "error" ? "alert" : "check"
                color: Theme.surface
                size: Theme.dp(30)
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.current
                color: Theme.surface
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.titleSmall
                font.weight: Font.Medium
                elide: Text.ElideRight
                width: Math.min(implicitWidth, Theme.dp(950))
            }
        }
    }
    Accessible.role: Accessible.AlertMessage
    Accessible.name: current
}
