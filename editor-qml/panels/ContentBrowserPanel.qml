import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    color: Theme.DarkTheme.panel

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        MfPanelHeader {
            width: parent.width
            title: "Content"
            detail: assetGrid.count + " indexed assets"
            badge: "Assets"
            badgeColor: Theme.DarkTheme.info
        }

        Row {
            width: parent.width
            height: 32
            spacing: 8

            MfSearchBar {
                width: parent.width - refresh.width - 8
                placeholderText: "Search assets"
                onTextChanged: contentModel.filter = text
            }

            MfButton {
                id: refresh
                width: 84
                text: "Refresh"
                accent: true
                onClicked: editorController.executeCommand("assets.refresh")
            }
        }

        GridView {
            id: assetGrid
            width: parent.width
            height: parent.height - y
            clip: true
            model: contentModel
            cellWidth: 176
            cellHeight: 92

            delegate: Rectangle {
                id: assetDelegate
                required property string name
                required property string assetType
                required property string relativePath

                width: 168
                height: 84
                radius: Theme.DarkTheme.cardRadius
                color: mouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.surface
                border.color: mouse.containsMouse ? Theme.DarkTheme.focus : Theme.DarkTheme.borderSoft
                border.width: 1

                Column {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 4

                    Text {
                        width: parent.width
                        text: assetDelegate.name
                        color: Theme.DarkTheme.text
                        font.pixelSize: 13
                        font.bold: true
                        elide: Text.ElideRight
                    }

                    Rectangle {
                        width: parent.width
                        height: 20
                        radius: 10
                        color: Theme.DarkTheme.accentSoft

                        Text {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            text: assetDelegate.assetType
                            color: Theme.DarkTheme.accent
                            font.pixelSize: 11
                            font.bold: true
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        width: parent.width
                        text: assetDelegate.relativePath
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 10
                        elide: Text.ElideMiddle
                    }
                }

                MouseArea {
                    id: mouse
                    anchors.fill: parent
                    hoverEnabled: true
                }
            }

            Text {
                visible: assetGrid.count === 0
                anchors.centerIn: parent
                text: "No assets"
                color: Theme.DarkTheme.muted
                font.pixelSize: 12
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
