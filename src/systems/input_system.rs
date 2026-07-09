use std::collections::{BTreeMap, VecDeque};

use crate::engine::input_map::InputMap;
use crate::input::input_handler::InputHandler;

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    Key { key: String, pressed: bool },
    PointerMoved { x: f64, y: f64 },
}

#[derive(Debug, Clone, Default)]
pub struct InputSystem {
    pub handler: InputHandler,
    pub pending: VecDeque<InputEvent>,
    pub stats: BTreeMap<String, usize>,
}

impl InputSystem {
    pub fn update(&mut self) {
        // Snapshot first; queued platform events then become observable as
        // just-pressed/released for exactly this frame.
        self.handler.update();
        let mut processed = 0;
        while let Some(event) = self.pending.pop_front() {
            match event {
                InputEvent::Key { key, pressed } => self.handler.set_pressed(&key, pressed),
                InputEvent::PointerMoved { x, y } => self.handler.state.set_mouse_position(x, y),
            }
            processed += 1;
        }
        self.stats.insert("events".to_string(), processed);
        self.stats
            .insert("pressed_keys".to_string(), self.handler.state.pressed.len());
    }

    pub fn queue_key(&mut self, key: impl Into<String>, pressed: bool) {
        self.pending.push_back(InputEvent::Key {
            key: key.into().to_lowercase(),
            pressed,
        });
    }

    pub fn queue_pointer(&mut self, x: f64, y: f64) {
        if x.is_finite() && y.is_finite() {
            self.pending.push_back(InputEvent::PointerMoved { x, y });
        }
    }

    pub fn action_pressed(&self, map: &InputMap, action: &str) -> bool {
        self.handler.action_pressed(map, action)
    }

    pub fn action_just_pressed(&self, map: &InputMap, action: &str) -> bool {
        self.handler.action_just_pressed(map, action)
    }

    pub fn action_axis_2d(&self, map: &InputMap, action: &str) -> (f64, f64) {
        self.handler.action_axis_2d(map, action)
    }
}
