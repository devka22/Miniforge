import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    function runCommand(commandId) {
        if (typeof editorController !== "undefined")
            editorController.executeCommand(commandId)
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        MfPanelHeader {
            width: parent.width
            title: "Object Studio"
            detail: "Fast creation for common engine objects"
            badge: "Objects"
            badgeColor: Theme.DarkTheme.accent
        }

        Repeater {
            model: [
                ["Sprite Actor", "SpriteRenderer + Animator2D", "object.create_sprite_actor"],
                ["Camera Rig", "Camera2D setup for scenes", "object.create_camera"],
                ["HUD Text", "UI text object for gameplay feedback", "object.create_ui_text"]
            ]

            Rectangle {
                width: parent.width
                height: 62
                radius: Theme.DarkTheme.cardRadius
                color: mouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.surface
                border.color: mouse.containsMouse ? Theme.DarkTheme.focus : Theme.DarkTheme.borderSoft
                border.width: 1

                Row {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 8

                    Column {
                        width: parent.width - createButton.width - 8
                        height: parent.height
                        spacing: 4

                        Text {
                            width: parent.width
                            text: modelData[0]
                            color: Theme.DarkTheme.text
                            font.pixelSize: 13
                            font.bold: true
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width
                            text: modelData[1]
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                    }

                    MfButton {
                        id: createButton
                        width: 82
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Create"
                        accent: index === 0
                        onClicked: root.runCommand(modelData[2])
                    }
                }

                MouseArea {
                    id: mouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
        }
    }
}
