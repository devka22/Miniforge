import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.background

    property var catalog: ({})
    property var profiles: []
    property var installPlan: ({})
    property string statusText: ""
    property bool statusError: false

    function formatGiB(bytes) {
        var gib = Number(bytes || 0) / (1024 * 1024 * 1024)
        return gib.toFixed(gib >= 10 ? 1 : 2) + " GiB"
    }

    function loadCatalog() {
        var source = editorBridge.sdkPackCatalogJson()
        if (source.length === 0) {
            statusText = editorBridge.lastError
            statusError = true
            return
        }
        try {
            var envelope = JSON.parse(source)
            catalog = envelope.catalog || {}
            profiles = catalog.profiles || []
            statusText = envelope.validation && envelope.validation.valid
                ? "Catalog verified · signed release manifests required"
                : "Catalog validation reported issues"
            statusError = !(envelope.validation && envelope.validation.valid)
        } catch (error) {
            statusText = "Invalid SDK pack catalog · " + error
            statusError = true
        }
    }

    function planProfile(profile) {
        var source = editorBridge.sdkPackPlanJson(
            String(profile.id),
            JSON.stringify({"installed":[]})
        )
        if (source.length === 0) {
            statusText = editorBridge.lastError
            statusError = true
            return
        }
        try {
            var envelope = JSON.parse(source)
            installPlan = envelope.plan || {}
            statusText = String(installPlan.profile_label || profile.label)
                + " resolves to " + formatGiB(installPlan.projected_installed_bytes)
                + " installed"
            statusError = !installPlan.meets_profile_target
        } catch (error) {
            statusText = "Invalid SDK pack plan · " + error
            statusError = true
        }
    }

    Component.onCompleted: loadCatalog()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 9

        MfPanelHeader {
            Layout.fillWidth: true
            title: "SDK & Content Packs"
            detail: "Versioned production toolchains and reusable engine content. Games are never part of a pack."
            badge: root.formatGiB(root.catalog.total_available_installed_bytes || 0)
            badgeColor: Theme.DarkTheme.info
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 44
            radius: Theme.DarkTheme.radius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 12

                Text {
                    Layout.fillWidth: true
                    text: root.statusText
                    color: root.statusError ? Theme.DarkTheme.danger : Theme.DarkTheme.accent
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
                Text {
                    text: "Plan → verify SHA-256 → install atomically"
                    color: Theme.DarkTheme.muted
                    font.pixelSize: 9
                }
            }
        }

        ListView {
            id: profileList
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(contentHeight, 390)
            clip: true
            spacing: 7
            model: root.profiles
            ScrollBar.vertical: ScrollBar {}

            delegate: Rectangle {
                id: profileCard
                required property var modelData
                width: ListView.view.width
                height: 116
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: String(root.installPlan.profile_id || "") === String(modelData.id)
                    ? Theme.DarkTheme.accent : Theme.DarkTheme.borderSoft
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 11
                    spacing: 12

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Text {
                            Layout.fillWidth: true
                            text: profileCard.modelData.label
                            color: Theme.DarkTheme.text
                            font.pixelSize: 14
                            font.bold: true
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: profileCard.modelData.summary
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 10
                            wrapMode: Text.WordWrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: (profileCard.modelData.pack_ids || []).length + " packs · target "
                                + root.formatGiB(profileCard.modelData.target_min_bytes) + "–"
                                + root.formatGiB(profileCard.modelData.target_max_bytes)
                            color: Theme.DarkTheme.accent
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                    }

                    MfButton {
                        Layout.preferredWidth: 126
                        text: "Plan install"
                        accent: String(root.installPlan.profile_id || "")
                            !== String(profileCard.modelData.id)
                        onClicked: root.planProfile(profileCard.modelData)
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 150
            radius: Theme.DarkTheme.cardRadius
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 11
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        Layout.fillWidth: true
                        text: root.installPlan.profile_label
                            ? root.installPlan.profile_label + " installation plan"
                            : "Select a profile to inspect its dependency-resolved plan"
                        color: Theme.DarkTheme.text
                        font.pixelSize: 12
                        font.bold: true
                        elide: Text.ElideRight
                    }
                    Text {
                        visible: !!root.installPlan.profile_label
                        text: root.formatGiB(root.installPlan.total_download_bytes) + " download · "
                            + root.formatGiB(root.installPlan.projected_installed_bytes) + " installed"
                        color: Theme.DarkTheme.info
                        font.pixelSize: 10
                    }
                }

                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 3
                    model: root.installPlan.install || []
                    ScrollBar.vertical: ScrollBar {}

                    delegate: RowLayout {
                        required property var modelData
                        width: ListView.view.width
                        height: 28
                        spacing: 8

                        Text {
                            Layout.fillWidth: true
                            text: modelData.label + " · " + modelData.reason
                            color: Theme.DarkTheme.text
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                        Text {
                            text: root.formatGiB(modelData.download_bytes) + " → "
                                + root.formatGiB(modelData.installed_bytes)
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 9
                        }
                    }
                }
            }
        }
    }
}
