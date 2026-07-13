import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property var studioData: ({"prefab_assets": [], "selected_entity_id": null, "selected_instance": null})
    property string statusText: "Select an entity or prefab asset"
    property bool operationOk: true

    function decode(json, fallback) {
        try {
            return JSON.parse(json)
        } catch (error) {
            statusText = "Bridge JSON error: " + error
            operationOk = false
            return fallback
        }
    }

    function refresh() {
        if (!editorBridge.projectOpen)
            return
        studioData = decode(editorBridge.prefabStateJson(), studioData)
    }

    function runAction(action, payload) {
        var result = decode(editorBridge.prefabActionJson(action, JSON.stringify(payload || {})), null)
        if (result === null)
            return
        operationOk = result.changed === true
        statusText = result.message || action
        refresh()
    }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.refresh() }
        function onSelectionChanged() { root.refresh() }
        function onAssetsChanged() { root.refresh() }
        function onDataChanged() { root.refresh() }
    }

    Component.onCompleted: refresh()

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Prefab Studio"
            detail: root.studioData.prefab_assets.length + " assets · "
                    + (root.studioData.selected_entity_id === null
                       ? "no selection" : "entity #" + root.studioData.selected_entity_id)
            badge: root.studioData.selected_instance !== null ? "Instance" : "Prefab"
            badgeColor: root.studioData.selected_instance !== null
                        ? Theme.DarkTheme.accent : Theme.DarkTheme.info
        }

        Flow {
            width: parent.width
            spacing: 6

            MfButton {
                text: "Create from Selection"
                accent: true
                enabled: root.studioData.selected_entity_id !== null
                onClicked: root.runAction("create_from_selected", {})
            }
            MfButton {
                text: "Create Variant"
                enabled: root.studioData.selected_instance !== null
                onClicked: root.runAction("create_variant", {})
            }
            MfButton {
                text: "Apply"
                enabled: root.studioData.selected_instance !== null
                         && root.studioData.selected_instance.can_apply === true
                onClicked: root.runAction("apply_overrides", {})
            }
            MfButton {
                text: "Revert"
                enabled: root.studioData.selected_instance !== null
                         && root.studioData.selected_instance.can_apply === true
                onClicked: root.runAction("revert_overrides", {})
            }
            MfButton {
                text: "Detach"
                enabled: root.studioData.selected_instance !== null
                onClicked: root.runAction("detach", {})
            }
        }

        Rectangle {
            width: parent.width
            height: 72
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft

            Column {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 4

                Text {
                    width: parent.width
                    text: root.studioData.selected_instance === null
                          ? "The selected entity is not a prefab instance."
                          : ((root.studioData.selected_instance.missing_source ? "Missing source · " : "Source ready · ")
                             + root.studioData.selected_instance.override_count + " overrides · "
                             + root.studioData.selected_instance.component_count + " components")
                    color: root.studioData.selected_instance !== null
                           && root.studioData.selected_instance.missing_source
                           ? Theme.DarkTheme.danger : Theme.DarkTheme.text
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }
                Text {
                    width: parent.width
                    text: root.studioData.selected_instance === null
                          ? "Create a reusable prefab from the current selection."
                          : (root.studioData.selected_instance.prefab_source || "Detached source")
                    color: Theme.DarkTheme.muted
                    font.pixelSize: 10
                    elide: Text.ElideMiddle
                }
                Text {
                    width: parent.width
                    text: root.statusText
                    color: root.operationOk ? Theme.DarkTheme.accent : Theme.DarkTheme.danger
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }
        }

        ListView {
            id: prefabList
            width: parent.width
            height: parent.height - y
            clip: true
            spacing: 5
            model: root.studioData.prefab_assets

            delegate: Rectangle {
                id: prefabRow
                required property var modelData
                width: ListView.view.width
                height: 54
                radius: Theme.DarkTheme.cardRadius
                color: rowMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.surface
                border.color: rowMouse.containsMouse ? Theme.DarkTheme.focus : Theme.DarkTheme.borderSoft

                Row {
                    anchors.fill: parent
                    anchors.margins: 7
                    spacing: 8

                    Column {
                        width: parent.width - instantiateButton.width - 12
                        spacing: 4
                        Text {
                            width: parent.width
                            text: prefabRow.modelData.name
                            color: Theme.DarkTheme.text
                            font.pixelSize: 12
                            font.bold: true
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: prefabRow.modelData.relative_path
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 10
                            elide: Text.ElideMiddle
                        }
                    }

                    MfButton {
                        id: instantiateButton
                        width: 92
                        text: "Instantiate"
                        accent: true
                        onClicked: root.runAction("instantiate", {
                            "relative_path": prefabRow.modelData.relative_path,
                            "x": 0,
                            "y": 0
                        })
                    }
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }

            Text {
                anchors.centerIn: parent
                visible: prefabList.count === 0
                text: "No prefab assets yet"
                color: Theme.DarkTheme.muted
                font.pixelSize: 12
            }
        }
    }
}
