use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::entities::game_object::GameObject;

const DEFAULT_MAX_CELLS_PER_ENTITY: usize = 256;
const MAX_DIRECT_QUERY_CELLS: usize = 65_536;

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

/// Deterministic uniform-grid index shared by runtime queries and Luau.
///
/// Entries are owned exactly once in `entries`; occupied cells store only IDs.
/// This avoids cloning names, tags and layers into every covered cell. Very
/// large entities are kept in a small overflow set so a malformed collider can
/// never allocate millions of grid cells.
#[derive(Debug, Clone)]
pub struct SpatialIndex {
    pub cell_size: f64,
    pub cells: BTreeMap<(i32, i32), Vec<u64>>,
    pub entity_cells: BTreeMap<u64, Vec<(i32, i32)>>,
    pub entries: BTreeMap<u64, SpatialEntry>,
    pub oversized_entities: BTreeSet<u64>,
    pub max_cells_per_entity: usize,
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new(4.0)
    }
}

impl SpatialIndex {
    pub fn new(cell_size: f64) -> Self {
        Self::with_limits(cell_size, DEFAULT_MAX_CELLS_PER_ENTITY)
    }

    pub fn with_limits(cell_size: f64, max_cells_per_entity: usize) -> Self {
        Self {
            cell_size: finite_or(cell_size, 4.0).abs().max(0.1),
            cells: BTreeMap::new(),
            entity_cells: BTreeMap::new(),
            entries: BTreeMap::new(),
            oversized_entities: BTreeSet::new(),
            max_cells_per_entity: max_cells_per_entity.max(1),
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
        self.entries.clear();
        self.oversized_entities.clear();
    }

    pub fn insert(&mut self, entity: &GameObject) {
        // `insert` is also safe as an upsert. This matters during hot reload and
        // avoids duplicate IDs accumulating in cell vectors.
        self.remove(entity.id);
        if !entity.enabled || !entity.active {
            return;
        }

        let entry = SpatialEntry::from_entity(entity);
        let min_cell = self.cell_for(entry.x - entry.half_w, entry.y - entry.half_h);
        let max_cell = self.cell_for(entry.x + entry.half_w, entry.y + entry.half_h);
        let covered_cells = cell_span(min_cell, max_cell);
        self.entries.insert(entity.id, entry);

        if covered_cells > self.max_cells_per_entity {
            self.entity_cells.insert(entity.id, Vec::new());
            self.oversized_entities.insert(entity.id);
            return;
        }

        let mut occupied = Vec::with_capacity(covered_cells);
        for cy in min_cell.1..=max_cell.1 {
            for cx in min_cell.0..=max_cell.0 {
                let cell = (cx, cy);
                self.cells.entry(cell).or_default().push(entity.id);
                occupied.push(cell);
            }
        }
        self.entity_cells.insert(entity.id, occupied);
    }

    pub fn remove(&mut self, entity_id: u64) -> bool {
        let existed = self.entries.remove(&entity_id).is_some();
        self.oversized_entities.remove(&entity_id);
        let Some(cells) = self.entity_cells.remove(&entity_id) else {
            return existed;
        };

        for cell in cells {
            let remove_cell = if let Some(ids) = self.cells.get_mut(&cell) {
                ids.retain(|id| *id != entity_id);
                ids.is_empty()
            } else {
                false
            };
            if remove_cell {
                self.cells.remove(&cell);
            }
        }
        true
    }

    pub fn update_entity(&mut self, entity: &GameObject) {
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
        let x = finite_or(x, 0.0);
        let y = finite_or(y, 0.0);
        let radius = finite_or(radius, 0.0).max(0.0);
        let min_cell = self.cell_for(x - radius, y - radius);
        let max_cell = self.cell_for(x + radius, y + radius);
        let mut seen = HashSet::new();
        let mut hits = Vec::new();

        self.visit_candidate_ids(min_cell, max_cell, |entity_id| {
            if !seen.insert(entity_id) {
                return;
            }
            let Some(entry) = self.entries.get(&entity_id) else {
                return;
            };
            if !matches_filters(entry, tag, layer) {
                return;
            }
            let combined_radius = radius + entry.radius;
            let dx = entry.x - x;
            let dy = entry.y - y;
            if dx * dx + dy * dy <= combined_radius * combined_radius {
                hits.push(entry.clone());
            }
        });
        hits.sort_unstable_by_key(|entry| entry.entity_id);
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
        let raw_min_x = finite_or(min_x, 0.0);
        let raw_min_y = finite_or(min_y, 0.0);
        let raw_max_x = finite_or(max_x, raw_min_x);
        let raw_max_y = finite_or(max_y, raw_min_y);
        let min_x = raw_min_x.min(raw_max_x);
        let min_y = raw_min_y.min(raw_max_y);
        let max_x = raw_min_x.max(raw_max_x);
        let max_y = raw_min_y.max(raw_max_y);
        let min_cell = self.cell_for(min_x, min_y);
        let max_cell = self.cell_for(max_x, max_y);
        let mut seen = HashSet::new();
        let mut hits = Vec::new();

        self.visit_candidate_ids(min_cell, max_cell, |entity_id| {
            if !seen.insert(entity_id) {
                return;
            }
            let Some(entry) = self.entries.get(&entity_id) else {
                return;
            };
            if !matches_filters(entry, tag, layer) {
                return;
            }
            if entry.x + entry.half_w >= min_x
                && entry.x - entry.half_w <= max_x
                && entry.y + entry.half_h >= min_y
                && entry.y - entry.half_h <= max_y
            {
                hits.push(entry.clone());
            }
        });
        hits.sort_unstable_by_key(|entry| entry.entity_id);
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
                da.total_cmp(&db).then(a.entity_id.cmp(&b.entity_id))
            })
    }

    pub fn stats(&self) -> BTreeMap<String, usize> {
        BTreeMap::from([
            ("cells".to_string(), self.cells.len()),
            ("entities".to_string(), self.entries.len()),
            ("entries".to_string(), self.entries.len()),
            (
                "cell_references".to_string(),
                self.cells.values().map(Vec::len).sum::<usize>(),
            ),
            (
                "oversized_entities".to_string(),
                self.oversized_entities.len(),
            ),
        ])
    }

    pub fn cell_for(&self, x: f64, y: f64) -> (i32, i32) {
        (
            (finite_or(x, 0.0) / self.cell_size).floor() as i32,
            (finite_or(y, 0.0) / self.cell_size).floor() as i32,
        )
    }

    fn visit_candidate_ids(
        &self,
        min_cell: (i32, i32),
        max_cell: (i32, i32),
        mut visitor: impl FnMut(u64),
    ) {
        if cell_span(min_cell, max_cell) <= MAX_DIRECT_QUERY_CELLS {
            for cy in min_cell.1..=max_cell.1 {
                for cx in min_cell.0..=max_cell.0 {
                    if let Some(ids) = self.cells.get(&(cx, cy)) {
                        for id in ids {
                            visitor(*id);
                        }
                    }
                }
            }
        } else {
            // Huge queries iterate populated cells instead of walking billions
            // of empty coordinates.
            for (&(cx, cy), ids) in &self.cells {
                if cx < min_cell.0 || cx > max_cell.0 || cy < min_cell.1 || cy > max_cell.1 {
                    continue;
                }
                for id in ids {
                    visitor(*id);
                }
            }
        }
        for id in &self.oversized_entities {
            visitor(*id);
        }
    }
}

