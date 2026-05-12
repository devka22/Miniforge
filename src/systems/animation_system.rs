use serde_json::json;

use crate::engine::animation_graph::AnimationGraphLibrary;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn update_entities(
        &self,
        entities: &mut [GameObject],
        _graphs: &AnimationGraphLibrary,
        dt: f64,
        mode: &str,
    ) {
        if mode != "PLAY" {
            return;
        }
        for entity in entities {
            let Some(animator) = entity.get_component_mut("Animator") else {
                continue;
            };
            let mut normalized = animator.get_f64("normalized_time", 0.0);
            normalized = (normalized + dt * animator.get_f64("speed", 1.0)) % 1.0;
            animator.set_f64("normalized_time", normalized);
            if animator.get_bool("apply_tint", true) {
                let value = (180.0 + normalized * 60.0) as i64;
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("tint", json!([value, 220, 255]));
                }
            }
        }
    }
}
