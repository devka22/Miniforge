import QtQuick
import QtQuick.Controls
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel
    property bool scanning: false
    property bool testing: false

    function runDoctor() {
        if (scanning || !editorBridge.projectOpen)
            return
        scanning = true
        Qt.callLater(function() {
            forgeAiModel.runDoctor()
            scanning = false
        })
    }

    function runSmokeTest() {
        if (testing || !editorBridge.projectOpen)
            return
        testing = true
        Qt.callLater(function() {
            forgeAiModel.runEnemySmoke()
            testing = false
        })
    }

    function severityColor(severity) {
        if (severity === "Critical" || severity === "Error")
            return Theme.DarkTheme.danger
        if (severity === "Warning")
            return Theme.DarkTheme.warning
        return Theme.DarkTheme.info
    }

    function testColor(status) {
        if (status === "Passed")
            return Theme.DarkTheme.accent
        if (status === "Failed" || status === "Error")
            return Theme.DarkTheme.danger
        return Theme.DarkTheme.borderSoft
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Forge AI"
            detail: forgeAiModel.scanSummary
            badge: root.scanning || root.testing
                ? "Working"
                : (forgeAiModel.criticalCount + forgeAiModel.errorCount > 0 ? "Action" : "Doctor")
            badgeColor: forgeAiModel.criticalCount + forgeAiModel.errorCount > 0
                ? Theme.DarkTheme.danger
                : (root.scanning || root.testing ? Theme.DarkTheme.warning : Theme.DarkTheme.accent)
        }

        Row {
            width: parent.width
            height: 32
            spacing: 8

            MfButton {
                width: 116
                text: root.scanning ? "Scanning…" : "Run Doctor"
                accent: true
                enabled: editorBridge.projectOpen && !root.scanning && !root.testing
                onClicked: root.runDoctor()
            }

            MfButton {
                width: 138
                text: root.testing ? "Testing…" : "Enemy Smoke"
                enabled: editorBridge.projectOpen && !root.scanning && !root.testing
                onClicked: root.runSmokeTest()
            }

            BusyIndicator {
                width: 26
                height: 26
                running: root.scanning || root.testing
                visible: running
            }

            Text {
                width: parent.width - x
                height: parent.height
                text: forgeAiModel.diagnosticCount + " findings"
                color: Theme.DarkTheme.muted
                font.pixelSize: 11
                horizontalAlignment: Text.AlignRight
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }

        Row {
            width: parent.width
            height: 36
            spacing: 6

            Repeater {
                model: [
                    ["Blocking", forgeAiModel.criticalCount + forgeAiModel.errorCount, Theme.DarkTheme.danger],
                    ["Warnings", forgeAiModel.warningCount, Theme.DarkTheme.warning],
                    ["Suggestions", forgeAiModel.suggestionCount, Theme.DarkTheme.info]
                ]

                Rectangle {
                    required property var modelData
                    width: (parent.width - 12) / 3
                    height: parent.height
                    radius: Theme.DarkTheme.cardRadius
                    color: Theme.DarkTheme.surface
                    border.color: modelData[2]
                    border.width: 1

                    Text {
                        anchors.centerIn: parent
                        text: modelData[0] + " · " + modelData[1]
                        color: modelData[2]
                        font.pixelSize: 10
                        font.bold: true
                    }
                }
            }
        }

        Rectangle {
            width: parent.width
            height: 48
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: root.testColor(forgeAiModel.testStatus)
            border.width: 1

            Column {
                anchors.fill: parent
                anchors.margins: 7
                spacing: 2

                Text {
                    width: parent.width
                    text: "NPC / Enemy validation · " + forgeAiModel.testStatus
                    color: forgeAiModel.testStatus === "NotRun"
                        ? Theme.DarkTheme.text
                        : root.testColor(forgeAiModel.testStatus)
                    font.pixelSize: 12
                    font.bold: true
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: forgeAiModel.testSummary
                    color: Theme.DarkTheme.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }
        }

        ListView {
            id: diagnosticList
            width: parent.width
            height: parent.height - y
            clip: true
            spacing: 6
            model: forgeAiModel

            delegate: Rectangle {
                id: diagnosticCard
                required property var model

                width: ListView.view.width
                height: Math.max(
                    82,
                    messageText.implicitHeight
                        + (evidenceText.visible ? evidenceText.implicitHeight + 4 : 0)
                        + fixText.implicitHeight
                        + 46
                )
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surfaceRaised
                border.color: root.severityColor(diagnosticCard.model.severity)
                border.width: 1

                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: 4
                    radius: 2
                    color: root.severityColor(diagnosticCard.model.severity)
                }

                Column {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 9
                    anchors.topMargin: 7
                    anchors.bottomMargin: 7
                    spacing: 4

                    Row {
                        width: parent.width
                        height: 17
                        spacing: 7

                        Text {
                            width: 62
                            height: parent.height
                            text: diagnosticCard.model.severity
                            color: root.severityColor(diagnosticCard.model.severity)
                            font.pixelSize: 10
                            font.bold: true
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width - x
                            height: parent.height
                            text: diagnosticCard.model.code
                            color: Theme.DarkTheme.muted
                            font.family: "Menlo"
                            font.pixelSize: 10
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        id: messageText
                        width: parent.width
                        text: diagnosticCard.model.message
                        color: Theme.DarkTheme.text
                        font.pixelSize: 12
                        wrapMode: Text.Wrap
                    }

                    Text {
                        id: evidenceText
                        visible: diagnosticCard.model.evidence.length > 0
                        width: parent.width
                        text: "Evidence: " + diagnosticCard.model.evidence
                        color: Theme.DarkTheme.muted
                        font.family: "Menlo"
                        font.pixelSize: 9
                        wrapMode: Text.Wrap
                    }

                    Text {
                        id: fixText
                        width: parent.width
                        text: "Fix: " + diagnosticCard.model.proposedFix
                        color: Theme.DarkTheme.accent
                        font.pixelSize: 10
                        wrapMode: Text.Wrap
                    }
                }
            }

            Text {
                visible: diagnosticList.count === 0
                anchors.centerIn: parent
                width: Math.max(120, parent.width - 32)
                text: editorBridge.projectOpen
                    ? "Run Project Doctor to inspect scene, assets, physics and project validation."
                    : "Open a project to use Forge AI diagnostics."
                color: Theme.DarkTheme.muted
                font.pixelSize: 11
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
            }
        }
    }
}
