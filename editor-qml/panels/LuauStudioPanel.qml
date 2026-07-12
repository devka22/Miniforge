import QtQuick
import QtQuick.Controls
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property string currentPath: ""
    property string savedSource: ""
    property string diagnostic: "Select a Luau script"
    property bool sourceValid: true
    readonly property bool dirty: currentPath.length > 0 && codeEditor.text !== savedSource

    function parseJson(value, fallback) {
        try {
            return JSON.parse(value)
        } catch (error) {
            diagnostic = "Bridge JSON error: " + error
            sourceValid = false
            return fallback
        }
    }

    function scriptIndex(path) {
        for (var index = 0; index < scriptModel.count; ++index) {
            if (scriptModel.get(index).relativePath === path)
                return index
        }
        return -1
    }

    function openScript(path, force) {
        if (!force && dirty && path !== currentPath) {
            diagnostic = "Save or revert " + currentPath + " before switching files"
            sourceValid = false
            return
        }
        var source = editorBridge.readLuauScript(path)
        if (source.length === 0 && editorBridge.lastError.length > 0) {
            diagnostic = editorBridge.lastError
            sourceValid = false
            return
        }
        currentPath = path
        savedSource = source
        codeEditor.text = source
        diagnostic = "Loaded " + path
        sourceValid = true
        codeEditor.forceActiveFocus()
    }

    function refreshScripts(preferredPath, reloadCurrent) {
        var rows = parseJson(editorBridge.luauScriptsJson(), [])
        scriptModel.clear()
        for (var index = 0; index < rows.length; ++index) {
            var row = rows[index]
            scriptModel.append({
                "relativePath": row.relative_path,
                "name": row.name,
                "bytes": row.bytes,
                "valid": row.valid,
                "diagnostic": row.diagnostic || ""
            })
        }

        var target = preferredPath || currentPath
        if (target.length === 0 && scriptModel.count > 0)
            target = scriptModel.get(0).relativePath
        if (target.length > 0 && scriptIndex(target) < 0) {
            currentPath = ""
            savedSource = ""
            codeEditor.text = ""
            target = scriptModel.count > 0 ? scriptModel.get(0).relativePath : ""
        }
        if (target.length > 0 && (currentPath.length === 0 || reloadCurrent))
            openScript(target, true)
        else if (scriptModel.count === 0) {
            diagnostic = "No .luau or .lua files under scripts/"
            sourceValid = true
        }
    }

    function validateCurrent() {
        if (currentPath.length === 0)
            return false
        var result = parseJson(
            editorBridge.validateLuauSource(currentPath, codeEditor.text),
            {"valid": false, "diagnostic": editorBridge.lastError}
        )
        sourceValid = result.valid === true
        diagnostic = sourceValid ? "Luau validation passed" : (result.diagnostic || "Luau validation failed")
        return sourceValid
    }

    function saveCurrent() {
        if (!validateCurrent())
            return
        if (editorBridge.saveLuauScript(currentPath, codeEditor.text)) {
            savedSource = codeEditor.text
            diagnostic = "Saved atomically · recovery backups enabled"
            sourceValid = true
            refreshScripts(currentPath, false)
        } else {
            diagnostic = editorBridge.lastError
            sourceValid = false
        }
    }

    ListModel {
        id: scriptModel
    }

    Connections {
        target: editorBridge
        function onLuauScriptsChanged() {
            root.refreshScripts(root.currentPath, false)
        }
    }

    Component.onCompleted: refreshScripts("", true)

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        MfPanelHeader {
            width: parent.width
            title: "Luau Studio"
            detail: currentPath.length > 0
                ? currentPath + (root.dirty ? " · unsaved" : " · saved")
                : scriptModel.count + " project scripts"
            badge: root.dirty ? "Dirty" : (root.sourceValid ? "Ready" : "Error")
            badgeColor: root.dirty
                ? Theme.DarkTheme.warning
                : (root.sourceValid ? Theme.DarkTheme.accent : Theme.DarkTheme.danger)
        }

        Row {
            width: parent.width
            height: 32
            spacing: 7

            MfButton {
                width: 118
                text: "New Controller"
                accent: true
                enabled: editorBridge.projectOpen
                onClicked: editorBridge.executeCommand("luau.new_controller")
            }

            MfButton {
                width: 74
                text: "Refresh"
                onClicked: root.refreshScripts(root.currentPath, false)
            }

            MfButton {
                width: 76
                text: "Validate"
                enabled: root.currentPath.length > 0
                onClicked: root.validateCurrent()
            }

            MfButton {
                width: 66
                text: "Save"
                accent: root.dirty
                enabled: root.dirty
                onClicked: root.saveCurrent()
            }

            MfButton {
                width: 68
                text: "Revert"
                enabled: root.currentPath.length > 0 && root.dirty
                onClicked: root.openScript(root.currentPath, true)
            }

            Text {
                width: parent.width - x
                height: parent.height
                text: root.diagnostic
                color: root.sourceValid ? Theme.DarkTheme.muted : Theme.DarkTheme.danger
                font.pixelSize: 10
                horizontalAlignment: Text.AlignRight
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }

        Row {
            width: parent.width
            height: parent.height - y
            spacing: 8

            Rectangle {
                width: Math.min(230, Math.max(170, parent.width * 0.24))
                height: parent.height
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft
                border.width: 1

                ListView {
                    id: scriptList
                    anchors.fill: parent
                    anchors.margins: 5
                    clip: true
                    spacing: 3
                    model: scriptModel

                    delegate: Rectangle {
                        id: scriptRow
                        required property string relativePath
                        required property string name
                        required property int bytes
                        required property bool valid
                        required property string diagnostic

                        width: ListView.view.width
                        height: 48
                        radius: 4
                        color: root.currentPath === scriptRow.relativePath
                            ? Theme.DarkTheme.accentSoft
                            : (scriptMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent")
                        border.color: root.currentPath === scriptRow.relativePath
                            ? Theme.DarkTheme.accent
                            : "transparent"
                        border.width: 1

                        Column {
                            anchors.fill: parent
                            anchors.margins: 6
                            spacing: 3

                            Text {
                                width: parent.width
                                text: scriptRow.name
                                color: Theme.DarkTheme.text
                                font.pixelSize: 12
                                font.bold: true
                                elide: Text.ElideRight
                            }

                            Text {
                                width: parent.width
                                text: (scriptRow.valid ? "✓ " : "! ") + scriptRow.relativePath
                                color: scriptRow.valid ? Theme.DarkTheme.muted : Theme.DarkTheme.danger
                                font.pixelSize: 9
                                elide: Text.ElideMiddle
                            }
                        }

                        MouseArea {
                            id: scriptMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: root.openScript(scriptRow.relativePath, false)
                        }

                        ToolTip.visible: scriptMouse.containsMouse && scriptRow.diagnostic.length > 0
                        ToolTip.text: scriptRow.diagnostic
                    }

                    Text {
                        visible: scriptList.count === 0
                        anchors.centerIn: parent
                        width: Math.max(100, parent.width - 20)
                        text: "No Luau scripts"
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.Wrap
                    }
                }
            }

            Rectangle {
                width: parent.width - x
                height: parent.height
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.background
                border.color: root.sourceValid ? Theme.DarkTheme.borderSoft : Theme.DarkTheme.danger
                border.width: 1

                ScrollView {
                    anchors.fill: parent
                    anchors.margins: 1
                    clip: true

                    TextArea {
                        id: codeEditor
                        width: Math.max(parent.width, implicitWidth)
                        height: Math.max(parent.height, contentHeight + 24)
                        readOnly: root.currentPath.length === 0
                        textFormat: TextEdit.PlainText
                        wrapMode: TextEdit.NoWrap
                        selectByMouse: true
                        tabStopDistance: 32
                        color: Theme.DarkTheme.text
                        selectionColor: Theme.DarkTheme.accent
                        selectedTextColor: Theme.DarkTheme.background
                        font.family: "Menlo"
                        font.pixelSize: 12
                        leftPadding: 12
                        rightPadding: 12
                        topPadding: 10
                        bottomPadding: 10
                        background: Rectangle {
                            color: Theme.DarkTheme.background
                        }

                        onTextChanged: {
                            if (root.dirty) {
                                root.diagnostic = "Unsaved changes · validate before saving"
                                root.sourceValid = true
                            }
                        }
                    }
                }
            }
        }
    }
}
