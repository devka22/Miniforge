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
            fixed_delta: finite_or(fixed_delta, 1.0 / 60.0).max(0.0001),
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
        self.sanitize_configuration();
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

        let real_dt = finite_or(real_dt, 0.0).clamp(0.0, 0.25);
        let scaled_dt = (real_dt * self.time_scale).min(0.25);
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
        self.time_scale = finite_or(time_scale, 1.0).clamp(0.0, 8.0);
    }

    pub fn configure_fixed_step(&mut self, fixed_delta: f64, max_steps_per_frame: usize) {
        self.fixed_delta = finite_or(fixed_delta, 1.0 / 60.0).clamp(0.0001, 0.25);
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

    fn sanitize_configuration(&mut self) {
        self.fixed_delta = finite_or(self.fixed_delta, 1.0 / 60.0).clamp(0.0001, 0.25);
        self.target_frame_delta =
            finite_or(self.target_frame_delta, 1.0 / 60.0).clamp(1.0 / 360.0, 1.0);
        self.time_scale = finite_or(self.time_scale, 1.0).clamp(0.0, 8.0);
        self.accumulator = finite_or(self.accumulator, 0.0).clamp(
            0.0,
            self.fixed_delta * self.max_steps_per_frame.clamp(1, 16) as f64,
        );
        self.max_steps_per_frame = self.max_steps_per_frame.clamp(1, 16);
        self.total_time = finite_or(self.total_time, 0.0).max(0.0);
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::GameClock;

    #[test]
    fn invalid_public_clock_state_cannot_poison_the_simulation() {
        let mut clock = GameClock::new(f64::NAN);
        clock.fixed_delta = f64::NAN;
        clock.target_frame_delta = f64::INFINITY;
        clock.time_scale = f64::NAN;
        clock.accumulator = f64::NEG_INFINITY;
        clock.total_time = f64::NAN;
        clock.max_steps_per_frame = 0;

        let advance = clock.advance(f64::NAN);

        assert_eq!(advance.scaled_dt, 0.0);
        assert!(clock.fixed_delta.is_finite());
        assert!(clock.target_frame_delta.is_finite());
        assert!(clock.time_scale.is_finite());
        assert!(clock.accumulator.is_finite());
        assert!(clock.total_time.is_finite());
        assert_eq!(clock.max_steps_per_frame, 1);
    }

    #[test]
    fn paused_clock_returns_zero_simulation_delta() {
        let mut clock = GameClock {
            paused: true,
            ..GameClock::default()
        };
        let advance = clock.advance(1.0);
        assert_eq!(advance.scaled_dt, 0.0);
        assert_eq!(advance.fixed_steps, 0);
    }
}
