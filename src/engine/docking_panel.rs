#[derive(Debug, Clone)]
pub struct DockingPanel {
    pub id: String,
    pub title: String,
    pub visible: bool,
    pub rect: (f64, f64, f64, f64),
}

impl DockingPanel {
    pub fn new(id: &str, title: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            visible: true,
            rect: (0.0, 0.0, 240.0, 240.0),
        }
    }
}
