import QtQuick
import Bigify

// Home: a featured "jump back in" hero followed by shelves built only from
// data Spotify returns for this account (no invented recommendations).
FocusScope {
    id: root
    signal itemActivated(var item)
    signal playRequested(var item)

    property var sections: []
    property string status: "loading"     // loading | ready | empty | error
    property string errorKind: ""
    property string errorMessage: ""
    property bool loadedOnce: false

    readonly property var heroItem: sections.length > 0 && sections[0].items.length > 0 ? sections[0].items[0] : null
    readonly property var rows: heroItem ? [{ id: "hero", layout: "hero" }].concat(sections) : sections

    function reload() {
        if (Backend.designPreview) {
            sections = PreviewData.homeSections
            status = "ready"
            return
        }
        status = loadedOnce ? status : "loading"
        Backend.loadHome()
    }

    function greeting() {
        const h = new Date().getHours()
        if (h < 5) return qsTr("Good night")
        if (h < 12) return qsTr("Good morning")
        if (h < 18) return qsTr("Good afternoon")
        return qsTr("Good evening")
    }

    Connections {
        target: Backend
        function onHomeLoaded(json) {
            const parsed = JSON.parse(json)
            root.sections = parsed
            root.loadedOnce = true
            root.status = parsed.length > 0 ? "ready" : "empty"
        }
        function onHomeFailed(kind, message) {
            root.errorKind = kind
            root.errorMessage = message
            if (!root.loadedOnce) root.status = "error"
        }
    }

    Text {
        id: title
        x: Theme.safeH
        y: Theme.safeV
        text: root.greeting()
        color: Theme.surfaceContent
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.displaySmall
        font.weight: Theme.weightDisplay
    }

    StatePanel {
        anchors.fill: parent
        visible: root.status !== "ready"
        focus: visible
        state_: root.status === "empty" ? "empty" : root.status
        title: root.status === "empty" ? qsTr("Nothing here yet")
             : root.status === "error" ? qsTr("Couldn't load Home") : ""
        message: root.status === "empty"
                 ? qsTr("Play something with Search or the Spotify app on your phone, and your recent music will show up here.")
                 : root.status === "error" ? root.errorMessage : ""
        errorKind: root.errorKind
        icon: root.status === "empty" ? "home" : ""
        onRetry: root.reload()
    }

    ListView {
        id: list
        visible: root.status === "ready"
        focus: visible
        anchors { top: title.bottom; topMargin: Theme.space.xxl; left: parent.left; right: parent.right; bottom: parent.bottom }
        clip: true
        model: root.rows
        spacing: Theme.space.xxxl
        keyNavigationEnabled: true
        highlightFollowsCurrentItem: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: Theme.dp(20)
        preferredHighlightEnd: height * 0.45
        highlightMoveDuration: Theme.motion.slow
        highlightMoveVelocity: -1
        boundsBehavior: Flickable.StopAtBounds
        bottomMargin: Theme.safeV + Theme.size.miniPlayer
        cacheBuffer: Theme.dp(1200)

        delegate: Loader {
            id: rowLoader
            required property var modelData
            required property int index
            width: list.width
            focus: ListView.isCurrentItem
            sourceComponent: modelData.layout === "hero" ? heroComponent : shelfComponent

            Component {
                id: shelfComponent
                Shelf {
                    title: rowLoader.modelData.title
                    items: rowLoader.modelData.items
                    leftInset: Theme.safeH
                    focus: true
                    onItemActivated: (item, i) => root.itemActivated(item)
                }
            }
            Component {
                id: heroComponent
                Hero {
                    item: root.heroItem
                    focus: true
                    onOpen: root.itemActivated(root.heroItem)
                    onPlay: root.playRequested(root.heroItem)
                }
            }
        }
    }

    // Featured item: large artwork, expressive type and the two key actions.
    component Hero: FocusScope {
        id: hero
        property var item: ({})
        signal open()
        signal play()
        height: Theme.dp(380)

        Rectangle {
            anchors { fill: parent; leftMargin: Theme.safeH; rightMargin: Theme.safeH }
            radius: Theme.shape.extraExtraLarge
            color: Theme.surfaceContainer

            Artwork {
                id: heroArt
                anchors { left: parent.left; top: parent.top; bottom: parent.bottom; margins: Theme.space.xxl }
                width: height
                source: hero.item ? (hero.item.artworkUrl || "") : ""
                kind: hero.item ? hero.item.kind : "album"
                circular: hero.item && hero.item.kind === "artist"
                radius: Theme.shape.extraLarge
            }
            Column {
                anchors {
                    left: heroArt.right; leftMargin: Theme.space.xxxl
                    right: parent.right; rightMargin: Theme.space.xxxl
                    verticalCenter: parent.verticalCenter
                }
                spacing: Theme.space.m
                Text {
                    text: qsTr("JUMP BACK IN")
                    color: Theme.primary
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelLarge
                    font.weight: Font.Bold
                    font.letterSpacing: Theme.dp(2)
                }
                Text {
                    width: parent.width
                    text: hero.item ? hero.item.title : ""
                    color: Theme.surfaceContent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.displaySmall
                    font.weight: Font.Bold
                    elide: Text.ElideRight
                    maximumLineCount: 2
                    wrapMode: Text.WordWrap
                }
                Text {
                    width: parent.width
                    text: hero.item ? hero.item.subtitle : ""
                    color: Theme.surfaceContentVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.titleMedium
                    elide: Text.ElideRight
                }
                Item { width: 1; height: Theme.space.m }
                Row {
                    spacing: Theme.space.l
                    PillButton {
                        id: playButton
                        focus: true
                        text: qsTr("Play")
                        icon: "play"
                        busy: /\bload\b/.test(Backend.pending)
                        onClicked: hero.play()
                        KeyNavigation.right: openButton
                    }
                    PillButton {
                        id: openButton
                        visible: hero.item && (hero.item.kind === "album" || hero.item.kind === "playlist")
                        text: qsTr("Open")
                        variant: "tonal"
                        icon: hero.item && hero.item.kind === "playlist" ? "playlist" : "album"
                        onClicked: hero.open()
                    }
                }
            }
        }
    }
}
