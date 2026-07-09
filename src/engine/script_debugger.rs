use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::luau_scripting::{LuauScriptRuntime, ScriptDebugSnapshot, ScriptTraceEntry};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptDebugIssue {
    pub severity: String,
    pub message: String,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScriptDebugger {
    pub issues: Vec<ScriptDebugIssue>,
    pub active_scripts: Vec<String>,
    pub traces: Vec<ScriptTraceEntry>,
    pub reloads: usize,
    pub watcher_active: bool,
}

impl ScriptDebugger {
    pub fn refresh(
        &mut self,
        runtime: &LuauScriptRuntime,
        project_path: impl AsRef<Path>,
        entities: &[GameObject],
    ) {
        let snapshot = runtime.debug_snapshot(project_path, entities);
        self.apply_snapshot(snapshot);
    }

    pub fn reload(runtime: &mut LuauScriptRuntime) -> usize {
        runtime.reload_all()
    }

    pub fn apply_snapshot(&mut self, snapshot: ScriptDebugSnapshot) {
        self.issues = snapshot
            .errors
            .into_iter()
            .map(|error| ScriptDebugIssue {
                line: parse_line_number(&error),
                path: parse_path(&error),
                severity: "error".to_string(),
                message: error,
            })
            .collect();
        self.active_scripts = snapshot.active_scripts;
        self.traces = snapshot.traces;
        self.reloads = snapshot.reload_count;
        self.watcher_active = snapshot.watcher_active;
    }

    pub fn has_errors(&self) -> bool {
        !self.issues.is_empty()
    }
}

fn parse_line_number(message: &str) -> Option<usize> {
    let marker = "line ";
    let start = message.find(marker)? + marker.len();
    let tail = &message[start..];
    let digits = tail
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn parse_path(message: &str) -> Option<PathBuf> {
    let path = message.split(':').next()?.trim();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}
