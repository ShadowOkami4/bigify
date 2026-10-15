import QtQuick
import Bigify

// "[glyph] label" hint that follows the input device in use: controller
// button glyphs (Steam Deck / Xbox / PlayStation / Nintendo labels) after
// controller input, key caps after keyboard or mouse input.
//
// Actions: select, back, playPause, more, lyrics, prevNext, volume,
// nowPlaying, upNext, seek.
Row {
    id: root
    property string action: ""
    property string label: ""
    property color textColor: Theme.surfaceContentVariant

    readonly property bool gamepad: Backend.inputMode === "gamepad"
    readonly property string family: Backend.controllerFamily

    spacing: Theme.space.s
    Accessible.role: Accessible.StaticText
    Accessible.name: label

    // Face buttons by position: [south, east, west, north].
    readonly property var faces: ({
        deck: ["A", "B", "X", "Y"],
        xbox: ["A", "B", "X", "Y"],
        playstation: ["✕", "○", "□", "△"],
        nintendo: ["B", "A", "Y", "X"]
    })[family] || ["A", "B", "X", "Y"]

    readonly property var shoulders: ({
        deck: ["L1", "R1", "L2", "R2"],
        xbox: ["LB", "RB", "LT", "RT"],
        playstation: ["L1", "R1", "L2", "R2"],
        nintendo: ["L", "R", "ZL", "ZR"]
    })[family] || ["LB", "RB", "LT", "RT"]

    // Each glyph: { text } or { icon }, `round` for single face buttons.
    readonly property var glyphs: {
        if (gamepad) {
            switch (action) {
            case "select": return [{ text: faces[0], round: true }]
            case "back": return [{ text: faces[1], round: true }]
            case "playPause": return [{ text: faces[2], round: true }]
            case "more": return [{ text: faces[3], round: true }]
            case "lyrics": return [{ text: faces[3], round: true }]
            case "results": return [{ text: faces[3], round: true }]
            case "prevNext": return [{ text: shoulders[0] }, { text: shoulders[1] }]
            case "skip10": return [{ text: shoulders[2] }, { text: shoulders[3] }]
            case "volume": return [{ text: "Vol" }]
            case "nowPlaying": return family === "nintendo" ? [{ text: "+", round: true }]
                                    : family === "playstation" ? [{ text: "OPTIONS" }] : [{ icon: "btnMenu", round: true }]
            case "upNext": return family === "nintendo" ? [{ text: "−", round: true }]
                                : family === "playstation" ? [{ text: "CREATE" }] : [{ icon: "btnView", round: true }]
            case "seek": return [{ icon: "back", round: true }, { icon: "forward", round: true }]
            }
        } else {
            switch (action) {
            case "select": return [{ text: "Enter" }]
            case "back": return [{ text: "Esc" }]
            case "playPause": return [{ text: "Ctrl" }, { text: "P" }]
            case "more": return [{ text: "Q", round: true }]
            case "lyrics": return [{ text: "L", round: true }]
            case "results": return [{ text: "Tab" }]
            case "prevNext": return [{ text: "Ctrl" }, { icon: "back", round: true }, { icon: "forward", round: true }]
            case "skip10": return [{ text: "Ctrl" }, { text: "Shift" }, { icon: "back", round: true }, { icon: "forward", round: true }]
            case "volume": return [{ text: "Ctrl" }, { icon: "up", round: true }, { icon: "down", round: true }]
            case "nowPlaying": return [{ text: "F2" }]
            case "upNext": return [{ text: "F3" }]
            case "seek": return [{ icon: "back", round: true }, { icon: "forward", round: true }]
            }
        }
        return []
    }

    readonly property int capHeight: Theme.dp(34)

    Row {
        spacing: Theme.space.xs
        anchors.verticalCenter: parent.verticalCenter
        Repeater {
            model: root.glyphs
            Rectangle {
                required property var modelData
                height: root.capHeight
                width: modelData.round ? height : Math.max(height, capText.implicitWidth + Theme.space.l)
                radius: modelData.round || root.gamepad ? height / 2 : Theme.shape.small
                color: Theme.surfaceContainerHighest
                border.width: Math.max(1, Theme.dp(1.5))
                border.color: Theme.outlineVariant
                Text {
                    id: capText
                    visible: !parent.modelData.icon
                    anchors.centerIn: parent
                    text: parent.modelData.text || ""
                    color: Theme.surfaceContent
                    font.family: Theme.fontFamily
                    font.pixelSize: text.length > 2 ? Theme.dp(15) : Theme.dp(18)
                    font.weight: Font.Bold
                }
                Icon {
                    visible: !!parent.modelData.icon
                    anchors.centerIn: parent
                    name: parent.modelData.icon || ""
                    size: Theme.dp(20)
                    weight: 2.6
                    color: Theme.surfaceContent
                }
            }
        }
    }

    Text {
        anchors.verticalCenter: parent.verticalCenter
        visible: root.label !== ""
        text: root.label
        color: root.textColor
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelSmall
    }
}
