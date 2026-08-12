use std::collections::BTreeMap;

use serde_json::json;

use crate::engine::camera::Camera;
use crate::engine::survival_world::SurvivalWorldSystems;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;

#[derive(Debug, Clone, Default)]
pub struct Runtime2DSystem {
    pub stats: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControllerMode {
    TopDown,
    Platformer,
}

impl Runtime2DSystem {
    pub fn update_entities(&mut self, entities: &mut [GameObject], dt: f64, mode: &str) {
        let mut character_controllers = 0;
        let mut dashes = 0;
        let mut checkpoint_activations = 0;
        let mut fall_respawns = 0;

        if mode == "PLAY" {
            let dt = if dt.is_finite() {
                dt.clamp(0.0, 0.05)
            } else {
                0.0
            };
            checkpoint_activations = activate_checkpoints(entities);
            for entity in entities.iter_mut().filter(|entity| entity.enabled) {
                if update_character_controller(entity, dt) {
                    character_controllers += 1;
                    if entity
                        .get_component("CharacterController2D")
                        .is_some_and(|controller| controller.get_bool("dashing", false))
                    {
                        dashes += 1;
                    }
                }
                if respawn_after_fall(entity) {
                    fall_respawns += 1;
                }
            }
        }

        self.stats = BTreeMap::from([
            ("character_controllers".to_string(), character_controllers),
            ("dashes".to_string(), dashes),
            ("checkpoint_activations".to_string(), checkpoint_activations),
            ("fall_respawns".to_string(), fall_respawns),
        ]);
    }

    pub fn resolve_tilemap_collisions(
        &mut self,
        entities: &mut [GameObject],
        grid: &Grid,
        mode: &str,
    ) {
        if mode != "PLAY" {
            self.stats.insert("tile_collisions".to_string(), 0);
            return;
        }

        let mut resolved = 0;
        for entity in entities.iter_mut().filter(|entity| {
            entity.enabled && entity.get_component("CharacterController2D").is_some()
        }) {
            if resolve_entity_against_grid(entity, grid) {
                resolved += 1;
            }
        }
        self.stats.insert("tile_collisions".to_string(), resolved);
    }

    pub fn update_camera(
        &mut self,
        camera: &mut Camera,
        entities: &[GameObject],
        dt: f64,
        mode: &str,
        tile_size: f64,
    ) {
        if mode != "PLAY" {
            self.stats.insert("camera_follows".to_string(), 0);
            return;
        }

        let Some((follower, target)) = camera_follow_pair(entities) else {
            self.stats.insert("camera_follows".to_string(), 0);
            return;
        };
        let Some(follow) = follower.get_component("CameraFollow") else {
            self.stats.insert("camera_follows".to_string(), 0);
            return;
        };

        let viewport_w = follow.get_f64("viewport_width", 960.0).max(1.0);
        let viewport_h = follow.get_f64("viewport_height", 540.0).max(1.0);
        let desired_zoom = follow.get_f64("zoom", camera.zoom).clamp(0.1, 6.0);
        camera.set_zoom(lerp(
            camera.zoom,
            desired_zoom,
            smoothing_alpha(follow.get_f64("zoom_smoothness", 10.0), dt),
        ));

        let target_x = (target.x + follow.get_f64("offset_x", 0.0)) * tile_size
            - viewport_w * 0.5 / camera.zoom.max(0.1);
        let target_y = (target.y + follow.get_f64("offset_y", 0.0)) * tile_size
            - viewport_h * 0.5 / camera.zoom.max(0.1);
        let dead_zone = follow.get_f64("dead_zone", 0.0).max(0.0) * tile_size;
        let alpha = smoothing_alpha(follow.get_f64("smoothness", 8.0), dt);

        if follow.get_bool("follow_x", true) && (camera.x - target_x).abs() > dead_zone {
            camera.x = lerp(camera.x, target_x, alpha);
        }
        if follow.get_bool("follow_y", true) && (camera.y - target_y).abs() > dead_zone {
            camera.y = lerp(camera.y, target_y, alpha);
        }

        let shake = camera_shake_offset(entities, dt);
        camera.x += shake.0;
        camera.y += shake.1;
        camera.clamp_to_bounds();
        self.stats.insert("camera_follows".to_string(), 1);
    }

