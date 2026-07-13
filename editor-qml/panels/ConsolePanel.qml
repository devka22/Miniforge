import QtQuick
import QtQuick.Controls
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

        Row {
            width: parent.width
            height: 32
            spacing: 7

            MfSearchBar {
                width: Math.max(80, parent.width - severityFilter.width - latestButton.width - clearButton.width - errorsButton.width - 35)
                placeholderText: "Filter channel, message or frame"
                onTextChanged: consoleModel.filter = text
            }

            ComboBox {
                id: severityFilter
                width: 104
                height: 32
                model: ["All", "Info+", "Warning+", "Errors"]
                onCurrentIndexChanged: consoleModel.minimumSeverity = currentIndex

                contentItem: Text {
                    leftPadding: 9
                    text: severityFilter.displayText
                    color: Theme.DarkTheme.text
                    font.pixelSize: 11
                    verticalAlignment: Text.AlignVCenter
                }

                background: Rectangle {
                    radius: Theme.DarkTheme.radius
                    color: Theme.DarkTheme.surface
                    border.color: severityFilter.activeFocus ? Theme.DarkTheme.focus : Theme.DarkTheme.border
                    border.width: 1
                }
            }

            MfButton {
                id: latestButton
                width: 64
                text: "Latest"
                enabled: consoleList.count > 0
                onClicked: consoleList.positionViewAtEnd()
            }
            MfButton {
                id: clearButton
                width: 58
                text: "Clear"
                onClicked: editorBridge.executeCommand("console.clear")
            }
            MfButton {
                id: errorsButton
                width: 86
                text: "Clear Errors"
                onClicked: editorBridge.executeCommand("console.clear_errors")
            }
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
                text: editorBridge.projectOpen ? "No matching console entries" : "Open a project to view runtime events"
                color: Theme.DarkTheme.muted
                font.pixelSize: 12
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
