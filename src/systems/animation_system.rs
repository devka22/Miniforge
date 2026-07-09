use serde_json::json;

use crate::engine::animation_graph::{AnimationGraphLibrary, AnimationSample};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct AnimationSystem;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnimationFrameReport {
    pub animated: usize,
    pub paused: usize,
    pub events_emitted: usize,
    pub transitions: usize,
}

impl AnimationSystem {
    pub fn update_entities(
        &self,
        entities: &mut [GameObject],
        graphs: &AnimationGraphLibrary,
        dt: f64,
        mode: &str,
    ) {
        let _ = self.update_entities_with_report(entities, graphs, dt, mode);
    }

    pub fn update_entities_with_report(
        &self,
        entities: &mut [GameObject],
        graphs: &AnimationGraphLibrary,
        dt: f64,
        mode: &str,
    ) -> AnimationFrameReport {
        let mut report = AnimationFrameReport::default();
        if mode != "PLAY" {
            return report;
        }
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        for entity in entities {
            let Some(advance) = advance_animator(entity, graphs, dt) else {
                continue;
            };
            if advance.paused {
                report.paused += 1;
                continue;
            }
            report.animated += 1;
            report.events_emitted += advance.events;
            report.transitions += usize::from(advance.transitioned);
            if let Some(sample) = advance.sample {
                apply_sample(entity, &sample, advance.apply_tint);
            } else if advance.apply_tint {
                let value = (180.0 + advance.normalized * 60.0) as i64;
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("tint", json!([value, 220, 255]));
                }
            }
        }
        report
    }
}

struct AnimatorAdvance {
    sample: Option<AnimationSample>,
    apply_tint: bool,
    normalized: f64,
    paused: bool,
    events: usize,
    transitioned: bool,
}

fn advance_animator(
    entity: &mut GameObject,
    graphs: &AnimationGraphLibrary,
    dt: f64,
) -> Option<AnimatorAdvance> {
    let animator = entity.get_component_mut("Animator")?;
    if !animator.enabled {
        return None;
    }
    if animator.get_bool("paused", false) {
        return Some(AnimatorAdvance {
            sample: None,
            apply_tint: false,
            normalized: animator.get_f64("normalized_time", 0.0),
            paused: true,
            events: 0,
            transitioned: false,
        });
    }
    let controller_name = animator.get_string("controller", "Default");
    let Some(controller) = graphs.controllers.get(&controller_name) else {
        return Some(AnimatorAdvance {
            sample: None,
            apply_tint: animator.get_bool("apply_tint", true),
            normalized: animator.get_f64("normalized_time", 0.0),
            paused: false,
            events: 0,
            transitioned: false,
        });
    };
    let mut current_state = animator.get_string("current_state", &controller.default_state);
    if !controller.states.contains_key(&current_state) {
        current_state = controller.default_state.clone();
        animator.set("current_state", json!(current_state.clone()));
    }
    let parameters = animator
        .get("parameters")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let apply_tint = animator.get_bool("apply_tint", true);
    let previous_normalized = animator.get_f64("normalized_time", 0.0).clamp(0.0, 1.0);
    let state = controller.state(&current_state)?;
    let clip = controller.clip_for_state(&current_state)?;
    let effective_speed = animator.get_f64("speed", 1.0) * state.speed;
    let delta_normalized = dt * effective_speed / clip.duration.max(0.0001);
    let raw_normalized = previous_normalized + delta_normalized;
    let mut normalized = if state.looped {
        raw_normalized.rem_euclid(1.0)
    } else {
        raw_normalized.clamp(0.0, 1.0)
    };
    let mut emitted = crossed_events(clip, previous_normalized, normalized, state.looped);
    let mut transitioned = false;

    if let Some(transition) = controller.resolve_transition(&current_state, normalized, &parameters)
    {
        current_state = transition.to.clone();
        animator.set("current_state", json!(current_state.clone()));
        normalized = 0.0;
        emitted.clear();
        transitioned = true;
    }

    animator.set_f64("normalized_time", normalized);
    animator.set("_finished", json!(!state.looped && raw_normalized >= 1.0));
    animator.set("_events", json!(emitted));
    let sample = controller
        .clip_for_state(&current_state)
        .map(|clip| clip.sample(normalized * clip.duration.max(0.0001)));
    Some(AnimatorAdvance {
        sample,
        apply_tint,
        normalized,
        paused: false,
        events: emitted.len(),
        transitioned,
    })
}

fn crossed_events(
    clip: &crate::engine::animation_graph::AnimationClip,
    from: f64,
    to: f64,
    looped: bool,
) -> Vec<serde_json::Value> {
    clip.events
        .iter()
        .filter(|event| {
            let at = (event.time / clip.duration.max(0.0001)).clamp(0.0, 1.0);
            if looped && to < from {
                at > from || at <= to
            } else {
                at > from && at <= to
            }
        })
        .map(|event| json!({"name": event.name, "payload": event.payload, "time": event.time}))
        .collect()
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
