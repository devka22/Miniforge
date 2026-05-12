#[derive(Debug, Clone, Default)]
pub struct CreateAssetModal {
    pub open: bool,
    pub mode: String,
    pub title: String,
    pub placeholder: String,
    pub value: String,
}

impl CreateAssetModal {
    pub fn open(&mut self, mode: &str, title: &str, placeholder: &str) {
        self.open = true;
        self.mode = mode.to_string();
        self.title = title.to_string();
        self.placeholder = placeholder.to_string();
        self.value.clear();
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}
