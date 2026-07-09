use crate::engine::game_clock::ClockAdvance;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FrameHealth {
    #[default]
    Stable,
    OverBudget,
    Saturated,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameDiagnostics {
    pub scaled_dt_ms: f64,
    pub fixed_delta_ms: f64,
    pub fixed_steps: usize,
    pub interpolation_alpha: f64,
    pub dropped_time_ms: f64,
    pub target_frame_ms: f64,
    pub systems_time_ms: f64,
    pub entity_count: usize,
    pub over_budget: bool,
    pub saturated_fixed_steps: bool,
    pub slowest_system: Option<String>,
    pub slowest_system_ms: f64,
    pub health: FrameHealth,
}

impl FrameDiagnostics {
    pub fn headline(&self) -> String {
        let health = match self.health {
            FrameHealth::Stable => "stable",
            FrameHealth::OverBudget => "over budget",
            FrameHealth::Saturated => "fixed step saturated",
        };
        let slowest = self
            .slowest_system
            .as_ref()
            .map(|name| format!("{name} {:.2}ms", self.slowest_system_ms))
            .unwrap_or_else(|| "no systems recorded".to_string());
        format!(
            "{health}: frame {:.2}ms / budget {:.2}ms, systems {:.2}ms, fixed steps {}, slowest {}",
            self.scaled_dt_ms,
            self.target_frame_ms,
            self.systems_time_ms,
            self.fixed_steps,
            slowest
        )
    }

    pub fn action_items(&self) -> Vec<String> {
        let mut actions = Vec::new();
        if self.saturated_fixed_steps {
            actions.push(
                "Reduce trabajo fixed-step o sube runtime_config.max_frame_steps para recuperar picos"
                    .to_string(),
            );
        }
        if self.dropped_time_ms > 0.0 {
            actions.push(format!(
                "Se descartaron {:.2}ms de simulacion; perfila fisica/scripts antes de publicar",
                self.dropped_time_ms
            ));
        }
        if self.over_budget {
            actions.push(format!(
                "El frame excedio el presupuesto por {:.2}ms",
                (self.scaled_dt_ms - self.target_frame_ms).max(0.0)
            ));
        }
        if let Some(system) = &self.slowest_system
            && self.slowest_system_ms > self.target_frame_ms * 0.5
        {
            actions.push(format!(
                "Perfila {system}: uso {:.2}ms de un presupuesto de {:.2}ms",
                self.slowest_system_ms, self.target_frame_ms
            ));
        }
        actions
    }
}

#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    pub fps: f64,
    pub frame_time_ms: f64,
    pub frame_budget_ms: f64,
    pub average_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub uptime: f64,
    pub frames: u64,
    pub dropped_frames: u64,
    pub over_budget_frames: u64,
    pub last_frame: FrameDiagnostics,
    pub warnings: Vec<String>,
}

impl Diagnostics {
    pub fn update(&mut self, dt: f64) {
        self.update_with_budget(dt, 1000.0 / 30.0);
    }

    pub fn update_with_budget(&mut self, dt: f64, frame_budget_ms: f64) {
        self.uptime += dt;
        self.frame_time_ms = dt * 1000.0;
        self.frame_budget_ms = frame_budget_ms.max(0.1);
        self.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        self.frames = self.frames.saturating_add(1);
        if self.frame_time_ms > self.frame_budget_ms {
            self.over_budget_frames = self.over_budget_frames.saturating_add(1);
        }
        if self.frame_time_ms > self.frame_budget_ms * 2.0 {
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
        self.over_budget_frames = 0;
        self.last_frame = FrameDiagnostics::default();
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
        let late_ratio = self.over_budget_frames as f64 / self.frames as f64;
        let drop_ratio = self.dropped_frames as f64 / self.frames as f64;
        (1.0 - late_ratio * 0.5 - drop_ratio * 0.5).clamp(0.0, 1.0)
    }

    pub fn record_frame_runtime(
        &mut self,
        advance: ClockAdvance,
        fixed_delta: f64,
        entity_count: usize,
        systems_time_ms: f64,
        slowest_system: Option<(String, f64)>,
    ) {
        let target_frame_ms = advance.target_frame_delta * 1000.0;
        let (slowest_system, slowest_system_ms) = slowest_system
            .map(|(name, ms)| (Some(name), ms))
            .unwrap_or((None, 0.0));
        let health = if advance.saturated_fixed_steps || advance.dropped_time > 0.0 {
            FrameHealth::Saturated
        } else if advance.over_budget || systems_time_ms > target_frame_ms {
            FrameHealth::OverBudget
        } else {
            FrameHealth::Stable
        };
        self.last_frame = FrameDiagnostics {
            scaled_dt_ms: advance.scaled_dt * 1000.0,
            fixed_delta_ms: fixed_delta * 1000.0,
            fixed_steps: advance.fixed_steps,
            interpolation_alpha: advance.interpolation_alpha,
            dropped_time_ms: advance.dropped_time * 1000.0,
            target_frame_ms,
            systems_time_ms,
            entity_count,
            over_budget: advance.over_budget,
            saturated_fixed_steps: advance.saturated_fixed_steps,
            slowest_system,
            slowest_system_ms,
            health,
        };
        for action in self.last_frame.action_items() {
            self.push_warning(action);
        }
    }

    pub fn health_summary(&self) -> String {
        if self.frames == 0 {
            return "no frames recorded".to_string();
        }
        format!(
            "{} | stability {:.0}% | avg {:.2}ms | warnings {}",
            self.last_frame.headline(),
            self.stability_score() * 100.0,
            self.average_frame_time_ms,
            self.warnings.len()
        )
    }
}
