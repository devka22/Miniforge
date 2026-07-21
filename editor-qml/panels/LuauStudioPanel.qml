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
    property int diagnosticLine: 0
    property int diagnosticColumn: 0
    property var apiEntries: []
    property string completionPrefix: ""
    property string apiDocSignature: "Select an API symbol"
    property string apiDocDetail: "Type a MiniForge API name or browse the reference."
    property string apiFilter: ""
    property var debugState: ({"paused": false, "breakpoints": []})
    property bool findVisible: false
    property bool replaceVisible: false
    property string findStatus: ""
    property bool minimapEnabled: true
    property bool formatOnSave: false
    property bool autoOpenExternal: false
    property string externalEditorCommand: ""
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

    function tabIndex(path) {
        for (var index = 0; index < tabModel.count; ++index) {
            if (tabModel.get(index).path === path)
                return index
        }
        return -1
    }

    function checkpointCurrentTab() {
        var index = tabIndex(currentPath)
        if (index < 0)
            return
        tabModel.setProperty(index, "source", codeEditor.text)
        tabModel.setProperty(index, "saved", savedSource)
        tabModel.setProperty(index, "dirty", codeEditor.text !== savedSource)
    }

    function openScript(path, force) {
        checkpointCurrentTab()
        var index = tabIndex(path)
        var source = ""
        var diskSource = ""
        if (index >= 0 && !force) {
            source = tabModel.get(index).source
            diskSource = tabModel.get(index).saved
        } else {
            source = editorBridge.readLuauScript(path)
            if (source.length === 0 && editorBridge.lastError.length > 0) {
                diagnostic = editorBridge.lastError
                sourceValid = false
                return
            }
            diskSource = source
            if (index < 0) {
                tabModel.append({"path": path, "name": path.split("/").pop(), "source": source, "saved": source, "dirty": false})
            } else {
                tabModel.setProperty(index, "source", source)
                tabModel.setProperty(index, "saved", source)
                tabModel.setProperty(index, "dirty", false)
            }
        }
        currentPath = path
        savedSource = diskSource
        codeEditor.text = source
        diagnostic = "Loaded " + path
        sourceValid = true
        diagnosticLine = 0
        diagnosticColumn = 0
        codeEditor.forceActiveFocus()
        outlineTimer.restart()
        refreshHighlighter()
        rebuildCodeActions()
        scheduleWorkspaceSave()
        if (autoOpenExternal && !editorBridge.openExternalEditor(path, externalEditorCommand))
            diagnostic = editorBridge.lastError
    }

    function lineForPosition(position) {
        if (position <= 0)
            return 1
        var line = 1
        var limit = Math.min(position, codeEditor.text.length)
        for (var index = 0; index < limit; ++index) {
            if (codeEditor.text.charAt(index) === "\n")
                ++line
        }
        return line
    }

    function goToLine(line) {
        if (currentPath.length === 0)
            return
        var target = Math.max(1, Math.min(Number(line) || 1, codeEditor.lineCount))
        var start = 0
        for (var current = 1; current < target; ++current) {
            var newline = codeEditor.text.indexOf("\n", start)
            if (newline < 0)
                break
            start = newline + 1
        }
        var end = codeEditor.text.indexOf("\n", start)
        if (end < 0)
            end = codeEditor.text.length
        codeEditor.select(start, end)
        codeEditor.forceActiveFocus()
        diagnostic = "Line " + target
        refreshHighlighter()
    }

    function rebuildOutline() {
        var symbols = []
        var declaration = /(?:^|\n)\s*(?:local\s+)?function\s+([A-Za-z_][A-Za-z0-9_.:]*)\s*\(/g
        var assignment = /(?:^|\n)\s*(?:local\s+)?([A-Za-z_][A-Za-z0-9_.:]*)\s*=\s*function\s*\(/g
        var match
        while ((match = declaration.exec(codeEditor.text)) !== null) {
            var declarationPosition = match.index + match[0].indexOf(match[1])
            symbols.push({"name": match[1], "line": lineForPosition(declarationPosition)})
        }
        while ((match = assignment.exec(codeEditor.text)) !== null) {
            var assignmentPosition = match.index + match[0].indexOf(match[1])
            symbols.push({"name": match[1], "line": lineForPosition(assignmentPosition)})
        }
        symbols.sort(function(left, right) { return left.line - right.line })
        outlineModel.clear()
        var seen = ({})
        for (var index = 0; index < symbols.length; ++index) {
            var key = symbols[index].line + ":" + symbols[index].name
            if (!seen[key]) {
                seen[key] = true
                outlineModel.append(symbols[index])
            }
        }
        refreshHighlighter()
    }

    function outlineLineForFunction(functionName) {
        for (var index = 0; index < outlineModel.count; ++index) {
            var symbol = outlineModel.get(index)
            if (symbol.name === functionName
                    || symbol.name.endsWith(":" + functionName)
                    || symbol.name.endsWith("." + functionName))
                return symbol.line
        }
        return 0
    }

    function breakpointHighlightLines() {
        var lines = []
        for (var index = 0; index < breakpointModel.count; ++index) {
            var breakpoint = breakpointModel.get(index)
            if (!breakpoint.active || breakpoint.path !== currentPath)
                continue
            var line = breakpoint.line > 0
                ? breakpoint.line
                : outlineLineForFunction(breakpoint.functionName)
            if (line > 0 && lines.indexOf(line) < 0)
                lines.push(line)
        }
        return lines
    }

    function refreshHighlighter() {
        if (!luauSyntaxHighlighter)
            return
        luauSyntaxHighlighter.diagnosticLine = sourceValid ? 0 : diagnosticLine
        luauSyntaxHighlighter.currentLine = currentPath.length > 0
            ? lineForPosition(codeEditor.cursorPosition)
            : 0
        luauSyntaxHighlighter.breakpointLines = breakpointHighlightLines()
    }

    function showFind(includeReplace) {
        findVisible = true
        replaceVisible = includeReplace
        findField.forceActiveFocus()
        findField.selectAll()
    }

    function findNext(reverse) {
        var query = findField.text
        if (query.length === 0 || currentPath.length === 0) {
            findStatus = "Enter text to find"
            return false
        }
        var source = codeEditor.text
        var haystack = findCaseSensitive.checked ? source : source.toLowerCase()
        var needle = findCaseSensitive.checked ? query : query.toLowerCase()
        var position = -1
        if (reverse) {
            var before = Math.max(0, codeEditor.selectionStart - 1)
            position = haystack.lastIndexOf(needle, before)
            if (position < 0)
                position = haystack.lastIndexOf(needle)
        } else {
            var after = Math.max(codeEditor.cursorPosition, codeEditor.selectionEnd)
            position = haystack.indexOf(needle, after)
            if (position < 0)
                position = haystack.indexOf(needle)
        }
        if (position < 0) {
            findStatus = "No matches"
            return false
        }
        codeEditor.select(position, position + query.length)
        codeEditor.forceActiveFocus()
        findStatus = "Line " + lineForPosition(position)
        refreshHighlighter()
        return true
    }

    function replaceCurrent() {
        var query = findField.text
        if (query.length === 0)
            return
        var selected = codeEditor.selectedText
        var matches = findCaseSensitive.checked
            ? selected === query
            : selected.toLowerCase() === query.toLowerCase()
        if (!matches) {
            findNext(false)
            return
        }
        var start = codeEditor.selectionStart
        codeEditor.remove(start, codeEditor.selectionEnd)
        codeEditor.insert(start, replaceField.text)
        codeEditor.cursorPosition = start + replaceField.text.length
        findStatus = "Replaced one match"
        findNext(false)
    }

    function replaceAll() {
        var query = findField.text
        if (query.length === 0)
            return
        var source = codeEditor.text
        var haystack = findCaseSensitive.checked ? source : source.toLowerCase()
        var needle = findCaseSensitive.checked ? query : query.toLowerCase()
        var output = ""
        var cursor = 0
        var count = 0
        var position = haystack.indexOf(needle, cursor)
        while (position >= 0) {
            output += source.substring(cursor, position) + replaceField.text
            cursor = position + query.length
            ++count
            position = haystack.indexOf(needle, cursor)
        }
        if (count > 0) {
            output += source.substring(cursor)
            codeEditor.text = output
        }
        findStatus = count + (count === 1 ? " replacement" : " replacements")
    }

    function currentLineBounds() {
        var start = codeEditor.text.lastIndexOf("\n", Math.max(0, codeEditor.cursorPosition - 1)) + 1
        var end = codeEditor.text.indexOf("\n", codeEditor.cursorPosition)
        if (end < 0)
            end = codeEditor.text.length
        return {"start": start, "end": end}
    }

    function duplicateCurrentLine() {
        if (currentPath.length === 0)
            return
        var bounds = currentLineBounds()
        var line = codeEditor.text.substring(bounds.start, bounds.end)
        codeEditor.insert(bounds.end, "\n" + line)
        codeEditor.cursorPosition = bounds.end + 1 + line.length
        diagnostic = "Duplicated current line"
    }

    function toggleCurrentLineComment() {
        if (currentPath.length === 0)
            return
        var bounds = currentLineBounds()
        var line = codeEditor.text.substring(bounds.start, bounds.end)
        var leading = line.match(/^\s*/)[0]
        var body = line.substring(leading.length)
        var replacement = body.indexOf("--") === 0
            ? leading + body.substring(2).replace(/^\s/, "")
            : leading + "-- " + body
        codeEditor.remove(bounds.start, bounds.end)
        codeEditor.insert(bounds.start, replacement)
        codeEditor.cursorPosition = bounds.start + String(replacement).length
        diagnostic = "Toggled line comment"
    }

    function formatDocument() {
        if (currentPath.length === 0)
            return
        var lines = codeEditor.text.replace(/\r\n/g, "\n").split("\n")
        var indent = 0
        var output = []
        for (var index = 0; index < lines.length; ++index) {
            var content = lines[index].replace(/[ \t]+$/g, "").replace(/^\s+/, "")
            if (/^(end\b|else\b|elseif\b|until\b)/.test(content))
                indent = Math.max(0, indent - 1)
            output.push(content.length > 0 ? "    ".repeat(indent) + content : "")
            if (/\b(function|then|do)\s*(?:--.*)?$/.test(content)
                    || /^(repeat|else\b|elseif\b.*\bthen)\s*(?:--.*)?$/.test(content))
                ++indent
        }
        var formatted = output.join("\n")
        if (codeEditor.text.endsWith("\n") && !formatted.endsWith("\n"))
            formatted += "\n"
        codeEditor.text = formatted
        diagnostic = "Formatted document · 4-space indentation"
        rebuildCodeActions()
    }

    function rebuildCodeActions() {
        codeActionModel.clear()
        if (currentPath.length === 0)
            return
        var source = codeEditor.text
        if (!/^--!strict\b/.test(source))
            codeActionModel.append({"label": "Enable strict mode", "kind": "strict"})
        if (/\bon_start\b/.test(source))
            codeActionModel.append({"label": "Use on_ready", "kind": "on_ready"})
        if (/\bInput\.get_axis\b/.test(source))
            codeActionModel.append({"label": "Use Input.axis", "kind": "input_axis"})
        if (/\bTime\.deltaTime\b/.test(source))
            codeActionModel.append({"label": "Fix Time.delta_time", "kind": "delta_time"})
        if (!sourceValid && /expected.*end|missing.*end/i.test(diagnostic))
            codeActionModel.append({"label": "Append missing end", "kind": "append_end"})
    }

    function applyCodeAction(kind) {
        if (kind === "strict")
            codeEditor.text = "--!strict\n\n" + codeEditor.text
        else if (kind === "on_ready")
            codeEditor.text = codeEditor.text.replace(/\bon_start\b/g, "on_ready")
        else if (kind === "input_axis")
            codeEditor.text = codeEditor.text.replace(/\bInput\.get_axis\b/g, "Input.axis")
        else if (kind === "delta_time")
            codeEditor.text = codeEditor.text.replace(/\bTime\.deltaTime\b/g, "Time.delta_time")
        else if (kind === "append_end")
            codeEditor.text = codeEditor.text.replace(/\s*$/, "\nend\n")
        rebuildCodeActions()
        codeEditor.forceActiveFocus()
    }

    function insertProductivitySnippet(kind) {
        var snippets = {
            "update": "\nfunction Script:on_update(dt: number)\n    -- Frame-rate independent logic.\nend\n",
            "log": "Debug.log(\"Gameplay event\")",
            "spawn": "local spawned = Entity.spawn(\"EntityName\", 0, 0, { tag = \"Gameplay\" })",
            "event": "Events.emit(\"gameplay.event\", { source = self.entity_id })",
            "delay": "Task.delay(0.25, function()\n    Debug.log(\"Timer complete\")\nend)"
        }
        var source = snippets[kind] || ""
        if (source.length === 0)
            return
        codeEditor.insert(codeEditor.cursorPosition, source)
        codeEditor.cursorPosition += source.length
        codeEditor.forceActiveFocus()
        diagnostic = "Inserted " + kind + " snippet"
    }

    function closeTab(path) {
        checkpointCurrentTab()
        var index = tabIndex(path)
        if (index < 0)
            return
        if (tabModel.get(index).dirty) {
            diagnostic = "Save or revert " + path + " before closing its tab"
            return
        }
        var wasCurrent = currentPath === path
        tabModel.remove(index)
        if (wasCurrent) {
            currentPath = ""
            savedSource = ""
            codeEditor.text = ""
            if (tabModel.count > 0)
                openScript(tabModel.get(Math.min(index, tabModel.count - 1)).path, false)
        }
        scheduleWorkspaceSave()
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
        diagnosticLine = Number(result.line || 0)
        diagnosticColumn = Number(result.column || 0)
        if (sourceValid) {
            diagnostic = "Luau validation passed"
        } else {
            var location = diagnosticLine > 0
                ? "line " + diagnosticLine + (diagnosticColumn > 0 ? ", column " + diagnosticColumn : "") + " · "
                : ""
            diagnostic = location + (result.diagnostic || "Luau validation failed")
        }
        refreshHighlighter()
        rebuildCodeActions()
        return sourceValid
    }

    function refreshApiReference() {
        apiEntries = parseJson(editorBridge.luauApiJson(), [])
        filterApiBrowser()
        updateCompletions()
    }

    function filterApiBrowser() {
        var needle = apiFilter.trim().toLowerCase()
        apiBrowserModel.clear()
        for (var index = 0; index < apiEntries.length; ++index) {
            var row = apiEntries[index]
            var haystack = (row.category + " " + row.label + " " + row.signature + " " + row.detail).toLowerCase()
            if (needle.length === 0 || haystack.indexOf(needle) >= 0) {
                apiBrowserModel.append({
                    "category": row.category,
                    "label": row.label,
                    "signature": row.signature,
                    "detail": row.detail,
                    "insertText": row.insert_text
                })
            }
        }
    }

    function updateCompletions() {
        if (!codeEditor || currentPath.length === 0)
            return
        var head = codeEditor.text.substring(0, codeEditor.cursorPosition)
        var match = head.match(/[A-Za-z_][A-Za-z0-9_.]*$/)
        completionPrefix = match ? match[0] : ""
        completionModel.clear()
        if (completionPrefix.length < 1)
            return
        var needle = completionPrefix.toLowerCase()
        var memberNeedle = needle.indexOf(".") >= 0 ? needle.substring(needle.lastIndexOf(".") + 1) : needle
        var candidates = []
        var seen = ({})
        for (var index = 0; index < apiEntries.length; ++index) {
            var row = apiEntries[index]
            var label = row.label.toLowerCase()
            var member = label.indexOf(".") >= 0 ? label.substring(label.lastIndexOf(".") + 1) : label
            var rank = label.indexOf(needle) === 0 ? 0
                : (memberNeedle.length > 0 && member.indexOf(memberNeedle) === 0 ? 1
                : (label.indexOf(needle) >= 0 ? 2 : 99))
            if (rank < 99 && !seen[row.label]) {
                seen[row.label] = true
                candidates.push({
                    "rank": rank,
                    "category": row.category,
                    "label": row.label,
                    "signature": row.signature,
                    "detail": row.detail,
                    "insertText": row.insert_text
                })
            }
        }
        if (needle.indexOf(".") < 0) {
            for (var symbolIndex = 0; symbolIndex < outlineModel.count; ++symbolIndex) {
                var symbolName = outlineModel.get(symbolIndex).name
                if (symbolName.toLowerCase().indexOf(needle) === 0 && !seen[symbolName]) {
                    seen[symbolName] = true
                    candidates.push({"rank": 1, "category": "Local", "label": symbolName,
                        "signature": symbolName, "detail": "Symbol in the current document.", "insertText": symbolName})
                }
            }
            var keywords = ["function", "local", "return", "if", "then", "else", "elseif", "for", "while", "do", "end", "type", "export"]
            for (var keywordIndex = 0; keywordIndex < keywords.length; ++keywordIndex) {
                var keyword = keywords[keywordIndex]
                if (keyword.indexOf(needle) === 0 && !seen[keyword])
                    candidates.push({"rank": 3, "category": "Keyword", "label": keyword,
                        "signature": keyword, "detail": "Luau keyword.", "insertText": keyword})
            }
        }
        candidates.sort(function(left, right) {
            return left.rank === right.rank ? left.label.localeCompare(right.label) : left.rank - right.rank
        })
        for (var candidateIndex = 0; candidateIndex < candidates.length && candidateIndex < 12; ++candidateIndex)
            completionModel.append(candidates[candidateIndex])
    }

    function showApiDocumentation(signature, detail) {
        apiDocSignature = signature
        apiDocDetail = detail
    }

    function insertApiCompletion(insertText, signature, detail) {
        var cursor = codeEditor.cursorPosition
        var start = Math.max(0, cursor - completionPrefix.length)
        codeEditor.remove(start, cursor)
        codeEditor.insert(start, insertText)
        codeEditor.cursorPosition = start + insertText.length
        showApiDocumentation(signature, detail)
        completionModel.clear()
        codeEditor.forceActiveFocus()
    }

    function scriptTemplate(index, rawName) {
        var moduleName = rawName.replace(/[^A-Za-z0-9_]/g, "_")
        if (!moduleName.match(/^[A-Za-z_]/))
            moduleName = "Script_" + moduleName
        if (index === 1) {
            return "--!strict\n\nlocal " + moduleName + " = {}\nlocal speed = 180.0\n\nfunction " + moduleName + ":on_update(dt: number)\n    local x = Input.axis(\"move_left\", \"move_right\")\n    local y = Input.axis(\"move_up\", \"move_down\")\n    CharacterBody2D.move(self.entity, x, y, Input.action_pressed(\"jump\"), Input.action_pressed(\"run\"))\nend\n\nreturn " + moduleName + "\n"
        }
        if (index === 2) {
            return "--!strict\n\nlocal " + moduleName + " = {}\n\nfunction " + moduleName + ":on_update(dt: number)\n    local target = Entity.nearest(self.entity, 24.0, { tag = \"Player\" })\n    if target then\n        Navigation2D.set_destination(self.entity, target.transform.position.x, target.transform.position.y)\n    end\nend\n\nreturn " + moduleName + "\n"
        }
        if (index === 3) {
            return "--!strict\n\nlocal " + moduleName + " = {}\n\nfunction " + moduleName + ":on_ready()\n    ui_text(\"Ready\")\nend\n\nfunction " + moduleName + ":on_event(name: string, payload: any)\n    if name == \"ui.refresh\" then\n        Events.emit(\"ui.updated\", { source = entity_name })\n    end\nend\n\nreturn " + moduleName + "\n"
        }
        return ""
    }

    function createScript() {
        var path = editorBridge.createContentFile("luau", scriptFolder.text, scriptName.text)
        if (path.length === 0) {
            diagnostic = editorBridge.lastError
            sourceValid = false
            return
        }
        var customSource = scriptTemplate(scriptTemplateKind.currentIndex, scriptName.text)
        if (customSource.length > 0 && !editorBridge.saveLuauScript(path, customSource)) {
            diagnostic = editorBridge.lastError
            sourceValid = false
            return
        }
        refreshScripts(path, true)
        diagnostic = "Created " + path + " · Ctrl+S saves, API suggestions are live"
        sourceValid = true
    }

    function saveCurrent() {
        if (formatOnSave)
            formatDocument()
        if (!validateCurrent())
            return
        if (editorBridge.saveLuauScript(currentPath, codeEditor.text)) {
            savedSource = codeEditor.text
            checkpointCurrentTab()
            diagnostic = "Saved atomically · recovery backups enabled"
            sourceValid = true
            refreshScripts(currentPath, false)
        } else {
            diagnostic = editorBridge.lastError
            sourceValid = false
        }
    }

    function breakpointArray() {
        var values = []
        for (var index = 0; index < breakpointModel.count; ++index) {
            var row = breakpointModel.get(index)
            values.push({"path": row.path, "line": row.line > 0 ? row.line : null, "function": row.functionName.length > 0 ? row.functionName : null, "enabled": row.active})
        }
        return values
    }

    function watchArray() {
        var values = []
        for (var index = 0; index < watchModel.count; ++index)
            values.push(watchModel.get(index).expression)
        return values
    }

    function scheduleWorkspaceSave() { workspaceSaveTimer.restart() }

    function saveWorkspace() {
        checkpointCurrentTab()
        var tabs = []
        for (var index = 0; index < tabModel.count; ++index) {
            var tab = tabModel.get(index)
            tabs.push({"path": tab.path, "source": tab.source, "saved": tab.saved, "dirty": tab.dirty})
        }
        editorBridge.saveWorkspaceState(JSON.stringify({
            "tabs": tabs,
            "active": currentPath,
            "breakpoints": breakpointArray(),
            "watches": watchArray(),
            "preferences": {
                "external_editor": externalEditorCommand,
                "auto_open_external": autoOpenExternal,
                "format_on_save": formatOnSave,
                "minimap": minimapEnabled
            }
        }))
    }

    function restoreWorkspace() {
        var state = parseJson(editorBridge.workspaceStateJson(), {})
        var preferences = state.preferences || {}
        externalEditorCommand = String(preferences.external_editor || "")
        autoOpenExternal = preferences.auto_open_external === true
        formatOnSave = preferences.format_on_save === true
        minimapEnabled = preferences.minimap !== false
        // Component startup opens the first disk script while the script index is
        // populated. Detach that transient document before restoring tabs so
        // checkpointCurrentTab() cannot overwrite a recovered dirty buffer.
        currentPath = ""
        savedSource = ""
        codeEditor.text = ""
        tabModel.clear()
        var tabs = state.tabs || []
        for (var index = 0; index < tabs.length; ++index) {
            if (scriptIndex(tabs[index].path) >= 0) {
                tabModel.append({
                    "path": tabs[index].path,
                    "name": tabs[index].path.split("/").pop(),
                    "source": String(tabs[index].source || ""),
                    "saved": String(tabs[index].saved || ""),
                    "dirty": tabs[index].dirty === true
                })
            }
        }
        breakpointModel.clear()
        var breakpoints = state.breakpoints || []
        for (var bp = 0; bp < breakpoints.length; ++bp) {
            breakpointModel.append({
                "path": String(breakpoints[bp].path || ""),
                "line": Number(breakpoints[bp].line || 0),
                "functionName": String(breakpoints[bp].function || ""),
                "active": breakpoints[bp].enabled !== false
            })
        }
        watchModel.clear()
        var watches = state.watches || []
        for (var watch = 0; watch < watches.length; ++watch)
            watchModel.append({"expression": String(watches[watch])})
        editorBridge.setLuauBreakpointsJson(JSON.stringify(breakpointArray()))
        var active = String(state.active || "")
        if (active.length > 0 && tabIndex(active) >= 0)
            openScript(active, false)
        else if (tabModel.count > 0)
            openScript(tabModel.get(0).path, false)
        refreshDebugger()
    }

    function addBreakpoint() {
        if (currentPath.length === 0)
            return
        breakpointModel.append({
            "path": currentPath,
            "line": 0,
            "functionName": breakpointFunction.currentText,
            "active": true
        })
        editorBridge.setLuauBreakpointsJson(JSON.stringify(breakpointArray()))
        scheduleWorkspaceSave()
        refreshHighlighter()
        refreshDebugger()
    }

    function removeBreakpoint(index) {
        breakpointModel.remove(index)
        editorBridge.setLuauBreakpointsJson(JSON.stringify(breakpointArray()))
        scheduleWorkspaceSave()
        refreshHighlighter()
    }

    function addWatch() {
        var expression = watchExpression.text.trim()
        if (expression.length === 0)
            return
        watchModel.append({"expression": expression})
        watchExpression.text = ""
        scheduleWorkspaceSave()
        refreshWatches()
    }

    function refreshWatches() {
        var results = parseJson(editorBridge.luauWatchesJson(JSON.stringify(watchArray())), [])
        watchResultModel.clear()
        for (var index = 0; index < results.length; ++index) {
            watchResultModel.append({
                "expression": results[index].expression,
                "valueText": results[index].error ? results[index].error : JSON.stringify(results[index].value)
            })
        }
    }

    function refreshDebugger() {
        var state = parseJson(editorBridge.luauDebugStateJson(), {"paused": false, "breakpoints": []})
        debugState = state
        if (state.paused && state.frame) {
            diagnostic = "Paused · " + state.frame.path + ":" + (state.frame.line || 0) + " · " + state.frame.function
            sourceValid = true
        }
        refreshWatches()
    }

    ListModel {
        id: scriptModel
    }
    ListModel { id: completionModel }
    ListModel { id: apiBrowserModel }
    ListModel { id: tabModel }
    ListModel { id: breakpointModel }
    ListModel { id: watchModel }
    ListModel { id: watchResultModel }
    ListModel { id: outlineModel }
    ListModel { id: codeActionModel }

    Connections {
        target: editorBridge
        function onLuauScriptsChanged() {
            root.refreshScripts(root.currentPath, false)
        }
        function onContentAssetOpenRequested(relativePath, assetType) {
            if (assetType === "LuauScript")
                root.openScript(relativePath, false)
        }
        function onLuauDebuggerChanged() { root.refreshDebugger() }
    }

    Component.onCompleted: {
        luauSyntaxHighlighter.textDocument = codeEditor.textDocument
        refreshApiReference()
        refreshScripts("", true)
        restoreWorkspace()
        outlineTimer.restart()
    }

    Timer {
        id: validationTimer
        interval: 650
        repeat: false
        onTriggered: {
            if (root.dirty)
                root.validateCurrent()
        }
    }
    Timer { id: workspaceSaveTimer; interval: 400; repeat: false; onTriggered: root.saveWorkspace() }
    Timer { interval: 500; repeat: true; running: editorBridge.projectOpen; onTriggered: root.refreshDebugger() }
    Timer { id: outlineTimer; interval: 250; repeat: false; onTriggered: root.rebuildOutline() }

    Shortcut { sequences: [StandardKey.Save]; enabled: root.dirty; onActivated: root.saveCurrent() }
    Shortcut { sequence: "Ctrl+Space"; enabled: root.currentPath.length > 0; onActivated: root.updateCompletions() }
    Shortcut { sequences: [StandardKey.Find]; enabled: root.currentPath.length > 0; onActivated: root.showFind(false) }
    Shortcut { sequence: "Ctrl+H"; enabled: root.currentPath.length > 0; onActivated: root.showFind(true) }
    Shortcut { sequence: "Ctrl+G"; enabled: root.currentPath.length > 0; onActivated: goToLineDialog.open() }
    Shortcut { sequence: "F3"; enabled: root.findVisible; onActivated: root.findNext(false) }
    Shortcut { sequence: "Shift+F3"; enabled: root.findVisible; onActivated: root.findNext(true) }
    Shortcut { sequence: "Escape"; enabled: root.findVisible; onActivated: root.findVisible = false }
    Shortcut { sequence: "Ctrl+/"; enabled: root.currentPath.length > 0; onActivated: root.toggleCurrentLineComment() }
    Shortcut { sequence: "Ctrl+Shift+D"; enabled: root.currentPath.length > 0; onActivated: root.duplicateCurrentLine() }
    Shortcut { sequence: "Alt+Shift+F"; enabled: root.currentPath.length > 0; onActivated: root.formatDocument() }
    Shortcut {
        sequence: "Ctrl+Alt+M"
        onActivated: {
            root.minimapEnabled = !root.minimapEnabled
            root.scheduleWorkspaceSave()
        }
    }

    Dialog {
        id: goToLineDialog
        x: Math.max(8, (root.width - width) / 2)
        y: Math.max(8, (root.height - height) / 2)
        width: 280
        modal: true
        title: "Go to Line"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            goToLineField.text = String(root.lineForPosition(codeEditor.cursorPosition))
            goToLineField.selectAll()
            goToLineField.forceActiveFocus()
        }
        onAccepted: root.goToLine(Number(goToLineField.text))
        contentItem: TextField {
            id: goToLineField
            validator: IntValidator { bottom: 1 }
            color: Theme.DarkTheme.text
            placeholderText: "Line number"
            background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 4 }
        }
    }

    Dialog {
        id: newScriptDialog
        x: Math.max(8, (root.width - width) / 2)
        y: Math.max(8, (root.height - height) / 2)
        width: 420
        modal: true
        title: "New Luau script"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            scriptName.text = "GameplayScript"
            scriptName.selectAll()
            scriptName.forceActiveFocus()
        }
        onAccepted: root.createScript()

        contentItem: Column {
            spacing: 8
            Text { text: "Name"; color: Theme.DarkTheme.muted; font.pixelSize: 11 }
            TextField {
                id: scriptName
                width: parent.width
                color: Theme.DarkTheme.text
                placeholderText: "GameplayScript"
                background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 4 }
            }
            Text { text: "Folder under scripts/"; color: Theme.DarkTheme.muted; font.pixelSize: 11 }
            TextField {
                id: scriptFolder
                width: parent.width
                text: "scripts"
                color: Theme.DarkTheme.text
                background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 4 }
            }
            Text { text: "Template"; color: Theme.DarkTheme.muted; font.pixelSize: 11 }
            ComboBox {
                id: scriptTemplateKind
                width: parent.width
                model: ["Basic lifecycle", "Character controller", "NPC navigation", "UI event handler"]
            }
            Text {
                width: parent.width
                text: "Scripts are validated before atomic save and receive rotating recovery backups."
                color: Theme.DarkTheme.muted
                font.pixelSize: 10
                wrapMode: Text.Wrap
            }
        }
    }

    Dialog {
        id: editorPreferencesDialog
        x: Math.max(8, (root.width - width) / 2)
        y: Math.max(8, (root.height - height) / 2)
        width: 440
        modal: true
        title: "Luau Editor Preferences"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            externalEditorField.text = root.externalEditorCommand
            autoOpenExternalCheck.checked = root.autoOpenExternal
            formatOnSaveCheck.checked = root.formatOnSave
            minimapCheck.checked = root.minimapEnabled
            externalEditorField.forceActiveFocus()
        }
        onAccepted: {
            root.externalEditorCommand = externalEditorField.text.trim()
            root.autoOpenExternal = autoOpenExternalCheck.checked
            root.formatOnSave = formatOnSaveCheck.checked
            root.minimapEnabled = minimapCheck.checked
            root.scheduleWorkspaceSave()
            root.diagnostic = "Luau editor preferences saved"
        }

        contentItem: Column {
            spacing: 9
            Text { text: "External editor command"; color: Theme.DarkTheme.muted; font.pixelSize: 11 }
            TextField {
                id: externalEditorField
                width: parent.width
                color: Theme.DarkTheme.text
                placeholderText: "Leave blank for the system default"
                background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 4 }
            }
            Text {
                width: parent.width
                text: "Use {file} in a custom command to control where the script path is inserted."
                color: Theme.DarkTheme.muted
                font.pixelSize: 9
                wrapMode: Text.Wrap
            }
            CheckBox { id: autoOpenExternalCheck; text: "Open scripts in the external editor automatically" }
            CheckBox { id: formatOnSaveCheck; text: "Format document before save" }
            CheckBox { id: minimapCheck; text: "Show code minimap" }
        }
    }

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

        ListView {
            width: parent.width
            height: tabModel.count > 0 ? 30 : 0
            visible: tabModel.count > 0
            orientation: ListView.Horizontal
            spacing: 3
            clip: true
            model: tabModel
            delegate: Rectangle {
                id: tabChip
                required property string path
                required property string name
                required property bool dirty
                width: Math.min(190, Math.max(105, tabName.implicitWidth + 36))
                height: ListView.view.height
                radius: 4
                color: currentPath === tabChip.path ? Theme.DarkTheme.accentSoft : Theme.DarkTheme.surface
                border.color: currentPath === tabChip.path ? Theme.DarkTheme.accent : Theme.DarkTheme.borderSoft
                Row {
                    anchors.fill: parent
                    anchors.margins: 5
                    spacing: 5
                    Text { id: tabName; width: parent.width - 20; height: parent.height; text: (tabChip.dirty ? "● " : "") + tabChip.name; color: Theme.DarkTheme.text; font.pixelSize: 9; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
                    Text { width: 15; height: parent.height; text: "×"; color: Theme.DarkTheme.muted; font.pixelSize: 12; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                    onClicked: function(mouse) {
                        if (mouse.button === Qt.MiddleButton)
                            root.closeTab(tabChip.path)
                        else
                            root.openScript(tabChip.path, false)
                    }
                }
                ToolTip.visible: tabHover.hovered
                ToolTip.text: "Click to activate · middle-click to close"
                HoverHandler { id: tabHover }
            }
        }

        Row {
            width: parent.width
            height: 32
            spacing: 7

            MfButton {
                width: 94
                text: "New Script"
                accent: true
                enabled: editorBridge.projectOpen
                onClicked: newScriptDialog.open()
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
            height: 27
            spacing: 5
            MfButton { width: 72; height: 27; text: "Comment"; enabled: root.currentPath.length > 0; onClicked: root.toggleCurrentLineComment() }
            MfButton { width: 66; height: 27; text: "Dupe Line"; enabled: root.currentPath.length > 0; onClicked: root.duplicateCurrentLine() }
            MfButton { width: 76; height: 27; text: "on_update"; enabled: root.currentPath.length > 0; onClicked: root.insertProductivitySnippet("update") }
            MfButton { width: 54; height: 27; text: "+ Log"; enabled: root.currentPath.length > 0; onClicked: root.insertProductivitySnippet("log") }
            MfButton { width: 62; height: 27; text: "+ Spawn"; enabled: root.currentPath.length > 0; onClicked: root.insertProductivitySnippet("spawn") }
            MfButton { width: 58; height: 27; text: "+ Event"; enabled: root.currentPath.length > 0; onClicked: root.insertProductivitySnippet("event") }
            MfButton { width: 58; height: 27; text: "+ Delay"; enabled: root.currentPath.length > 0; onClicked: root.insertProductivitySnippet("delay") }
            MfButton {
                width: 72; height: 27; text: "External"
                enabled: root.currentPath.length > 0
                onClicked: {
                    if (!editorBridge.openExternalEditor(root.currentPath, root.externalEditorCommand))
                        root.diagnostic = editorBridge.lastError
                }
            }
            Text { width: Math.max(0, parent.width - x); height: parent.height; text: "Ctrl+/ · Ctrl+Shift+D"; color: Theme.DarkTheme.muted; font.pixelSize: 8; horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
        }

        Row {
            width: parent.width
            height: 27
            spacing: 5
            MfButton { width: 72; height: 27; text: "Format"; enabled: root.currentPath.length > 0; onClicked: root.formatDocument() }
            MfButton {
                width: 74; height: 27; text: root.minimapEnabled ? "Minimap On" : "Minimap Off"
                onClicked: {
                    root.minimapEnabled = !root.minimapEnabled
                    root.scheduleWorkspaceSave()
                }
            }
            MfButton { width: 88; height: 27; text: "Preferences"; onClicked: editorPreferencesDialog.open() }
            Text {
                width: Math.max(0, parent.width - x)
                height: parent.height
                text: "Alt+Shift+F format · Ctrl+Alt+M minimap · quick fixes appear above the editor"
                color: Theme.DarkTheme.muted
                font.pixelSize: 8
                horizontalAlignment: Text.AlignRight
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }

        Rectangle {
            width: parent.width
            height: root.findVisible ? (root.replaceVisible ? 62 : 32) : 0
            visible: root.findVisible
            radius: 4
            color: Theme.DarkTheme.surface
            border.color: Theme.DarkTheme.borderSoft

            Column {
                anchors.fill: parent
                anchors.margins: 3
                spacing: 3

                Row {
                    width: parent.width
                    height: 25
                    spacing: 4
                    TextField {
                        id: findField
                        width: Math.max(120, parent.width - 318)
                        height: parent.height
                        placeholderText: "Find in current script"
                        color: Theme.DarkTheme.text
                        selectByMouse: true
                        background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 3 }
                        onAccepted: root.findNext(false)
                    }
                    MfButton { width: 48; height: parent.height; text: "Prev"; onClicked: root.findNext(true) }
                    MfButton { width: 48; height: parent.height; text: "Next"; onClicked: root.findNext(false) }
                    CheckBox {
                        id: findCaseSensitive
                        width: 58
                        height: parent.height
                        text: "Case"
                        checked: false
                        palette.windowText: Theme.DarkTheme.muted
                    }
                    Text {
                        width: Math.max(45, parent.width - x - 27)
                        height: parent.height
                        text: root.findStatus
                        color: Theme.DarkTheme.muted
                        font.pixelSize: 9
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }
                    MfButton { width: 23; height: parent.height; text: "×"; onClicked: root.findVisible = false }
                }

                Row {
                    width: parent.width
                    height: 25
                    visible: root.replaceVisible
                    spacing: 4
                    TextField {
                        id: replaceField
                        width: Math.max(120, parent.width - 178)
                        height: parent.height
                        placeholderText: "Replace with"
                        color: Theme.DarkTheme.text
                        selectByMouse: true
                        background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 3 }
                        onAccepted: root.replaceCurrent()
                    }
                    MfButton { width: 82; height: parent.height; text: "Replace"; onClicked: root.replaceCurrent() }
                    MfButton { width: 88; height: parent.height; text: "Replace All"; onClicked: root.replaceAll() }
                }
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

                Column {
                    anchors.fill: parent
                    anchors.margins: 5
                    spacing: 4

                    ListView {
                        id: scriptList
                        width: parent.width
                        height: Math.max(90, parent.height * 0.42)
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

                    Rectangle { width: parent.width; height: 1; color: Theme.DarkTheme.borderSoft }
                    Text {
                        width: parent.width
                        text: debugState.paused && debugState.frame
                            ? "Paused · " + debugState.frame.function + " @ " + (debugState.frame.line || 0)
                            : (debugState.pause_requested ? "Pause requested" : "Luau Debugger · callback-level")
                        color: debugState.paused ? Theme.DarkTheme.warning : Theme.DarkTheme.muted
                        font.pixelSize: 9
                        font.bold: debugState.paused
                        elide: Text.ElideRight
                    }
                    Row {
                        width: parent.width; height: 26; spacing: 3
                        MfButton { width: (parent.width - 6) / 3; height: 26; text: "Pause"; onClicked: editorBridge.luauDebugCommand("pause") }
                        MfButton { width: (parent.width - 6) / 3; height: 26; text: "Resume"; enabled: debugState.paused; onClicked: editorBridge.luauDebugCommand("resume") }
                        MfButton { width: (parent.width - 6) / 3; height: 26; text: "Step"; enabled: debugState.paused; onClicked: editorBridge.luauDebugCommand("step") }
                    }
                    Row {
                        width: parent.width; height: 28; spacing: 3
                        ComboBox {
                            id: breakpointFunction
                            width: parent.width - 54; height: 28
                            model: ["on_ready", "on_update", "on_fixed_update", "on_event", "on_collision_enter", "on_destroy"]
                        }
                        MfButton { width: 51; height: 28; text: "+ BP"; enabled: currentPath.length > 0; onClicked: root.addBreakpoint() }
                    }
                    ListView {
                        width: parent.width
                        height: Math.min(66, contentHeight)
                        clip: true; spacing: 2; model: breakpointModel
                        delegate: Rectangle {
                            id: breakpointRow
                            required property string path
                            required property string functionName
                            required property bool active
                            width: ListView.view.width; height: 30; radius: 3; color: Theme.DarkTheme.background
                            Text { anchors.fill: parent; anchors.leftMargin: 5; anchors.rightMargin: 24; text: breakpointRow.functionName + " · " + breakpointRow.path.split("/").pop(); color: Theme.DarkTheme.text; font.pixelSize: 8; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
                            Text { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; width: 22; text: "×"; color: Theme.DarkTheme.danger; horizontalAlignment: Text.AlignHCenter }
                            MouseArea { anchors.fill: parent; onClicked: root.removeBreakpoint(index) }
                            ToolTip.visible: breakpointHover.hovered; ToolTip.text: "Callback breakpoint · click to remove"
                            HoverHandler { id: breakpointHover }
                        }
                    }
                    Row {
                        width: parent.width; height: 28; spacing: 3
                        TextField {
                            id: watchExpression
                            width: parent.width - 42; height: 28
                            placeholderText: "self.speed"
                            color: Theme.DarkTheme.text
                            font.pixelSize: 9
                            background: Rectangle { color: Theme.DarkTheme.background; border.color: Theme.DarkTheme.border; radius: 3 }
                            onAccepted: root.addWatch()
                        }
                        MfButton { width: 39; height: 28; text: "+"; onClicked: root.addWatch() }
                    }
                    ListView {
                        width: parent.width
                        height: Math.max(48, parent.height - y)
                        clip: true; spacing: 2; model: watchResultModel
                        delegate: Rectangle {
                            id: watchRow
                            required property string expression
                            required property string valueText
                            width: ListView.view.width; height: 34; radius: 3; color: Theme.DarkTheme.background
                            Column { anchors.fill: parent; anchors.margins: 4; spacing: 1
                                Text { width: parent.width; text: watchRow.expression; color: Theme.DarkTheme.accent; font.pixelSize: 8; font.bold: true; elide: Text.ElideRight }
                                Text { width: parent.width; text: watchRow.valueText; color: Theme.DarkTheme.muted; font.pixelSize: 8; elide: Text.ElideRight }
                            }
                        }
                    }
                }
            }

            Rectangle {
                width: Math.max(180, parent.width - x - (apiPanel.visible ? apiPanel.width + 8 : 0))
                height: parent.height
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.background
                border.color: root.sourceValid ? Theme.DarkTheme.borderSoft : Theme.DarkTheme.danger
                border.width: 1

                Column {
                    anchors.fill: parent
                    anchors.margins: 2
                    spacing: 2

                    Rectangle {
                        visible: codeActionModel.count > 0
                        width: parent.width
                        height: visible ? 31 : 0
                        color: Theme.DarkTheme.panelAlt
                        radius: 4

                        ListView {
                            anchors.fill: parent
                            anchors.margins: 3
                            orientation: ListView.Horizontal
                            spacing: 4
                            clip: true
                            model: codeActionModel
                            delegate: MfButton {
                                required property string label
                                required property string kind
                                width: Math.max(112, implicitWidth + 18)
                                height: ListView.view.height
                                text: "Fix · " + label
                                onClicked: root.applyCodeAction(kind)
                            }
                        }
                    }

                    Rectangle {
                        visible: completionModel.count > 0
                        width: parent.width
                        height: visible ? 34 : 0
                        color: Theme.DarkTheme.surfaceRaised
                        radius: 4

                        ListView {
                            anchors.fill: parent
                            anchors.margins: 3
                            orientation: ListView.Horizontal
                            spacing: 4
                            clip: true
                            model: completionModel

                            delegate: Rectangle {
                                id: completionChip
                                required property string label
                                required property string signature
                                required property string detail
                                required property string insertText
                                width: Math.max(90, completionLabel.implicitWidth + 18)
                                height: ListView.view.height
                                radius: 4
                                color: completionMouse.containsMouse ? Theme.DarkTheme.accentSoft : Theme.DarkTheme.panelAlt
                                border.color: completionMouse.containsMouse ? Theme.DarkTheme.accent : Theme.DarkTheme.borderSoft
                                Text {
                                    id: completionLabel
                                    anchors.centerIn: parent
                                    text: completionChip.label
                                    color: Theme.DarkTheme.text
                                    font.pixelSize: 10
                                    font.bold: true
                                }
                                MouseArea {
                                    id: completionMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onEntered: root.showApiDocumentation(completionChip.signature, completionChip.detail)
                                    onClicked: root.insertApiCompletion(completionChip.insertText, completionChip.signature, completionChip.detail)
                                }
                                ToolTip.visible: completionMouse.containsMouse
                                ToolTip.text: completionChip.signature
                            }
                        }
                    }

                    Item {
                        width: parent.width
                        height: Math.max(80, parent.height - y - 25)

                        ScrollView {
                            id: codeScroll
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: minimap.visible ? minimap.left : parent.right
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
                                background: Rectangle { color: Theme.DarkTheme.background }

                                onCursorPositionChanged: {
                                    root.updateCompletions()
                                    root.refreshHighlighter()
                                    minimapCanvas.requestPaint()
                                }
                                onTextChanged: {
                                    root.updateCompletions()
                                    root.rebuildCodeActions()
                                    outlineTimer.restart()
                                    minimapCanvas.requestPaint()
                                    if (root.dirty) {
                                        root.checkpointCurrentTab()
                                        root.scheduleWorkspaceSave()
                                        root.diagnostic = "Unsaved changes · background validation pending"
                                        root.sourceValid = true
                                        validationTimer.restart()
                                    }
                                }
                            }
                        }

                        Rectangle {
                            id: minimap
                            visible: root.minimapEnabled && parent.width > 320
                            width: visible ? 84 : 0
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            anchors.right: parent.right
                            color: Theme.DarkTheme.surface
                            border.color: Theme.DarkTheme.borderSoft

                            Canvas {
                                id: minimapCanvas
                                anchors.fill: parent
                                anchors.margins: 4
                                onPaint: {
                                    var context = getContext("2d")
                                    context.reset()
                                    var lines = codeEditor.text.split("\n")
                                    var rowHeight = Math.max(1, height / Math.max(1, lines.length))
                                    for (var index = 0; index < lines.length; ++index) {
                                        var trimmed = lines[index].trim()
                                        var lineWidth = Math.min(width - 2, Math.max(2, trimmed.length * 1.25))
                                        context.fillStyle = index + 1 === root.lineForPosition(codeEditor.cursorPosition)
                                            ? Theme.DarkTheme.accent
                                            : (trimmed.indexOf("--") === 0 ? "#6f8799" : "#617083")
                                        context.fillRect(1, index * rowHeight, lineWidth, Math.max(1, rowHeight * 0.55))
                                    }
                                }
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: function(mouse) {
                                    root.goToLine(1 + Math.floor(mouse.y / Math.max(1, height) * codeEditor.lineCount))
                                }
                            }
                            ToolTip.visible: minimapHover.hovered
                            ToolTip.text: "Code minimap · click to navigate"
                            HoverHandler { id: minimapHover }
                        }
                    }

                    Row {
                        width: parent.width
                        height: 22
                        spacing: 8
                        Text {
                            id: diagnosticStatus
                            width: parent.width - apiHint.width - 8
                            height: parent.height
                            text: root.sourceValid
                                ? (root.dirty ? "Modified" : "Validated")
                                : (root.diagnosticLine > 0 ? "Error at " + root.diagnosticLine + ":" + Math.max(1, root.diagnosticColumn) : "Luau error")
                            color: root.sourceValid ? Theme.DarkTheme.muted : Theme.DarkTheme.danger
                            font.pixelSize: 9
                            verticalAlignment: Text.AlignVCenter
                            MouseArea {
                                anchors.fill: parent
                                enabled: root.diagnosticLine > 0
                                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                                onClicked: root.goToLine(root.diagnosticLine)
                            }
                            ToolTip.visible: diagnosticHover.hovered && root.diagnosticLine > 0
                            ToolTip.text: "Click to open diagnostic line"
                            HoverHandler { id: diagnosticHover }
                        }
                        Text {
                            id: apiHint
                            height: parent.height
                            text: codeEditor.lineCount + " lines · Ctrl+F find · Ctrl+G line · Ctrl+Space API · Alt+Shift+F format"
                            color: Theme.DarkTheme.muted
                            font.pixelSize: 9
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
            }

            Rectangle {
                id: apiPanel
                visible: parent.width >= 760
                width: Math.min(260, Math.max(210, parent.width * 0.22))
                height: parent.height
                radius: Theme.DarkTheme.cardRadius
                color: Theme.DarkTheme.surface
                border.color: Theme.DarkTheme.borderSoft
                border.width: 1

                Column {
                    anchors.fill: parent
                    anchors.margins: 6
                    spacing: 5

                    Text {
                        width: parent.width
                        text: "Outline · " + outlineModel.count + " symbols"
                        color: Theme.DarkTheme.accent
                        font.pixelSize: 10
                        font.bold: true
                    }
                    ListView {
                        width: parent.width
                        height: Math.min(105, contentHeight)
                        clip: true
                        spacing: 2
                        model: outlineModel
                        delegate: Rectangle {
                            id: outlineRow
                            required property string name
                            required property int line
                            width: ListView.view.width
                            height: 25
                            radius: 3
                            color: outlineMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent"
                            Text {
                                anchors.fill: parent
                                anchors.leftMargin: 5
                                anchors.rightMargin: 5
                                text: outlineRow.name + "  ·  " + outlineRow.line
                                color: Theme.DarkTheme.text
                                font.family: "Menlo"
                                font.pixelSize: 8
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideMiddle
                            }
                            MouseArea {
                                id: outlineMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: root.goToLine(outlineRow.line)
                            }
                            ToolTip.visible: outlineMouse.containsMouse
                            ToolTip.text: "Go to line " + outlineRow.line
                        }
                    }
                    Rectangle { width: parent.width; height: 1; color: Theme.DarkTheme.borderSoft }
                    Text {
                        width: parent.width
                        text: "MiniForge Luau API"
                        color: Theme.DarkTheme.text
                        font.pixelSize: 11
                        font.bold: true
                    }
                    MfSearchBar {
                        id: apiSearch
                        width: parent.width
                        placeholderText: "Search API"
                        onTextChanged: {
                            root.apiFilter = text
                            root.filterApiBrowser()
                        }
                    }
                    ListView {
                        id: apiBrowser
                        width: parent.width
                        height: Math.max(70, parent.height - y - apiDocumentation.height - 6)
                        clip: true
                        spacing: 2
                        model: apiBrowserModel

                        delegate: Rectangle {
                            id: apiRow
                            required property string category
                            required property string label
                            required property string signature
                            required property string detail
                            required property string insertText
                            width: ListView.view.width
                            height: 38
                            radius: 4
                            color: apiMouse.containsMouse ? Theme.DarkTheme.surfaceRaised : "transparent"
                            Column {
                                anchors.fill: parent
                                anchors.margins: 5
                                spacing: 1
                                Text { width: parent.width; text: apiRow.label; color: Theme.DarkTheme.text; font.pixelSize: 10; font.bold: true; elide: Text.ElideRight }
                                Text { width: parent.width; text: apiRow.category; color: Theme.DarkTheme.muted; font.pixelSize: 8; elide: Text.ElideRight }
                            }
                            MouseArea {
                                id: apiMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: root.showApiDocumentation(apiRow.signature, apiRow.detail)
                                onDoubleClicked: {
                                    root.completionPrefix = ""
                                    root.insertApiCompletion(apiRow.insertText, apiRow.signature, apiRow.detail)
                                }
                            }
                            ToolTip.visible: apiMouse.containsMouse
                            ToolTip.text: "Double-click to insert"
                        }
                    }
                    Rectangle {
                        id: apiDocumentation
                        width: parent.width
                        height: 112
                        radius: 4
                        color: Theme.DarkTheme.background
                        border.color: Theme.DarkTheme.borderSoft
                        Column {
                            anchors.fill: parent
                            anchors.margins: 7
                            spacing: 5
                            Text {
                                width: parent.width
                                text: root.apiDocSignature
                                color: Theme.DarkTheme.accent
                                font.family: "Menlo"
                                font.pixelSize: 9
                                wrapMode: Text.WrapAnywhere
                                maximumLineCount: 3
                                elide: Text.ElideRight
                            }
                            Text {
                                width: parent.width
                                height: parent.height - y
                                text: root.apiDocDetail
                                color: Theme.DarkTheme.muted
                                font.pixelSize: 9
                                wrapMode: Text.Wrap
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }
        }
    }
}
