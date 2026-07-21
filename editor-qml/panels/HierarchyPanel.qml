import QtQuick
import QtQuick.Controls
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property string sceneName: "Scene"
    property string sceneMode: "EDITOR"
    property bool sceneDirty: false
    property string dirtyReason: ""

    function refreshSceneState() {
        var json = editorBridge.sceneStateJson()
        if (json.length === 0)
            return
        try {
            var state = JSON.parse(json)
            sceneName = state.scene_name || "Scene"
            sceneMode = state.mode || "EDITOR"
            sceneDirty = state.dirty === true
            dirtyReason = state.dirty_reason || ""
        } catch (error) {
            dirtyReason = "Invalid scene state · " + error
        }
    }

    function selectionMode(modifiers) {
        if ((modifiers & Qt.ControlModifier) || (modifiers & Qt.MetaModifier))
            return "toggle"
        if (modifiers & Qt.ShiftModifier)
            return "add"
        return "replace"
    }

    function runAction(entityId, action, payload) {
        return editorBridge.performEntityAction(entityId, action, JSON.stringify(payload || {}))
    }

    Connections {
        target: editorBridge
        function onSceneStateChanged() { root.refreshSceneState() }
        function onProjectChanged() { root.refreshSceneState() }
    }

    Component.onCompleted: refreshSceneState()

    Menu {
        id: createMenu
        MenuItem { text: "Empty Entity"; onTriggered: editorBridge.executeCommand("entity.create_empty") }
        MenuItem { text: "Node2D"; onTriggered: editorBridge.executeCommand("object.create_node2d") }
        MenuItem { text: "Sprite Actor"; onTriggered: editorBridge.executeCommand("object.create_sprite_actor") }
        MenuItem { text: "Camera Rig"; onTriggered: editorBridge.executeCommand("object.create_camera") }
        MenuItem { text: "Area2D"; onTriggered: editorBridge.executeCommand("object.create_area2d") }
        MenuItem { text: "CharacterBody2D"; onTriggered: editorBridge.executeCommand("object.create_character_body2d") }
        MenuSeparator {}
        MenuItem { text: "HUD Text"; onTriggered: editorBridge.executeCommand("object.create_ui_text") }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: root.sceneName + (root.sceneDirty ? " *" : "")
            detail: tree.rows + " visible · " + editorBridge.selectedEntityCount + " selected"
            badge: root.sceneMode
            badgeColor: root.sceneMode === "PLAY" ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        Row {
            width: parent.width
            height: 32
            spacing: 6

            MfSearchBar {
                id: hierarchySearch
                width: Math.max(80, parent.width - addButton.width - clearButton.width - 12)
                placeholderText: "Search name, tag, layer or type"
                onTextChanged: hierarchyModel.filter = text
                Keys.onEscapePressed: text = ""
            }

            MfButton {
                id: clearButton
                width: 62
                text: "Clear"
                enabled: editorBridge.selectedEntityCount > 0
                onClicked: editorBridge.clearSelection()
            }

            MfButton {
                id: addButton
                width: 68
                text: "+ Add"
                accent: true
                enabled: editorBridge.projectOpen
                onClicked: createMenu.popup()
            }
        }

        Text {
            visible: root.sceneDirty && root.dirtyReason.length > 0
            width: parent.width
            height: visible ? 18 : 0
            text: "Unsaved · " + root.dirtyReason
            color: Theme.DarkTheme.warning
            font.pixelSize: 9
            elide: Text.ElideRight
        }

        TreeView {
            id: tree
            width: parent.width
            height: parent.height - y
            clip: true
            model: hierarchyModel
            focus: true

            delegate: Rectangle {
                id: entityDelegate
                required property TreeView treeView
                required property bool isTreeNode
                required property int depth
                required property bool expanded
                required property bool hasChildren
                required property int row
                required property var model

                property bool renaming: false
                property bool dropHover: false

                implicitWidth: entityDelegate.treeView.width
                implicitHeight: Theme.DarkTheme.rowHeight
                width: entityDelegate.treeView.width
                height: Theme.DarkTheme.rowHeight
                color: entityDelegate.dropHover
                    ? Theme.DarkTheme.warningSoft
                    : (entityDelegate.model.selected
                        ? Theme.DarkTheme.accentSoft
                        : (rowMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.panel))
                border.color: entityDelegate.dropHover
                    ? Theme.DarkTheme.warning
                    : (entityDelegate.model.selected ? Theme.DarkTheme.accent : "transparent")
                border.width: entityDelegate.model.selected || entityDelegate.dropHover ? 1 : 0

                Drag.active: dragHandler.active
                Drag.source: entityDelegate
                Drag.keys: ["MiniForgeEntity"]
                Drag.mimeData: {"application/x-miniforge-entity": String(entityDelegate.model.entityId)}
                Drag.hotSpot.x: width / 2
                Drag.hotSpot.y: height / 2

                Row {
                    z: 1
                    anchors.fill: parent
                    anchors.leftMargin: 6 + entityDelegate.depth * 16
                    anchors.rightMargin: 5
                    spacing: 5

                    Text {
                        width: 14
                        height: parent.height
                        text: entityDelegate.isTreeNode && entityDelegate.hasChildren
                            ? (entityDelegate.expanded ? "▾" : "▸")
                            : ""
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 11
                        verticalAlignment: Text.AlignVCenter
                    }

                    Text {
                        id: enabledIcon
                        width: 15
                        height: parent.height
                        text: entityDelegate.model.enabled ? "●" : "○"
                        color: entityDelegate.model.enabled ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
                        font.pixelSize: 9
                        verticalAlignment: Text.AlignVCenter

                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.runAction(
                                entityDelegate.model.entityId,
                                "set_enabled",
                                {"value": !entityDelegate.model.enabled}
                            )
                        }
                    }

                    Item {
                        width: Math.max(54, parent.width - 104)
                        height: parent.height

                        Text {
                            anchors.fill: parent
                            visible: !entityDelegate.renaming
                            text: entityDelegate.model.name
                            color: entityDelegate.model.locked ? Theme.DarkTheme.muted : Theme.DarkTheme.text
                            font.pixelSize: 12
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }

                        TextInput {
                            id: renameEditor
                            anchors.fill: parent
                            visible: entityDelegate.renaming
                            text: entityDelegate.model.name
                            color: Theme.DarkTheme.text
                            selectionColor: Theme.DarkTheme.accent
                            selectedTextColor: Theme.DarkTheme.background
                            font.pixelSize: 12
                            verticalAlignment: TextInput.AlignVCenter
                            selectByMouse: true
                            onVisibleChanged: {
                                if (visible) {
                                    forceActiveFocus()
                                    selectAll()
                                }
                            }
                            onEditingFinished: {
                                var nextName = text.trim()
                                if (nextName.length > 0 && nextName !== entityDelegate.model.name)
                                    root.runAction(entityDelegate.model.entityId, "rename", {"name": nextName})
                                entityDelegate.renaming = false
                            }
                            Keys.onEscapePressed: {
                                text = entityDelegate.model.name
                                entityDelegate.renaming = false
                                event.accepted = true
                            }
                        }
                    }

                    Text {
                        width: 30
                        height: parent.height
                        text: entityDelegate.model.componentCount
                        color: Theme.DarkTheme.warning
                        font.pixelSize: 10
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignVCenter
                        ToolTip.visible: componentMouse.containsMouse
                        ToolTip.text: entityDelegate.model.componentCount + " components · " + entityDelegate.model.layer

                        MouseArea {
                            id: componentMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.NoButton
                        }
                    }

                    Text {
                        id: visibleIcon
                        width: 16
                        height: parent.height
                        text: entityDelegate.model.visible ? "◉" : "—"
                        color: entityDelegate.model.visible ? Theme.DarkTheme.info : Theme.DarkTheme.muted
                        font.pixelSize: 10
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter

                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.runAction(
                                entityDelegate.model.entityId,
                                "set_visible",
                                {"value": !entityDelegate.model.visible}
                            )
                        }
                    }

                    Text {
                        width: 16
                        height: parent.height
                        text: entityDelegate.model.locked ? "◆" : "◇"
                        color: entityDelegate.model.locked ? Theme.DarkTheme.warning : Theme.DarkTheme.muted
                        font.pixelSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter

                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.runAction(
                                entityDelegate.model.entityId,
                                "set_locked",
                                {"value": !entityDelegate.model.locked}
                            )
                        }
                    }
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: function(event) {
                        if (event.button === Qt.RightButton) {
                            if (!entityDelegate.model.selected)
                                editorBridge.updateSelection(entityDelegate.model.entityId, "replace")
                            entityMenu.popup()
                            return
                        }
                        if (entityDelegate.isTreeNode
                                && entityDelegate.hasChildren
                                && event.x <= 26 + entityDelegate.depth * 16) {
                            entityDelegate.treeView.toggleExpanded(entityDelegate.row)
                        }
                        editorBridge.updateSelection(
                            entityDelegate.model.entityId,
                            root.selectionMode(event.modifiers)
                        )
                    }
                    onDoubleClicked: function(event) {
                        if (event.button === Qt.LeftButton && !entityDelegate.model.locked)
                            entityDelegate.renaming = true
                    }
                }

                DragHandler {
                    id: dragHandler
                    target: null
                    enabled: !entityDelegate.model.locked
                }

                DropArea {
                    anchors.fill: parent
                    keys: ["MiniForgeEntity"]
                    onEntered: function(drag) {
                        var sourceId = Number(drag.getDataAsString("application/x-miniforge-entity"))
                        entityDelegate.dropHover = sourceId > 0
                            && sourceId !== entityDelegate.model.entityId
                    }
                    onExited: entityDelegate.dropHover = false
                    onDropped: function(drop) {
                        entityDelegate.dropHover = false
                        var sourceId = Number(drop.getDataAsString("application/x-miniforge-entity"))
                        if (sourceId <= 0 || sourceId === entityDelegate.model.entityId)
                            return
                        root.runAction(
                            sourceId,
                            "reparent",
                            {"parent_id": entityDelegate.model.entityId}
                        )
                        drop.acceptProposedAction()
                    }
                }

                Menu {
                    id: entityMenu
                    MenuItem {
                        text: "Rename"
                        enabled: !entityDelegate.model.locked
                        onTriggered: entityDelegate.renaming = true
                    }
                    MenuItem {
                        text: "Duplicate"
                        onTriggered: root.runAction(entityDelegate.model.entityId, "duplicate", {})
                    }
                    MenuItem {
                        text: "Reset Transform"
                        onTriggered: root.runAction(entityDelegate.model.entityId, "reset_transform", {})
                    }
                    MenuItem {
                        text: "Move to Scene Root"
                        enabled: entityDelegate.model.hasParent
                        onTriggered: root.runAction(entityDelegate.model.entityId, "unparent", {})
                    }
                    MenuSeparator {}
                    MenuItem {
                        text: "Pack Selected Branch"
                        onTriggered: editorBridge.executeCommand("scene.pack_selected")
                    }
                    MenuSeparator {}
                    MenuItem {
                        text: "Delete"
                        onTriggered: root.runAction(entityDelegate.model.entityId, "delete", {})
                    }
                }
            }

            Text {
                visible: tree.rows === 0
                anchors.centerIn: parent
                width: Math.max(100, parent.width - 20)
                text: hierarchySearch.text.length > 0 ? "No matching entities" : "No entities · use + Add to create one"
                color: Theme.DarkTheme.muted
                font.pixelSize: 11
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
            }
        }
    }
}
