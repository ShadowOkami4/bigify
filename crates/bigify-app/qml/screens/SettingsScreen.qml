import QtQuick
import Bigify

// Settings. Only options the engine or UI actually honour are listed;
// engine options state when they take effect.
FocusScope {
    id: root
    signal quitRequested()

    readonly property var s: JSON.parse(Backend.settingsJson || "{}")
    function set(key, value) {
        if (!Backend.setSetting(key, JSON.stringify(value)))
            toastRequested("error", qsTr("That setting couldn't be changed"))
    }
    signal toastRequested(string level, string message)

    readonly property var diagnostics: JSON.parse(Backend.diagnostics())

    Text {
        id: title
        x: Theme.safeH
        y: Theme.safeV
        text: qsTr("Settings")
        color: Theme.surfaceContent
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.displaySmall
        font.weight: Theme.weightDisplay
    }

    Rectangle {
        id: restartBanner
        visible: Backend.restartRequired
        anchors { top: title.bottom; topMargin: Theme.space.xl; left: parent.left; leftMargin: Theme.safeH; right: parent.right; rightMargin: Theme.safeH }
        height: visible ? Theme.dp(72) : 0
        radius: Theme.shape.extraLarge
        color: Theme.tertiaryContainer
        Row {
            anchors { left: parent.left; leftMargin: Theme.space.xxl; verticalCenter: parent.verticalCenter }
            spacing: Theme.space.l
            Icon { name: "info"; color: Theme.tertiaryContainerContent; size: Theme.dp(30); anchors.verticalCenter: parent.verticalCenter }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("Some changes take effect the next time Bigify starts.")
                color: Theme.tertiaryContainerContent
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.titleSmall
            }
        }
    }

    ListView {
        id: list
        anchors { top: restartBanner.visible ? restartBanner.bottom : title.bottom; topMargin: Theme.space.xxl; left: parent.left; right: parent.right; bottom: parent.bottom }
        leftMargin: Theme.safeH
        rightMargin: Theme.safeH
        bottomMargin: Theme.safeV + Theme.size.miniPlayer
        focus: true
        clip: true
        spacing: Theme.space.m
        keyNavigationEnabled: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: Theme.dp(80)
        preferredHighlightEnd: height - Theme.dp(260)
        highlightMoveDuration: Theme.motion.medium
        highlightMoveVelocity: -1
        boundsBehavior: Flickable.StopAtBounds
        model: settingsModel
        section.property: "group"
        section.delegate: Text {
            required property string section
            topPadding: Theme.space.xxl
            bottomPadding: Theme.space.s
            text: section
            color: Theme.primary
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.titleMedium
            font.weight: Font.Bold
        }

        delegate: SettingRow {
            required property var model
            width: list.width - list.leftMargin - list.rightMargin
            focus: ListView.isCurrentItem
            label: model.label
            description: model.description
            control: model.control
            destructive: model.destructive
            checked: model.control === "toggle" ? root.s[model.key] === true : false
            value: model.key ? root.s[model.key] : undefined
            options: model.options ? JSON.parse(model.options) : []
            valueText: model.key === "account" ? (Backend.userName || "")
                     : model.key === "version" ? Backend.appVersion
                     : model.key === "logFile" ? Backend.logFilePath
                     : model.key === "deviceName" ? (root.s.deviceName || "")
                     : ""
            onToggled: c => root.set(model.key, c)
            onChosen: v => root.set(model.key, v)
            onTriggered: {
                if (model.key === "signOut") Backend.signOut()
                else if (model.key === "quit") root.quitRequested()
            }
        }
    }

    ListModel {
        id: settingsModel
        // Account
        ListElement { group: "Account"; key: "account"; label: "Signed in"; description: "Spotify Premium is required for playback."; control: "info"; destructive: false; options: "" }
        ListElement { group: "Account"; key: "signOut"; label: "Sign out"; description: "Removes Bigify's stored Spotify credentials from this device."; control: "action"; destructive: true; options: "" }
        // Playback
        ListElement { group: "Playback"; key: "audioQuality"; label: "Audio quality"; description: "Streaming bitrate. Applies after restart."; control: "choice"; destructive: false
            options: '[{"value":"low","label":"Low · 96 kbit/s"},{"value":"normal","label":"Normal · 160 kbit/s"},{"value":"high","label":"High · 320 kbit/s"}]' }
        ListElement { group: "Playback"; key: "normalisation"; label: "Volume normalisation"; description: "Keeps loudness consistent between tracks. Applies after restart."; control: "toggle"; destructive: false; options: "" }
        ListElement { group: "Playback"; key: "gapless"; label: "Gapless playback"; description: "No silence between tracks. Applies after restart."; control: "toggle"; destructive: false; options: "" }
        ListElement { group: "Playback"; key: "audioBackend"; label: "Audio output"; description: "PulseAudio works with PipeWire (SteamOS, most desktops). Use ALSA only without a sound server. Applies after restart."; control: "choice"; destructive: false
            options: '[{"value":"pulseaudio","label":"PulseAudio / PipeWire"},{"value":"alsa","label":"ALSA"}]' }
        ListElement { group: "Playback"; key: "zeroconfDiscovery"; label: "Sign in from phone"; description: "Advertise Bigify on your network so the Spotify app can sign it in. Applies after restart."; control: "toggle"; destructive: false; options: "" }
        ListElement { group: "Playback"; key: "deviceName"; label: "Device name"; description: "How Bigify appears in Spotify Connect. Edit deviceName in settings.toml; applies after restart."; control: "info"; destructive: false; options: "" }
        // Display
        ListElement { group: "Display"; key: "startFullscreen"; label: "Start full screen"; description: "Recommended for TVs and Steam gaming mode."; control: "toggle"; destructive: false; options: "" }
        ListElement { group: "Display"; key: "dynamicColor"; label: "Colors from artwork"; description: "Tint the interface with colors from the current album art."; control: "toggle"; destructive: false; options: "" }
        ListElement { group: "Display"; key: "reducedMotion"; label: "Reduce motion"; description: "Replace movement with simple fades."; control: "toggle"; destructive: false; options: "" }
        ListElement { group: "Display"; key: "uiScale"; label: "Interface size"; description: "On top of automatic sizing for your screen."; control: "choice"; destructive: false
            options: '[{"value":0.9,"label":"Compact"},{"value":1.0,"label":"Default"},{"value":1.15,"label":"Large"},{"value":1.3,"label":"Extra large"}]' }
        // Controls
        ListElement { group: "Controls"; key: "controllerInput"; label: "Read controllers directly"; description: "Turn off if Steam Input already maps your controller to keyboard keys and you see double input."; control: "toggle"; destructive: false; options: "" }
        ListElement { group: "Controls"; key: "help"; label: "Controller map"; description: "D-pad/stick move · A select · B back · X play/pause · Y add to queue · LB/RB previous/next · LT/RT volume · Start Now Playing · Select Up next"; control: "info"; destructive: false; options: "" }
        // About
        ListElement { group: "About"; key: "version"; label: "Bigify version"; description: "Free software under the GNU GPL v3 or later. Playback by librespot (MIT). UI by Qt (LGPL v3). Bigify is an independent project, not affiliated with or endorsed by Spotify."; control: "info"; destructive: false; options: "" }
        ListElement { group: "About"; key: "logFile"; label: "Log file"; description: "Attach this file to bug reports. It never contains passwords or tokens."; control: "info"; destructive: false; options: "" }
        ListElement { group: "About"; key: "quit"; label: "Exit Bigify"; description: ""; control: "action"; destructive: false; options: "" }
    }
}
