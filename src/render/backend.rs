use serde::{Deserialize, Serialize};

use crate::engine::error_handler::{MFResult, MiniForgeError};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TilemapDrawCommand {
    pub tilemap_id: u64,
    pub layer: String,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub tile_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ParticleDrawCommand {
    pub system_id: u64,
    pub particle_count: usize,
    pub texture_id: Option<u64>,
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

#[derive(Debug, Clone, Default)]
pub struct WgpuBackend {
    pub enabled: bool,
    pub prefer_metal: bool,
    pub initialized: bool,
    pub last_error: Option<String>,
    pub caps: Option<RenderDeviceCaps>,
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

impl RenderBackend for WgpuBackend {
    fn name(&self) -> &'static str {
        "wgpu_experimental"
    }

    fn init(&mut self) -> MFResult<()> {
        if !self.enabled {
            self.last_error = Some("wgpu backend experimental desactivado".to_string());
            return Err(MiniForgeError::RenderError(
                "wgpu backend experimental desactivado".to_string(),
            ));
        }
        self.caps = Some(if self.prefer_metal && cfg!(target_os = "macos") {
            RenderDeviceCaps::simulated_wgpu_metal()
        } else {
            RenderDeviceCaps {
                api: if cfg!(target_os = "windows") {
                    GraphicsApi::WgpuDx12
                } else {
                    GraphicsApi::WgpuVulkan
                },
                device_name: "wgpu-experimental".to_string(),
                max_texture_size: 8192,
                supports_compute: true,
                supports_storage_buffers: true,
                supports_timestamp_queries: false,
                supports_multisampled_render_targets: true,
                preferred_texture_format: "rgba8unorm_srgb".to_string(),
            }
        });
        self.initialized = true;
        Ok(())
    }

    fn begin_frame(&mut self) -> MFResult<()> {
        Ok(())
    }

    fn end_frame(&mut self) -> MFResult<()> {
        Ok(())
    }

    fn resize(&mut self, _width: u32, _height: u32) -> MFResult<()> {
        Ok(())
    }

    fn draw_sprite(&mut self, _cmd: SpriteDrawCommand) -> MFResult<()> {
        Ok(())
    }

    fn draw_tilemap(&mut self, _cmd: TilemapDrawCommand) -> MFResult<()> {
        Ok(())
    }

    fn draw_particles(&mut self, _cmd: ParticleDrawCommand) -> MFResult<()> {
        Ok(())
    }

    fn draw_ui(&mut self, _cmd: UiDrawCommand) -> MFResult<()> {
        Ok(())
    }

    fn set_camera_3d(&mut self, _cmd: CameraCommand3D) -> MFResult<()> {
        Ok(())
    }

    fn draw_mesh_3d(&mut self, _cmd: MeshDrawCommand3D) -> MFResult<()> {
        Ok(())
    }

    fn draw_light_3d(&mut self, _cmd: LightDrawCommand3D) -> MFResult<()> {
        Ok(())
    }
}
