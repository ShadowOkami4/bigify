import QtQuick
import Bigify

// Filter chip used for category navigation (e.g. Library sections).
Item {
    id: root
    property string text: ""
    property string icon: ""
    property bool selected: false
    signal clicked()

    implicitHeight: Theme.dp(64)
    implicitWidth: row.implicitWidth + Theme.space.xxl * 2
    activeFocusOnTab: true
    Accessible.role: Accessible.RadioButton
    Accessible.name: text
    Accessible.checked: selected

    Rectangle {
        id: bg
        anchors.fill: parent
        radius: root.selected ? Theme.shape.large : height / 2
        color: root.selected ? Theme.secondaryContainer : "transparent"
        border.width: root.selected ? 0 : Math.max(1, Theme.dp(2))
        border.color: Theme.outlineVariant
        scale: root.activeFocus ? Theme.focusScale : 1
        Behavior on radius { NumberAnimation { duration: Theme.motion.medium; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
        Behavior on scale { NumberAnimation { duration: Theme.motion.quick } }
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }

        Row {
            id: row
            anchors.centerIn: parent
            spacing: Theme.space.s
            Icon {
                visible: root.selected || root.icon !== ""
                name: root.selected ? "check" : root.icon
                size: Theme.dp(26)
                color: root.selected ? Theme.secondaryContainerContent : Theme.surfaceContentVariant
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                text: root.text
                anchors.verticalCenter: parent.verticalCenter
                color: root.selected ? Theme.secondaryContainerContent : Theme.surfaceContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelLarge
                font.weight: Font.DemiBold
            }
        }
        FocusRing { shown: root.activeFocus; radius: bg.radius }
    }

    Keys.onReturnPressed: root.clicked()
    Keys.onEnterPressed: root.clicked()
    MouseArea { anchors.fill: parent; onClicked: { root.forceActiveFocus(); root.clicked() } }
}
