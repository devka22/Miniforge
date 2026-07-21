import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel
    property string interpreter: "Python not inspected"
    property string statusText: editorBridge.projectOpen ? "Install or run trusted project tools" : "Open a project"

    function refreshTools() {
        if (!editorBridge.projectOpen) {
            toolModel.clear()
            return
        }
        var source = editorBridge.pythonToolsJson()
        try {
            var state = JSON.parse(source || "{}")
            interpreter = state.interpreter || "Python unavailable"
            toolModel.clear()
            var tools = state.tools || []
            for (var index = 0; index < tools.length; ++index) {
                toolModel.append({
                    "toolId": String(tools[index].id),
                    "label": String(tools[index].label),
                    "description": String(tools[index].description || ""),
                    "menuPath": String(tools[index].menu_path || "Tools/Python"),
                    "trusted": tools[index].trusted === true
                })
            }
            statusText = tools.length + " trusted automation manifests discovered"
        } catch (error) {
            statusText = editorBridge.lastError || ("Invalid Python tool state · " + error)
        }
    }

    function installTools() {
        if (editorBridge.installPythonTools())
            refreshTools()
        else
            statusText = editorBridge.lastError
    }

    function runTool(toolId, label) {
        var parameters
        try {
            parameters = JSON.parse(parametersEditor.text || "{}")
        } catch (error) {
            statusText = "Parameters must be valid JSON · " + error
            return
        }
        var source = editorBridge.runPythonTool(toolId, JSON.stringify(parameters))
        if (source.length === 0) {
            statusText = editorBridge.lastError
            return
        }
        try {
            var report = JSON.parse(source)
            var result = report.result || {}
            statusText = label + " · " + (result.message || (result.success ? "completed" : "failed"))
            resultEditor.text = JSON.stringify(report, null, 2)
        } catch (error) {
            statusText = "Invalid Python result · " + error
        }
    }

    ListModel { id: toolModel }
    Connections {
        target: editorBridge
        function onProjectChanged() { root.refreshTools() }
        function onPythonToolsChanged() { root.refreshTools() }
    }
    Component.onCompleted: refreshTools()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Python Automation"
            detail: root.statusText
            badge: toolModel.count + " tools"
            badgeColor: toolModel.count > 0 ? Theme.DarkTheme.accent : Theme.DarkTheme.warning
        }

        RowLayout {
            Layout.fillWidth: true
            Label { Layout.fillWidth: true; text: root.interpreter; color: Theme.DarkTheme.muted; elide: Text.ElideRight }
            MfButton { text: "Refresh"; onClicked: root.refreshTools() }
            MfButton { text: "Install Built-ins"; accent: true; onClicked: root.installTools() }
        }

        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Horizontal

            ListView {
                SplitView.preferredWidth: 340
                SplitView.minimumWidth: 240
                clip: true
                spacing: 4
                model: toolModel
                delegate: Rectangle {
                    id: toolRow
                    required property string toolId
                    required property string label
                    required property string description
                    required property string menuPath
                    required property bool trusted
                    width: ListView.view.width
                    height: 62
                    radius: Theme.DarkTheme.cardRadius
                    color: Theme.DarkTheme.surface
                    border.color: toolRow.trusted ? Theme.DarkTheme.border : Theme.DarkTheme.danger
                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 7
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Label { Layout.fillWidth: true; text: toolRow.label; color: Theme.DarkTheme.text; font.bold: true; elide: Text.ElideRight }
                            Label { Layout.fillWidth: true; text: toolRow.menuPath + " · " + toolRow.description; color: Theme.DarkTheme.muted; font.pixelSize: 9; elide: Text.ElideRight }
                        }
                        MfButton { text: "Run"; enabled: toolRow.trusted; onClicked: root.runTool(toolRow.toolId, toolRow.label) }
                    }
                }
            }

            ColumnLayout {
                SplitView.fillWidth: true
                SplitView.minimumWidth: 300
                Label { text: "Parameters JSON"; color: Theme.DarkTheme.text; font.bold: true }
                TextArea {
                    id: parametersEditor
                    Layout.fillWidth: true
                    implicitHeight: 90
                    text: "{}"
                    color: Theme.DarkTheme.text
                    font.family: "Menlo"
                    background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border }
                }
                Label { text: "Last protocol result"; color: Theme.DarkTheme.text; font.bold: true }
                TextArea {
                    id: resultEditor
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    readOnly: true
                    text: "Run a trusted .mftool.json manifest to inspect its result."
                    color: Theme.DarkTheme.text
                    font.family: "Menlo"
                    font.pixelSize: 10
                    wrapMode: TextEdit.Wrap
                    background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border }
                }
            }
        }
    }
}
