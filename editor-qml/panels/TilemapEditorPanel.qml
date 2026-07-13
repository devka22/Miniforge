import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property var toolState: ({"editor":{"tilemap":{"width":1,"height":1,"layers":[]},"active_layer":0,"palette":{"selected":1,"tiles":{}},"selection":{"cells":[]}},"issues":[]})
    property string tool: "Pencil"
    property real zoom: 1
    property var strokeCells: []
    property var strokeStart: ({"x":0,"y":0})
    property string statusText: "Open a project to edit a tilemap"
    readonly property var editor: toolState.editor || ({})
    readonly property var map: editor.tilemap || ({"width":1,"height":1,"layers":[]})

    function parse(source, fallback) {
        try { return JSON.parse(source) } catch (error) {
            statusText = "Tilemap JSON error · " + error
            return fallback
        }
    }

    function acceptState(source, action) {
        if (source.length === 0) {
            statusText = editorBridge.lastError
            return
        }
        toolState = parse(source, toolState)
        statusText = action || (toolState.dirty ? "Unsaved terrain edits" : toolState.document_path)
        mapCanvas.requestPaint()
    }

    function refresh() {
        if (editorBridge.projectOpen)
            acceptState(editorBridge.toolStateJson("tilemap"), "Tilemap ready")
    }

    function run(action, payload) {
        acceptState(editorBridge.toolActionJson("tilemap", action, JSON.stringify(payload || {})), action + " complete")
    }

    function cellAt(x, y) {
        var cellWidth = mapCanvas.width / Math.max(1, Number(map.width || 1))
        var cellHeight = mapCanvas.height / Math.max(1, Number(map.height || 1))
        return {
            "x": Math.max(0, Math.min(Number(map.width || 1) - 1, Math.floor(x / cellWidth))),
            "y": Math.max(0, Math.min(Number(map.height || 1) - 1, Math.floor(y / cellHeight)))
        }
    }

    function appendStroke(cell) {
        for (var index = 0; index < strokeCells.length; ++index) {
            if (strokeCells[index].x === cell.x && strokeCells[index].y === cell.y)
                return
        }
        var next = strokeCells.slice()
        next.push(cell)
        strokeCells = next
    }

    function finishStroke(cell) {
        var tile = tool === "Eraser" ? 0 : Number(editor.palette.selected || 0)
        var payload = {"layer":Number(editor.active_layer || 0), "start":strokeStart, "end":cell, "value":tile}
        if (tool === "Pencil" || tool === "Eraser") {
            run("paint_cells", {"layer":payload.layer, "cells":strokeCells, "value":tile})
        } else if (tool === "Line") {
            run("line", payload)
        } else if (tool === "Rectangle") {
            payload.filled = false
            run("rectangle", payload)
        } else if (tool === "Fill") {
            run("flood_fill", {"layer":payload.layer, "origin":cell, "value":tile})
        } else if (tool === "Select") {
            run("select", payload)
        } else if (tool === "Terrain") {
            run("terrain", {"layer":payload.layer})
        }
        strokeCells = []
    }

    function tileColor(value, alpha) {
        if (value === 0) return Qt.rgba(0.10, 0.11, 0.13, alpha)
        if (value === 1) return Qt.rgba(0.25, 0.62, 0.32, alpha)
        if (value === 2) return Qt.rgba(0.53, 0.35, 0.20, alpha)
        if (value === 3) return Qt.rgba(0.48, 0.50, 0.54, alpha)
        if (value === 4) return Qt.rgba(0.16, 0.43, 0.72, alpha)
        if (value === 9) return Qt.rgba(0.20, 0.22, 0.26, alpha)
        var hue = (Math.abs(value) * 0.117) % 1
        return Qt.hsva(hue, 0.62, 0.78, alpha)
    }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.refresh() }
        function onEditorToolChanged(toolName) { if (toolName === "tilemap") root.refresh() }
    }
    Component.onCompleted: refresh()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 7

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Tilemap & Terrain"
            detail: root.statusText
            badge: root.map.width + "×" + root.map.height + " · " + (root.map.layers || []).length + " layers"
            badgeColor: root.toolState.dirty ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 5
            MfButton { text: "Undo"; enabled: root.toolState.can_undo === true; onClicked: root.run("undo", {}) }
            MfButton { text: "Redo"; enabled: root.toolState.can_redo === true; onClicked: root.run("redo", {}) }
            MfButton { text: "Save"; accent: root.toolState.dirty === true; onClicked: root.run("save", {}) }
            MfButton { text: "Terrain Rules"; onClicked: terrainPopup.open() }
            MfButton { text: "Apply Rules"; onClicked: root.run("rules", {"layer":Number(root.editor.active_layer || 0)}) }
            MfButton { text: "Copy"; enabled: (root.editor.selection.cells || []).length > 0; onClicked: root.run("copy", {"name":"Tile Selection"}) }
            MfButton { text: "Paste"; enabled: root.editor.clipboard !== null; onClicked: root.run("paste", {"origin":root.strokeStart}) }
            Item { Layout.fillWidth: true }
            Text { text: "Zoom"; color: Theme.DarkTheme.muted; font.pixelSize: 10 }
            Slider { Layout.preferredWidth: 90; from: 0.5; to: 2.5; value: root.zoom; onMoved: root.zoom = value }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 7

            Rectangle {
                Layout.preferredWidth: 176
                Layout.fillHeight: true
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft

                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 7; spacing: 6
                    Text { text: "BRUSH"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true }
                    GridLayout {
                        Layout.fillWidth: true; columns: 2; columnSpacing: 4; rowSpacing: 4
                        Repeater {
                            model: ["Pencil", "Eraser", "Line", "Rectangle", "Fill", "Terrain", "Select"]
                            delegate: MfButton {
                                required property var modelData
                                text: modelData
                                accent: root.tool === modelData
                                Layout.fillWidth: true
                                onClicked: {
                                    root.tool = modelData
                                    root.run("set_brush", {"brush":modelData})
                                }
                            }
                        }
                    }
                    Text { text: "PALETTE"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: root.toolState.atlas && root.toolState.atlas.source ? 112 : 0
                        visible: Layout.preferredHeight > 0
                        color: Theme.DarkTheme.background; radius: 4; border.color: Theme.DarkTheme.borderSoft; clip: true
                        Image {
                            id: atlasImage
                            anchors.fill: parent; anchors.margins: 3
                            source: root.toolState.atlas ? root.toolState.atlas.source : ""
                            fillMode: Image.PreserveAspectFit
                            smooth: false
                        }
                        MouseArea {
                            anchors.fill: atlasImage
                            onClicked: function(mouse) {
                                var tileSize = Number(root.toolState.atlas.tile_size || 16)
                                var columns = Math.max(1, Math.floor(atlasImage.paintedWidth / tileSize))
                                var localX = mouse.x - (atlasImage.width-atlasImage.paintedWidth)/2
                                var localY = mouse.y - (atlasImage.height-atlasImage.paintedHeight)/2
                                if (localX >= 0 && localY >= 0)
                                    root.run("set_tile", {"tile":Math.floor(localY/tileSize)*columns+Math.floor(localX/tileSize)+1})
                            }
                        }
                        Text { anchors.left: parent.left; anchors.bottom: parent.bottom; anchors.margins: 4; text: "Atlas · click tile"; color: "white"; font.pixelSize: 8; style: Text.Outline; styleColor: "black" }
                    }
                    Flow {
                        Layout.fillWidth: true; spacing: 4
                        Repeater {
                            model: Object.keys(root.editor.palette.tiles || {})
                            delegate: Rectangle {
                                required property var modelData
                                property int tileValue: Number(root.editor.palette.tiles[modelData])
                                width: 48; height: 42; radius: 5
                                color: root.tileColor(tileValue, 1)
                                border.width: Number(root.editor.palette.selected) === tileValue ? 2 : 1
                                border.color: Number(root.editor.palette.selected) === tileValue ? Theme.DarkTheme.warning : Theme.DarkTheme.border
                                Text { anchors.centerIn: parent; text: modelData + "\n" + parent.tileValue; color: "white"; font.pixelSize: 8; horizontalAlignment: Text.AlignHCenter }
                                MouseArea { anchors.fill: parent; onClicked: root.run("set_tile", {"tile":parent.tileValue}) }
                            }
                        }
                    }
                    Text { text: "LAYERS"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true }
                    ListView {
                        id: layerList
                        Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 3
                        model: root.map.layers || []
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: layerList.width; height: 34; radius: 4
                            color: Number(root.editor.active_layer) === index ? Theme.DarkTheme.surfaceRaised : "transparent"
                            Row {
                                anchors.fill: parent; anchors.margins: 5; spacing: 5
                                CheckBox {
                                    checked: modelData.visible !== false
                                    onToggled: root.run("set_layer_visible", {"layer":index, "value":checked})
                                }
                                Text { width: parent.width - 56; anchors.verticalCenter: parent.verticalCenter; text: modelData.name + (modelData.locked ? "  🔒" : ""); color: Theme.DarkTheme.text; elide: Text.ElideRight; font.pixelSize: 10 }
                            }
                            MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton; onClicked: root.run("set_layer", {"layer":index}) }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        TextField { id: layerName; Layout.fillWidth: true; placeholderText: "Layer name" }
                        MfButton { text: "+"; onClicked: { root.run("add_layer", {"name":layerName.text || "Layer"}); layerName.clear() } }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: Theme.DarkTheme.cardRadius
                color: "#15171b"
                border.color: Theme.DarkTheme.borderSoft
                clip: true

                Item {
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 16, Math.max(240, Number(root.map.width || 1) * 24 * root.zoom))
                    height: Math.min(parent.height - 16, Math.max(160, Number(root.map.height || 1) * 24 * root.zoom))

                    Canvas {
                        id: mapCanvas
                        anchors.fill: parent
                        antialiasing: false
                        onPaint: {
                            var context = getContext("2d")
                            context.reset()
                            context.fillStyle = "#181b20"
                            context.fillRect(0, 0, width, height)
                            var columns = Math.max(1, Number(root.map.width || 1))
                            var rows = Math.max(1, Number(root.map.height || 1))
                            var cw = width / columns
                            var ch = height / rows
                            var layers = root.map.layers || []
                            for (var layer = 0; layer < layers.length; ++layer) {
                                if (layers[layer].visible === false) continue
                                var tiles = layers[layer].tiles || []
                                for (var y = 0; y < rows; ++y) {
                                    for (var x = 0; x < columns; ++x) {
                                        var value = tiles[y] ? Number(tiles[y][x] || 0) : 0
                                        if (value !== 0) {
                                            context.fillStyle = root.tileColor(value, layer === Number(root.editor.active_layer) ? 1 : 0.42)
                                            context.fillRect(x * cw, y * ch, Math.ceil(cw), Math.ceil(ch))
                                        }
                                    }
                                }
                            }
                            context.strokeStyle = "#303640"
                            context.lineWidth = 1
                            for (var gx = 0; gx <= columns; ++gx) { context.beginPath(); context.moveTo(gx*cw,0); context.lineTo(gx*cw,height); context.stroke() }
                            for (var gy = 0; gy <= rows; ++gy) { context.beginPath(); context.moveTo(0,gy*ch); context.lineTo(width,gy*ch); context.stroke() }
                            context.strokeStyle = "#54d49a"
                            var selected = root.editor.selection.cells || []
                            for (var s = 0; s < selected.length; ++s)
                                context.strokeRect(selected[s].x*cw+1, selected[s].y*ch+1, Math.max(1,cw-2), Math.max(1,ch-2))
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onPressed: function(mouse) {
                            root.strokeStart = root.cellAt(mouse.x, mouse.y)
                            root.strokeCells = []
                            root.appendStroke(root.strokeStart)
                        }
                        onPositionChanged: function(mouse) {
                            if (pressed && (root.tool === "Pencil" || root.tool === "Eraser"))
                                root.appendStroke(root.cellAt(mouse.x, mouse.y))
                        }
                        onReleased: function(mouse) { root.finishStroke(root.cellAt(mouse.x, mouse.y)) }
                    }
                }
            }
        }
    }

    Popup {
        id: terrainPopup
        anchors.centerIn: parent
        width: Math.min(520, root.width - 20)
        height: Math.min(520, root.height - 20)
        modal: true; focus: true
        background: Rectangle { color: Theme.DarkTheme.surfaceRaised; radius: 8; border.color: Theme.DarkTheme.border }
        ColumnLayout {
            anchors.fill: parent; anchors.margins: 12; spacing: 7
            MfPanelHeader {
                Layout.fillWidth: true
                title: "Terrain & Rule Tiles"
                detail: (root.editor.terrain_sets || []).length + " terrain sets · " + (root.editor.rule_tiles || []).length + " probabilistic rules"
                badge: "Auto-tile"; badgeColor: Theme.DarkTheme.accent
            }
            TabBar {
                id: ruleTabs
                Layout.fillWidth: true
                TabButton { text: "Terrain" }
                TabButton { text: "Rule Tiles" }
            }
            StackLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; currentIndex: ruleTabs.currentIndex
                ColumnLayout {
                    ListView {
                        Layout.fillWidth: true; Layout.fillHeight: true; clip: true
                        model: root.editor.terrain_sets || []
                        delegate: Column {
                            id: terrainSetRow
                            required property var modelData
                            width: ListView.view.width
                            Text { text: modelData.name; color: Theme.DarkTheme.accent; font.bold: true }
                            Repeater {
                                model: parent.modelData.rules || []
                                delegate: Row {
                                    required property var modelData
                                    width: parent.width; height: 30; spacing: 6
                                    Text { width: parent.width-80; text: modelData.name + " · " + modelData.center_tile + " → " + modelData.output_tile; color: Theme.DarkTheme.text; verticalAlignment: Text.AlignVCenter }
                                    MfButton { width: 70; text: "Remove"; onClicked: root.run("remove_terrain_rule", {"terrain_set":terrainSetRow.modelData.name,"name":modelData.name}) }
                                }
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        TextField { id: terrainSetName; Layout.fillWidth: true; placeholderText: "Set"; text: "Terrain" }
                        TextField { id: terrainRuleName; Layout.fillWidth: true; placeholderText: "Rule name" }
                        TextField { id: terrainOutput; Layout.preferredWidth: 64; placeholderText: "Tile"; text: String(root.editor.palette.selected || 1) }
                        MfButton { text: "Add"; accent: true; onClicked: root.run("add_terrain_rule", {"terrain_set":terrainSetName.text,"name":terrainRuleName.text,"center_tile":Number(root.editor.palette.selected),"output_tile":Number(terrainOutput.text),"priority":0}) }
                    }
                }
                ColumnLayout {
                    ListView {
                        Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 3
                        model: root.editor.rule_tiles || []
                        delegate: Row {
                            required property var modelData
                            width: ListView.view.width; height: 34; spacing: 6
                            Text { width: parent.width-82; text: modelData.name + " · tile " + modelData.output_tile + " · " + modelData.probability_percent + "%"; color: Theme.DarkTheme.text; verticalAlignment: Text.AlignVCenter }
                            MfButton { width: 72; text: "Remove"; onClicked: root.run("remove_rule_tile", {"name":modelData.name}) }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        TextField { id: ruleName; Layout.fillWidth: true; placeholderText: "Rule tile name" }
                        TextField { id: ruleOutput; Layout.preferredWidth: 62; placeholderText: "Tile"; text: String(root.editor.palette.selected || 1) }
                        TextField { id: ruleChance; Layout.preferredWidth: 62; placeholderText: "%"; text: "100" }
                        MfButton { text: "Add"; accent: true; onClicked: root.run("add_rule_tile", {"name":ruleName.text,"output_tile":Number(ruleOutput.text),"probability_percent":Number(ruleChance.text)}) }
                    }
                }
            }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                MfButton { text: "Apply to Layer"; accent: true; onClicked: root.run("rules", {"layer":Number(root.editor.active_layer || 0)}) }
                MfButton { text: "Close"; onClicked: terrainPopup.close() }
            }
        }
    }
}
