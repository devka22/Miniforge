#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    pub fps: f64,
    pub frame_time_ms: f64,
    pub average_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub uptime: f64,
    pub frames: u64,
}

impl Diagnostics {
    pub fn update(&mut self, dt: f64) {
        self.uptime += dt;
        self.frame_time_ms = dt * 1000.0;
        self.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        self.frames = self.frames.saturating_add(1);
        if self.frames == 1 {
            self.min_frame_time_ms = self.frame_time_ms;
            self.max_frame_time_ms = self.frame_time_ms;
            self.average_frame_time_ms = self.frame_time_ms;
            return;
        }
        self.min_frame_time_ms = self.min_frame_time_ms.min(self.frame_time_ms);
        self.max_frame_time_ms = self.max_frame_time_ms.max(self.frame_time_ms);
        let previous_weight = (self.frames - 1) as f64;
        self.average_frame_time_ms = (self.average_frame_time_ms * previous_weight
            + self.frame_time_ms)
            / self.frames as f64;
    }

    pub fn reset_frame_stats(&mut self) {
        self.average_frame_time_ms = 0.0;
        self.min_frame_time_ms = 0.0;
        self.max_frame_time_ms = 0.0;
        self.frames = 0;
    }
}
