import QtQuick
import Bigify

// Shared loading / empty / error presentation. `kind` follows the backend's
// error kinds: retry is only offered when retrying can help.
FocusScope {
    id: root
    property string state_: "loading"   // "loading", "empty", "error"
    property string title: ""
    property string message: ""
    property string errorKind: ""
    property string icon: ""
    signal retry()

    readonly property bool retryable: state_ === "error"
        && ["network", "rateLimited", "internal", "notReady"].indexOf(errorKind) >= 0

    implicitHeight: column.implicitHeight
    implicitWidth: column.implicitWidth

    Column {
        id: column
        anchors.centerIn: parent
        spacing: Theme.space.xl
        width: Math.min(Theme.dp(900), root.width)

        LoadingIndicator {
            visible: root.state_ === "loading"
            running: visible
            size: Theme.dp(72)
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Icon {
            visible: root.state_ !== "loading"
            name: root.icon || (root.state_ === "error" ? "alert" : "info")
            size: Theme.dp(64)
            color: root.state_ === "error" ? Theme.error : Theme.surfaceContentVariant
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Text {
            width: parent.width
            visible: text !== ""
            text: root.title
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            color: Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.headlineSmall
            font.weight: Theme.weightDisplay
        }
        Text {
            width: parent.width
            visible: text !== ""
            text: root.message
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            color: Theme.surfaceContentVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodyLarge
        }
        PillButton {
            visible: root.retryable
            focus: true
            anchors.horizontalCenter: parent.horizontalCenter
            text: qsTr("Try again")
            icon: "repeat"
            variant: "tonal"
            onClicked: root.retry()
        }
    }
}
