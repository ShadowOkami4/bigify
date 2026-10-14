import QtQuick
import QtQuick.Shapes
import Bigify

// Bigify's icon set: rounded 2 px strokes on a 24×24 grid, drawn as vector
// paths so they stay crisp at any TV resolution. Solid icons are filled.
Item {
    id: root
    property string name: ""
    property color color: Theme.surfaceContent
    property real size: Theme.size.icon
    // Stroke width in grid units; heavier reads better from the couch.
    property real weight: 2.2

    implicitWidth: size
    implicitHeight: size

    readonly property var paths: ({
        "home": "M4 10.5 L12 4 L20 10.5 V19 A1 1 0 0 1 19 20 H15 V14.5 H9 V20 H5 A1 1 0 0 1 4 19 Z",
        "search": "M17 10.5 A6.5 6.5 0 1 1 4 10.5 A6.5 6.5 0 1 1 17 10.5 Z M15.3 15.3 L20 20",
        "library": "M5 4 V20 M10 4 V20 M14.5 5 L19.5 19",
        "settings": "M4 7 H13 M17 7 H20 M15 4.5 V9.5 M4 17 H7 M11 17 H20 M9 14.5 V19.5",
        "nowPlaying": "M9 18 V5.5 L19 3.5 V16 M9 18 A3 3 0 1 1 3 18 A3 3 0 1 1 9 18 Z M19 16 A3 3 0 1 1 13 16 A3 3 0 1 1 19 16 Z",
        "play": "M8 5.6 V18.4 A1 1 0 0 0 9.5 19.3 L19.6 12.9 A1 1 0 0 0 19.6 11.1 L9.5 4.7 A1 1 0 0 0 8 5.6 Z",
        "pause": "M7 5 H10 V19 H7 Z M14 5 H17 V19 H14 Z",
        "next": "M6 6.5 L14.5 12 L6 17.5 Z M18 6 V18",
        "previous": "M18 6.5 L9.5 12 L18 17.5 Z M6 6 V18",
        "shuffle": "M4 7 H7 C11 7 13 17 17 17 H20 M17 14 L20 17 L17 20 M4 17 H7 C8.7 17 9.9 15.2 10.8 13.2 M13.2 10.8 C14.1 8.8 15.3 7 17 7 H20 M17 4 L20 7 L17 10",
        "repeat": "M7 4 L4 7 L7 10 M4 7 H17 A3 3 0 0 1 20 10 V11.5 M17 20 L20 17 L17 14 M20 17 H7 A3 3 0 0 1 4 14 V12.5",
        "repeatOne": "M7 4 L4 7 L7 10 M4 7 H17 A3 3 0 0 1 20 10 V11.5 M17 20 L20 17 L17 14 M20 17 H7 A3 3 0 0 1 4 14 V12.5 M11 10.8 L12.4 10 V14.2",
        "queue": "M4 6 H16 M4 11 H16 M4 16 H10 M14 13.5 V20.5 L19.5 17 Z",
        "addQueue": "M4 6 H14 M4 11 H14 M4 16 H10 M17 13 V21 M13 17 H21",
        "volumeDown": "M4 9.5 H7.5 L12 5.5 V18.5 L7.5 14.5 H4 Z M16 9.5 A3.5 3.5 0 0 1 16 14.5",
        "volumeUp": "M4 9.5 H7.5 L12 5.5 V18.5 L7.5 14.5 H4 Z M16 9.5 A3.5 3.5 0 0 1 16 14.5 M18.5 6.8 A7.5 7.5 0 0 1 18.5 17.2",
        "back": "M15 5 L8 12 L15 19",
        "forward": "M9 5 L16 12 L9 19",
        "backspace": "M9 5.5 H19 A1 1 0 0 1 20 6.5 V17.5 A1 1 0 0 1 19 18.5 H9 L3.5 12 Z M12 9.5 L16.5 14.5 M16.5 9.5 L12 14.5",
        "space": "M5 11 V15 H19 V11",
        "clear": "M6 6 L18 18 M18 6 L6 18",
        "check": "M5 12.5 L10 17.5 L19 7",
        "alert": "M12 21 A9 9 0 1 0 12 3 A9 9 0 1 0 12 21 Z M12 7.5 V13 M12 16.4 V16.6",
        "info": "M12 21 A9 9 0 1 0 12 3 A9 9 0 1 0 12 21 Z M12 11 V16.5 M12 7.6 V7.8",
        "artist": "M12 12 A4 4 0 1 0 12 4 A4 4 0 1 0 12 12 Z M4.5 20 C5.5 16.5 8.5 14.5 12 14.5 C15.5 14.5 18.5 16.5 19.5 20",
        "album": "M12 21 A9 9 0 1 0 12 3 A9 9 0 1 0 12 21 Z M12 14 A2 2 0 1 0 12 10 A2 2 0 1 0 12 14 Z",
        "playlist": "M4 6 H16 M4 11 H16 M4 16 H11 M18 18 V9 L21 8 M18 18 A2 2 0 1 1 14 18 A2 2 0 1 1 18 18 Z",
        "heart": "M12 20 C5 15 3 12 3 8.5 A4.5 4.5 0 0 1 12 6.5 A4.5 4.5 0 0 1 21 8.5 C21 12 19 15 12 20 Z",
        "phone": "M8 3 H16 A2 2 0 0 1 18 5 V19 A2 2 0 0 1 16 21 H8 A2 2 0 0 1 6 19 V5 A2 2 0 0 1 8 3 Z M11 18 H13",
        "browser": "M12 21 A9 9 0 1 0 12 3 A9 9 0 1 0 12 21 Z M3 12 H21 M12 3 C9.5 6 9.5 18 12 21 M12 3 C14.5 6 14.5 18 12 21",
        "power": "M12 3 V11 M7 5.8 A8 8 0 1 0 17 5.8",
        "tv": "M4 5 H20 A1 1 0 0 1 21 6 V16 A1 1 0 0 1 20 17 H4 A1 1 0 0 1 3 16 V6 A1 1 0 0 1 4 5 Z M8 21 H16",
        "gamepad": "M7 8 H17 A4 4 0 0 1 21 12 V14 A3 3 0 0 1 15.8 16 L14.5 14.5 H9.5 L8.2 16 A3 3 0 0 1 3 14 V12 A4 4 0 0 1 7 8 Z M7 11 V14 M5.5 12.5 H8.5 M16 11.5 V11.6 M17.5 13 V13.1",
        "speaker": "M7 3 H17 A1 1 0 0 1 18 4 V20 A1 1 0 0 1 17 21 H7 A1 1 0 0 1 6 20 V4 A1 1 0 0 1 7 3 Z M15 14.5 A3 3 0 1 1 9 14.5 A3 3 0 1 1 15 14.5 Z M12 7 V7.1"
    })
    readonly property var solid: ["play", "pause", "next", "previous"]

    Shape {
        anchors.centerIn: parent
        width: 24
        height: 24
        scale: root.size / 24
        preferredRendererType: Shape.CurveRenderer
        visible: root.name in root.paths

        ShapePath {
            strokeColor: root.color
            strokeWidth: root.weight
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            fillColor: root.solid.indexOf(root.name) >= 0 ? root.color : "transparent"
            PathSvg { path: root.paths[root.name] || "" }
        }
    }
}
