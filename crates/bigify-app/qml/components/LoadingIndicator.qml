import QtQuick
import QtQuick.Shapes
import Bigify

// Material 3 Expressive loading indicator: a rotating soft polygon that
// morphs between a scalloped "cookie" and a softer shape. With reduced
// motion it becomes a slowly pulsing dot instead of spinning.
Item {
    id: root
    property real size: Theme.dp(48)
    property color color: Theme.primary
    property bool running: true

    implicitWidth: size
    implicitHeight: size

    property real morph: 0
    property real spin: 0

    SequentialAnimation on morph {
        running: root.running && root.visible && !Theme.reducedMotion
        loops: Animation.Infinite
        NumberAnimation { to: 1; duration: 650; easing.type: Easing.InOutCubic }
        NumberAnimation { to: 0; duration: 650; easing.type: Easing.InOutCubic }
    }
    NumberAnimation on spin {
        running: root.running && root.visible && !Theme.reducedMotion
        from: 0; to: 360
        duration: 1600
        loops: Animation.Infinite
    }

    function shapePath(m) {
        // Polar curve: r = base + amp·cos(lobes·θ); lobes/amp morph.
        const lobes = 8
        const amp = 0.07 + 0.08 * (1 - m)
        const base = 0.36 + 0.04 * m
        const steps = 72
        let d = ""
        for (let i = 0; i <= steps; i++) {
            const t = i / steps * Math.PI * 2
            const r = (base + amp * Math.cos(lobes * t)) * 24
            const x = 12 + r * Math.cos(t)
            const y = 12 + r * Math.sin(t)
            d += (i === 0 ? "M" : "L") + x.toFixed(2) + " " + y.toFixed(2) + " "
        }
        return d + "Z"
    }

    Shape {
        anchors.centerIn: parent
        width: 24; height: 24
        scale: root.size / 24
        rotation: root.spin
        visible: !Theme.reducedMotion
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeWidth: 0
            strokeColor: "transparent"
            fillColor: root.color
            PathSvg { path: root.shapePath(root.morph) }
        }
    }

    Rectangle {
        visible: Theme.reducedMotion
        anchors.centerIn: parent
        width: root.size * 0.5; height: width; radius: width / 2
        color: root.color
        SequentialAnimation on opacity {
            running: root.running && root.visible && Theme.reducedMotion
            loops: Animation.Infinite
            NumberAnimation { to: 0.35; duration: 900 }
            NumberAnimation { to: 1; duration: 900 }
        }
    }
}
