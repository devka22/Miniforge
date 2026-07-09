import QtQuick
import QtQuick.Controls
import "../themes" as Theme

Button {
    id: root
    property bool accent: false

    implicitHeight: 30
    leftPadding: 12
    rightPadding: 12

    contentItem: Text {
        text: root.text
        color: root.enabled ? (root.accent ? Theme.DarkTheme.background : Theme.DarkTheme.text) : Theme.DarkTheme.muted
        font.pixelSize: 13
        font.bold: root.accent
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: Theme.DarkTheme.radius
        color: root.accent
            ? (root.down ? Theme.DarkTheme.accentSoft : Theme.DarkTheme.accent)
            : (root.down ? Theme.DarkTheme.border : (root.hovered ? Theme.DarkTheme.panelAlt : Theme.DarkTheme.panel))
        border.color: root.activeFocus ? Theme.DarkTheme.focus : (root.accent ? Theme.DarkTheme.accent : Theme.DarkTheme.border)
        border.width: 1
    }
}
