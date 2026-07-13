import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property bool loading: false
    property int edgeCount: 0
    property int cycleCount: 0
    property int unresolvedCount: 0
    property string statusText: editorBridge.projectOpen
        ? "Dependency graph ready"
        : "Open a project to inspect dependencies"

    function loadGraph() {
        if (loading || !editorBridge.projectOpen)
            return
        loading = true
        var source = editorBridge.assetDependencyGraphJson()
        if (source.length === 0) {
            statusText = editorBridge.lastError || "Dependency graph unavailable"
            loading = false
            return
        }
        try {
            var graph = JSON.parse(source)
            nodeModel.clear()
            edgeModel.clear()
            cycleModel.clear()
            var nodes = graph.nodes || []
            var edges = graph.edges || []
            var cycles = graph.cycles || []
            for (var nodeIndex = 0; nodeIndex < nodes.length; ++nodeIndex) {
                var node = nodes[nodeIndex]
                nodeModel.append({
                    "path": String(node.path),
                    "typeName": String(node.asset_type),
                    "dependencies": Number(node.dependency_count || 0),
                    "consumers": Number(node.reverse_dependency_count || 0),
                    "bytes": Number(node.size_bytes || 0)
                })
            }
            for (var edgeIndex = 0; edgeIndex < edges.length; ++edgeIndex) {
                var edge = edges[edgeIndex]
                edgeModel.append({
                    "dependency": String(edge.dependency),
                    "consumer": String(edge.consumer),
                    "resolved": edge.resolved === true
                })
            }
            for (var cycleIndex = 0; cycleIndex < cycles.length; ++cycleIndex)
                cycleModel.append({"description": cycles[cycleIndex].join("  →  ")})
            edgeCount = Number(graph.edge_count || 0)
            cycleCount = cycles.length
            unresolvedCount = (graph.unresolved_dependencies || []).length
            statusText = nodes.length + " nodes · " + edgeCount + " resolved edges"
        } catch (error) {
            statusText = "Invalid dependency graph · " + error
        }
        loading = false
    }

    function rebuildGraph() {
        if (!editorBridge.rebuildAssetDependencies()) {
            statusText = editorBridge.lastError
            return
        }
        loadGraph()
    }

    ListModel { id: nodeModel }
    ListModel { id: edgeModel }
    ListModel { id: cycleModel }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.loadGraph() }
        function onAssetsChanged() { root.loadGraph() }
    }

    Component.onCompleted: loadGraph()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Asset Dependency Graph"
            detail: root.statusText
            badge: root.cycleCount > 0 ? root.cycleCount + " cycles" : "Acyclic"
            badgeColor: root.cycleCount > 0 ? Theme.DarkTheme.danger : Theme.DarkTheme.accent
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                text: root.edgeCount + " edges · " + root.unresolvedCount + " unresolved"
                color: root.unresolvedCount > 0 ? Theme.DarkTheme.warning : Theme.DarkTheme.muted
            }
            MfButton { text: "Refresh"; onClicked: root.loadGraph() }
            MfButton { text: "Rebuild"; accent: true; enabled: !root.loading; onClicked: root.rebuildGraph() }
        }

        TabBar {
            id: tabs
            Layout.fillWidth: true
            TabButton { text: "Assets (" + nodeModel.count + ")" }
            TabButton { text: "Edges (" + edgeModel.count + ")" }
            TabButton { text: "Cycles (" + cycleModel.count + ")" }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabs.currentIndex

            ListView {
                clip: true
                spacing: 4
                model: nodeModel
                delegate: Rectangle {
                    required property string path
                    required property string typeName
                    required property int dependencies
                    required property int consumers
                    width: ListView.view.width
                    height: 46
                    color: Theme.DarkTheme.surface
                    radius: 4
                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 7
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Label { Layout.fillWidth: true; text: path; color: Theme.DarkTheme.text; elide: Text.ElideMiddle }
                            Label { text: typeName; color: Theme.DarkTheme.muted; font.pixelSize: 9 }
                        }
                        Label { text: dependencies + " deps"; color: Theme.DarkTheme.info }
                        Label { text: consumers + " users"; color: Theme.DarkTheme.accent }
                    }
                }
            }

            ListView {
                clip: true
                spacing: 4
                model: edgeModel
                delegate: Rectangle {
                    required property string dependency
                    required property string consumer
                    required property bool resolved
                    width: ListView.view.width
                    height: 42
                    color: Theme.DarkTheme.surface
                    radius: 4
                    border.color: resolved ? Theme.DarkTheme.border : Theme.DarkTheme.warning
                    Label {
                        anchors.fill: parent
                        anchors.margins: 7
                        text: dependency + "  →  " + consumer
                        color: resolved ? Theme.DarkTheme.text : Theme.DarkTheme.warning
                        elide: Text.ElideMiddle
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }

            ListView {
                clip: true
                spacing: 4
                model: cycleModel
                delegate: Rectangle {
                    required property string description
                    width: ListView.view.width
                    height: 44
                    color: Theme.DarkTheme.surface
                    radius: 4
                    border.color: Theme.DarkTheme.danger
                    Label {
                        anchors.fill: parent
                        anchors.margins: 7
                        text: description
                        color: Theme.DarkTheme.danger
                        elide: Text.ElideMiddle
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }
    }
}