    pub fn advance_camera_shakes(&mut self, entities: &mut [GameObject], dt: f64, mode: &str) {
        if mode != "PLAY" {
            self.stats.insert("camera_shakes".to_string(), 0);
            return;
        }
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        let mut active = 0;
        for entity in entities {
            let Some(shake) = entity.get_component_mut("CameraShake") else {
                continue;
            };
            if !shake.get_bool("active", false) {
                continue;
            }
            let duration = shake.get_f64("duration", 0.25).max(0.0001);
            let elapsed = shake.get_f64("elapsed", 0.0) + dt;
            let trauma = (1.0 - elapsed / duration).clamp(0.0, 1.0);
            shake.set_f64("elapsed", elapsed);
            shake.set_f64("trauma", trauma);
            if elapsed >= duration {
                shake.set("active", json!(false));
            } else {
                active += 1;
            }
        }
        self.stats.insert("camera_shakes".to_string(), active);
    }
}

fn update_character_controller(entity: &mut GameObject, dt: f64) -> bool {
    let Some(controller) = entity.get_component("CharacterController2D") else {
        return false;
    };
    if !controller.enabled || !controller.get_bool("input_enabled", true) {
        return false;
    }

    let input_x = controller.get_f64("input_x", 0.0).clamp(-1.0, 1.0);
    let input_y = controller.get_f64("input_y", 0.0).clamp(-1.0, 1.0);
    let jump_pressed = controller.get_bool("jump_pressed", false);
    let jump_held = controller.get_bool("jump_held", jump_pressed);
    let run_pressed = controller.get_bool("run_pressed", false);
    let dash_pressed = controller.get_bool("dash_pressed", false);
    let body_uses_gravity = entity
        .get_component("Rigidbody2D")
        .map(|body| body.get_bool("use_gravity", false))
        .unwrap_or(false);
    let controller_mode = controller_mode(controller.get_string("mode", ""), body_uses_gravity);
    let mut grounded = controller.get_bool("grounded", false);
    let mut coyote_timer = controller.get_f64("coyote_timer", 0.0);
    let coyote_time = controller.get_f64("coyote_time", 0.12).max(0.0);
    let mut jump_buffer_timer = if jump_pressed {
        controller.get_f64("jump_buffer_time", 0.12).max(0.0)
    } else {
        (controller.get_f64("jump_buffer_timer", 0.0) - dt).max(0.0)
    };
    let mut jumps_used = controller.get_i64("jumps_used", 0).max(0);
    let max_jumps = controller.get_i64("max_jumps", 1).max(0);
    let walk_speed = controller.get_f64("walk_speed", 5.0).max(0.0);
    let run_speed = controller.get_f64("run_speed", walk_speed).max(walk_speed);
    let stealth_multiplier = entity
        .get_component("StealthState2D")
        .map(|stealth| stealth.get_f64("movement_multiplier", 1.0))
        .unwrap_or(1.0)
        .clamp(0.05, 3.0);
    let speed = (if run_pressed { run_speed } else { walk_speed }) * stealth_multiplier;
    let mut facing_x = controller.get_f64("facing_x", 1.0);
    let mut facing_y = controller.get_f64("facing_y", 0.0);
    let has_input = input_x.abs() + input_y.abs() > 0.01;
    if has_input {
        facing_x = input_x;
        facing_y = input_y;
    }

    let (mut vx, mut vy) = entity
        .get_component("Rigidbody2D")
        .map(|body| {
            (
                body.get_f64("velocity_x", 0.0),
                body.get_f64("velocity_y", 0.0),
            )
        })
        .unwrap_or((0.0, 0.0));

    let mut dash_timer = (controller.get_f64("dash_timer", 0.0) - dt).max(0.0);
    let mut dash_cooldown_timer = (controller.get_f64("dash_cooldown_timer", 0.0) - dt).max(0.0);
    let dash_duration = controller.get_f64("dash_duration", 0.12).max(0.0);
    let dash_cooldown = controller.get_f64("dash_cooldown", 0.45).max(0.0);
    let dash_speed = controller.get_f64("dash_speed", speed * 2.0).max(0.0);
    if dash_pressed && dash_cooldown_timer <= f64::EPSILON && dash_duration > 0.0 {
        dash_timer = dash_duration;
        dash_cooldown_timer = dash_cooldown;
    }

    match controller_mode {
        ControllerMode::TopDown => {
            let direction = normalize_or_zero((input_x, input_y));
            vx = direction.0 * speed;
            vy = direction.1 * speed;
        }
        ControllerMode::Platformer => {
            let target_vx = input_x * speed;
            if grounded {
                vx = target_vx;
                coyote_timer = coyote_time;
                jumps_used = 0;
            } else {
                coyote_timer = (coyote_timer - dt).max(0.0);
                let air_control = controller.get_f64("air_control", 0.6).clamp(0.0, 1.0);
                vx = lerp(vx, target_vx, air_control);
            }

            let can_ground_jump = grounded || coyote_timer > 0.0;
            if jump_buffer_timer > 0.0 && (can_ground_jump || jumps_used < max_jumps) {
                vy = -controller.get_f64("jump_force", 9.0).max(0.0);
                grounded = false;
                coyote_timer = 0.0;
                jumps_used = if can_ground_jump { 1 } else { jumps_used + 1 };
                jump_buffer_timer = 0.0;
            } else if !jump_held && vy < 0.0 {
                vy *= controller
                    .get_f64("jump_cut_multiplier", 0.55)
                    .clamp(0.0, 1.0);
            }
        }
    }

    if dash_timer > 0.0 {
        let dash_dir = normalize_or_zero(if has_input {
            (input_x, input_y)
        } else {
            (facing_x, facing_y)
        });
        vx = dash_dir.0 * dash_speed;
        if controller_mode == ControllerMode::TopDown || dash_dir.1.abs() > 0.01 {
            vy = dash_dir.1 * dash_speed;
        }
    }

    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
        body.set_f64("velocity_x", vx);
        body.set_f64("velocity_y", vy);
        body.set("sleeping", json!(false));
    } else {
        entity.x += vx * dt;
        entity.y += vy * dt;
    }

