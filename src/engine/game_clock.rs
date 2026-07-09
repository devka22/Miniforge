use crate::engine::runtime_config::RuntimeTuning;

#[derive(Debug, Clone)]
pub struct GameClock {
    pub fixed_delta: f64,
    pub max_steps_per_frame: usize,
    pub target_frame_delta: f64,
    pub time_scale: f64,
    pub accumulator: f64,
    pub total_time: f64,
    pub frame: u64,
    pub tick: u64,
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockAdvance {
    pub scaled_dt: f64,
    pub fixed_steps: usize,
    pub interpolation_alpha: f64,
    pub dropped_time: f64,
    pub target_frame_delta: f64,
    pub over_budget: bool,
    pub saturated_fixed_steps: bool,
}

impl Default for GameClock {
    fn default() -> Self {
        Self::new(1.0 / 60.0)
    }
}

impl GameClock {
    pub fn new(fixed_delta: f64) -> Self {
        Self {
            fixed_delta: fixed_delta.max(0.0001),
            max_steps_per_frame: 5,
            target_frame_delta: 1.0 / 60.0,
            time_scale: 1.0,
            accumulator: 0.0,
            total_time: 0.0,
            frame: 0,
            tick: 0,
            paused: false,
        }
    }

    pub fn advance(&mut self, real_dt: f64) -> ClockAdvance {
        self.frame = self.frame.saturating_add(1);
        if self.paused {
            return ClockAdvance {
                scaled_dt: 0.0,
                fixed_steps: 0,
                interpolation_alpha: 0.0,
                dropped_time: 0.0,
                target_frame_delta: self.target_frame_delta,
                over_budget: false,
                saturated_fixed_steps: false,
            };
        }

        let scaled_dt = (real_dt.max(0.0) * self.time_scale.max(0.0)).min(0.25);
        self.total_time += scaled_dt;
        self.accumulator += scaled_dt;

        let mut fixed_steps = 0;
        while self.accumulator + f64::EPSILON >= self.fixed_delta
            && fixed_steps < self.max_steps_per_frame
        {
            self.accumulator -= self.fixed_delta;
            self.tick = self.tick.saturating_add(1);
            fixed_steps += 1;
        }

        let mut dropped_time = 0.0;
        if self.accumulator >= self.fixed_delta {
            dropped_time = self.accumulator;
            self.accumulator = 0.0;
        }

        ClockAdvance {
            scaled_dt,
            fixed_steps,
            interpolation_alpha: (self.accumulator / self.fixed_delta).clamp(0.0, 1.0),
            dropped_time,
            target_frame_delta: self.target_frame_delta,
            over_budget: scaled_dt > self.target_frame_delta,
            saturated_fixed_steps: fixed_steps >= self.max_steps_per_frame && dropped_time > 0.0,
        }
    }

    pub fn fixed_step_dts(&self, advance: ClockAdvance) -> impl Iterator<Item = f64> {
        std::iter::repeat_n(self.fixed_delta, advance.fixed_steps)
    }

    pub fn set_time_scale(&mut self, time_scale: f64) {
        self.time_scale = time_scale.clamp(0.0, 8.0);
    }

    pub fn configure_fixed_step(&mut self, fixed_delta: f64, max_steps_per_frame: usize) {
        self.fixed_delta = fixed_delta.clamp(0.0001, 0.25);
        self.max_steps_per_frame = max_steps_per_frame.clamp(1, 16);
        self.accumulator = self
            .accumulator
            .min(self.fixed_delta * self.max_steps_per_frame as f64);
    }

    pub fn configure_frame_budget(&mut self, target_fps: u32) {
        self.target_frame_delta = 1.0 / f64::from(target_fps.clamp(15, 360));
    }

    pub fn configure_from_tuning(&mut self, tuning: &RuntimeTuning) {
        self.configure_fixed_step(tuning.fixed_timestep, tuning.max_frame_steps as usize);
        self.configure_frame_budget(tuning.target_fps);
    }

    pub fn from_tuning(tuning: &RuntimeTuning) -> Self {
        let mut clock = Self::new(tuning.fixed_timestep);
        clock.configure_from_tuning(tuning);
        clock
    }

    pub fn frame_budget_ms(&self) -> f64 {
        self.target_frame_delta * 1000.0
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.total_time = 0.0;
        self.frame = 0;
        self.tick = 0;
    }
}
