use serde::{Deserialize, Serialize};

use crate::engine::error_handler::MFResult;

pub use super::wgpu_backend::WgpuBackend;

pub const BUILTIN_RADIAL_LIGHT_TEXTURE_ID: u64 = u64::MAX - 1;
pub const BUILTIN_RADIAL_LIGHT_TEXTURE_SIZE: u32 = 64;

pub fn radial_light_texture_rgba8(size: u32) -> Vec<u8> {
    let size = size.clamp(2, 512);
    let mut pixels = Vec::with_capacity(size as usize * size as usize * 4);
    let center = (size as f32 - 1.0) * 0.5;
    let radius = center.max(1.0);
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - center) / radius;
            let dy = (y as f32 - center) / radius;
            let distance = (dx * dx + dy * dy).sqrt();
            let coverage = (1.0 - distance).clamp(0.0, 1.0);
            let alpha = (coverage * coverage * 255.0).round() as u8;
            pixels.extend_from_slice(&[255, 255, 255, alpha]);
        }
    }
    pixels
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GraphicsApi {
    Macroquad,
    OpenGl,
    WgpuMetal,
    WgpuVulkan,
    WgpuDx12,
    WgpuWebGpu,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackendPreference {
    Auto,
    Macroquad,
    Wgpu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderDeviceCaps {
    pub api: GraphicsApi,
    pub device_name: String,
    pub max_texture_size: u32,
    pub supports_compute: bool,
    pub supports_storage_buffers: bool,
    pub supports_timestamp_queries: bool,
    pub supports_multisampled_render_targets: bool,
    pub preferred_texture_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetalOptimizationConfig {
    pub prefer_metal_on_macos: bool,
    pub use_memoryless_targets: bool,
    pub prefer_low_power_gpu: bool,
    pub use_frame_capture_labels: bool,
    pub triple_buffering: bool,
    pub use_argument_buffers_future: bool,
    pub allow_compute_particles: bool,
    pub allow_compute_tile_visibility: bool,
    pub allow_compute_flow_fields: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderBackendSelection {
    pub selected: GraphicsApi,
    pub reason: String,
    pub fallback: Option<GraphicsApi>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SpriteDrawCommand {
    pub entity_id: u64,
    pub texture_id: u64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SpriteRegionDrawCommand {
    pub sprite: SpriteDrawCommand,
    /// Normalized atlas rectangle: `[u_min, v_min, u_max, v_max]`.
    pub uv_rect: [f32; 4],
    /// Optional pixel-space clip rectangle: `[x, y, width, height]`.
    pub clip_rect: Option<[u32; 4]>,
}

/// Stable, backend-independent blend modes for sprites, UI geometry and particles.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpriteBlendMode {
    /// Conventional straight-alpha compositing.
    #[default]
    Alpha,
    /// Alpha compositing after the backend premultiplies the fragment color.
    PremultipliedAlpha,
    /// Adds the alpha-weighted source color to the destination.
    Additive,
    /// Multiplies the destination while preserving partial-alpha coverage.
    Multiply,
    /// Brightens the destination with a screen-style effect.
    Screen,
}

impl SpriteBlendMode {
    pub fn from_name(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        match normalized.as_str() {
            "alpha" | "normal" | "translucent" => Some(Self::Alpha),
            "premultiplied" | "premultiplied_alpha" | "premultipliedalpha" => {
                Some(Self::PremultipliedAlpha)
            }
            "add" | "additive" => Some(Self::Additive),
            "multiply" | "multiplicative" => Some(Self::Multiply),
            "screen" => Some(Self::Screen),
            _ => None,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpriteMaterialEffect {
    #[default]
    None = 0,
    Grayscale = 1,
    Sepia = 2,
    Invert = 3,
    Flash = 4,
}

impl SpriteMaterialEffect {
    pub fn from_name(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        match normalized.as_str() {
            "none" | "default" | "sprite_default" | "unlit" => Some(Self::None),
            "gray" | "grey" | "grayscale" | "greyscale" | "sprite_grayscale" => {
                Some(Self::Grayscale)
            }
            "sepia" | "sprite_sepia" => Some(Self::Sepia),
            "invert" | "inverted" | "sprite_invert" => Some(Self::Invert),
            "flash" | "hit_flash" | "sprite_flash" => Some(Self::Flash),
            _ => None,
        }
    }
}

/// Optional sprite render state kept separate from geometry for compatibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpriteDrawOptions {
    #[serde(default)]
    pub blend_mode: SpriteBlendMode,
    #[serde(default)]
    pub material_effect: SpriteMaterialEffect,
    #[serde(default = "default_effect_strength")]
    pub effect_strength: u8,
}

impl Default for SpriteDrawOptions {
    fn default() -> Self {
        Self {
            blend_mode: SpriteBlendMode::Alpha,
            material_effect: SpriteMaterialEffect::None,
            effect_strength: u8::MAX,
        }
    }
}

fn default_effect_strength() -> u8 {
    u8::MAX
}

/// Line breaking policy for backend-independent text.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextWrapMode {
    None,
    #[default]
    Word,
    Glyph,
}

/// A shaped text area submitted in physical screen pixels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextDrawCommand {
    pub text_id: u64,
    pub text: String,
    #[serde(default)]
    pub font_family: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub color: [u8; 4],
    #[serde(default)]
    pub wrap: TextWrapMode,
    pub clip_rect: Option<[u32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TilemapDrawCommand {
    pub tilemap_id: u64,
    pub layer: String,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub tile_count: usize,
}

/// Backend-independent GPU particle emitter submitted once per rendered frame.
///
/// The command describes an emitter rather than uploading individual particle
/// quads. Compute-capable backends keep persistent particle state keyed by
/// `system_id`; compatibility backends may use `particle_count` as a bounded
/// CPU fallback budget.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ParticleDrawCommand {
    pub system_id: u64,
    /// Maximum number of persistent particles owned by this emitter.
    pub particle_count: usize,
    pub texture_id: Option<u64>,
    pub origin: [f32; 2],
    pub velocity: [f32; 2],
    pub gravity: [f32; 2],
    pub spread: f32,
    pub lifetime: f32,
    pub drag: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub color: [f32; 4],
    pub emission_rate: f32,
    pub burst_count: u32,
    pub delta_seconds: f32,
    pub playing: bool,
    pub blend_mode: SpriteBlendMode,
}

impl Default for ParticleDrawCommand {
    fn default() -> Self {
        Self {
            system_id: 0,
            particle_count: 1_024,
            texture_id: None,
            origin: [0.0; 2],
            velocity: [0.0, -40.0],
            gravity: [0.0, 20.0],
            spread: 18.0,
            lifetime: 1.0,
            drag: 0.0,
            start_size: 8.0,
            end_size: 0.0,
            color: [1.0, 0.82, 0.42, 0.86],
            emission_rate: 64.0,
            burst_count: 16,
            delta_seconds: 1.0 / 60.0,
            playing: true,
            blend_mode: SpriteBlendMode::Additive,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiDrawCommand {
    pub widget_id: String,
    pub layer: i32,
    pub clip_rect: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshDrawCommand3D {
    pub entity_id: u64,
    pub mesh_id: u64,
    pub material_id: Option<u64>,
    pub position: [f32; 3],
    pub rotation_euler: [f32; 3],
    pub scale: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightDrawCommand3D {
    pub entity_id: u64,
    pub light_type: String,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: [f32; 4],
    pub intensity: f32,
    pub range: f32,
    pub casts_shadows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraCommand3D {
    pub entity_id: u64,
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov_y_degrees: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderBackendConfig {
    pub backend: String,
    pub experimental_wgpu: bool,
    pub prefer_metal_on_macos: bool,
    #[serde(default)]
    pub metal: MetalOptimizationConfig,
    pub vsync: bool,
    pub pixel_perfect: bool,
    pub render_scale: f32,
    pub max_texture_size: u32,
    pub sprite_batching: bool,
    pub tilemap_chunk_batching: bool,
    #[serde(default = "default_true")]
    pub view_frustum_culling: bool,
    #[serde(default = "default_true")]
    pub occlusion_culling: bool,
    #[serde(default = "default_true")]
    pub lod_enabled: bool,
    #[serde(default = "default_true")]
    pub backface_culling: bool,
    pub gpu_particles: bool,
    pub post_processing: bool,
    #[serde(default)]
    pub opengl_compatibility: bool,
    #[serde(default)]
    pub shader_hot_reload: bool,
    #[serde(default)]
    pub enable_3d: bool,
    #[serde(default)]
    pub hybrid_2d_3d: bool,
    #[serde(default)]
    pub depth_buffer: bool,
    #[serde(default)]
    pub mesh_batching: bool,
    #[serde(default)]
    pub shadow_maps_3d: bool,
}

pub trait RenderBackend {
    fn name(&self) -> &'static str;
    fn init(&mut self) -> MFResult<()>;
    fn begin_frame(&mut self) -> MFResult<()>;
    fn end_frame(&mut self) -> MFResult<()>;
    fn resize(&mut self, width: u32, height: u32) -> MFResult<()>;
    fn draw_sprite(&mut self, cmd: SpriteDrawCommand) -> MFResult<()>;
    fn draw_sprite_with_options(
        &mut self,
        cmd: SpriteDrawCommand,
        _options: SpriteDrawOptions,
    ) -> MFResult<()> {
        self.draw_sprite(cmd)
    }
    fn draw_sprite_region(&mut self, cmd: SpriteRegionDrawCommand) -> MFResult<()> {
        self.draw_sprite(cmd.sprite)
    }
    fn draw_sprite_region_with_options(
        &mut self,
        cmd: SpriteRegionDrawCommand,
        _options: SpriteDrawOptions,
    ) -> MFResult<()> {
        self.draw_sprite_region(cmd)
    }
    fn draw_text(&mut self, cmd: TextDrawCommand) -> MFResult<()>;
    fn draw_tilemap(&mut self, cmd: TilemapDrawCommand) -> MFResult<()>;
    fn draw_particles(&mut self, cmd: ParticleDrawCommand) -> MFResult<()>;
    fn draw_ui(&mut self, cmd: UiDrawCommand) -> MFResult<()>;
    fn set_camera_3d(&mut self, cmd: CameraCommand3D) -> MFResult<()>;
    fn draw_mesh_3d(&mut self, cmd: MeshDrawCommand3D) -> MFResult<()>;
    fn draw_light_3d(&mut self, cmd: LightDrawCommand3D) -> MFResult<()>;
}

#[derive(Debug, Clone, Default)]
pub struct MacroquadBackend {
    pub initialized: bool,
    pub frame_open: bool,
    pub width: u32,
    pub height: u32,
    pub draw_calls: usize,
}

impl Default for RenderBackendConfig {
    fn default() -> Self {
        let metal = MetalOptimizationConfig::default();
        Self {
            backend: "macroquad".to_string(),
            experimental_wgpu: false,
            prefer_metal_on_macos: true,
            metal,
            vsync: true,
            pixel_perfect: true,
            render_scale: 1.0,
            max_texture_size: 8192,
            sprite_batching: true,
            tilemap_chunk_batching: true,
            view_frustum_culling: true,
            occlusion_culling: true,
            lod_enabled: true,
            backface_culling: true,
            gpu_particles: false,
            post_processing: true,
            opengl_compatibility: true,
            shader_hot_reload: true,
            enable_3d: false,
            hybrid_2d_3d: false,
            depth_buffer: false,
            mesh_batching: false,
            shadow_maps_3d: false,
        }
    }
}

fn default_true() -> bool {
    true
}

impl Default for MetalOptimizationConfig {
    fn default() -> Self {
        Self {
            prefer_metal_on_macos: true,
            use_memoryless_targets: true,
            prefer_low_power_gpu: false,
            use_frame_capture_labels: true,
            triple_buffering: true,
            use_argument_buffers_future: false,
            allow_compute_particles: false,
            allow_compute_tile_visibility: false,
            allow_compute_flow_fields: false,
        }
    }
}

impl RenderDeviceCaps {
    pub fn macroquad() -> Self {
        Self {
            api: GraphicsApi::Macroquad,
            device_name: "macroquad-compatible".to_string(),
            max_texture_size: 8192,
            supports_compute: false,
            supports_storage_buffers: false,
            supports_timestamp_queries: false,
            supports_multisampled_render_targets: true,
            preferred_texture_format: "rgba8".to_string(),
        }
    }

    pub fn opengl_compatibility() -> Self {
        Self {
            api: GraphicsApi::OpenGl,
            device_name: "opengl-compatibility-planned".to_string(),
            max_texture_size: 8192,
            supports_compute: false,
            supports_storage_buffers: false,
            supports_timestamp_queries: false,
            supports_multisampled_render_targets: true,
            preferred_texture_format: "rgba8".to_string(),
        }
    }

    pub fn simulated_wgpu_metal() -> Self {
        Self {
            api: GraphicsApi::WgpuMetal,
            device_name: "wgpu-metal-experimental".to_string(),
            max_texture_size: 16384,
            supports_compute: true,
            supports_storage_buffers: true,
            supports_timestamp_queries: true,
            supports_multisampled_render_targets: true,
            preferred_texture_format: "bgra8unorm_srgb".to_string(),
        }
    }
}

impl RenderBackendSelection {
    pub fn choose(config: &RenderBackendConfig) -> Self {
        let backend = config.backend.to_ascii_lowercase();
        if matches!(backend.as_str(), "opengl" | "gl" | "gl_compat") {
            return Self {
                selected: GraphicsApi::OpenGl,
                reason:
                    "backend OpenGL compatibility seleccionado para plugins/herramientas heredadas"
                        .to_string(),
                fallback: Some(GraphicsApi::Macroquad),
            };
        }
        let wants_wgpu = backend == "wgpu" || (backend == "auto" && config.experimental_wgpu);
        if wants_wgpu && config.experimental_wgpu {
            let api = if config.prefer_metal_on_macos && cfg!(target_os = "macos") {
                GraphicsApi::WgpuMetal
            } else if cfg!(target_os = "windows") {
                GraphicsApi::WgpuDx12
            } else {
                GraphicsApi::WgpuVulkan
            };
            return Self {
                selected: api,
                reason: "wgpu experimental habilitado".to_string(),
                fallback: Some(GraphicsApi::Macroquad),
            };
        }
        Self {
            selected: GraphicsApi::Macroquad,
            reason: if wants_wgpu {
                "wgpu solicitado pero experimental_wgpu=false; usando macroquad".to_string()
            } else {
                "backend macroquad estable".to_string()
            },
            fallback: None,
        }
    }
}

impl RenderBackend for MacroquadBackend {
    fn name(&self) -> &'static str {
        "macroquad"
    }

    fn init(&mut self) -> MFResult<()> {
        self.initialized = true;
        Ok(())
    }

    fn begin_frame(&mut self) -> MFResult<()> {
        self.frame_open = true;
        self.draw_calls = 0;
        Ok(())
    }

    fn end_frame(&mut self) -> MFResult<()> {
        self.frame_open = false;
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> MFResult<()> {
        self.width = width;
        self.height = height;
        Ok(())
    }

    fn draw_sprite(&mut self, _cmd: SpriteDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_text(&mut self, _cmd: TextDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_tilemap(&mut self, _cmd: TilemapDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_particles(&mut self, _cmd: ParticleDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_ui(&mut self, _cmd: UiDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn set_camera_3d(&mut self, _cmd: CameraCommand3D) -> MFResult<()> {
        Ok(())
    }

    fn draw_mesh_3d(&mut self, _cmd: MeshDrawCommand3D) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_light_3d(&mut self, _cmd: LightDrawCommand3D) -> MFResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{SpriteBlendMode, SpriteMaterialEffect, radial_light_texture_rgba8};

    #[test]
    fn sprite_blend_mode_accepts_editor_and_asset_aliases() {
        assert_eq!(
            SpriteBlendMode::from_name("premultiplied-alpha"),
            Some(SpriteBlendMode::PremultipliedAlpha)
        );
        assert_eq!(
            SpriteBlendMode::from_name("add"),
            Some(SpriteBlendMode::Additive)
        );
        assert_eq!(
            SpriteBlendMode::from_name("multiplicative"),
            Some(SpriteBlendMode::Multiply)
        );
        assert_eq!(
            SpriteBlendMode::from_name("screen"),
            Some(SpriteBlendMode::Screen)
        );
        assert_eq!(SpriteBlendMode::from_name("custom_shader"), None);
    }

    #[test]
    fn builtin_radial_light_texture_has_a_soft_bounded_falloff() {
        let size = 8;
        let pixels = radial_light_texture_rgba8(size);
        assert_eq!(pixels.len(), size as usize * size as usize * 4);
        let alpha = |x: usize, y: usize| pixels[(y * size as usize + x) * 4 + 3];
        assert!(alpha(3, 3) > alpha(1, 1));
        assert_eq!(alpha(0, 0), 0);
    }

    #[test]
    fn sprite_material_effect_accepts_editor_shader_aliases() {
        assert_eq!(
            SpriteMaterialEffect::from_name("sprite-grayscale"),
            Some(SpriteMaterialEffect::Grayscale)
        );
        assert_eq!(
            SpriteMaterialEffect::from_name("hit_flash"),
            Some(SpriteMaterialEffect::Flash)
        );
        assert_eq!(SpriteMaterialEffect::from_name("custom_wgsl"), None);
    }
}
