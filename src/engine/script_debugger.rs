use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::luau_scripting::{
    LuauScriptRuntime, ScriptDebugSnapshot, ScriptQueryFrameStats, ScriptSchedulerFrameStats,
    ScriptTraceEntry, parse_luau_source_location,
};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptDebugIssue {
    pub severity: String,
    pub message: String,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScriptDebugger {
    pub issues: Vec<ScriptDebugIssue>,
    pub active_scripts: Vec<String>,
    pub traces: Vec<ScriptTraceEntry>,
    pub reloads: usize,
    pub watcher_active: bool,
    pub cached_scripts: usize,
    pub persistent_contexts: usize,
    pub last_frame_scripts: usize,
    pub memory_bytes: usize,
    pub scheduler: ScriptSchedulerFrameStats,
    pub queries: ScriptQueryFrameStats,
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
            .map(|error| {
                let (line, column) = parse_luau_source_location(&error);
                ScriptDebugIssue {
                    line,
                    column,
                    path: parse_path(&error),
                    severity: "error".to_string(),
                    message: error,
                }
            })
            .collect();
        self.active_scripts = snapshot.active_scripts;
        self.traces = snapshot.traces;
        self.reloads = snapshot.reload_count;
        self.watcher_active = snapshot.watcher_active;
        self.cached_scripts = snapshot.cached_scripts;
        self.persistent_contexts = snapshot.persistent_contexts;
        self.last_frame_scripts = snapshot.last_frame_scripts;
        self.memory_bytes = snapshot.memory_bytes;
        self.scheduler = snapshot.scheduler;
        self.queries = snapshot.queries;
    }

    pub fn has_errors(&self) -> bool {
        !self.issues.is_empty()
    }
}

fn parse_path(message: &str) -> Option<PathBuf> {
    let lower = message.to_ascii_lowercase();
    let end = lower
        .find(".luau")
        .map(|index| index + ".luau".len())
        .or_else(|| lower.find(".lua").map(|index| index + ".lua".len()))?;
    let prefix = &message[..end];
    let start = prefix
        .rfind('@')
        .map(|index| index + 1)
        .or_else(|| prefix.rfind(": /").map(|index| index + 2))
        .unwrap_or(0);
    let path = prefix[start..].trim();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debugger_extracts_runtime_source_location_and_metrics() {
        let mut debugger = ScriptDebugger::default();
        debugger.apply_snapshot(ScriptDebugSnapshot {
            errors: vec!["/tmp/game/scripts/Player.luau::on_update: /tmp/game/scripts/Player.luau:17:4: bad value".to_string()],
            cached_scripts: 3,
            persistent_contexts: 2,
            last_frame_scripts: 5,
            memory_bytes: 4096,
            scheduler: ScriptSchedulerFrameStats {
                update_candidates: 8,
                update_budget_used: 5,
                ..Default::default()
            },
            queries: ScriptQueryFrameStats {
                nearby_queries: 2,
                nearby_indexed: 2,
                ..Default::default()
            },
            ..Default::default()
        });

        assert_eq!(debugger.issues[0].line, Some(17));
        assert_eq!(debugger.issues[0].column, Some(4));
        assert_eq!(
            debugger.issues[0].path.as_deref(),
            Some(Path::new("/tmp/game/scripts/Player.luau"))
        );
        assert_eq!(debugger.cached_scripts, 3);
        assert_eq!(debugger.last_frame_scripts, 5);
        assert_eq!(debugger.queries.nearby_indexed, 2);
    }
}