impl SpatialEntry {
    pub fn from_entity(entity: &GameObject) -> Self {
        let x = finite_or(entity.x, 0.0);
        let y = finite_or(entity.y, 0.0);
        let scale_x = finite_or(entity.scale_x, 1.0).abs();
        let scale_y = finite_or(entity.scale_y, 1.0).abs();
        let radius = finite_or(entity.radius, 0.0).abs();
        let local_half_w = finite_or(entity.width, 0.0)
            .abs()
            .max(radius * 2.0)
            .max(0.1)
            * scale_x
            * 0.5;
        let local_half_h = finite_or(entity.height, 0.0)
            .abs()
            .max(radius * 2.0)
            .max(0.1)
            * scale_y
            * 0.5;
        let angle = finite_or(entity.rotation, 0.0).to_radians();
        let cosine = angle.cos().abs();
        let sine = angle.sin().abs();
        let half_w = cosine * local_half_w + sine * local_half_h;
        let half_h = sine * local_half_w + cosine * local_half_h;
        Self {
            entity_id: entity.id,
            name: entity.name.clone(),
            tag: entity.tag.clone(),
            layer: entity.layer.clone(),
            x,
            y,
            radius: half_w.hypot(half_h),
            half_w,
            half_h,
        }
    }
}

fn matches_filters(entry: &SpatialEntry, tag: Option<&str>, layer: Option<&str>) -> bool {
    tag.is_none_or(|tag| entry.tag == tag) && layer.is_none_or(|layer| entry.layer == layer)
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

fn cell_span(min_cell: (i32, i32), max_cell: (i32, i32)) -> usize {
    let width = i64::from(max_cell.0)
        .saturating_sub(i64::from(min_cell.0))
        .saturating_add(1)
        .max(0) as usize;
    let height = i64::from(max_cell.1)
        .saturating_sub(i64::from(min_cell.1))
        .saturating_add(1)
        .max(0) as usize;
    width.saturating_mul(height)
}

#[cfg(test)]
mod tests {
    use super::SpatialIndex;
    use crate::entities::game_object::GameObject;

    #[test]
    fn stores_each_entry_once_even_when_it_occupies_many_cells() {
        let mut entity = GameObject::new(4.0, 4.0, Some("Wide".to_string()));
        entity.width = 20.0;
        entity.height = 12.0;
        let mut index = SpatialIndex::new(2.0);
        index.insert(&entity);

        assert_eq!(index.entries.len(), 1);
        assert!(index.stats()["cell_references"] > 1);
        assert_eq!(
            index.query_rect(-20.0, -20.0, 20.0, 20.0, None, None).len(),
            1
        );
    }

    #[test]
    fn oversized_entities_do_not_explode_cell_storage_and_are_queryable() {
        let mut entity = GameObject::new(0.0, 0.0, Some("World".to_string()));
        entity.width = 1_000_000.0;
        entity.height = 1_000_000.0;
        let id = entity.id;
        let mut index = SpatialIndex::with_limits(1.0, 16);
        index.insert(&entity);

        assert!(index.cells.is_empty());
        assert!(index.oversized_entities.contains(&id));
        assert_eq!(
            index.query_rect(200.0, 200.0, 201.0, 201.0, None, None)[0].entity_id,
            id
        );
    }

    #[test]
    fn upsert_and_remove_never_leave_duplicate_or_empty_cells() {
        let mut entity = GameObject::new(1.0, 1.0, Some("Mover".to_string()));
        let id = entity.id;
        let mut index = SpatialIndex::new(2.0);
        index.insert(&entity);
        entity.x = 80.0;
        index.update_entity(&entity);

        assert!(index.query_radius(1.0, 1.0, 1.0, None, None).is_empty());
        assert_eq!(index.query_radius(80.0, 1.0, 1.0, None, None).len(), 1);
        assert!(index.remove(id));
        assert!(index.cells.is_empty());
        assert!(index.entries.is_empty());
    }

    #[test]
    fn filters_are_deterministic_and_inactive_entities_are_excluded() {
        let mut second = GameObject::new(0.0, 0.0, Some("Second".to_string()));
        second.tag = "Enemy".to_string();
        second.layer = "Actors".to_string();
        let mut inactive = GameObject::new(0.0, 0.0, Some("Inactive".to_string()));
        inactive.active = false;
        let first = GameObject::new(0.0, 0.0, Some("First".to_string()));
        let expected = second.id;
        let mut index = SpatialIndex::new(4.0);
        index.rebuild(&[first, second, inactive]);

        let hits = index.query_radius(0.0, 0.0, 2.0, Some("Enemy"), Some("Actors"));
        assert_eq!(
            hits.iter().map(|entry| entry.entity_id).collect::<Vec<_>>(),
            vec![expected]
        );
    }
}
