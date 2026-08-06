import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtCore
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.background

    property var catalog: ({})
    property var presets: []
    property var visiblePresets: []
    property var pendingPreset: ({})
    property var applicationPlan: ({})
    property bool applicationPlanError: false
    property string kindFilter: "all"
    property string statusText: ""
    property bool statusError: false

    Settings {
        id: preferences
        category: "MiniForgeAuthoringHub"
        property string favoritePresetIds: ""
    }

    function favoriteIds() {
        if (preferences.favoritePresetIds.trim().length === 0)
            return []
        return preferences.favoritePresetIds.split(",").filter(function(value) {
            return value.length > 0
        })
    }

    function isFavorite(presetId) {
        return favoriteIds().indexOf(String(presetId)) >= 0
    }

    function toggleFavorite(presetId) {
        var ids = favoriteIds()
        var id = String(presetId)
        var index = ids.indexOf(id)
        if (index >= 0)
            ids.splice(index, 1)
        else
            ids.push(id)
        ids.sort()
        preferences.favoritePresetIds = ids.join(",")
        refreshFilters()
    }

    function loadCatalog() {
        var source = editorBridge.authoringCatalogJson()
        if (source.length === 0) {
            statusText = editorBridge.lastError
            statusError = true
            return
        }
        try {
            catalog = JSON.parse(source)
            presets = catalog.presets || []
            statusText = presets.length + " production-ready authoring presets loaded"
            statusError = false
            refreshFilters()
        } catch (error) {
            statusText = "Invalid authoring catalog · " + error
            statusError = true
        }
    }

    function refreshFilters() {
        var query = presetSearch.text.trim().toLowerCase()
        var favorites = favoritesOnly.checked
        var next = []
        for (var index = 0; index < presets.length; ++index) {
            var preset = presets[index]
            var kind = String(preset.kind || "")
            if (kindFilter !== "all" && kind !== kindFilter)
                continue
            if (favorites && !isFavorite(preset.id))
                continue
            var haystack = [
                preset.id,
                preset.label,
                preset.summary,
                preset.category,
                (preset.tags || []).join(" "),
                (preset.genres || []).join(" "),
                (preset.components || []).join(" ")
            ].join(" ").toLowerCase()
            var terms = query.split(/\s+/).filter(function(value) { return value.length > 0 })
            var matches = true
            for (var term = 0; term < terms.length; ++term) {
                if (haystack.indexOf(terms[term]) < 0) {
                    matches = false
                    break
                }
            }
            if (matches)
                next.push(preset)
        }
        next.sort(function(left, right) {
            var leftFavorite = isFavorite(left.id) ? 1 : 0
            var rightFavorite = isFavorite(right.id) ? 1 : 0
            if (leftFavorite !== rightFavorite)
                return rightFavorite - leftFavorite
            return String(left.label).localeCompare(String(right.label))
        })
        visiblePresets = next
    }

    function applyPreset(preset, parameters) {
        if (editorBridge.selectedEntityCount < 1) {
            statusText = "Select one or more entities before applying a preset"
            statusError = true
            return
        }
        var payload = {"bundle": preset.id}
        if (parameters)
            payload.parameters = parameters
        var ok = editorBridge.performSelectedEntityAction(
            "add_component_bundle",
            JSON.stringify(payload)
        )
        if (ok) {
            statusText = preset.label + " applied to "
                + editorBridge.selectedEntityCount + " selected object(s)"
            statusError = false
        } else {
            statusText = editorBridge.lastError
            statusError = true
        }
    }

    function openConfiguration(preset) {
        pendingPreset = preset
        applicationPlan = ({})
        applicationPlanError = false
        parameterModel.clear()
        var parameters = preset.parameters || []
        for (var index = 0; index < parameters.length; ++index) {
            var parameter = parameters[index]
            parameterModel.append({
                "parameterId": String(parameter.id || ""),
                "label": String(parameter.label || parameter.id || "Parameter"),
                "description": String(parameter.description || ""),
                "valueType": String(parameter.value_type || "number"),
                "editorValue": String(parameter.default_value),
                "minimum": Number(parameter.minimum === null ? -1000000 : parameter.minimum),
                "maximum": Number(parameter.maximum === null ? 1000000 : parameter.maximum)
            })
        }
        refreshApplicationPlan()
        configurationPopup.open()
    }

    function configuredParameters() {
        var parameters = {}
        for (var index = 0; index < parameterModel.count; ++index) {
            var parameter = parameterModel.get(index)
            if (parameter.valueType === "bool") {
                parameters[parameter.parameterId] = String(parameter.editorValue) === "true"
                continue
            }
            var value = Number(parameter.editorValue)
            if (!isFinite(value))
                continue
            value = Math.max(parameter.minimum, Math.min(parameter.maximum, value))
            if (parameter.valueType === "integer")
                value = Math.round(value)
            parameters[parameter.parameterId] = value
        }
        return parameters
    }

    function refreshApplicationPlan() {
        if (!pendingPreset.id || editorBridge.selectedEntityCount < 1)
            return
        var source = editorBridge.authoringPlanJson(
            String(pendingPreset.id),
            JSON.stringify(configuredParameters())
        )
        if (source.length === 0) {
            applicationPlan = ({})
            applicationPlanError = true
            return
        }
        try {
            applicationPlan = JSON.parse(source)
            applicationPlanError = false
        } catch (error) {
            applicationPlan = ({})
            applicationPlanError = true
        }
    }

    function applyConfiguredPreset() {
        var parameters = configuredParameters()
        configurationPopup.close()
        applyPreset(pendingPreset, parameters)
    }

    function selectKind(kind) {
        var requested = String(kind || "all")
        var index = kindPicker.find(requested)
        kindPicker.currentIndex = index >= 0 ? index : 0
        kindFilter = kindPicker.currentText
        refreshFilters()
    }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.loadCatalog() }
    }

    Component.onCompleted: loadCatalog()

    ListModel {
        id: parameterModel
    }

    Popup {
        id: configurationPopup
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(590, parent.width - 40)
        height: Math.min(620, parent.height - 40)
        padding: 1
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.panel
            border.color: Theme.DarkTheme.focus
            border.width: 1
        }

        contentItem: ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 9

            MfPanelHeader {
                Layout.fillWidth: true
                title: String(root.pendingPreset.label || "Configure preset")
                detail: "Review exact component changes and tune values. No scripting required."
                badge: String(root.applicationPlan.total_components_to_add || 0) + " changes"
                badgeColor: Theme.DarkTheme.info
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 92
                radius: Theme.DarkTheme.radius
                color: Theme.DarkTheme.surface
                border.color: root.applicationPlanError
                    ? Theme.DarkTheme.danger : Theme.DarkTheme.borderSoft
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 3

                    Text {
                        Layout.fillWidth: true
                        text: root.applicationPlanError
                            ? editorBridge.lastError
                            : String(root.applicationPlan.target_count || 0) + " object(s) · "
                                + String(root.applicationPlan.total_components_to_add || 0) + " components to add · "
                                + String(root.applicationPlan.total_components_existing || 0) + " already present"
                        color: root.applicationPlanError
                            ? Theme.DarkTheme.danger : Theme.DarkTheme.text
                        font.pixelSize: 11
                        font.bold: true
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "Requirements · "
                            + ((root.applicationPlan.requirements || []).join(" · ") || "none")
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "First steps · "
                            + ((root.applicationPlan.workflow_steps || []).slice(0, 2).join(" → ")
                                || "ready to use")
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.applicationPlan.world_profile_will_change
                            ? "Physics world profile will also be updated"
                            : "No global physics settings will change"
                        color: root.applicationPlan.world_profile_will_change
                            ? Theme.DarkTheme.info : Theme.DarkTheme.muted
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                }
            }

            ListView {
                id: parameterList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                spacing: 7
                model: parameterModel
                ScrollBar.vertical: ScrollBar {}

                delegate: Rectangle {
                    id: parameterRow
                    required property int index
                    required property string parameterId
                    required property string label
                    required property string description
                    required property string valueType
                    required property string editorValue
                    required property real minimum
                    required property real maximum
                    width: ListView.view.width
                    height: 74
                    radius: Theme.DarkTheme.radius
                    color: Theme.DarkTheme.surface
                    border.color: valueInput.activeFocus || boolInput.activeFocus
                        ? Theme.DarkTheme.focus : Theme.DarkTheme.borderSoft
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 9
                        spacing: 10

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text {
                                Layout.fillWidth: true
                                text: parameterRow.label
                                color: Theme.DarkTheme.text
                                font.pixelSize: 11
                                font.bold: true
                                elide: Text.ElideRight
                            }
                            Text {
                                Layout.fillWidth: true
                                text: parameterRow.description
                                color: Theme.DarkTheme.muted
                                font.pixelSize: 9
                                wrapMode: Text.WordWrap
                                maximumLineCount: 2
                                elide: Text.ElideRight
                            }
                        }

                        TextField {
                            id: valueInput
                            visible: parameterRow.valueType !== "bool"
                            Layout.preferredWidth: 108
                            text: parameterRow.editorValue
                            horizontalAlignment: TextInput.AlignRight
                            selectByMouse: true
                            validator: DoubleValidator {
                                bottom: parameterRow.minimum
                                top: parameterRow.maximum
                                decimals: parameterRow.valueType === "integer" ? 0 : 4
                            }
                            onEditingFinished: {
                                parameterModel.setProperty(parameterRow.index, "editorValue", text)
                                root.refreshApplicationPlan()
                            }
                            ToolTip.visible: hovered
                            ToolTip.text: "Range " + parameterRow.minimum + " to " + parameterRow.maximum
                        }

                        Switch {
                            id: boolInput
                            visible: parameterRow.valueType === "bool"
                            Layout.preferredWidth: 108
                            checked: parameterRow.editorValue === "true"
                            text: checked ? "On" : "Off"
                            onToggled: {
                                parameterModel.setProperty(
                                    parameterRow.index,
                                    "editorValue",
                                    checked ? "true" : "false"
                                )
                                root.refreshApplicationPlan()
                            }
                            ToolTip.visible: hovered
                            ToolTip.text: parameterRow.description
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Item { Layout.fillWidth: true }
                MfButton {
                    text: "Cancel"
                    onClicked: configurationPopup.close()
                }
                MfButton {
                    text: parameterModel.count > 0 ? "Apply configured" : "Apply preset"
                    accent: true
                    enabled: editorBridge.selectedEntityCount > 0
                        && !root.applicationPlanError
                    onClicked: root.applyConfiguredPreset()
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 9

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Mega Authoring Hub"
            detail: "Players, AI, gameplay, physics, worlds, effects, UI and strategy from one engine catalog"
            badge: root.visiblePresets.length + " / " + root.presets.length
            badgeColor: Theme.DarkTheme.accent
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            MfSearchBar {
                id: presetSearch
                Layout.fillWidth: true
                placeholderText: "Search genre, system, component or workflow"
                onTextChanged: root.refreshFilters()
                Keys.onEscapePressed: text = ""
            }

            ComboBox {
                id: kindPicker
                Layout.preferredWidth: 150
                model: ["all", "actor", "gameplay", "physics", "world", "effects", "user_interface", "strategy"]
                onCurrentTextChanged: {
                    root.kindFilter = currentText
                    root.refreshFilters()
                }
            }

            CheckBox {
                id: favoritesOnly
                text: "Favorites"
                onToggled: root.refreshFilters()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 50
            radius: Theme.DarkTheme.radius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 18

                Repeater {
                    model: [
                        {"label":"Presets", "value":root.presets.length},
                        {"label":"Components", "value":root.catalog.total_components_referenced || 0},
                        {"label":"Physics", "value":(root.catalog.kinds || {}).physics || 0},
                        {"label":"Favorites", "value":root.favoriteIds().length}
                    ]
                    delegate: Column {
                        required property var modelData
                        Layout.preferredWidth: 110
                        spacing: 1
                        Text {
                            text: modelData.value
                            color: Theme.DarkTheme.text
                            font.pixelSize: 16
                            font.bold: true
                        }
                        Text {
                            text: modelData.label
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 9
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    Layout.maximumWidth: 380
                    text: root.statusText
                    color: root.statusError ? Theme.DarkTheme.danger : Theme.DarkTheme.accent
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }
        }

        ListView {
            id: presetList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 7
            model: root.visiblePresets
            ScrollBar.vertical: ScrollBar {}

            delegate: Rectangle {
                id: presetCard
                required property var modelData
                width: ListView.view.width
                height: 166
                radius: Theme.DarkTheme.cardRadius
                color: cardMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.surface
                border.color: root.isFavorite(modelData.id)
                    ? Theme.DarkTheme.accent
                    : (cardMouse.containsMouse ? Theme.DarkTheme.focus : Theme.DarkTheme.borderSoft)
                border.width: 1

                MouseArea {
                    id: cardMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 10

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 4

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                Layout.fillWidth: true
                                text: presetCard.modelData.label
                                color: Theme.DarkTheme.text
                                font.pixelSize: 14
                                font.bold: true
                                elide: Text.ElideRight
                            }
                            Text {
                                text: String(presetCard.modelData.category || "Systems")
                                    + " · " + String(presetCard.modelData.maturity || "production")
                                color: Theme.DarkTheme.accent
                                font.pixelSize: 9
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: presetCard.modelData.summary
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 10
                            wrapMode: Text.WordWrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: (presetCard.modelData.components || []).join("  ·  ")
                            color: Theme.DarkTheme.text
                            opacity: 0.78
                            font.pixelSize: 9
                            maximumLineCount: 2
                            wrapMode: Text.Wrap
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: (presetCard.modelData.parameters || []).length + " parameters"
                                + "  ·  " + (presetCard.modelData.workflow_steps || []).length + " guided steps"
                                + "  ·  " + (presetCard.modelData.requirements || []).length + " requirements"
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                    }

                    ColumnLayout {
                        Layout.preferredWidth: 148
                        Layout.fillHeight: true
                        spacing: 7

                        MfButton {
                            Layout.fillWidth: true
                            text: root.isFavorite(presetCard.modelData.id) ? "★ Favorite" : "☆ Favorite"
                            onClicked: root.toggleFavorite(presetCard.modelData.id)
                        }

                        MfButton {
                            Layout.fillWidth: true
                            text: (presetCard.modelData.parameters || []).length > 0
                                ? "Configure" : "Review"
                            accent: true
                            enabled: editorBridge.selectedEntityCount > 0
                            onClicked: root.openConfiguration(presetCard.modelData)
                        }

                        Text {
                            Layout.fillWidth: true
                            text: (presetCard.modelData.components || []).length + " components"
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 9
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }
            }

            Text {
                visible: presetList.count === 0
                anchors.centerIn: parent
                width: Math.min(420, parent.width - 40)
                text: "No presets match this search. Clear filters or switch system type."
                color: Theme.DarkTheme.muted
                font.pixelSize: 12
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
            }
        }
    }
}
