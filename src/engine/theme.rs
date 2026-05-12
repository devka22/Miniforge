#[derive(Debug, Clone)]
pub struct Theme {
    pub background: (u8, u8, u8),
    pub panel: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub text: (u8, u8, u8),
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: (22, 24, 30),
            panel: (34, 37, 46),
            accent: (75, 145, 255),
            text: (240, 242, 248),
        }
    }
}
