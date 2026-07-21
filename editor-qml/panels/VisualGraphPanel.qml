import QtQuick
import QtQuick.Controls
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property string currentPath: ""
    property var graphDocument: ({})
    property string savedGraphText: ""
    property string selectedNodeId: ""
    property string statusText: "Select or create a Visual Graph"
    property var graphCatalog: []
    property var graphTemplates: []
    property bool dirty: false

    function markGraphChanged() {
        dirty = currentPath.length > 0
    }

    function parseJson(value, fallback) {
        try { return JSON.parse(value) } catch (error) {
            statusText = "JSON error: " + error
            return fallback
        }
    }

    function loadCatalog() {
        var catalog = parseJson(editorBridge.visualGraphCatalogJson(), {"nodes":[], "templates":[]})
        graphCatalog = catalog.nodes || []
        graphTemplates = catalog.templates || []
        filterPalette()
    }

    function refreshGraphs(preferredPath, reloadCurrent) {
        var entries = parseJson(editorBridge.contentEntriesJson("scripts/visual_graphs"), [])
        graphListModel.clear()
        for (var index = 0; index < entries.length; ++index) {
            if (entries[index].asset_type === "VisualGraph") {
                graphListModel.append({
                    "name": entries[index].name,
                    "relativePath": entries[index].relative_path
                })
            }
        }
        var target = preferredPath || currentPath
        if (target.length === 0 && graphListModel.count > 0)
            target = graphListModel.get(0).relativePath
        if (target.length > 0 && (currentPath.length === 0 || reloadCurrent))
            openGraph(target, true)
    }

    function openGraph(path, force) {
        if (!force && dirty && path !== currentPath) {
            statusText = "Save or revert " + currentPath + " before switching graphs"
            return
        }
        var source = editorBridge.readTextAsset(path)
        if (source.length === 0 && editorBridge.lastError.length > 0) {
            statusText = editorBridge.lastError
            return
        }
        var parsed = parseJson(source, null)
        if (!parsed || !Array.isArray(parsed.nodes)) {
            statusText = "Visual Graph requires a JSON object with a nodes array"
            return
        }
        currentPath = path
        graphDocument = parsed
        savedGraphText = JSON.stringify(parsed, null, 2)
        dirty = false
        selectedNodeId = ""
        rebuildNodes()
        statusText = "Loaded " + path
    }

    function rebuildNodes() {
        nodeModel.clear()
        var nodes = graphDocument.nodes || []
        for (var index = 0; index < nodes.length; ++index) {
            var node = nodes[index]
            var position = node.position || {}
            nodeModel.append({
                "nodeId": String(node.id || ("node_" + index)),
                "nodeType": String(node.type || "Node"),
                "nodeX": Number(node.x === undefined ? (position.x === undefined ? 80 + (index % 4) * 250 : position.x) : node.x),
                "nodeY": Number(node.y === undefined ? (position.y === undefined ? 80 + Math.floor(index / 4) * 150 : position.y) : node.y),
                "nextId": node.next === null || node.next === undefined ? "" : String(node.next),
                "pinsJson": JSON.stringify(outputPinsForType(String(node.type || "Node"))),
                "dataJson": JSON.stringify(node)
            })
        }
        graphCanvas.requestPaint()
    }

    function outputPinsForType(typeName) {
        for (var index = 0; index < graphCatalog.length; ++index) {
            if (graphCatalog[index].type === typeName)
                return graphCatalog[index].output_pins || ["exec"]
        }
        return ["exec"]
    }

    function linkKey(pin) {
        if (pin === "true") return "true_next"
        if (pin === "false") return "false_next"
        if (pin === "a") return "a_next"
        if (pin === "b") return "b_next"
        if (pin === "then_0" || pin === "then_1") return pin
        return "next"
    }

    function targetForPin(row, pin) {
        var node = parseJson(row.dataJson, {})
        var target = node[linkKey(pin)]
        return target === null || target === undefined ? "" : String(target)
    }

    function syncGraph() {
        var nodes = []
        for (var index = 0; index < nodeModel.count; ++index) {
            var row = nodeModel.get(index)
            var node = parseJson(row.dataJson, {})
            node.id = row.nodeId
            node.type = row.nodeType
            node.x = Math.round(row.nodeX)
            node.y = Math.round(row.nodeY)
            node.position = {"x": node.x, "y": node.y}
            node.next = row.nextId.length > 0 ? row.nextId : null
            nodes.push(node)
        }
        graphDocument.nodes = nodes
        graphDocument.format = "miniforge.visual-graph"
        graphDocument.schema_version = 1
        graphDocument.engine_version = "0.9.3.4"
        graphDocument.kind = "MiniForgeVisualGraph"
        graphDocument.runtime = "rust_visual_graph"
    }

    function serializeGraph() {
        if (currentPath.length === 0)
            return ""
        syncGraph()
        return JSON.stringify(graphDocument, null, 2)
    }

    function validateGraph() {
        syncGraph()
        var ids = ({})
        var hasEvent = false
        for (var index = 0; index < graphDocument.nodes.length; ++index) {
            var node = graphDocument.nodes[index]
            if (!node.id || ids[node.id]) {
                statusText = "Duplicate or empty node id at node " + (index + 1)
                return false
            }
            ids[node.id] = true
            if (String(node.type).indexOf("Event") === 0)
                hasEvent = true
        }
        var linkKeys = ["next", "true_next", "false_next", "a_next", "b_next", "then_0", "then_1"]
        for (var second = 0; second < graphDocument.nodes.length; ++second) {
            for (var linkIndex = 0; linkIndex < linkKeys.length; ++linkIndex) {
                var next = graphDocument.nodes[second][linkKeys[linkIndex]]
                if (next && next === graphDocument.nodes[second].id) {
                    statusText = "A node cannot link to itself: " + next
                    return false
                }
                if (next && !ids[next]) {
                    statusText = "Broken link: " + graphDocument.nodes[second].id + "." + linkKeys[linkIndex] + " -> " + next
                    return false
                }
            }
        }
        if (!hasEvent) {
            statusText = "Graph needs at least one Event node"
            return false
        }
        var backendResult = parseJson(editorBridge.validateVisualGraph(currentPath, JSON.stringify(graphDocument)), null)
        if (!backendResult || backendResult.valid !== true) {
            statusText = editorBridge.lastError.length > 0 ? editorBridge.lastError : "Visual Graph schema validation failed"
            return false
        }
        statusText = "Graph validation passed · " + backendResult.node_count + " nodes"
        return true
    }

    function saveGraph() {
        if (!validateGraph())
            return
        var validation = parseJson(editorBridge.validateVisualGraph(currentPath, serializeGraph()), null)
        if (!validation || validation.valid !== true) {
            statusText = editorBridge.lastError
            return
        }
        graphDocument = validation.normalized
        var source = JSON.stringify(graphDocument, null, 2)
        if (editorBridge.saveVisualGraph(currentPath, source)) {
            savedGraphText = source
            dirty = false
            rebuildNodes()
            statusText = "Saved Visual Graph atomically · " + currentPath
            refreshGraphs(currentPath, false)
        } else {
            statusText = editorBridge.lastError
        }
    }

    function uniqueNodeId(typeName) {
        var base = typeName.replace(/[^A-Za-z0-9_]/g, "_").toLowerCase()
        var candidate = base
        var suffix = 2
        var used = ({})
        for (var index = 0; index < nodeModel.count; ++index)
            used[nodeModel.get(index).nodeId] = true
        while (used[candidate])
            candidate = base + "_" + suffix++
        return candidate
    }

    function addNode(catalogIndex) {
        if (currentPath.length === 0 || catalogIndex < 0 || catalogIndex >= paletteModel.count)
            return
        var definition = paletteModel.get(catalogIndex)
        var id = uniqueNodeId(definition.nodeType)
        var data = parseJson(definition.defaultsJson, {})
        data.id = id
        data.type = definition.nodeType
        data.next = null
        nodeModel.append({
            "nodeId": id,
            "nodeType": definition.nodeType,
            "nodeX": 120 + (nodeModel.count % 4) * 230,
            "nodeY": 100 + Math.floor(nodeModel.count / 4) * 145,
            "nextId": "",
            "pinsJson": JSON.stringify(parseJson(definition.pinsJson, ["exec"])),
            "dataJson": JSON.stringify(data)
        })
        selectedNodeId = id
        selectNode(id)
        graphCanvas.requestPaint()
        markGraphChanged()
        statusText = "Added " + definition.title
    }

    function selectNode(nodeId) {
        selectedNodeId = nodeId
        for (var index = 0; index < nodeModel.count; ++index) {
            if (nodeModel.get(index).nodeId === nodeId) {
                nodeJsonEditor.text = nodeModel.get(index).dataJson
                outputPin.currentIndex = 0
                refreshNextSelector()
                return
            }
        }
    }

    function indexForNodeId(nodeId) {
        for (var index = 0; index < nodeModel.count; ++index) {
            if (nodeModel.get(index).nodeId === nodeId)
                return index
        }
        return -1
    }

    function selectedNodeIndex() { return indexForNodeId(selectedNodeId) }

    function applyNodeJson() {
        var index = selectedNodeIndex()
        if (index < 0)
            return
        var node = parseJson(nodeJsonEditor.text, null)
        if (!node || !node.id || !node.type) {
            statusText = "Node JSON needs id and type"
            return
        }
        var oldId = nodeModel.get(index).nodeId
        var newId = String(node.id)
        var duplicate = indexForNodeId(newId)
        if (duplicate >= 0 && duplicate !== index) {
            statusText = "Another node already uses id " + newId
            return
        }
        var newX = node.x === undefined ? nodeModel.get(index).nodeX : Number(node.x)
        var newY = node.y === undefined ? nodeModel.get(index).nodeY : Number(node.y)
        if (!isFinite(newX) || !isFinite(newY)) {
            statusText = "Node x and y must be finite numbers"
            return
        }
        var newNext = node.next === null || node.next === undefined ? "" : String(node.next)
        if (newNext === newId) {
            statusText = "A node cannot link to itself"
            return
        }
        nodeModel.setProperty(index, "nodeId", newId)
        nodeModel.setProperty(index, "nodeType", String(node.type))
        nodeModel.setProperty(index, "pinsJson", JSON.stringify(outputPinsForType(String(node.type))))
        nodeModel.setProperty(index, "nodeX", newX)
        nodeModel.setProperty(index, "nodeY", newY)
        nodeModel.setProperty(index, "nextId", newNext)
        nodeModel.setProperty(index, "dataJson", JSON.stringify(node))
        if (oldId !== newId) {
            for (var row = 0; row < nodeModel.count; ++row) {
                if (row === index)
                    continue
                var linked = nodeModel.get(row)
                var linkedData = parseJson(linked.dataJson, {})
                var keys = ["next", "true_next", "false_next", "a_next", "b_next", "then_0", "then_1"]
                for (var keyIndex = 0; keyIndex < keys.length; ++keyIndex) {
                    if (linkedData[keys[keyIndex]] === oldId)
                        linkedData[keys[keyIndex]] = newId
                }
                nodeModel.setProperty(row, "dataJson", JSON.stringify(linkedData))
                nodeModel.setProperty(row, "nextId", linkedData.next || "")
            }
        }
        selectedNodeId = newId
        refreshNextSelector()
        graphCanvas.requestPaint()
        markGraphChanged()
        statusText = "Node properties applied"
    }

    function setSelectedNext() {
        var index = selectedNodeIndex()
        if (index < 0)
            return
        var next = nextNode.currentIndex <= 0 ? "" : nodeModel.get(nextNode.currentIndex - 1).nodeId
        if (next === selectedNodeId) {
            statusText = "A node cannot link to itself"
            return
        }
        var row = nodeModel.get(index)
        var node = parseJson(row.dataJson, {})
        var key = linkKey(outputPin.currentText)
        node[key] = next.length > 0 ? next : null
        nodeModel.setProperty(index, "dataJson", JSON.stringify(node))
        if (key === "next")
            nodeModel.setProperty(index, "nextId", next)
        graphCanvas.requestPaint()
        markGraphChanged()
    }

    function refreshNextSelector() {
        var index = selectedNodeIndex()
        if (index < 0)
            return
        nextNode.currentIndex = indexForNodeId(targetForPin(nodeModel.get(index), outputPin.currentText)) + 1
    }

    function deleteSelectedNode() {
        var index = selectedNodeIndex()
        if (index < 0)
            return
        var removed = selectedNodeId
        nodeModel.remove(index)
        for (var row = 0; row < nodeModel.count; ++row) {
            var candidate = nodeModel.get(row)
            var data = parseJson(candidate.dataJson, {})
            var keys = ["next", "true_next", "false_next", "a_next", "b_next", "then_0", "then_1"]
            for (var keyIndex = 0; keyIndex < keys.length; ++keyIndex) {
                if (data[keys[keyIndex]] === removed)
                    data[keys[keyIndex]] = null
            }
            nodeModel.setProperty(row, "dataJson", JSON.stringify(data))
            nodeModel.setProperty(row, "nextId", data.next || "")
        }
        selectedNodeId = ""
        nodeJsonEditor.text = ""
        graphCanvas.requestPaint()
        markGraphChanged()
    }

    function filterPalette() {
        var needle = paletteSearch.text.trim().toLowerCase()
        paletteModel.clear()
        for (var index = 0; index < graphCatalog.length; ++index) {
            var row = graphCatalog[index]
            var haystack = (row.type + " " + row.category + " " + row.title + " " + row.detail).toLowerCase()
            if (needle.length === 0 || haystack.indexOf(needle) >= 0) {
                paletteModel.append({
                    "nodeType": row.type,
                    "category": row.category,
                    "title": row.title,
                    "detail": row.detail,
                    "defaultsJson": JSON.stringify(row.defaults || {}),
                    "pinsJson": JSON.stringify(row.output_pins || ["exec"])
                })
            }
        }
    }

    ListModel { id: graphListModel }
    ListModel { id: nodeModel }
    ListModel { id: paletteModel }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.loadCatalog(); root.refreshGraphs("", true) }
        function onContentAssetOpenRequested(relativePath, assetType) {
            if (assetType === "VisualGraph")
                root.openGraph(relativePath, false)
        }
        function onAssetsChanged() { root.refreshGraphs(root.currentPath, false) }
    }

    Component.onCompleted: {
        loadCatalog()
        refreshGraphs("", true)
    }

    Shortcut { sequences: [StandardKey.Save]; enabled: root.dirty; onActivated: root.saveGraph() }

    Dialog {
        id: newGraphDialog
        x: Math.max(8, (root.width - width) / 2)
        y: Math.max(8, (root.height - height) / 2)
        width: 360
        title: "New Visual Graph"
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: { graphName.text = "GameplayGraph"; graphName.selectAll(); graphName.forceActiveFocus() }
        onAccepted: {
            var templateName = graphTemplate.currentIndex >= 0 && graphTemplate.currentIndex < graphTemplates.length
                ? graphTemplates[graphTemplate.currentIndex].name : "LogAndMove"
            var path = editorBridge.createVisualGraphTemplate(graphName.text, templateName)
            if (path.length > 0)
                root.refreshGraphs(path, true)
            else
                root.statusText = editorBridge.lastError
        }
        contentItem: Column {
            spacing: 7
            TextField {
                id: graphName
                width: parent.width
                color: Theme.DarkTheme.text
                placeholderText: "GameplayGraph"
                background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 4 }
            }
            ComboBox {
                id: graphTemplate
                width: parent.width
                model: graphTemplates.map(function(template) { return template.name })
            }
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 7

        MfPanelHeader {
            width: parent.width
            title: "Blueprints / Visual Graph"
            detail: currentPath.length > 0 ? currentPath : graphListModel.count + " graphs"
            badge: dirty ? "Dirty" : "Graph"
            badgeColor: dirty ? Theme.DarkTheme.warning : Theme.DarkTheme.info
        }
        Row {
            width: parent.width; height: 30; spacing: 6
            MfButton { width: 82; text: "New Graph"; accent: true; onClicked: newGraphDialog.open() }
            MfButton { width: 66; text: "Save"; accent: dirty; enabled: dirty; onClicked: root.saveGraph() }
            MfButton { width: 72; text: "Validate"; enabled: currentPath.length > 0; onClicked: root.validateGraph() }
            MfButton { width: 68; text: "Revert"; enabled: currentPath.length > 0 && dirty; onClicked: root.openGraph(currentPath, true) }
            Text { width: parent.width - x; height: parent.height; text: statusText; color: Theme.DarkTheme.muted; font.pixelSize: 10; horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
        }

        Row {
            width: parent.width; height: parent.height - y; spacing: 7

            Rectangle {
                width: 175; height: parent.height; radius: 5; color: Theme.DarkTheme.surface; border.color: Theme.DarkTheme.borderSoft
                ListView {
                    anchors.fill: parent; anchors.margins: 5; spacing: 3; clip: true; model: graphListModel
                    delegate: Rectangle {
                        id: graphRow
                        required property string name
                        required property string relativePath
                        width: ListView.view.width; height: 38; radius: 4
                        color: currentPath === graphRow.relativePath ? Theme.DarkTheme.accentSoft : (graphMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent")
                        Text { anchors.fill: parent; anchors.margins: 6; text: graphRow.name; color: Theme.DarkTheme.text; font.pixelSize: 10; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
                        MouseArea { id: graphMouse; anchors.fill: parent; hoverEnabled: true; onClicked: root.openGraph(graphRow.relativePath, false) }
                    }
                }
            }

            Rectangle {
                width: Math.max(280, parent.width - x - inspectorPanel.width - 7)
                height: parent.height; radius: 5; color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.borderSoft; clip: true
                Flickable {
                    anchors.fill: parent; contentWidth: 1500; contentHeight: 900; clip: true
                    Item {
                        width: 1500; height: 900
                        Canvas {
                            id: graphCanvas
                            anchors.fill: parent
                            onPaint: {
                                var context = getContext("2d")
                                context.reset()
                                context.strokeStyle = "#252a33"
                                context.lineWidth = 1
                                for (var x = 0; x < width; x += 32) { context.beginPath(); context.moveTo(x, 0); context.lineTo(x, height); context.stroke() }
                                for (var y = 0; y < height; y += 32) { context.beginPath(); context.moveTo(0, y); context.lineTo(width, y); context.stroke() }
                                context.strokeStyle = "#6cc58f"
                                context.lineWidth = 2
                                for (var index = 0; index < nodeModel.count; ++index) {
                                    var from = nodeModel.get(index)
                                    var pins = root.parseJson(from.pinsJson, ["exec"])
                                    for (var pinIndex = 0; pinIndex < pins.length; ++pinIndex) {
                                        var targetIndex = root.indexForNodeId(root.targetForPin(from, pins[pinIndex]))
                                        if (targetIndex >= 0) {
                                            var to = nodeModel.get(targetIndex)
                                            var pinY = from.nodeY + 26 + pinIndex * 14
                                            context.strokeStyle = pins[pinIndex] === "false" ? "#e56f66" : (pins[pinIndex] === "true" ? "#6cc58f" : "#7db9ff")
                                            context.beginPath()
                                            context.moveTo(from.nodeX + 170, pinY)
                                            context.bezierCurveTo(from.nodeX + 215, pinY, to.nodeX - 45, to.nodeY + 35, to.nodeX, to.nodeY + 35)
                                            context.stroke()
                                        }
                                    }
                                }
                            }
                        }
                        Repeater {
                            model: nodeModel
                            delegate: Rectangle {
                                id: nodeCard
                                required property string nodeId
                                required property string nodeType
                                required property real nodeX
                                required property real nodeY
                                required property string nextId
                                required property string pinsJson
                                x: nodeX; y: nodeY; width: 170; height: 72; radius: 6
                                color: Theme.DarkTheme.surfaceRaised
                                border.color: selectedNodeId === nodeId ? Theme.DarkTheme.accent : Theme.DarkTheme.border
                                border.width: selectedNodeId === nodeId ? 2 : 1
                                Column { anchors.fill: parent; anchors.margins: 7; spacing: 5
                                    Text { width: parent.width; text: nodeCard.nodeType; color: Theme.DarkTheme.text; font.pixelSize: 11; font.bold: true; elide: Text.ElideRight }
                                    Text { width: parent.width; text: nodeCard.nodeId; color: Theme.DarkTheme.muted; font.pixelSize: 9; elide: Text.ElideRight }
                                    Text { width: parent.width; text: root.parseJson(nodeCard.pinsJson, ["exec"]).join(" · "); color: Theme.DarkTheme.accent; font.pixelSize: 8; elide: Text.ElideRight }
                                }
                                MouseArea {
                                    anchors.fill: parent; drag.target: nodeCard; drag.minimumX: 0; drag.minimumY: 0; drag.maximumX: 1330; drag.maximumY: 828
                                    onPressed: root.selectNode(nodeCard.nodeId)
                                    onReleased: {
                                        var row = root.indexForNodeId(nodeCard.nodeId)
                                        if (row >= 0) { nodeModel.setProperty(row, "nodeX", nodeCard.x); nodeModel.setProperty(row, "nodeY", nodeCard.y) }
                                        graphCanvas.requestPaint()
                                        root.markGraphChanged()
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Rectangle {
                id: inspectorPanel
                width: 250; height: parent.height; radius: 5; color: Theme.DarkTheme.surface; border.color: Theme.DarkTheme.borderSoft
                Column {
                    anchors.fill: parent; anchors.margins: 6; spacing: 5
                    MfSearchBar { id: paletteSearch; width: parent.width; placeholderText: "Node palette"; onTextChanged: root.filterPalette() }
                    ListView {
                        width: parent.width; height: Math.min(150, contentHeight); clip: true; spacing: 2; model: paletteModel
                        delegate: Rectangle {
                            id: paletteRow
                            required property string title
                            required property string category
                            width: ListView.view.width; height: 32; radius: 4; color: paletteMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent"
                            Text { anchors.fill: parent; anchors.margins: 5; text: paletteRow.title + " · " + paletteRow.category; color: Theme.DarkTheme.text; font.pixelSize: 9; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
                            MouseArea { id: paletteMouse; anchors.fill: parent; hoverEnabled: true; onDoubleClicked: root.addNode(index) }
                            ToolTip.visible: paletteMouse.containsMouse; ToolTip.text: "Double-click to add"
                        }
                    }
                    Rectangle { width: parent.width; height: 1; color: Theme.DarkTheme.borderSoft }
                    Text { text: selectedNodeId.length > 0 ? "Node: " + selectedNodeId : "Select a node"; color: Theme.DarkTheme.accent; font.pixelSize: 10; font.bold: true }
                    ComboBox {
                        id: outputPin; width: parent.width
                        model: {
                            var selected = root.selectedNodeIndex()
                            return selected >= 0 ? root.parseJson(nodeModel.get(selected).pinsJson, ["exec"]) : []
                        }
                        enabled: selectedNodeId.length > 0
                        onActivated: root.refreshNextSelector()
                    }
                    ComboBox {
                        id: nextNode; width: parent.width
                        model: ["No next link"].concat((function() { var values = []; for (var i = 0; i < nodeModel.count; ++i) values.push(nodeModel.get(i).nodeId); return values })())
                        enabled: selectedNodeId.length > 0
                        onActivated: root.setSelectedNext()
                    }
                    ScrollView {
                        width: parent.width; height: Math.max(80, parent.height - y - 68); clip: true
                        TextArea {
                            id: nodeJsonEditor
                            width: Math.max(parent.width, implicitWidth); height: Math.max(parent.height, contentHeight + 16)
                            textFormat: TextEdit.PlainText; wrapMode: TextEdit.NoWrap; color: Theme.DarkTheme.text
                            font.family: "Menlo"; font.pixelSize: 9; background: Rectangle { color: Theme.DarkTheme.background }
                        }
                    }
                    Row { width: parent.width; height: 28; spacing: 5
                        MfButton { width: (parent.width - 5) / 2; text: "Apply JSON"; enabled: selectedNodeId.length > 0; onClicked: root.applyNodeJson() }
                        MfButton { width: (parent.width - 5) / 2; text: "Delete Node"; enabled: selectedNodeId.length > 0; onClicked: root.deleteSelectedNode() }
                    }
                }
            }
        }
    }
}
