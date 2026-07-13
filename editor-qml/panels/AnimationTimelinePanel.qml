import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property var toolState: ({"sequence":{"name":"Timeline","duration":1,"frame_rate":30,"tracks":[]}, "track_types":[]})
    property string statusText: "Open a project to edit a sequence"
    property bool curveMode: false
    readonly property real duration: Math.max(0.01, Number(toolState.sequence.duration || 1))
    readonly property real cursor: Number(toolState.cursor || 0)

    function parse(source, fallback) {
        try { return JSON.parse(source) } catch (error) {
            statusText = "Timeline JSON error · " + error
            return fallback
        }
    }

    function refresh() {
        if (!editorBridge.projectOpen)
            return
        var source = editorBridge.toolStateJson("sequencer")
        if (source.length > 0) {
            toolState = parse(source, toolState)
            statusText = (toolState.dirty ? "Unsaved changes" : toolState.document_path)
        }
    }

    function run(action, payload) {
        var source = editorBridge.toolActionJson("sequencer", action, JSON.stringify(payload || {}))
        if (source.length > 0) {
            toolState = parse(source, toolState)
            statusText = action + " complete"
        } else {
            statusText = editorBridge.lastError
        }
    }

    function addKey(trackId, time) {
        run("add_keyframe", {"track_id": trackId, "time": time, "easing":"linear", "value":{}})
    }

    function waveformFor(trackId) {
        var waveforms = toolState.waveforms || []
        for (var index = 0; index < waveforms.length; ++index)
            if (waveforms[index].track_id === trackId) return waveforms[index]
        return null
    }

    function selectedTrackObject() {
        var tracks = toolState.sequence.tracks || []
        for (var index = 0; index < tracks.length; ++index)
            if (tracks[index].id === toolState.selected_track) return tracks[index]
        return null
    }

    function selectedKeyObject() {
        var track = selectedTrackObject()
        var index = Number(toolState.selected_key)
        return track && index >= 0 && index < track.keyframes.length ? track.keyframes[index] : null
    }

    function numericValue(value) {
        if (typeof value === "number") return Number(value)
        if (value && typeof value.value === "number") return Number(value.value)
        if (value && typeof value.x === "number") return Number(value.x)
        return 0
    }

    Connections {
        target: editorBridge
        function onProjectChanged() { root.refresh() }
        function onEditorToolChanged(tool) { if (tool === "sequencer") root.refresh() }
    }

    Timer {
        interval: 100
        repeat: true
        running: root.visible && root.toolState.playing === true
        onTriggered: root.run("tick", {"delta": interval / 1000})
    }

    Component.onCompleted: refresh()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 7

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Animation Timeline"
            detail: root.statusText
            badge: root.toolState.dirty ? "Unsaved" : (root.toolState.playing ? "Playing" : "Ready")
            badgeColor: root.toolState.dirty ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 5

            MfButton { text: "Undo"; enabled: root.toolState.can_undo === true; onClicked: root.run("undo", {}) }
            MfButton { text: "Redo"; enabled: root.toolState.can_redo === true; onClicked: root.run("redo", {}) }
            MfButton { text: root.toolState.playing ? "Pause" : "Play"; accent: true; onClicked: root.run("set_playing", {"value":!root.toolState.playing}) }
            MfButton { text: "Stop"; onClicked: { root.run("set_playing", {"value":false}); root.run("set_cursor", {"cursor":0}) } }
            MfButton { text: "Save"; accent: root.toolState.dirty === true; onClicked: root.run("save", {}) }
            MfButton { text: "+ Track"; onClicked: addTrackPopup.open() }
            MfButton { text: "Curves"; accent: root.curveMode; onClicked: root.curveMode = !root.curveMode }

            Item { Layout.fillWidth: true }
            Text { text: root.cursor.toFixed(2) + "s"; color: Theme.DarkTheme.text; font.bold: true }
            CheckBox {
                text: "Loop"
                checked: root.toolState.looped === true
                onToggled: root.run("set_looped", {"value":checked})
            }
            TextField {
                id: durationField
                Layout.preferredWidth: 64
                text: root.duration.toFixed(2)
                placeholderText: "Seconds"
                onEditingFinished: root.run("set_duration", {"duration":Number(text)})
            }
            TextField {
                id: fpsField
                Layout.preferredWidth: 52
                text: Number(root.toolState.sequence.frame_rate || 30).toFixed(0)
                placeholderText: "FPS"
                onEditingFinished: root.run("set_frame_rate", {"frame_rate":Number(text)})
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft

            Column {
                anchors.fill: parent
                anchors.margins: 6
                spacing: 0

                Row {
                    width: parent.width
                    height: 28
                    Rectangle {
                        width: 190; height: parent.height; color: Theme.DarkTheme.surfaceRaised
                        Text { anchors.centerIn: parent; text: root.toolState.sequence.name || "Timeline"; color: Theme.DarkTheme.text; font.bold: true }
                    }
                    Rectangle {
                        id: ruler
                        width: parent.width - 190
                        height: parent.height
                        color: Theme.DarkTheme.surfaceRaised
                        Repeater {
                            model: 9
                            delegate: Item {
                                required property int index
                                x: index * ruler.width / 8
                                width: 1; height: ruler.height
                                Rectangle { width: 1; height: parent.height; color: Theme.DarkTheme.border }
                                Text {
                                    x: 3; y: 2
                                    text: (index * root.duration / 8).toFixed(1)
                                    color: Theme.DarkTheme.muted; font.pixelSize: 9
                                }
                            }
                        }
                    }
                }

                Flickable {
                    id: tracksFlick
                    width: parent.width
                    height: parent.height - 28
                    contentHeight: Math.max(height, tracksColumn.height)
                    clip: true

                    Column {
                        id: tracksColumn
                        width: tracksFlick.width

                        Repeater {
                            model: root.toolState.sequence.tracks || []
                            delegate: Rectangle {
                                id: trackRow
                                required property var modelData
                                required property int index
                                width: tracksColumn.width
                                height: 48
                                color: root.toolState.selected_track === modelData.id ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.surface
                                border.color: Theme.DarkTheme.borderSoft

                                Rectangle {
                                    width: 190; height: parent.height
                                    color: "transparent"
                                    Column {
                                        anchors.left: parent.left; anchors.leftMargin: 8; anchors.verticalCenter: parent.verticalCenter
                                        Text { text: trackRow.modelData.id; color: Theme.DarkTheme.text; font.bold: true; font.pixelSize: 11 }
                                        Text { text: trackRow.modelData.track_type + (trackRow.modelData.target ? " · " + trackRow.modelData.target : ""); color: Theme.DarkTheme.muted; font.pixelSize: 9 }
                                    }
                                    MouseArea { anchors.fill: parent; onClicked: root.run("select_track", {"track_id":trackRow.modelData.id}) }
                                }

                                Item {
                                    id: lane
                                    x: 190; width: parent.width - x; height: parent.height
                                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 1; color: Theme.DarkTheme.border }
                                    Canvas {
                                        anchors.fill: parent
                                        visible: trackRow.modelData.track_type === "audio" && root.waveformFor(trackRow.modelData.id) !== null
                                        opacity: 0.72
                                        onVisibleChanged: requestPaint()
                                        onPaint: {
                                            var context = getContext("2d")
                                            context.reset()
                                            var waveform = root.waveformFor(trackRow.modelData.id)
                                            if (!waveform) return
                                            var samples = waveform.samples || []
                                            context.strokeStyle = "#5fd39a"
                                            context.lineWidth = 1
                                            for (var sample = 0; sample < samples.length; ++sample) {
                                                var x = sample / Math.max(1, samples.length - 1) * width
                                                var half = Number(samples[sample]) * height * 0.42
                                                context.beginPath(); context.moveTo(x, height/2-half); context.lineTo(x, height/2+half); context.stroke()
                                            }
                                        }
                                    }
                                    Repeater {
                                        model: trackRow.modelData.keyframes || []
                                        delegate: Rectangle {
                                            required property var modelData
                                            required property int index
                                            property real dragOffset: 0
                                            property real dragStartX: 0
                                            x: Math.max(0, Math.min(lane.width - width, Number(modelData.time) / root.duration * lane.width - width / 2 + dragOffset))
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: 11; height: 11; rotation: 45; radius: 1
                                            color: root.toolState.selected_track === trackRow.modelData.id && root.toolState.selected_key === index ? Theme.DarkTheme.warning : Theme.DarkTheme.accent
                                            border.color: Theme.DarkTheme.text
                                            MouseArea {
                                                anchors.fill: parent
                                                onPressed: function(mouse) {
                                                    parent.dragStartX = mapToItem(lane, mouse.x, mouse.y).x
                                                    root.run("select_key", {"track_id":trackRow.modelData.id, "index":index})
                                                }
                                                onPositionChanged: function(mouse) {
                                                    if (pressed)
                                                        parent.dragOffset = mapToItem(lane, mouse.x, mouse.y).x - parent.dragStartX
                                                }
                                                onReleased: {
                                                    var time = Math.max(0, Math.min(root.duration, (parent.x + parent.width/2) / lane.width * root.duration))
                                                    parent.dragOffset = 0
                                                    root.run("move_keyframe", {"track_id":trackRow.modelData.id, "index":index, "time":time})
                                                }
                                            }
                                            ToolTip.visible: keyHover.containsMouse
                                            ToolTip.text: Number(modelData.time).toFixed(3) + "s · " + modelData.easing
                                            MouseArea { id: keyHover; anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.NoButton }
                                        }
                                    }
                                    Rectangle {
                                        x: root.cursor / root.duration * lane.width
                                        width: 1; height: parent.height; color: Theme.DarkTheme.warning
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        acceptedButtons: Qt.LeftButton
                                        onPressed: function(mouse) { root.run("set_cursor", {"cursor":mouse.x / width * root.duration}) }
                                        onDoubleClicked: function(mouse) { root.addKey(trackRow.modelData.id, mouse.x / width * root.duration) }
                                    }
                                }
                            }
                        }

                        Text {
                            width: parent.width; height: 90
                            visible: (root.toolState.sequence.tracks || []).length === 0
                            text: "Add a track, then double-click a lane to create keyframes"
                            color: Theme.DarkTheme.muted
                            horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: root.curveMode ? 132 : 0
            visible: root.curveMode
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft

            RowLayout {
                anchors.fill: parent; anchors.margins: 7; spacing: 8
                Canvas {
                    id: curveCanvas
                    Layout.fillWidth: true; Layout.fillHeight: true
                    onPaint: {
                        var context = getContext("2d")
                        context.reset(); context.fillStyle = "#171a20"; context.fillRect(0,0,width,height)
                        context.strokeStyle = "#343b46"; context.beginPath(); context.moveTo(0,height/2); context.lineTo(width,height/2); context.stroke()
                        var track = root.selectedTrackObject()
                        if (!track || track.keyframes.length === 0) return
                        var values = []
                        for (var i=0;i<track.keyframes.length;++i) values.push(root.numericValue(track.keyframes[i].value))
                        var minValue = Math.min.apply(Math, values); var maxValue = Math.max.apply(Math, values)
                        if (maxValue === minValue) { maxValue += 1; minValue -= 1 }
                        context.strokeStyle = "#5fd39a"; context.lineWidth = 2; context.beginPath()
                        for (var key=0;key<track.keyframes.length;++key) {
                            var x = Number(track.keyframes[key].time) / root.duration * width
                            var y = height - (values[key]-minValue)/(maxValue-minValue)*height
                            if (key===0) context.moveTo(x,y); else context.lineTo(x,y)
                        }
                        context.stroke()
                        for (var point=0;point<track.keyframes.length;++point) {
                            var px = Number(track.keyframes[point].time)/root.duration*width
                            var py = height-(values[point]-minValue)/(maxValue-minValue)*height
                            context.fillStyle = point===Number(root.toolState.selected_key) ? "#ffc258" : "#6aaeff"
                            context.beginPath(); context.arc(px,py,4,0,Math.PI*2); context.fill()
                            var curve = track.keyframes[point].value && track.keyframes[point].value.__curve
                            if (curve) {
                                context.strokeStyle="#ffc258"; context.beginPath(); context.moveTo(px-24,py+Number(curve.in_tangent)*12); context.lineTo(px+24,py-Number(curve.out_tangent)*12); context.stroke()
                            }
                        }
                    }
                    Connections {
                        target: root
                        function onToolStateChanged() { curveCanvas.requestPaint() }
                    }
                }
                ColumnLayout {
                    Layout.preferredWidth: 148
                    Text { text: "TANGENTS"; color: Theme.DarkTheme.muted; font.bold: true; font.pixelSize: 9 }
                    TextField { id: inTangent; Layout.fillWidth: true; placeholderText: "In tangent"; text: { var key=root.selectedKeyObject(); return key && key.value && key.value.__curve ? String(key.value.__curve.in_tangent) : "0" } }
                    TextField { id: outTangent; Layout.fillWidth: true; placeholderText: "Out tangent"; text: { var key=root.selectedKeyObject(); return key && key.value && key.value.__curve ? String(key.value.__curve.out_tangent) : "0" } }
                    MfButton {
                        Layout.fillWidth: true; text: "Apply Tangents"; accent: true
                        enabled: root.selectedKeyObject() !== null
                        onClicked: root.run("set_tangents", {"track_id":root.toolState.selected_track,"index":Number(root.toolState.selected_key),"in_tangent":Number(inTangent.text),"out_tangent":Number(outTangent.text)})
                    }
                }
            }
        }
    }

    Popup {
        id: addTrackPopup
        anchors.centerIn: parent
        width: 330; height: 220
        modal: true; focus: true
        background: Rectangle { color: Theme.DarkTheme.surfaceRaised; radius: 8; border.color: Theme.DarkTheme.border }
        ColumnLayout {
            anchors.fill: parent; anchors.margins: 14; spacing: 9
            Text { text: "Add Animation Track"; color: Theme.DarkTheme.text; font.bold: true; font.pixelSize: 15 }
            TextField { id: trackId; Layout.fillWidth: true; placeholderText: "Unique track id" }
            TextField { id: trackTarget; Layout.fillWidth: true; placeholderText: "Target entity / widget (optional)" }
            ComboBox { id: trackType; Layout.fillWidth: true; model: root.toolState.track_types || [] }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                MfButton { text: "Cancel"; onClicked: addTrackPopup.close() }
                MfButton {
                    text: "Add"; accent: true
                    enabled: trackId.text.trim().length > 0 && trackType.currentText.length > 0
                    onClicked: {
                        root.run("add_track", {"id":trackId.text.trim(), "target":trackTarget.text.trim(), "track_type":trackType.currentText})
                        trackId.clear(); trackTarget.clear(); addTrackPopup.close()
                    }
                }
            }
        }
    }
}
