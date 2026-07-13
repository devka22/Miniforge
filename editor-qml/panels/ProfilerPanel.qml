import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property bool loading: false
    property real frameTime: 0
    property real frameBudget: 16.67
    property real fps: 0
    property real systemsTotal: 0
    property real budgetUsage: 0
    property bool overBudget: false
    property string slowest: "—"
    property string statusText: "Waiting for runtime telemetry"

    function refreshProfiler() {
        if (loading || !editorBridge.projectOpen)
            return
        loading = true
        var source = editorBridge.profilerSnapshotJson()
        if (source.length === 0) {
            statusText = editorBridge.lastError || "Profiler data unavailable"
            loading = false
            return
        }
        try {
            var snapshot = JSON.parse(source)
            frameTime = Number(snapshot.frame_time_ms || 0)
            frameBudget = Number(snapshot.frame_budget_ms || 16.67)
            fps = Number(snapshot.fps || 0)
            systemsTotal = Number(snapshot.systems_total_ms || 0)
            budgetUsage = Number(snapshot.budget_usage_percent || 0)
            overBudget = snapshot.over_budget === true
            slowest = snapshot.slowest_system || "—"
            systemModel.clear()
            var systems = snapshot.systems || []
            for (var index = 0; index < systems.length; ++index) {
                var system = systems[index]
                systemModel.append({
                    "name": String(system.name),
                    "milliseconds": Number(system.milliseconds || 0),
                    "percent": Number(system.frame_percent || 0),
                    "over": system.over_frame_budget === true
                })
            }
            statusText = systems.length + " systems sampled · real runtime counters"
        } catch (error) {
            statusText = "Invalid profiler snapshot · " + error
            systemModel.clear()
        }
        loading = false
    }

    ListModel { id: systemModel }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.refreshProfiler() }
    }

    Timer {
        interval: 750
        repeat: true
        running: root.visible && editorBridge.projectOpen
        onTriggered: root.refreshProfiler()
    }

    Component.onCompleted: refreshProfiler()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Profiler"
            detail: root.statusText
            badge: root.loading ? "Sampling" : (root.overBudget ? "Over budget" : "On budget")
            badgeColor: root.overBudget ? Theme.DarkTheme.danger : Theme.DarkTheme.accent
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 4
            columnSpacing: 6

            Repeater {
                model: [
                    ["Frame", root.frameTime.toFixed(2) + " ms"],
                    ["Budget", root.frameBudget.toFixed(2) + " ms"],
                    ["FPS", root.fps.toFixed(1)],
                    ["Slowest", root.slowest]
                ]
                Rectangle {
                    required property var modelData
                    Layout.fillWidth: true
                    implicitHeight: 54
                    radius: Theme.DarkTheme.cardRadius
                    color: Theme.DarkTheme.surface
                    border.color: Theme.DarkTheme.border
                    Column {
                        anchors.fill: parent
                        anchors.margins: 7
                        spacing: 3
                        Label { text: modelData[0]; color: Theme.DarkTheme.muted; font.pixelSize: 9 }
                        Label { text: modelData[1]; color: Theme.DarkTheme.text; font.bold: true; elide: Text.ElideRight; width: parent.width }
                    }
                }
            }
        }

        ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: Math.max(150, root.budgetUsage)
            value: root.budgetUsage
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                text: "Systems " + root.systemsTotal.toFixed(2) + " ms · Budget " + root.budgetUsage.toFixed(0) + "%"
                color: root.overBudget ? Theme.DarkTheme.danger : Theme.DarkTheme.muted
            }
            MfButton { text: "Refresh"; onClicked: root.refreshProfiler() }
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 4
            model: systemModel
            delegate: Rectangle {
                required property string name
                required property real milliseconds
                required property real percent
                required property bool over
                width: ListView.view.width
                height: 42
                radius: 4
                color: Theme.DarkTheme.surface
                border.color: over ? Theme.DarkTheme.danger : Theme.DarkTheme.border
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 7
                    Label { Layout.fillWidth: true; text: name; color: Theme.DarkTheme.text; font.bold: true }
                    Label { text: percent.toFixed(1) + "%"; color: Theme.DarkTheme.muted }
                    Label { text: milliseconds.toFixed(2) + " ms"; color: over ? Theme.DarkTheme.danger : Theme.DarkTheme.info }
                }
            }
        }
    }
}
