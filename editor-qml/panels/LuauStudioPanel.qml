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
            title: "Luau Studio"
            detail: "Scripts, snippets, diagnostics and hot reload workflow"
            badge: "Code"
            badgeColor: Theme.DarkTheme.info
        }

        Row {
            width: parent.width
            height: 34
            spacing: 8

            MfButton {
                width: 132
                text: "New Controller"
                accent: true
                onClicked: root.runCommand("luau.new_controller")
            }

            MfButton {
                width: 120
                text: "Validate"
                onClicked: root.runCommand("luau.validate_scripts")
            }

            MfButton {
                width: 126
                text: "Attach Actor"
                onClicked: root.runCommand("object.create_sprite_actor")
            }
        }

        Row {
            width: parent.width
            height: parent.height - y
            spacing: 10

            Rectangle {
                width: Math.min(420, parent.width * 0.52)
                height: parent.height
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.background
                border.color: Theme.DarkTheme.borderSoft
                border.width: 1

                Column {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 4

                    Repeater {
                        model: [
                            "local speed = 180.0",
                            "function on_update(dt: number)",
                            "    local x = Input.axis(\"A\", \"D\")",
                            "    move(x * speed * dt, 0.0)",
                            "end"
                        ]

                        Text {
                            width: parent.width
                            height: 20
                            text: modelData
                            color: index === 1 || index === 4 ? Theme.DarkTheme.accent : Theme.DarkTheme.text
                            font.family: "Menlo"
                            font.pixelSize: 12
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Column {
                width: parent.width - x
                height: parent.height
                spacing: 8

                Repeater {
                    model: [
                        ["Events", "on_start, on_update, on_fixed_update, on_event"],
                        ["Gameplay API", "Entity, Transform2D, Rigidbody2D, Camera, Events"],
                        ["Diagnostics", "Roblox globals, unsafe loops and dt hints"],
                        ["Snippets", "controller2d, sprite_state, projectile, particles"]
                    ]

                    Rectangle {
                        width: parent.width
                        height: 52
                        radius: Theme.DarkTheme.cardRadius
                        color: Theme.DarkTheme.surfaceRaised
                        border.color: Theme.DarkTheme.borderSoft
                        border.width: 1

                        Column {
                            anchors.fill: parent
                            anchors.margins: 8
                            spacing: 3

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
                    }
                }
            }
        }
    }
}
