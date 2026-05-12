#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub viewport: (f64, f64, f64, f64),
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
            viewport: (0.0, 0.0, 1100.0, 740.0),
        }
    }
}

impl Camera {
    pub fn set_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.min_x = min_x;
        self.min_y = min_y;
        self.max_x = max_x;
        self.max_y = max_y;
        self.clamp_to_bounds();
    }

    pub fn set_viewport(&mut self, viewport: (f64, f64, f64, f64)) {
        self.viewport = viewport;
    }

    pub fn move_by(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
        self.clamp_to_bounds();
    }

    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(0.1, 8.0);
    }

    pub fn clamp_to_bounds(&mut self) {
        if self.max_x > self.min_x {
            self.x = self.x.clamp(self.min_x, self.max_x);
        }
        if self.max_y > self.min_y {
            self.y = self.y.clamp(self.min_y, self.max_y);
        }
    }
}
