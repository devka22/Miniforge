use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::game_clock::ClockAdvance;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StabilityLevel {
    #[default]
    Stable,
    Guarded,
    Recovery,
}

impl StabilityLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Guarded => "guarded",
            Self::Recovery => "recovery",
        }
    }

    pub fn optional_cadence_divisor(self) -> u64 {
        match self {
            Self::Stable => 1,
            Self::Guarded => 2,
            Self::Recovery => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RuntimeStabilityConfig {
    pub enabled: bool,
    pub max_delta_seconds: f64,
    pub repair_invalid_numbers: bool,
    pub quarantine_corrupt_entities: bool,
    pub repairs_before_quarantine: usize,
    pub guarded_after_slow_frames: u32,
    pub recovery_after_slow_frames: u32,
    pub stable_frames_to_recover: u32,
    pub throttle_optional_systems: bool,
    pub max_world_coordinate: f64,
    pub max_entities: usize,
}

impl Default for RuntimeStabilityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_delta_seconds: 0.1,
            repair_invalid_numbers: true,
            quarantine_corrupt_entities: true,
            repairs_before_quarantine: 8,
            guarded_after_slow_frames: 4,
            recovery_after_slow_frames: 12,
            stable_frames_to_recover: 90,
            throttle_optional_systems: true,
            max_world_coordinate: 1_000_000_000.0,
            max_entities: 5_000,
        }
    }
}

impl RuntimeStabilityConfig {
    pub fn from_runtime_config(data: &Value, max_entities: usize) -> Self {
        let defaults = Self {
            max_entities: max_entities.max(1),
            ..Self::default()
        };
        let Some(value) = data.get("stability_guard") else {
            return defaults;
        };
        Self {
            enabled: bool_value(value, "enabled", defaults.enabled),
            max_delta_seconds: float_value(value, "max_delta_seconds", defaults.max_delta_seconds)
                .clamp(0.001, 0.25),
            repair_invalid_numbers: bool_value(
                value,
                "repair_invalid_numbers",
                defaults.repair_invalid_numbers,
            ),
            quarantine_corrupt_entities: bool_value(
                value,
                "quarantine_corrupt_entities",
                defaults.quarantine_corrupt_entities,
            ),
            repairs_before_quarantine: usize_value(
                value,
                "repairs_before_quarantine",
                defaults.repairs_before_quarantine,
            )
            .clamp(1, 128),
            guarded_after_slow_frames: u32_value(
                value,
                "guarded_after_slow_frames",
                defaults.guarded_after_slow_frames,
            )
            .clamp(1, 10_000),
            recovery_after_slow_frames: u32_value(
                value,
                "recovery_after_slow_frames",
                defaults.recovery_after_slow_frames,
            )
            .clamp(2, 20_000),
            stable_frames_to_recover: u32_value(
                value,
                "stable_frames_to_recover",
                defaults.stable_frames_to_recover,
            )
            .clamp(1, 100_000),
            throttle_optional_systems: bool_value(
                value,
                "throttle_optional_systems",
                defaults.throttle_optional_systems,
            ),
            max_world_coordinate: float_value(
                value,
                "max_world_coordinate",
                defaults.max_world_coordinate,
            )
            .clamp(1_000.0, 1.0e15),
            max_entities: max_entities.max(1),
        }
        .normalized()
    }

