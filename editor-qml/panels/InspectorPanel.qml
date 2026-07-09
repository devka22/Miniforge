import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    color: Theme.DarkTheme.panel

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Inspector"
            detail: inspectorModel.entityId > 0 ? "Entity #" + inspectorModel.entityId : "No selection"
            badge: inspectorList.count + " fields"
            badgeColor: inspectorModel.entityId > 0 ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
        }

        ListView {
            id: inspectorList
            width: parent.width
            height: parent.height - y
            clip: true
            model: inspectorModel
            section.property: "target"
            section.criteria: ViewSection.FullString
            section.delegate: Rectangle {
                id: inspectorSection
                required property string section

                width: ListView.view.width
                height: 28
                color: Theme.DarkTheme.surface
                Text {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    text: inspectorSection.section
                    color: Theme.DarkTheme.accent
                    font.pixelSize: 12
                    font.bold: true
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }

            delegate: Rectangle {
                id: inspectorField
                required property int index
                required property var model

                width: ListView.view.width
                height: Theme.DarkTheme.rowHeight
                color: inspectorField.index % 2 === 0 ? Theme.DarkTheme.panel : Theme.DarkTheme.surfaceRaised
                MfPropertyRow {
                    anchors.fill: parent
                    label: inspectorField.model.displayName
                    value: inspectorField.model.valueJson
                    valueType: inspectorField.model.valueType
                    editable: inspectorField.model.editable
                    onValueCommitted: function(value) {
                        editorController.setInspectorValue(
                            inspectorField.model.entityId,
                            inspectorField.model.target,
                            inspectorField.model.key,
                            value
                        )
                    }
                }
            }

            Text {
                visible: inspectorList.count === 0
                anchors.centerIn: parent
                text: inspectorModel.entityId > 0 ? "No editable fields" : "No selection"
                color: Theme.DarkTheme.muted
                font.pixelSize: 12
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
