import QtQuick
import Bigify

// First-run / signed-out experience. Signing in from a phone over Spotify
// Connect needs no typing on the TV; browser sign-in on this device is the
// fallback (e.g. Steam Deck desktop mode).
FocusScope {
    id: root
    signal quitRequested()

    readonly property string state_: Backend.authState
    readonly property bool busy: state_ === "starting" || state_ === "connecting"

    Rectangle { anchors.fill: parent; color: Theme.surface }

    // Brand panel
    Item {
        id: brand
        anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
        width: parent.width * 0.42
        Rectangle {
            anchors.fill: parent
            anchors.margins: Theme.safeV
            radius: Theme.shape.extraExtraLarge
            color: Theme.primaryContainer
            Column {
                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: Theme.dp(72) }
                spacing: Theme.space.xl
                Text {
                    text: "Bigify"
                    color: Theme.primaryContainerContent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.displayLarge * 1.25
                    font.weight: Font.Black
                }
                Text {
                    width: parent.width
                    text: qsTr("Your music, made for the big screen.")
                    wrapMode: Text.WordWrap
                    color: Theme.primaryContainerContent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.headlineMedium
                    font.weight: Font.Medium
                }
                Text {
                    width: parent.width
                    text: qsTr("Requires Spotify Premium. Bigify is an independent open-source project and is not affiliated with Spotify.")
                    wrapMode: Text.WordWrap
                    color: Theme.alpha(Theme.primaryContainerContent, 0.8)
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelLarge
                }
            }
        }
    }

    // Busy: starting / connecting
    Column {
        anchors { left: brand.right; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: Theme.dp(72); rightMargin: Theme.safeH }
        visible: root.busy
        spacing: Theme.space.xl
        LoadingIndicator { size: Theme.dp(88); running: root.busy }
        Text {
            text: root.state_ === "connecting" ? qsTr("Connecting to Spotify…") : qsTr("Starting…")
            color: Theme.surfaceContent
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.headlineMedium
            font.weight: Theme.weightDisplay
        }
    }

    FocusScope {
        id: options
        anchors { left: brand.right; right: parent.right; top: parent.top; bottom: parent.bottom; leftMargin: Theme.dp(72); rightMargin: Theme.safeH }
        visible: !root.busy
        focus: visible

        Column {
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter }
            spacing: Theme.space.xxl

            Text {
                width: parent.width
                text: root.state_ === "failed" ? qsTr("Something went wrong") : qsTr("Sign in")
                color: Theme.surfaceContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.displaySmall
                font.weight: Theme.weightDisplay
            }
            Text {
                width: parent.width
                visible: Backend.authMessage !== ""
                text: Backend.authMessage
                wrapMode: Text.WordWrap
                color: Theme.error
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.bodyLarge
            }

            // Option 1: phone via Spotify Connect discovery
            Rectangle {
                width: parent.width
                height: phoneColumn.implicitHeight + Theme.space.xxxl * 2
                radius: Theme.shape.extraExtraLarge
                color: Theme.surfaceContainer
                Row {
                    anchors { fill: parent; margins: Theme.space.xxxl }
                    spacing: Theme.space.xxl
                    Rectangle {
                        width: Theme.dp(96); height: width; radius: Theme.shape.extraLarge
                        color: Theme.secondaryContainer
                        Icon { anchors.centerIn: parent; name: "phone"; size: Theme.dp(52); color: Theme.secondaryContainerContent }
                    }
                    Column {
                        id: phoneColumn
                        width: parent.width - Theme.dp(96) - Theme.space.xxl
                        spacing: Theme.space.m
                        Text {
                            text: qsTr("Use your phone")
                            color: Theme.surfaceContent
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.headlineSmall
                            font.weight: Theme.weightTitle
                        }
                        Text {
                            width: parent.width
                            wrapMode: Text.WordWrap
                            text: Backend.discoveryAvailable
                                  ? qsTr("1.  Connect your phone to the same Wi-Fi\n2.  Open Spotify and start any song\n3.  Tap the devices icon and choose “%1”").arg(Backend.deviceName || "Bigify")
                                  : qsTr("Network discovery isn't available right now. Check that Bigify may use the local network, or sign in with a browser below.")
                            color: Theme.surfaceContentVariant
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.bodyLarge
                            lineHeight: 1.25
                        }
                        Row {
                            visible: Backend.discoveryAvailable
                            spacing: Theme.space.s
                            LoadingIndicator { size: Theme.dp(28); running: parent.visible; anchors.verticalCenter: parent.verticalCenter }
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: qsTr("Waiting for your phone")
                                color: Theme.primary
                                font.family: Theme.fontFamily
                                font.pixelSize: Theme.type.labelLarge
                                font.weight: Font.Bold
                            }
                        }
                    }
                }
            }

            // Option 2: browser on this device
            Row {
                spacing: Theme.space.l
                PillButton {
                    id: browserButton
                    focus: true
                    visible: root.state_ !== "browserSignIn"
                    text: qsTr("Sign in with a browser on this device")
                    icon: "browser"
                    variant: Backend.discoveryAvailable ? "tonal" : "filled"
                    onClicked: Backend.signInWithBrowser()
                    KeyNavigation.right: quitButton
                }
                PillButton {
                    id: cancelButton
                    visible: root.state_ === "browserSignIn"
                    focus: visible
                    text: qsTr("Cancel browser sign-in")
                    icon: "clear"
                    variant: "outlined"
                    onClicked: Backend.cancelSignIn()
                    KeyNavigation.right: quitButton
                }
                PillButton {
                    id: quitButton
                    text: qsTr("Exit")
                    icon: "power"
                    variant: "text"
                    onClicked: root.quitRequested()
                }
            }
            Text {
                visible: root.state_ === "browserSignIn"
                width: parent.width
                wrapMode: Text.WordWrap
                text: qsTr("Finish signing in in the browser window that just opened, then come back here.")
                color: Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.bodyLarge
            }
        }
    }
}
