import QtQuick
import QtQuick.Shapes
import Bigify

// The Bigify app icon (packaging/linux/io.github.ShadowOkami4.bigify.svg),
// drawn natively so it stays crisp at any scale. Brand colors are fixed on
// purpose: the logo does not follow artwork-derived theming.
Item {
    id: root
    property real size: Theme.dp(64)
    implicitWidth: size
    implicitHeight: size
    Accessible.role: Accessible.Graphic
    Accessible.name: "Bigify"

    // Authored on the icon's 128×128 grid.
    Item {
        width: 128
        height: 128
        scale: root.size / 128
        transformOrigin: Item.TopLeft

        Rectangle {
            x: 8; y: 8; width: 112; height: 112; radius: 34
            color: "#7C5CFF"
            antialiasing: true
            // Soft top highlight, clipped to the squircle.
            Rectangle {
                width: parent.width; height: 56; radius: 34
                color: "#9A82FF"
                opacity: 0.35
                antialiasing: true
            }
        }
        Rectangle {
            x: 26; y: 32; width: 76; height: 52; radius: 14
            color: "#E9E1FF"
            antialiasing: true
        }
        Shape {
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeWidth: 0
                strokeColor: "transparent"
                fillColor: "#2A1A73"
                PathSvg { path: "M56 46.5v23a2.5 2.5 0 0 0 3.8 2.1l18.4-11.5a2.5 2.5 0 0 0 0-4.2L59.8 44.4A2.5 2.5 0 0 0 56 46.5z" }
            }
        }
        Rectangle {
            x: 46; y: 92; width: 36; height: 8; radius: 4
            color: "#E9E1FF"
            antialiasing: true
        }
    }
}
