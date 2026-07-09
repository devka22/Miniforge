use crate::engine::input_map::InputMap;
use crate::input::input::InputState;

#[derive(Debug, Clone, Default)]
pub struct InputHandler {
    pub state: InputState,
}

impl InputHandler {
    pub fn update(&mut self) {
        self.state.begin_frame();
    }

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
            .map(|keys| keys.iter().any(|key| binding_pressed(&self.state, key)))
            .unwrap_or(false)
    }

    pub fn action_just_pressed(&self, input_map: &InputMap, action: &str) -> bool {
        input_map
            .bindings
            .get(action)
            .map(|keys| {
                keys.iter()
                    .any(|key| binding_just_pressed(&self.state, key))
            })
            .unwrap_or(false)
    }

    pub fn action_just_released(&self, input_map: &InputMap, action: &str) -> bool {
        input_map
            .bindings
            .get(action)
            .map(|keys| {
                keys.iter()
                    .any(|key| binding_just_released(&self.state, key))
            })
            .unwrap_or(false)
    }

    pub fn action_axis_2d(&self, input_map: &InputMap, action: &str) -> (f64, f64) {
        let Some(bindings) = input_map.bindings.get(action) else {
            return (0.0, 0.0);
        };
        if bindings.iter().any(|binding| binding == "keyboard:wasd") {
            return self.state.axis_2d("a", "d", "w", "s");
        }
        if bindings.iter().any(|binding| binding == "keyboard:arrows") {
            return self.state.axis_2d("left", "right", "up", "down");
        }
        (0.0, 0.0)
    }
}

fn binding_pressed(state: &crate::input::input::InputState, binding: &str) -> bool {
    match binding {
        "keyboard:wasd" => ["w", "a", "s", "d"].iter().any(|key| state.is_pressed(key)),
        "keyboard:arrows" => ["up", "down", "left", "right"]
            .iter()
            .any(|key| state.is_pressed(key)),
        other => state.is_pressed(other),
    }
}

fn binding_just_pressed(state: &crate::input::input::InputState, binding: &str) -> bool {
    match binding {
        "keyboard:wasd" => ["w", "a", "s", "d"]
            .iter()
            .any(|key| state.just_pressed(key)),
        "keyboard:arrows" => ["up", "down", "left", "right"]
            .iter()
            .any(|key| state.just_pressed(key)),
        other => state.just_pressed(other),
    }
}

fn binding_just_released(state: &crate::input::input::InputState, binding: &str) -> bool {
    match binding {
        "keyboard:wasd" => ["w", "a", "s", "d"]
            .iter()
            .any(|key| state.just_released(key)),
        "keyboard:arrows" => ["up", "down", "left", "right"]
            .iter()
            .any(|key| state.just_released(key)),
        other => state.just_released(other),
    }
}
