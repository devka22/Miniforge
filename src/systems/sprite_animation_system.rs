use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::json;

use crate::engine::miniforge_2d::paper2d::{SpriteAnimation2D, SpriteFrames2D};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone)]
struct CachedSpriteFrames {
    modified: Option<SystemTime>,
    asset: SpriteFrames2D,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpriteAnimationReport {
    pub animated_entities: usize,
    pub loaded_assets: usize,
    pub emitted_events: usize,
    pub errors: Vec<String>,
}

/// Runtime bridge between editable `.spriteframes` assets and SpriteRenderer.
#[derive(Debug, Clone, Default)]
pub struct SpriteAnimationSystem {
    project_root: PathBuf,
    cache: BTreeMap<PathBuf, CachedSpriteFrames>,
    pub last_report: SpriteAnimationReport,
}

impl SpriteAnimationSystem {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            cache: BTreeMap::new(),
            last_report: SpriteAnimationReport::default(),
        }
    }

    pub fn update_entities(&mut self, entities: &mut [GameObject], dt: f64, mode: &str) {
        self.last_report = SpriteAnimationReport::default();
        if mode != "PLAY" {
            return;
        }
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        for entity in entities {
            let Some(sprite) = entity.get_component("SpriteRenderer") else {
                continue;
            };
            if !sprite.enabled || !sprite.get_bool("use_2d_animation", false) {
                continue;
            }
            let frames_path = sprite.get_string("sprite_frames", "");
            let animation_name = sprite.get_string("active_animation", "default");
            if frames_path.is_empty() {
                continue;
            }
            let path = self.resolve(&frames_path);
            let Some(asset) = self.load(&path) else {
                continue;
            };
            let Some(animation) = asset
                .animations
                .iter()
                .find(|animation| animation.name == animation_name)
                .or_else(|| asset.animations.first())
                .cloned()
            else {
                self.last_report.errors.push(format!(
                    "{}: animation '{animation_name}' no existe en {}",
                    entity.name,
                    path.display()
                ));
                continue;
            };
            let texture = asset.texture.clone();
            let emitted = advance_sprite(entity, &animation, &texture, dt);
            self.last_report.animated_entities += 1;
            self.last_report.emitted_events += emitted;
        }
        self.last_report.loaded_assets = self.cache.len();
    }

    fn resolve(&self, value: &str) -> PathBuf {
        let path = Path::new(value);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        }
    }

    fn load(&mut self, path: &Path) -> Option<SpriteFrames2D> {
        let modified = fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok();
        if let Some(cached) = self
            .cache
            .get(path)
            .filter(|cached| cached.modified == modified)
        {
            return Some(cached.asset.clone());
        }
        let result = fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<SpriteFrames2D>(&bytes).map_err(|error| error.to_string())
            })
            .and_then(|asset| {
                asset
                    .validate()
                    .then_some(asset)
                    .ok_or_else(|| "asset SpriteFrames2D inválido".to_string())
            });
        match result {
            Ok(asset) => {
                self.cache.insert(
                    path.to_path_buf(),
                    CachedSpriteFrames {
                        modified,
                        asset: asset.clone(),
                    },
                );
                Some(asset)
            }
            Err(error) => {
                self.last_report
                    .errors
                    .push(format!("{}: {error}", path.display()));
                None
            }
        }
    }
}

fn advance_sprite(
    entity: &mut GameObject,
    animation: &SpriteAnimation2D,
    texture: &str,
    dt: f64,
) -> usize {
    if animation.frames.is_empty() {
        return 0;
    }
    let Some(sprite) = entity.get_component_mut("SpriteRenderer") else {
        return 0;
    };
    let mut frame = sprite
        .get_usize("_frame_index", 0)
        .min(animation.frames.len() - 1);
    let mut elapsed = sprite.get_f64("_frame_elapsed", 0.0) + dt;
    let mut direction = sprite.get_i64("_frame_direction", 1).signum();
    if direction == 0 {
        direction = 1;
    }
    let previous = frame;
    let mut guard = 0;
    while elapsed
        >= animation.frames[frame]
            .duration
            .max(1.0 / animation.fps.max(1.0))
        && guard < animation.frames.len().saturating_mul(2).max(1)
    {
        elapsed -= animation.frames[frame]
            .duration
            .max(1.0 / animation.fps.max(1.0));
        if animation.ping_pong && animation.frames.len() > 1 {
            if direction > 0 && frame + 1 >= animation.frames.len() {
                direction = -1;
            } else if direction < 0 && frame == 0 {
                direction = 1;
            }
            frame = (frame as i64 + direction).clamp(0, animation.frames.len() as i64 - 1) as usize;
        } else if frame + 1 < animation.frames.len() {
            frame += 1;
        } else if animation.looped {
            frame = 0;
        }
        guard += 1;
    }
    let current = &animation.frames[frame];
    sprite.set("_frame_index", json!(frame));
    sprite.set_f64("_frame_elapsed", elapsed);
    sprite.set("_frame_direction", json!(direction));
    sprite.set("_texture_path", json!(texture));
    sprite.set(
        "_source_rect",
        json!({
            "x": current.rect.x,
            "y": current.rect.y,
            "width": current.rect.width,
            "height": current.rect.height,
        }),
    );
    let events = if frame != previous {
        current.events.clone()
    } else {
        Vec::new()
    };
    sprite.set("_frame_events", json!(events));
    events.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_frame_runtime_advances_without_persisting_internal_state() {
        let mut entity = GameObject::new(0.0, 0.0, Some("Animated".to_string()));
        let sprite = entity
            .get_component_mut("SpriteRenderer")
            .expect("sprite renderer");
        sprite.set("use_2d_animation", json!(true));
        let animation = SpriteAnimation2D {
            name: "idle".to_string(),
            fps: 10.0,
            looped: true,
            ping_pong: false,
            frames: vec![
                crate::engine::miniforge_2d::paper2d::SpriteFrame2D {
                    rect: crate::engine::miniforge_2d::paper2d::SpriteRect2D {
                        x: 0,
                        y: 0,
                        width: 16,
                        height: 16,
                    },
                    duration: 0.1,
                    pivot: (8.0, 8.0),
                    hitboxes: Vec::new(),
                    hurtboxes: Vec::new(),
                    collision_shapes: Vec::new(),
                    events: Vec::new(),
                },
                crate::engine::miniforge_2d::paper2d::SpriteFrame2D {
                    rect: crate::engine::miniforge_2d::paper2d::SpriteRect2D {
                        x: 16,
                        y: 0,
                        width: 16,
                        height: 16,
                    },
                    duration: 0.1,
                    pivot: (8.0, 8.0),
                    hitboxes: Vec::new(),
                    hurtboxes: Vec::new(),
                    collision_shapes: Vec::new(),
                    events: Vec::new(),
                },
            ],
            tags: Vec::new(),
        };
        assert_eq!(advance_sprite(&mut entity, &animation, "hero.png", 0.11), 0);
        let sprite = entity.get_component("SpriteRenderer").expect("sprite");
        assert_eq!(sprite.get_usize("_frame_index", 0), 1);
        assert!(sprite.serialize().get("_frame_index").is_none());
    }
}
