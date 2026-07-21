//! Native `wgpu` renderer used by MiniForge's 2D migration path.
//!
//! The backend owns a real adapter, device, queue, render pipeline and color
//! target. It deliberately starts with an off-screen target so the same code is
//! usable by the Qt editor, headless tests and a future window surface without
//! tying the engine renderer to a particular windowing crate.

use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::engine::error_handler::{MFResult, MiniForgeError};

use super::backend::{
    CameraCommand3D, GraphicsApi, LightDrawCommand3D, MeshDrawCommand3D, ParticleDrawCommand,
    RenderBackend, RenderDeviceCaps, SpriteDrawCommand, SpriteRegionDrawCommand,
    TilemapDrawCommand, UiDrawCommand,
};

const OFFSCREEN_TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SPRITE_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;

const SPRITE_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    color: [f32; 4],
    uv: [f32; 2],
}

#[derive(Clone, Copy)]
struct QueuedSprite {
    sprite: SpriteDrawCommand,
    uv_rect: [f32; 4],
    clip_rect: Option<[u32; 4]>,
}

const SPRITE_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2];

struct WgpuTexture {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

struct WgpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    target: Option<wgpu::Texture>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white_texture: WgpuTexture,
    textures: HashMap<u64, WgpuTexture>,
}

/// Concrete, renderer-agnostic 2D backend backed by a physical GPU adapter.
pub struct WgpuBackend {
    pub enabled: bool,
    pub prefer_metal: bool,
    pub initialized: bool,
    pub last_error: Option<String>,
    pub caps: Option<RenderDeviceCaps>,
    pub width: u32,
    pub height: u32,
    pub draw_calls: usize,
    pub submitted_frames: u64,
    pub skipped_surface_frames: u64,
    pub surface_reconfigurations: u64,
    frame_open: bool,
    clear_color: [f64; 4],
    sprites: Vec<QueuedSprite>,
    state: Option<WgpuState>,
}

impl fmt::Debug for WgpuBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WgpuBackend")
            .field("enabled", &self.enabled)
            .field("prefer_metal", &self.prefer_metal)
            .field("initialized", &self.initialized)
            .field("last_error", &self.last_error)
            .field("caps", &self.caps)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("draw_calls", &self.draw_calls)
            .field("submitted_frames", &self.submitted_frames)
            .field("skipped_surface_frames", &self.skipped_surface_frames)
            .field("surface_reconfigurations", &self.surface_reconfigurations)
            .field("has_surface", &self.has_surface())
            .field("frame_open", &self.frame_open)
            .finish_non_exhaustive()
    }
}

impl Default for WgpuBackend {
    fn default() -> Self {
        Self {
            enabled: false,
            prefer_metal: cfg!(target_os = "macos"),
            initialized: false,
            last_error: None,
            caps: None,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            draw_calls: 0,
            submitted_frames: 0,
            skipped_surface_frames: 0,
            surface_reconfigurations: 0,
            frame_open: false,
            clear_color: [0.035, 0.043, 0.059, 1.0],
            sprites: Vec::new(),
            state: None,
        }
    }
}

impl WgpuBackend {
    pub fn new(enabled: bool, prefer_metal: bool) -> Self {
        Self {
            enabled,
            prefer_metal,
            ..Self::default()
        }
    }

    pub fn set_clear_color(&mut self, color: [f64; 4]) {
        self.clear_color = color.map(|channel| channel.clamp(0.0, 1.0));
    }

    pub fn is_using_physical_device(&self) -> bool {
        self.initialized && self.state.is_some()
    }

