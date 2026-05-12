#[derive(Debug, Clone)]
pub struct Toolbar {
    pub active_tool: String,
    pub tools: Vec<String>,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self {
            active_tool: "Select".to_string(),
            tools: vec![
                "Select".to_string(),
                "Move".to_string(),
                "Rotate".to_string(),
                "Scale".to_string(),
                "Paint".to_string(),
            ],
        }
    }
}

impl Toolbar {
    pub fn set_tool(&mut self, tool: &str) {
        if self.tools.iter().any(|existing| existing == tool) {
            self.active_tool = tool.to_string();
        }
    }
}
