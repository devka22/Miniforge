use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EditorCommandSource {
    Shell,
    Panel,
    Document,
    Runtime,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EditorShellCommand {
    OpenDocument(PathBuf),
    SaveDocument(PathBuf),
    CloseDocument(PathBuf),
    ShowPanel(String),
    HidePanel(String),
    ReportProblem(String),
    RequestQuit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueuedEditorCommand {
    pub source: EditorCommandSource,
    pub command: EditorShellCommand,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorCommandBus {
    pub queue: VecDeque<QueuedEditorCommand>,
    pub rejected: Vec<QueuedEditorCommand>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelState {
    pub visible: BTreeMap<String, bool>,
    pub last_focused: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentState {
    pub active_document: Option<PathBuf>,
    pub dirty_documents: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeState {
    pub running: bool,
    pub safe_mode: bool,
    pub disabled_systems: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayModeState {
    pub in_play_mode: bool,
    pub snapshot_available: bool,
    pub last_exit_reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectState {
    pub project_path: Option<PathBuf>,
    pub safe_mode_requested: bool,
    pub validation_errors: usize,
    pub validation_warnings: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorShellState {
    pub panels: PanelState,
    pub documents: DocumentState,
    pub runtime: RuntimeState,
    pub play_mode: PlayModeState,
    pub project: ProjectState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorShell {
    pub state: EditorShellState,
    pub command_bus: EditorCommandBus,
}

impl EditorCommandBus {
    pub fn emit(&mut self, source: EditorCommandSource, command: EditorShellCommand) -> bool {
        let queued = QueuedEditorCommand { source, command };
        if matches!(queued.command, EditorShellCommand::RequestQuit)
            && queued.source != EditorCommandSource::Shell
        {
            self.rejected.push(queued);
            return false;
        }
        self.queue.push_back(queued);
        true
    }

    pub fn emit_panel(&mut self, command: EditorShellCommand) -> bool {
        self.emit(EditorCommandSource::Panel, command)
    }

    pub fn drain(&mut self) -> Vec<QueuedEditorCommand> {
        self.queue.drain(..).collect()
    }
}

impl EditorShell {
    pub fn request_close_document_from_panel(&mut self, path: impl Into<PathBuf>) {
        self.command_bus
            .emit_panel(EditorShellCommand::CloseDocument(path.into()));
    }

    pub fn apply_command(&mut self, queued: QueuedEditorCommand) {
        match queued.command {
            EditorShellCommand::OpenDocument(path) => {
                self.state.documents.active_document = Some(path);
            }
            EditorShellCommand::SaveDocument(path) => {
                self.state
                    .documents
                    .dirty_documents
                    .retain(|dirty| dirty != &path);
            }
            EditorShellCommand::CloseDocument(path) => {
                self.state
                    .documents
                    .dirty_documents
                    .retain(|dirty| dirty != &path);
                if self.state.documents.active_document.as_ref() == Some(&path) {
                    self.state.documents.active_document = None;
                }
            }
            EditorShellCommand::ShowPanel(id) => {
                self.state.panels.visible.insert(id.clone(), true);
                self.state.panels.last_focused = Some(id);
            }
            EditorShellCommand::HidePanel(id) => {
                self.state.panels.visible.insert(id, false);
            }
            EditorShellCommand::ReportProblem(_) | EditorShellCommand::RequestQuit => {}
        }
    }
}
