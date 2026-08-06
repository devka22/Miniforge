//! Backend-independent bridge for games that combine 2D gameplay with a 3D presentation.
//!
//! Physics and navigation may remain authoritative in 2D while renderers consume
//! synchronized X/Z positions, elevations and billboard metadata. No project-specific
//! entity types or rendering library are required by this module.

use serde::{Deserialize, Serialize};

use crate::engine::component::default_component;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HybridSyncMode2D3D {
    #[default]
    From2D,
    From3D,
    Manual,
}

impl HybridSyncMode2D3D {
    fn from_name(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "from_3d" | "3d" => Self::From3D,
            "manual" | "none" => Self::Manual,
            _ => Self::From2D,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridWorldSettings2D3D {
    pub enabled: bool,
    pub world_scale: f32,
    pub ground_elevation: f32,
    pub render_2d_overlay: bool,
    pub depth_buffer: bool,
    pub physics_mode: String,
    pub camera_pitch_degrees: f32,
    pub camera_yaw_degrees: f32,
}

impl Default for HybridWorldSettings2D3D {
    fn default() -> Self {
        Self {
            enabled: false,
            world_scale: 1.0,
            ground_elevation: 0.0,
            render_2d_overlay: true,
            depth_buffer: true,
            physics_mode: "2d_gameplay".to_string(),
            camera_pitch_degrees: 58.0,
            camera_yaw_degrees: 35.0,
        }
    }
}

impl HybridWorldSettings2D3D {
    pub fn from_entities(entities: &[GameObject]) -> Self {
        let Some(component) = entities
            .iter()
            .filter(|entity| entity.enabled)
            .find_map(|entity| entity.get_component("HybridScene3D"))
        else {
            return Self::default();
        };
        Self {
            enabled: component.enabled && component.get_bool("enabled", false),
            world_scale: finite_positive(component.get_f64("world_scale", 1.0), 1.0),
            ground_elevation: finite(component.get_f64("ground_elevation", 0.0), 0.0) as f32,
            render_2d_overlay: component.get_bool("render_2d_overlay", true),
            depth_buffer: component.get_bool("depth_buffer", true),
            physics_mode: component.get_string("physics_mode", "2d_gameplay"),
            camera_pitch_degrees: finite(component.get_f64("camera_pitch_degrees", 58.0), 58.0)
                .clamp(-89.0, 89.0) as f32,
            camera_yaw_degrees: finite(component.get_f64("camera_yaw_degrees", 35.0), 35.0) as f32,
        }
    }

    pub fn position_2d_to_3d(&self, position: [f64; 2], elevation: f64) -> [f32; 3] {
        [
            finite(position[0], 0.0) as f32 * self.world_scale,
            (finite(elevation, 0.0) as f32 + self.ground_elevation) * self.world_scale,
            finite(position[1], 0.0) as f32 * self.world_scale,
        ]
    }

    pub fn position_3d_to_2d(&self, position: [f64; 3]) -> [f64; 2] {
        let scale = f64::from(self.world_scale.max(f32::EPSILON));
        [
            finite(position[0], 0.0) / scale,
            finite(position[2], 0.0) / scale,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridEntityPose2D3D {
    pub entity_id: u64,
    pub name: String,
    pub sync_mode: HybridSyncMode2D3D,
    pub position_2d: [f64; 2],
    pub position_3d: [f32; 3],
    pub elevation: f32,
    pub billboard: bool,
    pub billboard_size: [f32; 2],
    pub sprite: Option<String>,
    pub face_camera: bool,
    pub lock_y_axis: bool,
    pub casts_shadow: bool,
    pub receives_shadow: bool,
    pub depth_bias: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HybridFramePlan2D3D {
    pub settings: HybridWorldSettings2D3D,
    pub entities: Vec<HybridEntityPose2D3D>,
    pub billboards: usize,
    pub meshes: usize,
    pub overlays_2d: usize,
}

impl HybridFramePlan2D3D {
    pub fn from_entities(entities: &[GameObject]) -> Self {
        let settings = HybridWorldSettings2D3D::from_entities(entities);
        let mut poses = entities
            .iter()
            .filter(|entity| entity.enabled && entity.visible)
            .filter_map(|entity| extract_pose(entity, &settings))
            .collect::<Vec<_>>();
        poses.sort_by(|left, right| {
            left.position_3d[2]
                .total_cmp(&right.position_3d[2])
                .then_with(|| left.elevation.total_cmp(&right.elevation))
                .then_with(|| left.entity_id.cmp(&right.entity_id))
        });
        let billboards = poses.iter().filter(|pose| pose.billboard).count();
        let meshes = poses.len().saturating_sub(billboards);
        let overlays_2d = entities
            .iter()
            .filter(|entity| {
                entity.enabled
                    && entity.visible
                    && entity.get_component("SpriteRenderer").is_some()
                    && entity.get_component("HybridAnchor2D3D").is_none()
            })
            .count();
        Self {
            settings,
            entities: poses,
            billboards,
            meshes,
            overlays_2d,
        }
    }
}

pub fn sync_entity_hybrid_transform(entity: &mut GameObject, settings: &HybridWorldSettings2D3D) {
    let anchor = entity.get_component("HybridAnchor2D3D");
    let sync_mode = anchor
        .map(|component| {
            HybridSyncMode2D3D::from_name(&component.get_string("sync_mode", "from_2d"))
        })
        .unwrap_or_default();
    let elevation = anchor
        .map(|component| component.get_f64("elevation", 0.0))
        .unwrap_or(0.0);
    match sync_mode {
        HybridSyncMode2D3D::From2D => {
            if entity.get_component("Transform3D").is_none()
                && let Some(transform) = default_component("Transform3D")
            {
                entity.add_component(transform);
            }
            let position = settings.position_2d_to_3d([entity.x, entity.y], elevation);
            if let Some(transform) = entity.get_component_mut("Transform3D") {
                transform.set_f64("x", f64::from(position[0]));
                transform.set_f64("y", f64::from(position[1]));
                transform.set_f64("z", f64::from(position[2]));
            }
        }
        HybridSyncMode2D3D::From3D => {
            let Some(transform) = entity.get_component("Transform3D") else {
                return;
            };
            let position = settings.position_3d_to_2d([
                transform.get_f64("x", 0.0),
                transform.get_f64("y", 0.0),
                transform.get_f64("z", 0.0),
            ]);
            entity.x = position[0];
            entity.y = position[1];
            entity.sync_to_components();
        }
        HybridSyncMode2D3D::Manual => {}
    }
}

fn extract_pose(
    entity: &GameObject,
    settings: &HybridWorldSettings2D3D,
) -> Option<HybridEntityPose2D3D> {
    let anchor = entity.get_component("HybridAnchor2D3D");
    let billboard = entity.get_component("Billboard3D");
    let mesh = entity.get_component("MeshRenderer3D");
    if anchor.is_none() && billboard.is_none() && mesh.is_none() {
        return None;
    }
    let sync_mode = anchor
        .map(|component| {
            HybridSyncMode2D3D::from_name(&component.get_string("sync_mode", "from_2d"))
        })
        .unwrap_or_default();
    let elevation = anchor
        .map(|component| component.get_f64("elevation", 0.0))
        .unwrap_or(0.0);
    let transform = entity.get_component("Transform3D");
    let position_3d = if matches!(
        sync_mode,
        HybridSyncMode2D3D::From3D | HybridSyncMode2D3D::Manual
    ) {
        match transform {
            Some(transform) => [
                finite(transform.get_f64("x", entity.x), entity.x) as f32,
                finite(transform.get_f64("y", elevation), elevation) as f32,
                finite(transform.get_f64("z", entity.y), entity.y) as f32,
            ],
            None => settings.position_2d_to_3d([entity.x, entity.y], elevation),
        }
    } else {
        settings.position_2d_to_3d([entity.x, entity.y], elevation)
    };
    let sprite = billboard
        .and_then(|component| non_empty(component.get_string("sprite", "")))
        .or_else(|| {
            entity
                .get_component("SpriteRenderer")
                .and_then(|component| non_empty(component.get_string("texture_path", "")))
        });
    Some(HybridEntityPose2D3D {
        entity_id: entity.id,
        name: entity.name.clone(),
        sync_mode,
        position_2d: [entity.x, entity.y],
        position_3d,
        elevation: position_3d[1],
        billboard: billboard.is_some(),
        billboard_size: [
            billboard
                .map(|component| finite_positive(component.get_f64("width", entity.width), 1.0))
                .unwrap_or(entity.width.max(0.001) as f32),
            billboard
                .map(|component| finite_positive(component.get_f64("height", entity.height), 1.0))
                .unwrap_or(entity.height.max(0.001) as f32),
        ],
        sprite,
        face_camera: billboard.is_none_or(|component| component.get_bool("face_camera", true)),
        lock_y_axis: billboard.is_some_and(|component| component.get_bool("lock_y_axis", true)),
        casts_shadow: anchor.is_none_or(|component| component.get_bool("casts_shadow", true)),
        receives_shadow: anchor.is_none_or(|component| component.get_bool("receives_shadow", true)),
        depth_bias: anchor
            .map(|component| finite(component.get_f64("depth_bias", 0.0), 0.0) as f32)
            .unwrap_or(0.0),
    })
}

fn finite(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

fn finite_positive(value: f64, fallback: f64) -> f32 {
    finite(value, fallback).max(f64::EPSILON) as f32
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::component::default_component;
    use serde_json::json;

    #[test]
    fn coordinates_round_trip_between_2d_gameplay_and_3d_xz_space() {
        let settings = HybridWorldSettings2D3D {
            world_scale: 2.0,
            ground_elevation: 0.5,
            ..HybridWorldSettings2D3D::default()
        };
        assert_eq!(
            settings.position_2d_to_3d([3.0, -4.0], 1.0),
            [6.0, 3.0, -8.0]
        );
        assert_eq!(settings.position_3d_to_2d([6.0, 3.0, -8.0]), [3.0, -4.0]);
    }

    #[test]
    fn frame_plan_extracts_billboards_and_keeps_deterministic_depth_order() {
        let mut settings_entity = GameObject::new(0.0, 0.0, Some("Hybrid World".to_string()));
        let mut settings_component = default_component("HybridScene3D").unwrap();
        settings_component.enabled = true;
        settings_component.set("enabled", json!(true));
        settings_entity.add_component(settings_component);

        let mut far = GameObject::new(2.0, 8.0, Some("Far actor".to_string()));
        far.add_component(default_component("HybridAnchor2D3D").unwrap());
        far.add_component(default_component("Billboard3D").unwrap());
        far.get_component_mut("SpriteRenderer")
            .unwrap()
            .set("texture_path", json!("assets/far.png"));

        let mut near = GameObject::new(1.0, 2.0, Some("Near mesh".to_string()));
        near.add_component(default_component("HybridAnchor2D3D").unwrap());
        near.add_component(default_component("MeshRenderer3D").unwrap());

        let plan = HybridFramePlan2D3D::from_entities(&[settings_entity, far, near]);
        assert!(plan.settings.enabled);
        assert_eq!(plan.entities.len(), 2);
        assert_eq!(plan.billboards, 1);
        assert_eq!(plan.meshes, 1);
        assert_eq!(plan.entities[0].name, "Near mesh");
        assert_eq!(plan.entities[1].sprite.as_deref(), Some("assets/far.png"));
    }

    #[test]
    fn transform_sync_supports_both_authority_directions() {
        let settings = HybridWorldSettings2D3D {
            world_scale: 2.0,
            ..HybridWorldSettings2D3D::default()
        };
        let mut entity = GameObject::new(3.0, 4.0, Some("Actor".to_string()));
        entity.add_component(default_component("HybridAnchor2D3D").unwrap());
        entity
            .get_component_mut("HybridAnchor2D3D")
            .unwrap()
            .set_f64("elevation", 1.5);
        sync_entity_hybrid_transform(&mut entity, &settings);
        let transform = entity.get_component("Transform3D").unwrap();
        assert_eq!(transform.get_f64("x", 0.0), 6.0);
        assert_eq!(transform.get_f64("y", 0.0), 3.0);
        assert_eq!(transform.get_f64("z", 0.0), 8.0);

        entity
            .get_component_mut("HybridAnchor2D3D")
            .unwrap()
            .set("sync_mode", json!("from_3d"));
        entity
            .get_component_mut("Transform3D")
            .unwrap()
            .set_f64("z", 14.0);
        sync_entity_hybrid_transform(&mut entity, &settings);
        assert_eq!([entity.x, entity.y], [3.0, 7.0]);
    }
}
