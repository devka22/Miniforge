//! Periodic editor-session checkpoints for crash recovery.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::engine::document_manager::EditorDocument;
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
use crate::engine::script_editor::ScriptEditor;

pub const SESSION_SCHEMA_VERSION: u32 = 1;
const MAX_BUFFER_BYTES: usize = 2 * 1024 * 1024;
const MAX_TOTAL_BUFFER_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionUiState {
    pub show_console: bool,
    pub show_grid: bool,
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub active_bottom_panel: String,
    pub script_window_open: bool,
    pub sprite_window_open: bool,
    pub blueprint_picker_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDocument {
    pub path: PathBuf,
    pub project_relative: bool,
    pub dirty: bool,
    pub language: String,
    pub buffer: Option<String>,
    #[serde(default)]
    pub buffer_omitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorSessionSnapshot {
    pub schema_version: u32,
    pub engine_version: String,
    pub saved_unix_ms: u128,
    pub project_name: String,
    pub current_scene: String,
    pub scene_dirty: bool,
    pub scene_dirty_reason: String,
    pub active_document: Option<PathBuf>,
    pub active_document_project_relative: bool,
    pub documents: Vec<SessionDocument>,
    pub ui: SessionUiState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionCheckpointReport {
    pub path: PathBuf,
    pub documents: usize,
    pub dirty_buffers: usize,
    pub omitted_buffers: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionRestoreReport {
    pub restored_documents: usize,
    pub restored_dirty_buffers: usize,
    pub missing_documents: Vec<PathBuf>,
    pub omitted_buffers: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SessionRecoveryManager {
    pub project_path: PathBuf,
    pub path: PathBuf,
    pub interval: Duration,
    pub last_checkpoint: Instant,
    pub last_error: Option<String>,
}

impl SessionRecoveryManager {
    pub fn new(project_path: impl AsRef<Path>, interval: Duration) -> Self {
        let project_path = project_path.as_ref().to_path_buf();
        Self {
            path: project_path
                .join(".miniforge")
                .join("recovery")
                .join("editor_session.json"),
            project_path,
            interval,
            last_checkpoint: Instant::now(),
            last_error: None,
        }
    }

    pub fn should_checkpoint(&self) -> bool {
        self.last_checkpoint.elapsed() >= self.interval
    }

    pub fn is_pending(&self) -> bool {
        self.path.is_file()
    }

    pub fn checkpoint(
        &mut self,
        current_scene: &str,
        scene_dirty: bool,
        scene_dirty_reason: &str,
        script_editor: &mut ScriptEditor,
        ui: SessionUiState,
    ) -> io::Result<SessionCheckpointReport> {
        let (editor_documents, active_document) = script_editor.checkpoint_documents();
        let mut total_buffer_bytes: usize = 0;
        let mut dirty_buffers = 0;
        let mut omitted_buffers = Vec::new();
        let documents = editor_documents
            .into_iter()
            .map(|document| {
                let (path, project_relative) = self.store_path(&document.path);
                let mut buffer = None;
                let mut buffer_omitted = false;
                if document.dirty {
                    let bytes = document.text.len();
                    if bytes <= MAX_BUFFER_BYTES
                        && total_buffer_bytes.saturating_add(bytes) <= MAX_TOTAL_BUFFER_BYTES
                    {
                        total_buffer_bytes += bytes;
                        dirty_buffers += 1;
                        buffer = Some(document.text);
                    } else {
                        buffer_omitted = true;
                        omitted_buffers.push(document.path.clone());
                    }
                }
                SessionDocument {
                    path,
                    project_relative,
                    dirty: document.dirty,
                    language: document.language,
                    buffer,
                    buffer_omitted,
                }
            })
            .collect::<Vec<_>>();
        let (active_document, active_document_project_relative) = active_document
            .as_deref()
            .map(|path| self.store_path(path))
            .map(|(path, relative)| (Some(path), relative))
            .unwrap_or((None, false));
        let snapshot = EditorSessionSnapshot {
            schema_version: SESSION_SCHEMA_VERSION,
            engine_version: crate::engine::version::ENGINE_VERSION.to_string(),
            saved_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or(0),
            project_name: self
                .project_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("MiniForgeProject")
                .to_string(),
            current_scene: current_scene.to_string(),
            scene_dirty,
            scene_dirty_reason: scene_dirty_reason.to_string(),
            active_document,
            active_document_project_relative,
            documents,
            ui,
        };
        let backup = self.path.with_file_name("editor_session.json.bak");
        let result = ProjectStorage::write_json_atomic_with_backup(
            &self.path,
            &snapshot,
            BackupPolicy::new(backup, DEFAULT_BACKUP_GENERATIONS),
        )
        .map_err(io::Error::from);
        match result {
            Ok(_) => {
                self.last_checkpoint = Instant::now();
                self.last_error = None;
                Ok(SessionCheckpointReport {
                    path: self.path.clone(),
                    documents: snapshot.documents.len(),
                    dirty_buffers,
                    omitted_buffers,
                })
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    pub fn load_pending(&self) -> io::Result<Option<EditorSessionSnapshot>> {
        if !self.path.is_file() {
            return Ok(None);
        }
        let value = fs::read_to_string(&self.path)?;
        let snapshot = serde_json::from_str::<EditorSessionSnapshot>(&value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if snapshot.schema_version > SESSION_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "editor session schema {} is newer than supported schema {}",
                    snapshot.schema_version, SESSION_SCHEMA_VERSION
                ),
            ));
        }
        Ok(Some(snapshot))
    }

    pub fn restore_script_editor(
        &self,
        snapshot: &EditorSessionSnapshot,
        script_editor: &mut ScriptEditor,
    ) -> SessionRestoreReport {
        let mut documents = Vec::new();
        let mut report = SessionRestoreReport::default();
        for recovered in &snapshot.documents {
            let path = self.resolve_path(&recovered.path, recovered.project_relative);
            let document = if let Some(buffer) = &recovered.buffer {
                report.restored_dirty_buffers += 1;
                Some(EditorDocument::from_text(
                    path.clone(),
                    buffer.clone(),
                    true,
                ))
            } else if path.is_file() {
                EditorDocument::from_disk(&path).ok()
            } else {
                None
            };
            if recovered.buffer_omitted {
                report.omitted_buffers.push(path.clone());
            }
            if let Some(mut document) = document {
                document.language = recovered.language.clone();
                documents.push(document);
            } else {
                report.missing_documents.push(path);
            }
        }
        let active = snapshot
            .active_document
            .as_ref()
            .map(|path| self.resolve_path(path, snapshot.active_document_project_relative));
        report.restored_documents = script_editor.restore_documents(documents, active);
        report
    }

    pub fn clear(&mut self) -> io::Result<()> {
        let backup = self.path.with_file_name("editor_session.json.bak");
        let mut paths = vec![self.path.clone(), backup.clone()];
        paths.extend((1..DEFAULT_BACKUP_GENERATIONS).map(|generation| {
            backup.with_file_name(format!("editor_session.json.bak.{generation}"))
        }));
        for path in paths {
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        self.last_error = None;
        Ok(())
    }

    fn store_path(&self, path: &Path) -> (PathBuf, bool) {
        path.strip_prefix(&self.project_path)
            .map(|relative| (relative.to_path_buf(), true))
            .unwrap_or_else(|_| (path.to_path_buf(), false))
    }

    fn resolve_path(&self, path: &Path, project_relative: bool) -> PathBuf {
        if project_relative {
            self.project_path.join(path)
        } else {
            path.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use super::{SessionRecoveryManager, SessionUiState};
    use crate::engine::script_editor::ScriptEditor;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestProject(std::path::PathBuf);

    impl TestProject {
        fn new() -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "miniforge_session_recovery_{}_{}",
                std::process::id(),
                sequence
            ));
            fs::create_dir_all(path.join("scripts")).expect("test project");
            Self(path)
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn checkpoint_restores_dirty_buffers_and_clears_on_clean_shutdown() {
        let project = TestProject::new();
        let first = project.0.join("scripts/player.luau");
        let second = project.0.join("scripts/enemy.luau");
        fs::write(&first, "function on_start()\nend\n").expect("first script");
        fs::write(&second, "function on_update(dt)\nend\n").expect("second script");
        let mut editor = ScriptEditor::default();
        editor.open(first).expect("open first");
        editor.open(second.clone()).expect("open second");
        editor.set_text("function on_update(dt)\n    move(1, 0)\nend\n");

        let mut recovery = SessionRecoveryManager::new(&project.0, Duration::ZERO);
        let checkpoint = recovery
            .checkpoint(
                "main.scene",
                true,
                "Edit Script",
                &mut editor,
                SessionUiState {
                    show_console: true,
                    show_grid: true,
                    show_hierarchy: true,
                    show_inspector: true,
                    active_bottom_panel: "Programming".to_string(),
                    script_window_open: true,
                    sprite_window_open: false,
                    blueprint_picker_open: false,
                },
            )
            .expect("checkpoint");
        assert_eq!(checkpoint.documents, 2);
        assert_eq!(checkpoint.dirty_buffers, 1);
        assert!(recovery.is_pending());

        let snapshot = recovery
            .load_pending()
            .expect("load checkpoint")
            .expect("pending session");
        let mut restored_editor = ScriptEditor::default();
        let restored = recovery.restore_script_editor(&snapshot, &mut restored_editor);
        assert_eq!(restored.restored_documents, 2);
        assert_eq!(restored.restored_dirty_buffers, 1);
        assert_eq!(
            restored_editor.document.path.as_deref(),
            Some(second.as_path())
        );
        assert!(restored_editor.document.dirty);
        assert!(restored_editor.text().contains("move(1, 0)"));

        recovery.clear().expect("clean shutdown");
        assert!(!recovery.is_pending());
    }
}