    pub fn has_surface(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| state.surface.is_some())
    }

    /// Initializes the backend against an owned window or canvas handle.
    ///
    /// Passing an owned handle such as `Arc<winit::window::Window>` allows
    /// `wgpu` to keep the surface target alive for the backend's lifetime.
    pub fn init_with_surface<T>(&mut self, target: T) -> MFResult<()>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        if !self.enabled {
            return self.disabled_error();
        }
        let instance = self.create_instance();
        let surface = instance
            .create_surface(target)
            .map_err(|error| render_error(format!("wgpu surface creation failed: {error}")))?;
        self.initialize_state(&instance, Some(surface))
    }

    /// Uploads or replaces a complete RGBA8 sprite texture.
    pub fn upload_texture_rgba8(
        &mut self,
        texture_id: u64,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> MFResult<()> {
        if texture_id == 0 {
            return Err(render_error(
                "texture id 0 is reserved for the white texture",
            ));
        }
        let width = width.max(1);
        let height = height.max(1);
        let expected = width as usize * height as usize * 4;
        if pixels.len() != expected {
            return Err(render_error(format!(
                "texture {texture_id} has {} bytes; expected {expected} for {width}x{height} RGBA8",
                pixels.len()
            )));
        }
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| render_error("wgpu backend has not been initialized"))?;
        let texture = create_sampled_texture(
            &state.device,
            &state.queue,
            &state.texture_layout,
            &state.sampler,
            width,
            height,
            pixels,
            "MiniForge uploaded sprite texture",
        );
        state.textures.insert(texture_id, texture);
        Ok(())
    }

    pub fn remove_texture(&mut self, texture_id: u64) -> bool {
        self.state
            .as_mut()
            .is_some_and(|state| state.textures.remove(&texture_id).is_some())
    }

    pub fn texture_count(&self) -> usize {
        self.state.as_ref().map_or(0, |state| state.textures.len())
    }

    /// Copies the last submitted GPU target into tightly packed RGBA8 bytes.
    /// This is primarily for editor previews, screenshots and renderer tests.
    pub fn readback_rgba8(&self) -> MFResult<Vec<u8>> {
        let state = self.state()?;
        let target = state.target.as_ref().ok_or_else(|| {
            render_error("readback is only available for the off-screen wgpu target")
        })?;
        let unpadded_bytes_per_row = self.width.saturating_mul(4);
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(alignment) * alignment;
        let buffer_size = u64::from(padded_bytes_per_row) * u64::from(self.height);
        let readback = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MiniForge wgpu 2D readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MiniForge wgpu 2D readback encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        state.queue.submit([encoder.finish()]);

        let slice = readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        state
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| render_error(format!("wgpu readback poll failed: {error}")))?;
        receiver
            .recv()
            .map_err(|error| render_error(format!("wgpu readback callback failed: {error}")))?
            .map_err(|error| render_error(format!("wgpu readback map failed: {error}")))?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|error| render_error(format!("wgpu readback range failed: {error}")))?;
        let mut pixels = vec![0; unpadded_bytes_per_row as usize * self.height as usize];
        for row in 0..self.height as usize {
            let source_start = row * padded_bytes_per_row as usize;
            let destination_start = row * unpadded_bytes_per_row as usize;
            pixels[destination_start..destination_start + unpadded_bytes_per_row as usize]
                .copy_from_slice(
                    &mapped[source_start..source_start + unpadded_bytes_per_row as usize],
                );
        }
        drop(mapped);
        readback.unmap();
        Ok(pixels)
    }

    fn create_instance(&self) -> wgpu::Instance {
        let backends = if self.prefer_metal && cfg!(target_os = "macos") {
            wgpu::Backends::METAL
        } else {
            wgpu::Backends::PRIMARY
        };
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = backends;
        wgpu::Instance::new(instance_descriptor)
    }

    fn create_state(
        &self,
        instance: &wgpu::Instance,
        surface: Option<wgpu::Surface<'static>>,
    ) -> MFResult<(WgpuState, RenderDeviceCaps)> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|error| render_error(format!("wgpu adapter unavailable: {error}")))?;

        let adapter_info = adapter.get_info();
        let adapter_features = adapter.features();
        let adapter_limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("MiniForge wgpu 2D device"),
            required_features: wgpu::Features::empty(),
            required_limits:
                wgpu::Limits::downlevel_defaults().using_resolution(adapter_limits.clone()),
            ..Default::default()
        }))
        .map_err(|error| render_error(format!("wgpu device creation failed: {error}")))?;

        let (target_format, surface_config) = if let Some(surface) = surface.as_ref() {
            let mut config = surface
                .get_default_config(&adapter, self.width, self.height)
                .ok_or_else(|| render_error("wgpu adapter does not support the window surface"))?;
            let capabilities = surface.get_capabilities(&adapter);
            if let Some(srgb) = capabilities
                .formats
                .iter()
                .copied()
                .find(|format| format.is_srgb())
            {
                config.format = srgb;
            }
            config.present_mode = wgpu::PresentMode::AutoVsync;
            config.desired_maximum_frame_latency = 2;
            surface.configure(&device, &config);
            (config.format, Some(config))
        } else {
            (OFFSCREEN_TARGET_FORMAT, None)
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MiniForge sprite shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MiniForge sprite texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("MiniForge pixel-perfect sprite sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let white_texture = create_sampled_texture(
            &device,
            &queue,
            &texture_layout,
            &sampler,
            1,
            1,
            &[255, 255, 255, 255],
            "MiniForge white sprite texture",
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MiniForge wgpu 2D pipeline layout"),
            bind_group_layouts: &[Some(&texture_layout)],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &SPRITE_ATTRIBUTES,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MiniForge wgpu 2D sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let target = surface
            .is_none()
            .then(|| create_target(&device, self.width, self.height));
        let caps = RenderDeviceCaps {
            api: graphics_api(adapter_info.backend),
            device_name: adapter_info.name,
            max_texture_size: adapter_limits.max_texture_dimension_2d,
            supports_compute: adapter_limits.max_compute_workgroups_per_dimension > 0,
            supports_storage_buffers: adapter_limits.max_storage_buffers_per_shader_stage > 0,
            supports_timestamp_queries: adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY),
            supports_multisampled_render_targets: true,
            preferred_texture_format: format!("{target_format:?}"),
        };
        Ok((
            WgpuState {
                device,
                queue,
                pipeline,
                target,
                surface,
                surface_config,
                texture_layout,
                sampler,
                white_texture,
                textures: HashMap::new(),
            },
            caps,
        ))
    }

    fn disabled_error<T>(&mut self) -> MFResult<T> {
        let message = "wgpu backend disabled by project configuration".to_string();
        self.last_error = Some(message.clone());
        Err(render_error(message))
    }

    fn initialize_state(
        &mut self,
        instance: &wgpu::Instance,
        surface: Option<wgpu::Surface<'static>>,
    ) -> MFResult<()> {
        match self.create_state(instance, surface) {
            Ok((state, caps)) => {
                self.state = Some(state);
                self.caps = Some(caps);
                self.initialized = true;
                self.last_error = None;
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn state(&self) -> MFResult<&WgpuState> {
        self.state
            .as_ref()
            .ok_or_else(|| render_error("wgpu backend has not been initialized"))
    }

    fn render_frame(&mut self) -> MFResult<bool> {
        let vertices = sprites_to_vertices(&self.sprites, self.width, self.height);
        let state = self.state()?;
        let vertex_buffer = (!vertices.is_empty()).then(|| {
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MiniForge wgpu 2D sprite vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let mut reconfigure_after_present = false;
        let mut reconfiguration_count = 0u64;
        let mut surface_output = if let Some(surface) = state.surface.as_ref() {
            let mut acquire =
                |surface: &wgpu::Surface<'static>| match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(texture) => Ok(Some(texture)),
                    wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                        reconfigure_after_present = true;
                        Ok(Some(texture))
                    }
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => Ok(None),
                    wgpu::CurrentSurfaceTexture::Outdated => {
                        let config = state.surface_config.as_ref().ok_or_else(|| {
                            render_error("wgpu surface is missing its presentation configuration")
                        })?;
                        surface.configure(&state.device, config);
                        reconfiguration_count += 1;
                        match surface.get_current_texture() {
                            wgpu::CurrentSurfaceTexture::Success(texture)
                            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Ok(Some(texture)),
                            wgpu::CurrentSurfaceTexture::Timeout
                            | wgpu::CurrentSurfaceTexture::Occluded
                            | wgpu::CurrentSurfaceTexture::Outdated => Ok(None),
                            wgpu::CurrentSurfaceTexture::Lost => {
                                Err(render_error("wgpu window surface was lost"))
                            }
                            wgpu::CurrentSurfaceTexture::Validation => Err(render_error(
                                "wgpu surface acquisition failed validation after reconfigure",
                            )),
                        }
                    }
                    wgpu::CurrentSurfaceTexture::Lost => {
                        Err(render_error("wgpu window surface was lost"))
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        Err(render_error("wgpu surface acquisition failed validation"))
                    }
                };
            acquire(surface)?
        } else {
            None
        };
        if state.surface.is_some() && surface_output.is_none() {
            self.surface_reconfigurations += reconfiguration_count;
            return Ok(false);
        }
        let target_texture = surface_output
            .as_ref()
            .map(|output| &output.texture)
            .or(state.target.as_ref())
            .ok_or_else(|| render_error("wgpu frame has no render target"))?;
        let view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MiniForge wgpu 2D frame encoder"),
            });
        {
            let color_attachment = wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.clear_color[0],
                        g: self.clear_color[1],
                        b: self.clear_color[2],
                        a: self.clear_color[3],
                    }),
                    store: wgpu::StoreOp::Store,
                },
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MiniForge wgpu 2D frame"),
                color_attachments: &[Some(color_attachment)],
                ..Default::default()
            });
            if let Some(buffer) = vertex_buffer.as_ref() {
                pass.set_pipeline(&state.pipeline);
                pass.set_vertex_buffer(0, buffer.slice(..));
                for (index, queued) in self.sprites.iter().enumerate() {
                    let texture = state
                        .textures
                        .get(&queued.sprite.texture_id)
                        .unwrap_or(&state.white_texture);
                    pass.set_bind_group(0, &texture.bind_group, &[]);
                    let clip = queued.clip_rect.unwrap_or([0, 0, self.width, self.height]);
                    let x = clip[0].min(self.width.saturating_sub(1));
                    let y = clip[1].min(self.height.saturating_sub(1));
                    let width = clip[2].min(self.width.saturating_sub(x));
                    let height = clip[3].min(self.height.saturating_sub(y));
                    if width == 0 || height == 0 {
                        continue;
                    }
                    pass.set_scissor_rect(x, y, width, height);
                    let first_vertex = index as u32 * 6;
                    pass.draw(first_vertex..first_vertex + 6, 0..1);
                }
            }
        }
        state.queue.submit([encoder.finish()]);
        if let Some(output) = surface_output.take() {
            state.queue.present(output);
            if reconfigure_after_present {
                let surface = state
                    .surface
                    .as_ref()
                    .expect("surface output requires a surface");
                let config = state
                    .surface_config
                    .as_ref()
                    .expect("surface output requires configuration");
                surface.configure(&state.device, config);
                reconfiguration_count += 1;
            }
        }
        self.surface_reconfigurations += reconfiguration_count;
        Ok(true)
    }
}

