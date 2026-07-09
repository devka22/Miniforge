import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    color: Theme.DarkTheme.panel

    function scoreColor(score) {
        return score >= 85 ? Theme.DarkTheme.accent : (score >= 70 ? Theme.DarkTheme.warning : Theme.DarkTheme.danger)
    }

    function scoreBack(score) {
        return score >= 85 ? Theme.DarkTheme.accentSoft : (score >= 70 ? Theme.DarkTheme.warningSoft : Theme.DarkTheme.dangerSoft)
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        MfPanelHeader {
            width: parent.width
            title: "Readiness"
            detail: readinessList.count + " systems tracked"
            badge: readinessModel.score + "%"
            badgeColor: scoreColor(readinessModel.score)
        }

        MfButton {
            width: parent.width
            text: "Run Audit"
            accent: true
            onClicked: editorController.executeCommand("project.audit")
        }

        Rectangle {
            width: parent.width
            height: 10
            radius: 5
            color: Theme.DarkTheme.background
            border.color: Theme.DarkTheme.borderSoft
            border.width: 1

            Rectangle {
                width: Math.max(0, Math.min(parent.width, parent.width * readinessModel.score / 100))
                height: parent.height
                radius: 5
                color: scoreColor(readinessModel.score)
            }
        }

        ListView {
            id: readinessList
            width: parent.width
            height: parent.height - y
            clip: true
            spacing: 6
            model: readinessModel

            delegate: Rectangle {
                id: row
                required property var model

                width: readinessList.width
                height: 88
                radius: Theme.DarkTheme.cardRadius
                color: mouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.surface
                border.color: scoreColor(row.model.score)
                border.width: 1

                Column {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 4

                    Row {
                        width: parent.width
                        height: 20
                        spacing: 8

                        Text {
                            width: parent.width - scoreLabel.width - 8
                            height: parent.height
                            text: row.model.system
                            color: Theme.DarkTheme.text
                            font.pixelSize: 13
                            font.bold: true
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }

                        Text {
                            id: scoreLabel
                            width: 44
                            height: parent.height
                            text: row.model.score + "%"
                            color: scoreColor(row.model.score)
                            font.pixelSize: 13
                            font.bold: true
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignVCenter
                        }
                    }

                    Rectangle {
                        width: parent.width
                        height: 5
                        radius: 3
                        color: Theme.DarkTheme.background

                        Rectangle {
                            width: Math.max(0, Math.min(parent.width, parent.width * row.model.score / 100))
                            height: parent.height
                            radius: 3
                            color: scoreColor(row.model.score)
                        }
                    }

                    Text {
                        width: parent.width
                        height: 18
                        text: row.model.levelLabel + "  |  " + row.model.gapCount + " gaps  |  " + row.model.strengthCount + " strengths"
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 11
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        height: 28
                        text: row.model.topAction.length > 0 ? row.model.topAction : "Ready"
                        color: Theme.DarkTheme.text
                        font.pixelSize: 12
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }
                }

                MouseArea {
                    id: mouse
                    anchors.fill: parent
                    hoverEnabled: true
                }
            }
        }
    }
}
