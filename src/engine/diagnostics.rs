#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    pub fps: f64,
    pub frame_time_ms: f64,
    pub average_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub uptime: f64,
    pub frames: u64,
    pub dropped_frames: u64,
    pub warnings: Vec<String>,
}

impl Diagnostics {
    pub fn update(&mut self, dt: f64) {
        self.uptime += dt;
        self.frame_time_ms = dt * 1000.0;
        self.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        self.frames = self.frames.saturating_add(1);
        if self.frame_time_ms > 33.34 {
            self.dropped_frames = self.dropped_frames.saturating_add(1);
        }
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
        self.dropped_frames = 0;
        self.warnings.clear();
    }

    pub fn cleanup(&mut self) {
        self.warnings.sort();
        self.warnings.dedup();
        self.warnings.truncate(32);
    }

    pub fn push_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
        self.cleanup();
    }

    pub fn stability_score(&self) -> f64 {
        if self.frames == 0 {
            return 1.0;
        }
        let drop_ratio = self.dropped_frames as f64 / self.frames as f64;
        (1.0 - drop_ratio).clamp(0.0, 1.0)
    }
}