    if let Some(controller) = entity.get_component_mut("CharacterController2D") {
        controller.set(
            "mode",
            json!(match controller_mode {
                ControllerMode::TopDown => "topdown",
                ControllerMode::Platformer => "platformer",
            }),
        );
        controller.set("grounded", json!(grounded));
        controller.set_f64("coyote_timer", coyote_timer);
        controller.set_f64("jump_buffer_timer", jump_buffer_timer);
        controller.set("jumps_used", json!(jumps_used));
        controller.set_f64("dash_timer", dash_timer);
        controller.set_f64("dash_cooldown_timer", dash_cooldown_timer);
        controller.set("dashing", json!(dash_timer > 0.0));
        controller.set_f64("facing_x", facing_x);
        controller.set_f64("facing_y", facing_y);
        controller.set("moving", json!(has_input));
        controller.set("jump_pressed", json!(false));
        controller.set("dash_pressed", json!(false));
    }

    if let Some(blackboard) = entity.get_component_mut("Blackboard") {
        blackboard.blackboard_set("moving", json!(has_input));
        blackboard.blackboard_set("grounded", json!(grounded));
        blackboard.blackboard_set("dashing", json!(dash_timer > 0.0));
    }
    if has_input && entity.get_component("NoiseEmitter2D").is_some() {
        let crouching = entity
            .get_component("StealthState2D")
            .is_some_and(|stealth| stealth.get_bool("crouching", false));
        let kind = if crouching {
            "crouch"
        } else if run_pressed {
            "sprint"
        } else {
            "movement"
        };
        let _ = SurvivalWorldSystems::emit_noise(entity, kind, 1.0);
    }
    entity.sync_to_components();
    true
}

fn resolve_entity_against_grid(entity: &mut GameObject, grid: &Grid) -> bool {
    if grid.width == 0 || grid.height == 0 {
        return false;
    }
    let (mut vx, mut vy) = entity
        .get_component("Rigidbody2D")
        .map(|body| {
            (
                body.get_f64("velocity_x", 0.0),
                body.get_f64("velocity_y", 0.0),
            )
        })
        .unwrap_or((0.0, 0.0));
    let half_w = (entity.width * entity.scale_x.abs()).max(0.1) * 0.5;
    let half_h = (entity.height * entity.scale_y.abs()).max(0.1) * 0.5;
    let mut resolved = false;
    let mut grounded = false;

    if vy >= 0.0 {
        let bottom = entity.y + half_h;
        let tile_y = bottom.floor() as i32;
        if aabb_overlaps_solid_row(grid, entity.x - half_w, entity.x + half_w, tile_y)
            && entity.y - half_h < tile_y as f64
        {
            entity.y = tile_y as f64 - half_h - 0.0001;
            vy = 0.0;
            grounded = true;
            resolved = true;
        }
    } else {
        let top = entity.y - half_h;
        let tile_y = top.floor() as i32;
        if aabb_overlaps_solid_row(grid, entity.x - half_w, entity.x + half_w, tile_y)
            && entity.y + half_h > (tile_y + 1) as f64
        {
            entity.y = (tile_y + 1) as f64 + half_h + 0.0001;
            vy = 0.0;
            resolved = true;
        }
    }

    if vx > 0.0 {
        let right = entity.x + half_w;
        let tile_x = right.floor() as i32;
        if aabb_overlaps_solid_column(grid, tile_x, entity.y - half_h, entity.y + half_h) {
            entity.x = tile_x as f64 - half_w - 0.0001;
            vx = 0.0;
            resolved = true;
        }
    } else if vx < 0.0 {
        let left = entity.x - half_w;
        let tile_x = left.floor() as i32;
        if aabb_overlaps_solid_column(grid, tile_x, entity.y - half_h, entity.y + half_h) {
            entity.x = (tile_x + 1) as f64 + half_w + 0.0001;
            vx = 0.0;
            resolved = true;
        }
    }

    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
        body.set_f64("velocity_x", vx);
        body.set_f64("velocity_y", vy);
    }
    if let Some(controller) = entity.get_component_mut("CharacterController2D") {
        controller.set("grounded", json!(grounded));
        if grounded {
            controller.set("jumps_used", json!(0));
            controller.set_f64("coyote_timer", controller.get_f64("coyote_time", 0.12));
        }
    }
    if resolved {
        entity.sync_to_components();
    }
    resolved
}

