use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolbarAction2D {
    SaveScene,
    SaveAll,
    Undo,
    Redo,
    Play,
    Pause,
    Stop,
    Build,
    Export,
    ProjectSettings,
    OpenContentBrowser,
    OpenConsole,
    OpenBlueprintLibrary,
    ValidateProject,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EditorRunState2D {
    Editing,
    Playing,
    Paused,
    Building,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolbarAlert2D {
    ExportError,
    ScriptError,
    GraphError,
    AssetsMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolbarButton2D {
    pub action: ToolbarAction2D,
    pub label: String,
    pub tooltip: String,
    pub enabled: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolbarStatus2D {
    pub state: EditorRunState2D,
    pub fps: f32,
    pub current_scene: String,
    pub errors: usize,
    pub warnings: usize,
    pub alerts: Vec<ToolbarAlert2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Toolbar2D {
    pub buttons: Vec<ToolbarButton2D>,
    pub status: ToolbarStatus2D,
}

impl Default for ToolbarStatus2D {
    fn default() -> Self {
        Self {
            state: EditorRunState2D::Editing,
            fps: 60.0,
            current_scene: "main.scene".to_string(),
            errors: 0,
            warnings: 0,
            alerts: Vec::new(),
        }
    }
}

impl Toolbar2D {
    pub fn new(status: ToolbarStatus2D) -> Self {
        Self {
            buttons: all_actions()
                .into_iter()
                .map(|action| button_for(action, status.state))
                .collect(),
            status,
        }
    }

    pub fn available_actions(&self) -> Vec<ToolbarAction2D> {
        self.buttons
            .iter()
            .filter(|button| button.enabled)
            .map(|button| button.action)
            .collect()
    }

    pub fn refresh_alerts(
        &mut self,
        script_errors: usize,
        graph_errors: usize,
        missing_assets: usize,
    ) {
        self.status.alerts.clear();
        if script_errors > 0 {
            self.status.alerts.push(ToolbarAlert2D::ScriptError);
        }
        if graph_errors > 0 {
            self.status.alerts.push(ToolbarAlert2D::GraphError);
        }
        if missing_assets > 0 {
            self.status.alerts.push(ToolbarAlert2D::AssetsMissing);
        }
    }
}

pub fn all_actions() -> Vec<ToolbarAction2D> {
    vec![
        ToolbarAction2D::SaveScene,
        ToolbarAction2D::SaveAll,
        ToolbarAction2D::Undo,
        ToolbarAction2D::Redo,
        ToolbarAction2D::Play,
        ToolbarAction2D::Pause,
        ToolbarAction2D::Stop,
        ToolbarAction2D::Build,
        ToolbarAction2D::Export,
        ToolbarAction2D::ProjectSettings,
        ToolbarAction2D::OpenContentBrowser,
        ToolbarAction2D::OpenConsole,
        ToolbarAction2D::OpenBlueprintLibrary,
        ToolbarAction2D::ValidateProject,
    ]
}

fn button_for(action: ToolbarAction2D, state: EditorRunState2D) -> ToolbarButton2D {
    let label = format!("{action:?}");
    let enabled = match action {
        ToolbarAction2D::Play => !matches!(
            state,
            EditorRunState2D::Playing | EditorRunState2D::Building
        ),
        ToolbarAction2D::Pause => matches!(state, EditorRunState2D::Playing),
        ToolbarAction2D::Stop => {
            matches!(state, EditorRunState2D::Playing | EditorRunState2D::Paused)
        }
        ToolbarAction2D::Build | ToolbarAction2D::Export => {
            !matches!(state, EditorRunState2D::Playing)
        }
        _ => true,
    };
    ToolbarButton2D {
        action,
        label: label.clone(),
        tooltip: label,
        enabled,
        active: matches!(
            (action, state),
            (ToolbarAction2D::Play, EditorRunState2D::Playing)
                | (ToolbarAction2D::Pause, EditorRunState2D::Paused)
        ),
    }
}
