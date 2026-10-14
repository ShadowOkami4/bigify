import QtQuick
import Bigify

// Navigation rail. Moving focus between destinations switches the visible
// section immediately (TV convention); Right or A enters the content.
// Destinations with `overlay: true` (Now Playing) open only on Right/A.
FocusScope {
    id: root
    property var destinations: []          // [{ id, label, icon }]
    property string current: "home"
    signal navigate(string id)
    signal enterContent(string id)

    width: Theme.size.navRail

    Rectangle {
        anchors.fill: parent
        color: root.activeFocus ? Theme.surfaceContainerLow : "transparent"
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
    }

    // Wordmark
    Text {
        id: mark
        anchors { top: parent.top; topMargin: Theme.safeV; horizontalCenter: parent.horizontalCenter }
        text: "B"
        color: Theme.primary
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.displaySmall
        font.weight: Font.Black
        Accessible.ignored: true
    }

    ListView {
        id: list
        anchors { top: mark.bottom; topMargin: Theme.space.xxxl; bottom: parent.bottom; left: parent.left; right: parent.right }
        spacing: Theme.space.l
        focus: true
        interactive: false
        keyNavigationEnabled: true
        model: root.destinations
        currentIndex: Math.max(0, root.destinations.findIndex(d => d.id === root.current))
        onCurrentIndexChanged: {
            const d = root.destinations[currentIndex]
            if (activeFocus && d && !d.overlay) root.navigate(d.id)
        }

        delegate: Item {
            id: navItem
            required property var modelData
            required property int index
            readonly property bool selected: root.current === modelData.id
            width: list.width
            height: Theme.size.navItem
            focus: ListView.isCurrentItem
            activeFocusOnTab: true
            Accessible.role: Accessible.PageTab
            Accessible.name: modelData.label
            Accessible.selected: selected

            // M3 rail: the active indicator is a pill behind the icon.
            Rectangle {
                id: indicator
                anchors { horizontalCenter: parent.horizontalCenter; top: parent.top }
                height: Theme.dp(52)
                width: navItem.selected ? Theme.dp(88) : Theme.dp(56)
                radius: height / 2
                color: navItem.selected ? Theme.secondaryContainer
                     : (navItem.activeFocus ? Theme.alpha(Theme.surfaceContent, Theme.stateFocus) : "transparent")
                Behavior on width { NumberAnimation { duration: Theme.motion.medium; easing.type: Easing.OutBack; easing.overshoot: Theme.motion.springOvershoot } }
                Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
                Icon {
                    anchors.centerIn: parent
                    name: navItem.modelData.icon
                    size: Theme.dp(32)
                    color: navItem.selected ? Theme.secondaryContainerContent : Theme.surfaceContentVariant
                }
                FocusRing { shown: navItem.activeFocus; radius: indicator.radius; gap: Theme.dp(3) }
            }
            Text {
                anchors { top: indicator.bottom; topMargin: Theme.space.xs; horizontalCenter: parent.horizontalCenter }
                text: navItem.modelData.label
                color: navItem.selected || navItem.activeFocus ? Theme.surfaceContent : Theme.surfaceContentVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall
                font.weight: navItem.selected ? Font.Bold : Font.Medium
            }

            Keys.onReturnPressed: root.enterContent(modelData.id)
            Keys.onEnterPressed: root.enterContent(modelData.id)
            MouseArea {
                anchors.fill: parent
                onClicked: { list.currentIndex = navItem.index; root.navigate(navItem.modelData.id); root.enterContent(navItem.modelData.id) }
            }
        }
    }

    Keys.onRightPressed: if (destinations[list.currentIndex]) root.enterContent(destinations[list.currentIndex].id)
}
