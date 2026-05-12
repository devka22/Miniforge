use crate::engine::input_map::InputMap;

#[derive(Debug, Clone, Default)]
pub struct VisualInputEditor {
    pub visible: bool,
    pub selected_action: Option<String>,
}

impl VisualInputEditor {
    pub fn toggle(&mut self) -> bool {
        self.visible = !self.visible;
        self.visible
    }

    pub fn select(&mut self, action: &str) {
        self.selected_action = Some(action.to_string());
    }

    pub fn add_binding(&self, input_map: &mut InputMap, key: &str) -> std::io::Result<()> {
        let Some(action) = &self.selected_action else {
            return Ok(());
        };
        let mut keys = input_map.bindings.get(action).cloned().unwrap_or_default();
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.to_string());
        }
        input_map.set_binding(action, keys)
    }
}
