use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::render::backend::{GraphicsApi, RenderBackendConfig, RenderBackendSelection};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vec3Def {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transform3DDef {
    pub translation: Vec3Def,
    pub rotation_euler: Vec3Def,
    pub scale: Vec3Def,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mesh3D {
    pub name: String,
    pub source: String,
    pub format: String,
    pub primitive: String,
    pub unit_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Material3D {
    pub name: String,
    pub shader: String,
    pub albedo: [f32; 4],
    pub albedo_texture: Option<String>,
    pub normal_map: Option<String>,
    pub metallic: f32,
    pub roughness: f32,
    pub cull_mode: String,
    pub depth_write: bool,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshRenderer3DDef {
    pub mesh: String,
    pub material: String,
    pub visible: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    pub layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Camera3DDef {
    pub active: bool,
    pub projection: String,
    pub position: Vec3Def,
    pub target: Vec3Def,
    pub up: Vec3Def,
    pub fov_y_degrees: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Light3DDef {
    pub light_type: String,
    pub position: Vec3Def,
    pub direction: Vec3Def,
    pub color: [f32; 4],
    pub intensity: f32,
    pub range: f32,
    pub casts_shadows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderGraph3D {
    #[serde(default)]
    pub passes: Vec<RenderPass3D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderPass3D {
    pub name: String,
    pub target: String,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Render3DCompatibilityPlan {
    pub enabled: bool,
    pub backend: GraphicsApi,
    pub hybrid_2d_3d: bool,
    #[serde(default)]
    pub supported_features: Vec<String>,
    #[serde(default)]
    pub deferred_features: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub graph: RenderGraph3D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HybridScene3DStarter {
    pub camera: Camera3DDef,
    #[serde(default)]
    pub lights: Vec<Light3DDef>,
    #[serde(default)]
    pub meshes: Vec<MeshRenderer3DDef>,
    #[serde(default)]
    pub materials: Vec<Material3D>,
}

impl Default for Vec3Def {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Default for Transform3DDef {
    fn default() -> Self {
        Self {
            translation: Vec3Def::default(),
            rotation_euler: Vec3Def::default(),
            scale: Vec3Def {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        }
    }
}

impl Mesh3D {
    pub fn cube() -> Self {
        Self {
            name: "Cube".to_string(),
            source: "builtin:cube".to_string(),
            format: "builtin".to_string(),
            primitive: "cube".to_string(),
            unit_scale: 1.0,
        }
    }

    pub fn billboard_quad() -> Self {
        Self {
            name: "BillboardQuad".to_string(),
            source: "builtin:quad".to_string(),
            format: "builtin".to_string(),
            primitive: "quad".to_string(),
            unit_scale: 1.0,
        }
    }
}

impl Material3D {
    pub fn default_lit(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            shader: "standard_lit_3d".to_string(),
            albedo: [1.0, 1.0, 1.0, 1.0],
            albedo_texture: None,
            normal_map: None,
            metallic: 0.0,
            roughness: 0.75,
            cull_mode: "back".to_string(),
            depth_write: true,
            params: BTreeMap::new(),
        }
    }
}

impl RenderGraph3D {
    pub fn default_hybrid_2d3d(post_processing: bool) -> Self {
        Self {
            passes: vec![
                RenderPass3D {
                    name: "depth_prepass_3d".to_string(),
                    target: "depth".to_string(),
                    reads: Vec::new(),
                    writes: vec!["depth".to_string()],
                    enabled: true,
                },
                RenderPass3D {
                    name: "opaque_3d".to_string(),
                    target: "scene_color".to_string(),
                    reads: vec!["depth".to_string()],
                    writes: vec!["scene_color".to_string()],
                    enabled: true,
                },
                RenderPass3D {
                    name: "sprite_billboards_3d".to_string(),
                    target: "scene_color".to_string(),
                    reads: vec!["depth".to_string()],
                    writes: vec!["scene_color".to_string()],
                    enabled: true,
                },
                RenderPass3D {
                    name: "transparent_3d".to_string(),
                    target: "scene_color".to_string(),
                    reads: vec!["scene_color".to_string(), "depth".to_string()],
                    writes: vec!["scene_color".to_string()],
                    enabled: true,
                },
                RenderPass3D {
                    name: "post_process_3d".to_string(),
                    target: "scene_color".to_string(),
                    reads: vec!["scene_color".to_string()],
                    writes: vec!["scene_color".to_string()],
                    enabled: post_processing,
                },
                RenderPass3D {
                    name: "overlay_2d_ui".to_string(),
                    target: "backbuffer".to_string(),
                    reads: vec!["scene_color".to_string()],
                    writes: vec!["backbuffer".to_string()],
                    enabled: true,
                },
            ],
        }
    }
}

impl Render3DCompatibilityPlan {
    pub fn from_config(config: &RenderBackendConfig) -> Self {
        let selection = RenderBackendSelection::choose(config);
        let mut plan = Self {
            enabled: config.enable_3d,
            backend: selection.selected,
            hybrid_2d_3d: config.hybrid_2d_3d,
            supported_features: Vec::new(),
            deferred_features: Vec::new(),
            warnings: Vec::new(),
            graph: RenderGraph3D::default_hybrid_2d3d(config.post_processing),
        };

        if !config.enable_3d {
            plan.warnings
                .push("3D desactivado; el motor usa el pipeline 2D estable".to_string());
            plan.deferred_features.push("3d_rendering".to_string());
            return plan;
        }

        match selection.selected {
            GraphicsApi::Macroquad => {
                plan.supported_features.extend(
                    [
                        "basic_meshes",
                        "camera_3d",
                        "view_frustum_culling",
                        "backface_culling",
                        "unlit_materials",
                        "debug_primitives",
                        "sprite_billboards",
                        "2d_ui_overlay",
                    ]
                    .into_iter()
                    .map(ToString::to_string),
                );
                plan.deferred_features.extend(
                    [
                        "pbr_materials",
                        "shadow_maps",
                        "skinned_meshes",
                        "gpu_instancing",
                        "3d_particles",
                        "scene_lighting_baker",
                    ]
                    .into_iter()
                    .map(ToString::to_string),
                );
                plan.warnings.push(
                    "Macroquad permite preview 3D basico; proyectos 3D grandes deben esperar al backend wgpu"
                        .to_string(),
                );
            }
            GraphicsApi::OpenGl => {
                plan.supported_features.extend(
                    [
                        "basic_meshes",
                        "camera_3d",
                        "view_frustum_culling",
                        "backface_culling",
                        "unlit_materials",
                        "debug_primitives",
                        "sprite_billboards",
                        "2d_ui_overlay",
                        "plugin_render_preview",
                    ]
                    .into_iter()
                    .map(ToString::to_string),
                );
                plan.deferred_features.extend(
                    [
                        "pbr_materials",
                        "shadow_maps",
                        "skinned_meshes",
                        "gpu_instancing",
                        "3d_particles",
                        "scene_lighting_baker",
                    ]
                    .into_iter()
                    .map(ToString::to_string),
                );
                plan.warnings.push(
                    "OpenGL es una ruta de compatibilidad para tooling/plugins, no el backend principal de shipping"
                        .to_string(),
                );
            }
            GraphicsApi::WgpuMetal
            | GraphicsApi::WgpuVulkan
            | GraphicsApi::WgpuDx12
            | GraphicsApi::WgpuWebGpu => {
                plan.supported_features.extend(
                    [
                        "basic_meshes",
                        "camera_3d",
                        "view_frustum_culling",
                        "occlusion_culling",
                        "lod_selection",
                        "backface_culling",
                        "lit_materials",
                        "depth_buffer",
                        "sprite_billboards",
                        "2d_ui_overlay",
                        "mesh_batching",
                    ]
                    .into_iter()
                    .map(ToString::to_string),
                );
                if config.shadow_maps_3d {
                    plan.supported_features.push("shadow_maps".to_string());
                } else {
                    plan.deferred_features.push("shadow_maps".to_string());
                }
                plan.deferred_features.extend(
                    ["skinned_meshes", "3d_particles", "lighting_baker"]
                        .into_iter()
                        .map(ToString::to_string),
                );
            }
        }

        if config.hybrid_2d_3d {
            plan.supported_features
                .push("2d_gameplay_in_3d_world".to_string());
        }
        plan
    }

    pub fn can_preview_3d(&self) -> bool {
        self.enabled
            && self
                .supported_features
                .iter()
                .any(|item| item == "basic_meshes")
    }

    pub fn is_large_3d_game_ready(&self) -> bool {
        self.enabled
            && !matches!(self.backend, GraphicsApi::Macroquad)
            && self
                .supported_features
                .iter()
                .any(|item| item == "mesh_batching")
            && self
                .deferred_features
                .iter()
                .all(|item| item != "shadow_maps")
    }
}

impl HybridScene3DStarter {
    pub fn minimal() -> Self {
        Self {
            camera: Camera3DDef {
                active: true,
                projection: "perspective".to_string(),
                position: Vec3Def {
                    x: 0.0,
                    y: 4.0,
                    z: 8.0,
                },
                target: Vec3Def::default(),
                up: Vec3Def {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                fov_y_degrees: 60.0,
                near: 0.05,
                far: 500.0,
            },
            lights: vec![Light3DDef {
                light_type: "directional".to_string(),
                position: Vec3Def {
                    x: 0.0,
                    y: 6.0,
                    z: 0.0,
                },
                direction: Vec3Def {
                    x: -0.4,
                    y: -1.0,
                    z: -0.3,
                },
                color: [1.0, 0.96, 0.88, 1.0],
                intensity: 1.0,
                range: 64.0,
                casts_shadows: false,
            }],
            meshes: vec![
                MeshRenderer3DDef {
                    mesh: "builtin:cube".to_string(),
                    material: "Default3D".to_string(),
                    visible: true,
                    cast_shadows: false,
                    receive_shadows: true,
                    layer: "World3D".to_string(),
                },
                MeshRenderer3DDef {
                    mesh: "builtin:quad".to_string(),
                    material: "BillboardSprite".to_string(),
                    visible: true,
                    cast_shadows: false,
                    receive_shadows: false,
                    layer: "Billboard3D".to_string(),
                },
            ],
            materials: vec![
                Material3D::default_lit("Default3D"),
                Material3D {
                    name: "BillboardSprite".to_string(),
                    shader: "sprite_billboard_3d".to_string(),
                    albedo: [1.0, 1.0, 1.0, 1.0],
                    albedo_texture: Some("assets/sprites/player.png".to_string()),
                    normal_map: None,
                    metallic: 0.0,
                    roughness: 1.0,
                    cull_mode: "none".to_string(),
                    depth_write: false,
                    params: BTreeMap::new(),
                },
            ],
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.camera.near <= 0.0 || self.camera.far <= self.camera.near {
            issues.push("Camera3D near/far invalido".to_string());
        }
        if self.meshes.is_empty() {
            issues.push("Escena 3D sin meshes".to_string());
        }
        for material in &self.materials {
            if material.name.trim().is_empty() || material.shader.trim().is_empty() {
                issues.push("Material3D sin nombre o shader".to_string());
            }
        }
        issues
    }
}
