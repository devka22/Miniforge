use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

use crate::engine::document_manager::{
    CloseDocumentChoice, CloseDocumentOutcome, DocumentManager, EditorDocument, backup_path_for,
};
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptDiagnosticSeverity {
    Error,
    Warning,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDiagnostic {
    pub line: Option<usize>,
    pub severity: ScriptDiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSymbol {
    pub name: String,
    pub line: usize,
    pub kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptEditorStats {
    pub lines: usize,
    pub characters: usize,
    pub functions: usize,
    pub graph_nodes: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEditorCodeAction {
    pub title: String,
    pub kind: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEditorSearchResult {
    pub line: usize,
    pub column: usize,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEditorCompletion {
    pub label: String,
    pub detail: String,
    pub insert_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEditorMiniMapLine {
    pub line: usize,
    pub kind: String,
    pub intensity: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptDiagnosticSummary {
    pub errors: usize,
    pub warnings: usize,
    pub hints: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptDocument {
    pub path: Option<PathBuf>,
    pub syntax_error: Option<String>,
    pub dirty: bool,
    pub language: String,
    pub last_saved_backup: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptEditor {
    pub document: ScriptDocument,
    pub lines: Vec<String>,
    pub tabs: Vec<PathBuf>,
    pub closed_tabs: Vec<PathBuf>,
    pub diagnostics: Vec<ScriptDiagnostic>,
    pub outline: Vec<ScriptSymbol>,
    pub document_manager: DocumentManager,
}

impl ScriptEditor {
    pub fn open(&mut self, path: PathBuf) -> io::Result<()> {
        self.sync_active_document();
        let document = self.document_manager.open(&path)?;
        self.apply_editor_document(&document);
        self.sync_tabs_from_manager();
        self.validate();
        Ok(())
    }

    pub fn open_project_file(
        &mut self,
        project_path: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> io::Result<PathBuf> {
        let path = resolve_project_path(project_path.as_ref(), path.as_ref())?;
        self.open(path.clone())?;
        Ok(path)
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.lines = split_text_lines(&text.into());
        self.document.dirty = true;
        self.sync_active_document();
    }

    pub fn insert_text(&mut self, line: usize, column: usize, text: &str) -> (usize, usize) {
        let mut cursor_line = line.min(self.lines.len().saturating_sub(1));
        let mut cursor_column = column;
        for (index, chunk) in text.split('\n').enumerate() {
            if index > 0 {
                let next = self.split_line(cursor_line, cursor_column);
                cursor_line = next.0;
                cursor_column = next.1;
            }
            for character in chunk.chars() {
                let next = self.insert_char(cursor_line, cursor_column, character);
                cursor_line = next.0;
                cursor_column = next.1;
            }
        }
        (cursor_line, cursor_column)
    }

    pub fn insert_char(&mut self, line: usize, column: usize, character: char) -> (usize, usize) {
        let line = line.min(self.lines.len().saturating_sub(1));
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let column = clamp_to_char_boundary(&self.lines[line], column);
        self.lines[line].insert(column, character);
        self.document.dirty = true;
        self.sync_active_document();
        (line, column + character.len_utf8())
    }

    pub fn split_line(&mut self, line: usize, column: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        let column = clamp_to_char_boundary(&self.lines[line], column);
        let tail = self.lines[line].split_off(column);
        self.lines.insert(line + 1, tail);
        self.document.dirty = true;
        self.sync_active_document();
        (line + 1, 0)
    }

    pub fn backspace(&mut self, line: usize, column: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
            return (0, 0);
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        let column = clamp_to_char_boundary(&self.lines[line], column);
        if column > 0 {
            let prev = previous_char_boundary(&self.lines[line], column);
            self.lines[line].drain(prev..column);
            self.document.dirty = true;
            self.sync_active_document();
            return (line, prev);
        }
        if line > 0 {
            let previous_len = self.lines[line - 1].len();
            let current = self.lines.remove(line);
            self.lines[line - 1].push_str(&current);
            self.document.dirty = true;
            self.sync_active_document();
            return (line - 1, previous_len);
        }
        (line, column)
    }

    pub fn delete_forward(&mut self, line: usize, column: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
            return (0, 0);
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        let column = clamp_to_char_boundary(&self.lines[line], column);
        if column < self.lines[line].len() {
            let next = next_char_boundary(&self.lines[line], column);
            self.lines[line].drain(column..next);
            self.document.dirty = true;
            self.sync_active_document();
            return (line, column);
        }
        if line + 1 < self.lines.len() {
            let next_line = self.lines.remove(line + 1);
            self.lines[line].push_str(&next_line);
            self.document.dirty = true;
            self.sync_active_document();
        }
        (line, column)
    }

    pub fn duplicate_line(&mut self, line: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        let duplicate = self.lines[line].clone();
        self.lines.insert(line + 1, duplicate);
        self.document.dirty = true;
        self.sync_active_document();
        (line + 1, 0)
    }

    pub fn delete_line(&mut self, line: usize) -> (usize, usize) {
        if self.lines.len() <= 1 {
            if let Some(first) = self.lines.first_mut() {
                first.clear();
            } else {
                self.lines.push(String::new());
            }
            self.document.dirty = true;
            self.sync_active_document();
            return (0, 0);
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        self.lines.remove(line);
        self.document.dirty = true;
        self.sync_active_document();
        (line.min(self.lines.len().saturating_sub(1)), 0)
    }

    pub fn indent_line(&mut self, line: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        self.lines[line].insert_str(0, "    ");
        self.document.dirty = true;
        self.sync_active_document();
        (line, 4)
    }

    pub fn outdent_line(&mut self, line: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        let remove = self.lines[line]
            .chars()
            .take_while(|character| *character == ' ')
            .take(4)
            .count();
        if remove > 0 {
            self.lines[line].drain(0..remove);
            self.document.dirty = true;
            self.sync_active_document();
        }
        (line, 0)
    }

    pub fn toggle_line_comment(&mut self, line: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let line = line.min(self.lines.len().saturating_sub(1));
        let trimmed_start = self.lines[line].trim_start();
        let leading = self.lines[line].len() - trimmed_start.len();
        let extension = self
            .document
            .path
            .as_deref()
            .and_then(Path::extension)
            .and_then(|value| value.to_str());
        let is_luau = matches!(extension, Some("luau" | "lua"))
            || self.document.language.eq_ignore_ascii_case("luau")
            || self.document.language.eq_ignore_ascii_case("lua")
            || self.document.path.is_none();
        let marker = if is_luau { "--" } else { "//" };
        if trimmed_start.starts_with(marker) {
            let start = leading;
            let end = (leading + 2).min(self.lines[line].len());
            self.lines[line].drain(start..end);
            if self.lines[line].get(start..start + 1) == Some(" ") {
                self.lines[line].drain(start..start + 1);
            }
        } else {
            self.lines[line].insert_str(leading, &format!("{marker} "));
        }
        self.document.dirty = true;
        self.sync_active_document();
        (line, leading)
    }

    pub fn format_json_pretty(&mut self) -> io::Result<bool> {
        let source = self.text();
        let data: Value = serde_json::from_str(&source).map_err(io::Error::other)?;
        let pretty = serde_json::to_string_pretty(&data).map_err(io::Error::other)?;
        self.set_text(pretty);
        self.validate();
        Ok(true)
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.document.path {
            if path.exists() {
                let backup = backup_path_for(path);
                ProjectStorage::write_atomic_with_backup(
                    path,
                    self.text().as_bytes(),
                    BackupPolicy::new(&backup, DEFAULT_BACKUP_GENERATIONS),
                )
                .map_err(io::Error::from)?;
                self.document.last_saved_backup = Some(backup);
            } else {
                ProjectStorage::write_atomic(path, self.text().as_bytes())
                    .map_err(io::Error::from)?;
            }
            self.document.dirty = false;
            self.validate();
            self.sync_active_document();
        }
        Ok(())
    }

    pub fn close_tab(&mut self, path: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        Ok(self
            .close_tab_with_choice(path, CloseDocumentChoice::Discard)?
            .active)
    }

    pub fn close_current_tab(&mut self) -> io::Result<Option<PathBuf>> {
        Ok(self
            .close_current_tab_with_choice(CloseDocumentChoice::Discard)?
            .active)
    }

    pub fn close_current_tab_with_choice(
        &mut self,
        choice: CloseDocumentChoice,
    ) -> io::Result<CloseDocumentOutcome> {
        self.sync_active_document();
        let outcome = self.document_manager.close_active(choice)?;
        self.after_close(outcome)
    }

    pub fn close_tab_with_choice(
        &mut self,
        path: impl AsRef<Path>,
        choice: CloseDocumentChoice,
    ) -> io::Result<CloseDocumentOutcome> {
        self.sync_active_document();
        let outcome = self.document_manager.close(path, choice)?;
        self.after_close(outcome)
    }

    pub fn activate_next_tab(&mut self, direction: i32) -> io::Result<Option<PathBuf>> {
        self.sync_active_document();
        if self.document_manager.tabs.tabs.is_empty() && self.tabs.is_empty() {
            return Ok(None);
        }
        if self.document_manager.tabs.tabs.is_empty() {
            self.document_manager
                .tabs
                .sync_from_paths(&self.tabs, self.document.path.clone());
        }
        let Some(path) = self.document_manager.tabs.activate_relative(direction) else {
            return Ok(None);
        };
        self.open(path.clone())?;
        Ok(Some(path))
    }

    pub fn tab_label(path: &Path) -> String {
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("asset")
            .to_string()
    }

    pub fn validate(&mut self) -> bool {
        let source = self.text();
        self.diagnostics.clear();
        self.refresh_outline();
        self.add_size_hints(&source);
        let path = self.document.path.as_deref();
        let language_is_luau = self.document.language.eq_ignore_ascii_case("luau")
            || path
                .and_then(|path| path.extension())
                .and_then(|value| value.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("luau") || ext.eq_ignore_ascii_case("lua")
                });
        if language_is_luau {
            return self.validate_luau(&source);
        }

        let extension = path
            .and_then(|path| path.extension())
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let infer_graph_without_path =
            path.is_none() && source.trim_start().starts_with('{') && source.contains("\"nodes\"");
        if extension != "mfgraph"
            && !infer_graph_without_path
            && !matches!(
                extension,
                "json" | "scene" | "prefab" | "material" | "shader" | "particles"
            )
        {
            self.document.syntax_error = None;
            self.sync_active_document();
            return true;
        }

        let data = match serde_json::from_str::<Value>(&source) {
            Ok(data) => data,
            Err(error) => {
                self.set_error(Some(error.line()), format!("JSON invalido: {error}"));
                self.sync_active_document();
                return false;
            }
        };
        if extension != "mfgraph" && !infer_graph_without_path {
            self.document.syntax_error = None;
            self.sync_active_document();
            return true;
        }
        let Some(nodes) = data.get("nodes").and_then(Value::as_array) else {
            self.set_error(None, "Graph is missing nodes array".to_string());
            self.sync_active_document();
            return false;
        };
        if nodes.is_empty() {
            self.set_error(None, "Graph has no nodes".to_string());
            self.sync_active_document();
            return false;
        }
        if !nodes.iter().any(|node| {
            node.get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.starts_with("Event"))
        }) {
            self.set_error(None, "Graph needs an Event node".to_string());
            self.sync_active_document();
            return false;
        }
        self.validate_graph_links(nodes);
        self.outline = nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let id = node.get("id").and_then(Value::as_str)?;
                let node_type = node.get("type").and_then(Value::as_str).unwrap_or("Node");
                Some(ScriptSymbol {
                    name: id.to_string(),
                    line: index + 1,
                    kind: node_type.to_string(),
                })
            })
            .collect();
        self.document.syntax_error = None;
        self.sync_active_document();
        true
    }

    fn validate_luau(&mut self, source: &str) -> bool {
        let name = self
            .document
            .path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|value| value.to_str())
            .unwrap_or("script.luau");
        let diagnostics =
            crate::engine::luau_scripting::LuauScriptRuntime::validate_source_diagnostics(
                source, name,
            );
        if let Some(diagnostic) = diagnostics.into_iter().next() {
            let message = match diagnostic.column {
                Some(column) => format!("columna {column}: {}", diagnostic.message),
                None => diagnostic.message,
            };
            self.set_error(diagnostic.line, format!("Luau inválido: {message}"));
            self.sync_active_document();
            false
        } else {
            self.add_luau_api_hints(source);
            self.document.syntax_error = None;
            self.sync_active_document();
            true
        }
    }

    pub fn stats(&self) -> ScriptEditorStats {
        let graph_nodes = serde_json::from_str::<Value>(&self.text())
            .ok()
            .and_then(|data| data.get("nodes").and_then(Value::as_array).map(Vec::len))
            .unwrap_or(0);
        ScriptEditorStats {
            lines: self.lines.len(),
            characters: self.text().chars().count(),
            functions: self
                .outline
                .iter()
                .filter(|symbol| symbol.kind == "function")
                .count(),
            graph_nodes,
            diagnostics: self.diagnostics.len(),
        }
    }

    pub fn code_actions(&self) -> Vec<ScriptEditorCodeAction> {
        let mut actions = Vec::new();
        let text = self.text();
        let extension = self
            .document
            .path
            .as_deref()
            .and_then(|path| path.extension())
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if matches!(extension, "luau" | "lua") || self.document.path.is_none() {
            if !text.contains("function on_start") {
                actions.push(code_action("Insert on_start()", "luau.template", None));
            }
            if !text.contains("function on_update") {
                actions.push(code_action("Insert on_update(dt)", "luau.template", None));
            }
            if !text.contains("function on_collision_enter") {
                actions.push(code_action(
                    "Insert on_collision_enter(other)",
                    "luau.template",
                    None,
                ));
            }
            if !text.contains("function on_fixed_update") {
                actions.push(code_action(
                    "Insert on_fixed_update(dt)",
                    "luau.template",
                    None,
                ));
            }
            actions.push(code_action(
                "Insert 2D controller template",
                "luau.template.controller2d",
                None,
            ));
            actions.push(code_action(
                "Insert sprite state machine",
                "luau.template.sprite_state",
                None,
            ));
            actions.push(code_action(
                "Insert interaction trigger",
                "luau.template.interaction",
                None,
            ));
            actions.push(code_action(
                "Attach script to selected entity",
                "scene.assign_script",
                None,
            ));
        }
        if extension == "mfgraph" || text.contains("\"nodes\"") {
            actions.push(code_action(
                "Open in Blueprint editor",
                "blueprint.open_visual",
                None,
            ));
            actions.push(code_action(
                "Auto layout Blueprint nodes",
                "blueprint.auto_layout",
                None,
            ));
        }
        if extension == "json" || extension == "mfgraph" || text.trim_start().starts_with('{') {
            actions.push(code_action("Format JSON", "format.json", None));
        }
        actions.extend(self.diagnostics.iter().map(|diagnostic| {
            code_action(
                match diagnostic.severity {
                    ScriptDiagnosticSeverity::Error => "Jump to error",
                    ScriptDiagnosticSeverity::Warning => "Review warning",
                    ScriptDiagnosticSeverity::Hint => "Review hint",
                },
                "diagnostic.jump",
                diagnostic.line,
            )
        }));
        actions
    }

    pub fn apply_code_action(&mut self, action: &ScriptEditorCodeAction) -> io::Result<bool> {
        match action.kind.as_str() {
            "format.json" => self.format_json_pretty(),
            "luau.template.controller2d"
            | "luau.template.sprite_state"
            | "luau.template.interaction" => Ok(self.insert_luau_template(&action.kind)),
            "luau.template" => {
                let event = if action.title.contains("on_update") {
                    "on_update"
                } else if action.title.contains("on_key_down") {
                    "on_key_down"
                } else if action.title.contains("on_collision_enter") {
                    "on_collision_enter"
                } else if action.title.contains("on_fixed_update") {
                    "on_fixed_update"
                } else {
                    "on_start"
                };
                Ok(self.insert_luau_event_template(event))
            }
            "blueprint.auto_layout" => self.auto_layout_graph_json(),
            _ => Ok(false),
        }
    }

    pub fn insert_luau_event_template(&mut self, event: &str) -> bool {
        let template = match event {
            "on_start" => "function on_start()\n    -- called once when the entity starts\nend\n",
            "on_update" => "function on_update(dt: number)\n    -- called every frame\nend\n",
            "on_key_down" => {
                "function on_key_down(key: string)\n    -- keyboard input event\nend\n"
            }
            "on_collision_enter" => {
                "function on_collision_enter(other: string)\n    -- collision enter event\nend\n"
            }
            "on_fixed_update" => {
                "function on_fixed_update(dt: number)\n    -- deterministic physics step\nend\n"
            }
            "on_ready" => "function on_ready()\n    -- called after on_create/on_start\nend\n",
            "on_destroy" => {
                "function on_destroy()\n    -- cleanup before entity is destroyed\nend\n"
            }
            "on_event" => {
                "function on_event(name: string, payload)\n    -- custom gameplay event\nend\n"
            }
            _ => return false,
        };
        if self.text().contains(&format!("function {event}")) {
            return false;
        }
        if !self.lines.is_empty() && self.lines.last().is_some_and(|line| !line.is_empty()) {
            self.lines.push(String::new());
        }
        self.lines.extend(split_text_lines(template));
        self.document.dirty = true;
        self.document.language = "luau".to_string();
        self.sync_active_document();
        self.validate();
        true
    }

    pub fn insert_luau_template(&mut self, template_name: &str) -> bool {
        let template = match template_name {
            "luau.template.controller2d" => {
                r#"local speed = 180.0
local jump_impulse = 280.0

function on_start()
    set_sprite("assets/sprites/player.sprite.json")
end

function on_update(dt: number)
    local x = Input.axis("A", "D")
    local y = Input.axis("W", "S")
    move(x * speed * dt, y * speed * dt)

    if x < 0 then
        face_left()
    elseif x > 0 then
        face_right()
    end

    if Input.action_pressed("jump") then
        Rigidbody2D.apply_impulse(Entity.current(), 0.0, -jump_impulse)
    end
end
"#
            }
            "luau.template.sprite_state" => {
                r#"local current_state = "idle"

local function play_state(state: string)
    if current_state == state then
        return
    end
    current_state = state
    play_sprite_animation("assets/animations/player.spriteframes", state)
end

function on_update(dt: number)
    local moving = math.abs(Input.axis("A", "D")) > 0.01
    if moving then
        play_state("run")
    else
        play_state("idle")
    end
end
"#
            }
            "luau.template.interaction" => {
                r#"local prompt = "Press E"

function on_start()
    set_ui_text("HUD_Prompt", "")
end

function on_collision_enter(other: string)
    set_ui_text("HUD_Prompt", prompt)
end

function on_collision_exit(other: string)
    set_ui_text("HUD_Prompt", "")
end

function on_key_down(key: string)
    if key == "E" then
        Events.emit("interact", { entity = entity_name })
    end
end
"#
            }
            _ => return false,
        };
        if !self.lines.is_empty() && self.lines.iter().any(|line| !line.trim().is_empty()) {
            self.lines.push(String::new());
        }
        self.lines.extend(split_text_lines(template.trim_end()));
        self.document.dirty = true;
        self.document.language = "luau".to_string();
        self.sync_active_document();
        self.validate();
        true
    }

    pub fn insert_snippet(
        &mut self,
        name: &str,
        line: usize,
        column: usize,
    ) -> Option<(usize, usize)> {
        let snippet = match name {
            "log" => "Debug.log(\"message\")",
            "spawn" => "local target = Spawner.spawn(\"PrefabName\", x, y)",
            "sprite" => "set_sprite(\"assets/sprites/player.sprite.json\")\nface_right()",
            "anim" => {
                "play_sprite_animation(\"assets/animations/player.spriteframes\", \"default\")"
            }
            "ui" => "set_ui_text(\"HUD_Status\", \"Ready\")",
            "dialogue" => "set_ui_text(\"HUD_Dialogue\", \"New line\")",
            "choice" => {
                "if input_pressed(\"1\") then\n    set_ui_text(\"HUD_Dialogue\", \"Choice A\")\nelseif input_pressed(\"2\") then\n    set_ui_text(\"HUD_Dialogue\", \"Choice B\")\nend"
            }
            "quest" => {
                "add_quest(\"quest_01\", \"New Quest\", \"objective_01\", \"Do something\", 1)"
            }
            "ability" => {
                "if input_pressed(\"Space\") then\n    set_ui_text(\"HUD_Status\", \"Ability fired\")\nend"
            }
            "timer" => "Task.delay(1.0, function()\n    Debug.log(\"timer finished\")\nend)",
            "blackboard" => "set_blackboard(\"key\", value)",
            "signal" => "Events.emit(\"EventName\", {})",
            "controller2d" => {
                "local speed = 180.0\nfunction on_update(dt: number)\n    local x = Input.axis(\"A\", \"D\")\n    move(x * speed * dt, 0.0)\nend"
            }
            "sprite_state" => {
                "local state = \"idle\"\nplay_sprite_animation(\"assets/animations/player.spriteframes\", state)"
            }
            "camera_follow" => "Camera.main():follow(Entity.current())",
            "projectile" => {
                "local shot = Spawner.spawn(\"Projectile\", entity.x, entity.y)\nRigidbody2D.set_velocity(shot, 420.0, 0.0)"
            }
            "particles" => "Particles2D.burst(Entity.current(), 18)",
            "safe_event" => "Events.emit(\"EventName\", { source = entity_name })",
            _ => return None,
        };
        let cursor = self.insert_text(line, column, snippet);
        self.validate();
        Some(cursor)
    }

    pub fn search_text(&self, query: &str) -> Vec<ScriptEditorSearchResult> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        let needle = query.to_lowercase();
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let haystack = line.to_lowercase();
                let column = haystack.find(&needle)?;
                Some(ScriptEditorSearchResult {
                    line: index + 1,
                    column,
                    preview: line.trim().chars().take(96).collect(),
                })
            })
            .collect()
    }

    pub fn completions_at(&self, line: usize, column: usize) -> Vec<ScriptEditorCompletion> {
        let prefix = self
            .lines
            .get(line)
            .map(|text| current_identifier_prefix(text, column))
            .unwrap_or_default()
            .to_lowercase();
        let mut completions = Vec::new();
        let text = self.text();
        for (event, signature) in [
            ("on_create", "function on_create()\n    \nend"),
            ("on_ready", "function on_ready()\n    \nend"),
            ("on_start", "function on_start()\n    \nend"),
            ("on_update", "function on_update(dt: number)\n    \nend"),
            (
                "on_fixed_update",
                "function on_fixed_update(dt: number)\n    \nend",
            ),
            (
                "on_key_down",
                "function on_key_down(key: string)\n    \nend",
            ),
            (
                "on_collision_enter",
                "function on_collision_enter(other: string)\n    \nend",
            ),
            (
                "on_collision_exit",
                "function on_collision_exit(other: string)\n    \nend",
            ),
            (
                "on_event",
                "function on_event(name: string, payload)\n    \nend",
            ),
            ("on_destroy", "function on_destroy()\n    \nend"),
        ] {
            if !text.contains(&format!("function {event}")) {
                completions.push(ScriptEditorCompletion {
                    label: event.to_string(),
                    detail: "Luau event".to_string(),
                    insert_text: signature.to_string(),
                });
            }
        }
        for keyword in [
            "local", "function", "if", "then", "else", "elseif", "for", "while", "do", "end",
            "return", "true", "false", "nil",
        ] {
            completions.push(ScriptEditorCompletion {
                label: keyword.to_string(),
                detail: "Luau keyword".to_string(),
                insert_text: keyword.to_string(),
            });
        }
        for (label, detail, insert_text) in [
            ("move", "MiniForge API", "move(x, y);"),
            ("input_pressed", "MiniForge API", "input_pressed(\"E\")"),
            (
                "set_sprite",
                "MiniForge API",
                "set_sprite(\"assets/sprites/player.sprite.json\");",
            ),
            (
                "play_sprite_animation",
                "MiniForge API",
                "play_sprite_animation(\"assets/animations/player.spriteframes\", \"default\");",
            ),
            ("face_left", "MiniForge API", "face_left();"),
            ("face_right", "MiniForge API", "face_right();"),
            (
                "set_sprite_flip",
                "MiniForge API",
                "set_sprite_flip(true, false);",
            ),
            (
                "set_ui_text",
                "MiniForge API",
                "set_ui_text(\"HUD_Status\", \"Ready\");",
            ),
            ("ui_text", "MiniForge API", "ui_text(\"Ready\");"),
            ("spawn", "MiniForge API", "spawn(\"PrefabName\", x, y);"),
            ("play_sound", "MiniForge API", "play_sound(\"SfxName\");"),
            (
                "set_blackboard",
                "MiniForge API",
                "set_blackboard(\"key\", value);",
            ),
            (
                "add_quest",
                "MiniForge API",
                "add_quest(\"quest_01\", \"New Quest\", \"objective_01\", \"Do something\", 1);",
            ),
            (
                "quest_progress",
                "MiniForge API",
                "quest_progress(\"quest_01\", \"objective_01\", 1);",
            ),
            ("trigger_ability", "MiniForge API", "trigger_ability();"),
            ("Vector2", "MiniForge API", "Vector2.new(0.0, 0.0)"),
            (
                "Vector2.distance",
                "MiniForge API",
                "Vector2.distance(a, b)",
            ),
            (
                "Vector2.move_towards",
                "MiniForge API",
                "Vector2.move_towards(current, target, speed * dt)",
            ),
            ("Input.axis", "MiniForge API", "Input.axis(\"A\", \"D\")"),
            (
                "Input.action_pressed",
                "MiniForge API",
                "Input.action_pressed(\"jump\")",
            ),
            ("Time.delta_time", "MiniForge API", "Time.delta_time"),
            ("Entity.current", "MiniForge API", "Entity.current()"),
            (
                "Entity.spawn",
                "MiniForge API",
                "Entity.spawn(\"EntityName\", x, y, { tag = \"Gameplay\" })",
            ),
            (
                "Entity.find",
                "MiniForge API",
                "Entity.find(\"EntityName\")",
            ),
            (
                "Entity.nearby",
                "MiniForge API",
                "Entity.nearby(Entity.current(), radius, { tag = \"Enemy\" })",
            ),
            (
                "Entity.nearest",
                "MiniForge API",
                "Entity.nearest(Entity.current(), radius, { tag = \"Enemy\" })",
            ),
            (
                "Entity.exists",
                "MiniForge API",
                "Entity.exists(\"EntityName\")",
            ),
            (
                "Entity.all_with_tag",
                "MiniForge API",
                "Entity.all_with_tag(\"Enemy\")",
            ),
            (
                "Entity.count_with_tag",
                "MiniForge API",
                "Entity.count_with_tag(\"Enemy\")",
            ),
            (
                "Component.add",
                "MiniForge API",
                "Component.add(Entity.current(), \"Component\", {});",
            ),
            (
                "Component.get",
                "MiniForge API",
                "Component.get(Entity.current(), \"Health\", \"health\", 0)",
            ),
            (
                "Component.set",
                "MiniForge API",
                "Component.set(Entity.current(), \"Component\", \"key\", value);",
            ),
            (
                "Component.remove",
                "MiniForge API",
                "Component.remove(Entity.current(), \"Component\");",
            ),
            (
                "Component.has",
                "MiniForge API",
                "Component.has(Entity.current(), \"Component\")",
            ),
            (
                "Transform2D.set_position",
                "MiniForge API",
                "Transform2D.set_position(Entity.current(), x, y);",
            ),
            (
                "Rigidbody2D.set_velocity",
                "MiniForge API",
                "Rigidbody2D.set_velocity(Entity.current(), x, y);",
            ),
            (
                "Rigidbody2D.apply_impulse",
                "MiniForge API",
                "Rigidbody2D.apply_impulse(Entity.current(), x, y);",
            ),
            (
                "Camera.main",
                "MiniForge API",
                "Camera.main():follow(Entity.current());",
            ),
            (
                "Physics2D.raycast",
                "MiniForge API",
                "Physics2D.raycast(origin, target, { mask = Layers.WORLD })",
            ),
            (
                "AnimationPlayer.play",
                "MiniForge API",
                "AnimationPlayer.play(Entity.current(), \"Run\");",
            ),
            (
                "Tween.to",
                "MiniForge API",
                "Tween.to(Entity.current(), \"position.x\", x, duration, { easing = \"linear\" });",
            ),
            (
                "Navigation2D.set_destination",
                "MiniForge API",
                "Navigation2D.set_destination(Entity.current(), x, y);",
            ),
            (
                "Audio2D.play",
                "MiniForge API",
                "Audio2D.play(\"SfxName\", { bus = \"SFX\", volume = 1.0 });",
            ),
            (
                "Particles2D.burst",
                "MiniForge API",
                "Particles2D.burst(Entity.current(), 16);",
            ),
            (
                "Spawner.spawn",
                "MiniForge API",
                "Spawner.spawn(\"EntityName\", x, y)",
            ),
            (
                "Task.delay",
                "MiniForge API",
                "Task.delay(1.0, function()\n    \nend)",
            ),
            ("Debug.log", "MiniForge API", "Debug.log(\"message\");"),
            (
                "Events.emit",
                "MiniForge API",
                "Events.emit(\"EventName\", { source = entity_name });",
            ),
            (
                "Assets.exists",
                "MiniForge API",
                "Assets.exists(\"assets/sprites/player.png\")",
            ),
        ] {
            completions.push(ScriptEditorCompletion {
                label: label.to_string(),
                detail: detail.to_string(),
                insert_text: insert_text.to_string(),
            });
        }
        for symbol in &self.outline {
            completions.push(ScriptEditorCompletion {
                label: symbol.name.clone(),
                detail: symbol.kind.clone(),
                insert_text: symbol.name.clone(),
            });
        }
        completions.retain(|completion| {
            prefix.is_empty() || completion.label.to_lowercase().starts_with(&prefix)
        });
        completions.sort_by(|a, b| a.label.cmp(&b.label));
        completions.dedup_by(|a, b| a.label == b.label);
        completions
    }

    pub fn rename_symbol(&mut self, old_name: &str, new_name: &str) -> usize {
        if old_name.trim().is_empty()
            || new_name.trim().is_empty()
            || !is_identifier(new_name)
            || old_name == new_name
        {
            return 0;
        }
        let mut replacements = 0usize;
        for line in &mut self.lines {
            let (updated, count) = replace_identifier(line, old_name, new_name);
            if count > 0 {
                *line = updated;
                replacements += count;
            }
        }
        if replacements > 0 {
            self.document.dirty = true;
            self.sync_active_document();
            self.validate();
        }
        replacements
    }

    pub fn minimap(&self) -> Vec<ScriptEditorMiniMapLine> {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let line_number = index + 1;
                let trimmed = line.trim_start();
                let has_diagnostic = self
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.line == Some(line_number));
                let kind = if has_diagnostic {
                    "diagnostic"
                } else if luau_function_symbol(line, line_number).is_some() {
                    "function"
                } else if trimmed.starts_with("//") || trimmed.starts_with("--") {
                    "comment"
                } else if trimmed.contains("\"id\"") || trimmed.contains("\"type\"") {
                    "graph_node"
                } else if trimmed.is_empty() {
                    "blank"
                } else {
                    "code"
                };
                ScriptEditorMiniMapLine {
                    line: line_number,
                    kind: kind.to_string(),
                    intensity: trimmed.len().min(96),
                }
            })
            .collect()
    }

    pub fn diagnostic_summary(&self) -> ScriptDiagnosticSummary {
        let mut summary = ScriptDiagnosticSummary::default();
        for diagnostic in &self.diagnostics {
            match diagnostic.severity {
                ScriptDiagnosticSeverity::Error => summary.errors += 1,
                ScriptDiagnosticSeverity::Warning => summary.warnings += 1,
                ScriptDiagnosticSeverity::Hint => summary.hints += 1,
            }
        }
        summary
    }

    pub fn find_symbols(&self, query: &str) -> Vec<&ScriptSymbol> {
        let query = query.to_lowercase();
        self.outline
            .iter()
            .filter(|symbol| {
                query.is_empty()
                    || symbol.name.to_lowercase().contains(&query)
                    || symbol.kind.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn is_dirty(&self, path: impl AsRef<Path>) -> bool {
        if self.document.path.as_deref() == Some(path.as_ref()) {
            return self.document.dirty;
        }
        self.document_manager.is_dirty(path)
    }

    pub fn checkpoint_documents(&mut self) -> (Vec<EditorDocument>, Option<PathBuf>) {
        self.sync_active_document();
        let documents = self
            .document_manager
            .tabs
            .tabs
            .iter()
            .filter_map(|path| self.document_manager.documents.get(path).cloned())
            .collect();
        (documents, self.document_manager.tabs.active.clone())
    }

    pub fn has_dirty_documents(&mut self) -> bool {
        self.sync_active_document();
        self.document_manager
            .documents
            .values()
            .any(|document| document.dirty)
    }

    pub fn restore_documents(
        &mut self,
        documents: Vec<EditorDocument>,
        active: Option<PathBuf>,
    ) -> usize {
        self.document_manager = DocumentManager::default();
        for document in documents {
            self.document_manager.upsert(document);
        }
        let restored = self.document_manager.documents.len();
        let active = active
            .filter(|path| self.document_manager.documents.contains_key(path))
            .or_else(|| self.document_manager.tabs.tabs.first().cloned());
        if let Some(path) = active {
            self.document_manager.tabs.activate(&path);
            if let Some(document) = self.document_manager.get(&path).cloned() {
                self.apply_editor_document(&document);
                self.validate();
            }
        } else {
            self.document = ScriptDocument::default();
            self.lines = vec![String::new()];
        }
        self.sync_tabs_from_manager();
        restored
    }

    fn after_close(&mut self, outcome: CloseDocumentOutcome) -> io::Result<CloseDocumentOutcome> {
        if outcome.cancelled {
            return Ok(outcome);
        }
        if let Some(path) = &outcome.active {
            if let Some(document) = self.document_manager.get(path).cloned() {
                self.apply_editor_document(&document);
            } else {
                self.open(path.clone())?;
            }
        } else {
            self.document = ScriptDocument::default();
            self.lines = vec![String::new()];
        }
        self.sync_tabs_from_manager();
        Ok(outcome)
    }

    fn sync_active_document(&mut self) {
        let Some(path) = self.document.path.clone() else {
            return;
        };
        let mut document =
            EditorDocument::from_text(path.clone(), self.text(), self.document.dirty);
        document.syntax_error = self.document.syntax_error.clone();
        document.language = self.document.language.clone();
        document.last_saved_backup = self.document.last_saved_backup.clone();
        self.document_manager.upsert(document);
        self.sync_tabs_from_manager();
    }

    fn apply_editor_document(&mut self, document: &EditorDocument) {
        self.lines = split_text_lines(&document.text);
        self.document.path = Some(document.path.clone());
        self.document.language = document.language.clone();
        self.document.syntax_error = document.syntax_error.clone();
        self.document.dirty = document.dirty;
        self.document.last_saved_backup = document.last_saved_backup.clone();
    }

    fn sync_tabs_from_manager(&mut self) {
        self.tabs = self.document_manager.tabs.tabs.clone();
        self.closed_tabs = self.document_manager.tabs.closed_tabs.clone();
    }

    fn set_error(&mut self, line: Option<usize>, message: String) {
        self.document.syntax_error = Some(match line {
            Some(line) => format!("line {line}: {message}"),
            None => message.clone(),
        });
        self.diagnostics.push(ScriptDiagnostic {
            line,
            severity: ScriptDiagnosticSeverity::Error,
            message,
        });
    }

    fn push_hint(&mut self, line: Option<usize>, message: impl Into<String>) {
        self.diagnostics.push(ScriptDiagnostic {
            line,
            severity: ScriptDiagnosticSeverity::Hint,
            message: message.into(),
        });
    }

    fn push_warning(&mut self, line: Option<usize>, message: impl Into<String>) {
        self.diagnostics.push(ScriptDiagnostic {
            line,
            severity: ScriptDiagnosticSeverity::Warning,
            message: message.into(),
        });
    }

    fn add_size_hints(&mut self, source: &str) {
        if self.lines.len() > 1200 {
            self.push_warning(
                None,
                "Large script: consider splitting gameplay into multiple scripts or graphs.",
            );
        }
        if source.len() > 128_000 {
            self.push_warning(
                None,
                "Large file: editor remains safe, but smaller assets are easier to diff and hot reload.",
            );
        }
    }

    fn add_luau_api_hints(&mut self, source: &str) {
        let mut pending = Vec::new();
        for (index, line) in self.lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("function ") && trimmed.contains("on_update()") {
                pending.push((
                    ScriptDiagnosticSeverity::Warning,
                    Some(index + 1),
                    "on_update should accept dt: use function on_update(dt: number).".to_string(),
                ));
            }
            if trimmed.contains("spawn(") {
                pending.push((
                    ScriptDiagnosticSeverity::Hint,
                    Some(index + 1),
                    "spawn(...) enqueues a new entity for the end of the Luau callback."
                        .to_string(),
                ));
            }
            if trimmed.contains("input_pressed(")
                && !source.contains("function on_update")
                && !source.contains("function on_key_down")
            {
                pending.push((
                    ScriptDiagnosticSeverity::Hint,
                    Some(index + 1),
                    "input_pressed works best inside on_update(dt) or on_key_down(key)."
                        .to_string(),
                ));
            }
            if trimmed.contains("while true do") {
                pending.push((
                    ScriptDiagnosticSeverity::Warning,
                    Some(index + 1),
                    "Avoid unbounded while true loops in gameplay scripts; use on_update(dt) or timers."
                        .to_string(),
                ));
            }
            if trimmed.contains("task.wait(") || trimmed.contains("wait(") {
                pending.push((
                    ScriptDiagnosticSeverity::Hint,
                    Some(index + 1),
                    "MiniForge Luau is frame-event driven; prefer Time.delta_time and on_update(dt)."
                        .to_string(),
                ));
            }
            if trimmed.contains("game.Workspace")
                || trimmed.contains("script.Parent")
                || trimmed.contains("Instance.new")
            {
                pending.push((
                    ScriptDiagnosticSeverity::Warning,
                    Some(index + 1),
                    "Roblox globals are not available here; use Entity, Component, Spawner and Assets."
                        .to_string(),
                ));
            }
            if trimmed.contains("move(") && !trimmed.contains("dt") {
                pending.push((
                    ScriptDiagnosticSeverity::Hint,
                    Some(index + 1),
                    "Movement inside on_update should usually be multiplied by dt for stable speed."
                        .to_string(),
                ));
            }
        }
        for (severity, line, message) in pending {
            self.diagnostics.push(ScriptDiagnostic {
                line,
                severity,
                message,
            });
        }
    }

    fn validate_graph_links(&mut self, nodes: &[Value]) {
        const LINK_KEYS: &[&str] = &[
            "next",
            "true_next",
            "false_next",
            "a_next",
            "b_next",
            "on_enter",
            "on_exit",
        ];
        let mut ids = BTreeSet::new();
        let mut duplicates = BTreeSet::new();
        for node in nodes {
            let Some(id) = node.get("id").and_then(Value::as_str) else {
                continue;
            };
            if !ids.insert(id.to_string()) {
                duplicates.insert(id.to_string());
            }
        }
        for duplicate in duplicates {
            self.push_warning(
                None,
                format!("Graph contains duplicate node id: {duplicate}"),
            );
        }
        for (index, node) in nodes.iter().enumerate() {
            for key in LINK_KEYS {
                let Some(target) = node.get(*key).and_then(Value::as_str) else {
                    continue;
                };
                if target.is_empty() || ids.contains(target) {
                    continue;
                }
                let node_id = node.get("id").and_then(Value::as_str).unwrap_or("node");
                self.push_warning(
                    Some(index + 1),
                    format!("{node_id}.{key} references missing node '{target}'"),
                );
            }
        }
    }

    fn refresh_outline(&mut self) {
        let path = self.document.path.as_deref();
        let extension = path
            .and_then(|path| path.extension())
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if matches!(extension, "luau" | "lua") || path.is_none() {
            self.outline = self
                .lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| luau_function_symbol(line, index + 1))
                .collect();
            if self.outline.is_empty()
                && self
                    .lines
                    .iter()
                    .any(|line| !line.trim().is_empty() && !line.trim_start().starts_with("--"))
            {
                self.push_hint(
                    None,
                    "No Luau functions found. Expected on_start, on_update, on_key_down, or a custom function.",
                );
            }
        }
    }

    fn auto_layout_graph_json(&mut self) -> io::Result<bool> {
        let mut data: Value = serde_json::from_str(&self.text()).map_err(io::Error::other)?;
        let Some(nodes) = data.get_mut("nodes").and_then(Value::as_array_mut) else {
            return Ok(false);
        };
        for (index, node) in nodes.iter_mut().enumerate() {
            let Some(object) = node.as_object_mut() else {
                continue;
            };
            let depth = index % 5;
            let row = index / 5;
            object.insert("x".to_string(), Value::from((depth as i64) * 260));
            object.insert("y".to_string(), Value::from((row as i64) * 150));
        }
        let pretty = serde_json::to_string_pretty(&data).map_err(io::Error::other)?;
        self.set_text(pretty);
        self.validate();
        Ok(true)
    }
}

fn split_text_lines(text: &str) -> Vec<String> {
    let mut lines = text.lines().map(ToString::to_string).collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn code_action(title: &str, kind: &str, line: Option<usize>) -> ScriptEditorCodeAction {
    ScriptEditorCodeAction {
        title: title.to_string(),
        kind: kind.to_string(),
        line,
    }
}

fn resolve_project_path(project_path: &Path, path: &Path) -> io::Result<PathBuf> {
    let project_path = if project_path.is_absolute() {
        project_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(project_path)
    };
    let project_path = project_path.canonicalize().unwrap_or(project_path);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_path.join(path)
    };
    let resolved = if resolved.exists() {
        resolved.canonicalize().unwrap_or(resolved)
    } else {
        resolved
    };
    if !resolved.starts_with(&project_path) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Archivo fuera del proyecto: {}", resolved.display()),
        ));
    }
    Ok(resolved)
}

