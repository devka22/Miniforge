use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::render::backend::{
    GraphicsApi, ParticleDrawCommand, RenderBackendConfig, RenderBackendSelection,
    RenderDeviceCaps, SpriteDrawCommand, TilemapDrawCommand,
};

mod texture_atlas;

pub use texture_atlas::{
    DynamicTextureAtlas2D, SpriteAtlasExportManifest2D, SpriteAtlasExportOptions2D,
    SpriteAtlasExportReport2D, TextureAtlasError2D, TextureAtlasStats2D,
    build_texture_atlas_from_files, export_sprite_atlas_pages_from_files,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderPipelineCache2D {
    #[serde(default)]
    pub pipelines: BTreeMap<String, PipelineState2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineState2D {
    pub shader: String,
    pub blend_mode: String,
    pub depth_enabled: bool,
    pub samples: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextureAtlas2D {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub regions: BTreeMap<String, AtlasRegion2D>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasRegion2D {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub extrude: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderGraph2D {
    #[serde(default)]
    pub passes: Vec<RenderPass2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderPass2D {
    pub name: String,
    pub target: String,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BatchStats2D {
    pub sprite_count: usize,
    pub sprite_batches: usize,
    pub tile_chunks: usize,
    pub particle_count: usize,
    pub ui_widgets: usize,
    pub draw_calls_estimate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteBatcher {
    pub enabled: bool,
    pub max_sprites_per_batch: usize,
    #[serde(default)]
    pub pending: Vec<SpriteDrawCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TilemapChunkRenderer {
    pub enabled: bool,
    pub chunk_width: u32,
    pub chunk_height: u32,
    #[serde(default)]
    pub visible_chunks: Vec<(i32, i32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Material2D {
    pub name: String,
    pub shader: String,
    pub texture: String,
    pub normal_map: Option<String>,
    pub blend_mode: String,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Shader2D {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub uniforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostProcessStack2D {
    pub enabled: bool,
    #[serde(default)]
    pub effects: Vec<PostProcessEffect2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostProcessEffect2D {
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RenderEffectKind2D {
    Lighting,
    Material,
    PostProcess,
    Particles,
    Damage,
    PixelArt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderEffectPreset2D {
    pub id: String,
    pub label: String,
    pub kind: RenderEffectKind2D,
    pub component: String,
    pub shader: String,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
    #[serde(default)]
    pub required_buffers: Vec<String>,
    pub gpu_preferred: bool,
    pub fallback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderTexture2D {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraStack2D {
    #[serde(default)]
    pub cameras: Vec<CameraLayer2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraLayer2D {
    pub name: String,
    pub order: i32,
    pub clear_color: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Light2D {
    pub light_type: String,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub intensity: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LightingWorld2D {
    #[serde(default)]
    pub ambient: [f32; 4],
    #[serde(default)]
    pub lights: Vec<Light2D>,
    #[serde(default)]
    pub shadow_casters: Vec<ShadowCaster2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowCaster2D {
    pub entity_id: u64,
    #[serde(default)]
    pub points: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalMap2D {
    pub texture: String,
    pub normal_map: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleRenderer2D {
    pub enabled: bool,
    #[serde(default)]
    pub pending: Vec<ParticleDrawCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Decal2D {
    pub material: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrailRenderer2D {
    pub material: String,
    pub lifetime: f32,
    #[serde(default)]
    pub points: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineRenderer2D {
    pub width: f32,
    pub color: [f32; 4],
    #[serde(default)]
    pub points: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DebugRenderer2D {
    #[serde(default)]
    pub lines: Vec<LineRenderer2D>,
    #[serde(default)]
    pub labels: Vec<(String, f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComputePipelineKind2D {
    GpuParticles,
    TileVisibility,
    FlowField,
    Lighting,
    PostProcess,
    UiLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeJob2D {
    pub name: String,
    pub kind: ComputePipelineKind2D,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
    pub workgroups: [u32; 3],
    pub enabled: bool,
    pub fallback_pass: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetalFramePlan2D {
    pub backend: String,
    pub memoryless_targets: bool,
    #[serde(default)]
    pub compute_jobs: Vec<ComputeJob2D>,
    pub render_graph: RenderGraph2D,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Render2DCompatibilityProfile {
    pub api: GraphicsApi,
    pub backend: String,
    pub shader_language: String,
    pub preferred_texture_format: String,
    pub atlas_page_size: u32,
    pub max_visible_sprites: usize,
    pub max_sprites_per_batch: usize,
    pub recommended_tile_chunk_size: [u32; 2],
    pub supports_compute: bool,
    pub supports_gpu_particles: bool,
    pub supports_tile_compute_culling: bool,
    pub supports_persistent_buffers: bool,
    pub huge_world_ready: bool,
    #[serde(default)]
    pub fallbacks: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl Default for SpriteBatcher {
    fn default() -> Self {
        Self {
            enabled: true,
            max_sprites_per_batch: 2048,
            pending: Vec::new(),
        }
    }
}

impl SpriteBatcher {
    pub fn push(&mut self, command: SpriteDrawCommand) {
        self.pending.push(command);
    }

    pub fn batches(&self) -> usize {
        if !self.enabled || self.pending.is_empty() {
            return self.pending.len();
        }
        self.pending
            .len()
            .div_ceil(self.max_sprites_per_batch.max(1))
    }

    pub fn stats(&self) -> BatchStats2D {
        BatchStats2D {
            sprite_count: self.pending.len(),
            sprite_batches: self.batches(),
            draw_calls_estimate: self.batches(),
            ..Default::default()
        }
    }
}

impl Default for TilemapChunkRenderer {
    fn default() -> Self {
        Self {
            enabled: true,
            chunk_width: 32,
            chunk_height: 32,
            visible_chunks: Vec::new(),
        }
    }
}

impl TilemapChunkRenderer {
    pub fn command_for_chunk(
        &self,
        tilemap_id: u64,
        layer: impl Into<String>,
        chunk: (i32, i32),
        tile_count: usize,
    ) -> TilemapDrawCommand {
        TilemapDrawCommand {
            tilemap_id,
            layer: layer.into(),
            chunk_x: chunk.0,
            chunk_y: chunk.1,
            tile_count,
        }
    }
}

impl Default for PostProcessStack2D {
    fn default() -> Self {
        Self {
            enabled: true,
            effects: vec![
                effect_with_params(
                    "Bloom",
                    &[("threshold", 0.8), ("intensity", 0.7), ("radius", 4.0)],
                ),
                effect("CRT"),
                effect("Vignette"),
                effect("ColorGrading"),
                effect_with_params(
                    "Pixelate",
                    &[
                        ("pixel_scale", 1.0),
                        ("palette_size", 16.0),
                        ("dither", 1.0),
                    ],
                ),
                effect("ChromaticAberration"),
                effect("Blur"),
                effect("ScreenShake"),
                effect_with_params(
                    "DamageFlash",
                    &[("duration", 0.12), ("intensity", 1.0), ("vignette", 0.3)],
                ),
                effect("NightVision"),
                effect_with_params(
                    "Underwater",
                    &[("refraction", 0.08), ("wave_speed", 1.2), ("foam", 0.4)],
                ),
                effect_with_params(
                    "HeatDistortion",
                    &[("strength", 0.06), ("speed", 1.0), ("frequency", 2.5)],
                ),
                effect_with_params(
                    "Fog",
                    &[("density", 0.18), ("height_falloff", 0.35), ("noise", 0.3)],
                ),
                effect_with_params(
                    "Outline",
                    &[("width", 1.0), ("threshold", 0.1), ("inside", 0.0)],
                ),
                effect("RainOverlay"),
            ],
        }
    }
}

impl Material2D {
    pub fn water_example() -> Self {
        Self {
            name: "Water2D".to_string(),
            shader: "water_wave".to_string(),
            texture: "water.png".to_string(),
            normal_map: Some("water_normal.png".to_string()),
            blend_mode: "alpha".to_string(),
            params: BTreeMap::from([
                ("wave_strength".to_string(), 0.15),
                ("wave_speed".to_string(), 1.2),
                ("foam_amount".to_string(), 0.4),
            ]),
        }
    }

    pub fn fire_example() -> Self {
        Self {
            name: "Fire2D".to_string(),
            shader: "fire_2d".to_string(),
            texture: "fire_noise.png".to_string(),
            normal_map: None,
            blend_mode: "additive".to_string(),
            params: BTreeMap::from([
                ("speed".to_string(), 1.4),
                ("distortion".to_string(), 0.08),
                ("emission".to_string(), 2.0),
            ]),
        }
    }

    pub fn pixel_art_example() -> Self {
        Self {
            name: "PixelArt2D".to_string(),
            shader: "pixel_art_2d".to_string(),
            texture: "sprite.png".to_string(),
            normal_map: None,
            blend_mode: "alpha".to_string(),
            params: BTreeMap::from([
                ("palette_size".to_string(), 16.0),
                ("dither".to_string(), 1.0),
                ("pixel_scale".to_string(), 1.0),
            ]),
        }
    }

    pub fn distortion_example() -> Self {
        Self {
            name: "HeatDistortion2D".to_string(),
            shader: "distortion_2d".to_string(),
            texture: "noise.png".to_string(),
            normal_map: None,
            blend_mode: "alpha".to_string(),
            params: BTreeMap::from([
                ("strength".to_string(), 0.06),
                ("speed".to_string(), 1.0),
                ("frequency".to_string(), 2.5),
            ]),
        }
    }

    pub fn to_value(&self) -> Value {
        json!(self)
    }
}

pub fn production_effect_presets_2d() -> Vec<RenderEffectPreset2D> {
    vec![
        preset(
            "light",
            "2D Light",
            RenderEffectKind2D::Lighting,
            "Light2D",
            "light_2d",
            &[("radius", 5.0), ("intensity", 1.0)],
            &["normal_map", "occluders"],
            true,
        ),
        preset(
            "shadow",
            "Soft Shadows",
            RenderEffectKind2D::Lighting,
            "ShadowCaster2D",
            "shadow_2d",
            &[("softness", 0.35), ("bias", 0.01)],
            &["occluders"],
            true,
        ),
        preset(
            "normal_map",
            "Sprite Normal Map",
            RenderEffectKind2D::Lighting,
            "NormalMap2D",
            "sprite_lit_2d",
            &[("strength", 1.0)],
            &["normal_map"],
            false,
        ),
        preset(
            "water",
            "Water and Refraction",
            RenderEffectKind2D::Material,
            "Water2D",
            "water_2d",
            &[("wave_strength", 0.15), ("refraction", 0.08)],
            &["scene_color"],
            true,
        ),
        preset(
            "distortion",
            "Heat Distortion",
            RenderEffectKind2D::PostProcess,
            "Distortion2D",
            "distortion_2d",
            &[("strength", 0.06), ("speed", 1.0)],
            &["scene_color"],
            true,
        ),
        preset(
            "fire",
            "Fire",
            RenderEffectKind2D::Particles,
            "Fire2D",
            "fire_2d",
            &[("emission", 2.0), ("rate", 64.0)],
            &["particle_state"],
            true,
        ),
        preset(
            "fog",
            "Layered Fog",
            RenderEffectKind2D::PostProcess,
            "Fog2D",
            "fog_2d",
            &[("density", 0.18), ("noise", 0.3)],
            &["scene_color", "depth_or_layers"],
            true,
        ),
        preset(
            "outline",
            "Sprite Outline",
            RenderEffectKind2D::PostProcess,
            "Outline2D",
            "outline_2d",
            &[("width", 1.0), ("threshold", 0.1)],
            &["scene_color"],
            false,
        ),
        preset(
            "bloom",
            "Bloom",
            RenderEffectKind2D::PostProcess,
            "Bloom2D",
            "bloom_2d",
            &[("threshold", 0.8), ("intensity", 0.7)],
            &["scene_color", "emissive"],
            true,
        ),
        preset(
            "gpu_particles",
            "WGPU Compute Particles",
            RenderEffectKind2D::Particles,
            "GpuParticles2D",
            "particles_compute_2d",
            &[
                ("max_particles", 8192.0),
                ("emission_rate", 128.0),
                ("lifetime", 1.25),
            ],
            &["persistent_storage", "compute_dispatch", "cpu_fallback"],
            true,
        ),
        preset(
            "damage",
            "Damage Feedback",
            RenderEffectKind2D::Damage,
            "DamageEffect2D",
            "damage_feedback_2d",
            &[("flash_duration", 0.12), ("shake", 5.0)],
            &["scene_color"],
            false,
        ),
        preset(
            "pixel_art",
            "Pixel Art Shader",
            RenderEffectKind2D::PixelArt,
            "PixelArtShader2D",
            "pixel_art_2d",
            &[("palette_size", 16.0), ("dither", 1.0)],
            &["scene_color"],
            false,
        ),
    ]
}

#[allow(clippy::too_many_arguments, reason = "declarative render preset table")]
fn preset(
    id: &str,
    label: &str,
    kind: RenderEffectKind2D,
    component: &str,
    shader: &str,
    params: &[(&str, f64)],
    required_buffers: &[&str],
    gpu_preferred: bool,
) -> RenderEffectPreset2D {
    RenderEffectPreset2D {
        id: id.to_string(),
        label: label.to_string(),
        kind,
        component: component.to_string(),
        shader: shader.to_string(),
        params: params
            .iter()
            .map(|(name, value)| ((*name).to_string(), *value))
            .collect(),
        required_buffers: required_buffers
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        gpu_preferred,
        fallback: if gpu_preferred {
            "cpu_or_sprite_fallback".to_string()
        } else {
            "sprite_material".to_string()
        },
    }
}

impl RenderPipelineCache2D {
    pub fn pipeline_for_material(&mut self, material: &Material2D, samples: u32) -> String {
        let key = format!("{}:{}:{samples}", material.shader, material.blend_mode);
        self.pipelines
            .entry(key.clone())
            .or_insert(PipelineState2D {
                shader: material.shader.clone(),
                blend_mode: material.blend_mode.clone(),
                depth_enabled: false,
                samples,
            });
        key
    }
}

impl TextureAtlas2D {
    pub fn add_region(&mut self, name: impl Into<String>, region: AtlasRegion2D) -> bool {
        let name = name.into();
        let Some(right) = region.x.checked_add(region.width) else {
            return false;
        };
        let Some(bottom) = region.y.checked_add(region.height) else {
            return false;
        };
        if region.width == 0
            || region.height == 0
            || right > self.width
            || bottom > self.height
            || self.regions.iter().any(|(existing_name, existing)| {
                existing_name != &name && regions_overlap(region, *existing)
            })
        {
            return false;
        }
        self.regions.insert(name, region);
        true
    }

    pub fn uv_rect(&self, name: &str) -> Option<[f32; 4]> {
        let region = self.regions.get(name)?;
        Some([
            region.x as f32 / self.width.max(1) as f32,
            region.y as f32 / self.height.max(1) as f32,
            (region.x + region.width) as f32 / self.width.max(1) as f32,
            (region.y + region.height) as f32 / self.height.max(1) as f32,
        ])
    }
}

fn regions_overlap(a: AtlasRegion2D, b: AtlasRegion2D) -> bool {
    let a_right = a.x.saturating_add(a.width);
    let a_bottom = a.y.saturating_add(a.height);
    let b_right = b.x.saturating_add(b.width);
    let b_bottom = b.y.saturating_add(b.height);
    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

impl RenderGraph2D {
    pub fn default_2d(post_processing: bool) -> Self {
        let mut passes = vec![
            pass("clear", "scene_color", &[], &["scene_color"]),
            pass("tilemaps", "scene_color", &["tiles"], &["scene_color"]),
            pass("sprites", "scene_color", &["sprites"], &["scene_color"]),
            pass("particles", "scene_color", &["particles"], &["scene_color"]),
            pass(
                "lighting",
                "scene_light",
                &["scene_color", "normal_map"],
                &["scene_light"],
            ),
            pass("ui", "backbuffer", &["scene_light", "ui"], &["backbuffer"]),
        ];
        if post_processing {
            passes.insert(
                5,
                pass(
                    "post_process",
                    "scene_post",
                    &["scene_color", "scene_light"],
                    &["scene_post"],
                ),
            );
        }
        Self { passes }
    }
}

impl LightingWorld2D {
    pub fn estimate_light_cost(&self) -> usize {
        self.lights.len() + self.shadow_casters.len() * self.lights.len()
    }
}

impl MetalFramePlan2D {
    pub fn from_config(
        config: &RenderBackendConfig,
        graph: RenderGraph2D,
        sprite_count: usize,
        visible_tile_chunks: usize,
        particle_count: usize,
    ) -> Self {
        let mut plan = Self {
            backend: config.backend.clone(),
            memoryless_targets: config.metal.use_memoryless_targets,
            compute_jobs: Vec::new(),
            render_graph: graph,
            warnings: Vec::new(),
        };

        if config.gpu_particles || config.metal.allow_compute_particles {
            plan.compute_jobs.push(ComputeJob2D {
                name: "gpu_particle_simulation".to_string(),
                kind: ComputePipelineKind2D::GpuParticles,
                reads: vec![
                    "particle_state_in".to_string(),
                    "emitter_params".to_string(),
                ],
                writes: vec![
                    "particle_state_out".to_string(),
                    "particle_indirect_draw".to_string(),
                ],
                workgroups: workgroups_for_items(particle_count.max(1), 64),
                enabled: config.experimental_wgpu && config.metal.allow_compute_particles,
                fallback_pass: "particles".to_string(),
            });
        }

        if config.tilemap_chunk_batching || config.metal.allow_compute_tile_visibility {
            plan.compute_jobs.push(ComputeJob2D {
                name: "tile_visibility_cull".to_string(),
                kind: ComputePipelineKind2D::TileVisibility,
                reads: vec!["tile_chunks".to_string(), "camera_bounds".to_string()],
                writes: vec!["visible_tile_chunks".to_string()],
                workgroups: workgroups_for_items(visible_tile_chunks.max(1), 32),
                enabled: config.experimental_wgpu && config.metal.allow_compute_tile_visibility,
                fallback_pass: "tilemaps".to_string(),
            });
        }

        if config.metal.allow_compute_flow_fields {
            plan.compute_jobs.push(ComputeJob2D {
                name: "flow_field_update".to_string(),
                kind: ComputePipelineKind2D::FlowField,
                reads: vec![
                    "navigation_grid".to_string(),
                    "dynamic_obstacles".to_string(),
                ],
                writes: vec!["flow_vectors".to_string()],
                workgroups: workgroups_for_items(visible_tile_chunks.max(sprite_count).max(1), 64),
                enabled: config.experimental_wgpu,
                fallback_pass: "physics".to_string(),
            });
        }

        if config.post_processing {
            plan.compute_jobs.push(ComputeJob2D {
                name: "post_process_prefilter".to_string(),
                kind: ComputePipelineKind2D::PostProcess,
                reads: vec!["scene_color".to_string()],
                writes: vec!["scene_post".to_string()],
                workgroups: [16, 16, 1],
                enabled: config.experimental_wgpu && config.backend.eq_ignore_ascii_case("wgpu"),
                fallback_pass: "post_process".to_string(),
            });
        }

        if !config.experimental_wgpu
            && plan
                .compute_jobs
                .iter()
                .any(|job| matches!(job.kind, ComputePipelineKind2D::GpuParticles))
        {
            plan.warnings.push(
                "gpu_particles solicitado, pero experimental_wgpu=false; usando CPU/fallback"
                    .to_string(),
            );
        }
        if !plan.has_enabled_compute() && !plan.compute_jobs.is_empty() {
            plan.warnings
                .push("compute plan listo, pero todos los jobs usan fallback estable".to_string());
        }
        plan
    }

    pub fn has_enabled_compute(&self) -> bool {
        self.compute_jobs.iter().any(|job| job.enabled)
    }

    pub fn fallback_passes(&self) -> Vec<String> {
        let mut passes = self
            .compute_jobs
            .iter()
            .filter(|job| !job.enabled)
            .map(|job| job.fallback_pass.clone())
            .collect::<Vec<_>>();
        passes.sort();
        passes.dedup();
        passes
    }
}

impl Render2DCompatibilityProfile {
    pub fn from_config(
        config: &RenderBackendConfig,
        expected_visible_sprites: usize,
        expected_visible_tile_chunks: usize,
    ) -> Self {
        let selection = RenderBackendSelection::choose(config);
        let caps = caps_for_selection(selection.selected);
        let mut warnings = Vec::new();
        let mut fallbacks = Vec::new();
        let backend = config.backend.clone();

        let (
            shader_language,
            atlas_page_size,
            max_visible_sprites,
            max_sprites_per_batch,
            recommended_tile_chunk_size,
            supports_persistent_buffers,
        ) = match selection.selected {
            GraphicsApi::WgpuMetal => (
                "WGSL -> Metal".to_string(),
                caps.max_texture_size
                    .min(config.max_texture_size)
                    .min(16384),
                150_000,
                8192,
                [64, 64],
                true,
            ),
            GraphicsApi::OpenGl => {
                warnings.push(
                    "OpenGL compatibility is intended as a stable tooling/plugin fallback for 2D."
                        .to_string(),
                );
                fallbacks
                    .push("Use CPU particle simulation when compute is unavailable".to_string());
                (
                    "GLSL 330 / GLSL ES compatible".to_string(),
                    caps.max_texture_size.min(config.max_texture_size).min(4096),
                    35_000,
                    2048,
                    [32, 32],
                    false,
                )
            }
            GraphicsApi::Macroquad => (
                "macroquad GLSL".to_string(),
                caps.max_texture_size.min(config.max_texture_size).min(8192),
                if config.sprite_batching {
                    60_000
                } else {
                    12_000
                },
                if config.sprite_batching { 4096 } else { 512 },
                [32, 32],
                false,
            ),
            GraphicsApi::WgpuVulkan | GraphicsApi::WgpuDx12 | GraphicsApi::WgpuWebGpu => (
                "WGSL".to_string(),
                caps.max_texture_size.min(config.max_texture_size).min(8192),
                100_000,
                4096,
                [64, 64],
                true,
            ),
        };

        let supports_compute = caps.supports_compute && config.experimental_wgpu;
        let supports_gpu_particles = supports_compute && config.gpu_particles;
        let supports_tile_compute_culling = supports_compute && config.tilemap_chunk_batching;
        if expected_visible_sprites > max_visible_sprites {
            warnings.push(format!(
                "Visible sprite target {expected_visible_sprites} exceeds recommended budget {max_visible_sprites}; use streaming/LOD/atlases."
            ));
        }
        if !config.view_frustum_culling {
            warnings.push(
                "View frustum culling disabled; off-camera sprites and chunks can waste frame time."
                    .to_string(),
            );
        }
        if !config.occlusion_culling && expected_visible_sprites > 10_000 {
            warnings.push(
                "Occlusion culling disabled for a dense scene; large occluders should hide covered entities."
                    .to_string(),
            );
        }
        if !config.lod_enabled && expected_visible_sprites > max_visible_sprites / 2 {
            warnings.push(
                "LOD disabled for a dense scene; distant sprites will keep full detail cost."
                    .to_string(),
            );
        }
        if expected_visible_tile_chunks > 4096 && !supports_tile_compute_culling {
            warnings.push(
                "High visible tile chunk count without compute culling; reduce chunk radius or enable Metal/wgpu path."
                    .to_string(),
            );
        }
        if config.gpu_particles && !supports_gpu_particles {
            fallbacks
                .push("GPU particles requested; CPU particle renderer remains active".to_string());
        }
        if config.post_processing && matches!(selection.selected, GraphicsApi::OpenGl) {
            fallbacks
                .push("Prefer single-pass post processing on OpenGL compatibility".to_string());
        }

        let huge_world_ready = expected_visible_sprites <= max_visible_sprites
            && expected_visible_tile_chunks <= 4096
            && config.sprite_batching
            && config.tilemap_chunk_batching
            && atlas_page_size >= 4096;

        Self {
            api: selection.selected,
            backend,
            shader_language,
            preferred_texture_format: caps.preferred_texture_format,
            atlas_page_size,
            max_visible_sprites,
            max_sprites_per_batch,
            recommended_tile_chunk_size,
            supports_compute,
            supports_gpu_particles,
            supports_tile_compute_culling,
            supports_persistent_buffers,
            huge_world_ready,
            fallbacks,
            warnings,
        }
    }
}

pub fn aggregate_batch_stats(
    sprites: &SpriteBatcher,
    tilemaps: &TilemapChunkRenderer,
    particles: &ParticleRenderer2D,
    ui_widgets: usize,
) -> BatchStats2D {
    let mut stats = sprites.stats();
    stats.tile_chunks = tilemaps.visible_chunks.len();
    stats.particle_count = particles.pending.iter().map(|cmd| cmd.particle_count).sum();
    stats.ui_widgets = ui_widgets;
    stats.draw_calls_estimate += stats.tile_chunks + particles.pending.len() + ui_widgets;
    stats
}

fn pass(name: &str, target: &str, reads: &[&str], writes: &[&str]) -> RenderPass2D {
    RenderPass2D {
        name: name.to_string(),
        target: target.to_string(),
        reads: reads.iter().map(|value| value.to_string()).collect(),
        writes: writes.iter().map(|value| value.to_string()).collect(),
        enabled: true,
    }
}

fn effect(name: &str) -> PostProcessEffect2D {
    PostProcessEffect2D {
        name: name.to_string(),
        enabled: false,
        params: BTreeMap::new(),
    }
}

fn effect_with_params(name: &str, params: &[(&str, f64)]) -> PostProcessEffect2D {
    PostProcessEffect2D {
        name: name.to_string(),
        enabled: false,
        params: params
            .iter()
            .map(|(key, value)| ((*key).to_string(), *value))
            .collect(),
    }
}

fn workgroups_for_items(items: usize, group_size: usize) -> [u32; 3] {
    [items.div_ceil(group_size).max(1) as u32, 1, 1]
}

fn caps_for_selection(api: GraphicsApi) -> RenderDeviceCaps {
    match api {
        GraphicsApi::Macroquad => RenderDeviceCaps::macroquad(),
        GraphicsApi::OpenGl => RenderDeviceCaps::opengl_compatibility(),
        GraphicsApi::WgpuMetal => RenderDeviceCaps::simulated_wgpu_metal(),
        GraphicsApi::WgpuVulkan | GraphicsApi::WgpuDx12 | GraphicsApi::WgpuWebGpu => {
            RenderDeviceCaps {
                api,
                device_name: "wgpu-2d-profile".to_string(),
                max_texture_size: 8192,
                supports_compute: true,
                supports_storage_buffers: true,
                supports_timestamp_queries: false,
                supports_multisampled_render_targets: true,
                preferred_texture_format: "rgba8unorm_srgb".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_2d_profiles_distinguish_opengl_fallback_and_metal_scale() {
        let opengl = Render2DCompatibilityProfile::from_config(
            &RenderBackendConfig {
                backend: "opengl".to_string(),
                gpu_particles: true,
                ..RenderBackendConfig::default()
            },
            30_000,
            256,
        );
        assert_eq!(opengl.api, GraphicsApi::OpenGl);
        assert!(!opengl.supports_compute);
        assert!(RenderBackendConfig::default().view_frustum_culling);
        assert!(RenderBackendConfig::default().occlusion_culling);
        assert!(RenderBackendConfig::default().lod_enabled);
        assert!(RenderBackendConfig::default().backface_culling);
        assert!(
            opengl
                .fallbacks
                .iter()
                .any(|item| item.contains("GPU particles"))
        );
        assert!(opengl.huge_world_ready);

        let metal = Render2DCompatibilityProfile::from_config(
            &RenderBackendConfig {
                backend: "wgpu".to_string(),
                experimental_wgpu: true,
                gpu_particles: true,
                tilemap_chunk_batching: true,
                ..RenderBackendConfig::default()
            },
            120_000,
            512,
        );
        if cfg!(target_os = "macos") {
            assert_eq!(metal.api, GraphicsApi::WgpuMetal);
            assert!(metal.supports_compute);
            assert!(metal.supports_gpu_particles);
            assert!(metal.supports_persistent_buffers);
        }
        assert!(metal.max_visible_sprites >= 100_000);
    }
}