impl RenderBackend for WgpuBackend {
    fn name(&self) -> &'static str {
        "wgpu"
    }

    fn init(&mut self) -> MFResult<()> {
        if !self.enabled {
            return self.disabled_error();
        }
        let instance = self.create_instance();
        self.initialize_state(&instance, None)
    }

    fn begin_frame(&mut self) -> MFResult<()> {
        self.state()?;
        self.sprites.clear();
        self.draw_calls = 0;
        self.frame_open = true;
        Ok(())
    }

    fn end_frame(&mut self) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error("wgpu end_frame called without begin_frame"));
        }
        let submitted = self.render_frame()?;
        self.frame_open = false;
        if submitted {
            self.submitted_frames += 1;
        } else {
            self.skipped_surface_frames += 1;
        }
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> MFResult<()> {
        self.width = width.max(1);
        self.height = height.max(1);
        if let Some(state) = self.state.as_mut() {
            if let (Some(surface), Some(config)) =
                (state.surface.as_ref(), state.surface_config.as_mut())
            {
                config.width = self.width;
                config.height = self.height;
                surface.configure(&state.device, config);
                self.surface_reconfigurations += 1;
            } else {
                state.target = Some(create_target(&state.device, self.width, self.height));
            }
        }
        Ok(())
    }

    fn draw_sprite(&mut self, command: SpriteDrawCommand) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error("wgpu draw_sprite called outside a frame"));
        }
        self.sprites.push(QueuedSprite {
            sprite: command,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            clip_rect: None,
        });
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_sprite_region(&mut self, command: SpriteRegionDrawCommand) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error(
                "wgpu draw_sprite_region called outside a frame",
            ));
        }
        if command.uv_rect.iter().any(|value| !value.is_finite())
            || command.uv_rect[0] >= command.uv_rect[2]
            || command.uv_rect[1] >= command.uv_rect[3]
        {
            return Err(render_error(
                "wgpu sprite atlas UV rectangle must be finite and ordered",
            ));
        }
        self.sprites.push(QueuedSprite {
            sprite: command.sprite,
            uv_rect: command.uv_rect.map(|value| value.clamp(0.0, 1.0)),
            clip_rect: command.clip_rect,
        });
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_tilemap(&mut self, _command: TilemapDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_particles(&mut self, _command: ParticleDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_ui(&mut self, _command: UiDrawCommand) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn set_camera_3d(&mut self, _command: CameraCommand3D) -> MFResult<()> {
        Ok(())
    }

    fn draw_mesh_3d(&mut self, _command: MeshDrawCommand3D) -> MFResult<()> {
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_light_3d(&mut self, _command: LightDrawCommand3D) -> MFResult<()> {
        Ok(())
    }
}

fn create_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MiniForge wgpu 2D color target"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OFFSCREEN_TARGET_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

#[allow(clippy::too_many_arguments)]
fn create_sampled_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
    pixels: &[u8],
    label: &str,
) -> WgpuTexture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SPRITE_TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    WgpuTexture {
        _texture: texture,
        bind_group,
    }
}

