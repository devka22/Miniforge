import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property bool loading: false
    property var opsState: ({"autosave":{}, "session":{}})
    property string statusText: editorBridge.projectOpen
        ? "Project operations ready"
        : "Open a project to manage packages and recovery"

    function reload() {
        if (loading || !editorBridge.projectOpen)
            return
        loading = true
        var source = editorBridge.projectOperationsJson()
        if (source.length === 0) {
            statusText = editorBridge.lastError || "Project operations unavailable"
            loading = false
            return
        }
        try {
            opsState = JSON.parse(source)
            var autosave = opsState.autosave || {}
            autosaveEnabled.checked = autosave.enabled !== false
            autosaveInterval.value = Number(autosave.interval_seconds || 60)
            var last = opsState.last_operation || null
            statusText = last ? String(last.message) : "Project operations ready"
        } catch (error) {
            statusText = "Invalid project operations snapshot · " + error
        }
        loading = false
    }

    function run(action, payload) {
        if (!editorBridge.runProjectOperation(action, JSON.stringify(payload || {}))) {
            statusText = editorBridge.lastError
            return false
        }
        reload()
        return true
    }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.reload() }
    }

    Component.onCompleted: reload()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Project Operations"
            detail: root.statusText
            badge: root.loading ? "Working" : "Recovery + Build"
            badgeColor: root.loading ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        TabBar {
            id: tabs
            Layout.fillWidth: true
            TabButton { text: "Packages" }
            TabButton { text: "Autosave & Session" }
            TabButton { text: "External Play" }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabs.currentIndex

            ScrollView {
                clip: true
                ColumnLayout {
                    width: Math.max(420, parent.width)
                    spacing: 8

                    Label { text: "Project package (.mfpkg.zip)"; color: Theme.DarkTheme.text; font.bold: true }
                    Label {
                        Layout.fillWidth: true
                        text: "Exports source assets/settings without builds, logs, recovery state or generated packages."
                        color: Theme.DarkTheme.muted
                        wrapMode: Text.WordWrap
                    }
                    MfButton {
                        text: "Export Project Package"
                        accent: true
                        onClicked: root.run("package_export", {})
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.DarkTheme.border }
                    Label { text: "Import package"; color: Theme.DarkTheme.text; font.bold: true }
                    TextField {
                        id: archivePath
                        Layout.fillWidth: true
                        placeholderText: "/absolute/path/project.mfpkg.zip"
                        selectByMouse: true
                    }
                    TextField {
                        id: importRoot
                        Layout.fillWidth: true
                        placeholderText: "/absolute/path/projects"
                        selectByMouse: true
                    }
                    MfButton {
                        text: "Import as New Project"
                        enabled: archivePath.text.length > 0 && importRoot.text.length > 0
                        onClicked: root.run("package_import", {
                            "archive_path": archivePath.text.trim(),
                            "destination_root": importRoot.text.trim()
                        })
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.DarkTheme.border }
                    Label { text: "Distributable folder"; color: Theme.DarkTheme.text; font.bold: true }
                    RowLayout {
                        Layout.fillWidth: true
                        ComboBox { id: packageProfile; model: ["debug", "release", "shipping"] }
                        TextField { id: packageLabel; Layout.fillWidth: true; text: "game"; placeholderText: "Package label" }
                        MfButton {
                            text: "Package"
                            onClicked: root.run("package_distributable", {
                                "profile": packageProfile.currentText,
                                "label": packageLabel.text.trim()
                            })
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            ScrollView {
                clip: true
                ColumnLayout {
                    width: Math.max(420, parent.width)
                    spacing: 8

                    Label { text: "Scene autosave"; color: Theme.DarkTheme.text; font.bold: true }
                    RowLayout {
                        Layout.fillWidth: true
                        Switch { id: autosaveEnabled; text: "Enabled" }
                        Label { text: "Interval"; color: Theme.DarkTheme.muted }
                        SpinBox { id: autosaveInterval; from: 5; to: 3600; value: 60; editable: true }
                        Label { text: "seconds"; color: Theme.DarkTheme.muted }
                        Item { Layout.fillWidth: true }
                        MfButton {
                            text: "Apply"
                            onClicked: root.run("autosave_configure", {
                                "enabled": autosaveEnabled.checked,
                                "interval_seconds": autosaveInterval.value
                            })
                        }
                    }
                    Label {
                        text: "State: " + String((root.opsState.autosave || {}).health || "empty")
                            + " · " + ((root.opsState.autosave || {}).exists ? "checkpoint available" : "no checkpoint")
                        color: (root.opsState.autosave || {}).exists ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
                    }
                    RowLayout {
                        MfButton { text: "Autosave Now"; accent: true; onClicked: root.run("autosave_now", {}) }
                        MfButton {
                            text: "Recover Scene"
                            enabled: (root.opsState.autosave || {}).exists === true
                            onClicked: root.run("autosave_recover", {})
                        }
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.DarkTheme.border }
                    Label { text: "Editor session recovery"; color: Theme.DarkTheme.text; font.bold: true }
                    Label {
                        Layout.fillWidth: true
                        text: (root.opsState.session || {}).pending
                            ? ((root.opsState.session || {}).documents + " documents · "
                                + (root.opsState.session || {}).dirty_buffers + " dirty buffers")
                            : "No pending backend session checkpoint"
                        color: (root.opsState.session || {}).pending ? Theme.DarkTheme.warning : Theme.DarkTheme.muted
                    }
                    RowLayout {
                        MfButton { text: "Checkpoint"; accent: true; onClicked: root.run("session_checkpoint", {}) }
                        MfButton {
                            text: "Restore"
                            enabled: (root.opsState.session || {}).pending === true
                            onClicked: root.run("session_restore", {})
                        }
                        MfButton {
                            text: "Clear"
                            enabled: (root.opsState.session || {}).pending === true
                            onClicked: root.run("session_clear", {})
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        text: "Scene/document recovery checkpoints every 10 seconds while the workbench refresh pulse is active. Luau tabs also persist in .miniforge/qt_workspace.json."
                        color: Theme.DarkTheme.muted
                        wrapMode: Text.WordWrap
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            ScrollView {
                clip: true
                ColumnLayout {
                    width: Math.max(420, parent.width)
                    spacing: 8

                    Label { text: "Run exported game in a separate process"; color: Theme.DarkTheme.text; font.bold: true }
                    RowLayout {
                        Layout.fillWidth: true
                        ComboBox { id: launchProfile; model: ["debug", "release", "shipping"] }
                        TextField { id: launchLabel; Layout.fillWidth: true; text: "game"; placeholderText: "Build label" }
                    }
                    RowLayout {
                        MfButton {
                            text: "Prepare & Play"
                            accent: true
                            onClicked: {
                                if (root.run("prepare_external_play", {"profile": launchProfile.currentText}))
                                    editorBridge.launchPreparedExternal()
                            }
                        }
                        MfButton {
                            text: "Package & Run"
                            onClicked: {
                                if (root.run("prepare_external_build", {
                                    "profile": launchProfile.currentText,
                                    "label": launchLabel.text.trim()
                                }))
                                    editorBridge.launchPreparedExternal()
                            }
                        }
                        MfButton {
                            text: "Stop External"
                            enabled: editorBridge.externalLaunchRunning
                            onClicked: editorBridge.stopExternalLaunch()
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        text: {
                            var plan = root.opsState.external_launch || null
                            if (!plan)
                                return "No external launch prepared"
                            return (editorBridge.externalLaunchRunning ? "Running" : (plan.ready ? "Ready" : "Runtime missing"))
                                + " · " + plan.artifact_path
                        }
                        color: editorBridge.externalLaunchRunning || (root.opsState.external_launch || {}).ready
                            ? Theme.DarkTheme.accent : Theme.DarkTheme.warning
                        wrapMode: Text.WrapAnywhere
                    }
                    Label {
                        Layout.fillWidth: true
                        text: "External runtime is isolated from the editor process and receives --build <artifact>."
                        color: Theme.DarkTheme.muted
                        wrapMode: Text.WordWrap
                    }
                    Item { Layout.fillHeight: true }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                text: String(((root.opsState.last_operation || {}).artifact_path) || "")
                color: Theme.DarkTheme.muted
                elide: Text.ElideMiddle
            }
            MfButton { text: "Refresh"; onClicked: root.reload() }
        }
    }
}
