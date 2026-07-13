import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property bool loading: false
    property string statusText: "Create, discover or repair MiniForge projects"
    property string selectedRecent: ""
    property var launcherSettings: ({})

    function projectDisplayName(path) {
        var normalized = String(path || "").replace(/\\/g, "/")
        var parts = normalized.split("/")
        return parts.length > 0 && parts[parts.length - 1].length > 0
            ? parts[parts.length - 1]
            : normalized
    }

    function templateDescription(name) {
        switch (String(name || "")) {
        case "TopDown": return "Player, camera, input and a ready top-down scene."
        case "Platformer": return "Platform movement, collisions, camera and starter level."
        case "RTS": return "Units, resources, command center and RTS systems."
        default: return "Minimal project structure for a custom game architecture."
        }
    }

    function refreshLauncher() {
        loading = true
        var source = editorBridge.launcherSnapshotJson(workspaceRoot.text.trim())
        if (source.length === 0) {
            statusText = editorBridge.lastError || "Launcher state unavailable"
            loading = false
            return
        }
        try {
            var snapshot = JSON.parse(source)
            recentModel.clear()
            var recent = snapshot.recent_projects || []
            for (var index = 0; index < recent.length; ++index)
                recentModel.append({"path": String(recent[index])})
            templateBox.model = snapshot.templates || ["Empty", "TopDown", "Platformer", "RTS"]
            launcherSettings = snapshot.settings || {}
            if (selectedRecent.length === 0 && recent.length > 0)
                selectedRecent = String(recent[0])
            if (location.text.length === 0)
                location.text = snapshot.project_location || workspaceRoot.text
            statusText = recent.length + " recent project(s) · launcher state persisted"
        } catch (error) {
            statusText = "Invalid launcher snapshot · " + error
        }
        loading = false
    }

    function createProject() {
        var path = editorBridge.createProject(
            workspaceRoot.text.trim(),
            location.text.trim(),
            projectName.text.trim(),
            templateBox.currentText)
        if (path.length === 0) {
            statusText = editorBridge.lastError
            return
        }
        statusText = "Created " + path
        selectedRecent = path
        refreshLauncher()
    }

    function repairProject(path) {
        if (!path || path.length === 0)
            return
        var source = editorBridge.repairProjectJson(workspaceRoot.text.trim(), path)
        if (source.length === 0) {
            statusText = editorBridge.lastError
            return
        }
        try {
            var report = JSON.parse(source)
            var notes = report.notes || []
            statusText = "Repair complete · " + notes.length + " note(s)"
            repairDetails.text = notes.join("\n")
        } catch (error) {
            statusText = "Invalid repair report · " + error
        }
    }

    ListModel { id: recentModel }

    Component.onCompleted: refreshLauncher()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Project Launcher"
            detail: root.statusText
            badge: root.loading ? "Scanning" : "Native"
            badgeColor: root.loading ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: 8
            rowSpacing: 7

            Label { text: "Workspace root"; color: Theme.DarkTheme.muted }
            TextField { id: workspaceRoot; Layout.fillWidth: true; text: "projects"; placeholderText: "/path/to/workspace" }
            Label { text: "Project location"; color: Theme.DarkTheme.muted }
            TextField { id: location; Layout.fillWidth: true; text: "projects"; placeholderText: "/path/to/projects" }
            Label { text: "Project name"; color: Theme.DarkTheme.muted }
            TextField { id: projectName; Layout.fillWidth: true; text: "NewProject"; maximumLength: 80 }
            Label { text: "Template"; color: Theme.DarkTheme.muted }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                ComboBox { id: templateBox; Layout.fillWidth: true; model: ["Empty", "TopDown", "Platformer", "RTS"] }
                Label {
                    Layout.fillWidth: true
                    text: root.templateDescription(templateBox.currentText)
                    color: Theme.DarkTheme.muted
                    font.pixelSize: 10
                    wrapMode: Text.Wrap
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            Label { text: "Open policy"; color: Theme.DarkTheme.muted }
            Label {
                text: root.launcherSettings.safe_mode === true ? "Safe recovery" : "Normal runtime"
                color: root.launcherSettings.safe_mode === true ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
                font.bold: true
            }
            Label {
                text: root.launcherSettings.validate_on_open === false ? "Validation off" : "Validate on open"
                color: root.launcherSettings.validate_on_open === false ? Theme.DarkTheme.warning : Theme.DarkTheme.info
            }
            Label {
                text: root.launcherSettings.remember_recent === false ? "Private session" : "Remember recent"
                color: Theme.DarkTheme.muted
            }
            Item { Layout.fillWidth: true }
            Label {
                text: root.selectedRecent.length > 0 ? root.projectDisplayName(root.selectedRecent) : "No project selected"
                color: Theme.DarkTheme.text
                font.bold: true
                elide: Text.ElideRight
                Layout.maximumWidth: 240
            }
        }

        RowLayout {
            Layout.fillWidth: true
            MfButton { text: "Create Project"; accent: true; enabled: !root.loading; onClicked: root.createProject() }
            MfButton { text: "Refresh Recent"; enabled: !root.loading; onClicked: root.refreshLauncher() }
            Item { Layout.fillWidth: true }
            MfButton {
                text: "Open Selected"
                enabled: root.selectedRecent.length > 0
                onClicked: editorBridge.openProject(root.selectedRecent)
            }
            MfButton {
                text: "Repair Selected"
                enabled: root.selectedRecent.length > 0
                onClicked: root.repairProject(root.selectedRecent)
            }
        }

        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Horizontal

            Rectangle {
                SplitView.preferredWidth: 420
                SplitView.minimumWidth: 280
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft
                radius: Theme.DarkTheme.cardRadius

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 7
                    Label { text: "Recent projects"; color: Theme.DarkTheme.text; font.bold: true }
                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: recentModel
                        spacing: 4
                        delegate: ItemDelegate {
                            required property string path
                            width: ListView.view.width
                            implicitHeight: 48
                            highlighted: root.selectedRecent === path
                            onClicked: root.selectedRecent = path
                            onDoubleClicked: editorBridge.openProject(path)
                            contentItem: Column {
                                spacing: 2
                                Text {
                                    width: parent.width
                                    text: root.projectDisplayName(path)
                                    color: Theme.DarkTheme.text
                                    font.bold: true
                                    elide: Text.ElideRight
                                }
                                Text {
                                    width: parent.width
                                    text: path
                                    color: Theme.DarkTheme.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideMiddle
                                }
                            }
                        }
                    }
                }
            }

            TextArea {
                id: repairDetails
                SplitView.fillWidth: true
                SplitView.minimumWidth: 280
                readOnly: true
                text: "Repair and validation notes appear here."
                color: Theme.DarkTheme.text
                font.family: "Menlo"
                font.pixelSize: 10
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                background: Rectangle { color: Theme.DarkTheme.background }
            }
        }
    }
}
