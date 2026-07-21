import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import "../themes" as Theme
import "../components"

Rectangle {
    id: root
    color: Theme.DarkTheme.panel

    property string currentDirectory: ""
    property string selectedPath: ""
    property string selectedType: ""
    property string selectedPreviewUrl: ""
    property bool selectedEditable: false
    property string selectedGuid: ""
    property var selectedLabels: []
    property bool selectedIncludeInBuild: false
    property var selectedDependencies: []
    property var selectedReverseDependencies: []
    property var selectedWarnings: []
    property string savedText: ""
    property string statusText: editorBridge.projectOpen ? "Project content ready" : "Open a project"
    property var unfilteredEntries: []
    property var selectedPaths: []
    property int selectionAnchor: -1
    property bool gridView: true
    property bool busy: false
    property string contextFolderPath: ""
    readonly property bool editorDirty: selectedEditable && textEditor.text !== savedText
    readonly property bool globalSearchActive: contentSearch.text.trim().length >= 2

    function parseJson(value, fallback) {
        try {
            return JSON.parse(value)
        } catch (error) {
            statusText = "Content bridge JSON error: " + error
            return fallback
        }
    }

    function displayDirectory() {
        if (globalSearchActive)
            return "All project content"
        return currentDirectory.length > 0 ? currentDirectory : editorBridge.projectName
    }

    function parentPath(path) {
        var separator = path.lastIndexOf("/")
        return separator >= 0 ? path.substring(0, separator) : ""
    }

    function isManagedAssetPath(path) {
        var first = String(path || "").split("/")[0]
        return ["assets", "scripts", "scenes", "saves", "settings", "components", "systems", "plugins", "templates"].indexOf(first) >= 0
    }

    function selectionIsManageable() {
        if (selectedPaths.length === 0 || editorDirty)
            return false
        for (var index = 0; index < selectedPaths.length; ++index) {
            if (!isManagedAssetPath(selectedPaths[index]))
                return false
        }
        return true
    }

    function parentDirectory() {
        if (currentDirectory.length === 0)
            return
        navigateTo(parentPath(currentDirectory))
    }

    function navigateTo(path) {
        if (editorDirty) {
            statusText = "Save or revert " + selectedPath + " before changing folders"
            return
        }
        currentDirectory = path || ""
        clearSelection()
        refreshEntries()
    }

    function breadcrumbs() {
        var result = [{"label": editorBridge.projectName || "Project", "path": ""}]
        if (currentDirectory.length === 0)
            return result
        var parts = currentDirectory.split("/")
        var path = ""
        for (var index = 0; index < parts.length; ++index) {
            path += (path.length > 0 ? "/" : "") + parts[index]
            result.push({"label": parts[index], "path": path})
        }
        return result
    }

    function formatBytes(value) {
        var bytes = Number(value || 0)
        if (bytes < 1024)
            return bytes + " B"
        if (bytes < 1024 * 1024)
            return (bytes / 1024).toFixed(bytes < 10240 ? 1 : 0) + " KB"
        return (bytes / (1024 * 1024)).toFixed(1) + " MB"
    }

    function iconLabel(typeName, directory) {
        if (directory) return "DIR"
        if (typeName === "LuauScript") return "LUAU"
        if (typeName === "VisualGraph") return "FLOW"
        if (typeName === "Texture" || typeName === "Sprite" || typeName === "SpriteSheet") return "IMG"
        if (typeName === "Audio" || typeName === "SoundCue" || typeName === "AudioEvent") return "SND"
        if (typeName === "Scene") return "SCN"
        if (typeName === "Prefab") return "PFB"
        if (typeName === "Material") return "MAT"
        if (typeName === "Shader") return "FX"
        if (typeName === "Tilemap") return "TILE"
        if (typeName === "UI") return "UI"
        return typeName.substring(0, 4).toUpperCase()
    }

    function iconColor(typeName, directory) {
        if (directory) return Theme.DarkTheme.warning
        if (typeName === "LuauScript" || typeName === "VisualGraph") return Theme.DarkTheme.info
        if (typeName === "Texture" || typeName === "Sprite" || typeName === "SpriteSheet") return "#ce85f0"
        if (typeName === "Audio" || typeName === "SoundCue" || typeName === "AudioEvent") return "#68d3cf"
        if (typeName === "Scene" || typeName === "Prefab") return Theme.DarkTheme.accent
        return Theme.DarkTheme.muted
    }

    function entryByPath(path) {
        for (var index = 0; index < unfilteredEntries.length; ++index) {
            if (unfilteredEntries[index].relative_path === path)
                return unfilteredEntries[index]
        }
        return null
    }

    function entryModelIndex(path) {
        for (var index = 0; index < fileModel.count; ++index) {
            if (fileModel.get(index).relativePath === path)
                return index
        }
        return -1
    }

    function arrayValue(value) {
        return Array.isArray(value) ? value : []
    }

    function updateSelectedMetadata(entry) {
        selectedGuid = entry ? String(entry.guid || "") : ""
        selectedLabels = entry ? arrayValue(entry.labels) : []
        selectedIncludeInBuild = entry ? entry.include_in_build !== false : false
        selectedDependencies = entry ? arrayValue(entry.dependencies) : []
        selectedReverseDependencies = entry ? arrayValue(entry.reverse_dependencies) : []
        selectedWarnings = entry ? arrayValue(entry.warnings) : []
    }

    function joinedMetadata(values, fallback) {
        var items = arrayValue(values)
        return items.length > 0 ? items.join(", ") : fallback
    }

    function refreshSelectionRoles() {
        for (var index = 0; index < fileModel.count; ++index)
            fileModel.setProperty(index, "selected", selectedPaths.indexOf(fileModel.get(index).relativePath) >= 0)
    }

    function applyFilter() {
        var needle = contentSearch.text.trim().toLowerCase()
        var typeName = typeFilter.currentText
        var rows = []
        for (var index = 0; index < unfilteredEntries.length; ++index) {
            var entry = unfilteredEntries[index]
            var haystack = (entry.name + " " + entry.asset_type + " " + entry.relative_path).toLowerCase()
            var typeMatches = typeName === "All"
                || (typeName === "Folders" && entry.is_directory === true)
                || typeName === entry.asset_type
            if (typeMatches && (needle.length === 0 || haystack.indexOf(needle) >= 0))
                rows.push(entry)
        }
        rows.sort(function(left, right) {
            if (left.is_directory !== right.is_directory)
                return left.is_directory ? -1 : 1
            var mode = sortMode.currentText
            var a = mode === "Type" ? left.asset_type : (mode === "Size" ? Number(left.bytes || 0) : left.name.toLowerCase())
            var b = mode === "Type" ? right.asset_type : (mode === "Size" ? Number(right.bytes || 0) : right.name.toLowerCase())
            if (typeof a === "number")
                return a === b ? left.name.localeCompare(right.name) : a - b
            var compared = String(a).localeCompare(String(b))
            return compared === 0 ? left.name.localeCompare(right.name) : compared
        })

        fileModel.clear()
        for (var row = 0; row < rows.length; ++row) {
            var item = rows[row]
            fileModel.append({
                "name": item.name,
                "relativePath": item.relative_path,
                "assetType": item.asset_type,
                "directory": item.is_directory === true,
                "editable": item.editable === true,
                "bytes": Number(item.bytes || 0),
                "modifiedMs": Number(item.modified_ms || 0),
                "childCount": Number(item.child_count || 0),
                "previewUrl": String(item.preview_url || ""),
                "selected": selectedPaths.indexOf(item.relative_path) >= 0
            })
        }
        statusText = fileModel.count + " entries · " + selectedPaths.length + " selected"
    }

    function refreshFolders() {
        folderModel.clear()
        if (!editorBridge.projectOpen)
            return
        var folders = parseJson(editorBridge.contentFoldersJson(), [])
        for (var index = 0; index < folders.length; ++index) {
            var folder = folders[index]
            folderModel.append({
                "folderPath": String(folder.path || ""),
                "folderName": String(folder.name || "Project"),
                "folderDepth": Number(folder.depth || 0),
                "assetCount": Number(folder.asset_count || 0),
                "childFolderCount": Number(folder.child_folder_count || 0)
            })
        }
    }

    function refreshEntries() {
        if (!editorBridge.projectOpen) {
            unfilteredEntries = []
            fileModel.clear()
            return
        }
        if (globalSearchActive) {
            var combined = []
            var folders = parseJson(editorBridge.contentFoldersJson(), [])
            for (var folderIndex = 0; folderIndex < folders.length; ++folderIndex) {
                var path = String(folders[folderIndex].path || "")
                var entries = parseJson(editorBridge.contentEntriesJson(path), [])
                for (var entryIndex = 0; entryIndex < entries.length; ++entryIndex) {
                    if (entries[entryIndex].is_directory !== true)
                        combined.push(entries[entryIndex])
                }
            }
            unfilteredEntries = combined
        } else {
            unfilteredEntries = parseJson(editorBridge.contentEntriesJson(currentDirectory), [])
        }
        var retained = []
        for (var index = 0; index < selectedPaths.length; ++index) {
            if (entryByPath(selectedPaths[index]) !== null)
                retained.push(selectedPaths[index])
        }
        selectedPaths = retained
        if (selectedPath.length > 0 && selectedPaths.indexOf(selectedPath) < 0)
            clearSelection()
        applyFilter()
    }

    function refreshAll() {
        refreshFolders()
        refreshEntries()
    }

    function clearSelection() {
        selectedPaths = []
        selectedPath = ""
        selectedType = ""
        selectedPreviewUrl = ""
        selectedEditable = false
        updateSelectedMetadata(null)
        savedText = ""
        textEditor.text = ""
        selectionAnchor = -1
        refreshSelectionRoles()
    }

    function openTextAsset(path, assetType, force) {
        if (!force && editorDirty && path !== selectedPath) {
            statusText = "Save or revert " + selectedPath + " before opening another file"
            return false
        }
        var source = editorBridge.readTextAsset(path)
        if (source.length === 0 && editorBridge.lastError.length > 0) {
            statusText = editorBridge.lastError
            return false
        }
        var entry = entryByPath(path)
        selectedPath = path
        selectedType = assetType
        selectedPreviewUrl = entry ? String(entry.preview_url || "") : ""
        selectedEditable = true
        updateSelectedMetadata(entry)
        savedText = source
        textEditor.text = source
        statusText = "Editing " + path
        return true
    }

    function selectEntry(path, modelIndex, modifiers) {
        var entry = entryByPath(path)
        if (!entry || entry.is_directory)
            return
        var additive = (modifiers & (Qt.ControlModifier | Qt.MetaModifier)) !== 0
        var range = (modifiers & Qt.ShiftModifier) !== 0 && selectionAnchor >= 0
        if (editorDirty && (path !== selectedPath || additive || range)) {
            statusText = "Save or revert " + selectedPath + " before changing selection"
            return
        }
        var next = additive ? selectedPaths.slice(0) : []
        if (range) {
            next = []
            var start = Math.min(selectionAnchor, modelIndex)
            var end = Math.max(selectionAnchor, modelIndex)
            for (var row = start; row <= end; ++row) {
                var rangeEntry = fileModel.get(row)
                if (!rangeEntry.directory)
                    next.push(rangeEntry.relativePath)
            }
        } else {
            var existing = next.indexOf(path)
            if (additive && existing >= 0)
                next.splice(existing, 1)
            else if (existing < 0)
                next.push(path)
            selectionAnchor = modelIndex
        }
        selectedPaths = next
        if (next.length === 1) {
            var selected = entryByPath(next[0])
            selectedPath = next[0]
            selectedType = selected.asset_type
            selectedPreviewUrl = String(selected.preview_url || "")
            selectedEditable = selected.editable === true
            updateSelectedMetadata(selected)
            if (selectedEditable)
                openTextAsset(selectedPath, selectedType, false)
            else {
                savedText = ""
                textEditor.text = ""
            }
        } else {
            selectedPath = next.length > 0 ? next[0] : ""
            selectedType = next.length > 0 ? "Multiple" : ""
            selectedPreviewUrl = ""
            selectedEditable = false
            updateSelectedMetadata(null)
            savedText = ""
            textEditor.text = ""
        }
        refreshSelectionRoles()
        statusText = next.length + (next.length === 1 ? " asset selected" : " assets selected")
    }

    function activateEntry(path, assetType, directory, editable) {
        if (directory) {
            navigateTo(path)
            return
        }
        var index = entryModelIndex(path)
        if (selectedPaths.indexOf(path) < 0)
            selectEntry(path, index, 0)
        if (editable) {
            openTextAsset(path, assetType, false)
            if (assetType === "LuauScript" || assetType === "VisualGraph")
                editorBridge.requestOpenContentAsset(path, assetType)
        } else {
            var entry = entryByPath(path)
            if (entry && String(entry.preview_url || "").length > 0) {
                statusText = "Previewing " + path
            } else if (!editorBridge.openExternalEditor(path, "")) {
                statusText = editorBridge.lastError
            }
        }
    }

    function saveSelected() {
        if (!selectedEditable || !editorDirty)
            return
        if (editorBridge.saveTextAsset(selectedPath, textEditor.text)) {
            savedText = textEditor.text
            statusText = "Saved atomically · " + selectedPath
            refreshEntries()
        } else {
            statusText = editorBridge.lastError
        }
    }

    function runManaged(action, payload) {
        var ok = editorBridge.manageAsset(action, JSON.stringify(payload))
        if (!ok)
            statusText = editorBridge.lastError
        return ok
    }

    function duplicateSelected() {
        if (selectedPaths.length === 0 || busy)
            return
        busy = true
        var completed = 0
        var paths = selectedPaths.slice(0)
        for (var index = 0; index < paths.length; ++index) {
            var payload = {"source": paths[index]}
            if (currentDirectory.length > 0 && parentPath(paths[index]) !== currentDirectory)
                payload.target_folder = currentDirectory
            if (runManaged("duplicate", payload))
                ++completed
        }
        busy = false
        clearSelection()
        refreshAll()
        statusText = "Duplicated " + completed + " of " + paths.length + " assets"
    }

    function movePaths(paths, targetFolder) {
        if (paths.length === 0 || targetFolder.length === 0 || busy)
            return
        busy = true
        var completed = 0
        for (var index = 0; index < paths.length; ++index) {
            if (parentPath(paths[index]) === targetFolder)
                continue
            if (runManaged("move", {"source": paths[index], "target_folder": targetFolder}))
                ++completed
        }
        busy = false
        clearSelection()
        refreshAll()
        statusText = "Moved " + completed + " of " + paths.length + " assets to " + targetFolder
    }

    function trashSelected(force) {
        if (selectedPaths.length === 0 || busy)
            return
        busy = true
        var completed = 0
        var paths = selectedPaths.slice(0)
        for (var index = 0; index < paths.length; ++index) {
            if (runManaged("delete", {"source": paths[index], "confirm": true, "force": force}))
                ++completed
        }
        busy = false
        clearSelection()
        refreshAll()
        statusText = "Moved " + completed + " of " + paths.length + " assets to MiniForge Trash"
    }

    function importUrls(urls) {
        if (!urls || urls.length === 0 || busy)
            return
        busy = true
        var completed = 0
        var target = isManagedAssetPath(currentDirectory) ? currentDirectory : "assets"
        for (var index = 0; index < urls.length; ++index) {
            var source = decodeURIComponent(String(urls[index]).replace(/^file:\/\//, ""))
            if (runManaged("import", {"source_external": source, "target_folder": target}))
                ++completed
        }
        busy = false
        refreshAll()
        statusText = "Imported " + completed + " of " + urls.length + " files into " + target
    }

    function creationKindKey() {
        return assetKindModel.get(assetKind.currentIndex).kindKey
    }

    function creationTypeName() {
        return assetKindModel.get(assetKind.currentIndex).typeName
    }

    ListModel { id: fileModel }
    ListModel { id: folderModel }
    ListModel {
        id: assetKindModel
        ListElement { label: "Luau Script"; kindKey: "luau"; typeName: "LuauScript" }
        ListElement { label: "Scene"; kindKey: "scene"; typeName: "Scene" }
        ListElement { label: "Prefab"; kindKey: "prefab"; typeName: "Prefab" }
        ListElement { label: "JSON Data"; kindKey: "json"; typeName: "Data" }
        ListElement { label: "Resource Config"; kindKey: "config"; typeName: "Data" }
        ListElement { label: "Material"; kindKey: "material"; typeName: "Material" }
        ListElement { label: "Shader"; kindKey: "shader"; typeName: "Shader" }
        ListElement { label: "Visual Graph"; kindKey: "visual_graph"; typeName: "VisualGraph" }
        ListElement { label: "UI Canvas"; kindKey: "ui"; typeName: "UI" }
        ListElement { label: "Tilemap 2D"; kindKey: "tilemap"; typeName: "Tilemap" }
        ListElement { label: "SoundCue"; kindKey: "sound_cue"; typeName: "SoundCue" }
    }

    Connections {
        target: editorBridge
        function onAssetsChanged() {
            if (!root.busy)
                root.refreshAll()
        }
        function onProjectChanged() {
            root.currentDirectory = ""
            root.clearSelection()
            root.refreshAll()
        }
    }

    Component.onCompleted: refreshAll()

    FileDialog {
        id: importDialog
        title: "Import assets into " + root.displayDirectory()
        fileMode: FileDialog.OpenFiles
        nameFilters: ["Supported assets (*.png *.jpg *.jpeg *.webp *.bmp *.wav *.ogg *.mp3 *.flac *.json *.toml *.ron *.yaml *.yml *.wgsl *.glsl *.ttf *.otf *.scene *.prefab *.luau *.lua *.mfgraph *.mfui *.mftilemap)", "All files (*)"]
        onAccepted: root.importUrls(selectedFiles)
    }

    Dialog {
        id: folderDialog
        anchors.centerIn: parent
        width: 360
        modal: true
        title: "Create folder"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            folderName.text = "NewFolder"
            folderName.selectAll()
            folderName.forceActiveFocus()
        }
        onAccepted: {
            if (editorBridge.createContentFolder(root.currentDirectory, folderName.text))
                root.refreshAll()
            else
                root.statusText = editorBridge.lastError
        }
        contentItem: Column {
            spacing: 8
            Text { text: "Parent: " + root.displayDirectory(); color: Theme.DarkTheme.muted; font.pixelSize: 11 }
            TextField { id: folderName; width: parent.width; placeholderText: "Folder name"; selectByMouse: true }
        }
    }

    Dialog {
        id: assetDialog
        anchors.centerIn: parent
        width: 410
        modal: true
        title: "Create project asset"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            assetName.text = "NewAsset"
            assetName.selectAll()
            assetName.forceActiveFocus()
        }
        onAccepted: {
            var path = editorBridge.createContentFile(root.creationKindKey(), root.currentDirectory, assetName.text)
            if (path.length > 0) {
                root.refreshAll()
                root.selectedPaths = [path]
                root.refreshSelectionRoles()
                var created = root.entryByPath(path)
                if (created && created.editable)
                    root.openTextAsset(path, root.creationTypeName(), true)
                root.statusText = "Created " + path
            } else {
                root.statusText = editorBridge.lastError
            }
        }
        contentItem: Column {
            spacing: 8
            Text { text: "Folder: " + root.displayDirectory(); color: Theme.DarkTheme.muted; font.pixelSize: 11 }
            ComboBox { id: assetKind; width: parent.width; model: assetKindModel; textRole: "label" }
            TextField { id: assetName; width: parent.width; placeholderText: "Asset name (extension is automatic)"; selectByMouse: true }
            Text {
                width: parent.width
                text: "At project root, MiniForge selects the conventional folder for this asset type."
                color: Theme.DarkTheme.muted
                font.pixelSize: 10
                wrapMode: Text.Wrap
            }
        }
    }

    Dialog {
        id: renameDialog
        anchors.centerIn: parent
        width: 380
        modal: true
        title: "Rename asset"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            var entry = root.entryByPath(root.selectedPath)
            renameName.text = entry ? entry.name : ""
            renameName.selectAll()
            renameName.forceActiveFocus()
        }
        onAccepted: {
            var oldPath = root.selectedPath
            if (root.runManaged("rename", {"source": oldPath, "new_name": renameName.text.trim()})) {
                root.clearSelection()
                root.refreshAll()
                root.statusText = "Renamed " + oldPath
            }
        }
        contentItem: Column {
            spacing: 8
            Text { text: root.selectedPath; color: Theme.DarkTheme.muted; font.pixelSize: 10; elide: Text.ElideMiddle; width: parent.width }
            TextField { id: renameName; width: parent.width; placeholderText: "New asset name"; selectByMouse: true }
        }
    }

    Dialog {
        id: moveDialog
        anchors.centerIn: parent
        width: 420
        modal: true
        title: "Move selected assets"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            var preferred = root.currentDirectory.length > 0 ? root.currentDirectory : "assets"
            var index = 0
            for (var row = 0; row < folderModel.count; ++row) {
                if (folderModel.get(row).folderPath === preferred) {
                    index = row
                    break
                }
            }
            moveTarget.currentIndex = index
        }
        onAccepted: {
            var target = folderModel.get(moveTarget.currentIndex).folderPath
            if (!root.isManagedAssetPath(target)) {
                root.statusText = "Choose an assets, scripts, scenes, saves, settings, components, systems, plugins or templates folder"
                return
            }
            root.movePaths(root.selectedPaths.slice(0), target)
        }
        contentItem: Column {
            spacing: 8
            Text { text: root.selectedPaths.length + " selected assets"; color: Theme.DarkTheme.muted; font.pixelSize: 11 }
            ComboBox {
                id: moveTarget
                width: parent.width
                model: folderModel
                textRole: "folderPath"
                displayText: currentIndex >= 0 && folderModel.get(currentIndex).folderPath.length > 0
                    ? folderModel.get(currentIndex).folderPath : "Project root (not movable)"
            }
        }
    }

    Dialog {
        id: deleteDialog
        anchors.centerIn: parent
        width: 420
        modal: true
        title: "Move assets to Trash?"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: forceDelete.checked = false
        onAccepted: root.trashSelected(forceDelete.checked)
        contentItem: Column {
            spacing: 9
            Text {
                width: parent.width
                text: root.selectedPaths.length + " assets will be moved into .miniforge/trash and can be recovered manually."
                color: Theme.DarkTheme.text
                wrapMode: Text.Wrap
            }
            CheckBox { id: forceDelete; text: "Force assets that still have references" }
        }
    }

    Menu {
        id: assetMenu
        MenuItem { text: "Open"; enabled: root.selectedPaths.length === 1; onTriggered: {
            var entry = root.entryByPath(root.selectedPath)
            if (entry) root.activateEntry(root.selectedPath, entry.asset_type, false, entry.editable)
        } }
        MenuItem { text: "Open in system editor"; enabled: root.selectedPaths.length === 1; onTriggered: editorBridge.openExternalEditor(root.selectedPath, "") }
        MenuSeparator {}
        MenuItem { text: "Rename"; enabled: root.selectedPaths.length === 1 && root.selectionIsManageable(); onTriggered: renameDialog.open() }
        MenuItem { text: "Duplicate"; enabled: root.selectionIsManageable(); onTriggered: root.duplicateSelected() }
        MenuItem { text: "Move to..."; enabled: root.selectionIsManageable(); onTriggered: moveDialog.open() }
        MenuSeparator {}
        MenuItem { text: "Move to Trash"; enabled: root.selectionIsManageable(); onTriggered: deleteDialog.open() }
    }

    Menu {
        id: folderMenu
        MenuItem { text: "Open folder"; onTriggered: root.navigateTo(root.contextFolderPath) }
        MenuSeparator {}
        MenuItem { text: "Create folder here"; onTriggered: { root.navigateTo(root.contextFolderPath); folderDialog.open() } }
        MenuItem { text: "Create asset here"; enabled: !root.editorDirty; onTriggered: { root.navigateTo(root.contextFolderPath); assetDialog.open() } }
        MenuItem { text: "Import here"; enabled: root.isManagedAssetPath(root.contextFolderPath); onTriggered: { root.navigateTo(root.contextFolderPath); importDialog.open() } }
    }

    Shortcut { sequences: [StandardKey.Save]; enabled: root.editorDirty; onActivated: root.saveSelected() }
    Shortcut { sequence: "Ctrl+D"; enabled: root.selectionIsManageable(); onActivated: root.duplicateSelected() }
    Shortcut { sequence: "F2"; enabled: root.selectedPaths.length === 1 && root.selectionIsManageable(); onActivated: renameDialog.open() }
    Shortcut { sequences: [StandardKey.Delete]; enabled: root.selectionIsManageable(); onActivated: deleteDialog.open() }

    Timer {
        id: searchRefreshTimer
        interval: 120
        repeat: false
        onTriggered: root.refreshEntries()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 6

        MfPanelHeader {
            Layout.fillWidth: true
            title: "Content Browser"
            detail: root.displayDirectory() + " · " + fileModel.count + " visible"
            badge: root.busy ? "Working" : (root.editorDirty ? "Dirty" : "Assets")
            badgeColor: root.busy || root.editorDirty ? Theme.DarkTheme.warning : Theme.DarkTheme.info
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            MfButton { text: "Up"; enabled: root.currentDirectory.length > 0; onClicked: root.parentDirectory() }
            MfSearchBar {
                id: contentSearch
                Layout.fillWidth: true
                placeholderText: "Search all content by name, type or path"
                onTextChanged: searchRefreshTimer.restart()
            }
            MfButton { text: "Import"; enabled: editorBridge.projectOpen && !root.busy; onClicked: importDialog.open() }
            MfButton { text: "Folder +"; enabled: editorBridge.projectOpen && !root.busy; onClicked: folderDialog.open() }
            MfButton { text: "Asset +"; accent: true; enabled: editorBridge.projectOpen && !root.busy && !root.editorDirty; onClicked: assetDialog.open() }
            MfButton { text: "Refresh"; enabled: !root.busy; onClicked: root.refreshAll() }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 30
            radius: Theme.DarkTheme.radius
            color: Theme.DarkTheme.background
            border.color: Theme.DarkTheme.borderSoft

            Row {
                anchors.fill: parent
                anchors.leftMargin: 5
                anchors.rightMargin: 5
                spacing: 2
                Repeater {
                    model: root.breadcrumbs()
                    delegate: Row {
                        required property int index
                        required property var modelData
                        spacing: 2
                        Text { text: index > 0 ? "/" : ""; color: Theme.DarkTheme.muted; anchors.verticalCenter: parent.verticalCenter }
                        Button {
                            id: breadcrumbButton
                            flat: true
                            height: 28
                            text: modelData.label
                            onClicked: root.navigateTo(modelData.path)
                            contentItem: Text {
                                text: breadcrumbButton.text
                                color: modelData.path === root.currentDirectory ? Theme.DarkTheme.accent : Theme.DarkTheme.text
                                font.pixelSize: 11
                                font.bold: modelData.path === root.currentDirectory
                                verticalAlignment: Text.AlignVCenter
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 6

            Rectangle {
                Layout.preferredWidth: 188
                Layout.minimumWidth: 150
                Layout.fillHeight: true
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 5
                    spacing: 4
                    Text { text: "FOLDERS"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true; Layout.leftMargin: 5 }
                    ListView {
                        id: folderTree
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        spacing: 1
                        model: folderModel
                        delegate: Rectangle {
                            id: folderRow
                            required property string folderPath
                            required property string folderName
                            required property int folderDepth
                            required property int assetCount
                            required property int childFolderCount
                            width: ListView.view.width
                            height: 27
                            radius: 4
                            color: root.currentDirectory === folderPath
                                ? Theme.DarkTheme.accentSoft
                                : (folderMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent")
                            border.color: folderDrop.containsDrag ? Theme.DarkTheme.accent : "transparent"

                            Row {
                                anchors.fill: parent
                                anchors.leftMargin: 6 + Math.min(8, folderRow.folderDepth) * 11
                                anchors.rightMargin: 5
                                spacing: 6
                                Text { text: folderRow.childFolderCount > 0 ? "▾" : "·"; color: Theme.DarkTheme.muted; width: 9; anchors.verticalCenter: parent.verticalCenter }
                                Rectangle { width: 14; height: 11; radius: 2; color: Theme.DarkTheme.warning; opacity: 0.85; anchors.verticalCenter: parent.verticalCenter }
                                Text { text: folderRow.folderName; color: Theme.DarkTheme.text; font.pixelSize: 10; width: parent.width - 58; elide: Text.ElideRight; anchors.verticalCenter: parent.verticalCenter }
                                Text { text: folderRow.assetCount; color: Theme.DarkTheme.muted; font.pixelSize: 9; anchors.verticalCenter: parent.verticalCenter }
                            }

                            MouseArea {
                                id: folderMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                onClicked: function(mouse) {
                                    if (mouse.button === Qt.RightButton) {
                                        root.contextFolderPath = folderRow.folderPath
                                        folderMenu.popup()
                                        return
                                    }
                                    root.navigateTo(folderRow.folderPath)
                                }
                            }
                            DropArea {
                                id: folderDrop
                                anchors.fill: parent
                                enabled: root.isManagedAssetPath(folderRow.folderPath)
                                keys: ["MiniForgeAsset"]
                                onDropped: function(drop) {
                                    if (folderRow.folderPath.length === 0)
                                        return
                                    var encoded = drop.getDataAsString("application/x-miniforge-assets")
                                    var paths = root.parseJson(encoded, [])
                                    root.movePaths(paths, folderRow.folderPath)
                                    drop.acceptProposedAction()
                                }
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 260
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 5
                    spacing: 4

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 5
                        ComboBox { id: typeFilter; model: ["All", "Folders", "LuauScript", "Scene", "Prefab", "Texture", "Audio", "SoundCue", "Material", "Shader", "VisualGraph", "UI", "Tilemap", "Data"]; onCurrentTextChanged: root.applyFilter() }
                        ComboBox { id: sortMode; model: ["Name", "Type", "Size"]; onCurrentTextChanged: root.applyFilter() }
                        Item { Layout.fillWidth: true }
                        Button { text: "Grid"; checkable: true; checked: root.gridView; onClicked: root.gridView = true }
                        Button { text: "List"; checkable: true; checked: !root.gridView; onClicked: root.gridView = false }
                    }

                    StackLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        currentIndex: root.gridView ? 0 : 1

                        GridView {
                            id: assetGrid
                            clip: true
                            model: fileModel
                            cellWidth: 112
                            cellHeight: 108
                            boundsBehavior: Flickable.StopAtBounds
                            delegate: Item {
                                id: gridCell
                                required property int index
                                required property string name
                                required property string relativePath
                                required property string assetType
                                required property bool directory
                                required property bool editable
                                required property double bytes
                                required property double modifiedMs
                                required property int childCount
                                required property string previewUrl
                                required property bool selected
                                width: GridView.view.cellWidth - 4
                                height: GridView.view.cellHeight - 4
                                Drag.active: gridDrag.active && !gridCell.directory && root.isManagedAssetPath(gridCell.relativePath)
                                Drag.dragType: Drag.Automatic
                                Drag.keys: ["MiniForgeAsset"]
                                Drag.supportedActions: Qt.MoveAction | Qt.CopyAction
                                Drag.mimeData: ({
                                    "application/x-miniforge-assets": JSON.stringify(root.selectedPaths.indexOf(gridCell.relativePath) >= 0 && root.selectionIsManageable() ? root.selectedPaths : [gridCell.relativePath]),
                                    "application/x-miniforge-asset": gridCell.relativePath,
                                    "text/plain": gridCell.relativePath
                                })

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 5
                                    color: gridCell.selected ? Theme.DarkTheme.accentSoft
                                        : (gridMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : Theme.DarkTheme.background)
                                    border.color: gridCell.selected ? Theme.DarkTheme.accent : Theme.DarkTheme.borderSoft

                                    Column {
                                        anchors.fill: parent
                                        anchors.margins: 5
                                        spacing: 3
                                        Rectangle {
                                            width: parent.width
                                            height: 66
                                            radius: 4
                                            color: Theme.DarkTheme.panelAlt
                                            clip: true
                                            Image {
                                                anchors.fill: parent
                                                anchors.margins: 3
                                                source: gridCell.previewUrl
                                                fillMode: Image.PreserveAspectFit
                                                asynchronous: true
                                                visible: gridCell.previewUrl.length > 0 && status === Image.Ready
                                            }
                                            Text {
                                                anchors.centerIn: parent
                                                text: root.iconLabel(gridCell.assetType, gridCell.directory)
                                                color: root.iconColor(gridCell.assetType, gridCell.directory)
                                                font.pixelSize: gridCell.directory ? 14 : 11
                                                font.bold: true
                                                visible: gridCell.previewUrl.length === 0 || parent.children[0].status !== Image.Ready
                                            }
                                            Text {
                                                anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 3
                                                text: gridCell.directory ? gridCell.childCount : (gridCell.editable ? "TEXT" : "")
                                                color: Theme.DarkTheme.muted; font.pixelSize: 7
                                            }
                                        }
                                        Text { width: parent.width; text: gridCell.name; color: Theme.DarkTheme.text; font.pixelSize: 10; font.bold: true; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideMiddle }
                                        Text { width: parent.width; text: gridCell.directory ? "Folder" : gridCell.assetType; color: Theme.DarkTheme.muted; font.pixelSize: 8; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight }
                                    }
                                }

                                MouseArea {
                                    id: gridMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                                    onClicked: function(mouse) {
                                        if (mouse.button === Qt.RightButton) {
                                            if (!gridCell.selected && !gridCell.directory)
                                                root.selectEntry(gridCell.relativePath, index, 0)
                                            if (!gridCell.directory)
                                                assetMenu.popup()
                                            return
                                        }
                                        if (gridCell.directory)
                                            root.statusText = "Double-click to open " + gridCell.relativePath
                                        else
                                            root.selectEntry(gridCell.relativePath, index, mouse.modifiers)
                                    }
                                    onDoubleClicked: root.activateEntry(gridCell.relativePath, gridCell.assetType, gridCell.directory, gridCell.editable)
                                }
                                DragHandler { id: gridDrag; target: null; enabled: !gridCell.directory && root.isManagedAssetPath(gridCell.relativePath) }
                            }
                            Text {
                                visible: assetGrid.count === 0
                                anchors.centerIn: parent
                                text: editorBridge.projectOpen ? "No assets match this view" : "Open a project to browse content"
                                color: Theme.DarkTheme.muted
                                font.pixelSize: 11
                            }
                        }

                        ListView {
                            id: assetList
                            clip: true
                            model: fileModel
                            spacing: 2
                            boundsBehavior: Flickable.StopAtBounds
                            delegate: Rectangle {
                                id: listRow
                                required property int index
                                required property string name
                                required property string relativePath
                                required property string assetType
                                required property bool directory
                                required property bool editable
                                required property double bytes
                                required property double modifiedMs
                                required property int childCount
                                required property string previewUrl
                                required property bool selected
                                width: ListView.view.width
                                height: 39
                                radius: 4
                                color: listRow.selected ? Theme.DarkTheme.accentSoft
                                    : (listMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent")
                                border.color: listRow.selected ? Theme.DarkTheme.accent : "transparent"
                                Drag.active: listDrag.active && !listRow.directory && root.isManagedAssetPath(listRow.relativePath)
                                Drag.dragType: Drag.Automatic
                                Drag.keys: ["MiniForgeAsset"]
                                Drag.supportedActions: Qt.MoveAction | Qt.CopyAction
                                Drag.mimeData: ({
                                    "application/x-miniforge-assets": JSON.stringify(root.selectedPaths.indexOf(listRow.relativePath) >= 0 && root.selectionIsManageable() ? root.selectedPaths : [listRow.relativePath]),
                                    "application/x-miniforge-asset": listRow.relativePath,
                                    "text/plain": listRow.relativePath
                                })

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.margins: 5
                                    spacing: 7
                                    Rectangle {
                                        Layout.preferredWidth: 29; Layout.preferredHeight: 29; radius: 4; color: Theme.DarkTheme.panelAlt; clip: true
                                        Image { anchors.fill: parent; anchors.margins: 2; source: listRow.previewUrl; fillMode: Image.PreserveAspectFit; asynchronous: true; visible: listRow.previewUrl.length > 0 && status === Image.Ready }
                                        Text { anchors.centerIn: parent; text: root.iconLabel(listRow.assetType, listRow.directory); color: root.iconColor(listRow.assetType, listRow.directory); font.pixelSize: 7; font.bold: true; visible: listRow.previewUrl.length === 0 || parent.children[0].status !== Image.Ready }
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 0
                                        Text { Layout.fillWidth: true; text: listRow.name; color: Theme.DarkTheme.text; font.pixelSize: 10; font.bold: true; elide: Text.ElideRight }
                                        Text { Layout.fillWidth: true; text: listRow.relativePath; color: Theme.DarkTheme.muted; font.pixelSize: 8; elide: Text.ElideMiddle }
                                    }
                                    Text { Layout.preferredWidth: 82; text: listRow.directory ? listRow.childCount + " items" : listRow.assetType; color: Theme.DarkTheme.muted; font.pixelSize: 9; horizontalAlignment: Text.AlignRight; elide: Text.ElideRight }
                                    Text { Layout.preferredWidth: 60; text: listRow.directory ? "" : root.formatBytes(listRow.bytes); color: Theme.DarkTheme.muted; font.pixelSize: 9; horizontalAlignment: Text.AlignRight }
                                }
                                MouseArea {
                                    id: listMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                                    onClicked: function(mouse) {
                                        if (mouse.button === Qt.RightButton) {
                                            if (!listRow.selected && !listRow.directory)
                                                root.selectEntry(listRow.relativePath, index, 0)
                                            if (!listRow.directory)
                                                assetMenu.popup()
                                            return
                                        }
                                        if (listRow.directory)
                                            root.statusText = "Double-click to open " + listRow.relativePath
                                        else
                                            root.selectEntry(listRow.relativePath, index, mouse.modifiers)
                                    }
                                    onDoubleClicked: root.activateEntry(listRow.relativePath, listRow.assetType, listRow.directory, listRow.editable)
                                }
                                DragHandler { id: listDrag; target: null; enabled: !listRow.directory && root.isManagedAssetPath(listRow.relativePath) }
                            }
                        }
                    }
                }
            }

            Rectangle {
                visible: root.selectedPaths.length > 0
                Layout.preferredWidth: visible ? Math.max(260, root.width * 0.28) : 0
                Layout.minimumWidth: visible ? 220 : 0
                Layout.fillHeight: true
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.background
                border.color: root.editorDirty ? Theme.DarkTheme.warning : Theme.DarkTheme.borderSoft

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 7
                    spacing: 5

                    RowLayout {
                        Layout.fillWidth: true
                        Text { Layout.fillWidth: true; text: root.selectedPaths.length === 1 ? "ASSET PREVIEW" : "MULTI-SELECTION"; color: Theme.DarkTheme.muted; font.pixelSize: 9; font.bold: true }
                        MfButton { text: "Save"; visible: root.selectedEditable; enabled: root.editorDirty; accent: root.editorDirty; onClicked: root.saveSelected() }
                    }
                    Text { Layout.fillWidth: true; text: root.selectedPaths.length === 1 ? root.selectedPath : root.selectedPaths.length + " assets"; color: Theme.DarkTheme.text; font.pixelSize: 11; font.bold: true; elide: Text.ElideMiddle }
                    Text { Layout.fillWidth: true; text: root.selectedType; color: Theme.DarkTheme.accent; font.pixelSize: 9; elide: Text.ElideRight }

                    Rectangle {
                        visible: root.selectedPaths.length === 1 && root.selectedGuid.length > 0
                        Layout.fillWidth: true
                        Layout.preferredHeight: visible ? Math.min(190, metadataColumn.implicitHeight + 14) : 0
                        radius: Theme.DarkTheme.radius
                        color: Theme.DarkTheme.surface
                        border.color: root.selectedWarnings.length > 0
                            ? Theme.DarkTheme.warning : Theme.DarkTheme.borderSoft

                        ScrollView {
                            anchors.fill: parent
                            anchors.margins: 7
                            clip: true

                            Column {
                                id: metadataColumn
                                width: Math.max(1, parent.width)
                                spacing: 4

                                Row {
                                    width: parent.width
                                    spacing: 6
                                    Text {
                                        width: Math.max(1, parent.width - buildBadge.width - 6)
                                        text: "GUID  " + root.selectedGuid
                                        color: Theme.DarkTheme.muted
                                        font.pixelSize: 8
                                        elide: Text.ElideMiddle
                                    }
                                    Rectangle {
                                        id: buildBadge
                                        width: buildLabel.implicitWidth + 10
                                        height: 17
                                        radius: 8
                                        color: root.selectedIncludeInBuild
                                            ? Theme.DarkTheme.accentSoft : Theme.DarkTheme.panelAlt
                                        Text {
                                            id: buildLabel
                                            anchors.centerIn: parent
                                            text: root.selectedIncludeInBuild ? "IN BUILD" : "EDITOR ONLY"
                                            color: root.selectedIncludeInBuild
                                                ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
                                            font.pixelSize: 7
                                            font.bold: true
                                        }
                                    }
                                }
                                Text {
                                    width: parent.width
                                    text: "Labels  " + root.joinedMetadata(root.selectedLabels, "none")
                                    color: Theme.DarkTheme.text
                                    font.pixelSize: 8
                                    wrapMode: Text.Wrap
                                }
                                Text {
                                    width: parent.width
                                    text: "Depends on  " + root.joinedMetadata(root.selectedDependencies, "none")
                                    color: root.selectedDependencies.length > 0
                                        ? Theme.DarkTheme.info : Theme.DarkTheme.muted
                                    font.pixelSize: 8
                                    wrapMode: Text.Wrap
                                }
                                Text {
                                    width: parent.width
                                    text: "Used by  " + root.joinedMetadata(root.selectedReverseDependencies, "none")
                                    color: root.selectedReverseDependencies.length > 0
                                        ? Theme.DarkTheme.accent : Theme.DarkTheme.muted
                                    font.pixelSize: 8
                                    wrapMode: Text.Wrap
                                }
                                Text {
                                    visible: root.selectedWarnings.length > 0
                                    width: parent.width
                                    text: "Warnings  " + root.joinedMetadata(root.selectedWarnings, "")
                                    color: Theme.DarkTheme.warning
                                    font.pixelSize: 8
                                    wrapMode: Text.Wrap
                                }
                            }
                        }
                    }

                    Image {
                        visible: root.selectedPreviewUrl.length > 0 && !root.selectedEditable
                        Layout.fillWidth: true
                        Layout.preferredHeight: visible ? Math.min(170, implicitHeight) : 0
                        source: root.selectedPreviewUrl
                        fillMode: Image.PreserveAspectFit
                        asynchronous: true
                    }

                    ScrollView {
                        visible: root.selectedEditable
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        TextArea {
                            id: textEditor
                            width: Math.max(parent.width, implicitWidth)
                            height: Math.max(parent.height, contentHeight + 18)
                            textFormat: TextEdit.PlainText
                            wrapMode: TextEdit.NoWrap
                            selectByMouse: true
                            tabStopDistance: 32
                            color: Theme.DarkTheme.text
                            selectionColor: Theme.DarkTheme.accent
                            selectedTextColor: Theme.DarkTheme.background
                            font.family: "Menlo"
                            font.pixelSize: 10
                            leftPadding: 8; rightPadding: 8; topPadding: 7; bottomPadding: 7
                            background: Rectangle { color: Theme.DarkTheme.surface }
                            onTextChanged: {
                                if (root.editorDirty)
                                    root.statusText = "Unsaved changes · " + root.selectedPath
                            }
                        }
                    }

                    Item { visible: !root.selectedEditable; Layout.fillHeight: visible }
                    RowLayout {
                        Layout.fillWidth: true
                        MfButton {
                            text: "Open"
                            enabled: root.selectedPaths.length === 1
                            onClicked: {
                                var entry = root.entryByPath(root.selectedPath)
                                if (entry) root.activateEntry(root.selectedPath, entry.asset_type, false, entry.editable)
                            }
                        }
                        MfButton { text: "System"; enabled: root.selectedPaths.length === 1; onClicked: editorBridge.openExternalEditor(root.selectedPath, "") }
                        Item { Layout.fillWidth: true }
                        MfButton { text: "..."; onClicked: assetMenu.popup() }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 22
            radius: 3
            color: Theme.DarkTheme.background
            Text {
                anchors.fill: parent
                anchors.leftMargin: 7
                anchors.rightMargin: 7
                text: root.statusText
                color: editorBridge.lastError.length > 0 && root.statusText === editorBridge.lastError
                    ? Theme.DarkTheme.danger : Theme.DarkTheme.muted
                font.pixelSize: 9
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }
    }
}
