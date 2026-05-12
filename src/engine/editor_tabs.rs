#[derive(Debug, Clone)]
pub struct EditorTabs {
    pub active: String,
    pub tabs: Vec<String>,
}

impl Default for EditorTabs {
    fn default() -> Self {
        Self {
            active: "Scene".to_string(),
            tabs: vec![
                "Scene".to_string(),
                "Script".to_string(),
                "Settings".to_string(),
            ],
        }
    }
}

impl EditorTabs {
    pub fn set_active(&mut self, tab: &str) {
        if !self.tabs.iter().any(|existing| existing == tab) {
            self.tabs.push(tab.to_string());
        }
        self.active = tab.to_string();
    }
}
