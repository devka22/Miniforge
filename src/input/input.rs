use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub pressed: BTreeSet<String>,
    pub previous_pressed: BTreeSet<String>,
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

    pub fn begin_frame(&mut self) {
        self.previous_pressed = self.pressed.clone();
    }

    pub fn just_pressed(&self, key: &str) -> bool {
        self.pressed.contains(key) && !self.previous_pressed.contains(key)
    }

    pub fn just_released(&self, key: &str) -> bool {
        !self.pressed.contains(key) && self.previous_pressed.contains(key)
    }

    pub fn set_mouse_position(&mut self, x: f64, y: f64) {
        self.mouse_position = (x, y);
    }

    pub fn axis_2d(
        &self,
        negative_x: &str,
        positive_x: &str,
        negative_y: &str,
        positive_y: &str,
    ) -> (f64, f64) {
        let x = axis_value(self.is_pressed(negative_x), self.is_pressed(positive_x));
        let y = axis_value(self.is_pressed(negative_y), self.is_pressed(positive_y));
        (x, y)
    }
}

pub type Unit = InputState;

fn axis_value(negative: bool, positive: bool) -> f64 {
    match (negative, positive) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
}
