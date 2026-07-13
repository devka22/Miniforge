import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property bool loading: false
    property bool loaded: false
    property bool healthy: true
    property string level: "stable"
    property string summary: "Waiting for runtime telemetry"
    property string mode: "editor"
    property bool safeModeActive: false
    property string safeModeReason: ""
    property var safeModeDisabledSystems: []
    property bool guardEnabled: false
    property real rawDeltaMs: 0
    property real safeDeltaMs: 0
    property bool deltaInvalid: false
    property bool deltaClamped: false
    property int repairedValues: 0
    property int quarantinedEntities: 0
    property int entityCount: 0
    property int maxEntities: 0
    property int entityLimitExceededBy: 0
    property int optionalCadenceDivisor: 1
    property real stabilityScore: 0
    readonly property real stabilityPercent: Math.max(0, Math.min(100, stabilityScore * 100))
    property real fps: 0
    property real averageFrameTimeMs: 0
    property real frameBudgetMs: 0

    function levelColor(value) {
        if (value === "recovery")
            return Theme.DarkTheme.danger
        if (value === "guarded")
            return Theme.DarkTheme.warning
        return Theme.DarkTheme.accent
    }

    function requestRefresh() {
        if (loading)
            return
        if (!editorBridge.projectOpen) {
            loaded = false
            summary = "Open a project to inspect runtime stability"
            warningModel.clear()
            return
        }
        loading = true
        Qt.callLater(root.loadHealth)
    }

    function loadHealth() {
        var json = editorBridge.runtimeHealthJson()
        if (json.length === 0) {
            loaded = false
            healthy = false
            summary = editorBridge.lastError || "Runtime health is unavailable"
            warningModel.clear()
            loading = false
            return
        }
        try {
            var report = JSON.parse(json)
            level = report.level || "stable"
            healthy = report.healthy === true
            summary = report.summary || "Runtime stability report updated"
            mode = report.mode || "editor"
            safeModeActive = report.safe_mode_active === true
            safeModeReason = String(report.safe_mode_reason || "")
            safeModeDisabledSystems = report.safe_mode_disabled_systems || []
            guardEnabled = report.guard_enabled === true
            rawDeltaMs = Number(report.raw_delta_ms || 0)
            safeDeltaMs = Number(report.safe_delta_ms || 0)
            deltaInvalid = report.delta_was_invalid === true
            deltaClamped = report.delta_was_clamped === true
            repairedValues = Number(report.repaired_values || 0)
            quarantinedEntities = Number(report.quarantined_entities || 0)
            entityCount = Number(report.entity_count || 0)
            maxEntities = Number(report.max_entities || 0)
            entityLimitExceededBy = Number(report.entity_limit_exceeded_by || 0)
            optionalCadenceDivisor = Math.max(1, Number(report.optional_cadence_divisor || 1))
            stabilityScore = Number(report.stability_score || 0)
            fps = Number(report.fps || 0)
            averageFrameTimeMs = Number(report.average_frame_time_ms || 0)
            frameBudgetMs = Number(report.frame_budget_ms || 0)
            warningModel.clear()
            var warnings = report.warnings || []
            for (var index = 0; index < warnings.length; ++index)
                warningModel.append({"message": String(warnings[index])})
            loaded = true
        } catch (error) {
            loaded = false
            healthy = false
            summary = "Invalid runtime health report · " + error
            warningModel.clear()
        }
        loading = false
    }

    ListModel {
        id: warningModel
    }

    Connections {
        target: editorBridge
        function onRuntimeHealthChanged() {
            root.requestRefresh()
        }
        function onProjectChanged() {
            root.requestRefresh()
        }
    }

    Timer {
        interval: 1500
        repeat: true
        running: root.visible && editorBridge.projectOpen
        onTriggered: root.requestRefresh()
    }

    Component.onCompleted: requestRefresh()

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Runtime Health"
            detail: root.safeModeActive
                ? "Recovery mode · " + (root.safeModeReason || "manual recovery")
                : root.summary
            badge: root.loading ? "Sampling" : (root.safeModeActive ? "SAFE MODE" : root.level.toUpperCase())
            badgeColor: root.loading ? Theme.DarkTheme.info : (root.safeModeActive ? Theme.DarkTheme.warning : root.levelColor(root.level))
        }

        Row {
            width: parent.width
            height: 30
            spacing: 7

            MfButton {
                width: 82
                text: root.loading ? "Reading…" : "Refresh"
                accent: !root.loaded
                enabled: editorBridge.projectOpen && !root.loading
                onClicked: root.requestRefresh()
            }

            Text {
                width: parent.width - x
                height: parent.height
                text: (root.safeModeActive
                        ? "Disabled: " + root.safeModeDisabledSystems.join(", ") + "  ·  "
                        : "")
                    + (root.guardEnabled ? "Guard enabled" : "Guard disabled")
                    + "  ·  " + root.mode
                    + "  ·  " + root.fps.toFixed(1) + " FPS"
                color: root.safeModeActive || !root.guardEnabled ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
                font.pixelSize: 10
                horizontalAlignment: Text.AlignRight
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }

        Grid {
            id: metricsGrid
            width: parent.width
            columns: width >= 420 ? 3 : 2
            spacing: 6
            height: Math.ceil(6 / columns) * 50 + (Math.ceil(6 / columns) - 1) * spacing

            Repeater {
                model: [
                    ["Stability", root.loaded ? Math.round(root.stabilityPercent) + "%" : "—", root.levelColor(root.level)],
                    ["Delta safe / raw", root.loaded ? root.safeDeltaMs.toFixed(2) + " / " + root.rawDeltaMs.toFixed(2) + " ms" : "—", root.deltaInvalid || root.deltaClamped ? Theme.DarkTheme.warning : Theme.DarkTheme.info],
                    ["Repairs", root.loaded ? String(root.repairedValues) : "—", root.repairedValues > 0 ? Theme.DarkTheme.warning : Theme.DarkTheme.accent],
                    ["Quarantined", root.loaded ? String(root.quarantinedEntities) : "—", root.quarantinedEntities > 0 ? Theme.DarkTheme.danger : Theme.DarkTheme.accent],
                    ["Entities", root.loaded ? root.entityCount + " / " + root.maxEntities : "—", root.entityLimitExceededBy > 0 ? Theme.DarkTheme.danger : Theme.DarkTheme.info],
                    ["Optional cadence", root.loaded ? (root.optionalCadenceDivisor === 1 ? "Every frame" : "Every " + root.optionalCadenceDivisor + " frames") : "—", root.optionalCadenceDivisor > 1 ? Theme.DarkTheme.warning : Theme.DarkTheme.accent]
                ]

                Rectangle {
                    required property var modelData
                    width: (metricsGrid.width - metricsGrid.spacing * (metricsGrid.columns - 1)) / metricsGrid.columns
                    height: 50
                    radius: Theme.DarkTheme.cardRadius
                    color: Theme.DarkTheme.surface
                    border.color: modelData[2]
                    border.width: 1

                    Column {
                        anchors.fill: parent
                        anchors.margins: 7
                        spacing: 3

                        Text {
                            width: parent.width
                            text: modelData[0]
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width
                            text: modelData[1]
                            color: modelData[2]
                            font.pixelSize: 12
                            font.bold: true
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }

        Rectangle {
            width: parent.width
            height: 58
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: root.frameBudgetMs > 0 && root.averageFrameTimeMs > root.frameBudgetMs
                ? Theme.DarkTheme.warning
                : Theme.DarkTheme.borderSoft
            border.width: 1

            Column {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 6

                Text {
                    width: parent.width
                    text: "Frame budget · " + root.averageFrameTimeMs.toFixed(2)
                        + " / " + root.frameBudgetMs.toFixed(2) + " ms"
                    color: Theme.DarkTheme.text
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }

                Rectangle {
                    width: parent.width
                    height: 8
                    radius: 4
                    color: Theme.DarkTheme.background
                    border.color: Theme.DarkTheme.borderSoft
                    border.width: 1

                    Rectangle {
                        width: root.frameBudgetMs > 0
                            ? Math.min(parent.width, parent.width * root.averageFrameTimeMs / root.frameBudgetMs)
                            : 0
                        height: parent.height
                        radius: 4
                        color: root.averageFrameTimeMs > root.frameBudgetMs
                            ? Theme.DarkTheme.warning
                            : Theme.DarkTheme.accent
                    }
                }

                Text {
                    width: parent.width
                    text: root.entityLimitExceededBy > 0
                        ? "Entity limit exceeded by " + root.entityLimitExceededBy
                        : (root.deltaInvalid || root.deltaClamped ? "Frame delta was sanitized for safe simulation" : "Frame pacing inside configured safety limits")
                    color: root.entityLimitExceededBy > 0
                        ? Theme.DarkTheme.danger
                        : (root.deltaInvalid || root.deltaClamped ? Theme.DarkTheme.warning : Theme.DarkTheme.muted)
                    font.pixelSize: 9
                    elide: Text.ElideRight
                }
            }
        }

        ListView {
            id: warningList
            width: parent.width
            height: parent.height - y
            clip: true
            spacing: 5
            model: warningModel

            delegate: Rectangle {
                id: warningRow
                required property string message
                width: ListView.view.width
                height: Math.max(38, warningText.implicitHeight + 16)
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.warningSoft
                border.color: Theme.DarkTheme.warning
                border.width: 1

                Text {
                    id: warningText
                    anchors.fill: parent
                    anchors.margins: 8
                    text: warningRow.message
                    color: Theme.DarkTheme.warning
                    font.pixelSize: 10
                    wrapMode: Text.Wrap
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Text {
                visible: warningList.count === 0
                anchors.centerIn: parent
                width: Math.max(100, parent.width - 24)
                text: root.loaded
                    ? (root.healthy ? "No runtime stability warnings" : "Runtime is guarded; refresh for details")
                    : root.summary
                color: root.loaded && root.healthy ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
                font.pixelSize: 11
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
            }
        }
    }
}
