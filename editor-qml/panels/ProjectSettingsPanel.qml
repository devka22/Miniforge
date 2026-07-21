import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property var engineSettings: ({})
    property var inputSettings: ({"bindings": {}, "actions": {}})
    property bool loading: false
    property bool dirty: false
    property string statusText: "Open a project to edit settings"

    function loadSettings() {
        if (!editorBridge.projectOpen) {
            actionModel.clear()
            tagModel.clear()
            layerModel.clear()
            statusText = "Open a project to edit settings"
            dirty = false
            return
        }
        loading = true
        var source = editorBridge.projectSettingsJson()
        if (source.length === 0) {
            statusText = editorBridge.lastError || "Project settings are unavailable"
            loading = false
            return
        }
        try {
            var snapshot = JSON.parse(source)
            engineSettings = snapshot.engine || {}
            inputSettings = snapshot.input || {"bindings": {}, "actions": {}}
            projectName.text = engineSettings.project_name || ""
            startScene.text = engineSettings.start_scene || "main.scene"
            autosave.checked = engineSettings.autosave !== false
            safeMode.checked = engineSettings.safe_mode !== false
            vsync.checked = !engineSettings.rendering || engineSettings.rendering.vsync !== false
            pixelPerfect.checked = !engineSettings.rendering || engineSettings.rendering.pixel_perfect !== false

            actionModel.clear()
            var bindings = inputSettings.bindings || {}
            var names = Object.keys(bindings).sort()
            for (var index = 0; index < names.length; ++index) {
                var name = names[index]
                actionModel.append({"name": name, "bindings": (bindings[name] || []).join(", ")})
            }
            tagModel.clear()
            var tags = snapshot.tags || []
            for (var tag = 0; tag < tags.length; ++tag)
                tagModel.append({"name": String(tags[tag])})
            layerModel.clear()
            var layers = snapshot.layers || []
            for (var layer = 0; layer < layers.length; ++layer)
                layerModel.append({"name": String(layers[layer])})
            dirty = false
            statusText = "Settings loaded · edits are persisted atomically"
        } catch (error) {
            statusText = "Invalid settings snapshot · " + error
        }
        loading = false
    }

    function saveGeneral() {
        var value = engineSettings || {}
        value.project_name = projectName.text.trim()
        value.start_scene = startScene.text.trim()
        value.autosave = autosave.checked
        value.safe_mode = safeMode.checked
        if (!value.rendering)
            value.rendering = {}
        value.rendering.vsync = vsync.checked
        value.rendering.pixel_perfect = pixelPerfect.checked
        if (!editorBridge.saveEngineSettingsJson(JSON.stringify(value))) {
            statusText = editorBridge.lastError
            return false
        }
        engineSettings = value
        return true
    }

    function saveInput() {
        var bindings = {}
        var previousActions = inputSettings.actions || {}
        var actions = {}
        for (var index = 0; index < actionModel.count; ++index) {
            var row = actionModel.get(index)
            var raw = row.bindings.split(",")
            var normalized = []
            for (var binding = 0; binding < raw.length; ++binding) {
                var text = raw[binding].trim()
                if (text.length > 0 && normalized.indexOf(text) < 0)
                    normalized.push(text)
            }
            bindings[row.name] = normalized
            if (previousActions[row.name]) {
                actions[row.name] = previousActions[row.name]
            } else {
                actions[row.name] = {
                    "display_name": row.name,
                    "category": "Gameplay",
                    "devices": ["keyboard"],
                    "description": ""
                }
            }
        }
        var value = {"bindings": bindings, "actions": actions}
        if (!editorBridge.saveInputMapJson(JSON.stringify(value))) {
            statusText = editorBridge.lastError
            return false
        }
        inputSettings = value
        return true
    }

    function modelNames(model) {
        var result = []
        for (var index = 0; index < model.count; ++index)
            result.push(model.get(index).name)
        return result
    }

    function saveTagsLayers() {
        var value = {"tags": modelNames(tagModel), "layers": modelNames(layerModel)}
        if (!editorBridge.saveTagsLayersJson(JSON.stringify(value))) {
            statusText = editorBridge.lastError
            return false
        }
        return true
    }

    function saveAll() {
        if (!saveGeneral() || !saveInput() || !saveTagsLayers())
            return
        dirty = false
        statusText = "Project Settings, Input Map and Tags/Layers saved"
    }

    function appendUnique(model, field) {
        var name = field.text.trim()
        if (name.length === 0)
            return
        for (var index = 0; index < model.count; ++index) {
            if (model.get(index).name === name)
                return
        }
        model.append({"name": name})
        field.clear()
        dirty = true
    }

    ListModel { id: actionModel }
    ListModel { id: tagModel }
    ListModel { id: layerModel }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.loadSettings() }
    }

    Component.onCompleted: loadSettings()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Project Settings"
            detail: root.statusText
            badge: root.dirty ? "Unsaved" : "Saved"
            badgeColor: root.dirty ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 32
            spacing: 8

            TabBar {
                id: tabs
                Layout.fillWidth: true
                currentIndex: 0
                TabButton { text: "General" }
                TabButton { text: "Input Map" }
                TabButton { text: "Tags & Layers" }
            }

            MfButton {
                Layout.preferredWidth: 72
                text: "Reload"
                enabled: editorBridge.projectOpen && !root.loading
                onClicked: root.loadSettings()
            }
            MfButton {
                Layout.preferredWidth: 82
                text: "Save All"
                accent: true
                enabled: editorBridge.projectOpen && !root.loading
                onClicked: root.saveAll()
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabs.currentIndex

            ScrollView {
                clip: true
                ColumnLayout {
                    width: Math.max(420, parent.width)
                    spacing: 9

                    Label { text: "Project name"; color: Theme.DarkTheme.muted }
                    TextField { id: projectName; Layout.fillWidth: true; onTextEdited: root.dirty = true }
                    Label { text: "Start scene"; color: Theme.DarkTheme.muted }
                    TextField { id: startScene; Layout.fillWidth: true; placeholderText: "main.scene"; onTextEdited: root.dirty = true }
                    Switch { id: autosave; text: "Autosave project"; onToggled: if (!root.loading) root.dirty = true }
                    Switch { id: safeMode; text: "Safe mode by default"; onToggled: if (!root.loading) root.dirty = true }
                    Switch { id: vsync; text: "Vertical sync"; onToggled: if (!root.loading) root.dirty = true }
                    Switch { id: pixelPerfect; text: "Pixel-perfect rendering"; onToggled: if (!root.loading) root.dirty = true }
                }
            }

            ColumnLayout {
                spacing: 7
                RowLayout {
                    Layout.fillWidth: true
                    TextField { id: newAction; Layout.fillWidth: true; placeholderText: "New action" }
                    TextField { id: newBindings; Layout.preferredWidth: 260; placeholderText: "space, gamepad:south" }
                    MfButton {
                        text: "Add"
                        onClicked: {
                            var name = newAction.text.trim()
                            if (name.length === 0) return
                            actionModel.append({"name": name, "bindings": newBindings.text.trim()})
                            newAction.clear(); newBindings.clear(); root.dirty = true
                        }
                    }
                }
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: actionModel
                    spacing: 5
                    delegate: Rectangle {
                        required property int index
                        required property string name
                        required property string bindings
                        width: ListView.view.width
                        height: 38
                        radius: Theme.DarkTheme.radius
                        color: Theme.DarkTheme.surface
                        border.color: Theme.DarkTheme.borderSoft
                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 5
                            TextField {
                                Layout.preferredWidth: 180
                                text: name
                                onEditingFinished: { actionModel.setProperty(index, "name", text.trim()); root.dirty = true }
                            }
                            TextField {
                                Layout.fillWidth: true
                                text: bindings
                                onEditingFinished: { actionModel.setProperty(index, "bindings", text); root.dirty = true }
                            }
                            MfButton { text: "Remove"; onClicked: { actionModel.remove(index); root.dirty = true } }
                        }
                    }
                }
            }

            RowLayout {
                spacing: 10
                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    radius: Theme.DarkTheme.cardRadius
                    color: Theme.DarkTheme.surface
                    border.color: Theme.DarkTheme.borderSoft
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 8
                        Label { text: "Tags"; color: Theme.DarkTheme.text; font.bold: true }
                        ListView {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: tagModel
                            delegate: ItemDelegate {
                                required property int index
                                required property string name
                                width: ListView.view.width
                                text: name
                                onClicked: if (index > 0) { tagModel.remove(index); root.dirty = true }
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            TextField { id: tagField; Layout.fillWidth: true; placeholderText: "Add tag" }
                            MfButton { text: "Add"; onClicked: root.appendUnique(tagModel, tagField) }
                        }
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    radius: Theme.DarkTheme.cardRadius
                    color: Theme.DarkTheme.surface
                    border.color: Theme.DarkTheme.borderSoft
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 8
                        Label { text: "Layers"; color: Theme.DarkTheme.text; font.bold: true }
                        ListView {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: layerModel
                            delegate: ItemDelegate {
                                required property int index
                                required property string name
                                width: ListView.view.width
                                text: name
                                onClicked: if (index > 0) { layerModel.remove(index); root.dirty = true }
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            TextField { id: layerField; Layout.fillWidth: true; placeholderText: "Add layer" }
                            MfButton { text: "Add"; onClicked: root.appendUnique(layerModel, layerField) }
                        }
                    }
                }
            }
        }
    }
}