fn sprites_to_vertices(commands: &[QueuedSprite], width: u32, height: u32) -> Vec<SpriteVertex> {
    let mut vertices = Vec::with_capacity(commands.len() * 6);
    for queued in commands {
        let command = queued.sprite;
        let center_x = command.x + command.width * 0.5;
        let center_y = command.y + command.height * 0.5;
        let half_width = command.width * 0.5;
        let half_height = command.height * 0.5;
        let (sine, cosine) = command.rotation.sin_cos();
        let corners = [
            (-half_width, -half_height),
            (half_width, -half_height),
            (half_width, half_height),
            (-half_width, half_height),
        ]
        .map(|(x, y)| {
            let rotated_x = x * cosine - y * sine + center_x;
            let rotated_y = x * sine + y * cosine + center_y;
            [
                rotated_x / width.max(1) as f32 * 2.0 - 1.0,
                1.0 - rotated_y / height.max(1) as f32 * 2.0,
            ]
        });
        let [u_min, v_min, u_max, v_max] = queued.uv_rect;
        let uvs = [
            [u_min, v_min],
            [u_max, v_min],
            [u_max, v_max],
            [u_min, v_max],
        ];
        let vertex = |index: usize| SpriteVertex {
            position: corners[index],
            color: command.color,
            uv: uvs[index],
        };
        vertices.extend([
            vertex(0),
            vertex(1),
            vertex(2),
            vertex(0),
            vertex(2),
            vertex(3),
        ]);
    }
    vertices
}

fn graphics_api(backend: wgpu::Backend) -> GraphicsApi {
    match backend {
        wgpu::Backend::Metal => GraphicsApi::WgpuMetal,
        wgpu::Backend::Dx12 => GraphicsApi::WgpuDx12,
        wgpu::Backend::BrowserWebGpu => GraphicsApi::WgpuWebGpu,
        _ => GraphicsApi::WgpuVulkan,
    }
}

fn render_error(message: impl Into<String>) -> MiniForgeError {
    MiniForgeError::RenderError(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_vertices_cover_requested_pixel_rect() {
        let vertices = sprites_to_vertices(
            &[QueuedSprite {
                sprite: SpriteDrawCommand {
                    entity_id: 1,
                    texture_id: 0,
                    x: 0.0,
                    y: 0.0,
                    width: 50.0,
                    height: 25.0,
                    rotation: 0.0,
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                uv_rect: [0.25, 0.5, 0.75, 1.0],
                clip_rect: None,
            }],
            100,
            100,
        );
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].position, [-1.0, 1.0]);
        assert_eq!(vertices[2].position, [0.0, 0.5]);
        assert_eq!(vertices[0].uv, [0.25, 0.5]);
        assert_eq!(vertices[2].uv, [0.75, 1.0]);
    }
}
