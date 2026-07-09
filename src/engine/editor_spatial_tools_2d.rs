use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::vector_canvas_2d::{VectorPath2D, VectorPoint2D, VectorStyle2D};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlignMode2D {
    Left,
    CenterX,
    Right,
    Top,
    CenterY,
    Bottom,
    DistributeX,
    DistributeY,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapAxis2D {
    X,
    Y,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SmartSnapGuide2D {
    pub axis: SnapAxis2D,
    pub value: f64,
    pub source_entity: u64,
    pub target_entity: u64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SmartSnapResult2D {
    pub point: (f64, f64),
    pub snapped_x: bool,
    pub snapped_y: bool,
    #[serde(default)]
    pub guides: Vec<SmartSnapGuide2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SmartSnapSettings2D {
    pub enabled: bool,
    pub grid_enabled: bool,
    pub grid_step: f64,
    pub tolerance_pixels: f64,
    pub snap_centers: bool,
    pub snap_edges: bool,
}

impl Default for SmartSnapSettings2D {
    fn default() -> Self {
        Self {
            enabled: true,
            grid_enabled: true,
            grid_step: 0.25,
            tolerance_pixels: 8.0,
            snap_centers: true,
            snap_edges: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorGroup2D {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub members: BTreeSet<u64>,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorGroupManager2D {
    #[serde(default)]
    pub groups: BTreeMap<String, EditorGroup2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorLayer2D {
    pub name: String,
    pub order: i32,
    pub visible: bool,
    pub locked: bool,
    pub selectable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorLayerManager2D {
    #[serde(default)]
    pub layers: BTreeMap<String, EditorLayer2D>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CameraFrame2D {
    pub aspect_ratio: f32,
    pub safe_margin: f32,
}

impl Default for CameraFrame2D {
    fn default() -> Self {
        Self {
            aspect_ratio: 16.0 / 9.0,
            safe_margin: 0.05,
        }
    }
}

impl CameraFrame2D {
    pub fn fit_inside(&self, viewport: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        let (x, y, width, height) = viewport;
        let viewport_aspect = width / height.max(1.0);
        let (frame_width, frame_height) = if viewport_aspect > self.aspect_ratio {
            (height * self.aspect_ratio, height)
        } else {
            (width, width / self.aspect_ratio.max(0.01))
        };
        (
            x + (width - frame_width) * 0.5,
            y + (height - frame_height) * 0.5,
            frame_width,
            frame_height,
        )
    }

    pub fn safe_rect(&self, viewport: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        let (x, y, width, height) = self.fit_inside(viewport);
        let margin_x = width * self.safe_margin.clamp(0.0, 0.45);
        let margin_y = height * self.safe_margin.clamp(0.0, 0.45);
        (
            x + margin_x,
            y + margin_y,
            width - margin_x * 2.0,
            height - margin_y * 2.0,
        )
    }
}

pub struct EditorSpatialTools2D;

impl EditorSpatialTools2D {
    pub fn smart_snap(
        moving: &GameObject,
        requested: (f64, f64),
        entities: &[GameObject],
        zoom: f64,
        settings: &SmartSnapSettings2D,
    ) -> SmartSnapResult2D {
        let mut point = requested;
        if settings.grid_enabled {
            let step = settings.grid_step.max(0.0001);
            point.0 = (point.0 / step).round() * step;
            point.1 = (point.1 / step).round() * step;
        }
        if !settings.enabled {
            return SmartSnapResult2D {
                point,
                snapped_x: false,
                snapped_y: false,
                guides: Vec::new(),
            };
        }

        let tolerance = settings.tolerance_pixels / (zoom.max(0.05) * 32.0);
        let half_width = (moving.width * moving.scale_x).abs() * 0.5;
        let half_height = (moving.height * moving.scale_y).abs() * 0.5;
        let moving_x = axis_values(point.0, half_width, settings);
        let moving_y = axis_values(point.1, half_height, settings);
        let mut best_x: Option<(f64, f64, u64, &'static str)> = None;
        let mut best_y: Option<(f64, f64, u64, &'static str)> = None;

        for target in entities.iter().filter(|entity| {
            entity.id != moving.id && entity.enabled && entity.visible && !entity.locked
        }) {
            let target_x = axis_values(
                target.x,
                (target.width * target.scale_x).abs() * 0.5,
                settings,
            );
            let target_y = axis_values(
                target.y,
                (target.height * target.scale_y).abs() * 0.5,
                settings,
            );
            compare_axis(&moving_x, &target_x, target.id, tolerance, &mut best_x);
            compare_axis(&moving_y, &target_y, target.id, tolerance, &mut best_y);
        }

        let mut guides = Vec::new();
        if let Some((delta, value, target, label)) = best_x {
            point.0 += delta;
            guides.push(SmartSnapGuide2D {
                axis: SnapAxis2D::X,
                value,
                source_entity: moving.id,
                target_entity: target,
                label: label.to_string(),
            });
        }
        if let Some((delta, value, target, label)) = best_y {
            point.1 += delta;
            guides.push(SmartSnapGuide2D {
                axis: SnapAxis2D::Y,
                value,
                source_entity: moving.id,
                target_entity: target,
                label: label.to_string(),
            });
        }
        SmartSnapResult2D {
            point,
            snapped_x: best_x.is_some(),
            snapped_y: best_y.is_some(),
            guides,
        }
    }

    pub fn align(entities: &mut [GameObject], selected_ids: &[u64], mode: AlignMode2D) -> Vec<u64> {
        let indices = selected_indices(entities, selected_ids);
        if indices.len() < 2 {
            return Vec::new();
        }
        match mode {
            AlignMode2D::DistributeX => distribute(entities, &indices, true),
            AlignMode2D::DistributeY => distribute(entities, &indices, false),
            _ => align_edges(entities, &indices, mode),
        }
        indices
            .into_iter()
            .map(|index| {
                entities[index].sync_to_components();
                entities[index].id
            })
            .collect()
    }

    pub fn set_pivot(
        entity: &mut GameObject,
        normalized: (f64, f64),
        preserve_visual_position: bool,
    ) -> bool {
        let old = Self::pivot(entity);
        let next = (normalized.0.clamp(0.0, 1.0), normalized.1.clamp(0.0, 1.0));
        if old == next {
            return false;
        }
        if preserve_visual_position {
            entity.x += (next.0 - old.0) * entity.width * entity.scale_x;
            entity.y += (next.1 - old.1) * entity.height * entity.scale_y;
        }
        if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
            sprite.set_f64("pivot_x", next.0);
            sprite.set_f64("pivot_y", next.1);
        }
        entity.sync_to_components();
        true
    }

    pub fn pivot(entity: &GameObject) -> (f64, f64) {
        entity
            .get_component("SpriteRenderer")
            .map(|sprite| {
                (
                    sprite.get_f64("pivot_x", 0.5),
                    sprite.get_f64("pivot_y", 0.5),
                )
            })
            .unwrap_or((0.5, 0.5))
    }

    pub fn collision_points(entity: &GameObject) -> Vec<(f64, f64)> {
        entity
            .get_component("Collider2D")
            .and_then(|collider| collider.get("points"))
            .and_then(Value::as_array)
            .map(|points| {
                points
                    .iter()
                    .filter_map(|point| {
                        let point = point.as_array()?;
                        Some((point.first()?.as_f64()?, point.get(1)?.as_f64()?))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn move_collision_vertex(
        entity: &mut GameObject,
        index: usize,
        local: (f64, f64),
        snap_step: Option<f64>,
    ) -> bool {
        let mut points = Self::collision_points(entity);
        let Some(point) = points.get_mut(index) else {
            return false;
        };
        let mut local = local;
        if let Some(step) = snap_step.filter(|step| *step > f64::EPSILON) {
            local.0 = (local.0 / step).round() * step;
            local.1 = (local.1 / step).round() * step;
        }
        *point = local;
        if let Some(collider) = entity.get_component_mut("Collider2D") {
            collider.set("shape", json!("polygon"));
            collider.set("points", json!(points));
            true
        } else {
            false
        }
    }

    pub fn add_collision_vertex(entity: &mut GameObject, local: (f64, f64)) -> bool {
        let mut points = Self::collision_points(entity);
        points.push(local);
        if let Some(collider) = entity.get_component_mut("Collider2D") {
            collider.set("shape", json!("polygon"));
            collider.set("points", json!(points));
            true
        } else {
            false
        }
    }

    pub fn remove_collision_vertex(entity: &mut GameObject, index: usize) -> bool {
        let mut points = Self::collision_points(entity);
        if points.len() <= 3 || index >= points.len() {
            return false;
        }
        points.remove(index);
        if let Some(collider) = entity.get_component_mut("Collider2D") {
            collider.set("points", json!(points));
            true
        } else {
            false
        }
    }

    pub fn collision_path(entity: &GameObject, style: VectorStyle2D) -> VectorPath2D {
        let Some(collider) = entity.get_component("Collider2D") else {
            return VectorPath2D::new(style);
        };
        let offset = (
            collider.get_f64("offset_x", 0.0),
            collider.get_f64("offset_y", 0.0),
        );
        if collider
            .get("shape")
            .and_then(Value::as_str)
            .is_some_and(|shape| shape == "circle")
        {
            let center = local_to_world(entity, offset);
            return VectorPath2D::circle(
                VectorPoint2D::new(center.0 as f32, center.1 as f32),
                (collider.get_f64("radius", entity.radius) * entity.scale_x.abs()) as f32,
                style,
            );
        }
        let points = Self::collision_points(entity);
        let points = if points.is_empty() {
            vec![
                (-entity.width * 0.5, -entity.height * 0.5),
                (entity.width * 0.5, -entity.height * 0.5),
                (entity.width * 0.5, entity.height * 0.5),
                (-entity.width * 0.5, entity.height * 0.5),
            ]
        } else {
            points
        };
        let points = points
            .into_iter()
            .map(|point| local_to_world(entity, (point.0 + offset.0, point.1 + offset.1)))
            .map(|point| VectorPoint2D::new(point.0 as f32, point.1 as f32))
            .collect::<Vec<_>>();
        VectorPath2D::polygon(&points, style)
    }
}

impl EditorGroupManager2D {
    pub fn group_entities(
        &mut self,
        entities: &mut [GameObject],
        selected_ids: &[u64],
        label: impl Into<String>,
    ) -> Option<String> {
        if selected_ids.len() < 2 {
            return None;
        }
        let label = label.into();
        let id = format!("group_{}", stable_text_hash(&label));
        let members = selected_ids.iter().copied().collect::<BTreeSet<_>>();
        for entity in entities
            .iter_mut()
            .filter(|entity| members.contains(&entity.id))
        {
            entity.editor_group = Some(id.clone());
        }
        self.groups.insert(
            id.clone(),
            EditorGroup2D {
                id: id.clone(),
                label,
                members,
                locked: false,
                visible: true,
            },
        );
        Some(id)
    }

    pub fn dissolve(&mut self, entities: &mut [GameObject], group_id: &str) -> bool {
        let Some(group) = self.groups.remove(group_id) else {
            return false;
        };
        for entity in entities
            .iter_mut()
            .filter(|entity| group.members.contains(&entity.id))
        {
            entity.editor_group = None;
        }
        true
    }

    pub fn selection_for(&self, group_id: &str) -> Vec<u64> {
        self.groups
            .get(group_id)
            .map(|group| group.members.iter().copied().collect())
            .unwrap_or_default()
    }
}

impl EditorLayerManager2D {
    pub fn from_entities(entities: &[GameObject]) -> Self {
        let names = entities
            .iter()
            .map(|entity| entity.layer.clone())
            .collect::<BTreeSet<_>>();
        Self {
            layers: names
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                    (
                        name.clone(),
                        EditorLayer2D {
                            name,
                            order: index as i32,
                            visible: true,
                            locked: false,
                            selectable: true,
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn apply(&self, entities: &mut [GameObject]) {
        for entity in entities {
            if let Some(layer) = self.layers.get(&entity.layer) {
                entity.visible = layer.visible;
                entity.locked = layer.locked || !layer.selectable;
            }
        }
    }

    pub fn set_visible(&mut self, name: &str, visible: bool) -> bool {
        let Some(layer) = self.layers.get_mut(name) else {
            return false;
        };
        layer.visible = visible;
        true
    }

    pub fn set_locked(&mut self, name: &str, locked: bool) -> bool {
        let Some(layer) = self.layers.get_mut(name) else {
            return false;
        };
        layer.locked = locked;
        true
    }
}

fn axis_values(
    center: f64,
    half_extent: f64,
    settings: &SmartSnapSettings2D,
) -> Vec<(f64, &'static str)> {
    let mut values = Vec::new();
    if settings.snap_centers {
        values.push((center, "center"));
    }
    if settings.snap_edges {
        values.push((center - half_extent, "near edge"));
        values.push((center + half_extent, "far edge"));
    }
    values
}

fn compare_axis(
    moving: &[(f64, &'static str)],
    target: &[(f64, &'static str)],
    target_id: u64,
    tolerance: f64,
    best: &mut Option<(f64, f64, u64, &'static str)>,
) {
    for (moving_value, _) in moving {
        for (target_value, label) in target {
            let delta = target_value - moving_value;
            if delta.abs() <= tolerance
                && best
                    .as_ref()
                    .is_none_or(|current| delta.abs() < current.0.abs())
            {
                *best = Some((delta, *target_value, target_id, label));
            }
        }
    }
}

fn selected_indices(entities: &[GameObject], selected_ids: &[u64]) -> Vec<usize> {
    let selected = selected_ids.iter().copied().collect::<BTreeSet<_>>();
    entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| selected.contains(&entity.id) && !entity.locked)
        .map(|(index, _)| index)
        .collect()
}

fn align_edges(entities: &mut [GameObject], indices: &[usize], mode: AlignMode2D) {
    let reference = &entities[indices[0]];
    let left = reference.x - reference.width * reference.scale_x.abs() * 0.5;
    let right = reference.x + reference.width * reference.scale_x.abs() * 0.5;
    let top = reference.y - reference.height * reference.scale_y.abs() * 0.5;
    let bottom = reference.y + reference.height * reference.scale_y.abs() * 0.5;
    let center_x = reference.x;
    let center_y = reference.y;
    for index in indices.iter().copied().skip(1) {
        let entity = &mut entities[index];
        match mode {
            AlignMode2D::Left => entity.x = left + entity.width * entity.scale_x.abs() * 0.5,
            AlignMode2D::CenterX => entity.x = center_x,
            AlignMode2D::Right => entity.x = right - entity.width * entity.scale_x.abs() * 0.5,
            AlignMode2D::Top => entity.y = top + entity.height * entity.scale_y.abs() * 0.5,
            AlignMode2D::CenterY => entity.y = center_y,
            AlignMode2D::Bottom => entity.y = bottom - entity.height * entity.scale_y.abs() * 0.5,
            AlignMode2D::DistributeX | AlignMode2D::DistributeY => {}
        }
    }
}

fn distribute(entities: &mut [GameObject], indices: &[usize], horizontal: bool) {
    if indices.len() < 3 {
        return;
    }
    let mut ordered = indices.to_vec();
    ordered.sort_by(|left, right| {
        let left_value = if horizontal {
            entities[*left].x
        } else {
            entities[*left].y
        };
        let right_value = if horizontal {
            entities[*right].x
        } else {
            entities[*right].y
        };
        left_value
            .partial_cmp(&right_value)
            .unwrap_or(Ordering::Equal)
    });
    let first = if horizontal {
        entities[ordered[0]].x
    } else {
        entities[ordered[0]].y
    };
    let last = if horizontal {
        entities[*ordered.last().unwrap()].x
    } else {
        entities[*ordered.last().unwrap()].y
    };
    let step = (last - first) / (ordered.len() - 1) as f64;
    for (order, index) in ordered.into_iter().enumerate().skip(1) {
        if order + 1 == indices.len() {
            continue;
        }
        if horizontal {
            entities[index].x = first + step * order as f64;
        } else {
            entities[index].y = first + step * order as f64;
        }
    }
}

fn local_to_world(entity: &GameObject, local: (f64, f64)) -> (f64, f64) {
    let x = local.0 * entity.scale_x;
    let y = local.1 * entity.scale_y;
    let radians = entity.rotation.to_radians();
    (
        entity.x + x * radians.cos() - y * radians.sin(),
        entity.y + x * radians.sin() + y * radians.cos(),
    )
}

fn stable_text_hash(text: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

const fn default_true() -> bool {
    true
}
