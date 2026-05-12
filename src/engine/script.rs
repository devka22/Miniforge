#[derive(Debug, Clone)]
pub struct Script {
    pub script_name: String,
    pub enabled: bool,
    pub started: bool,
}

impl Script {
    pub fn new(script_name: impl Into<String>) -> Self {
        Self {
            script_name: script_name.into(),
            enabled: true,
            started: false,
        }
    }
}
