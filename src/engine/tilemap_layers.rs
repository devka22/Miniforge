use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileLayer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub tiles: Vec<Vec<i32>>,
}

impl TileLayer {
    pub fn new(name: &str, width: usize, height: usize) -> Self {
        Self {
            name: name.to_string(),
            visible: true,
            locked: false,
            tiles: vec![vec![0; width]; height],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: i32) {
        if !self.locked
            && let Some(row) = self.tiles.get_mut(y)
            && let Some(tile) = row.get_mut(x)
        {
            *tile = value;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> i32 {
        self.tiles
            .get(y)
            .and_then(|row| row.get(x))
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TilemapLayers {
    pub width: usize,
    pub height: usize,
    pub active_layer: usize,
    pub layers: Vec<TileLayer>,
}

impl TilemapLayers {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            active_layer: 0,
            layers: ["Ground", "Decoration", "Collision", "Overlay"]
                .iter()
                .map(|name| TileLayer::new(name, width, height))
                .collect(),
        }
    }

    pub fn layer(&self, name: &str) -> Option<&TileLayer> {
        self.layers.iter().find(|layer| layer.name == name)
    }

    pub fn layer_mut(&mut self, name: &str) -> Option<&mut TileLayer> {
        self.layers.iter_mut().find(|layer| layer.name == name)
    }

    pub fn active_mut(&mut self) -> &mut TileLayer {
        &mut self.layers[self.active_layer]
    }

    pub fn set_tile(&mut self, x: usize, y: usize, value: i32) {
        self.active_mut().set(x, y, value);
    }

    pub fn cycle_layer(&mut self) -> String {
        self.active_layer = (self.active_layer + 1) % self.layers.len();
        self.layers[self.active_layer].name.clone()
    }

    pub fn fill_active(&mut self, x: usize, y: usize, width: usize, height: usize, value: i32) {
        for py in y..(y + height).min(self.height) {
            for px in x..(x + width).min(self.width) {
                self.set_tile(px, py, value);
            }
        }
    }

    pub fn stats(&self) -> BTreeMap<String, usize> {
        BTreeMap::from([
            ("layers".to_string(), self.layers.len()),
            ("width".to_string(), self.width),
            ("height".to_string(), self.height),
        ])
    }

    pub fn serialize(&self) -> Value {
        json!(self)
    }

    pub fn deserialize(&mut self, data: &Value) {
        if let Ok(next) = serde_json::from_value::<TilemapLayers>(data.clone()) {
            *self = next;
        }
    }
}
