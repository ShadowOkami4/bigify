import QtQuick
import Bigify

// Transient volume pill. There is no on-screen volume control (volume keys,
// Ctrl+Up/Down or a phone change it), so this briefly shows the confirmed level
// whenever it changes, then fades away.
Item {
    id: root
    width: Theme.dp(360)
    height: Theme.dp(72)
    opacity: shown ? 1 : 0
    visible: opacity > 0
    property bool shown: false
    // Ignore the level reported while the session starts up.
    property bool armed: false

    Accessible.role: Accessible.ProgressBar
    Accessible.name: qsTr("Volume %1 percent").arg(Math.round(Backend.volume * 100))

    Behavior on opacity { NumberAnimation { duration: Theme.motion.fade } }

    Timer { interval: 2500; running: true; onTriggered: root.armed = true }
    Timer { id: hideTimer; interval: 1600; onTriggered: root.shown = false }
    Connections {
        target: Backend
        function onVolumeChanged() {
            if (!root.armed) return
            root.shown = true
            hideTimer.restart()
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: Theme.surfaceContainerHighest
        border.width: Theme.dp(1)
        border.color: Theme.alpha(Theme.outlineVariant, 0.6)
    }
    Row {
        anchors { fill: parent; leftMargin: Theme.space.xl; rightMargin: Theme.space.xl }
        spacing: Theme.space.l
        Icon {
            anchors.verticalCenter: parent.verticalCenter
            name: Backend.volume <= 0.001 ? "volumeDown" : "volumeUp"
            size: Theme.dp(34)
            color: Theme.surfaceContent
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - Theme.dp(34) - percent.width - 2 * parent.spacing
            height: Theme.dp(10)
            radius: height / 2
            color: Theme.alpha(Theme.surfaceContent, 0.2)
            Rectangle {
                width: Math.max(height, parent.width * Backend.volume)
                height: parent.height
                radius: height / 2
                color: Backend.pending.indexOf("volume") >= 0 ? Theme.alpha(Theme.primary, 0.6) : Theme.primary
                Behavior on width { NumberAnimation { duration: Theme.motion.quick } }
            }
        }
        Text {
            id: percent
            anchors.verticalCenter: parent.verticalCenter
            width: Theme.dp(64)
            horizontalAlignment: Text.AlignRight
            text: Math.round(Backend.volume * 100) + "%"
            color: Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelLarge
            font.weight: Font.Bold
            font.features: { "tnum": 1 }
        }
    }
}
