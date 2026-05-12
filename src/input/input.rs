use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub pressed: BTreeSet<String>,
    pub mouse_position: (f64, f64),
}

impl InputState {
    pub fn press(&mut self, key: &str) {
        self.pressed.insert(key.to_string());
    }

    pub fn release(&mut self, key: &str) {
        self.pressed.remove(key);
    }

    pub fn is_pressed(&self, key: &str) -> bool {
        self.pressed.contains(key)
    }
}

pub type Unit = InputState;
