import QtQuick
import QtQuick.Controls
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel
    border.color: Theme.DarkTheme.border
    border.width: 1
    implicitWidth: 680
    implicitHeight: 440
    property string currentCommandId: ""
    property bool currentCommandEnabled: false

    function focusSearch() {
        commandSearch.text = ""
        commandList.currentIndex = commandList.count > 0 ? 0 : -1
        commandSearch.forceActiveFocus()
    }

    function closePalette() {
        commandModel.filter = ""
        if (typeof editorShell !== "undefined")
            editorShell.closeCommandPalette()
    }

    function dispatch(commandId) {
        if (commandId.indexOf("workspace.") === 0
                || commandId.indexOf("panel.") === 0
                || commandId.indexOf("view.") === 0) {
            if (typeof editorShell !== "undefined")
                editorShell.executeShellCommand(commandId)
        } else if (typeof editorController !== "undefined") {
            editorController.executeCommand(commandId)
        }
        closePalette()
    }

    function activateCurrent() {
        if (currentCommandEnabled && currentCommandId.length > 0)
            dispatch(currentCommandId)
    }

    Column {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Row {
            width: parent.width
            height: 32
            spacing: 8

            Text {
                width: parent.width - keyboardHint.width - 8
                height: parent.height
                text: "Command Palette"
                color: Theme.DarkTheme.text
                font.pixelSize: 16
                font.bold: true
                verticalAlignment: Text.AlignVCenter
            }

            Text {
                id: keyboardHint
                height: parent.height
                text: "↑↓ navigate   Enter run   Esc close"
                color: Theme.DarkTheme.muted
                font.pixelSize: 11
                verticalAlignment: Text.AlignVCenter
            }
        }

        MfSearchBar {
            id: commandSearch
            width: parent.width
            placeholderText: "Search commands, workspaces and panels..."
            onTextChanged: {
                commandModel.filter = text
                commandList.currentIndex = commandList.count > 0 ? 0 : -1
            }
            Keys.onPressed: function(event) {
                if (event.key === Qt.Key_Down) {
                    if (commandList.count > 0)
                        commandList.currentIndex = Math.min(commandList.count - 1, commandList.currentIndex + 1)
                    commandList.positionViewAtIndex(commandList.currentIndex, ListView.Contain)
                    event.accepted = true
                } else if (event.key === Qt.Key_Up) {
                    if (commandList.count > 0)
                        commandList.currentIndex = Math.max(0, commandList.currentIndex - 1)
                    commandList.positionViewAtIndex(commandList.currentIndex, ListView.Contain)
                    event.accepted = true
                } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                    root.activateCurrent()
                    event.accepted = true
                } else if (event.key === Qt.Key_Escape) {
                    root.closePalette()
                    event.accepted = true
                }
            }
        }

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.DarkTheme.borderSoft
        }

        ListView {
            id: commandList
            width: parent.width
            height: parent.height - y - footer.height - parent.spacing
            clip: true
            spacing: 2
            model: commandModel
            currentIndex: count > 0 ? 0 : -1
            keyNavigationWraps: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}

            delegate: Rectangle {
                id: commandRow
                required property string commandId
                required property var model
                width: commandList.width
                height: 42
                radius: Theme.DarkTheme.radius
                color: ListView.isCurrentItem
                    ? Theme.DarkTheme.accentSoft
                    : (rowMouse.containsMouse ? Theme.DarkTheme.panelAlt : "transparent")
                border.color: ListView.isCurrentItem ? Theme.DarkTheme.accent : "transparent"
                border.width: 1
                opacity: commandRow.model.enabled ? 1.0 : 0.45

                function activateCommand() {
                    if (commandRow.model.enabled)
                        root.dispatch(commandRow.commandId)
                }

                function syncCurrentCommand() {
                    if (ListView.isCurrentItem) {
                        root.currentCommandId = commandRow.commandId
                        root.currentCommandEnabled = commandRow.model.enabled
                    }
                }

                ListView.onIsCurrentItemChanged: syncCurrentCommand()
                Component.onCompleted: syncCurrentCommand()

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 11
                    anchors.rightMargin: 11
                    spacing: 10

                    Rectangle {
                        width: 88
                        height: 22
                        anchors.verticalCenter: parent.verticalCenter
                        radius: 3
                        color: Theme.DarkTheme.surfaceRaised
                        border.color: Theme.DarkTheme.borderSoft

                        Text {
                            anchors.fill: parent
                            anchors.leftMargin: 7
                            anchors.rightMargin: 7
                            text: commandRow.model.category
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 10
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        width: parent.width - x - shortcutText.width
                        height: parent.height
                        text: commandRow.model.label
                        color: Theme.DarkTheme.text
                        font.pixelSize: 13
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        id: shortcutText
                        width: Math.max(64, implicitWidth)
                        height: parent.height
                        text: commandRow.model.shortcut
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onEntered: commandList.currentIndex = index
                    onClicked: commandRow.activateCommand()
                }
            }
        }

        Text {
            id: footer
            width: parent.width
            height: 18
            text: commandList.count + " matching actions · Ctrl+Shift+P"
            color: Theme.DarkTheme.muted
            font.pixelSize: 11
            verticalAlignment: Text.AlignVCenter
        }
    }
}
