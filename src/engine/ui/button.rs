#[derive(Debug, Clone)]
pub struct Button {
    pub label: String,
    pub rect: (f64, f64, f64, f64),
    pub enabled: bool,
}

impl Button {
    pub fn new(label: &str, rect: (f64, f64, f64, f64)) -> Self {
        Self {
            label: label.to_string(),
            rect,
            enabled: true,
        }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        let (rx, ry, rw, rh) = self.rect;
        self.enabled && x >= rx && y >= ry && x <= rx + rw && y <= ry + rh
    }
}
