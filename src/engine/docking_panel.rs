use egui::{CornerRadius, Visuals};
use egui_dock::DockState;

use crate::engine::editor_ui::{EditorIcon, install_phosphor_fonts};

#[derive(Debug, Clone, PartialEq)]
pub struct DockingPanel {
    pub id: String,
    pub title: String,
    pub visible: bool,
    pub rect: (f64, f64, f64, f64),
    pub floating: bool,
}

impl DockingPanel {
    pub fn new(id: &str, title: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            visible: true,
            rect: (0.0, 0.0, 240.0, 240.0),
            floating: false,
        }
    }

    pub fn detached(mut self, rect: (f64, f64, f64, f64)) -> Self {
        self.rect = rect;
        self.floating = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorDockTab {
    World,
    Hierarchy,
    Inspector,
    ContentBrowser,
    ScriptEditor,
    BlueprintEditor,
    Prefabs,
    Console,
    Profiler,
    PlayWindow,
}

impl EditorDockTab {
    pub fn title(self) -> &'static str {
        match self {
            Self::World => "World",
            Self::Hierarchy => "Hierarchy",
            Self::Inspector => "Inspector",
            Self::ContentBrowser => "Content Browser",
            Self::ScriptEditor => "Script Editor",
            Self::BlueprintEditor => "Blueprints",
            Self::Prefabs => "Prefabs",
            Self::Console => "Console",
            Self::Profiler => "Profiler",
            Self::PlayWindow => "Play Window",
        }
    }

    pub fn icon(self) -> EditorIcon {
        match self {
            Self::World => EditorIcon::Scene,
            Self::Hierarchy => EditorIcon::Graph,
            Self::Inspector => EditorIcon::Component,
            Self::ContentBrowser => EditorIcon::Folder,
            Self::ScriptEditor => EditorIcon::Script,
            Self::BlueprintEditor => EditorIcon::Graph,
            Self::Prefabs => EditorIcon::Prefab,
            Self::Console => EditorIcon::Warning,
            Self::Profiler => EditorIcon::Validate,
            Self::PlayWindow => EditorIcon::Play,
        }
    }

    pub fn display_title(self) -> String {
        self.icon().label(self.title())
    }
}

#[derive(Debug, Clone)]
pub struct EguiDockingWorkspace {
    pub dock_state: DockState<EditorDockTab>,
    pub floating_panels: Vec<DockingPanel>,
    pub theme: DockingTheme,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockingTheme {
    pub accent: [u8; 4],
    pub panel: [u8; 4],
    pub panel_dark: [u8; 4],
    pub border: [u8; 4],
    pub egui_visuals: Visuals,
}

impl Default for EguiDockingWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl EguiDockingWorkspace {
    pub fn new() -> Self {
        let tabs = vec![
            EditorDockTab::World,
            EditorDockTab::Hierarchy,
            EditorDockTab::Inspector,
            EditorDockTab::ContentBrowser,
            EditorDockTab::Console,
        ];
        Self {
            dock_state: DockState::new(tabs),
            floating_panels: vec![
                DockingPanel::new("script_editor", "Script Editor")
                    .detached((86.0, 96.0, 860.0, 560.0)),
                DockingPanel::new("blueprint_editor", "Blueprints")
                    .detached((116.0, 126.0, 900.0, 600.0)),
                DockingPanel::new("play_window", "Play Window")
                    .detached((140.0, 112.0, 960.0, 540.0)),
            ],
            theme: DockingTheme::default(),
        }
    }

    pub fn open_tab(&mut self, tab: EditorDockTab) {
        if !self
            .dock_state
            .iter_all_tabs()
            .any(|(_, existing)| *existing == tab)
        {
            self.dock_state.push_to_focused_leaf(tab);
        }
    }

    pub fn dock_summary(&self) -> String {
        let tabs = self
            .dock_state
            .iter_all_tabs()
            .map(|(_, tab)| tab.title())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "egui_dock:{} tabs | floating:{} | extras:{}",
            self.dock_state.main_surface().num_tabs(),
            self.floating_panels
                .iter()
                .filter(|panel| panel.floating)
                .count(),
            egui_extras_marker()
        )
        .replace(
            "World, Hierarchy, Inspector, Content Browser, Console",
            &tabs,
        )
    }

    pub fn floating_panel(&self, id: &str) -> Option<&DockingPanel> {
        self.floating_panels.iter().find(|panel| panel.id == id)
    }

    pub fn set_floating_visibility(&mut self, id: &str, visible: bool) {
        if let Some(panel) = self.floating_panels.iter_mut().find(|panel| panel.id == id) {
            panel.visible = visible;
        }
    }

    pub fn apply_egui_visuals(&self, ctx: &egui::Context) {
        install_phosphor_fonts(ctx);
        ctx.set_visuals(self.theme.egui_visuals.clone());
    }
}

impl Default for DockingTheme {
    fn default() -> Self {
        let mut visuals = Visuals::dark();
        visuals.window_corner_radius = CornerRadius::same(4);
        visuals.widgets.active.corner_radius = 3.0.into();
        visuals.widgets.hovered.corner_radius = 3.0.into();
        Self {
            accent: [76, 151, 255, 255],
            panel: [25, 29, 38, 255],
            panel_dark: [15, 18, 25, 245],
            border: [72, 86, 112, 255],
            egui_visuals: visuals,
        }
    }
}

fn egui_extras_marker() -> &'static str {
    std::any::type_name::<egui_extras::Column>()
}