fn activate_checkpoints(entities: &mut [GameObject]) -> usize {
    let snapshot = entities.to_vec();
    let mut activations = Vec::new();
    for checkpoint in &snapshot {
        let Some(component) = checkpoint.get_component("Checkpoint") else {
            continue;
        };
        if !component.enabled {
            continue;
        }
        if component.get_bool("single_use", false) && component.get_bool("active", false) {
            continue;
        }
        let tag = component.get_string("activated_by_tag", "Player");
        let radius = component.get_f64("activation_radius", 1.2).max(0.0);
        for candidate in snapshot.iter().filter(|entity| entity.id != checkpoint.id) {
            if candidate.tag != tag || candidate.get_component("CharacterController2D").is_none() {
                continue;
            }
            let dx = candidate.x - checkpoint.x;
            let dy = candidate.y - checkpoint.y;
            if (dx * dx + dy * dy).sqrt() <= radius {
                activations.push((
                    checkpoint.id,
                    candidate.id,
                    component.get_string("checkpoint_id", "checkpoint"),
                    component.get_f64("respawn_x", checkpoint.x),
                    component.get_f64("respawn_y", checkpoint.y),
                ));
            }
        }
    }

    let mut applied = 0;
    for (checkpoint_id, player_id, name, respawn_x, respawn_y) in activations {
        if let Some(checkpoint) = entities
            .iter_mut()
            .find(|entity| entity.id == checkpoint_id)
            && let Some(component) = checkpoint.get_component_mut("Checkpoint")
        {
            component.set("active", json!(true));
            component.set("checkpoint_id", json!(name));
            component.set_f64("respawn_x", respawn_x);
            component.set_f64("respawn_y", respawn_y);
        }
        if let Some(player) = entities.iter_mut().find(|entity| entity.id == player_id)
            && let Some(component) = player.get_component_mut("Checkpoint")
        {
            component.set("active", json!(true));
            component.set("checkpoint_id", json!(name));
            component.set_f64("respawn_x", respawn_x);
            component.set_f64("respawn_y", respawn_y);
            applied += 1;
        }
    }
    applied
}

fn respawn_after_fall(entity: &mut GameObject) -> bool {
    let Some(controller) = entity.get_component("CharacterController2D") else {
        return false;
    };
    let kill_y = controller.get_f64("fall_death_y", 9999.0);
    if entity.y <= kill_y {
        return false;
    }
    respawn_entity(entity)
}

pub fn respawn_entity(entity: &mut GameObject) -> bool {
    let Some(checkpoint) = entity.get_component("Checkpoint").cloned() else {
        return false;
    };
    if !checkpoint.get_bool("active", false) {
        return false;
    }
    entity.x = checkpoint.get_f64("respawn_x", entity.x);
    entity.y = checkpoint.get_f64("respawn_y", entity.y);
    entity.path.clear();
    entity.command = "RESPAWN".to_string();
    entity.state = "IDLE".to_string();
    if let Some(health) = entity.get_component_mut("Health") {
        let max_health = health.get_f64("max_health", 100.0);
        let health_after_respawn = checkpoint
            .get_f64("respawn_health", max_health)
            .clamp(1.0, max_health);
        health.set_f64("health", health_after_respawn);
        health.set("alive", json!(true));
    }
    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
        body.set_f64("velocity_x", 0.0);
        body.set_f64("velocity_y", 0.0);
        body.set_f64("angular_velocity", 0.0);
        body.set("sleeping", json!(false));
    }
    if let Some(controller) = entity.get_component_mut("CharacterController2D") {
        controller.set("grounded", json!(false));
        controller.set("jumps_used", json!(0));
        controller.set_f64("dash_timer", 0.0);
    }
    entity.sync_to_components();
    true
}

