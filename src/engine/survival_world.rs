//! Reusable stealth, perception and barricadable-world interactions.
//!
//! These systems operate on ordinary MiniForge components and entity
//! transforms. They contain no project-specific tags, balance values or art,
//! so the same authoring surface can power survival, stealth, horror and RPG
//! projects without a custom Rust gameplay loop.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::default_component;
use crate::engine::survival_systems::SurvivalSystems;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseEvent2D {
    pub source_id: u64,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub intensity: f64,
    pub kind: String,
    pub age: f64,
    pub duration: f64,
}

impl NoiseEvent2D {
    pub fn active(&self) -> bool {
        self.radius > 0.0 && self.intensity > 0.0 && self.age < self.duration
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerceptionResult {
    pub detected: bool,
    pub target_id: Option<u64>,
    pub stimulus: String,
    pub x: f64,
    pub y: f64,
    pub distance: f64,
    pub score: f64,
    pub alertness: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorCommand<'a> {
    Toggle,
    Open,
    Close,
    Lock,
    Unlock { key_id: Option<&'a str> },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DoorActionResult {
    pub success: bool,
    pub state: String,
    pub locked: bool,
    pub blocked_by_barricade: bool,
    pub emitted_noise_radius: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BarricadeResult {
    pub success: bool,
    pub layers: i64,
    pub health: f64,
    pub destroyed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SurvivalWorldSystems;

impl SurvivalWorldSystems {
    pub fn set_crouching(entity: &mut GameObject, crouching: bool) -> bool {
        ensure_component(entity, "StealthState2D");
        let Some(stealth) = entity.get_component_mut("StealthState2D") else {
            return false;
        };
        stealth.set("crouching", json!(crouching));
        let movement = if crouching {
            stealth.get_f64("crouch_movement_multiplier", 0.55)
        } else {
            1.0
        };
        stealth.set_f64("movement_multiplier", movement.clamp(0.05, 3.0));
        true
    }

    pub fn movement_multiplier(entity: &GameObject) -> f64 {
        entity
            .get_component("StealthState2D")
            .map(|stealth| stealth.get_f64("movement_multiplier", 1.0))
            .unwrap_or(1.0)
            .clamp(0.05, 3.0)
            * SurvivalSystems::equipment_summary(entity).movement_multiplier
    }

    pub fn visibility(entity: &GameObject) -> f64 {
        let Some(stealth) = entity.get_component("StealthState2D") else {
            return 1.0;
        };
        let mut value = stealth.get_f64("visibility", 1.0)
            * stealth.get_f64("light_exposure", 1.0).clamp(0.05, 2.0);
        if stealth.get_bool("concealed", false) {
            value *= stealth.get_f64("concealment_visibility_multiplier", 0.35);
        }
        if stealth.get_bool("crouching", false) {
            value *= 0.7;
        }
        finite_or(value, 1.0).clamp(0.02, 3.0)
    }

    /// Emits a transient, serializable noise sample and mirrors its current
    /// level onto the source component for editor gizmos and UI bindings.
    pub fn emit_noise(entity: &mut GameObject, kind: &str, scale: f64) -> NoiseEvent2D {
        ensure_component(entity, "NoiseEmitter2D");
        let equipment_noise = SurvivalSystems::equipment_summary(entity).noise;
        let (radius, intensity, duration) = {
            let emitter = entity
                .get_component("NoiseEmitter2D")
                .expect("noise emitter was ensured");
            if !emitter.enabled || !emitter.get_bool("enabled", true) {
                return silent_noise(entity, kind);
            }
            let field = match kind {
                "sprint" => "sprint_radius",
                "crouch" => "crouch_radius",
                "interaction" => "interaction_radius",
                "combat" => "combat_radius",
                _ => "movement_radius",
            };
            let mut multiplier = entity
                .get_component("StealthState2D")
                .map(|stealth| {
                    if stealth.get_bool("crouching", false) {
                        stealth.get_f64("crouch_noise_multiplier", 0.3)
                    } else {
                        stealth.get_f64("noise_multiplier", 1.0)
                    }
                })
                .unwrap_or(1.0);
            multiplier *= emitter.get_f64("surface_multiplier", 1.0);
            multiplier += equipment_noise * emitter.get_f64("equipment_noise_scale", 0.08);
            let radius =
                emitter.get_f64(field, 2.5) * finite_or(scale, 0.0).max(0.0) * multiplier.max(0.0);
            let decay = emitter.get_f64("decay_per_second", 12.0).max(0.01);
            (
                radius.max(0.0),
                (radius / 10.0).clamp(0.0, 1.0),
                (radius / decay).clamp(0.08, 3.0),
            )
        };
        if let Some(emitter) = entity.get_component_mut("NoiseEmitter2D") {
            emitter.set_f64("current_radius", radius);
            emitter.set_f64("current_intensity", intensity);
            emitter.set("last_kind", json!(kind));
        }
        NoiseEvent2D {
            source_id: entity.id,
            x: entity.x,
            y: entity.y,
            radius,
            intensity,
            kind: kind.to_string(),
            age: 0.0,
            duration,
        }
    }

    pub fn tick_noise(entity: &mut GameObject, dt: f64) {
        let Some(emitter) = entity.get_component_mut("NoiseEmitter2D") else {
            return;
        };
        let dt = finite_or(dt, 0.0).clamp(0.0, 0.25);
        let decay = emitter.get_f64("decay_per_second", 12.0).max(0.0) * dt;
        emitter.set_f64(
            "current_radius",
            (emitter.get_f64("current_radius", 0.0) - decay).max(0.0),
        );
        emitter.set_f64(
            "current_intensity",
            (emitter.get_f64("current_intensity", 0.0) - decay / 10.0).max(0.0),
        );
    }

    pub fn tick_noise_events(events: &mut Vec<NoiseEvent2D>, dt: f64) {
        let dt = finite_or(dt, 0.0).clamp(0.0, 0.25);
        for event in events.iter_mut() {
            event.age += dt;
            let remaining = (1.0 - event.age / event.duration.max(0.001)).clamp(0.0, 1.0);
            event.intensity *= remaining;
        }
        events.retain(NoiseEvent2D::active);
    }

    /// Resolves the strongest visible or audible target. Sight uses observer
    /// rotation and FOV; hearing consumes emitted world-space noise events.
    pub fn perceive(
        observer: &GameObject,
        candidates: &[GameObject],
        noises: &[NoiseEvent2D],
    ) -> PerceptionResult {
        let Some(senses) = observer.get_component("Senses2D") else {
            return PerceptionResult::default();
        };
        if !senses.enabled || !senses.get_bool("enabled", true) {
            return PerceptionResult::default();
        }
        let target_tags = senses.get_string_list("target_tags");
        let sight_range = senses.get_f64("sight_range", 9.0).max(0.0);
        let hearing_range = senses.get_f64("hearing_range", 14.0).max(0.0);
        let half_fov = senses.get_f64("fov_degrees", 120.0).clamp(0.0, 360.0) * 0.5;
        let mut best = PerceptionResult::default();

        for target in candidates.iter().filter(|target| {
            target.id != observer.id
                && target.is_runtime_active()
                && (target_tags.is_empty() || target_tags.iter().any(|tag| tag == &target.tag))
        }) {
            let dx = target.x - observer.x;
            let dy = target.y - observer.y;
            let distance = dx.hypot(dy);
            let visible_range = sight_range * Self::visibility(target);
            let relative = normalize_degrees(dy.atan2(dx).to_degrees() - observer.rotation);
            if distance <= visible_range && (half_fov >= 180.0 || relative.abs() <= half_fov) {
                let score = (1.0 - distance / visible_range.max(0.001)).clamp(0.0, 1.0)
                    * Self::visibility(target);
                if !best.detected || score > best.score {
                    best = PerceptionResult {
                        detected: true,
                        target_id: Some(target.id),
                        stimulus: "sight".to_string(),
                        x: target.x,
                        y: target.y,
                        distance,
                        score,
                        alertness: 0.0,
                    };
                }
            }
        }

        for noise in noises.iter().filter(|event| event.active()) {
            let Some(target) = candidates
                .iter()
                .find(|target| target.id == noise.source_id)
            else {
                continue;
            };
            if !target_tags.is_empty() && !target_tags.iter().any(|tag| tag == &target.tag) {
                continue;
            }
            let distance = (noise.x - observer.x).hypot(noise.y - observer.y);
            let audible_range = noise.radius.min(hearing_range);
            if distance > audible_range {
                continue;
            }
            let score = noise.intensity * (1.0 - distance / audible_range.max(0.001));
            if !best.detected || score > best.score {
                best = PerceptionResult {
                    detected: true,
                    target_id: Some(noise.source_id),
                    stimulus: format!("hearing:{}", noise.kind),
                    x: noise.x,
                    y: noise.y,
                    distance,
                    score,
                    alertness: 0.0,
                };
            }
        }
        best
    }

    pub fn update_perception(
        observer: &mut GameObject,
        candidates: &[GameObject],
        noises: &[NoiseEvent2D],
        dt: f64,
    ) -> PerceptionResult {
        let sample = Self::perceive(observer, candidates, noises);
        let Some(senses) = observer.get_component_mut("Senses2D") else {
            return sample;
        };
        let dt = finite_or(dt, 0.0).clamp(0.0, 0.25);
        let mut result = sample;
        let current = senses.get_f64("alertness", 0.0);
        let alertness = if result.detected {
            let next = current + senses.get_f64("alert_gain", 45.0).max(0.0) * result.score * dt;
            senses.set("last_target_id", json!(result.target_id));
            senses.set("last_stimulus", json!(result.stimulus));
            senses.set_f64("last_known_x", result.x);
            senses.set_f64("last_known_y", result.y);
            senses.set_f64(
                "memory_remaining",
                senses.get_f64("memory_seconds", 4.0).max(0.0),
            );
            next
        } else {
            let memory = (senses.get_f64("memory_remaining", 0.0) - dt).max(0.0);
            senses.set_f64("memory_remaining", memory);
            if memory <= 0.0 {
                senses.set("last_target_id", Value::Null);
                senses.set("last_stimulus", json!("none"));
            }
            current - senses.get_f64("alert_decay", 10.0).max(0.0) * dt
        }
        .clamp(0.0, 100.0);
        senses.set_f64("alertness", alertness);
        result.alertness = alertness;
        result
    }

    pub fn door_action(target: &mut GameObject, command: DoorCommand<'_>) -> DoorActionResult {
        ensure_component(target, "Door2D");
        let barricaded = target
            .get_component("Barricade2D")
            .map(|value| value.get_i64("layers", 0) > 0 && value.get_f64("health", 0.0) > 0.0)
            .unwrap_or(false);
        let Some(mut door) = target.get_component("Door2D").cloned() else {
            return door_failure("component_missing", false, barricaded);
        };
        let mut locked = door.get_bool("locked", false);
        let is_open =
            door.get_bool("target_open", false) || door.get_string("state", "closed") == "open";
        let changes_motion = matches!(
            command,
            DoorCommand::Toggle | DoorCommand::Open | DoorCommand::Close
        );
        let wants_open = match command {
            DoorCommand::Toggle => !is_open,
            DoorCommand::Open => true,
            DoorCommand::Close => false,
            DoorCommand::Lock => {
                if is_open {
                    return door_failure("close_before_locking", locked, barricaded);
                }
                locked = true;
                false
            }
            DoorCommand::Unlock { key_id } => {
                let required = door.get("key_id").and_then(Value::as_str);
                if required.is_some() && required != key_id {
                    return door_failure("key_required", locked, barricaded);
                }
                locked = false;
                is_open
            }
        };
        if wants_open && locked {
            return door_failure("locked", locked, barricaded);
        }
        if wants_open && barricaded {
            return door_failure("barricaded", locked, true);
        }
        door.set("locked", json!(locked));
        door.set("target_open", json!(wants_open));
        let state = if !changes_motion {
            door.get_string("state", if is_open { "open" } else { "closed" })
        } else if wants_open {
            "opening".to_string()
        } else if is_open {
            "closing".to_string()
        } else {
            "closed".to_string()
        };
        door.set("state", json!(state));
        if wants_open {
            door.set_f64(
                "auto_close_remaining",
                door.get_f64("auto_close_seconds", 0.0).max(0.0),
            );
        }
        let noise = door.get_f64("noise_radius", 3.5).max(0.0);
        if let Some(current) = target.get_component_mut("Door2D") {
            *current = door;
        }
        sync_door_collision(target);
        DoorActionResult {
            success: true,
            state,
            locked,
            blocked_by_barricade: false,
            emitted_noise_radius: noise,
            reason: "ok".to_string(),
        }
    }

    pub fn tick_door(target: &mut GameObject, dt: f64) -> bool {
        let Some(mut door) = target.get_component("Door2D").cloned() else {
            return false;
        };
        let dt = finite_or(dt, 0.0).clamp(0.0, 0.25);
        let target_open = door.get_bool("target_open", false);
        let speed = door.get_f64("open_speed", 4.0).max(0.01);
        let progress = door.get_f64("open_progress", 0.0);
        let next = if target_open {
            (progress + speed * dt).min(1.0)
        } else {
            (progress - speed * dt).max(0.0)
        };
        door.set_f64("open_progress", next);
        door.set(
            "state",
            json!(if next >= 1.0 {
                "open"
            } else if next <= 0.0 {
                "closed"
            } else if target_open {
                "opening"
            } else {
                "closing"
            }),
        );
        if target_open && next >= 1.0 {
            let remaining = door.get_f64("auto_close_remaining", 0.0);
            if remaining > 0.0 {
                let remaining = (remaining - dt).max(0.0);
                door.set_f64("auto_close_remaining", remaining);
                if remaining <= 0.0 {
                    door.set("target_open", json!(false));
                    door.set("state", json!("closing"));
                }
            }
        }
        if let Some(current) = target.get_component_mut("Door2D") {
            *current = door;
        }
        sync_door_collision(target);
        true
    }

    pub fn add_barricade_layer(target: &mut GameObject) -> BarricadeResult {
        ensure_component(target, "Barricade2D");
        let Some(barricade) = target.get_component_mut("Barricade2D") else {
            return barricade_failure("component_missing");
        };
        let layers = barricade.get_i64("layers", 0);
        let max_layers = barricade.get_i64("max_layers", 4).max(0);
        if layers >= max_layers {
            return BarricadeResult {
                success: false,
                layers,
                health: barricade.get_f64("health", 0.0),
                destroyed: false,
                reason: "maximum_layers".to_string(),
            };
        }
        let layers = layers + 1;
        let per_layer = barricade.get_f64("health_per_layer", 40.0).max(0.0);
        let maximum = barricade
            .get_f64("max_health", per_layer * max_layers as f64)
            .max(0.0);
        let health = (barricade.get_f64("health", 0.0) + per_layer).min(maximum);
        barricade.set("layers", json!(layers));
        barricade.set_f64("health", health);
        sync_door_collision(target);
        BarricadeResult {
            success: true,
            layers,
            health,
            destroyed: false,
            reason: "layer_added".to_string(),
        }
    }

    pub fn damage_barricade(target: &mut GameObject, amount: f64) -> BarricadeResult {
        let Some(barricade) = target.get_component_mut("Barricade2D") else {
            return barricade_failure("component_missing");
        };
        let resistance = barricade.get_f64("damage_resistance", 0.1).clamp(0.0, 0.95);
        let damage = finite_or(amount, 0.0).max(0.0) * (1.0 - resistance);
        let health = (barricade.get_f64("health", 0.0) - damage).max(0.0);
        let per_layer = barricade.get_f64("health_per_layer", 40.0).max(0.001);
        let layers = if health <= 0.0 {
            0
        } else {
            (health / per_layer).ceil() as i64
        };
        barricade.set_f64("health", health);
        barricade.set("layers", json!(layers));
        let destroyed = health <= 0.0;
        sync_door_collision(target);
        BarricadeResult {
            success: true,
            layers,
            health,
            destroyed,
            reason: if destroyed { "destroyed" } else { "damaged" }.to_string(),
        }
    }
}

fn silent_noise(entity: &GameObject, kind: &str) -> NoiseEvent2D {
    NoiseEvent2D {
        source_id: entity.id,
        x: entity.x,
        y: entity.y,
        radius: 0.0,
        intensity: 0.0,
        kind: kind.to_string(),
        age: 0.0,
        duration: 0.0,
    }
}

fn ensure_component(entity: &mut GameObject, component_type: &str) {
    if entity.get_component(component_type).is_none()
        && let Some(component) = default_component(component_type)
    {
        entity.add_component(component);
    }
}

fn sync_door_collision(target: &mut GameObject) {
    let blocked = target
        .get_component("Barricade2D")
        .map(|value| value.get_i64("layers", 0) > 0 && value.get_f64("health", 0.0) > 0.0)
        .unwrap_or(false)
        || target
            .get_component("Door2D")
            .map(|door| {
                door.get_bool("collision_when_closed", true)
                    && door.get_f64("open_progress", 0.0) < 0.8
            })
            .unwrap_or(false);
    if let Some(collider) = target.get_component_mut("Collider2D") {
        collider.enabled = blocked;
    }
    if let Some(interaction) = target.get_component_mut("Interaction") {
        interaction.set(
            "prompt",
            json!(if blocked { "Open / Barricade" } else { "Close" }),
        );
    }
}

fn door_failure(reason: &str, locked: bool, barricaded: bool) -> DoorActionResult {
    DoorActionResult {
        success: false,
        state: "blocked".to_string(),
        locked,
        blocked_by_barricade: barricaded,
        emitted_noise_radius: 0.0,
        reason: reason.to_string(),
    }
}

fn barricade_failure(reason: &str) -> BarricadeResult {
    BarricadeResult {
        success: false,
        layers: 0,
        health: 0.0,
        destroyed: false,
        reason: reason.to_string(),
    }
}

fn normalize_degrees(value: f64) -> f64 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::component::default_component;

    fn actor(name: &str, tag: &str, x: f64, y: f64) -> GameObject {
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.tag = tag.to_string();
        entity
    }

    #[test]
    fn crouching_reduces_motion_visibility_and_noise() {
        let mut entity = actor("Player", "Player", 0.0, 0.0);
        assert!(SurvivalWorldSystems::set_crouching(&mut entity, true));
        let noise = SurvivalWorldSystems::emit_noise(&mut entity, "movement", 1.0);
        assert!((SurvivalWorldSystems::movement_multiplier(&entity) - 0.55).abs() < 0.001);
        assert!(SurvivalWorldSystems::visibility(&entity) < 1.0);
        assert!(noise.radius < 1.0);
    }

    #[test]
    fn senses_choose_visible_or_audible_target_and_build_alertness() {
        let mut observer = actor("Zombie", "Enemy", 0.0, 0.0);
        observer.add_component(default_component("Senses2D").unwrap());
        observer.rotation = 0.0;
        let mut player = actor("Player", "Player", 4.0, 0.0);
        let seen =
            SurvivalWorldSystems::update_perception(&mut observer, &[player.clone()], &[], 0.25);
        assert!(seen.detected);
        assert_eq!(seen.stimulus, "sight");
        player.x = -12.0;
        let noise = NoiseEvent2D {
            source_id: player.id,
            x: player.x,
            y: player.y,
            radius: 14.0,
            intensity: 1.0,
            kind: "combat".into(),
            age: 0.0,
            duration: 1.0,
        };
        let heard =
            SurvivalWorldSystems::update_perception(&mut observer, &[player], &[noise], 0.25);
        assert!(heard.detected);
        assert!(heard.stimulus.starts_with("hearing"));
        assert!(heard.alertness > seen.alertness);
    }

    #[test]
    fn barricade_blocks_door_until_destroyed() {
        let mut door = actor("Door", "Door", 0.0, 0.0);
        door.add_component(default_component("Door2D").unwrap());
        door.add_component(default_component("Barricade2D").unwrap());
        assert!(SurvivalWorldSystems::add_barricade_layer(&mut door).success);
        let blocked = SurvivalWorldSystems::door_action(&mut door, DoorCommand::Open);
        assert!(!blocked.success);
        assert!(blocked.blocked_by_barricade);
        let destroyed = SurvivalWorldSystems::damage_barricade(&mut door, 1000.0);
        assert!(destroyed.destroyed);
        assert!(SurvivalWorldSystems::door_action(&mut door, DoorCommand::Open).success);
        SurvivalWorldSystems::tick_door(&mut door, 0.25);
        assert_eq!(
            door.get_component("Door2D")
                .unwrap()
                .get_f64("open_progress", 0.0),
            1.0
        );
        assert!(!door.get_component("Collider2D").unwrap().enabled);
    }

    #[test]
    fn keyed_door_rejects_wrong_key() {
        let mut door = actor("Vault", "Door", 0.0, 0.0);
        let mut component = default_component("Door2D").unwrap();
        component.set("locked", json!(true));
        component.set("key_id", json!("vault_key"));
        door.add_component(component);
        assert!(
            !SurvivalWorldSystems::door_action(
                &mut door,
                DoorCommand::Unlock {
                    key_id: Some("house_key")
                }
            )
            .success
        );
        assert!(
            SurvivalWorldSystems::door_action(
                &mut door,
                DoorCommand::Unlock {
                    key_id: Some("vault_key")
                }
            )
            .success
        );
    }
}
