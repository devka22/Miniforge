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
        }
    })
}
