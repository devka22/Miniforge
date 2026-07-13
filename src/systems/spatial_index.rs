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
            .map(|(entity_index, entity_id, min, max)| {
                let (min, max) = sanitized_bounds(min, max);
                SpatialEntry {
                    entity_index,
                    entity_id,
                    envelope: AABB::from_corners(min, max),
                }
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
        let mut output = Vec::new();
        self.query_aabb_into(min, max, &mut output);
        output
    }

    /// Reuses caller-owned storage for per-frame visibility/interest queries.
    pub fn query_aabb_into(&self, min: [f64; 2], max: [f64; 2], output: &mut Vec<usize>) {
        output.clear();
        let (min, max) = sanitized_bounds(min, max);
        let envelope = AABB::from_corners(min, max);
        output.extend(
            self.tree
                .locate_in_envelope_intersecting(envelope)
                .map(|entry| entry.entity_index),
        );
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
        let mut pairs = Vec::new();
        for entry in self.tree.iter() {
            for other in self.tree.locate_in_envelope_intersecting(entry.envelope) {
                if entry.entity_index >= other.entity_index {
                    continue;
                }
                pairs.push((entry.entity_index, other.entity_index));
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }
}

pub fn entity_aabb(entity: &GameObject) -> ([f64; 2], [f64; 2]) {
    let collider = entity
        .get_component("Collider2D")
        .or_else(|| entity.get_component("Area2D"));
    let offset_x = collider
        .map(|value| value.get_f64("offset_x", 0.0))
        .map(|value| finite_or(value, 0.0))
        .unwrap_or(0.0);
    let offset_y = collider
        .map(|value| value.get_f64("offset_y", 0.0))
        .map(|value| finite_or(value, 0.0))
        .unwrap_or(0.0);
    let center = [
        finite_or(entity.x, 0.0) + offset_x,
        finite_or(entity.y, 0.0) + offset_y,
    ];
    let shape = collider
        .and_then(|value| value.get("shape"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("rect");

    if shape.eq_ignore_ascii_case("polygon")
        && let Some(points) = collider
            .and_then(|value| value.get("points"))
            .and_then(serde_json::Value::as_array)
    {
        let angle = finite_or(entity.rotation, 0.0).to_radians();
        let (cosine, sine) = (angle.cos(), angle.sin());
        let mut bounds: Option<([f64; 2], [f64; 2])> = None;
        for point in points {
            let Some(pair) = point.as_array() else {
                continue;
            };
            let (Some(x), Some(y)) = (
                pair.first().and_then(serde_json::Value::as_f64),
                pair.get(1).and_then(serde_json::Value::as_f64),
            ) else {
                continue;
            };
            let x = finite_or(x, 0.0) * finite_or(entity.scale_x, 1.0);
            let y = finite_or(y, 0.0) * finite_or(entity.scale_y, 1.0);
            let transformed = [
                center[0] + x * cosine - y * sine,
                center[1] + x * sine + y * cosine,
            ];
            if let Some((min, max)) = &mut bounds {
                min[0] = min[0].min(transformed[0]);
                min[1] = min[1].min(transformed[1]);
                max[0] = max[0].max(transformed[0]);
                max[1] = max[1].max(transformed[1]);
            } else {
                bounds = Some((transformed, transformed));
            }
        }
        if let Some(bounds) = bounds {
            return bounds;
        }
    }

    let (extent_x, extent_y) = if shape.eq_ignore_ascii_case("circle") {
        let radius = finite_or(
            collider
                .map(|value| value.get_f64("radius", entity.radius))
                .unwrap_or(entity.radius),
            0.001,
        )
        .max(0.001);
        let scale = finite_or(entity.scale_x, 1.0)
            .abs()
            .max(finite_or(entity.scale_y, 1.0).abs());
        (radius * scale, radius * scale)
    } else {
        let half_width = finite_or(
            collider
                .map(|value| value.get_f64("width", entity.width))
                .unwrap_or(entity.width),
            0.0,
        )
        .abs()
            * finite_or(entity.scale_x, 1.0).abs()
            * 0.5;
        let half_height = finite_or(
            collider
                .map(|value| value.get_f64("height", entity.height))
                .unwrap_or(entity.height),
            0.0,
        )
        .abs()
            * finite_or(entity.scale_y, 1.0).abs()
            * 0.5;
        let angle = finite_or(entity.rotation, 0.0).to_radians();
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

fn sanitized_bounds(min: [f64; 2], max: [f64; 2]) -> ([f64; 2], [f64; 2]) {
    let min_x = finite_or(min[0], 0.0);
    let min_y = finite_or(min[1], 0.0);
    let max_x = finite_or(max[0], min_x);
    let max_y = finite_or(max[1], min_y);
    (
        [min_x.min(max_x), min_y.min(max_y)],
        [min_x.max(max_x), min_y.max(max_y)],
    )
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
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

    #[test]
    fn reusable_query_clears_storage_and_normalizes_bounds() {
        let index = EntitySpatialIndex::from_bounds([
            (0, 1, [0.0, 0.0], [2.0, 2.0]),
            (1, 2, [8.0, 8.0], [9.0, 9.0]),
        ]);
        let mut output = vec![usize::MAX; 16];

        index.query_aabb_into([3.0, 3.0], [-1.0, -1.0], &mut output);
        assert_eq!(output, vec![0]);
    }
}
