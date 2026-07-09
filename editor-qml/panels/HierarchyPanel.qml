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
            title: "Hierarchy"
            detail: tree.rows + " visible entities"
            badge: "Scene"
            badgeColor: Theme.DarkTheme.accent
        }

        MfSearchBar {
            width: parent.width
            placeholderText: "Search entities"
            onTextChanged: hierarchyModel.filter = text
        }

        TreeView {
            id: tree
            width: parent.width
            height: parent.height - y
            clip: true
            model: hierarchyModel

            delegate: Rectangle {
                id: entityDelegate
                required property TreeView treeView
                required property bool isTreeNode
                required property int depth
                required property bool expanded
                required property bool hasChildren
                required property int row
                required property var model

                implicitWidth: entityDelegate.treeView.width
                implicitHeight: Theme.DarkTheme.rowHeight
                width: entityDelegate.treeView.width
                height: Theme.DarkTheme.rowHeight
                color: entityDelegate.model.selected ? Qt.rgba(0.42, 0.77, 0.56, 0.18) : (mouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.panel)
                border.color: entityDelegate.model.selected ? Theme.DarkTheme.accent : "transparent"
                border.width: entityDelegate.model.selected ? 1 : 0

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 8 + entityDelegate.depth * 18
                    anchors.rightMargin: 8
                    spacing: 8

                    Text {
                        width: 18
                        height: parent.height
                        text: entityDelegate.isTreeNode && entityDelegate.hasChildren ? (entityDelegate.expanded ? "▾" : "▸") : ""
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 12
                        verticalAlignment: Text.AlignVCenter
                    }

                    Text {
                        width: Math.max(80, parent.width - 180)
                        height: parent.height
                        text: entityDelegate.model.name
                        color: entityDelegate.model.locked ? Theme.DarkTheme.muted : Theme.DarkTheme.text
                        font.pixelSize: 13
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        width: 74
                        height: parent.height
                        text: entityDelegate.model.layer
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 11
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        width: 56
                        height: parent.height
                        text: entityDelegate.model.componentCount
                        color: Theme.DarkTheme.warning
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                MouseArea {
                    id: mouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: function(event) {
                        if (entityDelegate.isTreeNode && entityDelegate.hasChildren && event.x <= 32 + entityDelegate.depth * 18)
                            entityDelegate.treeView.toggleExpanded(entityDelegate.row)
                        editorController.selectEntity(entityDelegate.model.entityId)
                    }
                }
            }

            Text {
                visible: tree.rows === 0
                anchors.centerIn: parent
                text: "No entities"
                color: Theme.DarkTheme.muted
                font.pixelSize: 12
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
