import QtQuick
import QtQuick.Controls
import "../themes" as Theme

Item {
    id: root
    property string label: ""
    property string value: ""
    property string valueType: ""
    property bool editable: false
    property bool mixed: false
    property bool mixedEditing: false
    property bool editorValid: true
    readonly property bool booleanEditor: valueType === "bool" || valueType === "boolean"
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
        if (!valueEditor.activeFocus) {
            valueEditor.text = root.mixed && !root.mixedEditing ? "— Mixed —" : displayValue(root.value, root.valueType)
            editorValid = true
        }
    }

    onValueChanged: {
        syncEditor()
    }

    onValueTypeChanged: syncEditor()
    onMixedChanged: {
        mixedEditing = false
        syncEditor()
    }
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
            text: root.mixed ? "— Mixed —" : root.displayValue(root.value, root.valueType)
            color: root.editable ? Theme.DarkTheme.text : Theme.DarkTheme.muted
            font.pixelSize: 12
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        Rectangle {
            visible: root.editable && !root.booleanEditor
            width: parent.width - x
            height: Math.max(22, parent.height - 6)
            anchors.verticalCenter: parent.verticalCenter
            color: valueEditor.activeFocus ? Theme.DarkTheme.background : Theme.DarkTheme.panel
            border.color: !root.editorValid
                ? Theme.DarkTheme.danger
                : (valueEditor.activeFocus ? Theme.DarkTheme.accent : Theme.DarkTheme.border)
            border.width: 1
            radius: 3

            TextInput {
                id: valueEditor
                anchors.fill: parent
                anchors.leftMargin: 6
                anchors.rightMargin: 6
                text: root.mixed && !root.mixedEditing ? "— Mixed —" : root.displayValue(root.value, root.valueType)
                color: Theme.DarkTheme.text
                selectionColor: Theme.DarkTheme.accent
                selectedTextColor: Theme.DarkTheme.background
                font.pixelSize: 12
                verticalAlignment: TextInput.AlignVCenter
                clip: true
                selectByMouse: true
                onTextEdited: {
                    root.mixedEditing = true
                    var numeric = root.valueType === "number" || root.valueType === "float" || root.valueType === "integer"
                    root.editorValid = !numeric || (text.trim().length > 0 && !isNaN(Number(text)))
                }
                onEditingFinished: {
                    if (!root.editorValid) {
                        text = root.displayValue(root.value, root.valueType)
                        root.editorValid = true
                        return
                    }
                    var committed = root.commitValue(text, root.valueType)
                    if (root.mixed || committed !== root.value)
                        root.valueCommitted(committed)
                }
                Keys.onEscapePressed: {
                    text = root.displayValue(root.value, root.valueType)
                    root.editorValid = true
                    focus = false
                }
            }
        }

        Switch {
            id: boolEditor
            visible: root.editable && root.booleanEditor
            width: parent.width - x
            height: parent.height
            checked: !root.mixed && (root.value === "true" || root.value === "1")
            text: root.mixed ? "Mixed" : (checked ? "Enabled" : "Disabled")
            onClicked: root.valueCommitted(checked ? "true" : "false")

            contentItem: Text {
                leftPadding: boolEditor.indicator.width + boolEditor.spacing
                text: boolEditor.text
                color: boolEditor.checked ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
                font.pixelSize: 11
                verticalAlignment: Text.AlignVCenter
            }

            indicator: Rectangle {
                x: 0
                y: (parent.height - height) / 2
                width: 34
                height: 18
                radius: 9
                color: boolEditor.checked ? Theme.DarkTheme.accentSoft : Theme.DarkTheme.background
                border.color: boolEditor.activeFocus ? Theme.DarkTheme.focus : (boolEditor.checked ? Theme.DarkTheme.accent : Theme.DarkTheme.border)
                border.width: 1

                Rectangle {
                    width: 12
                    height: 12
                    radius: 6
                    x: boolEditor.checked ? parent.width - width - 3 : 3
                    anchors.verticalCenter: parent.verticalCenter
                    color: boolEditor.checked ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
                }
            }
        }
    }
}
