#[derive(Debug, Clone)]
pub struct ColorAnimation {
    pub colors: Vec<(u8, u8, u8)>,
    pub speed: f64,
    pub time: f64,
}

impl ColorAnimation {
    pub fn new(colors: Vec<(u8, u8, u8)>, speed: f64) -> Self {
        Self {
            colors,
            speed,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64) {
        self.time += dt;
    }

    pub fn color(&self) -> (u8, u8, u8) {
        if self.colors.is_empty() {
            return (255, 255, 255);
        }
        let index = ((self.time / self.speed.max(0.0001)) as usize) % self.colors.len();
        self.colors[index]
    }
}

#[derive(Debug, Clone)]
pub struct AnimationController {
    pub current: String,
    pub animations: std::collections::BTreeMap<String, ColorAnimation>,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self {
            current: "IDLE".to_string(),
            animations: Default::default(),
        }
    }
}

impl AnimationController {
    pub fn add(&mut self, name: &str, animation: ColorAnimation) {
        self.animations.insert(name.to_string(), animation);
    }

    pub fn play(&mut self, name: &str) {
        self.current = name.to_string();
    }

    pub fn update(&mut self, dt: f64) {
        if let Some(animation) = self.animations.get_mut(&self.current) {
            animation.update(dt);
        }
    }

    pub fn get_color(&self) -> (u8, u8, u8) {
        self.animations
            .get(&self.current)
            .map(ColorAnimation::color)
            .unwrap_or((0, 120, 255))
    }
}
