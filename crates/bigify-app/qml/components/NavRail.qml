import QtQuick
import Bigify

// Navigation rail. Main destinations sit at the top under the logo;
// destinations marked `bottom: true` (Playing, Settings) are pinned to the
// bottom edge. Up/Down move through all of them in order.
//
// Moving focus between destinations switches the visible section
// immediately (TV convention); Right or A enters the content. Destinations
// with `overlay: true` (Now Playing) open only on Right/A.
FocusScope {
    id: root
    property var destinations: []          // [{ id, label, icon, overlay?, bottom? }]
    property string current: "home"
    signal navigate(string id)
    signal enterContent(string id)

    width: Theme.size.navRail

    property int currentIndex: Math.max(0, destinations.findIndex(d => d.id === current))
    readonly property int bottomCount: destinations.filter(d => d.bottom).length
    readonly property int itemStep: Theme.size.navItem + Theme.space.l

    // Follow section changes made elsewhere (e.g. Back to Home) unless the
    // user is currently moving through the rail.
    onCurrentChanged: if (!activeFocus) currentIndex = Math.max(0, destinations.findIndex(d => d.id === current))

    function moveFocus(step) {
        const next = currentIndex + step
        if (next < 0 || next >= destinations.length) return
        currentIndex = next
        const d = destinations[next]
        if (!d.overlay) root.navigate(d.id)
    }

    Rectangle {
        anchors.fill: parent
        color: root.activeFocus ? Theme.surfaceContainerLow : "transparent"
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
    }

    LogoMark {
        id: logo
        anchors { top: parent.top; topMargin: Theme.safeV - Theme.dp(8); horizontalCenter: parent.horizontalCenter }
        size: Theme.dp(64)
    }

    Repeater {
        model: root.destinations

        delegate: Item {
            id: navItem
            required property var modelData
            required property int index
            readonly property bool selected: root.current === modelData.id
            // Position within the top or bottom group.
            readonly property int groupIndex: root.destinations.slice(0, index)
                .filter(d => !!d.bottom === !!modelData.bottom).length

            width: root.width
            height: Theme.size.navItem
            y: modelData.bottom
               ? root.height - Theme.safeV - (root.bottomCount - groupIndex) * root.itemStep + Theme.space.l
               : logo.y + logo.height + Theme.space.xxxl + groupIndex * root.itemStep
            focus: index === root.currentIndex
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

            MouseArea {
                anchors.fill: parent
                onClicked: {
                    root.currentIndex = navItem.index
                    navItem.forceActiveFocus()
                    if (!navItem.modelData.overlay) root.navigate(navItem.modelData.id)
                    root.enterContent(navItem.modelData.id)
                }
            }
        }
    }

    Keys.onUpPressed: moveFocus(-1)
    Keys.onDownPressed: moveFocus(1)
    Keys.onRightPressed: if (destinations[currentIndex]) root.enterContent(destinations[currentIndex].id)
    Keys.onReturnPressed: if (destinations[currentIndex]) root.enterContent(destinations[currentIndex].id)
    Keys.onEnterPressed: if (destinations[currentIndex]) root.enterContent(destinations[currentIndex].id)
}
