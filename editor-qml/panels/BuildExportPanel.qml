import QtQuick
import QtQuick.Controls
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property bool running: false
    property bool hasReport: false
    property bool failed: false
    property string statusText: "Choose a profile and export runtime data"
    property string outputPath: ""
    property string manifestPath: ""
    property string reportProfile: ""
    property int copiedFiles: 0
    property int usedAssets: 0
    property int missingAssets: 0
    property int warningCount: 0
    property int readinessScore: 0
    property string detailsText: ""

    function resetReport() {
        running = false
        hasReport = false
        failed = false
        outputPath = ""
        manifestPath = ""
        reportProfile = ""
        copiedFiles = 0
        usedAssets = 0
        missingAssets = 0
        warningCount = 0
        readinessScore = 0
        detailsText = ""
        statusText = editorBridge.projectOpen
            ? "Choose a profile and export runtime data"
            : "Open a project to export a runtime build"
    }

    function runExport() {
        running = true
        hasReport = false
        failed = false
        statusText = "Validating project and exporting…"
        Qt.callLater(function() {
            var json = editorBridge.exportRuntime(profileBox.currentText)
            if (json.length === 0) {
                statusText = editorBridge.lastError
                failed = true
                running = false
                return
            }
            try {
                var report = JSON.parse(json)
                outputPath = report.output_path || ""
                manifestPath = report.manifest_path || ""
                reportProfile = report.profile || profileBox.currentText
                copiedFiles = report.copied_files || 0
                usedAssets = report.used_assets ? report.used_assets.length : 0
                missingAssets = report.missing_assets ? report.missing_assets.length : 0
                warningCount = report.validation_warnings ? report.validation_warnings.length : 0
                readinessScore = report.readiness_score || 0
                var lines = []
                if (report.missing_assets && report.missing_assets.length > 0) {
                    lines.push("Missing assets:")
                    for (var missing = 0; missing < report.missing_assets.length; ++missing)
                        lines.push("  • " + report.missing_assets[missing])
                }
                if (report.validation_warnings && report.validation_warnings.length > 0) {
                    lines.push("Validation warnings:")
                    for (var warning = 0; warning < report.validation_warnings.length; ++warning)
                        lines.push("  • " + report.validation_warnings[warning])
                }
                if (report.readiness_actions && report.readiness_actions.length > 0) {
                    lines.push("Recommended next actions:")
                    for (var action = 0; action < report.readiness_actions.length; ++action)
                        lines.push("  • " + report.readiness_actions[action])
                }
                detailsText = lines.length > 0 ? lines.join("\n") : "No missing assets or validation warnings."
                statusText = "Export completed successfully"
                hasReport = true
                failed = false
            } catch (error) {
                statusText = "Invalid export report: " + error
                failed = true
            }
            running = false
        })
    }

    Connections {
        target: editorBridge
        function onProjectChanged() {
            root.resetReport()
        }
    }

    Component.onCompleted: resetReport()

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 9

        MfPanelHeader {
            width: parent.width
            title: "Build & Export"
            detail: root.statusText
            badge: root.running ? "Working" : (root.failed ? "Failed" : (root.hasReport ? "Complete" : "Runtime"))
            badgeColor: root.running
                ? Theme.DarkTheme.warning
                : (root.failed ? Theme.DarkTheme.danger : (root.hasReport ? Theme.DarkTheme.accent : Theme.DarkTheme.info))
        }

        Row {
            width: parent.width
            height: 34
            spacing: 8

            ComboBox {
                id: profileBox
                width: 126
                height: 32
                model: ["debug", "release", "shipping"]
                enabled: !root.running && editorBridge.projectOpen

                contentItem: Text {
                    leftPadding: 9
                    text: profileBox.displayText
                    color: Theme.DarkTheme.text
                    font.pixelSize: 12
                    verticalAlignment: Text.AlignVCenter
                }

                background: Rectangle {
                    radius: Theme.DarkTheme.radius
                    color: Theme.DarkTheme.background
                    border.color: profileBox.activeFocus ? Theme.DarkTheme.focus : Theme.DarkTheme.border
                    border.width: 1
                }
            }

            MfButton {
                width: 118
                text: root.running ? "Exporting…" : "Export Runtime"
                accent: true
                enabled: !root.running && editorBridge.projectOpen
                onClicked: root.runExport()
            }

            BusyIndicator {
                width: 28
                height: 28
                running: root.running
                visible: running
            }

            Text {
                width: parent.width - x
                height: parent.height
                text: "Validates → copies runtime project → writes manifest"
                color: Theme.DarkTheme.muted
                font.pixelSize: 10
                horizontalAlignment: Text.AlignRight
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }

        Grid {
            id: metricsGrid
            width: parent.width
            height: columns === 6 ? 58 : 116
            columns: width >= 620 ? 6 : 3
            spacing: 7

            Repeater {
                model: [
                    ["Profile", root.hasReport ? root.reportProfile : "—"],
                    ["Files", root.hasReport ? String(root.copiedFiles) : "—"],
                    ["Assets", root.hasReport ? String(root.usedAssets) : "—"],
                    ["Missing", root.hasReport ? String(root.missingAssets) : "—"],
                    ["Warnings", root.hasReport ? String(root.warningCount) : "—"],
                    ["Readiness", root.hasReport ? root.readinessScore + "%" : "—"]
                ]

                Rectangle {
                    required property var modelData
                    width: (metricsGrid.width - metricsGrid.spacing * (metricsGrid.columns - 1)) / metricsGrid.columns
                    height: 54
                    radius: Theme.DarkTheme.cardRadius
                    color: Theme.DarkTheme.surfaceRaised
                    border.color: root.hasReport && modelData[0] === "Missing" && root.missingAssets > 0
                        ? Theme.DarkTheme.danger
                        : (root.hasReport && modelData[0] === "Warnings" && root.warningCount > 0
                            ? Theme.DarkTheme.warning
                            : Theme.DarkTheme.borderSoft)
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
                            color: Theme.DarkTheme.text
                            font.pixelSize: 14
                            font.bold: true
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }

        Rectangle {
            width: parent.width
            height: 62
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft
            border.width: 1

            Column {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 4

                Text {
                    width: parent.width
                    text: "Output · " + (root.outputPath || "Not exported yet")
                    color: Theme.DarkTheme.text
                    font.pixelSize: 11
                    elide: Text.ElideMiddle
                }

                Text {
                    width: parent.width
                    text: "Manifest · " + (root.manifestPath || "runtime_manifest.json will be generated")
                    color: Theme.DarkTheme.muted
                    font.pixelSize: 10
                    elide: Text.ElideMiddle
                }
            }
        }

        ScrollView {
            width: parent.width
            height: parent.height - y
            clip: true

            TextArea {
                width: parent.width
                readOnly: true
                text: root.detailsText.length > 0
                    ? root.detailsText
                    : "The export report will show missing dependencies, project warnings and readiness actions here."
                color: Theme.DarkTheme.text
                font.family: "Menlo"
                font.pixelSize: 10
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                background: Rectangle {
                    color: Theme.DarkTheme.background
                }
            }
        }
    }
}
