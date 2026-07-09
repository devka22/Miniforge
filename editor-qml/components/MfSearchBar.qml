import QtQuick
import QtQuick.Controls
import "../themes" as Theme

TextField {
    id: root
    placeholderText: "Search"
    color: Theme.DarkTheme.text
    placeholderTextColor: Theme.DarkTheme.muted
    selectionColor: Theme.DarkTheme.focus
    selectedTextColor: Theme.DarkTheme.background
    font.pixelSize: 13
    implicitHeight: 32
    leftPadding: 10
    rightPadding: 10

    background: Rectangle {
        radius: 4
        color: Theme.DarkTheme.background
        border.color: root.activeFocus ? Theme.DarkTheme.focus : Theme.DarkTheme.border
        border.width: 1
    }
}
