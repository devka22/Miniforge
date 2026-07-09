import QtQuick
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    function runCommand(commandId) {
        if (typeof editorController !== "undefined")
            editorController.executeCommand(commandId)
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        MfPanelHeader {
            width: parent.width
            title: "Sprite Studio"
            detail: "Pixel art, palettes, slices and animation drafts"
            badge: "Art"
            badgeColor: Theme.DarkTheme.warning
        }

        Flow {
            width: parent.width
            height: 72
            spacing: 8

            MfButton {
                width: 112
                text: "New Sprite"
                accent: true
                onClicked: root.runCommand("sprite.new_pixel_art")
            }

            MfButton {
                width: 118
                text: "Hero Base"
                onClicked: root.runCommand("sprite.create_hero_template")
            }

            MfButton {
                width: 112
                text: "Frames"
                onClicked: root.runCommand("sprite.export_frames")
            }

            MfButton {
                width: 112
                text: "Atlas"
                onClicked: root.runCommand("sprite.export_atlas_pages")
            }

            MfButton {
                width: 112
                text: "Palette"
                onClicked: root.runCommand("sprite.optimize_palette")
            }
        }

        Row {
            width: parent.width
            height: parent.height - y
            spacing: 10

            Rectangle {
                width: Math.min(300, parent.width * 0.42)
                height: parent.height
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft
                border.width: 1

                Canvas {
                    id: spriteCanvas
                    anchors.fill: parent
                    anchors.margins: 12
                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.fillStyle = "#16181d"
                        ctx.fillRect(0, 0, width, height)
                        var cells = 16
                        var size = Math.floor(Math.min(width, height) / cells)
                        var ox = Math.floor((width - size * cells) / 2)
                        var oy = Math.floor((height - size * cells) / 2)
                        for (var y = 0; y < cells; y++) {
                            for (var x = 0; x < cells; x++) {
                                ctx.fillStyle = (x + y) % 2 === 0 ? "#20242c" : "#181b22"
                                ctx.fillRect(ox + x * size, oy + y * size, size, size)
                            }
                        }
                        ctx.fillStyle = "#6cc58f"
                        ctx.fillRect(ox + 6 * size, oy + 4 * size, 4 * size, 8 * size)
                        ctx.fillStyle = "#d6a84f"
                        ctx.fillRect(ox + 8 * size, oy + 5 * size, 4 * size, 2 * size)
                        ctx.strokeStyle = "#e8eaee"
                        ctx.lineWidth = Math.max(1, Math.floor(size / 8))
                        ctx.strokeRect(ox + 5 * size, oy + 3 * size, 6 * size, 10 * size)
                        ctx.strokeStyle = "#2f3540"
                        ctx.lineWidth = 1
                        for (var gx = 0; gx <= cells; gx++) {
                            ctx.beginPath()
                            ctx.moveTo(ox + gx * size, oy)
                            ctx.lineTo(ox + gx * size, oy + cells * size)
                            ctx.stroke()
                        }
                        for (var gy = 0; gy <= cells; gy++) {
                            ctx.beginPath()
                            ctx.moveTo(ox, oy + gy * size)
                            ctx.lineTo(ox + cells * size, oy + gy * size)
                            ctx.stroke()
                        }
                    }
                }
            }

            Column {
                width: parent.width - x
                height: parent.height
                spacing: 8

                Repeater {
                    model: [
                        ["Brush", "Mirror, radius, hard pixel and alpha-aware paint"],
                        ["Shape", "Rect, circle, line, fill and outline tools"],
                        ["Cleanup", "Trim, alpha outline, shadow and palette quantize"],
                        ["Animation", "Grid slicing, timeline markers and .spriteframes export"]
                    ]

                    Rectangle {
                        width: parent.width
                        height: 52
                        radius: Theme.DarkTheme.cardRadius
                        color: Theme.DarkTheme.surfaceRaised
                        border.color: Theme.DarkTheme.borderSoft
                        border.width: 1

                        Column {
                            anchors.fill: parent
                            anchors.margins: 8
                            spacing: 3

                            Text {
                                width: parent.width
                                text: modelData[0]
                                color: Theme.DarkTheme.text
                                font.pixelSize: 13
                                font.bold: true
                                elide: Text.ElideRight
                            }

                            Text {
                                width: parent.width
                                text: modelData[1]
                                color: Theme.DarkTheme.muted
                                font.pixelSize: 11
                                elide: Text.ElideRight
                            }
                        }
                    }
                }

                Row {
                    width: parent.width
                    height: 28
                    spacing: 6

                    Repeater {
                        model: ["#6cc58f", "#d6a84f", "#e56f66", "#7db9ff", "#e8eaee", "#181b22"]
                        Rectangle {
                            width: 28
                            height: 28
                            radius: 4
                            color: modelData
                            border.color: Theme.DarkTheme.border
                            border.width: 1
                        }
                    }
                }
            }
        }
    }
}
