import QtQuick
import Bigify

// A titled horizontal row of cards. Left/Right move within the row; at the
// left edge the key propagates (so the screen can hand focus to the rail).
// The row remembers its position when focus leaves and returns.
FocusScope {
    id: root
    property string title: ""
    property var items: []
    property real cardWidth: Theme.size.card
    property int leftInset: 0
    signal itemActivated(var item, int index)

    implicitHeight: header.height + Theme.space.xl + list.height
    width: parent ? parent.width : 0

    Text {
        id: header
        x: root.leftInset
        text: root.title
        color: root.activeFocus ? Theme.surfaceContent : Theme.surfaceContentVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.headlineSmall
        font.weight: Theme.weightDisplay
        Behavior on color { ColorAnimation { duration: Theme.motion.fade } }
    }

    ListView {
        id: list
        anchors { top: header.bottom; topMargin: Theme.space.xl; left: parent.left; right: parent.right }
        // Room for the focus lift and ring above/below the cards.
        height: cardWidth + Theme.dp(110)
        topMargin: Theme.dp(16)
        orientation: ListView.Horizontal
        spacing: Theme.space.xxl
        focus: true
        clip: false
        model: root.items
        leftMargin: root.leftInset
        rightMargin: Theme.safeH
        keyNavigationEnabled: true
        highlightFollowsCurrentItem: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: root.leftInset
        preferredHighlightEnd: width - Theme.safeH - root.cardWidth
        highlightMoveDuration: Theme.motion.slow
        highlightMoveVelocity: -1
        boundsBehavior: Flickable.StopAtBounds
        cacheBuffer: root.cardWidth * 4
        reuseItems: true

        delegate: MediaCard {
            required property var modelData
            required property int index
            item: modelData
            cardWidth: root.cardWidth
            focus: ListView.isCurrentItem
            onActivated: root.itemActivated(modelData, index)
        }
    }
}
