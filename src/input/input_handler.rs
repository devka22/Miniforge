use crate::engine::input_map::InputMap;
use crate::input::input::InputState;

#[derive(Debug, Clone, Default)]
pub struct InputHandler {
    pub state: InputState,
}

impl InputHandler {
    pub fn update(&mut self) {}

    pub fn set_pressed(&mut self, key: &str, pressed: bool) {
        if pressed {
            self.state.press(key);
        } else {
            self.state.release(key);
        }
    }

    pub fn action_pressed(&self, input_map: &InputMap, action: &str) -> bool {
        input_map
            .bindings
            .get(action)
            .map(|keys| keys.iter().any(|key| self.state.is_pressed(key)))
            .unwrap_or(false)
    }
}
