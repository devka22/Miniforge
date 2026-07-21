import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property string statusText: editorBridge.projectOpen
        ? "Select a project-relative asset operation"
        : "Open a project to manage assets"
    property bool busy: false

    function run(action, payload) {
        if (busy || !editorBridge.projectOpen)
            return
        busy = true
        var ok = editorBridge.manageAsset(action, JSON.stringify(payload))
        statusText = ok
            ? action.charAt(0).toUpperCase() + action.slice(1) + " completed · metadata refreshed"
            : editorBridge.lastError
        busy = false
    }

    Connections {
        target: editorBridge
        function onProjectChanged() {
            root.statusText = editorBridge.projectOpen
                ? "Asset management ready"
                : "Open a project to manage assets"
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Asset Management"
            detail: root.statusText
            badge: root.busy ? "Working" : "Safe ops"
            badgeColor: root.busy ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: sourceLayout.implicitHeight + 20
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.border

            GridLayout {
                id: sourceLayout
                anchors.fill: parent
                anchors.margins: 10
                columns: 2
                columnSpacing: 8
                rowSpacing: 7

                Label { text: "Asset"; color: Theme.DarkTheme.muted }
                TextField {
                    id: sourcePath
                    Layout.fillWidth: true
                    placeholderText: "assets/sprites/hero.png"
                    selectByMouse: true
                }
                Label { text: "New name"; color: Theme.DarkTheme.muted }
                TextField {
                    id: newName
                    Layout.fillWidth: true
                    placeholderText: "HeroIdle.png"
                    selectByMouse: true
                }
                Label { text: "Target folder"; color: Theme.DarkTheme.muted }
                TextField {
                    id: targetFolder
                    Layout.fillWidth: true
                    text: "assets"
                    selectByMouse: true
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 7

            MfButton {
                text: "Rename"
                enabled: !root.busy && sourcePath.text.length > 0 && newName.text.length > 0
                onClicked: root.run("rename", {
                    "source": sourcePath.text.trim(),
                    "new_name": newName.text.trim()
                })
            }
            MfButton {
                text: "Duplicate"
                enabled: !root.busy && sourcePath.text.length > 0
                onClicked: root.run("duplicate", {
                    "source": sourcePath.text.trim(),
                    "target_folder": targetFolder.text.trim()
                })
            }
            MfButton {
                text: "Move"
                accent: true
                enabled: !root.busy && sourcePath.text.length > 0 && targetFolder.text.length > 0
                onClicked: root.run("move", {
                    "source": sourcePath.text.trim(),
                    "target_folder": targetFolder.text.trim()
                })
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: importLayout.implicitHeight + 20
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.border

            ColumnLayout {
                id: importLayout
                anchors.fill: parent
                anchors.margins: 10
                spacing: 7

                Label { text: "Import external file"; color: Theme.DarkTheme.text; font.bold: true }
                TextField {
                    id: externalPath
                    Layout.fillWidth: true
                    placeholderText: "/absolute/path/to/asset.png"
                    selectByMouse: true
                }
                RowLayout {
                    Layout.fillWidth: true
                    Label {
                        Layout.fillWidth: true
                        text: "Creates an import sidecar and refreshes GUID metadata"
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 10
                    }
                    MfButton {
                        text: "Import"
                        accent: true
                        enabled: !root.busy && externalPath.text.length > 0 && targetFolder.text.length > 0
                        onClicked: root.run("import", {
                            "source_external": externalPath.text.trim(),
                            "target_folder": targetFolder.text.trim()
                        })
                    }
                }
            }
        }

        Item { Layout.fillHeight: true }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: deleteLayout.implicitHeight + 20
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.danger

            RowLayout {
                id: deleteLayout
                anchors.fill: parent
                anchors.margins: 10
                spacing: 8

                CheckBox { id: confirmDelete; text: "Confirm trash" }
                CheckBox { id: forceDelete; text: "Force referenced asset" }
                Item { Layout.fillWidth: true }
                MfButton {
                    text: "Move to Trash"
                    enabled: !root.busy && confirmDelete.checked && sourcePath.text.length > 0
                    onClicked: root.run("delete", {
                        "source": sourcePath.text.trim(),
                        "confirm": confirmDelete.checked,
                        "force": forceDelete.checked
                    })
                }
            }
        }
    }
}