fn camera_follow_pair(entities: &[GameObject]) -> Option<(&GameObject, &GameObject)> {
    let follower = entities
        .iter()
        .find(|entity| entity.enabled && entity.get_component("CameraFollow").is_some())?;
    let follow = follower.get_component("CameraFollow")?;
    if let Some(target_id) = follow.get("target_id").and_then(serde_json::Value::as_u64)
        && let Some(target) = entities.iter().find(|entity| entity.id == target_id)
    {
        return Some((follower, target));
    }
    Some((follower, follower))
}

fn camera_shake_offset(entities: &[GameObject], dt: f64) -> (f64, f64) {
    let Some(shake) = entities
        .iter()
        .filter_map(|entity| entity.get_component("CameraShake"))
        .find(|shake| shake.get_bool("active", false))
    else {
        return (0.0, 0.0);
    };
    let trauma = shake.get_f64("trauma", 0.0).clamp(0.0, 1.0);
    let amplitude = shake.get_f64("amplitude", 6.0) * trauma * trauma;
    let frequency = shake.get_f64("frequency", 24.0);
    let elapsed = shake.get_f64("elapsed", 0.0) + dt;
    (
        (elapsed * frequency).sin() * amplitude,
        (elapsed * frequency * 1.37).cos() * amplitude,
    )
}

fn controller_mode(value: String, body_uses_gravity: bool) -> ControllerMode {
    match value.as_str() {
        "topdown" | "top_down" => ControllerMode::TopDown,
        "platformer" | "side_scroller" | "sidescroller" => ControllerMode::Platformer,
        _ if body_uses_gravity => ControllerMode::Platformer,
        _ => ControllerMode::TopDown,
    }
}

fn aabb_overlaps_solid_row(grid: &Grid, left: f64, right: f64, y: i32) -> bool {
    let start_x = left.floor() as i32;
    let end_x = (right - 0.0001).floor() as i32;
    (start_x..=end_x).any(|x| solid(grid, x, y))
}

fn aabb_overlaps_solid_column(grid: &Grid, x: i32, top: f64, bottom: f64) -> bool {
    let start_y = top.floor() as i32;
    let end_y = (bottom - 0.0001).floor() as i32;
    (start_y..=end_y).any(|y| solid(grid, x, y))
}

fn solid(grid: &Grid, x: i32, y: i32) -> bool {
    !grid.in_bounds(x, y)
        || grid
            .get_tile(x as usize, y as usize)
            .is_some_and(|tile| tile != 0)
}

fn normalize_or_zero(value: (f64, f64)) -> (f64, f64) {
    let length = (value.0 * value.0 + value.1 * value.1).sqrt();
    if length <= f64::EPSILON {
        (0.0, 0.0)
    } else {
        (value.0 / length, value.1 / length)
    }
}

fn lerp(from: f64, to: f64, alpha: f64) -> f64 {
    from + (to - from) * alpha.clamp(0.0, 1.0)
}

fn smoothing_alpha(smoothness: f64, dt: f64) -> f64 {
    1.0 - (-smoothness.max(0.0) * dt.max(0.0)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::component::default_component;

    #[test]
    fn character_controller_applies_native_crouch_speed_and_noise() {
        let mut actor = GameObject::new(0.0, 0.0, Some("StealthActor".to_string()));
        actor.add_component(default_component("CharacterController2D").unwrap());
        actor.add_component(default_component("StealthState2D").unwrap());
        actor.add_component(default_component("NoiseEmitter2D").unwrap());
        assert!(SurvivalWorldSystems::set_crouching(&mut actor, true));
        let controller = actor.get_component_mut("CharacterController2D").unwrap();
        controller.set_f64("input_x", 1.0);
        controller.set_f64("input_y", 0.0);

        assert!(update_character_controller(&mut actor, 0.1));

        assert!((actor.x - 0.275).abs() < 0.001);
        let emitter = actor.get_component("NoiseEmitter2D").unwrap();
        assert_eq!(emitter.get_string("last_kind", ""), "crouch");
        assert!(emitter.get_f64("current_radius", 0.0) < 0.75);
    }
}
