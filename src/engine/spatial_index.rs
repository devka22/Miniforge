use std::collections::{BTreeMap, BTreeSet};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, PartialEq)]
pub struct SpatialEntry {
    pub entity_id: u64,
    pub name: String,
    pub tag: String,
    pub layer: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub half_w: f64,
    pub half_h: f64,
}

#[derive(Debug, Clone)]
pub struct SpatialIndex {
    pub cell_size: f64,
    pub cells: BTreeMap<(i32, i32), Vec<SpatialEntry>>,
    pub entity_cells: BTreeMap<u64, Vec<(i32, i32)>>,
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new(4.0)
    }
}

impl SpatialIndex {
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size: cell_size.max(0.1),
            cells: BTreeMap::new(),
            entity_cells: BTreeMap::new(),
        }
    }

    pub fn rebuild(&mut self, entities: &[GameObject]) {
        self.clear();
        for entity in entities {
            self.insert(entity);
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    pub fn insert(&mut self, entity: &GameObject) {
        if !entity.enabled {
            return;
        }
        let entry = SpatialEntry::from_entity(entity);
        let min_cell = self.cell_for(entry.x - entry.half_w, entry.y - entry.half_h);
        let max_cell = self.cell_for(entry.x + entry.half_w, entry.y + entry.half_h);
        let mut cells = Vec::new();
        for cy in min_cell.1..=max_cell.1 {
            for cx in min_cell.0..=max_cell.0 {
                let cell = (cx, cy);
                self.cells.entry(cell).or_default().push(entry.clone());
                cells.push(cell);
            }
        }
        self.entity_cells.insert(entity.id, cells);
    }

    pub fn remove(&mut self, entity_id: u64) -> bool {
        let Some(cells) = self.entity_cells.remove(&entity_id) else {
            return false;
        };
        for cell in cells {
            if let Some(entries) = self.cells.get_mut(&cell) {
                entries.retain(|entry| entry.entity_id != entity_id);
            }
        }
        self.cells.retain(|_, entries| !entries.is_empty());
        true
    }

    pub fn update_entity(&mut self, entity: &GameObject) {
        self.remove(entity.id);
        self.insert(entity);
    }

    pub fn query_radius(
        &self,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> Vec<SpatialEntry> {
        let radius = radius.max(0.0);
        let min_cell = self.cell_for(x - radius, y - radius);
        let max_cell = self.cell_for(x + radius, y + radius);
        let mut seen = BTreeSet::new();
        let mut hits = Vec::new();
        for cy in min_cell.1..=max_cell.1 {
            for cx in min_cell.0..=max_cell.0 {
                let Some(entries) = self.cells.get(&(cx, cy)) else {
                    continue;
                };
                for entry in entries {
                    if !seen.insert(entry.entity_id) {
                        continue;
                    }
                    if tag.is_some_and(|tag| entry.tag != tag) {
                        continue;
                    }
                    if layer.is_some_and(|layer| entry.layer != layer) {
                        continue;
                    }
                    let combined_radius = radius + entry.radius;
                    let dx = entry.x - x;
                    let dy = entry.y - y;
                    if dx * dx + dy * dy <= combined_radius * combined_radius {
                        hits.push(entry.clone());
                    }
                }
            }
        }
        hits
    }

    pub fn query_rect(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> Vec<SpatialEntry> {
        let raw_min_x = min_x;
        let raw_min_y = min_y;
        let min_x = raw_min_x.min(max_x);
        let min_y = raw_min_y.min(max_y);
        let max_x = raw_min_x.max(max_x);
        let max_y = raw_min_y.max(max_y);
        let min_cell = self.cell_for(min_x, min_y);
        let max_cell = self.cell_for(max_x, max_y);
        let mut seen = BTreeSet::new();
        let mut hits = Vec::new();
        for cy in min_cell.1..=max_cell.1 {
            for cx in min_cell.0..=max_cell.0 {
                let Some(entries) = self.cells.get(&(cx, cy)) else {
                    continue;
                };
                for entry in entries {
                    if !seen.insert(entry.entity_id) {
                        continue;
                    }
                    if tag.is_some_and(|tag| entry.tag != tag) {
                        continue;
                    }
                    if layer.is_some_and(|layer| entry.layer != layer) {
                        continue;
                    }
                    if entry.x + entry.half_w >= min_x
                        && entry.x - entry.half_w <= max_x
                        && entry.y + entry.half_h >= min_y
                        && entry.y - entry.half_h <= max_y
                    {
                        hits.push(entry.clone());
                    }
                }
            }
        }
        hits
    }

    pub fn nearest(
        &self,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> Option<SpatialEntry> {
        self.query_radius(x, y, radius, tag, layer)
            .into_iter()
            .min_by(|a, b| {
                let da = (a.x - x).powi(2) + (a.y - y).powi(2);
                let db = (b.x - x).powi(2) + (b.y - y).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn stats(&self) -> BTreeMap<String, usize> {
        BTreeMap::from([
            ("cells".to_string(), self.cells.len()),
            ("entities".to_string(), self.entity_cells.len()),
            (
                "entries".to_string(),
                self.cells.values().map(Vec::len).sum::<usize>(),
            ),
        ])
    }

    pub fn cell_for(&self, x: f64, y: f64) -> (i32, i32) {
        (
            (x / self.cell_size).floor() as i32,
            (y / self.cell_size).floor() as i32,
        )
    }
}

impl SpatialEntry {
    pub fn from_entity(entity: &GameObject) -> Self {
        let half_w = entity.width.max(entity.radius * 2.0).max(0.1) * 0.5;
        let half_h = entity.height.max(entity.radius * 2.0).max(0.1) * 0.5;
        Self {
            entity_id: entity.id,
            name: entity.name.clone(),
            tag: entity.tag.clone(),
            layer: entity.layer.clone(),
            x: entity.x,
            y: entity.y,
            radius: entity.radius.max(half_w.max(half_h)),
            half_w,
            half_h,
        }
    }
}
