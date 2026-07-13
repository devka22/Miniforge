//! Frontend-neutral editor panel intent state.
//!
//! Qt owns the actual dock widgets and layout persistence. The Rust backend
//! keeps only which tool surfaces should be opened, so engine operations never
//! depend on a Rust UI toolkit.

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
}

#[derive(Debug, Clone)]
pub struct EditorDockingWorkspace {
    pub open_tabs: Vec<EditorDockTab>,
    pub floating_panels: Vec<DockingPanel>,
    pub theme: DockingTheme,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockingTheme {
    pub accent: [u8; 4],
    pub panel: [u8; 4],
    pub panel_dark: [u8; 4],
    pub border: [u8; 4],
}

impl Default for EditorDockingWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorDockingWorkspace {
    pub fn new() -> Self {
        Self {
            open_tabs: vec![
                EditorDockTab::World,
                EditorDockTab::Hierarchy,
                EditorDockTab::Inspector,
                EditorDockTab::ContentBrowser,
                EditorDockTab::Console,
            ],
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
        if !self.open_tabs.contains(&tab) {
            self.open_tabs.push(tab);
        }
    }

    pub fn is_open(&self, tab: EditorDockTab) -> bool {
        self.open_tabs.contains(&tab)
    }

    pub fn dock_summary(&self) -> String {
        let tabs = self
            .open_tabs
            .iter()
            .map(|tab| tab.title())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "native:{} tabs [{}] | floating:{}",
            self.open_tabs.len(),
            tabs,
            self.floating_panels
                .iter()
                .filter(|panel| panel.floating)
                .count(),
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
}

impl Default for DockingTheme {
    fn default() -> Self {
        Self {
            accent: [76, 151, 255, 255],
            panel: [25, 29, 38, 255],
            panel_dark: [15, 18, 25, 245],
            border: [72, 86, 112, 255],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorDockTab, EditorDockingWorkspace};

    #[test]
    fn native_workspace_tracks_frontend_panel_intents() {
        let mut workspace = EditorDockingWorkspace::new();
        workspace.open_tab(EditorDockTab::ScriptEditor);
        workspace.open_tab(EditorDockTab::BlueprintEditor);
        workspace.set_floating_visibility("script_editor", true);

        assert!(workspace.is_open(EditorDockTab::ScriptEditor));
        assert!(workspace.floating_panel("script_editor").is_some());
        assert!(workspace.dock_summary().contains("native:"));
    }
}