    fn normalized(mut self) -> Self {
        self.recovery_after_slow_frames = self
            .recovery_after_slow_frames
            .max(self.guarded_after_slow_frames.saturating_add(1));
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RuntimeStabilityReport {
    pub frame: u64,
    pub level: StabilityLevel,
    pub raw_delta_seconds: f64,
    pub safe_delta_seconds: f64,
    pub delta_was_invalid: bool,
    pub delta_was_clamped: bool,
    pub repaired_values: usize,
    pub quarantined_entity_ids: Vec<u64>,
    pub entity_count: usize,
    pub entity_limit_exceeded_by: usize,
    pub consecutive_slow_frames: u32,
    pub consecutive_stable_frames: u32,
    pub optional_cadence_divisor: u64,
}

impl RuntimeStabilityReport {
    pub fn healthy(&self) -> bool {
        self.level == StabilityLevel::Stable
            && !self.delta_was_invalid
            && !self.delta_was_clamped
            && self.repaired_values == 0
            && self.quarantined_entity_ids.is_empty()
            && self.entity_limit_exceeded_by == 0
    }

    pub fn summary(&self) -> String {
        format!(
            "{} · dt {:.2}ms · repairs {} · quarantined {} · entities {}",
            self.level.label(),
            self.safe_delta_seconds * 1000.0,
            self.repaired_values,
            self.quarantined_entity_ids.len(),
            self.entity_count,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeStabilityGuard {
    pub config: RuntimeStabilityConfig,
    pub last_frame: RuntimeStabilityReport,
    pub total_clamped_deltas: u64,
    pub total_invalid_deltas: u64,
    pub total_repaired_values: u64,
    pub total_quarantined_entities: u64,
    level: StabilityLevel,
    frame: u64,
    consecutive_slow_frames: u32,
    consecutive_stable_frames: u32,
    optional_accumulator: f64,
    last_delta_event_frame: u64,
    entity_limit_warning_active: bool,
    quarantined_entity_ids: BTreeSet<u64>,
    events: Vec<String>,
}

impl Default for RuntimeStabilityGuard {
    fn default() -> Self {
        Self::new(RuntimeStabilityConfig::default())
    }
}

impl RuntimeStabilityGuard {
    pub fn new(config: RuntimeStabilityConfig) -> Self {
        Self {
            config,
            last_frame: RuntimeStabilityReport::default(),
            total_clamped_deltas: 0,
            total_invalid_deltas: 0,
            total_repaired_values: 0,
            total_quarantined_entities: 0,
            level: StabilityLevel::Stable,
            frame: 0,
            consecutive_slow_frames: 0,
            consecutive_stable_frames: 0,
            optional_accumulator: 0.0,
            last_delta_event_frame: 0,
            entity_limit_warning_active: false,
            quarantined_entity_ids: BTreeSet::new(),
            events: Vec::new(),
        }
    }

    pub fn from_runtime_config(data: &Value, max_entities: usize) -> Self {
        Self::new(RuntimeStabilityConfig::from_runtime_config(
            data,
            max_entities,
        ))
    }

    /// Starts a frame with a finite, non-negative and bounded delta. The safe
    /// value must be used by both the clock and variable-step systems.
    pub fn begin_frame(&mut self, raw_delta_seconds: f64) -> f64 {
        self.frame = self.frame.saturating_add(1);
        let invalid = !raw_delta_seconds.is_finite() || raw_delta_seconds < 0.0;
        let finite_delta = if invalid { 0.0 } else { raw_delta_seconds };
        let maximum = if self.config.enabled {
            self.config.max_delta_seconds
        } else {
            0.25
        };
        let safe_delta = finite_delta.min(maximum);
        let clamped = finite_delta > safe_delta;

        if invalid {
            self.total_invalid_deltas = self.total_invalid_deltas.saturating_add(1);
            if self.should_emit_delta_event() {
                self.events
                    .push("Stability Guard reemplazo un delta de frame invalido por 0".to_string());
                self.last_delta_event_frame = self.frame;
            }
        }
        if clamped {
            self.total_clamped_deltas = self.total_clamped_deltas.saturating_add(1);
            if self.should_emit_delta_event() {
                self.events.push(format!(
                    "Stability Guard limito un pico de {:.2}ms a {:.2}ms",
                    raw_delta_seconds * 1000.0,
                    safe_delta * 1000.0
                ));
                self.last_delta_event_frame = self.frame;
            }
        }

        self.last_frame = RuntimeStabilityReport {
            frame: self.frame,
            level: self.level,
            raw_delta_seconds: if raw_delta_seconds.is_finite() {
                raw_delta_seconds
            } else {
                0.0
            },
            safe_delta_seconds: safe_delta,
            delta_was_invalid: invalid,
            delta_was_clamped: clamped,
            optional_cadence_divisor: self.optional_cadence_divisor(),
            ..RuntimeStabilityReport::default()
        };
        safe_delta
    }

    /// Repairs non-finite runtime state before it can poison physics, spatial
    /// indexing or rendering. Heavily corrupted entities are quarantined for
    /// the rest of the session instead of crashing the whole game.
    pub fn sanitize_entities(&mut self, entities: &mut [GameObject]) -> usize {
        if !self.config.enabled || !self.config.repair_invalid_numbers {
            return 0;
        }

        self.quarantined_entity_ids.retain(|entity_id| {
            entities
                .iter()
                .any(|entity| entity.id == *entity_id && !entity.is_runtime_active())
        });

        let mut repaired_total = 0usize;
        for entity in entities {
            let repaired = sanitize_entity(entity, self.config.max_world_coordinate);
            if repaired == 0 {
                continue;
            }
            repaired_total = repaired_total.saturating_add(repaired);
            if self.config.quarantine_corrupt_entities
                && repaired >= self.config.repairs_before_quarantine
                && entity.is_runtime_active()
            {
                entity.active = false;
                entity.enabled = false;
                self.last_frame.quarantined_entity_ids.push(entity.id);
                if self.quarantined_entity_ids.insert(entity.id) {
                    self.total_quarantined_entities =
                        self.total_quarantined_entities.saturating_add(1);
                }
            }
        }

        if repaired_total > 0 {
            self.last_frame.repaired_values = self
                .last_frame
                .repaired_values
                .saturating_add(repaired_total);
            self.total_repaired_values = self
                .total_repaired_values
                .saturating_add(repaired_total as u64);
            self.events.push(format!(
                "Stability Guard reparo {repaired_total} valores numericos del mundo"
            ));
        }
        if !self.last_frame.quarantined_entity_ids.is_empty() {
            self.last_frame.quarantined_entity_ids.sort_unstable();
            self.last_frame.quarantined_entity_ids.dedup();
            self.events.push(format!(
                "Stability Guard puso en cuarentena {} entidades corruptas",
                self.last_frame.quarantined_entity_ids.len()
            ));
        }
        repaired_total
    }

    /// Returns accumulated time only on the cadence allowed for cosmetic work.
    /// Core simulation, input, gameplay, scripts and physics are never skipped.
    pub fn optional_system_delta(&mut self, simulation_delta: f64) -> Option<f64> {
        let simulation_delta = if simulation_delta.is_finite() {
            simulation_delta.max(0.0)
        } else {
            0.0
        };
        self.optional_accumulator =
            (self.optional_accumulator + simulation_delta).min(self.config.max_delta_seconds);
        let divisor = self.optional_cadence_divisor();
        if divisor > 1 && !self.frame.is_multiple_of(divisor) {
            return None;
        }
        let accumulated = std::mem::take(&mut self.optional_accumulator);
        Some(accumulated)
    }

    pub fn observe_frame(
        &mut self,
        advance: ClockAdvance,
        systems_time_ms: f64,
        entity_count: usize,
    ) {
        let target_ms = (advance.target_frame_delta * 1000.0).max(0.1);
        let exceeded_by = entity_count.saturating_sub(self.config.max_entities);
        let slow = advance.over_budget
            || advance.saturated_fixed_steps
            || advance.dropped_time > 0.0
            || systems_time_ms > target_ms
            || exceeded_by > 0;

        if slow {
            self.consecutive_slow_frames = self.consecutive_slow_frames.saturating_add(1);
            self.consecutive_stable_frames = 0;
        } else {
            self.consecutive_stable_frames = self.consecutive_stable_frames.saturating_add(1);
            self.consecutive_slow_frames = 0;
        }

        if self.config.enabled {
            let previous = self.level;
            if self.consecutive_slow_frames >= self.config.recovery_after_slow_frames {
                self.level = StabilityLevel::Recovery;
            } else if self.consecutive_slow_frames >= self.config.guarded_after_slow_frames {
                self.level = StabilityLevel::Guarded;
            } else if self.consecutive_stable_frames >= self.config.stable_frames_to_recover {
                self.level = match self.level {
                    StabilityLevel::Recovery => StabilityLevel::Guarded,
                    StabilityLevel::Guarded => StabilityLevel::Stable,
                    StabilityLevel::Stable => StabilityLevel::Stable,
                };
                self.consecutive_stable_frames = 0;
            }
            if previous != self.level {
                self.events.push(format!(
                    "Stability Guard cambio de {} a {}",
                    previous.label(),
                    self.level.label()
                ));
            }
        }

        if exceeded_by > 0 && !self.entity_limit_warning_active {
            self.events.push(format!(
                "El mundo supera max_entities por {exceeded_by}; se mantiene el contenido pero se activa presion de runtime"
            ));
            self.entity_limit_warning_active = true;
        } else if exceeded_by == 0 && self.entity_limit_warning_active {
            self.events
                .push("La cantidad de entidades volvio a estar dentro del presupuesto".to_string());
            self.entity_limit_warning_active = false;
        }
        self.last_frame.level = self.level;
        self.last_frame.entity_count = entity_count;
        self.last_frame.entity_limit_exceeded_by = exceeded_by;
        self.last_frame.consecutive_slow_frames = self.consecutive_slow_frames;
        self.last_frame.consecutive_stable_frames = self.consecutive_stable_frames;
        self.last_frame.optional_cadence_divisor = self.optional_cadence_divisor();
    }

    pub fn level(&self) -> StabilityLevel {
        self.level
    }

    pub fn optional_cadence_divisor(&self) -> u64 {
        if self.config.enabled && self.config.throttle_optional_systems {
            self.level.optional_cadence_divisor()
        } else {
            1
        }
    }

    pub fn quarantined_entity_count(&self) -> usize {
        self.quarantined_entity_ids.len()
    }

    pub fn take_events(&mut self) -> Vec<String> {
        let mut events = std::mem::take(&mut self.events);
        events.sort();
        events.dedup();
        events
    }

    fn should_emit_delta_event(&self) -> bool {
        self.last_delta_event_frame == 0
            || self.frame.saturating_sub(self.last_delta_event_frame) >= 120
    }
}

fn sanitize_entity(entity: &mut GameObject, max_coordinate: f64) -> usize {
    let mut repaired = 0usize;
    repaired += repair_bounded(&mut entity.x, 0.0, -max_coordinate, max_coordinate);
    repaired += repair_bounded(&mut entity.y, 0.0, -max_coordinate, max_coordinate);
    repaired += repair_bounded(&mut entity.local_x, 0.0, -max_coordinate, max_coordinate);
    repaired += repair_bounded(&mut entity.local_y, 0.0, -max_coordinate, max_coordinate);
    repaired += repair_bounded(&mut entity.rotation, 0.0, -1.0e12, 1.0e12);
    repaired += repair_bounded(&mut entity.scale_x, 1.0, -1.0e6, 1.0e6);
    repaired += repair_bounded(&mut entity.scale_y, 1.0, -1.0e6, 1.0e6);
    repaired += repair_bounded(&mut entity.width, 1.0, 0.0001, 1.0e9);
    repaired += repair_bounded(&mut entity.height, 1.0, 0.0001, 1.0e9);
    repaired += repair_bounded(&mut entity.speed, 0.0, 0.0, 1.0e9);
    repaired += repair_bounded(&mut entity.radius, 0.45, 0.0001, 1.0e9);

    repaired += sanitize_points(&mut entity.path, max_coordinate);
    repaired += sanitize_points(&mut entity.patrol_points, max_coordinate);
    if let Some((x, y)) = entity.attack_move_target
        && (!x.is_finite()
            || !y.is_finite()
            || x.abs() > max_coordinate
            || y.abs() > max_coordinate)
    {
        entity.attack_move_target = None;
        repaired += 1;
    }

    if repaired > 0 {
        entity.sync_to_components();
    }
    repaired
}

fn repair_bounded(value: &mut f64, fallback: f64, minimum: f64, maximum: f64) -> usize {
    let repaired = if !value.is_finite() {
        fallback
    } else {
        value.clamp(minimum, maximum)
    };
    if repaired == *value {
        return 0;
    }
    *value = repaired;
    1
}

fn sanitize_points(points: &mut Vec<(f64, f64)>, max_coordinate: f64) -> usize {
    let before = points.len();
    points.retain(|(x, y)| {
        x.is_finite() && y.is_finite() && x.abs() <= max_coordinate && y.abs() <= max_coordinate
    });
    before.saturating_sub(points.len())
}

fn bool_value(value: &Value, key: &str, fallback: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

fn float_value(value: &Value, key: &str, fallback: f64) -> f64 {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

fn usize_value(value: &Value, key: &str, fallback: usize) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value.min(usize::MAX as u64) as usize)
        .unwrap_or(fallback)
}

fn u32_value(value: &Value, key: &str, fallback: u32) -> u32 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value.min(u32::MAX as u64) as u32)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{RuntimeStabilityConfig, RuntimeStabilityGuard, StabilityLevel};
    use crate::engine::asset_tools::AssetTools;
    use crate::engine::game_clock::GameClock;
    use crate::entities::game_object::GameObject;
    use crate::runtime::EngineRuntime;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestProject(std::path::PathBuf);

    impl TestProject {
        fn new() -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "miniforge_runtime_stability_{}_{}",
                std::process::id(),
                sequence
            ));
            AssetTools::ensure_project_folders(&path).expect("test project");
            Self(path)
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn invalid_and_spiking_delta_are_made_safe() {
        let mut guard = RuntimeStabilityGuard::default();
        assert_eq!(guard.begin_frame(f64::NAN), 0.0);
        assert!(guard.last_frame.delta_was_invalid);
        assert_eq!(guard.begin_frame(2.0), 0.1);
        assert!(guard.last_frame.delta_was_clamped);
        assert_eq!(guard.total_invalid_deltas, 1);
        assert_eq!(guard.total_clamped_deltas, 1);
    }

    #[test]
    fn corrupt_entity_is_repaired_and_quarantined() {
        let mut guard = RuntimeStabilityGuard::new(RuntimeStabilityConfig {
            repairs_before_quarantine: 3,
            ..RuntimeStabilityConfig::default()
        });
        guard.begin_frame(1.0 / 60.0);
        let mut entity = GameObject::new(0.0, 0.0, Some("Broken".to_string()));
        entity.x = f64::NAN;
        entity.y = f64::INFINITY;
        entity.width = -1.0;
        entity.path.push((f64::NAN, 2.0));
        let id = entity.id;
        let mut entities = vec![entity];

        assert_eq!(guard.sanitize_entities(&mut entities), 4);
        assert!(entities[0].x.is_finite());
        assert!(entities[0].width > 0.0);
        assert!(!entities[0].enabled);
        assert_eq!(guard.last_frame.quarantined_entity_ids, vec![id]);
        assert_eq!(guard.quarantined_entity_count(), 1);

        guard.begin_frame(1.0 / 60.0);
        assert_eq!(guard.sanitize_entities(&mut entities), 0);
        assert_eq!(guard.quarantined_entity_count(), 1);
        entities.clear();
        guard.sanitize_entities(&mut entities);
        assert_eq!(guard.quarantined_entity_count(), 0);
    }

    #[test]
    fn sustained_pressure_degrades_only_optional_cadence_and_recovers() {
        let mut guard = RuntimeStabilityGuard::new(RuntimeStabilityConfig {
            guarded_after_slow_frames: 2,
            recovery_after_slow_frames: 3,
            stable_frames_to_recover: 2,
            ..RuntimeStabilityConfig::default()
        });
        let mut clock = GameClock::new(1.0 / 60.0);

        for _ in 0..3 {
            let dt = guard.begin_frame(0.1);
            let advance = clock.advance(dt);
            guard.observe_frame(advance, 40.0, 1);
        }
        assert_eq!(guard.level(), StabilityLevel::Recovery);
        assert_eq!(guard.optional_cadence_divisor(), 4);

        for _ in 0..4 {
            let dt = guard.begin_frame(1.0 / 120.0);
            let advance = clock.advance(dt);
            guard.observe_frame(advance, 0.1, 1);
        }
        assert_eq!(guard.level(), StabilityLevel::Stable);
    }

    #[test]
    fn entity_limit_creates_pressure_without_deleting_game_content() {
        let mut guard = RuntimeStabilityGuard::new(RuntimeStabilityConfig {
            max_entities: 2,
            guarded_after_slow_frames: 1,
            ..RuntimeStabilityConfig::default()
        });
        let dt = guard.begin_frame(1.0 / 120.0);
        let mut clock = GameClock::default();
        let advance = clock.advance(dt);
        guard.observe_frame(advance, 0.1, 5);
        assert_eq!(guard.last_frame.entity_limit_exceeded_by, 3);
        assert_eq!(guard.level(), StabilityLevel::Guarded);
    }

    #[test]
    fn exported_runtime_repairs_world_state_before_spatial_sync() {
        let project = TestProject::new();
        let mut runtime = EngineRuntime::new(&project.0).expect("runtime");
        let mut broken = GameObject::new(0.0, 0.0, Some("Broken".to_string()));
        let id = broken.id;
        broken.x = f64::NAN;
        broken.path.push((f64::INFINITY, 0.0));
        runtime.runtime_world.replace_entities(vec![broken]);

        runtime.run_headless_once(1.0 / 60.0);

        assert!(runtime.runtime_world.units[0].x.is_finite());
        assert!(runtime.runtime_world.units[0].path.is_empty());
        assert!(runtime.runtime_world.entity(id).is_some());
        assert_eq!(runtime.profiler.counters["StabilityRepairs"], 2);
        assert!(
            runtime
                .diagnostics
                .warnings
                .iter()
                .any(|warning| { warning.contains("Stability Guard reparo 2 valores") })
        );
    }

    #[test]
    fn exported_runtime_pause_freezes_variable_step_systems() {
        let project = TestProject::new();
        let mut runtime = EngineRuntime::new(&project.0).expect("runtime");
        let mut entity = GameObject::new_unit(1.0, 1.0, Some("Paused".to_string()));
        entity.command = "MOVE".to_string();
        entity.path.push((20.0, 1.0));
        runtime.runtime_world.replace_entities(vec![entity]);
        runtime.clock.paused = true;

        runtime.run_headless_once(0.1);

        assert_eq!(runtime.runtime_world.units[0].x, 1.0);
        assert_eq!(runtime.runtime_world.units[0].y, 1.0);
        assert_eq!(runtime.diagnostics.last_frame.scaled_dt_ms, 0.0);
        assert_eq!(runtime.profiler.metrics["SafeFrameDtMs"], 100.0);
        assert_eq!(runtime.profiler.metrics["FrameDtMs"], 0.0);
    }
}
