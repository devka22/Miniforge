#[derive(Debug, Clone, Default)]
pub struct DeveloperConsole {
    pub entries: Vec<(String, String)>,
}

impl DeveloperConsole {
    pub fn log(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        self.entries.push((channel.into(), message.into()));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
