use serde_json::json;

use crate::engine::animation_graph::{AnimationGraphLibrary, AnimationSample};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn update_entities(
        &self,
        entities: &mut [GameObject],
        graphs: &AnimationGraphLibrary,
        dt: f64,
        mode: &str,
    ) {
        if mode != "PLAY" {
            return;
        }
        for entity in entities {
            let Some((sample, apply_tint, normalized)) = advance_animator(entity, graphs, dt)
            else {
                continue;
            };
            if let Some(sample) = sample {
                apply_sample(entity, &sample, apply_tint);
            } else if apply_tint {
                let value = (180.0 + normalized * 60.0) as i64;
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("tint", json!([value, 220, 255]));
                }
            }
        }
    }
}

fn advance_animator(
    entity: &mut GameObject,
    graphs: &AnimationGraphLibrary,
    dt: f64,
) -> Option<(Option<AnimationSample>, bool, f64)> {
    let animator = entity.get_component_mut("Animator")?;
    let controller_name = animator.get_string("controller", "Default");
    let mut current_state = animator.get_string("current_state", "Idle");
    let parameters = animator
        .get("parameters")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let apply_tint = animator.get_bool("apply_tint", true);
    let mut normalized = animator.get_f64("normalized_time", 0.0);
    normalized = (normalized + dt * animator.get_f64("speed", 1.0)) % 1.0;

    if let Some(controller) = graphs.controllers.get(&controller_name)
        && let Some(transition) =
            controller.resolve_transition(&current_state, normalized, &parameters)
    {
        current_state = transition.to.clone();
        animator.set("current_state", json!(current_state.clone()));
        normalized = 0.0;
    }

    animator.set_f64("normalized_time", normalized);
    let sample = graphs
        .controllers
        .get(&controller_name)
        .and_then(|controller| controller.clip_for_state(&current_state))
        .map(|clip| clip.sample(normalized * clip.duration.max(0.0001)));
    Some((sample, apply_tint, normalized))
}

fn apply_sample(entity: &mut GameObject, sample: &AnimationSample, apply_tint: bool) {
    if let Some(x) = sample.x {
        entity.x = x;
    }
    if let Some(y) = sample.y {
        entity.y = y;
    }
    if let Some(rotation) = sample.rotation {
        entity.rotation = rotation;
    }
    if let Some(scale_x) = sample.scale_x {
        entity.scale_x = scale_x;
    }
    if let Some(scale_y) = sample.scale_y {
        entity.scale_y = scale_y;
    }
    entity.sync_to_components();
    if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
        if let Some(sprite_name) = &sample.sprite_name {
            sprite.set("sprite_name", json!(sprite_name));
        }
        if apply_tint && let Some((r, g, b)) = sample.tint {
            sprite.set("tint", json!([r, g, b]));
        }
    }
}
