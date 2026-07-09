import QtQuick
import "../themes" as Theme

Item {
    id: root
    property string title: ""
    property string detail: ""
    property string badge: ""
    property color badgeColor: Theme.DarkTheme.accent

    implicitHeight: 38

    Row {
        anchors.fill: parent
        spacing: 8

        Column {
            width: Math.max(80, parent.width - badgeBox.width - 8)
            height: parent.height
            spacing: 1

            Text {
                width: parent.width
                height: 19
                text: root.title
                color: Theme.DarkTheme.text
                font.pixelSize: 15
                font.bold: true
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                height: 16
                text: root.detail
                color: Theme.DarkTheme.muted
                font.pixelSize: 11
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }

        Rectangle {
            id: badgeBox
            visible: root.badge.length > 0
            width: Math.max(54, badgeText.implicitWidth + 16)
            height: 24
            anchors.verticalCenter: parent.verticalCenter
            radius: 12
            color: Qt.rgba(root.badgeColor.r, root.badgeColor.g, root.badgeColor.b, 0.16)
            border.color: root.badgeColor
            border.width: 1

            Text {
                id: badgeText
                anchors.centerIn: parent
                text: root.badge
                color: root.badgeColor
                font.pixelSize: 11
                font.bold: true
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
