use std::fs;
use std::io;
use std::path::PathBuf;

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ScriptDocument {
    pub path: Option<PathBuf>,
    pub syntax_error: Option<String>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptEditor {
    pub document: ScriptDocument,
    pub lines: Vec<String>,
    pub tabs: Vec<PathBuf>,
}

impl ScriptEditor {
    pub fn open(&mut self, path: PathBuf) -> io::Result<()> {
        let text = fs::read_to_string(&path)?;
        self.lines = text.lines().map(ToString::to_string).collect();
        self.document.path = Some(path.clone());
        if !self.tabs.contains(&path) {
            self.tabs.push(path);
        }
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.document.path {
            fs::write(path, self.lines.join("\n"))?;
            self.document.dirty = false;
        }
        Ok(())
    }

    pub fn validate(&mut self) -> bool {
        let source = self.lines.join("\n");
        let data = match serde_json::from_str::<Value>(&source) {
            Ok(data) => data,
            Err(error) => {
                self.document.syntax_error = Some(format!("Invalid graph JSON: {error}"));
                return false;
            }
        };
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
}
