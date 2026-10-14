import QtQuick
import Bigify

// Local playback-position estimate. Extrapolates from the last *confirmed*
// position only while the engine confirmed it is playing, and only while a
// consumer is visible — no polling when nothing shows progress.
Item {
    id: root
    property bool active: true
    property real now: Date.now()

    readonly property real position: {
        if (Backend.durationMs <= 0) return 0
        const base = Backend.positionMs
        const elapsed = Backend.isPlaying ? Math.max(0, now - Backend.positionSampledAt) : 0
        return Math.min(Backend.durationMs, base + elapsed)
    }
    readonly property real fraction: Backend.durationMs > 0 ? position / Backend.durationMs : 0

    Timer {
        interval: 250
        repeat: true
        running: root.active && Backend.isPlaying
        triggeredOnStart: true
        onTriggered: root.now = Date.now()
    }
    Connections {
        target: Backend
        function onPositionMsChanged() { root.now = Date.now() }
    }

    function format(ms) {
        const s = Math.max(0, Math.floor(ms / 1000))
        const h = Math.floor(s / 3600)
        const m = Math.floor((s % 3600) / 60)
        const sec = String(s % 60).padStart(2, "0")
        return h > 0 ? h + ":" + String(m).padStart(2, "0") + ":" + sec : m + ":" + sec
    }
}
