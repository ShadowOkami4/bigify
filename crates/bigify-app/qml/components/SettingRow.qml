import QtQuick
import Bigify

// A settings entry. `control`: "toggle", "choice" (Left/Right or A cycles
// through `options`), "action" (A triggers), or "info" (read-only).
Item {
    id: root
    property string label: ""
    property string description: ""
    property string control: "info"
    property bool checked: false
    property var options: []            // [{ value, label }]
    property var value
    property string valueText: ""
    property bool destructive: false
    signal toggled(bool checked)
    signal chosen(var value)
    signal triggered()

    height: Math.max(Theme.dp(104), textColumn.implicitHeight + Theme.space.xl * 2)
    activeFocusOnTab: control !== "info"
    Accessible.role: control === "toggle" ? Accessible.CheckBox : Accessible.Button
    Accessible.name: label
    Accessible.description: description
    Accessible.checked: checked

    readonly property int optionIndex: options.findIndex(o => o.value === value)

    Rectangle {
        id: bg
        anchors.fill: parent
        radius: Theme.shape.extraLarge
        color: root.activeFocus ? Theme.surfaceContainerHighest : Theme.surfaceContainerLow
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
        FocusRing { shown: root.activeFocus; radius: bg.radius; gap: Theme.dp(2) }
    }

    Column {
        id: textColumn
        anchors {
            left: parent.left; leftMargin: Theme.space.xxl
            right: trailing.left; rightMargin: Theme.space.xl
            verticalCenter: parent.verticalCenter
        }
        spacing: Theme.space.xs
        Text {
            width: parent.width
            text: root.label
            color: root.destructive ? Theme.error : Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.titleSmall
            font.weight: Theme.weightTitle
            wrapMode: Text.WordWrap
        }
        Text {
            width: parent.width
            visible: text !== ""
            text: root.description
            color: Theme.surfaceContentVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelMedium
            wrapMode: Text.WordWrap
        }
    }

    Item {
        id: trailing
        anchors { right: parent.right; rightMargin: Theme.space.xxl; verticalCenter: parent.verticalCenter }
        width: childrenRect.width
        height: Theme.dp(56)

        // M3 switch, enlarged.
        Rectangle {
            id: track
            visible: root.control === "toggle"
            width: Theme.dp(92); height: Theme.dp(52); radius: height / 2
            anchors.verticalCenter: parent.verticalCenter
            color: root.checked ? Theme.primary : Theme.surfaceContainerHighest
            border.width: root.checked ? 0 : Math.max(1, Theme.dp(2.5))
            border.color: Theme.outline
            Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
            Rectangle {
                width: root.checked ? Theme.dp(38) : Theme.dp(26)
                height: width; radius: width / 2
                anchors.verticalCenter: parent.verticalCenter
                x: root.checked ? parent.width - width - Theme.dp(7) : Theme.dp(13)
                color: root.checked ? Theme.primaryContent : Theme.outline
                Behavior on x { NumberAnimation { duration: Theme.motion.medium; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
                Behavior on width { NumberAnimation { duration: Theme.motion.quick } }
                Icon {
                    anchors.centerIn: parent
                    visible: root.checked
                    name: "check"
                    size: Theme.dp(24)
                    color: Theme.primary
                }
            }
        }

        Row {
            visible: root.control === "choice"
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.space.m
            Icon { name: "back"; size: Theme.dp(28); color: Theme.surfaceContentVariant; visible: root.activeFocus; anchors.verticalCenter: parent.verticalCenter }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.optionIndex >= 0 ? root.options[root.optionIndex].label : ""
                color: Theme.primary
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.titleSmall
                font.weight: Font.DemiBold
            }
            Icon { name: "forward"; size: Theme.dp(28); color: Theme.surfaceContentVariant; visible: root.activeFocus; anchors.verticalCenter: parent.verticalCenter }
        }

        Text {
            visible: root.control === "info" || root.control === "action"
            anchors.verticalCenter: parent.verticalCenter
            text: root.control === "action" ? (root.valueText || "") : root.valueText
            color: Theme.surfaceContentVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelLarge
            elide: Text.ElideMiddle
            width: Math.min(implicitWidth, Theme.dp(760))
        }
    }

    function cycle(step) {
        if (options.length === 0) return
        const i = optionIndex < 0 ? 0 : (optionIndex + step + options.length) % options.length
        root.chosen(options[i].value)
    }

    Keys.onReturnPressed: activate()
    Keys.onEnterPressed: activate()
    Keys.onLeftPressed: event => { if (control === "choice") cycle(-1); else event.accepted = false }
    Keys.onRightPressed: event => { if (control === "choice") cycle(1); else event.accepted = false }

    function activate() {
        if (control === "toggle") root.toggled(!checked)
        else if (control === "choice") cycle(1)
        else if (control === "action") root.triggered()
    }

    MouseArea { anchors.fill: parent; enabled: root.control !== "info"; onClicked: { root.forceActiveFocus(); root.activate() } }
}
