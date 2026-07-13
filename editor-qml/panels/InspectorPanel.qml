import QtQuick
import QtQuick.Controls
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel
    property string pendingQuickActionId: ""
    property string pendingQuickActionLabel: ""
    property string quickStatus: ""

    function runAction(action, payload) {
        if (inspectorModel.entityId === 0)
            return false
        if (editorBridge.selectedEntityCount > 1
                && (action === "add_component"
                    || action === "remove_component"
                    || action === "duplicate"
                    || action === "delete"))
            return editorBridge.performSelectedEntityAction(action, JSON.stringify(payload || {}))
        return editorBridge.performEntityAction(
            inspectorModel.entityId,
            action,
            JSON.stringify(payload || {})
        )
    }

    function loadQuickActions() {
        quickActionModel.clear()
        if (inspectorModel.entityId === 0)
            return
        var json = editorBridge.inspectorQuickActionsJson(inspectorModel.entityId)
        if (json.length === 0) {
            quickStatus = editorBridge.lastError
            return
        }
        try {
            var actions = JSON.parse(json)
            for (var index = 0; index < actions.length; ++index) {
                var action = actions[index]
                quickActionModel.append({
                    "actionId": String(action.id || ""),
                    "label": String(action.label || action.id || "Action"),
                    "enabled": action.enabled === true,
                    "disabledReason": String(action.disabled_reason || ""),
                    "requiresAsset": action.requires_asset === true,
                    "attachedAssetPath": String(action.attached_asset_path || ""),
                    "assetsJson": JSON.stringify(action.assets || [])
                })
            }
            quickStatus = ""
        } catch (error) {
            quickStatus = "Invalid quick action catalog · " + error
        }
    }

    function beginQuickAction(actionId, label, requiresAsset, attachedAssetPath, assetsJson) {
        if (!requiresAsset) {
            if (!editorBridge.performInspectorQuickAction(
                        inspectorModel.entityId, actionId, attachedAssetPath))
                quickStatus = editorBridge.lastError
            else {
                quickStatus = label + " complete"
                loadQuickActions()
            }
            return
        }
        quickAssetModel.clear()
        try {
            var assets = JSON.parse(assetsJson)
            for (var index = 0; index < assets.length; ++index) {
                quickAssetModel.append({
                    "label": String(assets[index].name || assets[index].relative_path),
                    "relativePath": String(assets[index].relative_path || ""),
                    "assetType": String(assets[index].asset_type || "")
                })
            }
        } catch (error) {
            quickStatus = "Invalid compatible asset list · " + error
            return
        }
        pendingQuickActionId = actionId
        pendingQuickActionLabel = label
        quickAssetPicker.currentIndex = quickAssetModel.count > 0 ? 0 : -1
        quickAssetPopup.open()
    }

    function loadComponentCatalog() {
        componentModel.clear()
        var json = editorBridge.componentCatalogJson()
        if (json.length === 0)
            return
        try {
            var categories = JSON.parse(json)
            for (var group = 0; group < categories.length; ++group) {
                var category = categories[group]
                var types = category.component_types || []
                for (var index = 0; index < types.length; ++index) {
                    componentModel.append({
                        "category": category.category || "Components",
                        "componentType": String(types[index])
                    })
                }
            }
        } catch (error) {
            componentCatalogError.text = "Invalid component catalog · " + error
        }
    }

    function removableSection(section) {
        return section !== "Transform"
            && section !== "Identity"
            && section !== "ScriptVariables"
            && section !== "Selectable"
            && section !== "SpriteRenderer"
            && section !== "Collider2D"
    }

    ListModel {
        id: componentModel
    }
    ListModel { id: quickActionModel }
    ListModel { id: quickAssetModel }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.loadComponentCatalog(); root.loadQuickActions() }
        function onSelectionChanged() { root.loadQuickActions() }
        function onEntitiesChanged() { root.loadQuickActions() }
        function onAssetsChanged() { root.loadQuickActions() }
    }

    Component.onCompleted: { loadComponentCatalog(); loadQuickActions() }

    Menu {
        id: inspectorMenu
        MenuItem {
            text: "Reset Transform"
            onTriggered: root.runAction("reset_transform", {})
        }
        MenuItem {
            text: "Duplicate Entity"
            onTriggered: root.runAction("duplicate", {})
        }
        MenuItem {
            text: "Move to Scene Root"
            onTriggered: root.runAction("unparent", {})
        }
        MenuSeparator {}
        MenuItem {
            text: "Delete Entity"
            onTriggered: root.runAction("delete", {})
        }
    }

    Popup {
        id: componentPopup
        width: Math.min(440, Math.max(280, root.width - 20))
        height: Math.min(520, Math.max(300, root.height - 24))
        x: Math.max(0, (root.width - width) / 2)
        y: Math.max(0, (root.height - height) / 2)
        padding: 1
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.panel
            border.color: Theme.DarkTheme.focus
            border.width: 1
        }

        contentItem: Column {
            anchors.fill: parent
            anchors.margins: 9
            spacing: 7

            MfPanelHeader {
                width: parent.width
                title: "Add Component"
                detail: componentModel.count + " registered component types"
                badge: "Registry"
                badgeColor: Theme.DarkTheme.info
            }

            MfSearchBar {
                id: componentSearch
                width: parent.width
                placeholderText: "Filter components"
                Keys.onEscapePressed: {
                    if (text.length > 0)
                        text = ""
                    else
                        componentPopup.close()
                }
            }

            Text {
                id: componentCatalogError
                width: parent.width
                visible: text.length > 0
                height: visible ? 18 : 0
                color: Theme.DarkTheme.danger
                font.pixelSize: 9
                elide: Text.ElideRight
            }

            ListView {
                id: componentList
                width: parent.width
                height: parent.height - y
                clip: true
                spacing: 3
                model: componentModel

                delegate: Rectangle {
                    id: componentRow
                    required property string category
                    required property string componentType
                    readonly property bool matches: componentSearch.text.trim().length === 0
                        || componentType.toLowerCase().indexOf(componentSearch.text.trim().toLowerCase()) >= 0
                        || category.toLowerCase().indexOf(componentSearch.text.trim().toLowerCase()) >= 0

                    width: ListView.view.width
                    height: matches ? 38 : 0
                    visible: matches
                    radius: Theme.DarkTheme.radius
                    color: componentMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.surface
                    border.color: componentMouse.containsMouse ? Theme.DarkTheme.focus : Theme.DarkTheme.borderSoft
                    border.width: 1

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 8

                        Text {
                            width: Math.max(90, parent.width - categoryLabel.width - 8)
                            height: parent.height
                            text: componentRow.componentType
                            color: Theme.DarkTheme.text
                            font.pixelSize: 11
                            font.bold: true
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }

                        Text {
                            id: categoryLabel
                            width: 82
                            height: parent.height
                            text: componentRow.category
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 9
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                    }

                    MouseArea {
                        id: componentMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            if (root.runAction(
                                    "add_component",
                                    {"component_type": componentRow.componentType})) {
                                componentPopup.close()
                                componentSearch.text = ""
                            }
                        }
                    }
                }
            }
        }

        onOpened: {
            componentCatalogError.text = ""
            root.loadComponentCatalog()
            componentSearch.forceActiveFocus()
        }
    }

    Popup {
        id: quickAssetPopup
        width: Math.min(460, Math.max(300, root.width - 20))
        height: 178
        x: Math.max(0, (root.width - width) / 2)
        y: Math.max(0, (root.height - height) / 2)
        padding: 10
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.panel
            border.color: Theme.DarkTheme.focus
            border.width: 1
        }

        contentItem: Column {
            spacing: 10
            Text {
                width: parent.width
                text: root.pendingQuickActionLabel
                color: Theme.DarkTheme.text
                font.pixelSize: 14
                font.bold: true
            }
            ComboBox {
                id: quickAssetPicker
                width: parent.width
                model: quickAssetModel
                textRole: "label"
            }
            Text {
                width: parent.width
                text: quickAssetPicker.currentIndex >= 0
                    ? quickAssetModel.get(quickAssetPicker.currentIndex).relativePath
                    : "No compatible indexed assets"
                color: Theme.DarkTheme.muted
                font.pixelSize: 10
                elide: Text.ElideMiddle
            }
            Row {
                spacing: 8
                MfButton { text: "Cancel"; onClicked: quickAssetPopup.close() }
                MfButton {
                    text: "Apply"
                    accent: true
                    enabled: quickAssetPicker.currentIndex >= 0
                    onClicked: {
                        var assetPath = quickAssetModel.get(quickAssetPicker.currentIndex).relativePath
                        if (editorBridge.performInspectorQuickAction(
                                    inspectorModel.entityId,
                                    root.pendingQuickActionId,
                                    assetPath)) {
                            root.quickStatus = root.pendingQuickActionLabel + " complete"
                            quickAssetPopup.close()
                            root.loadQuickActions()
                        } else {
                            root.quickStatus = editorBridge.lastError
                        }
                    }
                }
            }
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Inspector"
            detail: inspectorModel.entityId > 0
                ? (editorBridge.selectedEntityCount > 1
                    ? editorBridge.selectedEntityCount + " selected · editing #" + inspectorModel.entityId
                    : "Entity #" + inspectorModel.entityId)
                : "No selection"
            badge: inspectorList.count + " fields"
            badgeColor: inspectorModel.entityId > 0 ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
        }

        Row {
            width: parent.width
            height: 32
            spacing: 6

            MfSearchBar {
                width: Math.max(80, parent.width - addComponentButton.width - actionsButton.width - 12)
                placeholderText: "Filter properties"
                enabled: inspectorModel.entityId > 0
                onTextChanged: inspectorModel.filter = text
                Keys.onEscapePressed: text = ""
            }

            MfButton {
                id: addComponentButton
                width: 120
                text: "+ Component"
                accent: true
                enabled: inspectorModel.entityId > 0
                onClicked: componentPopup.open()
            }

            MfButton {
                id: actionsButton
                width: 78
                text: "Actions"
                enabled: inspectorModel.entityId > 0
                onClicked: inspectorMenu.popup()
            }
        }

        Column {
            width: parent.width
            height: visible ? 64 : 0
            spacing: 4
            visible: quickActionModel.count > 0

            Text {
                width: parent.width
                height: 16
                text: root.quickStatus.length > 0 ? root.quickStatus : "Quick Actions"
                color: root.quickStatus.length > 0 ? Theme.DarkTheme.muted : Theme.DarkTheme.accent
                font.pixelSize: 10
                elide: Text.ElideRight
            }

            Flickable {
                width: parent.width
                height: 42
                contentWidth: quickActionRow.width
                contentHeight: height
                clip: true
                boundsBehavior: Flickable.StopAtBounds

                Row {
                    id: quickActionRow
                    spacing: 6
                    Repeater {
                        model: quickActionModel
                        delegate: MfButton {
                            required property var model
                            width: Math.max(104, implicitWidth)
                            height: 34
                            text: model.label
                            enabled: model.enabled
                            ToolTip.visible: hovered && !enabled
                            ToolTip.text: model.disabledReason
                            onClicked: root.beginQuickAction(
                                model.actionId,
                                model.label,
                                model.requiresAsset,
                                model.attachedAssetPath,
                                model.assetsJson
                            )
                        }
                    }
                }
            }
        }

        ListView {
            id: inspectorList
            width: parent.width
            height: parent.height - y
            clip: true
            model: inspectorModel
            section.property: "target"
            section.criteria: ViewSection.FullString
            section.delegate: Rectangle {
                id: inspectorSection
                required property string section

                width: ListView.view.width
                height: 30
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft
                border.width: 1

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 6
                    spacing: 6

                    Text {
                        width: parent.width - removeSection.width - 6
                        height: parent.height
                        text: inspectorSection.section
                        color: Theme.DarkTheme.accent
                        font.pixelSize: 11
                        font.bold: true
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    MfButton {
                        id: removeSection
                        width: visible ? 66 : 0
                        height: 24
                        anchors.verticalCenter: parent.verticalCenter
                        visible: root.removableSection(inspectorSection.section)
                        text: "Remove"
                        onClicked: root.runAction(
                            "remove_component",
                            {"component_type": inspectorSection.section}
                        )
                    }
                }
            }

            delegate: Rectangle {
                id: inspectorField
                required property int index
                required property var model

                width: ListView.view.width
                height: Theme.DarkTheme.rowHeight
                color: inspectorField.index % 2 === 0 ? Theme.DarkTheme.panel : Theme.DarkTheme.surfaceRaised
                MfPropertyRow {
                    anchors.fill: parent
                    label: inspectorField.model.displayName
                    value: inspectorField.model.valueJson
                    valueType: inspectorField.model.valueType
                    editable: inspectorField.model.editable
                    mixed: inspectorField.model.mixed
                    onValueCommitted: function(value) {
                        editorController.setInspectorValue(
                            inspectorField.model.entityId,
                            inspectorField.model.target,
                            inspectorField.model.key,
                            value
                        )
                    }
                }
            }

            Text {
                visible: inspectorList.count === 0
                anchors.centerIn: parent
                width: Math.max(100, parent.width - 20)
                text: inspectorModel.entityId > 0 ? "No matching properties" : "Select an entity in Scene or Hierarchy"
                color: Theme.DarkTheme.muted
                font.pixelSize: 11
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
            }
        }
    }
}
