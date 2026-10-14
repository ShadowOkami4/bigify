import QtQuick
import Bigify

// Round icon button. `variant`: "standard" (no container), "tonal", "filled".
// A selected toggle (shuffle/repeat on) morphs into a rounded square, the
// Material 3 Expressive shape change for toggle buttons.
Item {
    id: root
    property string icon: ""
    property string variant: "standard"
    property bool toggle: false
    property bool checked: false
    property bool busy: false
    property real size: Theme.size.iconButton
    // Explicit corner radius; negative = automatic (circle, or rounded
    // square when a toggle is checked).
    property real cornerRadius: -1
    property string accessibleName: icon
    signal clicked()

    implicitWidth: size
    implicitHeight: size
    activeFocusOnTab: true
    opacity: enabled ? 1 : Theme.disabledContent

    Accessible.role: Accessible.Button
    Accessible.name: accessibleName
    Accessible.checkable: toggle
    Accessible.checked: checked

    readonly property bool pressed: mouse.pressed || keyDown
    property bool keyDown: false

    readonly property color containerColor: {
        if (toggle && checked) return Theme.primary
        if (variant === "filled") return Theme.primary
        if (variant === "tonal") return Theme.secondaryContainer
        return "transparent"
    }
    readonly property color contentColor: {
        if (toggle && checked) return Theme.primaryContent
        if (variant === "filled") return Theme.primaryContent
        if (variant === "tonal") return Theme.secondaryContainerContent
        return root.activeFocus ? Theme.surfaceContent : Theme.surfaceContentVariant
    }

    Rectangle {
        id: container
        anchors.fill: parent
        radius: root.cornerRadius >= 0 ? root.cornerRadius
              : (root.toggle && root.checked ? root.size * 0.3 : width / 2)
        color: root.containerColor
        scale: root.pressed ? 0.92 : (root.activeFocus ? Theme.focusScale : 1.0)
        Behavior on radius { NumberAnimation { duration: Theme.motion.medium; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
        Behavior on scale { NumberAnimation { duration: Theme.motion.quick; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }

        // State layer
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: root.contentColor
            opacity: root.pressed ? Theme.statePressed : (root.activeFocus || mouse.containsMouse ? Theme.stateFocus : 0)
        }

        Icon {
            anchors.centerIn: parent
            name: root.icon
            size: root.size * 0.44
            color: root.contentColor
        }

        FocusRing { shown: root.activeFocus; radius: container.radius }
    }

    LoadingIndicator {
        anchors.centerIn: parent
        size: root.size * 1.18
        running: root.busy
        visible: root.busy
        color: Theme.primary
    }

    function activate() { if (root.enabled) root.clicked() }

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
