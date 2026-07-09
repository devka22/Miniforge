import QtQuick
import "../themes" as Theme

Item {
    id: root
    property string label: ""
    property string value: ""
    property string valueType: ""
    property bool editable: false
    signal valueCommitted(string value)

    implicitHeight: Theme.DarkTheme.rowHeight

    function displayValue(raw, typeName) {
        if (typeName === "string") {
            try {
                return JSON.parse(raw)
            } catch (error) {
                return raw
            }
        }
        return raw
    }

    function commitValue(raw, typeName) {
        if (typeName === "string")
            return JSON.stringify(raw)
        if (typeName === "bool") {
            var normalized = raw.trim().toLowerCase()
            return normalized === "true" || normalized === "1" || normalized === "yes" ? "true" : "false"
        }
        if (typeName === "number" || typeName === "float" || typeName === "integer") {
            var numeric = Number(raw)
            return isNaN(numeric) ? root.value : String(numeric)
        }
        return raw
    }

    function syncEditor() {
        if (!valueEditor.activeFocus)
            valueEditor.text = displayValue(root.value, root.valueType)
    }

    onValueChanged: {
        syncEditor()
    }

    onValueTypeChanged: syncEditor()
    Component.onCompleted: syncEditor()

    Row {
        anchors.fill: parent
        anchors.leftMargin: 8
        anchors.rightMargin: 8
        spacing: 8

        Text {
            width: Math.max(100, parent.width * 0.42)
            height: parent.height
            text: root.label
            color: Theme.DarkTheme.muted
            font.pixelSize: 12
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        Text {
            visible: !root.editable
            width: parent.width - x
            height: parent.height
            text: root.displayValue(root.value, root.valueType)
            color: root.editable ? Theme.DarkTheme.text : Theme.DarkTheme.muted
            font.pixelSize: 12
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        Rectangle {
            visible: root.editable
            width: parent.width - x
            height: Math.max(22, parent.height - 6)
            anchors.verticalCenter: parent.verticalCenter
            color: valueEditor.activeFocus ? Theme.DarkTheme.background : Theme.DarkTheme.panel
            border.color: valueEditor.activeFocus ? Theme.DarkTheme.accent : Theme.DarkTheme.border
            border.width: 1
            radius: 3

            TextInput {
                id: valueEditor
                anchors.fill: parent
                anchors.leftMargin: 6
                anchors.rightMargin: 6
                text: root.displayValue(root.value, root.valueType)
                color: Theme.DarkTheme.text
                selectionColor: Theme.DarkTheme.accent
                selectedTextColor: Theme.DarkTheme.background
                font.pixelSize: 12
                verticalAlignment: TextInput.AlignVCenter
                clip: true
                selectByMouse: true
                onEditingFinished: {
                    var committed = root.commitValue(text, root.valueType)
                    if (committed !== root.value)
                        root.valueCommitted(committed)
                }
            }
        }
    }
}
