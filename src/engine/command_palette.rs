#[derive(Debug, Clone, Default)]
pub struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub commands: Vec<String>,
}

impl CommandPalette {
    pub fn toggle(&mut self) -> bool {
        self.open = !self.open;
        self.open
    }

    pub fn search(&self) -> Vec<String> {
        let query = self.query.to_lowercase();
        self.commands
            .iter()
            .filter(|command| command.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }
}
