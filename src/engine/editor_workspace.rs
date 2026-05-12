use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DockRegion {
    Top,
    Left,
    Center,
    Right,
    Bottom,
    Floating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EditorPanelKind {
    Scene,
    Game,
    Hierarchy,
    Inspector,
    AssetBrowser,
    Console,
    Profiler,
    Programming,
    Prefabs,
    Animator,
    Build,
    AssetGraph,
    Diagnostics,
}

impl EditorPanelKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Scene => "Scene",
            Self::Game => "Game",
            Self::Hierarchy => "Hierarchy",
            Self::Inspector => "Inspector",
            Self::AssetBrowser => "Content",
            Self::Console => "Console",
            Self::Profiler => "Profiler",
            Self::Programming => "Programming",
            Self::Prefabs => "Prefabs",
            Self::Animator => "Animator",
            Self::Build => "Build",
            Self::AssetGraph => "Asset Graph",
            Self::Diagnostics => "Diagnostics",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceMode {
    WorldBuilding,
    Scripting,
    PrefabEditing,
    Profiling,
    Shipping,
}

impl WorkspaceMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::WorldBuilding => "World",
            Self::Scripting => "Script",
            Self::PrefabEditing => "Prefab",
            Self::Profiling => "Profile",
            Self::Shipping => "Ship",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPanel {
    pub kind: EditorPanelKind,
    pub title: String,
    pub region: DockRegion,
    pub order: usize,
    pub visible: bool,
    pub pinned: bool,
    pub utility_score: f32,
}

impl EditorPanel {
    pub fn new(
        kind: EditorPanelKind,
        region: DockRegion,
        order: usize,
        utility_score: f32,
    ) -> Self {
        Self {
            kind,
            title: kind.title().to_string(),
            region,
            order,
            visible: true,
            pinned: false,
            utility_score,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorWorkspace {
    pub active_mode: WorkspaceMode,
    pub panels: Vec<EditorPanel>,
    pub focused_panel: EditorPanelKind,
    pub command_history: Vec<String>,
    pub frame_budget_ms: f64,
    pub browser_density: f32,
    pub live_recompile: bool,
}

impl Default for EditorWorkspace {
    fn default() -> Self {
        let mut workspace = Self {
            active_mode: WorkspaceMode::WorldBuilding,
            panels: vec![
                EditorPanel::new(EditorPanelKind::Scene, DockRegion::Center, 0, 1.0),
                EditorPanel::new(EditorPanelKind::Game, DockRegion::Center, 1, 0.82),
                EditorPanel::new(EditorPanelKind::Hierarchy, DockRegion::Left, 0, 0.95),
                EditorPanel::new(EditorPanelKind::Inspector, DockRegion::Right, 0, 0.95),
                EditorPanel::new(EditorPanelKind::AssetBrowser, DockRegion::Bottom, 0, 0.92),
                EditorPanel::new(EditorPanelKind::Console, DockRegion::Bottom, 1, 0.82),
                EditorPanel::new(EditorPanelKind::Profiler, DockRegion::Bottom, 2, 0.72),
                EditorPanel::new(EditorPanelKind::Programming, DockRegion::Bottom, 3, 0.84),
                EditorPanel::new(EditorPanelKind::Prefabs, DockRegion::Right, 1, 0.88),
                EditorPanel::new(EditorPanelKind::Animator, DockRegion::Bottom, 4, 0.64),
                EditorPanel::new(EditorPanelKind::Build, DockRegion::Right, 2, 0.62),
                EditorPanel::new(EditorPanelKind::AssetGraph, DockRegion::Bottom, 5, 0.66),
                EditorPanel::new(EditorPanelKind::Diagnostics, DockRegion::Floating, 0, 0.58),
            ],
            focused_panel: EditorPanelKind::Scene,
            command_history: Vec::new(),
            frame_budget_ms: 16.67,
            browser_density: 0.72,
            live_recompile: true,
        };
        workspace.apply_mode(WorkspaceMode::WorldBuilding);
        workspace
    }
}

impl EditorWorkspace {
    pub fn apply_mode(&mut self, mode: WorkspaceMode) {
        self.active_mode = mode;
        for panel in &mut self.panels {
            panel.visible = match mode {
                WorkspaceMode::WorldBuilding => matches!(
                    panel.kind,
                    EditorPanelKind::Scene
                        | EditorPanelKind::Hierarchy
                        | EditorPanelKind::Inspector
                        | EditorPanelKind::AssetBrowser
                        | EditorPanelKind::Console
                        | EditorPanelKind::Prefabs
                ),
                WorkspaceMode::Scripting => matches!(
                    panel.kind,
                    EditorPanelKind::Scene
                        | EditorPanelKind::Hierarchy
                        | EditorPanelKind::Inspector
                        | EditorPanelKind::AssetBrowser
                        | EditorPanelKind::Console
                        | EditorPanelKind::Programming
                        | EditorPanelKind::AssetGraph
                ),
                WorkspaceMode::PrefabEditing => matches!(
                    panel.kind,
                    EditorPanelKind::Scene
                        | EditorPanelKind::Hierarchy
                        | EditorPanelKind::Inspector
                        | EditorPanelKind::AssetBrowser
                        | EditorPanelKind::Prefabs
                        | EditorPanelKind::Console
                ),
                WorkspaceMode::Profiling => matches!(
                    panel.kind,
                    EditorPanelKind::Scene
                        | EditorPanelKind::Game
                        | EditorPanelKind::Profiler
                        | EditorPanelKind::Diagnostics
                        | EditorPanelKind::Console
                ),
                WorkspaceMode::Shipping => matches!(
                    panel.kind,
                    EditorPanelKind::Game
                        | EditorPanelKind::Build
                        | EditorPanelKind::AssetBrowser
                        | EditorPanelKind::Console
                        | EditorPanelKind::Profiler
                ),
            } || panel.pinned;
        }
        self.focused_panel = match mode {
            WorkspaceMode::WorldBuilding | WorkspaceMode::PrefabEditing => EditorPanelKind::Scene,
            WorkspaceMode::Scripting => EditorPanelKind::Programming,
            WorkspaceMode::Profiling => EditorPanelKind::Profiler,
            WorkspaceMode::Shipping => EditorPanelKind::Build,
        };
    }

    pub fn cycle_mode(&mut self) -> WorkspaceMode {
        let next = match self.active_mode {
            WorkspaceMode::WorldBuilding => WorkspaceMode::Scripting,
            WorkspaceMode::Scripting => WorkspaceMode::PrefabEditing,
            WorkspaceMode::PrefabEditing => WorkspaceMode::Profiling,
            WorkspaceMode::Profiling => WorkspaceMode::Shipping,
            WorkspaceMode::Shipping => WorkspaceMode::WorldBuilding,
        };
        self.apply_mode(next);
        next
    }

    pub fn toggle_panel(&mut self, kind: EditorPanelKind) -> bool {
        if let Some(panel) = self.panels.iter_mut().find(|panel| panel.kind == kind) {
            panel.visible = !panel.visible;
            self.focused_panel = kind;
            return panel.visible;
        }
        false
    }

    pub fn pin_panel(&mut self, kind: EditorPanelKind, pinned: bool) {
        if let Some(panel) = self.panels.iter_mut().find(|panel| panel.kind == kind) {
            panel.pinned = pinned;
            if pinned {
                panel.visible = true;
            }
        }
    }

    pub fn visible_panels(&self) -> Vec<&EditorPanel> {
        let mut panels = self
            .panels
            .iter()
            .filter(|panel| panel.visible)
            .collect::<Vec<_>>();
        panels.sort_by_key(|panel| (panel.region, panel.order));
        panels
    }

    pub fn visible_in_region(&self, region: DockRegion) -> Vec<&EditorPanel> {
        let mut panels = self
            .panels
            .iter()
            .filter(|panel| panel.visible && panel.region == region)
            .collect::<Vec<_>>();
        panels.sort_by_key(|panel| panel.order);
        panels
    }

    pub fn record_command(&mut self, command: impl Into<String>) {
        self.command_history.push(command.into());
        if self.command_history.len() > 32 {
            self.command_history.remove(0);
        }
    }

    pub fn performance_status(&self, frame_time_ms: f64) -> &'static str {
        if frame_time_ms <= self.frame_budget_ms {
            "Realtime"
        } else if frame_time_ms <= self.frame_budget_ms * 1.5 {
            "Pressure"
        } else {
            "Critical"
        }
    }

    pub fn workflow_summary(&self) -> String {
        let visible = self.visible_panels().len();
        format!(
            "{} workspace | {} panels | live graphs {} | budget {:.2} ms",
            self.active_mode.label(),
            visible,
            if self.live_recompile { "on" } else { "off" },
            self.frame_budget_ms
        )
    }
}
