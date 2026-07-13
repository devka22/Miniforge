import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property var toolState: ({"designer":{"canvas":{"name":"UI","viewport_width":1280,"viewport_height":720},"palette":[],"preview_resolution":[1280,720],"selected_widget":null,"snap":true,"guides":true,"show_safe_area":true},"hierarchy":[],"preview":[],"validation":{"issues":[]}})
    property string paletteFilter: ""
    property string statusText: "Open a project to design game UI"
    property string dragWidgetId: ""
    property real previewDx: 0
    property real previewDy: 0
    property real previewWidthDelta: 0
    property real previewHeightDelta: 0
    readonly property var designer: toolState.designer || ({})
    readonly property var resolution: designer.preview_resolution || [1280,720]
    readonly property bool hasSelection: designer.selected_widget !== null && designer.selected_widget !== undefined

    function parse(source, fallback) {
        try { return JSON.parse(source) } catch (error) {
            statusText = "UI Designer JSON error · " + error
            return fallback
        }
    }

    function accept(source, detail) {
        if (source.length === 0) {
            statusText = editorBridge.lastError
            return
        }
        toolState = parse(source, toolState)
        statusText = detail || (toolState.dirty ? "Unsaved UI changes" : toolState.document_path)
    }

    function refresh() {
        if (editorBridge.projectOpen)
            accept(editorBridge.toolStateJson("ui_designer"), "UI document ready")
    }

    function run(action, payload) {
        accept(editorBridge.toolActionJson("ui_designer", action, JSON.stringify(payload || {})), action + " complete")
    }

    function filteredPalette() {
        var source = designer.palette || []
        var query = paletteFilter.toLowerCase().trim()
        if (query.length === 0) return source
        var result = []
        for (var index = 0; index < source.length; ++index) {
            var text = (source[index].widget_type + " " + source[index].category + " " + source[index].description).toLowerCase()
            if (text.indexOf(query) >= 0) result.push(source[index])
        }
        return result
    }

    function selectedPreview() {
        var widgets = toolState.preview || []
        for (var index = 0; index < widgets.length; ++index)
            if (widgets[index].id === designer.selected_widget) return widgets[index]
        return null
    }

    function selectedWidgetData() {
        return toolState.selected_widget_data || ({"bindings":[], "callbacks":[]})
    }

    function widgetColor(typeName, selected) {
        if (selected) return Theme.DarkTheme.accent
        if (typeName.indexOf("Button") >= 0) return "#5967a8"
        if (typeName.indexOf("Text") >= 0 || typeName === "Label") return "#59616e"
        if (typeName.indexOf("Progress") >= 0) return "#3e8b69"
        if (typeName === "Canvas") return "transparent"
        return "#343a44"
    }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.refresh() }
        function onEditorToolChanged(tool) { if (tool === "ui_designer") root.refresh() }
    }
    Component.onCompleted: refresh()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 7

        MfPanelHeader {
            Layout.fillWidth: true
            title: "UI Designer"
            detail: root.statusText
            badge: root.resolution[0] + "×" + root.resolution[1]
            badgeColor: root.toolState.dirty ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        RowLayout {
            Layout.fillWidth: true; spacing: 5
            MfButton { text: "Undo"; enabled: root.toolState.can_undo === true; onClicked: root.run("undo", {}) }
            MfButton { text: "Redo"; enabled: root.toolState.can_redo === true; onClicked: root.run("redo", {}) }
            MfButton { text: "Save"; accent: root.toolState.dirty === true; onClicked: root.run("save", {}) }
            MfButton { text: "Validate"; onClicked: root.run("validate", {}) }
            MfButton { text: "Duplicate"; enabled: root.hasSelection; onClicked: root.run("duplicate", {"id":String(root.designer.selected_widget) + "Copy"}) }
            MfButton { text: "Delete"; enabled: root.hasSelection; onClicked: root.run("delete", {}) }
            Item { Layout.fillWidth: true }
            ComboBox {
                id: templateBox
                Layout.preferredWidth: 112
                model: ["hud", "main_menu", "pause", "settings"]
            }
            MfButton { text: "New"; onClicked: root.run("new", {"template":templateBox.currentText}) }
            ComboBox {
                id: resolutionBox
                Layout.preferredWidth: 118
                model: ["1280×720", "1920×1080", "800×600", "390×844"]
                onActivated: {
                    var parts = currentText.split("×")
                    root.run("resolution", {"width":Number(parts[0]), "height":Number(parts[1])})
                }
            }
            CheckBox { text: "Snap"; checked: root.designer.snap === true; onToggled: root.run("snap", {"value":checked}) }
            CheckBox { text: "Guides"; checked: root.designer.guides === true; onToggled: root.run("guides", {"value":checked}) }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 7

            Rectangle {
                Layout.preferredWidth: 190
                Layout.fillHeight: true
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft

                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 7; spacing: 5
                    Text { text: "HIERARCHY"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true }
                    ListView {
                        id: hierarchyList
                        Layout.fillWidth: true; Layout.preferredHeight: Math.min(210, contentHeight); clip: true; spacing: 2
                        model: root.toolState.hierarchy || []
                        delegate: Rectangle {
                            required property var modelData
                            width: hierarchyList.width; height: 28; radius: 4
                            color: root.designer.selected_widget === modelData.id ? Theme.DarkTheme.surfaceRaised : "transparent"
                            Text {
                                anchors.left: parent.left; anchors.leftMargin: 6 + Number(modelData.depth) * 12
                                anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                                text: modelData.id + "  ·  " + modelData.widget_type
                                color: Theme.DarkTheme.text; font.pixelSize: 9; elide: Text.ElideRight
                            }
                            MouseArea { anchors.fill: parent; onClicked: root.run("select", {"widget_id":parent.modelData.id}) }
                        }
                    }
                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.DarkTheme.borderSoft }
                    Text { text: "WIDGET PALETTE"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true }
                    TextField { Layout.fillWidth: true; placeholderText: "Search widgets"; onTextChanged: root.paletteFilter = text }
                    ListView {
                        id: paletteList
                        Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 3
                        model: root.filteredPalette()
                        delegate: Rectangle {
                            required property var modelData
                            width: paletteList.width; height: 38; radius: 4
                            color: paletteMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent"
                            Column {
                                anchors.left: parent.left; anchors.leftMargin: 6; anchors.verticalCenter: parent.verticalCenter
                                Text { text: modelData.widget_type; color: Theme.DarkTheme.text; font.pixelSize: 10; font.bold: true }
                                Text { text: modelData.category; color: Theme.DarkTheme.muted; font.pixelSize: 8 }
                            }
                            MouseArea {
                                id: paletteMouse; anchors.fill: parent; hoverEnabled: true
                                onDoubleClicked: root.run("add_widget", {
                                    "widget_type":parent.modelData.widget_type,
                                    "id":parent.modelData.widget_type + "_" + Date.now(),
                                    "x":64, "y":64
                                })
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: Theme.DarkTheme.cardRadius
                color: "#121419"
                border.color: Theme.DarkTheme.borderSoft

                Item {
                    id: designStage
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 28, (parent.height - 28) * Number(root.resolution[0]) / Math.max(1, Number(root.resolution[1])))
                    height: Math.min(parent.height - 28, (parent.width - 28) * Number(root.resolution[1]) / Math.max(1, Number(root.resolution[0])))

                    Rectangle { anchors.fill: parent; color: "#20242b"; border.color: Theme.DarkTheme.focus }
                    Rectangle {
                        visible: root.designer.show_safe_area === true
                        anchors.fill: parent; anchors.margins: Math.min(parent.width, parent.height) * 0.05
                        color: "transparent"; border.color: "#536070"; border.width: 1
                    }
                    Repeater {
                        model: root.toolState.preview || []
                        delegate: Rectangle {
                            id: widgetRect
                            required property var modelData
                            property bool selected: root.designer.selected_widget === modelData.id
                            x: Number(modelData.rect.x) / Number(root.resolution[0]) * designStage.width + (root.dragWidgetId === modelData.id ? root.previewDx : 0)
                            y: Number(modelData.rect.y) / Number(root.resolution[1]) * designStage.height + (root.dragWidgetId === modelData.id ? root.previewDy : 0)
                            width: Math.max(2, Number(modelData.rect.width) / Number(root.resolution[0]) * designStage.width + (root.dragWidgetId === modelData.id ? root.previewWidthDelta : 0))
                            height: Math.max(2, Number(modelData.rect.height) / Number(root.resolution[1]) * designStage.height + (root.dragWidgetId === modelData.id ? root.previewHeightDelta : 0))
                            color: root.widgetColor(modelData.widget_type, selected)
                            opacity: modelData.widget_type === "Canvas" ? 0.08 : 0.78
                            border.width: selected ? 2 : 1
                            border.color: selected ? Theme.DarkTheme.warning : "#8390a3"
                            z: Number(modelData.depth) + (selected ? 100 : 0)
                            Text {
                                anchors.centerIn: parent
                                width: parent.width - 4
                                visible: parent.width > 34 && parent.height > 16
                                text: widgetRect.modelData.id
                                color: "white"; font.pixelSize: 8; elide: Text.ElideRight; horizontalAlignment: Text.AlignHCenter
                            }
                            MouseArea {
                                anchors.fill: parent
                                property point startPoint
                                onPressed: function(mouse) {
                                    root.run("select", {"widget_id":parent.modelData.id})
                                    root.dragWidgetId = parent.modelData.id
                                    startPoint = mapToItem(designStage, mouse.x, mouse.y)
                                }
                                onPositionChanged: function(mouse) {
                                    if (!pressed) return
                                    var point = mapToItem(designStage, mouse.x, mouse.y)
                                    root.previewDx = point.x - startPoint.x
                                    root.previewDy = point.y - startPoint.y
                                }
                                onReleased: {
                                    var dx = root.previewDx / designStage.width * Number(root.resolution[0])
                                    var dy = root.previewDy / designStage.height * Number(root.resolution[1])
                                    root.previewDx = 0; root.previewDy = 0; root.dragWidgetId = ""
                                    if (Math.abs(dx) > 0.1 || Math.abs(dy) > 0.1)
                                        root.run("move", {"dx":dx,"dy":dy})
                                }
                            }
                            Rectangle {
                                visible: widgetRect.selected
                                width: 11; height: 11
                                anchors.right: parent.right; anchors.bottom: parent.bottom
                                color: Theme.DarkTheme.warning; border.color: "white"; z: 4
                                MouseArea {
                                    anchors.fill: parent
                                    property point startPoint
                                    onPressed: function(mouse) {
                                        root.dragWidgetId=widgetRect.modelData.id
                                        startPoint=mapToItem(designStage,mouse.x,mouse.y)
                                    }
                                    onPositionChanged: function(mouse) {
                                        if (!pressed) return
                                        var point=mapToItem(designStage,mouse.x,mouse.y)
                                        root.previewWidthDelta=point.x-startPoint.x
                                        root.previewHeightDelta=point.y-startPoint.y
                                    }
                                    onReleased: {
                                        var item=root.selectedPreview()
                                        var widthValue=Number(item.rect.width)+root.previewWidthDelta/designStage.width*Number(root.resolution[0])
                                        var heightValue=Number(item.rect.height)+root.previewHeightDelta/designStage.height*Number(root.resolution[1])
                                        root.previewWidthDelta=0; root.previewHeightDelta=0; root.dragWidgetId=""
                                        root.run("resize", {"width":widthValue,"height":heightValue})
                                    }
                                }
                            }
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        z: -1
                        onClicked: function(mouse) {
                            root.run("select_point", {
                                "x":mouse.x / width * Number(root.resolution[0]),
                                "y":mouse.y / height * Number(root.resolution[1])
                            })
                        }
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 188
                Layout.fillHeight: true
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft

                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 7; spacing: 6
                    Text { text: "WIDGET INSPECTOR"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true }
                    Text { Layout.fillWidth: true; text: root.hasSelection ? root.designer.selected_widget : "Nothing selected"; color: Theme.DarkTheme.text; font.bold: true; elide: Text.ElideRight }
                    RowLayout {
                        Layout.fillWidth: true
                        MfButton { text: "←"; enabled: root.hasSelection; onClicked: root.run("move", {"dx":-8,"dy":0}) }
                        MfButton { text: "↑"; enabled: root.hasSelection; onClicked: root.run("move", {"dx":0,"dy":-8}) }
                        MfButton { text: "↓"; enabled: root.hasSelection; onClicked: root.run("move", {"dx":0,"dy":8}) }
                        MfButton { text: "→"; enabled: root.hasSelection; onClicked: root.run("move", {"dx":8,"dy":0}) }
                    }
                    Text { text: "ALIGN"; color: Theme.DarkTheme.muted; font.pixelSize: 9 }
                    GridLayout {
                        Layout.fillWidth: true; columns: 2
                        Repeater {
                            model: ["left", "right", "top", "bottom", "center", "middle"]
                            delegate: MfButton {
                                required property var modelData
                                text: modelData; enabled: root.hasSelection; Layout.fillWidth: true
                                onClicked: root.run("align", {"alignment":modelData})
                            }
                        }
                    }
                    Text { text: "SIZE"; color: Theme.DarkTheme.muted; font.pixelSize: 9 }
                    RowLayout {
                        Layout.fillWidth: true
                        TextField { id: widthField; Layout.fillWidth: true; placeholderText: "W"; text: { var item=root.selectedPreview(); return item ? Number(item.rect.width).toFixed(0) : "" } }
                        TextField { id: heightField; Layout.fillWidth: true; placeholderText: "H"; text: { var item=root.selectedPreview(); return item ? Number(item.rect.height).toFixed(0) : "" } }
                    }
                    MfButton { Layout.fillWidth: true; text: "Apply Size"; enabled: root.hasSelection; onClicked: root.run("resize", {"width":Number(widthField.text),"height":Number(heightField.text)}) }
                    Text { text: "PROPERTY"; color: Theme.DarkTheme.muted; font.pixelSize: 9 }
                    TextField { id: propertyKey; Layout.fillWidth: true; placeholderText: "Property name" }
                    TextField { id: propertyValue; Layout.fillWidth: true; placeholderText: "JSON value" }
                    MfButton {
                        Layout.fillWidth: true; text: "Set Property"; enabled: root.hasSelection && propertyKey.text.length > 0
                        onClicked: {
                            var value = propertyValue.text
                            try { value = JSON.parse(value) } catch (error) {}
                            root.run("property", {"key":propertyKey.text,"value":value})
                        }
                    }
                    MfButton {
                        Layout.fillWidth: true
                        text: "Parent, Bindings & Events…"
                        accent: true
                        enabled: root.hasSelection
                        onClicked: advancedPopup.open()
                    }
                    Item { Layout.fillHeight: true }
                    Text {
                        Layout.fillWidth: true
                        text: (root.toolState.validation.issues || []).length + " validation issues"
                        color: (root.toolState.validation.issues || []).length > 0 ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
                        font.pixelSize: 9; wrapMode: Text.Wrap
                    }
                }
            }
        }
    }

    Popup {
        id: advancedPopup
        anchors.centerIn: parent
        width: Math.min(650, root.width - 20)
        height: Math.min(540, root.height - 20)
        modal: true; focus: true
        background: Rectangle { color: Theme.DarkTheme.surfaceRaised; radius: 8; border.color: Theme.DarkTheme.border }
        ColumnLayout {
            anchors.fill: parent; anchors.margins: 12; spacing: 8
            MfPanelHeader {
                Layout.fillWidth: true
                title: "Widget Logic"
                detail: root.designer.selected_widget || "No selection"
                badge: "Runtime bindings"; badgeColor: Theme.DarkTheme.accent
            }
            RowLayout {
                Layout.fillWidth: true
                Text { text: "Parent"; color: Theme.DarkTheme.muted }
                ComboBox { id: parentWidget; Layout.fillWidth: true; model: root.toolState.hierarchy || []; textRole: "id"; displayText: currentIndex < 0 ? "Scene root" : currentText }
                MfButton { text: "Reparent"; onClicked: root.run("reparent", {"widget_id":root.designer.selected_widget,"parent_id":parentWidget.currentIndex < 0 ? null : parentWidget.currentText}) }
                MfButton { text: "To Root"; onClicked: root.run("reparent", {"widget_id":root.designer.selected_widget,"parent_id":null}) }
            }
            SplitView {
                Layout.fillWidth: true; Layout.fillHeight: true; orientation: Qt.Horizontal
                Rectangle {
                    SplitView.fillWidth: true; color: Theme.DarkTheme.surface; border.color: Theme.DarkTheme.borderSoft
                    ColumnLayout {
                        anchors.fill: parent; anchors.margins: 8
                        Text { text: "DATA BINDINGS"; color: Theme.DarkTheme.accent; font.bold: true }
                        ListView {
                            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 3
                            model: root.selectedWidgetData().bindings || []
                            delegate: Row {
                                required property var modelData
                                width: ListView.view.width; height: 34; spacing: 5
                                Text { width: parent.width-74; text: modelData.property + " ← " + modelData.source_path; color: Theme.DarkTheme.text; elide: Text.ElideRight; verticalAlignment: Text.AlignVCenter }
                                MfButton { width: 68; text: "Remove"; onClicked: root.run("remove_binding", {"widget_id":root.designer.selected_widget,"property":modelData.property}) }
                            }
                        }
                        TextField { id: bindingProperty; Layout.fillWidth: true; placeholderText: "Widget property (text, value…)" }
                        TextField { id: bindingSource; Layout.fillWidth: true; placeholderText: "Runtime path (player.health…)" }
                        TextField { id: bindingFallback; Layout.fillWidth: true; placeholderText: "Fallback JSON"; text: "null" }
                        MfButton {
                            Layout.fillWidth: true; text: "Add / Update Binding"; accent: true
                            onClicked: { var fallback=null; try { fallback=JSON.parse(bindingFallback.text) } catch (error) { fallback=bindingFallback.text }; root.run("upsert_binding", {"widget_id":root.designer.selected_widget,"property":bindingProperty.text,"source_path":bindingSource.text,"fallback":fallback}) }
                        }
                    }
                }
                Rectangle {
                    SplitView.fillWidth: true; color: Theme.DarkTheme.surface; border.color: Theme.DarkTheme.borderSoft
                    ColumnLayout {
                        anchors.fill: parent; anchors.margins: 8
                        Text { text: "CALLBACKS"; color: Theme.DarkTheme.accent; font.bold: true }
                        ListView {
                            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 3
                            model: root.selectedWidgetData().callbacks || []
                            delegate: Row {
                                required property var modelData
                                width: ListView.view.width; height: 34; spacing: 5
                                Text { width: parent.width-74; text: modelData.event + " → " + (modelData.graph || modelData.function || "command"); color: Theme.DarkTheme.text; elide: Text.ElideRight; verticalAlignment: Text.AlignVCenter }
                                MfButton { width: 68; text: "Remove"; onClicked: root.run("remove_callback", {"widget_id":root.designer.selected_widget,"event":modelData.event}) }
                            }
                        }
                        ComboBox { id: callbackEvent; Layout.fillWidth: true; model: ["click","hover","pressed","focus","value_changed"] }
                        TextField { id: callbackGraph; Layout.fillWidth: true; placeholderText: "Visual Graph path (optional)" }
                        TextField { id: callbackFunction; Layout.fillWidth: true; placeholderText: "Luau function (optional)" }
                        TextField { id: callbackPayload; Layout.fillWidth: true; placeholderText: "Payload JSON"; text: "{}" }
                        MfButton {
                            Layout.fillWidth: true; text: "Add / Update Callback"; accent: true
                            onClicked: { var payload={}; try { payload=JSON.parse(callbackPayload.text) } catch (error) {}; root.run("upsert_callback", {"widget_id":root.designer.selected_widget,"event":callbackEvent.currentText,"graph":callbackGraph.text,"function":callbackFunction.text,"payload":payload}) }
                        }
                    }
                }
            }
            RowLayout { Layout.alignment: Qt.AlignRight; MfButton { text: "Close"; onClicked: advancedPopup.close() } }
        }
    }
}
