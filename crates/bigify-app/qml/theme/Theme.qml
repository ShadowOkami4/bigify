pragma Singleton
import QtQuick
import Bigify

// Bigify design tokens: Material 3 Expressive adapted for a 10-foot UI.
//
// Every size is authored for a 1920×1080 logical canvas and multiplied by
// `uiScale`, so the layout is identical at 720p, 1080p and 4K. Use `dp()` for
// any dimension. Colors are Material 3 role names; the scheme comes from the
// backend (brand seed or artwork-derived) and transitions smoothly.
Item {
    id: theme
    visible: false

    // ---- Scale -----------------------------------------------------------
    property real windowHeight: 1080
    property real windowWidth: 1920
    readonly property var settings: JSON.parse(Backend.settingsJson || "{}")
    readonly property real userScale: settings.uiScale || 1.0
    // Fit the 16:9 design canvas inside the window (handles 16:10 Deck).
    readonly property real uiScale: Math.max(0.5, Math.min(windowHeight / 1080, windowWidth / 1920)) * userScale
    function dp(v) { return Math.round(v * uiScale) }

    readonly property bool reducedMotion: settings.reducedMotion === true

    // ---- Color roles -----------------------------------------------------
    // Scheme changes (new artwork) cross-fade: every role is a mix of the
    // previous and the new scheme, driven by one animation. (Behaviors are
    // avoided here: Qt 6.8 crashes on deferred Behavior animations inside
    // QML singletons.)
    readonly property var scheme: JSON.parse(Backend.schemeJson)
    readonly property int colorDuration: reducedMotion ? 0 : 900
    property var fromScheme: scheme
    property var toScheme: scheme
    property real mixProgress: 1
    property bool ready: false
    Component.onCompleted: ready = true

    onSchemeChanged: {
        if (!ready || colorDuration === 0) {
            fromScheme = scheme
            toScheme = scheme
            mixProgress = 1
            return
        }
        const snapshot = {}
        for (const role of Object.keys(scheme))
        {
            const prop = theme.roleProperty[role] || role
            snapshot[role] = typeof theme[prop] === "object" ? theme[prop].toString() : scheme[role]
        }
        fromScheme = snapshot
        toScheme = scheme
        mixAnimation.restart()
    }

    NumberAnimation {
        id: mixAnimation
        target: theme
        property: "mixProgress"
        from: 0
        to: 1
        duration: Math.max(1, theme.colorDuration)
        easing.type: Easing.InOutQuad
    }

    // Material 3 role name -> Theme property. "on-X" roles are named
    // "XContent" because QML reserves `onFoo` names for signal handlers.
    readonly property var roleProperty: ({
        onPrimary: "primaryContent", onPrimaryContainer: "primaryContainerContent",
        onSecondaryContainer: "secondaryContainerContent", onTertiaryContainer: "tertiaryContainerContent",
        onSurface: "surfaceContent", onSurfaceVariant: "surfaceContentVariant",
        onError: "errorContent", focus: "focusColor"
    })

    function mix(role) {
        const a = Qt.color(fromScheme[role] || toScheme[role])
        const b = Qt.color(toScheme[role])
        const t = mixProgress
        return Qt.rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1)
    }

    readonly property color primary: mix("primary")
    readonly property color primaryContent: mix("onPrimary")
    readonly property color primaryContainer: mix("primaryContainer")
    readonly property color primaryContainerContent: mix("onPrimaryContainer")
    readonly property color secondary: mix("secondary")
    readonly property color secondaryContainer: mix("secondaryContainer")
    readonly property color secondaryContainerContent: mix("onSecondaryContainer")
    readonly property color tertiary: mix("tertiary")
    readonly property color tertiaryContainer: mix("tertiaryContainer")
    readonly property color tertiaryContainerContent: mix("onTertiaryContainer")
    readonly property color surface: mix("surface")
    readonly property color surfaceContainerLowest: mix("surfaceContainerLowest")
    readonly property color surfaceContainerLow: mix("surfaceContainerLow")
    readonly property color surfaceContainer: mix("surfaceContainer")
    readonly property color surfaceContainerHigh: mix("surfaceContainerHigh")
    readonly property color surfaceContainerHighest: mix("surfaceContainerHighest")
    readonly property color surfaceContent: mix("onSurface")
    readonly property color surfaceContentVariant: mix("onSurfaceVariant")
    readonly property color outline: mix("outline")
    readonly property color outlineVariant: mix("outlineVariant")
    readonly property color focusColor: mix("focus")
    readonly property color error: mix("error")
    readonly property color errorContent: mix("onError")
    readonly property color scrim: "#000000"

    // State layers (M3): overlay opacity on top of a container.
    readonly property real stateHover: 0.08
    readonly property real stateFocus: 0.12
    readonly property real statePressed: 0.16
    readonly property real disabledContent: 0.38
    readonly property real disabledContainer: 0.12

    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }

    // ---- Typography ------------------------------------------------------
    // M3 type scale × ~1.4 for viewing distance; nothing below 18 px @1080p.
    readonly property string fontFamily: {
        const wanted = ["Google Sans Flex", "Roboto Flex", "Google Sans", "Roboto",
                        "Inter", "Noto Sans", "Cantarell", "DejaVu Sans"]
        const available = Qt.fontFamilies()
        for (const f of wanted)
            if (available.indexOf(f) >= 0) return f
        return Qt.application.font.family
    }

    readonly property var type: ({
        displayLarge: theme.dp(88),
        displayMedium: theme.dp(64),
        displaySmall: theme.dp(52),
        headlineLarge: theme.dp(44),
        headlineMedium: theme.dp(38),
        headlineSmall: theme.dp(32),
        titleLarge: theme.dp(30),
        titleMedium: theme.dp(25),
        titleSmall: theme.dp(22),
        bodyLarge: theme.dp(24),
        bodyMedium: theme.dp(21),
        labelLarge: theme.dp(22),
        labelMedium: theme.dp(19),
        labelSmall: theme.dp(18)
    })
    // Expressive emphasis: display/headline use heavier weights than baseline M3.
    readonly property int weightDisplay: Font.DemiBold
    readonly property int weightTitle: Font.DemiBold
    readonly property int weightBody: Font.Normal
    readonly property int weightLabel: Font.Medium

    // ---- Spacing (4 px grid) --------------------------------------------
    readonly property var space: ({
        xs: theme.dp(4),
        s: theme.dp(8),
        m: theme.dp(12),
        l: theme.dp(16),
        xl: theme.dp(24),
        xxl: theme.dp(32),
        xxxl: theme.dp(48),
        huge: theme.dp(64)
    })
    // TV title-safe area (overscan): ~3.5% horizontally, ~4.5% vertically.
    readonly property int safeH: dp(64)
    readonly property int safeV: dp(48)

    // ---- Shape (M3 Expressive corner scale) ------------------------------
    readonly property var shape: ({
        extraSmall: theme.dp(4),
        small: theme.dp(8),
        medium: theme.dp(12),
        large: theme.dp(16),
        largeIncreased: theme.dp(20),
        extraLarge: theme.dp(28),
        extraLargeIncreased: theme.dp(32),
        extraExtraLarge: theme.dp(48)
    })

    // ---- Motion ----------------------------------------------------------
    // Spatial motion may overshoot slightly (expressive); effects (color,
    // opacity) never do. Reduced motion removes spatial movement entirely.
    readonly property var motion: ({
        quick: theme.reducedMotion ? 0 : 150,
        medium: theme.reducedMotion ? 0 : 300,
        slow: theme.reducedMotion ? 0 : 500,
        fade: theme.reducedMotion ? 120 : 200,
        fadeLong: theme.reducedMotion ? 150 : 350,
        // Bezier curves for Easing.BezierSpline.
        standard: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0],
        emphasizedDecelerate: [0.05, 0.7, 0.1, 1.0, 1.0, 1.0],
        emphasizedAccelerate: [0.3, 0.0, 0.8, 0.15, 1.0, 1.0],
        // Expressive "fast spatial" spring, approximated by a gentle back-out.
        springOvershoot: theme.reducedMotion ? 0 : 1.3
    })

    // ---- Focus -----------------------------------------------------------
    readonly property int focusWidth: dp(4)
    readonly property int focusGap: dp(4)
    readonly property real focusScale: reducedMotion ? 1.0 : 1.06

    // ---- Component metrics ----------------------------------------------
    readonly property var size: ({
        navRail: theme.dp(128),
        navItem: theme.dp(88),
        card: theme.dp(248),
        cardLarge: theme.dp(320),
        button: theme.dp(72),
        buttonSmall: theme.dp(60),
        iconButton: theme.dp(80),
        playButton: theme.dp(128),
        icon: theme.dp(32),
        iconLarge: theme.dp(44),
        trackRow: theme.dp(96),
        miniPlayer: theme.dp(120),
        nowPlayingArt: theme.dp(620)
    })
}
