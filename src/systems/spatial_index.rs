use std::collections::BTreeSet;

use rstar::{AABB, RTree, RTreeObject};

use crate::entities::game_object::GameObject;

/// A compact, rebuild-per-frame spatial index for runtime systems.
///
/// MiniForge scenes still own entities in a `Vec<GameObject>`; this index stores
/// only stable indices and AABBs, so physics, rendering and AI can share fast
/// broad-phase queries without duplicating or moving entity data.
#[derive(Debug, Clone)]
pub struct SpatialEntry {
    pub entity_index: usize,
    pub entity_id: u64,
    envelope: AABB<[f64; 2]>,
}

impl RTreeObject for SpatialEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

#[derive(Debug, Clone, Default)]
pub struct EntitySpatialIndex {
    tree: RTree<SpatialEntry>,
}

impl EntitySpatialIndex {
    pub fn from_entities(entities: &[GameObject]) -> Self {
        let entries = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.enabled && entity.active)
            .map(|(index, entity)| {
                let (min, max) = entity_aabb(entity);
                SpatialEntry {
                    entity_index: index,
                    entity_id: entity.id,
                    envelope: AABB::from_corners(min, max),
                }
            })
            .collect();
        Self {
            tree: RTree::bulk_load(entries),
        }
    }

    pub fn from_bounds(bounds: impl IntoIterator<Item = (usize, u64, [f64; 2], [f64; 2])>) -> Self {
        let entries = bounds
            .into_iter()
            .map(|(entity_index, entity_id, min, max)| SpatialEntry {
                entity_index,
                entity_id,
                envelope: AABB::from_corners(min, max),
            })
            .collect();
        Self {
            tree: RTree::bulk_load(entries),
        }
    }

    pub fn len(&self) -> usize {
        self.tree.size()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }

    pub fn query_aabb(&self, min: [f64; 2], max: [f64; 2]) -> Vec<usize> {
        let envelope = AABB::from_corners(min, max);
        self.tree
            .locate_in_envelope_intersecting(envelope)
            .map(|entry| entry.entity_index)
            .collect()
    }

    pub fn query_radius(&self, center: (f64, f64), radius: f64) -> Vec<usize> {
        let radius = radius.max(0.0);
        self.query_aabb(
            [center.0 - radius, center.1 - radius],
            [center.0 + radius, center.1 + radius],
        )
    }

    /// Returns unique entity-index pairs whose AABBs overlap.
    pub fn overlapping_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = BTreeSet::new();
        for entry in self.tree.iter() {
            for other in self.tree.locate_in_envelope_intersecting(entry.envelope) {
                if entry.entity_index == other.entity_index {
                    continue;
                }
                let pair = if entry.entity_index < other.entity_index {
                    (entry.entity_index, other.entity_index)
                } else {
                    (other.entity_index, entry.entity_index)
                };
                pairs.insert(pair);
            }
        }
        pairs.into_iter().collect()
    }
}

pub fn entity_aabb(entity: &GameObject) -> ([f64; 2], [f64; 2]) {
    let collider = entity
        .get_component("Collider2D")
        .or_else(|| entity.get_component("Area2D"));
    let offset_x = collider
        .map(|value| value.get_f64("offset_x", 0.0))
        .unwrap_or(0.0);
    let offset_y = collider
        .map(|value| value.get_f64("offset_y", 0.0))
        .unwrap_or(0.0);
    let center = [entity.x + offset_x, entity.y + offset_y];
    let shape = collider
        .map(|value| value.get_string("shape", "rect").to_lowercase())
        .unwrap_or_else(|| "rect".to_string());

    if shape == "polygon"
        && let Some(points) = collider
            .and_then(|value| value.get("points"))
            .and_then(serde_json::Value::as_array)
    {
        let angle = entity.rotation.to_radians();
        let (cosine, sine) = (angle.cos(), angle.sin());
        let transformed: Vec<[f64; 2]> = points
            .iter()
            .filter_map(|point| {
                let pair = point.as_array()?;
                let x = pair.first()?.as_f64()? * entity.scale_x;
                let y = pair.get(1)?.as_f64()? * entity.scale_y;
                Some([
                    center[0] + x * cosine - y * sine,
                    center[1] + x * sine + y * cosine,
                ])
            })
            .collect();
        if let Some(first) = transformed.first() {
            let mut min = *first;
            let mut max = *first;
            for point in transformed.iter().skip(1) {
                min[0] = min[0].min(point[0]);
                min[1] = min[1].min(point[1]);
                max[0] = max[0].max(point[0]);
                max[1] = max[1].max(point[1]);
            }
            return (min, max);
        }
    }

    let (extent_x, extent_y) = if shape == "circle" {
        let radius = collider
            .map(|value| value.get_f64("radius", entity.radius))
            .unwrap_or(entity.radius)
            .max(0.001);
        let scale = entity.scale_x.abs().max(entity.scale_y.abs());
        (radius * scale, radius * scale)
    } else {
        let half_width = collider
            .map(|value| value.get_f64("width", entity.width))
            .unwrap_or(entity.width)
            .abs()
            * entity.scale_x.abs()
            * 0.5;
        let half_height = collider
            .map(|value| value.get_f64("height", entity.height))
            .unwrap_or(entity.height)
            .abs()
            * entity.scale_y.abs()
            * 0.5;
        let angle = entity.rotation.to_radians();
        let cosine = angle.cos().abs();
        let sine = angle.sin().abs();
        (
            cosine * half_width + sine * half_height,
            sine * half_width + cosine * half_height,
        )
    };

    (
        [center[0] - extent_x, center[1] - extent_y],
        [center[0] + extent_x, center[1] + extent_y],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_returns_unique_overlapping_pairs() {
        let index = EntitySpatialIndex::from_bounds([
            (0, 1, [0.0, 0.0], [2.0, 2.0]),
            (1, 2, [1.0, 1.0], [3.0, 3.0]),
            (2, 3, [8.0, 8.0], [9.0, 9.0]),
        ]);
        assert_eq!(index.overlapping_pairs(), vec![(0, 1)]);
        assert_eq!(index.query_radius((8.5, 8.5), 0.2), vec![2]);
    }
}
