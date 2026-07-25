import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "components"
import "panels" as Panels
import "themes" as Theme

ApplicationWindow {
    id: root

    width: 1440
    height: 920
    minimumWidth: 1120
    minimumHeight: 720
    visible: true
    color: Theme.DarkTheme.background
    title: "MiniForge 0.9.3.4 Qt Editor"

    function runCommand(commandId) {
        if (typeof editorController !== "undefined")
            editorController.executeCommand(commandId)
    }

    function openAuthoringHub(kind) {
        workspaceTabs.currentIndex = 1
        if (kind && kind.length > 0)
            authoringHub.selectKind(kind)
    }

    function openSdkPacks() {
        workspaceTabs.currentIndex = 2
    }

    readonly property string projectSummaryText: typeof editorBridge !== "undefined" && editorBridge.projectSummary.length > 0
        ? editorBridge.projectSummary
        : "MiniForge 0.9.3.4 | No project"

    menuBar: MenuBar {
        Menu {
            title: "File"
            Menu {
                title: "New"
                Action { text: "Pixel Art Sprite"; onTriggered: root.runCommand("sprite.new_pixel_art") }
                Action { text: "Hero Sprite Template"; onTriggered: root.runCommand("sprite.create_hero_template") }
                Action { text: "Luau 2D Controller"; onTriggered: root.runCommand("luau.new_controller") }
                Action { text: "Sprite Actor"; onTriggered: root.runCommand("object.create_sprite_actor") }
            }
            Action { text: "Save Project"; onTriggered: root.runCommand("project.save") }
            Action { text: "Save Scene"; onTriggered: root.runCommand("scene.save") }
        }

        Menu {
            title: "Edit"
            Action { text: "Undo"; onTriggered: root.runCommand("edit.undo") }
            Action { text: "Redo"; onTriggered: root.runCommand("edit.redo") }
            Menu {
                title: "Sprite Tools"
                Action { text: "Create SpriteFrames"; onTriggered: root.runCommand("sprite.export_frames") }
                Action { text: "Export Atlas Pages"; onTriggered: root.runCommand("sprite.export_atlas_pages") }
                Action { text: "Create Palette Ramp"; onTriggered: root.runCommand("sprite.optimize_palette") }
            }
        }

        Menu {
            title: "Objects"
            Action { text: "Sprite Actor"; onTriggered: root.runCommand("object.create_sprite_actor") }
            Action { text: "Camera Rig"; onTriggered: root.runCommand("object.create_camera") }
            Action { text: "HUD Text"; onTriggered: root.runCommand("object.create_ui_text") }
        }

        Menu {
            title: "Systems"
            Action { text: "Mega Authoring Hub"; onTriggered: root.openAuthoringHub("all") }
            MenuSeparator {}
            Action { text: "Players and Actors"; onTriggered: root.openAuthoringHub("actor") }
            Action { text: "Gameplay Systems"; onTriggered: root.openAuthoringHub("gameplay") }
            Action { text: "Physics Profiles"; onTriggered: root.openAuthoringHub("physics") }
            Action { text: "World Building"; onTriggered: root.openAuthoringHub("world") }
            Action { text: "Effects and Audio"; onTriggered: root.openAuthoringHub("effects") }
            Action { text: "User Interface"; onTriggered: root.openAuthoringHub("user_interface") }
            Action { text: "Strategy and RTS"; onTriggered: root.openAuthoringHub("strategy") }
        }

        Menu {
            title: "Project"
            Action { text: "Run Audit"; onTriggered: root.runCommand("project.audit") }
            Action { text: "Refresh Assets"; onTriggered: root.runCommand("assets.refresh") }
            Action { text: "Validate Luau Scripts"; onTriggered: root.runCommand("luau.validate_scripts") }
            Action { text: "Write 2D Render Profile"; onTriggered: root.runCommand("render.write_2d_profile") }
            MenuSeparator {}
            Action { text: "SDK & Content Packs"; onTriggered: root.openSdkPacks() }
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.DarkTheme.background

        Rectangle {
            id: topBar
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 88
            color: Theme.DarkTheme.panel
            border.color: Theme.DarkTheme.borderSoft
            border.width: 1

            Row {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 54
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                spacing: 10

                Column {
                    width: Math.max(240, parent.width - saveButton.width - undoButton.width
                        - redoButton.width - systemsButton.width - auditButton.width - 56)
                    height: parent.height
                    spacing: 2

                    Text {
                        width: parent.width
                        height: 27
                        text: "MiniForge 0.9.3.4"
                        color: Theme.DarkTheme.text
                        font.pixelSize: 16
                        font.bold: true
                        verticalAlignment: Text.AlignBottom
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        height: 20
                        text: root.projectSummaryText
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 11
                        verticalAlignment: Text.AlignTop
                        elide: Text.ElideRight
                    }
                }

                MfButton {
                    id: saveButton
                    width: 78
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Save"
                    onClicked: root.runCommand("project.save")
                }

                MfButton {
                    id: undoButton
                    width: 70
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Undo"
                    onClicked: root.runCommand("edit.undo")
                }

                MfButton {
                    id: redoButton
                    width: 70
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Redo"
                    onClicked: root.runCommand("edit.redo")
                }

                MfButton {
                    id: systemsButton
                    width: 92
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Systems"
                    onClicked: root.openAuthoringHub("all")
                }

                MfButton {
                    id: auditButton
                    width: 104
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Audit"
                    accent: true
                    onClicked: root.runCommand("project.audit")
                }
            }

            TabBar {
                id: workspaceTabs
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 34
                currentIndex: 0

                background: Rectangle {
                    color: Theme.DarkTheme.surface
                    border.color: Theme.DarkTheme.borderSoft
                    border.width: 1
                }

                Repeater {
                    model: ["Scene", "Systems", "SDK Packs", "Sprite", "Luau", "Objects", "Debug"]
                    TabButton {
                        text: modelData
                        width: Math.max(100, workspaceTabs.width / 7)
                    }
                }
            }
        }

        SplitView {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: topBar.bottom
            anchors.bottom: parent.bottom
            orientation: Qt.Horizontal

            Panels.HierarchyPanel {
                SplitView.preferredWidth: 300
                SplitView.minimumWidth: 240
            }

            Item {
                SplitView.fillWidth: true
                SplitView.minimumWidth: 520

                Panels.CommandPalette {
                    id: commandPalette
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 112
                }

                StackLayout {
                    id: workspaceStack
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: commandPalette.bottom
                    anchors.bottom: bottomTray.top
                    currentIndex: workspaceTabs.currentIndex

                    Rectangle {
                        id: viewport
                        color: Theme.DarkTheme.background
                        border.color: Theme.DarkTheme.borderSoft
                        border.width: 1
                        clip: true

                        Repeater {
                            model: 18
                            Rectangle {
                                x: index * viewport.width / 18
                                width: 1
                                height: viewport.height
                                color: Theme.DarkTheme.borderSoft
                                opacity: 0.34
                            }
                        }

                        Repeater {
                            model: 12
                            Rectangle {
                                y: index * viewport.height / 12
                                width: viewport.width
                                height: 1
                                color: Theme.DarkTheme.borderSoft
                                opacity: 0.34
                            }
                        }

                        Column {
                            anchors.centerIn: parent
                            width: Math.min(420, parent.width - 48)
                            spacing: 8

                            Text {
                                width: parent.width
                                text: "2D Viewport"
                                color: Theme.DarkTheme.text
                                font.pixelSize: 20
                                font.bold: true
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                            }

                            Text {
                                width: parent.width
                                text: root.projectSummaryText
                                color: Theme.DarkTheme.muted
                                font.pixelSize: 12
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                            }
                        }
                    }

                    Panels.AuthoringHubPanel {
                        id: authoringHub
                    }
                    Panels.SdkPacksPanel {}
                    Panels.SpriteStudioPanel {}
                    Panels.LuauStudioPanel {}
                    Panels.ObjectStudioPanel {}
                    Panels.ConsolePanel {}
                }

                SplitView {
                    id: bottomTray
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: Math.max(210, parent.height * 0.28)
                    orientation: Qt.Horizontal

                    Panels.ContentBrowserPanel {
                        SplitView.fillWidth: true
                        SplitView.minimumWidth: 360
                    }

                    Panels.ConsolePanel {
                        SplitView.preferredWidth: 430
                        SplitView.minimumWidth: 300
                    }
                }
            }

            SplitView {
                orientation: Qt.Vertical
                SplitView.preferredWidth: 360
                SplitView.minimumWidth: 300

                Panels.InspectorPanel {
                    SplitView.fillHeight: true
                    SplitView.minimumHeight: 280
                }

                Panels.ReadinessPanel {
                    SplitView.preferredHeight: 280
                    SplitView.minimumHeight: 220
                }
            }
        }
    }
}
