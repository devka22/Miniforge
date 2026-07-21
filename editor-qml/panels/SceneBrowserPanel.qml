import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel
    property var browserState: ({"scenes":[],"current":"","loaded":[],"stack":[],"dirty":false})
    property string statusText: "Open a project to browse scenes"

    function parse(source) {
        try { return JSON.parse(source) } catch (error) { statusText="Scene JSON error · "+error; return browserState }
    }
    function refresh() {
        if (!editorBridge.projectOpen) return
        var source=editorBridge.sceneBrowserStateJson()
        if (source.length>0) { browserState=parse(source); statusText=(browserState.dirty ? "Unsaved · " : "")+(browserState.current || "No active scene") }
    }
    function run(action,payload) {
        var source=editorBridge.sceneBrowserActionJson(action,JSON.stringify(payload||{}))
        if (source.length>0) { browserState=parse(source); statusText=action+" complete" } else statusText=editorBridge.lastError
    }
    Connections {
        target: editorBridge
        function onProjectChanged(){root.refresh()}
        function onSceneStateChanged(){root.refresh()}
    }
    Component.onCompleted: refresh()

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 9; spacing: 7
        MfPanelHeader {
            Layout.fillWidth: true; title: "Scene Browser"; detail: root.statusText
            badge: (root.browserState.stack||[]).length+" stack"; badgeColor: root.browserState.dirty?Theme.DarkTheme.warning:Theme.DarkTheme.accent
        }
        RowLayout {
            Layout.fillWidth: true
            TextField { id: sceneName; Layout.fillWidth: true; placeholderText: "Scene name" }
            MfButton { text: "New"; accent: true; onClicked: root.run("new",{"name":sceneName.text||"NewScene"}) }
            MfButton { text: "Duplicate"; onClicked: root.run("duplicate",{"name":sceneName.text||String(root.browserState.current)+"Copy"}) }
            MfButton { text: "Save"; onClicked: root.run("save",{}) }
            MfButton { text: "Restart"; onClicked: root.run("restart",{}) }
            MfButton { text: "Pop"; enabled:(root.browserState.stack||[]).length>1; onClicked:root.run("pop",{}) }
        }
        SplitView {
            Layout.fillWidth: true; Layout.fillHeight: true; orientation:Qt.Horizontal
            Rectangle {
                SplitView.fillWidth:true; color:Theme.DarkTheme.surface; border.color:Theme.DarkTheme.borderSoft
                ListView {
                    anchors.fill:parent; anchors.margins:6; clip:true; spacing:4
                    model:root.browserState.scenes||[]
                    delegate:Rectangle {
                        required property var modelData
                        width:ListView.view.width; height:48; radius:5
                        color:modelData===root.browserState.current?Theme.DarkTheme.surfaceRaised:Theme.DarkTheme.panel
                        border.color:modelData===root.browserState.current?Theme.DarkTheme.accent:Theme.DarkTheme.borderSoft
                        RowLayout {
                            anchors.fill:parent; anchors.margins:6
                            ColumnLayout {
                                Layout.fillWidth:true; spacing:1
                                Text { text:modelData; color:Theme.DarkTheme.text; font.bold:true; elide:Text.ElideRight; Layout.fillWidth:true }
                                Text { text:(root.browserState.loaded||[]).indexOf(modelData)>=0?"Loaded":"On disk"; color:Theme.DarkTheme.muted; font.pixelSize:9 }
                            }
                            MfButton { text:"Load"; accent:modelData===root.browserState.current; onClicked:root.run("load",{"name":modelData}) }
                            MfButton { text:"Add"; onClicked:root.run("additive",{"name":modelData}) }
                            MfButton { text:"Push"; onClicked:root.run("push",{"name":modelData}) }
                            MfButton { text:"Unload"; enabled:(root.browserState.loaded||[]).indexOf(modelData)>=0&&modelData!==root.browserState.current; onClicked:root.run("unload",{"name":modelData}) }
                        }
                    }
                }
            }
            Rectangle {
                SplitView.preferredWidth:250; color:Theme.DarkTheme.surface; border.color:Theme.DarkTheme.borderSoft
                ColumnLayout {
                    anchors.fill:parent; anchors.margins:9
                    Text { text:"RUNTIME SCENE STATE"; color:Theme.DarkTheme.accent; font.bold:true; font.pixelSize:10 }
                    Text { Layout.fillWidth:true; text:"Current\n"+(root.browserState.current||"—"); color:Theme.DarkTheme.text; wrapMode:Text.Wrap }
                    Text { Layout.fillWidth:true; text:"Loaded\n"+(root.browserState.loaded||[]).join("\n"); color:Theme.DarkTheme.muted; wrapMode:Text.Wrap }
                    Text { text:"STACK"; color:Theme.DarkTheme.accent; font.bold:true; font.pixelSize:10 }
                    ListView {
                        Layout.fillWidth:true; Layout.fillHeight:true; clip:true; model:root.browserState.stack||[]
                        delegate:Text { required property var modelData; required property int index; width:ListView.view.width; height:26; text:(index+1)+"  "+modelData; color:Theme.DarkTheme.text; verticalAlignment:Text.AlignVCenter }
                    }
                }
            }
        }
    }
}
