use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::tilemap_layers::TilemapLayers;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileBrushKind2D {
    Pencil,
    Eraser,
    Fill,
    Rectangle,
    Line,
    Random,
    Terrain,
    Collision,
    Object,
    Stamp,
    Rule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TileChange2D {
    pub layer: usize,
    pub x: usize,
    pub y: usize,
    pub before: i32,
    pub after: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TileBrushStroke2D {
    pub brush: TileBrushKind2D,
    #[serde(default)]
    pub changes: Vec<TileChange2D>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TileCoord2D {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TileSelection2D {
    pub layer: usize,
    #[serde(default)]
    pub cells: Vec<TileCoord2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TilePattern2D {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub anchor_x: i32,
    pub anchor_y: i32,
    #[serde(default)]
    pub transparent_tile: Option<i32>,
    #[serde(default)]
    pub tiles: Vec<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TilePalette2D {
    pub name: String,
    pub selected: i32,
    #[serde(default)]
    pub tiles: BTreeMap<String, i32>,
    #[serde(default)]
    pub tags: BTreeMap<i32, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainRule2D {
    pub name: String,
    pub center_tile: i32,
    #[serde(default)]
    pub neighbors: BTreeMap<String, i32>,
    pub output_tile: i32,
    pub priority: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainSet2D {
    pub name: String,
    #[serde(default)]
    pub rules: Vec<TerrainRule2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleTile2D {
    pub name: String,
    pub output_tile: i32,
    pub probability_percent: u8,
    #[serde(default)]
    pub required_neighbors: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StampBrush2D {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub anchor_x: i32,
    pub anchor_y: i32,
    #[serde(default)]
    pub tiles: Vec<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectBrush2D {
    pub name: String,
    pub prefab: String,
    pub density: f32,
    pub align_to_grid: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileObjectPlacement2D {
    pub prefab: String,
    pub x: usize,
    pub y: usize,
    pub layer: usize,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TilemapEditor2D {
    pub tilemap: TilemapLayers,
    pub active_layer: usize,
    pub active_brush: TileBrushKind2D,
    pub palette: TilePalette2D,
    #[serde(default)]
    pub terrain_sets: Vec<TerrainSet2D>,
    #[serde(default)]
    pub rule_tiles: Vec<RuleTile2D>,
    #[serde(default)]
    pub stamps: Vec<StampBrush2D>,
    #[serde(default)]
    pub object_brushes: Vec<ObjectBrush2D>,
    #[serde(default)]
    pub placed_objects: Vec<TileObjectPlacement2D>,
    #[serde(default)]
    pub last_strokes: Vec<TileBrushStroke2D>,
    #[serde(default)]
    pub selection: TileSelection2D,
    #[serde(default)]
    pub clipboard: Option<TilePattern2D>,
    pub random_seed: u64,
}

impl TileSelection2D {
    pub fn rectangle(
        layer: usize,
        start: (usize, usize),
        end: (usize, usize),
        map_width: usize,
        map_height: usize,
    ) -> Self {
        let min_x = start.0.min(end.0);
        let max_x = start.0.max(end.0).min(map_width.saturating_sub(1));
        let min_y = start.1.min(end.1);
        let max_y = start.1.max(end.1).min(map_height.saturating_sub(1));
        let mut cells = Vec::new();
        if map_width == 0 || map_height == 0 || min_x >= map_width || min_y >= map_height {
            return Self { layer, cells };
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                cells.push(TileCoord2D { x, y });
            }
        }
        Self { layer, cells }
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let first = self.cells.first()?;
        let (mut min_x, mut max_x) = (first.x, first.x);
        let (mut min_y, mut max_y) = (first.y, first.y);
        for cell in &self.cells {
            min_x = min_x.min(cell.x);
            max_x = max_x.max(cell.x);
            min_y = min_y.min(cell.y);
            max_y = max_y.max(cell.y);
        }
        Some((min_x, min_y, max_x, max_y))
    }
}

impl TilePattern2D {
    pub fn rotated_right(&self) -> Self {
        let mut tiles = vec![vec![self.transparent_tile.unwrap_or(0); self.height]; self.width];
        for y in 0..self.height {
            for (x, column) in tiles.iter_mut().enumerate().take(self.width) {
                if let Some(tile) = self.tiles.get(y).and_then(|row| row.get(x)).copied() {
                    column[self.height - 1 - y] = tile;
                }
            }
        }
        Self {
            name: format!("{} Rotated", self.name),
            width: self.height,
            height: self.width,
            anchor_x: self.height as i32 - 1 - self.anchor_y,
            anchor_y: self.anchor_x,
            transparent_tile: self.transparent_tile,
            tiles,
        }
    }

    pub fn flipped_h(&self) -> Self {
        let mut next = self.clone();
        next.name = format!("{} FlipH", self.name);
        for row in &mut next.tiles {
            row.reverse();
        }
        next.anchor_x = self.width as i32 - 1 - self.anchor_x;
        next
    }

    pub fn flipped_v(&self) -> Self {
        let mut next = self.clone();
        next.name = format!("{} FlipV", self.name);
        next.tiles.reverse();
        next.anchor_y = self.height as i32 - 1 - self.anchor_y;
        next
    }
}

impl TilemapEditor2D {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            tilemap: TilemapLayers::new(width, height),
            active_layer: 0,
            active_brush: TileBrushKind2D::Pencil,
            palette: TilePalette2D {
                name: "DefaultTiles".to_string(),
                selected: 1,
                tiles: BTreeMap::from([
                    ("Grass".to_string(), 1),
                    ("Dirt".to_string(), 2),
                    ("Stone".to_string(), 3),
                    ("Water".to_string(), 4),
                    ("Wall".to_string(), 9),
                ]),
                tags: BTreeMap::from([
                    (1, vec!["ground".to_string()]),
                    (4, vec!["water".to_string(), "slow".to_string()]),
                    (9, vec!["collision".to_string()]),
                ]),
            },
            terrain_sets: vec![default_terrain_set()],
            rule_tiles: vec![RuleTile2D {
                name: "WaterEdge".to_string(),
                output_tile: 5,
                probability_percent: 100,
                required_neighbors: BTreeMap::from([("north".to_string(), 4)]),
            }],
            stamps: vec![StampBrush2D {
                name: "ThreeByThreeRoom".to_string(),
                width: 3,
                height: 3,
                anchor_x: 1,
                anchor_y: 1,
                tiles: vec![vec![9, 9, 9], vec![9, 1, 9], vec![9, 9, 9]],
            }],
            object_brushes: vec![ObjectBrush2D {
                name: "CoinScatter".to_string(),
                prefab: "assets/prefabs/coin.prefab".to_string(),
                density: 0.35,
                align_to_grid: true,
                tags: vec!["pickup".to_string()],
            }],
            placed_objects: Vec::new(),
            last_strokes: Vec::new(),
            selection: TileSelection2D::default(),
            clipboard: None,
            random_seed: 0xC0FFEE,
        }
    }

    pub fn paint_cell(
        &mut self,
        layer: usize,
        x: usize,
        y: usize,
        value: i32,
    ) -> TileBrushStroke2D {
        let mut stroke = TileBrushStroke2D {
            brush: TileBrushKind2D::Pencil,
            changes: Vec::new(),
        };
        self.record_change(layer, x, y, value, &mut stroke);
        self.remember(stroke.clone());
        stroke
    }

    pub fn apply_line(
        &mut self,
        layer: usize,
        start: (usize, usize),
        end: (usize, usize),
        value: i32,
    ) -> TileBrushStroke2D {
        let mut stroke = TileBrushStroke2D {
            brush: TileBrushKind2D::Line,
            changes: Vec::new(),
        };
        for (x, y) in bresenham(start, end) {
            self.record_change(layer, x, y, value, &mut stroke);
        }
        self.remember(stroke.clone());
        stroke
    }

    pub fn apply_rectangle(
        &mut self,
        layer: usize,
        start: (usize, usize),
        end: (usize, usize),
        value: i32,
        filled: bool,
    ) -> TileBrushStroke2D {
        let mut stroke = TileBrushStroke2D {
            brush: TileBrushKind2D::Rectangle,
            changes: Vec::new(),
        };
        let min_x = start.0.min(end.0);
        let max_x = start.0.max(end.0);
        let min_y = start.1.min(end.1);
        let max_y = start.1.max(end.1);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if filled || x == min_x || x == max_x || y == min_y || y == max_y {
                    self.record_change(layer, x, y, value, &mut stroke);
                }
            }
        }
        self.remember(stroke.clone());
        stroke
    }

    pub fn select_rectangle(
        &mut self,
        layer: usize,
        start: (usize, usize),
        end: (usize, usize),
    ) -> TileSelection2D {
        self.selection =
            TileSelection2D::rectangle(layer, start, end, self.tilemap.width, self.tilemap.height);
        self.selection.clone()
    }

    pub fn clear_selection(&mut self) {
        self.selection.cells.clear();
    }

    pub fn copy_selection(&mut self, name: &str) -> Option<TilePattern2D> {
        if self.selection.layer >= self.tilemap.layers.len() {
            return None;
        }
        let (min_x, min_y, max_x, max_y) = self.selection.bounds()?;
        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;
        let transparent_tile = -1;
        let mut tiles = vec![vec![transparent_tile; width]; height];
        for cell in &self.selection.cells {
            let x = cell.x - min_x;
            let y = cell.y - min_y;
            tiles[y][x] = self.tilemap.layers[self.selection.layer].get(cell.x, cell.y);
        }
        let pattern = TilePattern2D {
            name: name.to_string(),
            width,
            height,
            anchor_x: 0,
            anchor_y: 0,
            transparent_tile: Some(transparent_tile),
            tiles,
        };
        self.clipboard = Some(pattern.clone());
        Some(pattern)
    }

    pub fn paste_clipboard(&mut self, layer: usize, origin: (usize, usize)) -> TileBrushStroke2D {
        let Some(pattern) = self.clipboard.clone() else {
            return TileBrushStroke2D {
                brush: TileBrushKind2D::Stamp,
                changes: Vec::new(),
            };
        };
        self.paste_pattern(layer, &pattern, origin)
    }

    pub fn paste_pattern(
        &mut self,
        layer: usize,
        pattern: &TilePattern2D,
        origin: (usize, usize),
    ) -> TileBrushStroke2D {
        let mut stroke = TileBrushStroke2D {
            brush: TileBrushKind2D::Stamp,
            changes: Vec::new(),
        };
        for y in 0..pattern.height {
            for x in 0..pattern.width {
                let Some(tile) = pattern.tiles.get(y).and_then(|row| row.get(x)).copied() else {
                    continue;
                };
                if pattern.transparent_tile == Some(tile) {
                    continue;
                }
                let tx = origin.0 as i32 + x as i32 - pattern.anchor_x;
                let ty = origin.1 as i32 + y as i32 - pattern.anchor_y;
                if tx >= 0 && ty >= 0 {
                    self.record_change(layer, tx as usize, ty as usize, tile, &mut stroke);
                }
            }
        }
        self.remember(stroke.clone());
        stroke
    }

    pub fn apply_stamp(
        &mut self,
        layer: usize,
        stamp_name: &str,
        origin: (usize, usize),
    ) -> TileBrushStroke2D {
        let mut stroke = TileBrushStroke2D {
            brush: TileBrushKind2D::Stamp,
            changes: Vec::new(),
        };
        let Some(stamp) = self
            .stamps
            .iter()
            .find(|stamp| stamp.name == stamp_name)
            .cloned()
        else {
            return stroke;
        };
        for y in 0..stamp.height {
            for x in 0..stamp.width {
                let Some(row) = stamp.tiles.get(y) else {
                    continue;
                };
                let Some(&tile) = row.get(x) else {
                    continue;
                };
                let tx = origin.0 as i32 + x as i32 - stamp.anchor_x;
                let ty = origin.1 as i32 + y as i32 - stamp.anchor_y;
                if tx >= 0 && ty >= 0 {
                    self.record_change(layer, tx as usize, ty as usize, tile, &mut stroke);
                }
            }
        }
        self.remember(stroke.clone());
        stroke
    }

    pub fn apply_random(
        &mut self,
        layer: usize,
        start: (usize, usize),
        end: (usize, usize),
        choices: &[(i32, u32)],
    ) -> TileBrushStroke2D {
        let mut stroke = TileBrushStroke2D {
            brush: TileBrushKind2D::Random,
            changes: Vec::new(),
        };
        if choices.is_empty() {
            return stroke;
        }
        let min_x = start.0.min(end.0);
        let max_x = start.0.max(end.0);
        let min_y = start.1.min(end.1);
        let max_y = start.1.max(end.1);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let tile = weighted_choice(choices, next_seed(&mut self.random_seed));
                self.record_change(layer, x, y, tile, &mut stroke);
            }
        }
        self.remember(stroke.clone());
        stroke
    }

    pub fn apply_rule_tiles(&mut self, layer: usize) -> TileBrushStroke2D {
        let mut stroke = TileBrushStroke2D {
            brush: TileBrushKind2D::Rule,
            changes: Vec::new(),
        };
        let rules = self.rule_tiles.clone();
        for y in 0..self.tilemap.height {
            for x in 0..self.tilemap.width {
                for rule in &rules {
                    if self.matches_rule(layer, x, y, rule) {
                        self.record_change(layer, x, y, rule.output_tile, &mut stroke);
                        break;
                    }
                }
            }
        }
        self.remember(stroke.clone());
        stroke
    }

    pub fn place_objects(
        &mut self,
        layer: usize,
        brush_name: &str,
        start: (usize, usize),
        end: (usize, usize),
    ) -> Vec<TileObjectPlacement2D> {
        let Some(brush) = self
            .object_brushes
            .iter()
            .find(|brush| brush.name == brush_name)
            .cloned()
        else {
            return Vec::new();
        };
        let threshold = (brush.density.clamp(0.0, 1.0) * 10_000.0) as u64;
        let min_x = start.0.min(end.0);
        let max_x = start.0.max(end.0);
        let min_y = start.1.min(end.1);
        let max_y = start.1.max(end.1);
        let mut placed = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if next_seed(&mut self.random_seed) % 10_000 <= threshold {
                    placed.push(TileObjectPlacement2D {
                        prefab: brush.prefab.clone(),
                        x,
                        y,
                        layer,
                        tags: brush.tags.clone(),
                    });
                }
            }
        }
        self.placed_objects.extend(placed.clone());
        placed
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.tilemap.width == 0 || self.tilemap.height == 0 {
            issues.push("TilemapEditor2D necesita dimensiones mayores a cero".to_string());
        }
        if self.active_layer >= self.tilemap.layers.len() {
            issues.push("active_layer fuera de rango".to_string());
        }
        for stamp in &self.stamps {
            if stamp.tiles.len() != stamp.height {
                issues.push(format!("stamp {} tiene alto inconsistente", stamp.name));
            }
            if stamp.tiles.iter().any(|row| row.len() != stamp.width) {
                issues.push(format!("stamp {} tiene ancho inconsistente", stamp.name));
            }
        }
        for rule in &self.rule_tiles {
            if rule.probability_percent > 100 {
                issues.push(format!("rule tile {} tiene probabilidad > 100", rule.name));
            }
        }
        if self.selection.layer >= self.tilemap.layers.len() && !self.selection.cells.is_empty() {
            issues.push("selection usa layer fuera de rango".to_string());
        }
        if let Some(pattern) = &self.clipboard {
            if pattern.tiles.len() != pattern.height {
                issues.push(format!("pattern {} tiene alto inconsistente", pattern.name));
            }
            if pattern.tiles.iter().any(|row| row.len() != pattern.width) {
                issues.push(format!(
                    "pattern {} tiene ancho inconsistente",
                    pattern.name
                ));
            }
        }
        issues
    }

    pub fn as_value(&self) -> Value {
        json!(self)
    }

    fn record_change(
        &mut self,
        layer: usize,
        x: usize,
        y: usize,
        value: i32,
        stroke: &mut TileBrushStroke2D,
    ) {
        if layer >= self.tilemap.layers.len() || x >= self.tilemap.width || y >= self.tilemap.height
        {
            return;
        }
        let before = self.tilemap.layers[layer].get(x, y);
        if before == value {
            return;
        }
        self.tilemap.layers[layer].set(x, y, value);
        let after = self.tilemap.layers[layer].get(x, y);
        if before == after {
            return;
        }
        stroke.changes.push(TileChange2D {
            layer,
            x,
            y,
            before,
            after,
        });
    }

    fn matches_rule(&self, layer: usize, x: usize, y: usize, rule: &RuleTile2D) -> bool {
        if layer >= self.tilemap.layers.len() {
            return false;
        }
        let roll = seeded_cell_roll(self.random_seed, x, y);
        if roll % 100 >= u64::from(rule.probability_percent.max(1)) {
            return false;
        }
        rule.required_neighbors.iter().all(|(direction, tile)| {
            let (nx, ny) = match direction.as_str() {
                "north" if y > 0 => (x, y - 1),
                "south" if y + 1 < self.tilemap.height => (x, y + 1),
                "west" if x > 0 => (x - 1, y),
                "east" if x + 1 < self.tilemap.width => (x + 1, y),
                _ => return false,
            };
            self.tilemap.layers[layer].get(nx, ny) == *tile
        })
    }

    fn remember(&mut self, stroke: TileBrushStroke2D) {
        if stroke.changes.is_empty() {
            return;
        }
        self.last_strokes.push(stroke);
        if self.last_strokes.len() > 64 {
            self.last_strokes.remove(0);
        }
    }
}

pub fn minimal_tilemap_editor() -> Value {
    json!(TilemapEditor2D::new(16, 12))
}

fn default_terrain_set() -> TerrainSet2D {
    TerrainSet2D {
        name: "GroundTransitions".to_string(),
        rules: vec![
            TerrainRule2D {
                name: "GrassCenter".to_string(),
                center_tile: 1,
                neighbors: BTreeMap::new(),
                output_tile: 1,
                priority: 0,
            },
            TerrainRule2D {
                name: "WaterNorthEdge".to_string(),
                center_tile: 1,
                neighbors: BTreeMap::from([("north".to_string(), 4)]),
                output_tile: 5,
                priority: 10,
            },
        ],
    }
}

fn bresenham(start: (usize, usize), end: (usize, usize)) -> Vec<(usize, usize)> {
    let mut points = Vec::new();
    let (mut x0, mut y0) = (start.0 as i32, start.1 as i32);
    let (x1, y1) = (end.0 as i32, end.1 as i32);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 {
            points.push((x0 as usize, y0 as usize));
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
    points
}

fn weighted_choice(choices: &[(i32, u32)], seed: u64) -> i32 {
    let total = choices
        .iter()
        .map(|(_, weight)| *weight as u64)
        .sum::<u64>();
    if total == 0 {
        return choices[0].0;
    }
    let mut roll = seed % total;
    for (tile, weight) in choices {
        if roll < u64::from(*weight) {
            return *tile;
        }
        roll -= u64::from(*weight);
    }
    choices[0].0
}

fn seeded_cell_roll(seed: u64, x: usize, y: usize) -> u64 {
    seed.wrapping_add((x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add((y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .rotate_left(17)
}

fn next_seed(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed
}
