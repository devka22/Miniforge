#[derive(Debug, Clone, Default)]
pub struct MenuBar {
    pub open_menu: Option<String>,
    pub menus: Vec<String>,
}

impl MenuBar {
    pub fn open(&mut self, name: &str) {
        self.open_menu = Some(name.to_string());
        if !self.menus.iter().any(|menu| menu == name) {
            self.menus.push(name.to_string());
        }
    }

    pub fn close(&mut self) {
        self.open_menu = None;
    }
}
