use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

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
}

impl ScriptEditor {
    pub fn open(&mut self, path: PathBuf) -> io::Result<()> {
        let text = fs::read_to_string(&path)?;
        self.lines = split_text_lines(&text);
        self.document.path = Some(path.clone());
        self.document.language = language_for_path(&path);
        self.document.syntax_error = None;
        self.document.dirty = false;
        if !self.tabs.contains(&path) {
            self.tabs.push(path.clone());
        }
        self.closed_tabs.retain(|closed| closed != &path);
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
    }

    pub fn insert_char(&mut self, line: usize, column: usize, character: char) -> (usize, usize) {
        let line = line.min(self.lines.len().saturating_sub(1));
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let column = clamp_to_char_boundary(&self.lines[line], column);
        self.lines[line].insert(column, character);
        self.document.dirty = true;
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
            return (line, prev);
        }
        if line > 0 {
            let previous_len = self.lines[line - 1].len();
            let current = self.lines.remove(line);
            self.lines[line - 1].push_str(&current);
            self.document.dirty = true;
            return (line - 1, previous_len);
        }
        (line, column)
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.document.path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            if path.exists() {
                let backup = backup_path_for(path);
                fs::copy(path, &backup)?;
                self.document.last_saved_backup = Some(backup);
            }
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("script.txt");
            let tmp = path.with_file_name(format!("{filename}.tmp"));
            fs::write(&tmp, self.text())?;
            fs::rename(&tmp, path)?;
            self.document.dirty = false;
            self.validate();
        }
        Ok(())
    }

    pub fn close_tab(&mut self, path: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        let path = path.as_ref();
        let Some(index) = self.tabs.iter().position(|tab| tab == path) else {
            return Ok(self.document.path.clone());
        };
        let closed = self.tabs.remove(index);
        self.closed_tabs.push(closed.clone());
        self.closed_tabs.truncate(12);

        if self.document.path.as_deref() != Some(path) {
            return Ok(self.document.path.clone());
        }

        if let Some(next) = self
            .tabs
            .get(
                index
                    .saturating_sub(1)
                    .min(self.tabs.len().saturating_sub(1)),
            )
            .cloned()
        {
            self.open(next.clone())?;
            return Ok(Some(next));
        }

        self.document = ScriptDocument::default();
        self.lines = vec![String::new()];
        Ok(None)
    }

    pub fn close_current_tab(&mut self) -> io::Result<Option<PathBuf>> {
        let Some(path) = self.document.path.clone() else {
            return Ok(None);
        };
        self.close_tab(path)
    }

    pub fn activate_next_tab(&mut self, direction: i32) -> io::Result<Option<PathBuf>> {
        if self.tabs.is_empty() {
            return Ok(None);
        }
        let current = self
            .document
            .path
            .as_ref()
            .and_then(|path| self.tabs.iter().position(|tab| tab == path))
            .unwrap_or(0);
        let len = self.tabs.len() as i32;
        let next = (current as i32 + direction).rem_euclid(len) as usize;
        let path = self.tabs[next].clone();
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
        let path = self.document.path.as_deref();
        if path
            .and_then(|path| path.extension())
            .and_then(|value| value.to_str())
            == Some("rhai")
        {
            return self.validate_rhai(&source);
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
            return true;
        }

        let data = match serde_json::from_str::<Value>(&source) {
            Ok(data) => data,
            Err(error) => {
                self.document.syntax_error = Some(format!("JSON invalido: {error}"));
                return false;
            }
        };
        if extension != "mfgraph" && !infer_graph_without_path {
            self.document.syntax_error = None;
            return true;
        }
        let Some(nodes) = data.get("nodes").and_then(Value::as_array) else {
            self.document.syntax_error = Some("Graph is missing nodes array".to_string());
            return false;
        };
        if nodes.is_empty() {
            self.document.syntax_error = Some("Graph has no nodes".to_string());
            return false;
        }
        if !nodes.iter().any(|node| {
            node.get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.starts_with("Event"))
        }) {
            self.document.syntax_error = Some("Graph needs an Event node".to_string());
            return false;
        }
        self.document.syntax_error = None;
        true
    }

    fn validate_rhai(&mut self, source: &str) -> bool {
        let compile_source = source.replace("spawn(", "spawn_entity(");
        match rhai::Engine::new().compile(compile_source) {
            Ok(_) => {
                self.document.syntax_error = None;
                true
            }
            Err(error) => {
                self.document.syntax_error = Some(format!("Rhai invalido: {error}"));
                false
            }
        }
    }
}

fn split_text_lines(text: &str) -> Vec<String> {
    let mut lines = text.lines().map(ToString::to_string).collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn language_for_path(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    if name.ends_with(".mfgraph") {
        return "visual_graph".to_string();
    }
    if name.ends_with(".scene") {
        return "scene_json".to_string();
    }
    if name.ends_with(".prefab") {
        return "prefab_json".to_string();
    }
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "rhai" => "rhai".to_string(),
        "json" => "json".to_string(),
        "txt" | "md" => "text".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "text".to_string(),
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

fn backup_path_for(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    path.with_file_name(format!("{filename}.bak"))
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
