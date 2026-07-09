import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    color: Theme.DarkTheme.background

    Column {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Console"
            detail: consoleList.count + " runtime entries"
            badge: editorBridge.lastError.length > 0 ? "Error" : "Live"
            badgeColor: editorBridge.lastError.length > 0 ? Theme.DarkTheme.danger : Theme.DarkTheme.accent
        }

        ListView {
            id: consoleList
            width: parent.width
            height: parent.height - y
            clip: true
            spacing: 3
            model: consoleModel

            delegate: Rectangle {
                id: consoleEntry
                required property int index
                required property var model

                width: ListView.view.width
                height: Theme.DarkTheme.compactRowHeight
                radius: Theme.DarkTheme.radius
                color: consoleEntry.index % 2 === 0 ? Theme.DarkTheme.surface : Theme.DarkTheme.panel
                border.color: consoleEntry.model.severity >= 3 ? Theme.DarkTheme.dangerSoft : Theme.DarkTheme.borderSoft
                border.width: 1

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 8

                    Text {
                        width: 56
                        height: parent.height
                        text: "#" + consoleEntry.model.frame
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 10
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        width: 86
                        height: parent.height
                        text: consoleEntry.model.channel
                        color: consoleEntry.model.severity >= 3 ? Theme.DarkTheme.danger : (consoleEntry.model.severity >= 2 ? Theme.DarkTheme.warning : Theme.DarkTheme.accent)
                        font.pixelSize: 11
                        font.bold: true
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width - x
                        height: parent.height
                        text: consoleEntry.model.message
                        color: Theme.DarkTheme.text
                        font.pixelSize: 12
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }
                }
            }

            Text {
                visible: consoleList.count === 0
                anchors.centerIn: parent
                text: "No console entries"
                color: Theme.DarkTheme.muted
                font.pixelSize: 12
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
