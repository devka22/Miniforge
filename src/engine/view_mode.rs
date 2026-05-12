#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewModeKind {
    SceneView,
    GameView,
}

#[derive(Debug, Clone)]
pub struct ViewMode {
    pub mode: ViewModeKind,
}

impl Default for ViewMode {
    fn default() -> Self {
        Self {
            mode: ViewModeKind::SceneView,
        }
    }
}

impl ViewMode {
    pub const SCENE_VIEW: ViewModeKind = ViewModeKind::SceneView;
    pub const GAME_VIEW: ViewModeKind = ViewModeKind::GameView;

    pub fn toggle(&mut self) {
        self.mode = match self.mode {
            ViewModeKind::SceneView => ViewModeKind::GameView,
            ViewModeKind::GameView => ViewModeKind::SceneView,
        };
    }
}
