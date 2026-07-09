use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sprite2DAsset {
    pub guid: String,
    pub name: String,
    pub texture: String,
    pub pixels_per_unit: f64,
    pub pivot: (f64, f64),
    pub rect: SpriteRect2D,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAtlas2D {
    pub guid: String,
    pub name: String,
    pub texture: String,
    #[serde(default)]
    pub sprites: Vec<Sprite2DAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteFrames2D {
    pub name: String,
    pub texture: String,
    #[serde(default)]
    pub animations: Vec<SpriteAnimation2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAnimation2D {
    pub name: String,
    pub fps: f64,
    pub looped: bool,
    pub ping_pong: bool,
    #[serde(default)]
    pub frames: Vec<SpriteFrame2D>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteFrame2D {
    pub rect: SpriteRect2D,
    pub duration: f64,
    pub pivot: (f64, f64),
    #[serde(default)]
    pub hitboxes: Vec<Hitbox2D>,
    #[serde(default)]
    pub hurtboxes: Vec<Hitbox2D>,
    #[serde(default)]
    pub collision_shapes: Vec<CollisionShape2D>,
    #[serde(default)]
    pub events: Vec<FrameEvent2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hitbox2D {
    pub name: String,
    pub rect: SpriteRect2D,
    #[serde(default)]
    pub damage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollisionShape2D {
    pub name: String,
    pub shape: String,
    #[serde(default)]
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SpriteRect2D {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlipbookAnimation2D {
    pub name: String,
    pub frames_per_second: f64,
    pub looping: bool,
    pub frames: Vec<FlipbookFrame2D>,
    #[serde(default)]
    pub frame_events: Vec<FrameEvent2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlipbookFrame2D {
    pub sprite: String,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameEvent2D {
    pub frame: usize,
    pub event: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tileset2D {
    pub guid: String,
    pub name: String,
    pub texture: String,
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    #[serde(default)]
    pub collision_tiles: Vec<CollisionTile2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollisionTile2D {
    pub tile_id: u32,
    pub shape: String,
    pub layer: String,
    pub is_trigger: bool,
    #[serde(default)]
    pub one_way: bool,
    #[serde(default)]
    pub damage_per_second: f64,
    #[serde(default)]
    pub water: bool,
    #[serde(default)]
    pub ladder: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tilemap2D {
    pub name: String,
    pub tileset: String,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    #[serde(default)]
    pub chunk_width: u32,
    #[serde(default)]
    pub chunk_height: u32,
    #[serde(default)]
    pub layers: Vec<TilemapLayer2D>,
    #[serde(default)]
    pub autotiles: Vec<AutotileRule2D>,
    #[serde(default)]
    pub animated_tiles: Vec<AnimatedTile2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TilemapLayer2D {
    pub name: String,
    pub visible: bool,
    pub collision: bool,
    pub tiles: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutotileRule2D {
    pub name: String,
    pub source_tile: u32,
    pub variants: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimatedTile2D {
    pub tile_id: u32,
    pub frames: Vec<u32>,
    pub fps: f64,
}

impl FlipbookAnimation2D {
    pub fn sprite_at_time(&self, time: f64) -> Option<&str> {
        if self.frames.is_empty() {
            return None;
        }
        let total = self
            .frames
            .iter()
            .map(|frame| frame.duration.max(0.0))
            .sum::<f64>()
            .max(0.0001);
        let mut local = if self.looping {
            time.rem_euclid(total)
        } else {
            time.clamp(0.0, total)
        };
        for frame in &self.frames {
            let duration = frame.duration.max(0.0);
            if local <= duration {
                return Some(&frame.sprite);
            }
            local -= duration;
        }
        self.frames.last().map(|frame| frame.sprite.as_str())
    }
}

impl SpriteFrames2D {
    pub fn validate(&self) -> bool {
        !self.name.trim().is_empty()
            && !self.texture.trim().is_empty()
            && self.animations.iter().all(|animation| {
                !animation.name.trim().is_empty()
                    && animation.fps > 0.0
                    && !animation.frames.is_empty()
                    && animation.frames.iter().all(|frame| frame.duration > 0.0)
            })
    }

    pub fn animation_names(&self) -> Vec<&str> {
        self.animations
            .iter()
            .map(|animation| animation.name.as_str())
            .collect()
    }

    pub fn animation_index(&self, name: &str) -> Option<usize> {
        self.animations
            .iter()
            .position(|animation| animation.name == name)
    }

    pub fn duplicate_animation(&mut self, source_name: &str, new_name: &str) -> bool {
        let new_name = new_name.trim();
        if new_name.is_empty() || self.animation_index(new_name).is_some() {
            return false;
        }
        let Some(source) = self
            .animations
            .iter()
            .find(|animation| animation.name == source_name)
            .cloned()
        else {
            return false;
        };
        let mut clone = source;
        clone.name = new_name.to_string();
        self.animations.push(clone);
        true
    }

    pub fn move_frame(&mut self, animation_name: &str, from: usize, to: usize) -> bool {
        let Some(animation) = self
            .animations
            .iter_mut()
            .find(|animation| animation.name == animation_name)
        else {
            return false;
        };
        if from >= animation.frames.len() || to >= animation.frames.len() {
            return false;
        }
        if from == to {
            return true;
        }
        let frame = animation.frames.remove(from);
        animation.frames.insert(to, frame);
        true
    }

    pub fn set_frame_duration(
        &mut self,
        animation_name: &str,
        frame_indices: &[usize],
        duration: f64,
    ) -> usize {
        if duration <= 0.0 {
            return 0;
        }
        let Some(animation) = self
            .animations
            .iter_mut()
            .find(|animation| animation.name == animation_name)
        else {
            return 0;
        };
        let mut changed = 0;
        let unique_indices = frame_indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        for index in unique_indices {
            if let Some(frame) = animation.frames.get_mut(index) {
                frame.duration = duration;
                changed += 1;
            }
        }
        changed
    }

    pub fn toggle_loop(&mut self, animation_name: &str) -> Option<bool> {
        let animation = self
            .animations
            .iter_mut()
            .find(|animation| animation.name == animation_name)?;
        animation.looped = !animation.looped;
        Some(animation.looped)
    }

    pub fn frame_at_time(&self, animation_name: &str, time: f64) -> Option<&SpriteFrame2D> {
        let animation = self
            .animations
            .iter()
            .find(|animation| animation.name == animation_name)?;
        if animation.frames.is_empty() {
            return None;
        }
        let sequence = playback_sequence(animation.frames.len(), animation.ping_pong);
        let total = sequence
            .iter()
            .filter_map(|index| animation.frames.get(*index))
            .map(|frame| frame.duration.max(0.0))
            .sum::<f64>()
            .max(0.0001);
        let mut local = if animation.looped {
            time.rem_euclid(total)
        } else {
            time.clamp(0.0, total)
        };
        for index in sequence {
            let frame = animation.frames.get(index)?;
            let duration = frame.duration.max(0.0);
            if local <= duration {
                return Some(frame);
            }
            local -= duration;
        }
        animation.frames.last()
    }

    pub fn grid_slice(
        name: impl Into<String>,
        texture: impl Into<String>,
        columns: u32,
        rows: u32,
        frame_width: u32,
        frame_height: u32,
        fps: f64,
    ) -> Self {
        let mut frames = Vec::new();
        for row in 0..rows {
            for column in 0..columns {
                frames.push(SpriteFrame2D {
                    rect: SpriteRect2D {
                        x: column * frame_width,
                        y: row * frame_height,
                        width: frame_width,
                        height: frame_height,
                    },
                    duration: 1.0 / fps.max(1.0),
                    pivot: (frame_width as f64 * 0.5, frame_height as f64 * 0.5),
                    hitboxes: Vec::new(),
                    hurtboxes: Vec::new(),
                    collision_shapes: Vec::new(),
                    events: Vec::new(),
                });
            }
        }
        Self {
            name: name.into(),
            texture: texture.into(),
            animations: vec![SpriteAnimation2D {
                name: "default".to_string(),
                fps,
                looped: true,
                ping_pong: false,
                frames,
                tags: Vec::new(),
            }],
        }
    }
}

fn playback_sequence(frame_count: usize, ping_pong: bool) -> Vec<usize> {
    let mut sequence = (0..frame_count).collect::<Vec<_>>();
    if ping_pong && frame_count > 2 {
        sequence.extend((1..frame_count - 1).rev());
    }
    sequence
}

impl Tilemap2D {
    pub fn tile_at(&self, layer_name: &str, x: u32, y: u32) -> Option<u32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) as usize;
        self.layers
            .iter()
            .find(|layer| layer.name == layer_name)
            .and_then(|layer| layer.tiles.get(index).copied())
    }
}

pub fn minimal_paper2d_assets() -> Value {
    json!({
        "sprite": {
            "guid": "sprite-player-idle",
            "name": "PlayerIdle",
            "texture": "assets/sprites/player.png",
            "pixels_per_unit": 32.0,
            "pivot": [0.5, 0.5],
            "rect": {"x": 0, "y": 0, "width": 32, "height": 32},
            "labels": ["player", "demo"]
        },
        "tileset": {
            "guid": "tileset-demo",
            "name": "DemoTiles",
            "texture": "assets/tiles/demo_tiles.png",
            "tile_width": 16,
            "tile_height": 16,
            "columns": 8,
            "collision_tiles": [
                {"tile_id": 1, "shape": "rect", "layer": "WorldStatic", "is_trigger": false, "one_way": false, "damage_per_second": 0.0, "water": false, "ladder": false, "metadata": {"tag": "solid"}}
            ]
        },
        "tilemap": {
            "name": "DemoMap",
            "tileset": "assets/tiles/demo.tileset.json",
            "width": 4,
            "height": 3,
            "tile_width": 16,
            "tile_height": 16,
            "chunk_width": 16,
            "chunk_height": 16,
            "layers": [
                {"name": "Ground", "visible": true, "collision": false, "tiles": [0,0,0,0,0,1,1,0,0,0,0,0]},
                {"name": "Collision", "visible": false, "collision": true, "tiles": [0,0,0,0,0,1,1,0,0,0,0,0]}
            ],
            "autotiles": [{"name": "GrassEdge", "source_tile": 2, "variants": [2,3,4,5]}],
            "animated_tiles": [{"tile_id": 6, "frames": [6,7,8], "fps": 6.0}]
        },
        "flipbook": {
            "name": "PlayerRun",
            "frames_per_second": 10.0,
            "looping": true,
            "frames": [
                {"sprite": "PlayerRun_0", "duration": 0.1},
                {"sprite": "PlayerRun_1", "duration": 0.1}
            ],
            "frame_events": [{"frame": 1, "event": "footstep", "payload": {"volume": 0.5}}]
        },
        "spriteframes": {
            "name": "PlayerSpriteFrames",
            "texture": "assets/player_sheet.png",
            "animations": [{
                "name": "attack",
                "fps": 12.0,
                "looped": false,
                "ping_pong": false,
                "tags": ["combat"],
                "frames": [{
                    "rect": {"x": 0, "y": 64, "width": 32, "height": 32},
                    "duration": 0.083,
                    "pivot": [16.0, 24.0],
                    "hitboxes": [{"name": "sword", "rect": {"x": 22, "y": 10, "width": 20, "height": 12}, "damage": 10.0}],
                    "hurtboxes": [{"name": "body", "rect": {"x": 8, "y": 4, "width": 16, "height": 26}, "damage": 0.0}],
                    "collision_shapes": [{"name": "feet", "shape": "rect", "points": [[10.0, 24.0], [22.0, 32.0]]}],
                    "events": [{"frame": 1, "event": "spawn_attack_fx", "payload": {"time": 0.12}}]
                }]
            }]
        }
    })
}
