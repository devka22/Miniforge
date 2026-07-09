use crate::engine::editor_ui::fuzzy_rank;

#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub commands: Vec<String>,
    pub selected_index: usize,
    pub recent_commands: Vec<String>,
    pub max_recent: usize,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            commands: Vec::new(),
            selected_index: 0,
            recent_commands: Vec::new(),
            max_recent: 8,
        }
    }
}

impl CommandPalette {
    pub fn with_commands(commands: impl IntoIterator<Item = String>) -> Self {
        Self {
            commands: commands.into_iter().collect(),
            ..Default::default()
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn toggle(&mut self) -> bool {
        if self.open {
            self.close();
        } else {
            self.open();
        }
        self.open
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.selected_index = 0;
    }

    pub fn push(&mut self, character: char) {
        if !character.is_control() {
            self.query.push(character);
            self.selected_index = 0;
        }
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.selected_index = 0;
    }

    pub fn search_indices(&self) -> Vec<usize> {
        fuzzy_rank(&self.query, &self.commands, self.commands.len())
            .into_iter()
            .map(|result| result.index)
            .collect()
    }

    pub fn search(&self) -> Vec<String> {
        self.search_indices()
            .into_iter()
            .map(|index| self.commands[index].clone())
            .collect()
    }

    pub fn move_selection(&mut self, delta: isize) -> Option<usize> {
        let count = self.search_indices().len();
        if count == 0 {
            self.selected_index = 0;
            return None;
        }
        self.selected_index =
            (self.selected_index as isize + delta).rem_euclid(count as isize) as usize;
        Some(self.selected_index)
    }

    pub fn selected_command(&self) -> Option<&str> {
        let command_index = *self.search_indices().get(self.selected_index)?;
        self.commands.get(command_index).map(String::as_str)
    }

    pub fn record_execution(&mut self, command: &str) {
        self.recent_commands.retain(|recent| recent != command);
        self.recent_commands.insert(0, command.to_string());
        self.recent_commands.truncate(self.max_recent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_uses_fuzzy_ranking_instead_of_plain_substrings() {
        let palette = CommandPalette {
            query: "svsc".to_string(),
            commands: vec![
                "Open Project".to_string(),
                "Save Scene".to_string(),
                "Create Script".to_string(),
            ],
            ..Default::default()
        };
        assert_eq!(
            palette.search().first().map(String::as_str),
            Some("Save Scene")
        );
    }

    #[test]
    fn keyboard_selection_wraps_and_execution_history_is_deduplicated() {
        let mut palette = CommandPalette::with_commands(
            ["Save Scene", "Save Project", "Open Project"]
                .into_iter()
                .map(str::to_string),
        );
        palette.open();
        palette.set_query("save");
        assert_eq!(palette.selected_command(), Some("Save Scene"));
        palette.move_selection(-1);
        assert_eq!(palette.selected_command(), Some("Save Project"));
        palette.record_execution("save_project");
        palette.record_execution("save_project");
        assert_eq!(palette.recent_commands, ["save_project"]);
    }
}