fn clamp_to_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn previous_char_boundary(text: &str, index: usize) -> usize {
    let mut previous = 0;
    for (offset, _) in text.char_indices() {
        if offset >= index {
            break;
        }
        previous = offset;
    }
    previous
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    let index = clamp_to_char_boundary(text, index);
    text[index..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| index + offset)
        .unwrap_or(text.len())
}

fn luau_function_symbol(line: &str, line_number: usize) -> Option<ScriptSymbol> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("function ")
        .or_else(|| trimmed.strip_prefix("local function "))?;
    let name = rest.split('(').next().unwrap_or("").trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(ScriptSymbol {
        name,
        line: line_number,
        kind: "function".to_string(),
    })
}

fn current_identifier_prefix(line: &str, column: usize) -> String {
    let column = clamp_to_char_boundary(line, column);
    let head = &line[..column];
    head.chars()
        .rev()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || matches!(*character, '_' | '.')
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn replace_identifier(line: &str, old_name: &str, new_name: &str) -> (String, usize) {
    let mut output = String::with_capacity(line.len());
    let mut replacements = 0usize;
    let mut index = 0usize;
    while let Some(relative) = line[index..].find(old_name) {
        let start = index + relative;
        let end = start + old_name.len();
        output.push_str(&line[index..start]);
        if is_identifier_boundary(line, start, end) {
            output.push_str(new_name);
            replacements += 1;
        } else {
            output.push_str(&line[start..end]);
        }
        index = end;
    }
    output.push_str(&line[index..]);
    (output, replacements)
}

fn is_identifier_boundary(line: &str, start: usize, end: usize) -> bool {
    let before = line[..start]
        .chars()
        .next_back()
        .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
    let after = line[end..]
        .chars()
        .next()
        .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
    before && after
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luau_templates_snippets_and_completions_support_gameplay_workflow() {
        let mut editor = ScriptEditor::default();
        assert!(editor.insert_luau_template("luau.template.controller2d"));
        assert!(editor.text().contains("function on_update(dt: number)"));
        assert!(editor.text().contains("Input.axis"));

        let completions = editor.completions_at(0, 0);
        assert!(
            completions
                .iter()
                .any(|completion| completion.label == "Events.emit")
        );

        let cursor = editor.insert_snippet("projectile", 0, 0);
        assert!(cursor.is_some());
        assert!(editor.text().contains("Spawner.spawn"));

        editor.set_text("Entity.ne");
        let member_completions = editor.completions_at(0, "Entity.ne".len());
        assert!(
            member_completions
                .iter()
                .any(|completion| completion.label == "Entity.nearest")
        );
        assert!(
            member_completions
                .iter()
                .any(|completion| completion.label == "Entity.nearby")
        );
    }

    #[test]
    fn luau_diagnostics_catch_common_wrong_runtime_patterns() {
        let mut editor = ScriptEditor::default();
        editor.document.language = "luau".to_string();
        editor.set_text(
            "-- movement script\nfunction on_update()\n    while true do\n        wait(1)\n    end\n    game.Workspace.Part = script.Parent\n    move(4, 0)\nend",
        );
        assert!(!editor.validate() || !editor.diagnostics.is_empty());
        let messages = editor
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages
                .iter()
                .any(|message| message.contains("on_update should accept dt"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("Roblox globals"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("while true"))
        );
        let minimap = editor.minimap();
        assert!(minimap.iter().any(|line| line.kind == "diagnostic"));
        assert!(minimap.iter().any(|line| line.kind == "comment"));
    }

    #[test]
    fn luau_compile_errors_and_runtime_snippets_are_actionable() {
        let mut editor = ScriptEditor::default();
        editor.document.language = "luau".to_string();
        editor.set_text("local ok = true\nfunction on_update(\nend");
        assert!(!editor.validate());
        assert!(editor.diagnostics[0].line.is_some());

        editor.set_text("");
        assert!(editor.insert_snippet("timer", 0, 0).is_some());
        assert!(editor.text().contains("Task.delay"));
        assert!(editor.text().contains("Debug.log"));
    }
}
