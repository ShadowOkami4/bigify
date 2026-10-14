import QtQuick
import Bigify

// Text button with optional leading icon. `variant`: "filled", "tonal",
// "outlined", "text". Large, pill-shaped targets for couch distance.
Item {
    id: root
    property string text: ""
    property string icon: ""
    property string variant: "filled"
    property bool busy: false
    property real height_: Theme.size.button
    signal clicked()

    implicitHeight: height_
    implicitWidth: row.implicitWidth + Theme.space.xxl * 2
    activeFocusOnTab: true
    opacity: enabled ? 1 : Theme.disabledContent

    Accessible.role: Accessible.Button
    Accessible.name: text

    property bool keyDown: false
    readonly property bool pressed: mouse.pressed || keyDown

    readonly property color containerColor: ({
        "filled": Theme.primary,
        "tonal": Theme.secondaryContainer,
        "outlined": "transparent",
        "text": "transparent"
    })[variant]
    readonly property color contentColor: ({
        "filled": Theme.primaryContent,
        "tonal": Theme.secondaryContainerContent,
        "outlined": Theme.primary,
        "text": Theme.primary
    })[variant]

    Rectangle {
        id: container
        anchors.fill: parent
        // Expressive press: the pill flattens slightly into a rounded rect.
        radius: root.pressed ? Theme.shape.large : height / 2
        color: root.containerColor
        border.width: root.variant === "outlined" ? Math.max(1, Theme.dp(2)) : 0
        border.color: Theme.outline
        scale: root.pressed ? 0.96 : (root.activeFocus ? Theme.focusScale : 1.0)
        Behavior on radius { NumberAnimation { duration: Theme.motion.quick } }
        Behavior on scale { NumberAnimation { duration: Theme.motion.quick; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }

        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: root.contentColor
            opacity: root.pressed ? Theme.statePressed : (root.activeFocus || mouse.containsMouse ? Theme.stateFocus : 0)
        }

        Row {
            id: row
            anchors.centerIn: parent
            spacing: Theme.space.m
            Item {
                width: root.icon || root.busy ? Theme.size.icon : 0
                height: Theme.size.icon
                anchors.verticalCenter: parent.verticalCenter
                Icon {
                    anchors.centerIn: parent
                    visible: !root.busy && root.icon !== ""
                    name: root.icon
                    color: root.contentColor
                    size: Theme.size.icon * 0.9
                }
                LoadingIndicator {
                    anchors.centerIn: parent
                    running: root.busy
                    visible: root.busy
                    size: Theme.size.icon
                    color: root.contentColor
                }
            }
            Text {
                text: root.text
                color: root.contentColor
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelLarge
                font.weight: Font.DemiBold
                anchors.verticalCenter: parent.verticalCenter
            }
        }

        FocusRing { shown: root.activeFocus; radius: container.radius }
    }

    function activate() { if (root.enabled && !root.busy) root.clicked() }

    Keys.onPressed: event => {
        if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) && !event.isAutoRepeat) {
            keyDown = true
            event.accepted = true
        }
    }
    Keys.onReleased: event => {
        if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) && keyDown) {
            keyDown = false
            activate()
            event.accepted = true
        }
    }
    onActiveFocusChanged: if (!activeFocus) keyDown = false

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: { root.forceActiveFocus(); root.activate() }
    }
}
