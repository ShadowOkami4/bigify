import QtQuick
import Bigify

// D-pad friendly keyboard for search. Alphabetical grid (faster than QWERTY
// with a D-pad), plus space / delete / clear. Hardware keyboards and the
// Steam on-screen keyboard type straight into the field instead.
FocusScope {
    id: root
    signal typed(string text)
    signal backspace()
    signal cleared()

    readonly property int columns: 7
    readonly property var keys: "abcdefghijklmnopqrstuvwxyz1234567890".split("")
        .concat([{ action: "space", icon: "space", label: qsTr("Space") },
                 { action: "backspace", icon: "backspace", label: qsTr("Delete") },
                 { action: "clear", icon: "clear", label: qsTr("Clear") }])
    readonly property int keySize: Theme.dp(78)

    implicitWidth: grid.cellWidth * columns
    implicitHeight: grid.cellHeight * Math.ceil(keys.length / columns)

    GridView {
        id: grid
        anchors.fill: parent
        focus: true
        interactive: false
        cellWidth: root.keySize + Theme.space.s
        cellHeight: root.keySize + Theme.space.s
        model: root.keys
        keyNavigationEnabled: true
        keyNavigationWraps: false

        delegate: Item {
            id: key
            required property var modelData
            required property int index
            readonly property bool isAction: typeof modelData === "object"
            width: grid.cellWidth
            height: grid.cellHeight
            Accessible.role: Accessible.Button
            Accessible.name: isAction ? modelData.label : modelData

            Rectangle {
                id: face
                width: root.keySize; height: root.keySize
                radius: key.GridView.isCurrentItem && grid.activeFocus ? Theme.shape.extraLarge : Theme.shape.large
                color: key.GridView.isCurrentItem && grid.activeFocus ? Theme.primary
                     : (key.isAction ? Theme.secondaryContainer : Theme.surfaceContainerHigh)
                scale: key.GridView.isCurrentItem && grid.activeFocus ? 1.08 : 1
                Behavior on scale { NumberAnimation { duration: Theme.motion.quick; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
                Behavior on radius { NumberAnimation { duration: Theme.motion.quick } }
                Text {
                    visible: !key.isAction
                    anchors.centerIn: parent
                    text: key.isAction ? "" : key.modelData.toUpperCase()
                    color: key.GridView.isCurrentItem && grid.activeFocus ? Theme.primaryContent : Theme.surfaceContent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.titleLarge
                    font.weight: Font.DemiBold
                }
                Icon {
                    visible: key.isAction
                    anchors.centerIn: parent
                    name: key.isAction ? key.modelData.icon : ""
                    size: Theme.dp(34)
                    color: key.GridView.isCurrentItem && grid.activeFocus ? Theme.primaryContent : Theme.secondaryContainerContent
                }
            }
            MouseArea {
                anchors.fill: face
                onClicked: { grid.currentIndex = key.index; grid.forceActiveFocus(); root.press(key.modelData) }
            }
        }

        Keys.onReturnPressed: root.press(root.keys[currentIndex])
        Keys.onEnterPressed: root.press(root.keys[currentIndex])
    }

    function press(k) {
        if (typeof k === "string") root.typed(k)
        else if (k.action === "space") root.typed(" ")
        else if (k.action === "backspace") root.backspace()
        else if (k.action === "clear") root.cleared()
    }
}
