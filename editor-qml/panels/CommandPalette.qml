import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    color: Theme.DarkTheme.panel
    implicitHeight: 112

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Command Palette"
            detail: commandList.count + " available actions"
            badge: "Qt"
            badgeColor: Theme.DarkTheme.info
        }

        Row {
            width: parent.width
            height: 34
            spacing: 8

            MfSearchBar {
                id: commandSearch
                width: Math.min(280, Math.max(180, parent.width * 0.28))
                placeholderText: "Command"
                onTextChanged: commandModel.filter = text
            }

            ListView {
                id: commandList
                width: parent.width - x
                height: parent.height
                orientation: ListView.Horizontal
                spacing: 8
                clip: true
                model: commandModel

                delegate: MfButton {
                    id: commandButton
                    required property string commandId
                    required property var model

                    width: Math.min(260, Math.max(132, commandButton.model.label.length * 8 + commandButton.model.shortcut.length * 6))
                    text: commandButton.model.shortcut.length > 0 ? commandButton.model.label + "  " + commandButton.model.shortcut : commandButton.model.label
                    enabled: commandButton.model.enabled
                    onClicked: editorController.executeCommand(commandButton.commandId)
                }
            }
        }
    }
}
