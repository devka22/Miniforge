//! Native `wgpu` renderer used by MiniForge's 2D migration path.
//!
//! The backend owns a real adapter, device, queue, render pipeline and color
//! target. It deliberately starts with an off-screen target so the same code is
//! usable by the Qt editor, headless tests and a future window surface without
//! tying the engine renderer to a particular windowing crate.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::mpsc;

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer as TextBuffer, Cache as TextCache, Color as TextColor, Family, FontSystem,
    Metrics, Resolution as TextResolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport as TextViewport, Wrap,
};
use serde::{Deserialize, Serialize};

use crate::engine::error_handler::{MFResult, MiniForgeError};

use super::backend::{
    BUILTIN_FLAT_NORMAL_TEXTURE_ID, BUILTIN_RADIAL_LIGHT_TEXTURE_ID,
    BUILTIN_RADIAL_LIGHT_TEXTURE_SIZE, CameraCommand3D, GraphicsApi, LightDrawCommand3D,
    MAX_RENDER_TARGET_SIZE_2D, MeshDrawCommand3D, ParticleDrawCommand, PostProcessCommand2D,
    RenderBackend, RenderDeviceCaps, RenderTargetDescriptor2D, SpriteBlendMode, SpriteDrawCommand,
    SpriteDrawOptions, SpriteMaterialEffect, SpriteRegionDrawCommand, TextDrawCommand,
    TextWrapMode, TilemapDrawCommand, UiDrawCommand, radial_light_texture_rgba8,
};

const OFFSCREEN_TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SPRITE_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SPRITE_COLOR_VIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;
const INITIAL_VERTEX_BUFFER_BYTES: u64 = 64 * 1024;
const MAX_TEXT_BYTES_PER_AREA: usize = 1024 * 1024;
const MAX_GPU_PARTICLES_PER_SYSTEM: usize = 1_000_000;
const GPU_PARTICLE_WORKGROUP_SIZE: u32 = 64;

const SPRITE_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) material_effect: u32,
    @location(3) effect_strength: f32,
    @location(4) normal_strength: f32,
    @location(5) @interpolate(flat) normal_flip_y: u32,
    @location(6) light_direction: vec2<f32>,
    @location(7) light_color: vec3<f32>,
    @location(8) ambient_light: f32,
};

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;
@group(1) @binding(0) var normal_texture: texture_2d<f32>;
@group(1) @binding(1) var normal_sampler: sampler;

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) material_effect: u32,
    @location(4) effect_strength: f32,
    @location(5) normal_strength: f32,
    @location(6) normal_flip_y: u32,
    @location(7) light_direction: vec2<f32>,
    @location(8) light_color: vec3<f32>,
    @location(9) ambient_light: f32,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    out.uv = uv;
    out.material_effect = material_effect;
    out.effect_strength = effect_strength;
    out.normal_strength = normal_strength;
    out.normal_flip_y = normal_flip_y;
    out.light_direction = light_direction;
    out.light_color = light_color;
    out.ambient_light = ambient_light;
    return out;
}

fn apply_normal_lighting(sampled: vec4<f32>, in: VertexOut) -> vec4<f32> {
    if in.normal_strength <= 0.0001 {
        return sampled;
    }
    let sampled_normal = textureSample(normal_texture, normal_sampler, in.uv).xyz * 2.0 - 1.0;
    let normal_y_sign = select(1.0, -1.0, in.normal_flip_y != 0u);
    let tangent_normal = normalize(vec3<f32>(
        sampled_normal.x * in.normal_strength,
        sampled_normal.y * in.normal_strength * normal_y_sign,
        max(sampled_normal.z, 0.02),
    ));
    let light_length = length(in.light_direction);
    let normalized_light = in.light_direction / max(light_length, 0.0001);
    let light_xy = select(
        vec2<f32>(0.0, -1.0),
        normalized_light,
        light_length > 0.0001,
    );
    let light = normalize(vec3<f32>(light_xy, 0.72));
    let diffuse = max(dot(tangent_normal, light), 0.0);
    let illumination = clamp(
        vec3<f32>(in.ambient_light) + in.light_color * diffuse,
        vec3<f32>(0.0),
        vec3<f32>(2.0),
    );
    return vec4<f32>(sampled.rgb * illumination, sampled.a);
}

fn apply_material_effect(
    sampled: vec4<f32>,
    material_effect: u32,
    strength: f32,
) -> vec4<f32> {
    var effected = sampled.rgb;
    if material_effect == 1u {
        let luminance = dot(sampled.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        effected = vec3<f32>(luminance);
    } else if material_effect == 2u {
        effected = vec3<f32>(
            dot(sampled.rgb, vec3<f32>(0.393, 0.769, 0.189)),
            dot(sampled.rgb, vec3<f32>(0.349, 0.686, 0.168)),
            dot(sampled.rgb, vec3<f32>(0.272, 0.534, 0.131)),
        );
    } else if material_effect == 3u {
        effected = vec3<f32>(1.0) - sampled.rgb;
    } else if material_effect == 4u {
        effected = vec3<f32>(1.0);
    }
    let mixed = mix(sampled.rgb, clamp(effected, vec3<f32>(0.0), vec3<f32>(1.0)), strength);
    return vec4<f32>(mixed, sampled.a);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
    let lit = apply_normal_lighting(sampled, in);
    return apply_material_effect(lit, in.material_effect, in.effect_strength);
}

@fragment
fn fs_premultiplied(in: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
    let lit = apply_normal_lighting(sampled, in);
    let effected = apply_material_effect(lit, in.material_effect, in.effect_strength);
    return vec4<f32>(effected.rgb * effected.a, effected.a);
}
"#;

const GPU_PARTICLE_SHADER: &str = r#"
struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    age: f32,
    lifetime: f32,
    seed: f32,
    alive: f32,
};

struct ParticleParams {
    origin: vec2<f32>,
    velocity: vec2<f32>,
    gravity: vec2<f32>,
    viewport: vec2<f32>,
    color: vec4<f32>,
    delta_seconds: f32,
    spread: f32,
    lifetime: f32,
    drag: f32,
    start_size: f32,
    end_size: f32,
    max_particles: u32,
    spawn_start: u32,
    spawn_count: u32,
    frame_seed: u32,
    _padding: vec2<u32>,
};

@group(0) @binding(0) var<storage, read_write> particles_rw: array<Particle>;
@group(0) @binding(1) var<uniform> compute_params: ParticleParams;
@group(1) @binding(0) var<storage, read> particles_ro: array<Particle>;
@group(1) @binding(1) var<uniform> render_params: ParticleParams;

fn hash(value: u32) -> f32 {
    var state = value;
    state = state ^ (state >> 16u);
    state = state * 2246822519u;
    state = state ^ (state >> 13u);
    state = state * 3266489917u;
    state = state ^ (state >> 16u);
    return f32(state & 0x00ffffffu) / 16777215.0;
}

fn should_spawn(index: u32) -> bool {
    if compute_params.spawn_count == 0u {
        return false;
    }
    let end = compute_params.spawn_start + compute_params.spawn_count;
    if end <= compute_params.max_particles {
        return index >= compute_params.spawn_start && index < end;
    }
    return index >= compute_params.spawn_start || index < end - compute_params.max_particles;
}

@compute @workgroup_size(64)
fn simulate(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if index >= compute_params.max_particles {
        return;
    }
    var particle = particles_rw[index];
    if particle.alive > 0.5 {
        particle.age = particle.age + compute_params.delta_seconds;
        if particle.age >= particle.lifetime {
            particle.alive = 0.0;
        } else {
            particle.velocity =
                particle.velocity + compute_params.gravity * compute_params.delta_seconds;
            particle.velocity = particle.velocity /
                (1.0 + max(compute_params.drag, 0.0) * compute_params.delta_seconds);
            particle.position =
                particle.position + particle.velocity * compute_params.delta_seconds;
        }
    }
    if should_spawn(index) {
        let random = hash(index ^ compute_params.frame_seed);
        let jitter = random * 2.0 - 1.0;
        particle.position = compute_params.origin;
        particle.velocity = compute_params.velocity +
            vec2<f32>(
                jitter * compute_params.spread,
                -abs(jitter) * compute_params.spread,
            );
        particle.age = 0.0;
        particle.lifetime =
            max(compute_params.lifetime * (0.75 + random * 0.5), 0.01);
        particle.seed = random;
        particle.alive = 1.0;
    }
    particles_rw[index] = particle;
}

struct ParticleVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
};

@vertex
fn vs_particle(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> ParticleVertexOut {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    let particle = particles_ro[instance_index];
    var out: ParticleVertexOut;
    if particle.alive < 0.5 {
        out.position = vec4<f32>(-2.0, -2.0, 0.0, 1.0);
        out.color = vec4<f32>(0.0);
        out.local = vec2<f32>(2.0);
        return out;
    }
    let progress = clamp(particle.age / max(particle.lifetime, 0.01), 0.0, 1.0);
    let size =
        max(mix(render_params.start_size, render_params.end_size, progress), 0.25);
    let local = corners[vertex_index];
    let pixel = particle.position + local * size * 0.5;
    let viewport = max(render_params.viewport, vec2<f32>(1.0));
    out.position = vec4<f32>(
        pixel.x / viewport.x * 2.0 - 1.0,
        1.0 - pixel.y / viewport.y * 2.0,
        0.0,
        1.0,
    );
    out.color =
        vec4<f32>(render_params.color.rgb, render_params.color.a * (1.0 - progress));
    out.local = local;
    return out;
}

@fragment
fn fs_particle(in: ParticleVertexOut) -> @location(0) vec4<f32> {
    let distance = length(in.local);
    let coverage = 1.0 - smoothstep(0.45, 1.0, distance);
    return vec4<f32>(in.color.rgb, in.color.a * coverage);
}
"#;

const POST_PROCESS_SHADER: &str = r#"
struct PostProcessParams {
    resolution_time: vec4<f32>,
    color_grade: vec4<f32>,
    bloom: vec4<f32>,
    vignette: vec4<f32>,
    screen_fx: vec4<f32>,
    tint: vec4<f32>,
    damage_flash: vec4<f32>,
    damage_fog: vec4<f32>,
    fog_color: vec4<f32>,
};

struct PostProcessVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var scene_color: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(1) @binding(0) var<uniform> params: PostProcessParams;

@vertex
fn vs_post_process(@builtin(vertex_index) vertex_index: u32) -> PostProcessVertexOut {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: PostProcessVertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn sample_bloom(uv: vec2<f32>, texel: vec2<f32>, radius: f32, threshold: f32) -> vec3<f32> {
    let offsets = array<vec2<f32>, 8>(
        vec2<f32>(-1.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, -1.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(-0.707, -0.707), vec2<f32>(0.707, -0.707),
        vec2<f32>(-0.707, 0.707), vec2<f32>(0.707, 0.707),
    );
    var result = vec3<f32>(0.0);
    for (var index = 0u; index < 8u; index = index + 1u) {
        let sampled = textureSample(scene_color, scene_sampler, uv + offsets[index] * texel * radius).rgb;
        let contribution = smoothstep(threshold, 1.0, luminance(sampled));
        result = result + sampled * contribution;
    }
    return result * 0.125;
}

fn noise(uv: vec2<f32>) -> f32 {
    return fract(sin(dot(uv, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs_post_process(in: PostProcessVertexOut) -> @location(0) vec4<f32> {
    let resolution = max(params.resolution_time.xy, vec2<f32>(1.0));
    let texel = 1.0 / resolution;
    let pixel_size = max(params.screen_fx.y, 1.0);
    var uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
    if pixel_size > 1.001 {
        uv = (floor(uv * resolution / pixel_size) + vec2<f32>(0.5)) * pixel_size / resolution;
    }

    let chromatic = max(params.screen_fx.x, 0.0);
    let red = textureSample(scene_color, scene_sampler, uv + vec2<f32>(chromatic, 0.0)).r;
    let center = textureSample(scene_color, scene_sampler, uv);
    let blue = textureSample(scene_color, scene_sampler, uv - vec2<f32>(chromatic, 0.0)).b;
    var color = vec3<f32>(red, center.g, blue);

    if params.bloom.y > 0.0001 {
        color = color + sample_bloom(uv, texel, max(params.bloom.z, 0.5), params.bloom.x)
            * params.bloom.y;
    }

    color = color * max(params.color_grade.x, 0.0);
    color = (color - vec3<f32>(0.5)) * params.color_grade.y + vec3<f32>(0.5);
    let gray = vec3<f32>(luminance(color));
    color = mix(gray, color, params.color_grade.z);
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / max(params.color_grade.w, 0.05)));
    color = color * params.tint.rgb;

    let fog_noise = noise(uv * resolution * 0.0125 + params.resolution_time.zz);
    let fog_amount = clamp(
        params.damage_fog.y * (0.35 + uv.y * 0.65) * mix(0.82, 1.18, fog_noise),
        0.0,
        1.0,
    );
    color = mix(color, params.fog_color.rgb, fog_amount * params.fog_color.a);
    color = mix(color, params.damage_flash.rgb, params.damage_fog.x * params.damage_flash.a);

    let scanline = 0.5 + 0.5 * sin(uv.y * resolution.y * 3.14159265);
    color = color * (1.0 - clamp(params.screen_fx.z, 0.0, 1.0) * scanline);

    let aspect = resolution.x / resolution.y;
    let centered = (uv - vec2<f32>(0.5)) * vec2<f32>(aspect, 1.0);
    let vignette_edge = smoothstep(params.vignette.y, 0.92, length(centered));
    color = color * (1.0 - clamp(params.vignette.x, 0.0, 1.0) * vignette_edge);
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(8.0)), center.a * params.tint.a);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    color: [f32; 4],
    uv: [f32; 2],
    material_effect: u32,
    effect_strength: f32,
    normal_strength: f32,
    normal_flip_y: u32,
    light_direction: [f32; 2],
    light_color: [f32; 3],
    ambient_light: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuParticle {
    position: [f32; 2],
    velocity: [f32; 2],
    age: f32,
    lifetime: f32,
    seed: f32,
    active: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuParticleParams {
    origin: [f32; 2],
    velocity: [f32; 2],
    gravity: [f32; 2],
    viewport: [f32; 2],
    color: [f32; 4],
    delta_seconds: f32,
    spread: f32,
    lifetime: f32,
    drag: f32,
    start_size: f32,
    end_size: f32,
    max_particles: u32,
    spawn_start: u32,
    spawn_count: u32,
    frame_seed: u32,
    _padding: [u32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuPostProcessParams {
    resolution_time: [f32; 4],
    color_grade: [f32; 4],
    bloom: [f32; 4],
    vignette: [f32; 4],
    screen_fx: [f32; 4],
    tint: [f32; 4],
    damage_flash: [f32; 4],
    damage_fog: [f32; 4],
    fog_color: [f32; 4],
}

#[derive(Clone, Copy)]
struct QueuedSprite {
    sprite: SpriteDrawCommand,
    uv_rect: [f32; 4],
    clip_rect: Option<[u32; 4]>,
    blend_mode: SpriteBlendMode,
    material_effect: SpriteMaterialEffect,
    effect_strength: u8,
    normal_texture_id: u64,
    normal_strength: u8,
    normal_flip_y: bool,
    light_direction: [i16; 2],
    light_color: [u8; 3],
    ambient_light: u8,
    render_target_pass: Option<usize>,
    viewport_size: [u32; 2],
}

#[derive(Debug, Clone)]
struct QueuedText {
    command: TextDrawCommand,
    render_target_pass: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpriteBatch {
    texture_id: u64,
    normal_texture_id: u64,
    render_target_pass: Option<usize>,
    clip_rect: [u32; 4],
    blend_mode: SpriteBlendMode,
    first_sprite: u32,
    sprite_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WgpuFrameDiagnostics {
    pub frame_index: u64,
    pub logical_draw_calls: usize,
    pub queued_sprites: usize,
    pub culled_sprites: usize,
    pub queued_text_areas: usize,
    pub culled_text_areas: usize,
    #[serde(default)]
    pub render_target_text_areas: usize,
    pub queued_particle_systems: usize,
    pub gpu_particle_capacity: usize,
    pub gpu_particle_spawned: usize,
    pub particle_compute_dispatches: usize,
    pub gpu_draw_calls: usize,
    pub texture_bind_changes: usize,
    #[serde(default)]
    pub normal_texture_bind_changes: usize,
    #[serde(default)]
    pub render_target_passes: usize,
    #[serde(default)]
    pub post_process_passes: usize,
    #[serde(default)]
    pub post_process_effects: usize,
    pub pipeline_changes: usize,
    pub vertex_bytes_uploaded: u64,
    pub vertex_buffer_capacity_bytes: u64,
    pub vertex_buffer_reallocations: u64,
    pub submitted: bool,
    pub surface_reconfigurations: u64,
    pub surface_loss_recoveries: u64,
    pub device_loss_recoveries: u64,
}

const SPRITE_ATTRIBUTES: [wgpu::VertexAttribute; 10] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x4,
    2 => Float32x2,
    3 => Uint32,
    4 => Float32,
    5 => Float32,
    6 => Uint32,
    7 => Float32x2,
    8 => Float32x3,
    9 => Float32
];

struct WgpuTexture {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    normal_bind_group: wgpu::BindGroup,
}

struct WgpuRenderTarget {
    texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    normal_bind_group: wgpu::BindGroup,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_viewport: TextViewport,
    width: u32,
    height: u32,
}

struct WgpuPostProcess {
    scene_texture: wgpu::Texture,
    scene_view: wgpu::TextureView,
    scene_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_viewport: TextViewport,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy)]
struct ActiveRenderTargetPass {
    target_id: u64,
    pass_index: usize,
    first_sprite: usize,
    first_text: usize,
}

#[derive(Debug, Clone, Copy)]
struct QueuedRenderTargetPass {
    target_id: u64,
    clear_color: [f64; 4],
    first_sprite: usize,
    sprite_count: usize,
    first_text: usize,
    text_count: usize,
}

struct GpuParticleSystem {
    _particle_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    capacity: u32,
    spawn_cursor: u32,
    emit_accumulator: f32,
    burst_emitted: bool,
}

#[derive(Debug, Clone, Copy)]
struct PreparedGpuParticleDraw {
    system_id: u64,
    capacity: u32,
    spawned: u32,
    blend_mode: SpriteBlendMode,
}

#[derive(Clone)]
struct WgpuTextureBackup {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

struct DeviceLossNotice {
    reason: String,
    message: String,
}

struct WgpuState {
    instance: wgpu::Instance,
    device: wgpu::Device,
    device_loss_rx: mpsc::Receiver<DeviceLossNotice>,
    queue: wgpu::Queue,
    pipelines: HashMap<SpriteBlendMode, wgpu::RenderPipeline>,
    render_target_pipelines: HashMap<SpriteBlendMode, wgpu::RenderPipeline>,
    target: Option<wgpu::Texture>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    texture_layout: wgpu::BindGroupLayout,
    normal_texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white_texture: WgpuTexture,
    textures: HashMap<u64, WgpuTexture>,
    render_targets: HashMap<u64, WgpuRenderTarget>,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_capacity_bytes: u64,
    particle_compute_bind_group_layout: wgpu::BindGroupLayout,
    particle_render_bind_group_layout: wgpu::BindGroupLayout,
    particle_compute_pipeline: wgpu::ComputePipeline,
    particle_render_pipelines: HashMap<SpriteBlendMode, wgpu::RenderPipeline>,
    post_process_particle_pipelines: HashMap<SpriteBlendMode, wgpu::RenderPipeline>,
    particle_systems: HashMap<u64, GpuParticleSystem>,
    post_process: WgpuPostProcess,
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_viewport: TextViewport,
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
    pub surface_loss_recoveries: u64,
    pub device_loss_recoveries: u64,
    pub vertex_buffer_reallocations: u64,
    pub last_device_loss: Option<String>,
    frame_open: bool,
    clear_color: [f64; 4],
    sprites: Vec<QueuedSprite>,
    texts: Vec<QueuedText>,
    particles: Vec<ParticleDrawCommand>,
    render_target_descriptors: HashMap<u64, RenderTargetDescriptor2D>,
    render_target_passes: Vec<QueuedRenderTargetPass>,
    active_render_target: Option<ActiveRenderTargetPass>,
    post_process_command: Option<PostProcessCommand2D>,
    culled_sprites: usize,
    culled_text_areas: usize,
    last_frame_diagnostics: WgpuFrameDiagnostics,
    texture_backups: HashMap<u64, WgpuTextureBackup>,
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
            .field("surface_loss_recoveries", &self.surface_loss_recoveries)
            .field("device_loss_recoveries", &self.device_loss_recoveries)
            .field(
                "vertex_buffer_reallocations",
                &self.vertex_buffer_reallocations,
            )
            .field("last_frame_diagnostics", &self.last_frame_diagnostics)
            .field("last_device_loss", &self.last_device_loss)
            .field("post_process_command", &self.post_process_command)
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
            surface_loss_recoveries: 0,
            device_loss_recoveries: 0,
            vertex_buffer_reallocations: 0,
            last_device_loss: None,
            frame_open: false,
            clear_color: [0.035, 0.043, 0.059, 1.0],
            sprites: Vec::new(),
            texts: Vec::new(),
            particles: Vec::new(),
            render_target_descriptors: HashMap::new(),
            render_target_passes: Vec::new(),
            active_render_target: None,
            post_process_command: None,
            culled_sprites: 0,
            culled_text_areas: 0,
            last_frame_diagnostics: WgpuFrameDiagnostics::default(),
            texture_backups: HashMap::new(),
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
        if matches!(
            texture_id,
            0 | BUILTIN_RADIAL_LIGHT_TEXTURE_ID | BUILTIN_FLAT_NORMAL_TEXTURE_ID
        ) {
            return Err(render_error(
                "texture id is reserved for a MiniForge built-in texture",
            ));
        }
        if self.render_target_descriptors.contains_key(&texture_id) {
            return Err(render_error(format!(
                "texture id {texture_id} is already used by a Render Target 2D"
            )));
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
            &state.normal_texture_layout,
            &state.sampler,
            width,
            height,
            pixels,
            "MiniForge uploaded sprite texture",
        );
        state.textures.insert(texture_id, texture);
        self.texture_backups.insert(
            texture_id,
            WgpuTextureBackup {
                width,
                height,
                pixels: pixels.to_vec(),
            },
        );
        Ok(())
    }

    pub fn remove_texture(&mut self, texture_id: u64) -> bool {
        if matches!(
            texture_id,
            BUILTIN_RADIAL_LIGHT_TEXTURE_ID | BUILTIN_FLAT_NORMAL_TEXTURE_ID
        ) {
            return false;
        }
        let removed_from_gpu = self
            .state
            .as_mut()
            .is_some_and(|state| state.textures.remove(&texture_id).is_some());
        let removed_from_backup = self.texture_backups.remove(&texture_id).is_some();
        removed_from_gpu || removed_from_backup
    }

    pub fn texture_count(&self) -> usize {
        self.texture_backups
            .keys()
            .filter(|&&texture_id| {
                !matches!(
                    texture_id,
                    BUILTIN_RADIAL_LIGHT_TEXTURE_ID | BUILTIN_FLAT_NORMAL_TEXTURE_ID
                )
            })
            .count()
    }

    pub fn render_target_count(&self) -> usize {
        self.render_target_descriptors.len()
    }

    pub fn last_frame_diagnostics(&self) -> &WgpuFrameDiagnostics {
        &self.last_frame_diagnostics
    }

    /// Copies the last submitted GPU target into tightly packed RGBA8 bytes.
    /// This is primarily for editor previews, screenshots and renderer tests.
    pub fn readback_rgba8(&self) -> MFResult<Vec<u8>> {
        let state = self.state()?;
        let target = state.target.as_ref().ok_or_else(|| {
            render_error("readback is only available for the off-screen wgpu target")
        })?;
        readback_texture_rgba8(state, target, self.width, self.height)
    }

    pub fn readback_render_target_rgba8(&self, texture_id: u64) -> MFResult<Vec<u8>> {
        let state = self.state()?;
        let target = state.render_targets.get(&texture_id).ok_or_else(|| {
            render_error(format!("render target texture {texture_id} does not exist"))
        })?;
        readback_texture_rgba8(state, &target.texture, target.width, target.height)
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
        let (device_loss_tx, device_loss_rx) = mpsc::channel();
        device.set_device_lost_callback(move |reason, message| {
            let _ = device_loss_tx.send(DeviceLossNotice {
                reason: format!("{reason:?}"),
                message,
            });
        });

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
        let normal_texture_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MiniForge linear normal texture layout"),
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
            &normal_texture_layout,
            &sampler,
            1,
            1,
            &[255, 255, 255, 255],
            "MiniForge white sprite texture",
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MiniForge wgpu 2D pipeline layout"),
            bind_group_layouts: &[Some(&texture_layout), Some(&normal_texture_layout)],
            immediate_size: 0,
        });
        let pipelines = [
            SpriteBlendMode::Alpha,
            SpriteBlendMode::PremultipliedAlpha,
            SpriteBlendMode::Additive,
            SpriteBlendMode::Multiply,
            SpriteBlendMode::Screen,
        ]
        .into_iter()
        .map(|blend_mode| {
            (
                blend_mode,
                create_sprite_pipeline(
                    &device,
                    &shader,
                    &pipeline_layout,
                    target_format,
                    blend_mode,
                ),
            )
        })
        .collect();
        let render_target_pipelines = [
            SpriteBlendMode::Alpha,
            SpriteBlendMode::PremultipliedAlpha,
            SpriteBlendMode::Additive,
            SpriteBlendMode::Multiply,
            SpriteBlendMode::Screen,
        ]
        .into_iter()
        .map(|blend_mode| {
            (
                blend_mode,
                create_sprite_pipeline(
                    &device,
                    &shader,
                    &pipeline_layout,
                    SPRITE_COLOR_VIEW_FORMAT,
                    blend_mode,
                ),
            )
        })
        .collect();
        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MiniForge GPU particle compute and render shader"),
            source: wgpu::ShaderSource::Wgsl(GPU_PARTICLE_SHADER.into()),
        });
        let particle_compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MiniForge GPU particle compute state layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let particle_render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MiniForge GPU particle render state layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let particle_compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MiniForge GPU particle compute pipeline layout"),
                bind_group_layouts: &[Some(&particle_compute_bind_group_layout)],
                immediate_size: 0,
            });
        let particle_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MiniForge GPU particle render pipeline layout"),
                bind_group_layouts: &[None, Some(&particle_render_bind_group_layout)],
                immediate_size: 0,
            });
        let particle_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MiniForge GPU particle simulation pipeline"),
                layout: Some(&particle_compute_pipeline_layout),
                module: &particle_shader,
                entry_point: Some("simulate"),
                compilation_options: Default::default(),
                cache: None,
            });
        let particle_render_pipelines = [
            SpriteBlendMode::Alpha,
            SpriteBlendMode::PremultipliedAlpha,
            SpriteBlendMode::Additive,
            SpriteBlendMode::Multiply,
            SpriteBlendMode::Screen,
        ]
        .into_iter()
        .map(|blend_mode| {
            (
                blend_mode,
                create_particle_render_pipeline(
                    &device,
                    &particle_shader,
                    &particle_render_pipeline_layout,
                    target_format,
                    blend_mode,
                ),
            )
        })
        .collect();
        let post_process_particle_pipelines = [
            SpriteBlendMode::Alpha,
            SpriteBlendMode::PremultipliedAlpha,
            SpriteBlendMode::Additive,
            SpriteBlendMode::Multiply,
            SpriteBlendMode::Screen,
        ]
        .into_iter()
        .map(|blend_mode| {
            (
                blend_mode,
                create_particle_render_pipeline(
                    &device,
                    &particle_shader,
                    &particle_render_pipeline_layout,
                    SPRITE_COLOR_VIEW_FORMAT,
                    blend_mode,
                ),
            )
        })
        .collect();
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let text_cache = TextCache::new(&device);
        let text_viewport = TextViewport::new(&device, &text_cache);
        let mut text_atlas = TextAtlas::new(&device, &queue, &text_cache, target_format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let post_process = create_post_process_resources(
            PostProcessCreateResources {
                device: &device,
                queue: &queue,
                texture_layout: &texture_layout,
            },
            self.width,
            self.height,
            target_format,
        );
        let target = surface
            .is_none()
            .then(|| create_target(&device, self.width, self.height));
        let vertex_buffer = create_vertex_buffer(&device, INITIAL_VERTEX_BUFFER_BYTES);
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
                instance: instance.clone(),
                device,
                device_loss_rx,
                queue,
                pipelines,
                render_target_pipelines,
                target,
                surface,
                surface_config,
                texture_layout,
                normal_texture_layout,
                sampler,
                white_texture,
                textures: HashMap::new(),
                render_targets: HashMap::new(),
                vertex_buffer,
                vertex_buffer_capacity_bytes: INITIAL_VERTEX_BUFFER_BYTES,
                particle_compute_bind_group_layout,
                particle_render_bind_group_layout,
                particle_compute_pipeline,
                particle_render_pipelines,
                post_process_particle_pipelines,
                particle_systems: HashMap::new(),
                post_process,
                font_system,
                swash_cache,
                text_atlas,
                text_renderer,
                text_viewport,
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
        self.ensure_builtin_texture_backups();
        match self.create_state(instance, surface) {
            Ok((mut state, caps)) => {
                restore_texture_backups(&mut state, &self.texture_backups);
                restore_render_targets(&mut state, &self.render_target_descriptors);
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

    fn recover_device_if_needed(&mut self) -> MFResult<()> {
        if let Some(state) = self.state.as_ref() {
            let _ = state.device.poll(wgpu::PollType::Poll);
        }
        let notice = match self
            .state
            .as_ref()
            .and_then(|state| state.device_loss_rx.try_recv().ok())
        {
            Some(notice) => notice,
            None => return Ok(()),
        };
        let mut previous = self
            .state
            .take()
            .ok_or_else(|| render_error("wgpu device recovery has no previous state"))?;
        let instance = previous.instance.clone();
        let surface = previous.surface.take();
        let had_surface = surface.is_some();
        drop(previous);

        let loss_message = if notice.message.trim().is_empty() {
            notice.reason
        } else {
            format!("{} · {}", notice.reason, notice.message)
        };
        self.ensure_builtin_texture_backups();
        match self.create_state(&instance, surface) {
            Ok((mut state, caps)) => {
                restore_texture_backups(&mut state, &self.texture_backups);
                restore_render_targets(&mut state, &self.render_target_descriptors);
                self.state = Some(state);
                self.caps = Some(caps);
                self.initialized = true;
                self.last_error = None;
                self.last_device_loss = Some(loss_message);
                self.device_loss_recoveries += 1;
                self.surface_reconfigurations += u64::from(had_surface);
                Ok(())
            }
            Err(error) => {
                self.initialized = false;
                self.last_error = Some(format!(
                    "wgpu device recovery failed after {loss_message}: {error}"
                ));
                Err(error)
            }
        }
    }

    fn ensure_builtin_texture_backups(&mut self) {
        self.texture_backups
            .entry(BUILTIN_RADIAL_LIGHT_TEXTURE_ID)
            .or_insert_with(|| WgpuTextureBackup {
                width: BUILTIN_RADIAL_LIGHT_TEXTURE_SIZE,
                height: BUILTIN_RADIAL_LIGHT_TEXTURE_SIZE,
                pixels: radial_light_texture_rgba8(BUILTIN_RADIAL_LIGHT_TEXTURE_SIZE),
            });
        self.texture_backups
            .entry(BUILTIN_FLAT_NORMAL_TEXTURE_ID)
            .or_insert_with(|| WgpuTextureBackup {
                width: 1,
                height: 1,
                pixels: vec![128, 128, 255, 255],
            });
    }

    #[doc(hidden)]
    pub fn force_device_loss_for_testing(&self) -> MFResult<()> {
        let state = self.state()?;
        state.device.destroy();
        let _ = state.device.poll(wgpu::PollType::Poll);
        Ok(())
    }

    fn state(&self) -> MFResult<&WgpuState> {
        self.state
            .as_ref()
            .ok_or_else(|| render_error("wgpu backend has not been initialized"))
    }

    fn queued_draw_target(&self) -> (Option<usize>, [u32; 2]) {
        let Some(active) = self.active_render_target else {
            return (None, [self.width.max(1), self.height.max(1)]);
        };
        let descriptor = self
            .render_target_descriptors
            .get(&active.target_id)
            .expect("active render target descriptor must remain registered");
        (
            Some(active.pass_index),
            [descriptor.width.max(1), descriptor.height.max(1)],
        )
    }

    fn queue_sprite(&mut self, mut queued: QueuedSprite) -> MFResult<()> {
        let sprite = &mut queued.sprite;
        if self.active_render_target.is_some_and(|active| {
            sprite.texture_id == active.target_id || queued.normal_texture_id == active.target_id
        }) {
            return Err(render_error(
                "a render target pass cannot sample from the texture it is currently writing",
            ));
        }
        if [
            sprite.x,
            sprite.y,
            sprite.width,
            sprite.height,
            sprite.rotation,
        ]
        .iter()
        .chain(sprite.color.iter())
        .any(|value| !value.is_finite())
        {
            return Err(render_error(
                "wgpu sprite geometry and color values must be finite",
            ));
        }
        if sprite.width <= 0.0 || sprite.height <= 0.0 {
            return Err(render_error(
                "wgpu sprite width and height must be greater than zero",
            ));
        }
        sprite.color = sprite.color.map(|channel| channel.clamp(0.0, 1.0));
        self.draw_calls += 1;
        if !sprite_intersects_viewport(sprite, queued.viewport_size[0], queued.viewport_size[1])
            || queued.clip_rect.is_some_and(|clip| {
                normalize_clip_rect(clip, queued.viewport_size[0], queued.viewport_size[1])
                    .is_none()
            })
        {
            self.culled_sprites += 1;
            return Ok(());
        }
        self.sprites.push(queued);
        Ok(())
    }

    fn render_frame(&mut self) -> MFResult<WgpuFrameDiagnostics> {
        let vertices = sprites_to_vertices(&self.sprites);
        let batches = build_sprite_batches(&self.sprites);
        let post_process_command = self.post_process_command.clone();
        let has_post_process = post_process_command.is_some();
        let post_process_effects = post_process_command
            .as_ref()
            .map_or(0, PostProcessCommand2D::active_effect_count);
        let render_target_passes = self.render_target_passes.clone();
        let render_target_batches = (0..render_target_passes.len())
            .map(|pass_index| {
                batches
                    .iter()
                    .filter(|batch| batch.render_target_pass == Some(pass_index))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let main_batches = batches
            .iter()
            .filter(|batch| batch.render_target_pass.is_none())
            .collect::<Vec<_>>();
        let render_target_text_commands = (0..render_target_passes.len())
            .map(|pass_index| {
                self.texts
                    .iter()
                    .filter(|queued| queued.render_target_pass == Some(pass_index))
                    .map(|queued| queued.command.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let main_text_commands = self
            .texts
            .iter()
            .filter(|queued| queued.render_target_pass.is_none())
            .map(|queued| queued.command.clone())
            .collect::<Vec<_>>();
        let particle_commands = &self.particles;
        let has_main_text = !main_text_commands.is_empty();
        let target_text_passes = render_target_text_commands
            .iter()
            .filter(|commands| !commands.is_empty())
            .count();
        let render_target_text_areas = render_target_text_commands.iter().map(Vec::len).sum();
        let mut main_text_buffers = Vec::with_capacity(main_text_commands.len());
        let mut render_target_text_buffers = Vec::with_capacity(render_target_text_commands.len());
        let vertex_bytes = std::mem::size_of_val(vertices.as_slice()) as u64;
        let clear_color = self.clear_color;
        let mut reallocated = false;
        let mut reconfiguration_count = 0u64;
        let mut surface_loss_recovery_count = 0u64;
        let submitted;
        let vertex_buffer_capacity_bytes;
        let gpu_particle_capacity;
        let gpu_particle_spawned;
        let mut particle_compute_dispatches = 0usize;
        {
            let state = self
                .state
                .as_mut()
                .ok_or_else(|| render_error("wgpu backend has not been initialized"))?;
            if vertex_bytes > state.vertex_buffer_capacity_bytes {
                let capacity = vertex_bytes
                    .next_power_of_two()
                    .max(INITIAL_VERTEX_BUFFER_BYTES);
                state.vertex_buffer = create_vertex_buffer(&state.device, capacity);
                state.vertex_buffer_capacity_bytes = capacity;
                reallocated = true;
            }
            if vertex_bytes > 0 {
                state
                    .queue
                    .write_buffer(&state.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            }
            if has_main_text {
                if has_post_process {
                    let WgpuState {
                        device,
                        queue,
                        font_system,
                        swash_cache,
                        post_process,
                        ..
                    } = state;
                    prepare_text_areas(
                        TextRenderResources {
                            device,
                            queue,
                            font_system,
                            swash_cache,
                            text_atlas: &mut post_process.text_atlas,
                            text_renderer: &mut post_process.text_renderer,
                            text_viewport: &mut post_process.text_viewport,
                        },
                        &main_text_commands,
                        &mut main_text_buffers,
                        self.width,
                        self.height,
                    )?;
                } else {
                    let WgpuState {
                        device,
                        queue,
                        font_system,
                        swash_cache,
                        text_atlas,
                        text_renderer,
                        text_viewport,
                        ..
                    } = state;
                    prepare_text_areas(
                        TextRenderResources {
                            device,
                            queue,
                            font_system,
                            swash_cache,
                            text_atlas,
                            text_renderer,
                            text_viewport,
                        },
                        &main_text_commands,
                        &mut main_text_buffers,
                        self.width,
                        self.height,
                    )?;
                }
            }
            for (pass, commands) in render_target_passes
                .iter()
                .zip(render_target_text_commands.iter())
            {
                let mut buffers = Vec::with_capacity(commands.len());
                if !commands.is_empty() {
                    let WgpuState {
                        device,
                        queue,
                        font_system,
                        swash_cache,
                        render_targets,
                        ..
                    } = state;
                    let target = render_targets.get_mut(&pass.target_id).ok_or_else(|| {
                        render_error(format!(
                            "render target texture {} disappeared before text preparation",
                            pass.target_id
                        ))
                    })?;
                    prepare_text_areas(
                        TextRenderResources {
                            device,
                            queue,
                            font_system,
                            swash_cache,
                            text_atlas: &mut target.text_atlas,
                            text_renderer: &mut target.text_renderer,
                            text_viewport: &mut target.text_viewport,
                        },
                        commands,
                        &mut buffers,
                        target.width,
                        target.height,
                    )?;
                }
                render_target_text_buffers.push(buffers);
            }
            let prepared_particles =
                prepare_gpu_particle_draws(state, particle_commands, self.width, self.height)?;
            if let Some(command) = post_process_command.as_ref() {
                let params = post_process_gpu_params(command, self.width, self.height);
                state.queue.write_buffer(
                    &state.post_process.uniform_buffer,
                    0,
                    bytemuck::bytes_of(&params),
                );
            }
            gpu_particle_capacity = prepared_particles
                .iter()
                .map(|draw| draw.capacity as usize)
                .sum();
            gpu_particle_spawned = prepared_particles
                .iter()
                .map(|draw| draw.spawned as usize)
                .sum();

            let mut reconfigure_after_present = false;
            let mut surface_output = if let Some(surface) = state.surface.as_ref() {
                let mut acquire = |surface: &wgpu::Surface<'static>| match surface
                    .get_current_texture()
                {
                    wgpu::CurrentSurfaceTexture::Success(texture) => Ok(Some(texture)),
                    wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                        reconfigure_after_present = true;
                        Ok(Some(texture))
                    }
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => Ok(None),
                    status @ (wgpu::CurrentSurfaceTexture::Outdated
                    | wgpu::CurrentSurfaceTexture::Lost) => {
                        if matches!(status, wgpu::CurrentSurfaceTexture::Lost) {
                            surface_loss_recovery_count += 1;
                        }
                        let config = state.surface_config.as_ref().ok_or_else(|| {
                            render_error("wgpu surface is missing its presentation configuration")
                        })?;
                        surface.configure(&state.device, config);
                        reconfiguration_count += 1;
                        match surface.get_current_texture() {
                            wgpu::CurrentSurfaceTexture::Success(texture) => Ok(Some(texture)),
                            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                                reconfigure_after_present = true;
                                Ok(Some(texture))
                            }
                            wgpu::CurrentSurfaceTexture::Timeout
                            | wgpu::CurrentSurfaceTexture::Occluded
                            | wgpu::CurrentSurfaceTexture::Outdated => Ok(None),
                            wgpu::CurrentSurfaceTexture::Lost => Ok(None),
                            wgpu::CurrentSurfaceTexture::Validation => Err(render_error(
                                "wgpu surface acquisition failed validation after reconfigure",
                            )),
                        }
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
                submitted = false;
            } else {
                let target_texture = surface_output
                    .as_ref()
                    .map(|output| &output.texture)
                    .or(state.target.as_ref())
                    .ok_or_else(|| render_error("wgpu frame has no render target"))?;
                let view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    state
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("MiniForge wgpu 2D frame encoder"),
                        });
                if !prepared_particles.is_empty() {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("MiniForge GPU particle simulation"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_pipeline(&state.particle_compute_pipeline);
                    for draw in &prepared_particles {
                        let particle_system = state
                            .particle_systems
                            .get(&draw.system_id)
                            .expect("prepared GPU particle system must remain resident");
                        compute_pass.set_bind_group(0, &particle_system.compute_bind_group, &[]);
                        compute_pass.dispatch_workgroups(
                            draw.capacity.div_ceil(GPU_PARTICLE_WORKGROUP_SIZE),
                            1,
                            1,
                        );
                    }
                    particle_compute_dispatches = prepared_particles.len();
                }
                for (pass_index, target_pass) in render_target_passes.iter().enumerate() {
                    let target = state
                        .render_targets
                        .get(&target_pass.target_id)
                        .ok_or_else(|| {
                            render_error(format!(
                                "render target texture {} disappeared before submission",
                                target_pass.target_id
                            ))
                        })?;
                    debug_assert_eq!(
                        render_target_batches[pass_index]
                            .iter()
                            .map(|batch| batch.sprite_count as usize)
                            .sum::<usize>(),
                        target_pass.sprite_count
                    );
                    debug_assert!(
                        render_target_batches[pass_index]
                            .first()
                            .is_none_or(
                                |batch| batch.first_sprite as usize >= target_pass.first_sprite
                            )
                    );
                    debug_assert_eq!(
                        render_target_text_commands[pass_index].len(),
                        target_pass.text_count
                    );
                    debug_assert!(
                        target_pass.text_count == 0 || target_pass.first_text < self.texts.len()
                    );
                    let color_attachment = wgpu::RenderPassColorAttachment {
                        view: &target.color_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: target_pass.clear_color[0],
                                g: target_pass.clear_color[1],
                                b: target_pass.clear_color[2],
                                a: target_pass.clear_color[3],
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    };
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("MiniForge Render Target 2D sprite and text pass"),
                        color_attachments: &[Some(color_attachment)],
                        ..Default::default()
                    });
                    draw_sprite_batches(
                        &mut pass,
                        state,
                        &state.render_target_pipelines,
                        &render_target_batches[pass_index],
                        vertex_bytes,
                    );
                    if !render_target_text_commands[pass_index].is_empty() {
                        target
                            .text_renderer
                            .render(&target.text_atlas, &target.text_viewport, &mut pass)
                            .map_err(|error| {
                                render_error(format!(
                                    "wgpu render-target text render failed: {error}"
                                ))
                            })?;
                    }
                }
                {
                    let main_view = if has_post_process {
                        &state.post_process.scene_view
                    } else {
                        &view
                    };
                    let color_attachment = wgpu::RenderPassColorAttachment {
                        view: main_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: clear_color[0],
                                g: clear_color[1],
                                b: clear_color[2],
                                a: clear_color[3],
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    };
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("MiniForge wgpu 2D frame"),
                        color_attachments: &[Some(color_attachment)],
                        ..Default::default()
                    });
                    draw_sprite_batches(
                        &mut pass,
                        state,
                        if has_post_process {
                            &state.render_target_pipelines
                        } else {
                            &state.pipelines
                        },
                        &main_batches,
                        vertex_bytes,
                    );
                    for draw in &prepared_particles {
                        let pipelines = if has_post_process {
                            &state.post_process_particle_pipelines
                        } else {
                            &state.particle_render_pipelines
                        };
                        let pipeline = pipelines
                            .get(&draw.blend_mode)
                            .expect("every particle blend mode must have a pipeline");
                        let particle_system = state
                            .particle_systems
                            .get(&draw.system_id)
                            .expect("prepared GPU particle system must remain resident");
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(1, &particle_system.render_bind_group, &[]);
                        pass.set_scissor_rect(0, 0, self.width.max(1), self.height.max(1));
                        pass.draw(0..6, 0..draw.capacity);
                    }
                    if has_main_text {
                        if has_post_process {
                            state
                                .post_process
                                .text_renderer
                                .render(
                                    &state.post_process.text_atlas,
                                    &state.post_process.text_viewport,
                                    &mut pass,
                                )
                                .map_err(|error| {
                                    render_error(format!(
                                        "wgpu post-process scene text render failed: {error}"
                                    ))
                                })?;
                        } else {
                            state
                                .text_renderer
                                .render(&state.text_atlas, &state.text_viewport, &mut pass)
                                .map_err(|error| {
                                    render_error(format!("wgpu text render failed: {error}"))
                                })?;
                        }
                    }
                }
                if has_post_process {
                    let color_attachment = wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    };
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("MiniForge WGPU post-process composite"),
                        color_attachments: &[Some(color_attachment)],
                        ..Default::default()
                    });
                    pass.set_pipeline(&state.post_process.pipeline);
                    pass.set_bind_group(0, &state.post_process.scene_bind_group, &[]);
                    pass.set_bind_group(1, &state.post_process.uniform_bind_group, &[]);
                    pass.draw(0..3, 0..1);
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
                if has_main_text {
                    if has_post_process {
                        state.post_process.text_atlas.trim();
                    } else {
                        state.text_atlas.trim();
                    }
                }
                for (target_pass, _commands) in render_target_passes
                    .iter()
                    .zip(render_target_text_commands.iter())
                    .filter(|(_, commands)| !commands.is_empty())
                {
                    if let Some(target) = state.render_targets.get_mut(&target_pass.target_id) {
                        target.text_atlas.trim();
                    }
                }
                submitted = true;
            }
            vertex_buffer_capacity_bytes = state.vertex_buffer_capacity_bytes;
        }

        self.surface_reconfigurations += reconfiguration_count;
        self.surface_loss_recoveries += surface_loss_recovery_count;
        self.vertex_buffer_reallocations += u64::from(reallocated);
        Ok(WgpuFrameDiagnostics {
            frame_index: self.submitted_frames + self.skipped_surface_frames + 1,
            logical_draw_calls: self.draw_calls,
            queued_sprites: self.sprites.len(),
            culled_sprites: self.culled_sprites,
            queued_text_areas: self.texts.len(),
            culled_text_areas: self.culled_text_areas,
            render_target_text_areas,
            queued_particle_systems: self.particles.len(),
            gpu_particle_capacity,
            gpu_particle_spawned,
            particle_compute_dispatches,
            gpu_draw_calls: batches.len()
                + self.particles.len()
                + usize::from(has_main_text)
                + target_text_passes
                + usize::from(has_post_process),
            texture_bind_changes: texture_bind_changes(&batches),
            normal_texture_bind_changes: normal_texture_bind_changes(&batches),
            render_target_passes: render_target_passes.len(),
            post_process_passes: usize::from(has_post_process),
            post_process_effects,
            pipeline_changes: pipeline_changes(&batches)
                + self.particles.len()
                + usize::from(has_main_text)
                + target_text_passes
                + usize::from(has_post_process),
            vertex_bytes_uploaded: vertex_bytes,
            vertex_buffer_capacity_bytes,
            vertex_buffer_reallocations: self.vertex_buffer_reallocations,
            submitted,
            surface_reconfigurations: reconfiguration_count,
            surface_loss_recoveries: surface_loss_recovery_count,
            device_loss_recoveries: self.device_loss_recoveries,
        })
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
        self.recover_device_if_needed()?;
        self.state()?;
        self.sprites.clear();
        self.texts.clear();
        self.particles.clear();
        self.render_target_passes.clear();
        self.active_render_target = None;
        self.post_process_command = None;
        self.draw_calls = 0;
        self.culled_sprites = 0;
        self.culled_text_areas = 0;
        self.frame_open = true;
        Ok(())
    }

    fn end_frame(&mut self) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error("wgpu end_frame called without begin_frame"));
        }
        if self.active_render_target.is_some() {
            return Err(render_error(
                "wgpu end_frame called before end_render_target_2d",
            ));
        }
        let diagnostics = self.render_frame();
        self.frame_open = false;
        let diagnostics = diagnostics?;
        if diagnostics.submitted {
            self.submitted_frames += 1;
        } else {
            self.skipped_surface_frames += 1;
        }
        self.last_frame_diagnostics = diagnostics;
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
            resize_post_process_scene(
                &state.device,
                &state.texture_layout,
                &mut state.post_process,
                self.width,
                self.height,
            );
        }
        Ok(())
    }

    fn draw_sprite(&mut self, command: SpriteDrawCommand) -> MFResult<()> {
        self.draw_sprite_with_options(command, SpriteDrawOptions::default())
    }

    fn draw_sprite_with_options(
        &mut self,
        command: SpriteDrawCommand,
        options: SpriteDrawOptions,
    ) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error("wgpu draw_sprite called outside a frame"));
        }
        let (render_target_pass, viewport_size) = self.queued_draw_target();
        self.queue_sprite(QueuedSprite {
            sprite: command,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            clip_rect: None,
            blend_mode: options.blend_mode,
            material_effect: options.material_effect,
            effect_strength: options.effect_strength,
            normal_texture_id: options
                .normal_texture_id
                .unwrap_or(BUILTIN_FLAT_NORMAL_TEXTURE_ID),
            normal_strength: options.normal_strength,
            normal_flip_y: options.normal_flip_y,
            light_direction: options.light_direction,
            light_color: options.light_color,
            ambient_light: options.ambient_light,
            render_target_pass,
            viewport_size,
        })
    }

    fn draw_sprite_region(&mut self, command: SpriteRegionDrawCommand) -> MFResult<()> {
        self.draw_sprite_region_with_options(command, SpriteDrawOptions::default())
    }

    fn draw_sprite_region_with_options(
        &mut self,
        command: SpriteRegionDrawCommand,
        options: SpriteDrawOptions,
    ) -> MFResult<()> {
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
        let (render_target_pass, viewport_size) = self.queued_draw_target();
        self.queue_sprite(QueuedSprite {
            sprite: command.sprite,
            uv_rect: command.uv_rect.map(|value| value.clamp(0.0, 1.0)),
            clip_rect: command.clip_rect,
            blend_mode: options.blend_mode,
            material_effect: options.material_effect,
            effect_strength: options.effect_strength,
            normal_texture_id: options
                .normal_texture_id
                .unwrap_or(BUILTIN_FLAT_NORMAL_TEXTURE_ID),
            normal_strength: options.normal_strength,
            normal_flip_y: options.normal_flip_y,
            light_direction: options.light_direction,
            light_color: options.light_color,
            ambient_light: options.ambient_light,
            render_target_pass,
            viewport_size,
        })
    }

    fn draw_text(&mut self, command: TextDrawCommand) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error("wgpu draw_text called outside a frame"));
        }
        if command.text.is_empty() {
            return Ok(());
        }
        if command.text.len() > MAX_TEXT_BYTES_PER_AREA {
            return Err(render_error(format!(
                "wgpu text area {} exceeds the {} byte safety limit",
                command.text_id, MAX_TEXT_BYTES_PER_AREA
            )));
        }
        if [
            command.x,
            command.y,
            command.width,
            command.height,
            command.font_size,
            command.line_height,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || command.width <= 0.0
            || command.height <= 0.0
            || command.font_size <= 0.0
            || command.line_height <= 0.0
        {
            return Err(render_error(
                "wgpu text geometry and metrics must be finite and greater than zero",
            ));
        }
        let (render_target_pass, viewport_size) = self.queued_draw_target();
        self.draw_calls += 1;
        if command.x + command.width < 0.0
            || command.y + command.height < 0.0
            || command.x > viewport_size[0] as f32
            || command.y > viewport_size[1] as f32
            || command.clip_rect.is_some_and(|clip| {
                normalize_clip_rect(clip, viewport_size[0], viewport_size[1]).is_none()
            })
        {
            self.culled_text_areas += 1;
            return Ok(());
        }
        self.texts.push(QueuedText {
            command,
            render_target_pass,
        });
        Ok(())
    }

    fn draw_tilemap(&mut self, _command: TilemapDrawCommand) -> MFResult<()> {
        if self.active_render_target.is_some() {
            return Err(render_error(
                "native tilemap commands inside a Render Target 2D pass are not supported; expand visible tiles to sprite quads",
            ));
        }
        self.draw_calls += 1;
        Ok(())
    }

    fn draw_particles(&mut self, mut command: ParticleDrawCommand) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error("wgpu draw_particles called outside a frame"));
        }
        if self.active_render_target.is_some() {
            return Err(render_error(
                "compute particles inside a Render Target 2D pass are not supported yet",
            ));
        }
        if command.system_id == 0 {
            return Err(render_error("wgpu particle system id must be non-zero"));
        }
        let finite_values = command
            .origin
            .iter()
            .chain(command.velocity.iter())
            .chain(command.gravity.iter())
            .chain(command.color.iter())
            .copied()
            .chain([
                command.spread,
                command.lifetime,
                command.drag,
                command.start_size,
                command.end_size,
                command.emission_rate,
                command.delta_seconds,
            ]);
        if finite_values.into_iter().any(|value| !value.is_finite()) {
            return Err(render_error("wgpu particle emitter values must be finite"));
        }
        command.particle_count = command
            .particle_count
            .clamp(1, MAX_GPU_PARTICLES_PER_SYSTEM);
        command.lifetime = command.lifetime.clamp(0.01, 3_600.0);
        command.delta_seconds = command.delta_seconds.clamp(0.0, 0.1);
        command.spread = command.spread.max(0.0);
        command.drag = command.drag.max(0.0);
        command.start_size = command.start_size.max(0.25);
        command.end_size = command.end_size.max(0.25);
        command.emission_rate = command.emission_rate.clamp(0.0, 1_000_000.0);
        command.color = command.color.map(|channel| channel.clamp(0.0, 1.0));
        self.draw_calls += 1;
        self.particles.push(command);
        Ok(())
    }

    fn draw_ui(&mut self, _command: UiDrawCommand) -> MFResult<()> {
        if self.active_render_target.is_some() {
            return Err(render_error(
                "legacy UI commands inside a Render Target 2D pass are not supported",
            ));
        }
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

    fn create_render_target_2d(
        &mut self,
        mut descriptor: RenderTargetDescriptor2D,
    ) -> MFResult<()> {
        if self.frame_open {
            return Err(render_error(
                "render targets must be created outside an open frame",
            ));
        }
        if matches!(
            descriptor.texture_id,
            0 | BUILTIN_RADIAL_LIGHT_TEXTURE_ID | BUILTIN_FLAT_NORMAL_TEXTURE_ID
        ) {
            return Err(render_error(
                "render target texture id is reserved by MiniForge",
            ));
        }
        if self.texture_backups.contains_key(&descriptor.texture_id) {
            return Err(render_error(format!(
                "render target texture id {} is already used by an uploaded texture",
                descriptor.texture_id
            )));
        }
        let device_limit = self
            .caps
            .as_ref()
            .map_or(MAX_RENDER_TARGET_SIZE_2D, |caps| caps.max_texture_size)
            .min(MAX_RENDER_TARGET_SIZE_2D);
        if descriptor.width == 0
            || descriptor.height == 0
            || descriptor.width > device_limit
            || descriptor.height > device_limit
        {
            return Err(render_error(format!(
                "render target {} dimensions must be between 1 and {device_limit}; got {}x{}",
                descriptor.texture_id, descriptor.width, descriptor.height
            )));
        }
        if descriptor
            .clear_color
            .iter()
            .any(|channel| !channel.is_finite())
        {
            return Err(render_error("render target clear color must be finite"));
        }
        descriptor.clear_color = descriptor
            .clear_color
            .map(|channel| channel.clamp(0.0, 1.0));
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| render_error("wgpu backend has not been initialized"))?;
        let target = create_render_target_texture(
            RenderTargetTextureResources {
                device: &state.device,
                queue: &state.queue,
                color_layout: &state.texture_layout,
                normal_layout: &state.normal_texture_layout,
                sampler: &state.sampler,
            },
            descriptor.width,
            descriptor.height,
            if descriptor.label.trim().is_empty() {
                "MiniForge Render Target 2D"
            } else {
                &descriptor.label
            },
        );
        state.render_targets.insert(descriptor.texture_id, target);
        self.render_target_descriptors
            .insert(descriptor.texture_id, descriptor);
        Ok(())
    }

    fn remove_render_target_2d(&mut self, texture_id: u64) -> MFResult<bool> {
        if self.frame_open {
            return Err(render_error(
                "render targets must be removed outside an open frame",
            ));
        }
        let removed_gpu = self
            .state
            .as_mut()
            .is_some_and(|state| state.render_targets.remove(&texture_id).is_some());
        let removed_descriptor = self.render_target_descriptors.remove(&texture_id).is_some();
        Ok(removed_gpu || removed_descriptor)
    }

    fn begin_render_target_2d(&mut self, texture_id: u64) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error(
                "begin_render_target_2d called outside a frame",
            ));
        }
        if self.active_render_target.is_some() {
            return Err(render_error(
                "nested Render Target 2D passes are not supported",
            ));
        }
        if self
            .sprites
            .iter()
            .any(|sprite| sprite.render_target_pass.is_none())
            || self
                .texts
                .iter()
                .any(|text| text.render_target_pass.is_none())
            || !self.particles.is_empty()
        {
            return Err(render_error(
                "Render Target 2D passes must be queued before main-frame sprites, text and particles",
            ));
        }
        if self
            .render_target_passes
            .iter()
            .any(|pass| pass.target_id == texture_id)
        {
            return Err(render_error(format!(
                "render target texture {texture_id} can be written at most once per frame"
            )));
        }
        if !self.render_target_descriptors.contains_key(&texture_id) {
            return Err(render_error(format!(
                "render target texture {texture_id} does not exist"
            )));
        }
        self.active_render_target = Some(ActiveRenderTargetPass {
            target_id: texture_id,
            pass_index: self.render_target_passes.len(),
            first_sprite: self.sprites.len(),
            first_text: self.texts.len(),
        });
        Ok(())
    }

    fn end_render_target_2d(&mut self) -> MFResult<()> {
        let Some(active) = self.active_render_target.take() else {
            return Err(render_error(
                "end_render_target_2d called without an active target",
            ));
        };
        let descriptor = self
            .render_target_descriptors
            .get(&active.target_id)
            .expect("active render target descriptor must remain registered");
        self.render_target_passes.push(QueuedRenderTargetPass {
            target_id: active.target_id,
            clear_color: descriptor.clear_color,
            first_sprite: active.first_sprite,
            sprite_count: self.sprites.len().saturating_sub(active.first_sprite),
            first_text: active.first_text,
            text_count: self.texts.len().saturating_sub(active.first_text),
        });
        Ok(())
    }

    fn set_post_process_2d(&mut self, command: PostProcessCommand2D) -> MFResult<()> {
        if !self.frame_open {
            return Err(render_error(
                "wgpu set_post_process_2d called outside a frame",
            ));
        }
        if self.active_render_target.is_some() {
            return Err(render_error(
                "post-processing config cannot change inside a Render Target 2D pass",
            ));
        }
        let command = sanitize_post_process_command(command)?;
        self.post_process_command =
            (command.enabled && command.active_effect_count() > 0).then_some(command);
        Ok(())
    }
}

fn draw_sprite_batches<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    state: &'pass WgpuState,
    pipelines: &'pass HashMap<SpriteBlendMode, wgpu::RenderPipeline>,
    batches: &[&SpriteBatch],
    vertex_bytes: u64,
) {
    if batches.is_empty() {
        return;
    }
    pass.set_vertex_buffer(0, state.vertex_buffer.slice(..vertex_bytes));
    let mut bound_pipeline = None;
    let mut bound_texture = None;
    let mut bound_normal_texture = None;
    for batch in batches {
        if bound_pipeline != Some(batch.blend_mode) {
            let pipeline = pipelines
                .get(&batch.blend_mode)
                .expect("every sprite blend mode must have a pipeline");
            pass.set_pipeline(pipeline);
            bound_pipeline = Some(batch.blend_mode);
        }
        if bound_texture != Some(batch.texture_id) {
            let bind_group = state
                .textures
                .get(&batch.texture_id)
                .map(|texture| &texture.bind_group)
                .or_else(|| {
                    state
                        .render_targets
                        .get(&batch.texture_id)
                        .map(|target| &target.bind_group)
                })
                .unwrap_or(&state.white_texture.bind_group);
            pass.set_bind_group(0, bind_group, &[]);
            bound_texture = Some(batch.texture_id);
        }
        if bound_normal_texture != Some(batch.normal_texture_id) {
            let bind_group = state
                .textures
                .get(&batch.normal_texture_id)
                .map(|texture| &texture.normal_bind_group)
                .or_else(|| {
                    state
                        .render_targets
                        .get(&batch.normal_texture_id)
                        .map(|target| &target.normal_bind_group)
                })
                .or_else(|| {
                    state
                        .textures
                        .get(&BUILTIN_FLAT_NORMAL_TEXTURE_ID)
                        .map(|texture| &texture.normal_bind_group)
                })
                .unwrap_or(&state.white_texture.normal_bind_group);
            pass.set_bind_group(1, bind_group, &[]);
            bound_normal_texture = Some(batch.normal_texture_id);
        }
        let [x, y, clip_width, clip_height] = batch.clip_rect;
        pass.set_scissor_rect(x, y, clip_width, clip_height);
        let first_vertex = batch.first_sprite * 6;
        let vertex_count = batch.sprite_count * 6;
        pass.draw(first_vertex..first_vertex + vertex_count, 0..1);
    }
}

fn create_sprite_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    blend_mode: SpriteBlendMode,
) -> wgpu::RenderPipeline {
    let (label, fragment_entry) = match blend_mode {
        SpriteBlendMode::Alpha => ("MiniForge sprite alpha pipeline", "fs_main"),
        SpriteBlendMode::PremultipliedAlpha => (
            "MiniForge sprite premultiplied pipeline",
            "fs_premultiplied",
        ),
        SpriteBlendMode::Additive => ("MiniForge sprite additive pipeline", "fs_main"),
        SpriteBlendMode::Multiply => ("MiniForge sprite multiply pipeline", "fs_premultiplied"),
        SpriteBlendMode::Screen => ("MiniForge sprite screen pipeline", "fs_premultiplied"),
    };
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &SPRITE_ATTRIBUTES,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
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
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(sprite_blend_state(blend_mode)),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_particle_render_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    blend_mode: SpriteBlendMode,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("MiniForge GPU particle render pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_particle"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_particle"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(sprite_blend_state(blend_mode)),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn prepare_gpu_particle_draws(
    state: &mut WgpuState,
    commands: &[ParticleDrawCommand],
    width: u32,
    height: u32,
) -> MFResult<Vec<PreparedGpuParticleDraw>> {
    let live_systems = commands
        .iter()
        .map(|command| command.system_id)
        .collect::<HashSet<_>>();
    state
        .particle_systems
        .retain(|system_id, _| live_systems.contains(system_id));

    let mut prepared = Vec::with_capacity(commands.len());
    for command in commands {
        let capacity = command
            .particle_count
            .clamp(1, MAX_GPU_PARTICLES_PER_SYSTEM) as u32;
        let rebuild = state
            .particle_systems
            .get(&command.system_id)
            .is_none_or(|system| system.capacity != capacity);
        if rebuild {
            let particle_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("MiniForge persistent GPU particle state"),
                size: u64::from(capacity) * std::mem::size_of::<GpuParticle>() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let params_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("MiniForge GPU particle emitter parameters"),
                size: std::mem::size_of::<GpuParticleParams>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let compute_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MiniForge GPU particle compute bindings"),
                layout: &state.particle_compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });
            let render_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MiniForge GPU particle render bindings"),
                layout: &state.particle_render_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });
            state.particle_systems.insert(
                command.system_id,
                GpuParticleSystem {
                    _particle_buffer: particle_buffer,
                    params_buffer,
                    compute_bind_group,
                    render_bind_group,
                    capacity,
                    spawn_cursor: 0,
                    emit_accumulator: 0.0,
                    burst_emitted: false,
                },
            );
        }

        let system = state
            .particle_systems
            .get_mut(&command.system_id)
            .expect("GPU particle system was just created");
        let spawn_start = system.spawn_cursor;
        let mut spawn_count = 0u32;
        let simulation_dt = if command.playing {
            command.delta_seconds.clamp(0.0, 0.1)
        } else {
            0.0
        };
        if command.playing {
            system.emit_accumulator += command.emission_rate.max(0.0) * simulation_dt;
            spawn_count = system.emit_accumulator.floor().max(0.0) as u32;
            system.emit_accumulator -= spawn_count as f32;
            if !system.burst_emitted {
                spawn_count = spawn_count.saturating_add(command.burst_count);
                system.burst_emitted = true;
            }
            spawn_count = spawn_count.min(capacity);
            system.spawn_cursor = (spawn_start + spawn_count) % capacity;
        }
        let params = GpuParticleParams {
            origin: command.origin,
            velocity: command.velocity,
            gravity: command.gravity,
            viewport: [width.max(1) as f32, height.max(1) as f32],
            color: command.color,
            delta_seconds: simulation_dt,
            spread: command.spread,
            lifetime: command.lifetime,
            drag: command.drag,
            start_size: command.start_size,
            end_size: command.end_size,
            max_particles: capacity,
            spawn_start,
            spawn_count,
            frame_seed: (command.system_id as u32)
                .wrapping_mul(747_796_405)
                .wrapping_add(system.spawn_cursor.wrapping_mul(2_891_336_453)),
            _padding: [0; 2],
        };
        state
            .queue
            .write_buffer(&system.params_buffer, 0, bytemuck::bytes_of(&params));
        prepared.push(PreparedGpuParticleDraw {
            system_id: command.system_id,
            capacity,
            spawned: spawn_count,
            blend_mode: command.blend_mode,
        });
    }
    Ok(prepared)
}

fn sprite_blend_state(blend_mode: SpriteBlendMode) -> wgpu::BlendState {
    match blend_mode {
        SpriteBlendMode::Alpha => wgpu::BlendState::ALPHA_BLENDING,
        SpriteBlendMode::PremultipliedAlpha => wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        SpriteBlendMode::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
        SpriteBlendMode::Multiply => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        },
        SpriteBlendMode::Screen => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrc,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        },
    }
}

struct TextRenderResources<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    font_system: &'a mut FontSystem,
    swash_cache: &'a mut SwashCache,
    text_atlas: &'a mut TextAtlas,
    text_renderer: &'a mut TextRenderer,
    text_viewport: &'a mut TextViewport,
}

fn prepare_text_areas(
    resources: TextRenderResources<'_>,
    commands: &[TextDrawCommand],
    buffers: &mut Vec<TextBuffer>,
    width: u32,
    height: u32,
) -> MFResult<()> {
    let TextRenderResources {
        device,
        queue,
        font_system,
        swash_cache,
        text_atlas,
        text_renderer,
        text_viewport,
    } = resources;
    text_viewport.update(queue, TextResolution { width, height });
    for command in commands {
        let mut buffer = TextBuffer::new(
            font_system,
            Metrics::new(command.font_size, command.line_height),
        );
        buffer.set_size(Some(command.width), Some(command.height));
        buffer.set_wrap(match command.wrap {
            TextWrapMode::None => Wrap::None,
            TextWrapMode::Word => Wrap::Word,
            TextWrapMode::Glyph => Wrap::Glyph,
        });
        let attrs = if command.font_family.trim().is_empty() {
            Attrs::new().family(Family::SansSerif)
        } else {
            Attrs::new().family(Family::Name(&command.font_family))
        };
        buffer.set_text(&command.text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(font_system, false);
        buffers.push(buffer);
    }
    let text_areas = commands
        .iter()
        .zip(buffers.iter())
        .map(|(command, buffer)| TextArea {
            buffer,
            left: command.x,
            top: command.y,
            scale: 1.0,
            bounds: text_bounds(command, width, height),
            default_color: TextColor::rgba(
                command.color[0],
                command.color[1],
                command.color[2],
                command.color[3],
            ),
            custom_glyphs: &[],
        });
    text_renderer
        .prepare(
            device,
            queue,
            font_system,
            text_atlas,
            text_viewport,
            text_areas,
            swash_cache,
        )
        .map_err(|error| render_error(format!("wgpu text preparation failed: {error}")))
}

fn text_bounds(command: &TextDrawCommand, width: u32, height: u32) -> TextBounds {
    let [x, y, area_width, area_height] = command
        .clip_rect
        .and_then(|clip| normalize_clip_rect(clip, width, height))
        .unwrap_or_else(|| {
            let left = command.x.max(0.0).floor().min(width as f32) as u32;
            let top = command.y.max(0.0).floor().min(height as f32) as u32;
            let right = (command.x + command.width)
                .max(0.0)
                .ceil()
                .min(width as f32) as u32;
            let bottom = (command.y + command.height)
                .max(0.0)
                .ceil()
                .min(height as f32) as u32;
            [
                left,
                top,
                right.saturating_sub(left),
                bottom.saturating_sub(top),
            ]
        });
    TextBounds {
        left: x as i32,
        top: y as i32,
        right: x.saturating_add(area_width) as i32,
        bottom: y.saturating_add(area_height) as i32,
    }
}

fn create_vertex_buffer(device: &wgpu::Device, capacity_bytes: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("MiniForge persistent wgpu 2D sprite vertices"),
        size: capacity_bytes.max(1),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn restore_texture_backups(
    state: &mut WgpuState,
    texture_backups: &HashMap<u64, WgpuTextureBackup>,
) {
    state.textures.reserve(texture_backups.len());
    for (&texture_id, backup) in texture_backups {
        let texture = create_sampled_texture(
            &state.device,
            &state.queue,
            &state.texture_layout,
            &state.normal_texture_layout,
            &state.sampler,
            backup.width,
            backup.height,
            &backup.pixels,
            "MiniForge restored sprite texture",
        );
        state.textures.insert(texture_id, texture);
    }
}

fn restore_render_targets(
    state: &mut WgpuState,
    descriptors: &HashMap<u64, RenderTargetDescriptor2D>,
) {
    state.render_targets.reserve(descriptors.len());
    for (&texture_id, descriptor) in descriptors {
        let target = create_render_target_texture(
            RenderTargetTextureResources {
                device: &state.device,
                queue: &state.queue,
                color_layout: &state.texture_layout,
                normal_layout: &state.normal_texture_layout,
                sampler: &state.sampler,
            },
            descriptor.width,
            descriptor.height,
            if descriptor.label.trim().is_empty() {
                "MiniForge Render Target 2D"
            } else {
                &descriptor.label
            },
        );
        state.render_targets.insert(texture_id, target);
    }
}

fn normalize_clip_rect(clip: [u32; 4], width: u32, height: u32) -> Option<[u32; 4]> {
    let x = clip[0].min(width.saturating_sub(1));
    let y = clip[1].min(height.saturating_sub(1));
    let clip_width = clip[2].min(width.saturating_sub(x));
    let clip_height = clip[3].min(height.saturating_sub(y));
    (clip_width > 0 && clip_height > 0).then_some([x, y, clip_width, clip_height])
}

fn build_sprite_batches(sprites: &[QueuedSprite]) -> Vec<SpriteBatch> {
    let mut batches: Vec<SpriteBatch> = Vec::new();
    for (index, queued) in sprites.iter().enumerate() {
        let Some(clip_rect) = normalize_clip_rect(
            queued
                .clip_rect
                .unwrap_or([0, 0, queued.viewport_size[0], queued.viewport_size[1]]),
            queued.viewport_size[0],
            queued.viewport_size[1],
        ) else {
            continue;
        };
        if let Some(batch) = batches.last_mut()
            && batch.texture_id == queued.sprite.texture_id
            && batch.normal_texture_id == queued.normal_texture_id
            && batch.render_target_pass == queued.render_target_pass
            && batch.clip_rect == clip_rect
            && batch.blend_mode == queued.blend_mode
            && batch.first_sprite + batch.sprite_count == index as u32
        {
            batch.sprite_count += 1;
            continue;
        }
        batches.push(SpriteBatch {
            texture_id: queued.sprite.texture_id,
            normal_texture_id: queued.normal_texture_id,
            render_target_pass: queued.render_target_pass,
            clip_rect,
            blend_mode: queued.blend_mode,
            first_sprite: index as u32,
            sprite_count: 1,
        });
    }
    batches
}

fn texture_bind_changes(batches: &[SpriteBatch]) -> usize {
    batches
        .iter()
        .map(|batch| batch.texture_id)
        .fold((None, 0usize), |(previous, count), texture_id| {
            (
                Some(texture_id),
                count + usize::from(previous != Some(texture_id)),
            )
        })
        .1
}

fn normal_texture_bind_changes(batches: &[SpriteBatch]) -> usize {
    batches
        .iter()
        .map(|batch| batch.normal_texture_id)
        .fold((None, 0usize), |(previous, count), texture_id| {
            (
                Some(texture_id),
                count + usize::from(previous != Some(texture_id)),
            )
        })
        .1
}

fn pipeline_changes(batches: &[SpriteBatch]) -> usize {
    batches
        .iter()
        .map(|batch| batch.blend_mode)
        .fold((None, 0usize), |(previous, count), blend_mode| {
            (
                Some(blend_mode),
                count + usize::from(previous != Some(blend_mode)),
            )
        })
        .1
}

fn sprite_intersects_viewport(sprite: &SpriteDrawCommand, width: u32, height: u32) -> bool {
    let center_x = sprite.x + sprite.width * 0.5;
    let center_y = sprite.y + sprite.height * 0.5;
    let radius = sprite.width.hypot(sprite.height) * 0.5;
    center_x + radius >= 0.0
        && center_y + radius >= 0.0
        && center_x - radius <= width as f32
        && center_y - radius <= height as f32
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

fn sanitize_post_process_command(
    mut command: PostProcessCommand2D,
) -> MFResult<PostProcessCommand2D> {
    let scalar_values = [
        command.time_seconds,
        command.exposure,
        command.contrast,
        command.saturation,
        command.gamma,
        command.bloom_threshold,
        command.bloom_intensity,
        command.bloom_radius,
        command.vignette_intensity,
        command.vignette_softness,
        command.chromatic_aberration,
        command.pixel_size,
        command.scanline_intensity,
        command.damage_strength,
        command.fog_density,
    ];
    if scalar_values
        .into_iter()
        .chain(command.tint)
        .chain(command.damage_flash)
        .chain(command.fog_color)
        .any(|value| !value.is_finite())
    {
        return Err(render_error(
            "wgpu post-process values and colors must be finite",
        ));
    }
    command.time_seconds = command.time_seconds.max(0.0);
    command.exposure = command.exposure.clamp(0.0, 8.0);
    command.contrast = command.contrast.clamp(0.0, 4.0);
    command.saturation = command.saturation.clamp(0.0, 4.0);
    command.gamma = command.gamma.clamp(0.05, 5.0);
    command.bloom_threshold = command.bloom_threshold.clamp(0.0, 0.999);
    command.bloom_intensity = command.bloom_intensity.clamp(0.0, 4.0);
    command.bloom_radius = command.bloom_radius.clamp(0.5, 32.0);
    command.vignette_intensity = command.vignette_intensity.clamp(0.0, 1.0);
    command.vignette_softness = command.vignette_softness.clamp(0.0, 0.9);
    command.chromatic_aberration = command.chromatic_aberration.clamp(0.0, 0.05);
    command.pixel_size = command.pixel_size.clamp(1.0, 256.0);
    command.scanline_intensity = command.scanline_intensity.clamp(0.0, 1.0);
    command.tint = command.tint.map(|channel| channel.clamp(0.0, 2.0));
    command.damage_flash = command.damage_flash.map(|channel| channel.clamp(0.0, 1.0));
    command.damage_strength = command.damage_strength.clamp(0.0, 1.0);
    command.fog_color = command.fog_color.map(|channel| channel.clamp(0.0, 1.0));
    command.fog_density = command.fog_density.clamp(0.0, 1.0);
    Ok(command)
}

fn post_process_gpu_params(
    command: &PostProcessCommand2D,
    width: u32,
    height: u32,
) -> GpuPostProcessParams {
    GpuPostProcessParams {
        resolution_time: [
            width.max(1) as f32,
            height.max(1) as f32,
            command.time_seconds,
            0.0,
        ],
        color_grade: [
            command.exposure,
            command.contrast,
            command.saturation,
            command.gamma,
        ],
        bloom: [
            command.bloom_threshold,
            command.bloom_intensity,
            command.bloom_radius,
            0.0,
        ],
        vignette: [
            command.vignette_intensity,
            command.vignette_softness,
            0.0,
            0.0,
        ],
        screen_fx: [
            command.chromatic_aberration,
            command.pixel_size,
            command.scanline_intensity,
            0.0,
        ],
        tint: command.tint,
        damage_flash: command.damage_flash,
        damage_fog: [command.damage_strength, command.fog_density, 0.0, 0.0],
        fog_color: command.fog_color,
    }
}

struct PostProcessCreateResources<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    texture_layout: &'a wgpu::BindGroupLayout,
}

fn create_post_process_resources(
    resources: PostProcessCreateResources<'_>,
    width: u32,
    height: u32,
    output_format: wgpu::TextureFormat,
) -> WgpuPostProcess {
    let PostProcessCreateResources {
        device,
        queue,
        texture_layout,
    } = resources;
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("MiniForge post-process linear sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });
    let (scene_texture, scene_view, scene_bind_group) =
        create_post_process_scene_target(device, texture_layout, &sampler, width, height);
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("MiniForge post-process parameters"),
        size: std::mem::size_of::<GpuPostProcessParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("MiniForge post-process uniform layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MiniForge post-process uniform binding"),
        layout: &uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("MiniForge post-process shader"),
        source: wgpu::ShaderSource::Wgsl(POST_PROCESS_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("MiniForge post-process pipeline layout"),
        bind_group_layouts: &[Some(texture_layout), Some(&uniform_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("MiniForge post-process composite pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_post_process"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_post_process"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let text_cache = TextCache::new(device);
    let text_viewport = TextViewport::new(device, &text_cache);
    let mut text_atlas = TextAtlas::new(device, queue, &text_cache, SPRITE_COLOR_VIEW_FORMAT);
    let text_renderer = TextRenderer::new(
        &mut text_atlas,
        device,
        wgpu::MultisampleState::default(),
        None,
    );
    WgpuPostProcess {
        scene_texture,
        scene_view,
        scene_bind_group,
        sampler,
        uniform_buffer,
        uniform_bind_group,
        pipeline,
        text_atlas,
        text_renderer,
        text_viewport,
        width: width.max(1),
        height: height.max(1),
    }
}

fn create_post_process_scene_target(
    device: &wgpu::Device,
    texture_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
    let width = width.max(1);
    let height = height.max(1);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MiniForge post-process scene color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SPRITE_TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[SPRITE_COLOR_VIEW_FORMAT],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("MiniForge post-process scene color sRGB view"),
        format: Some(SPRITE_COLOR_VIEW_FORMAT),
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MiniForge post-process scene color binding"),
        layout: texture_layout,
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
    (texture, view, bind_group)
}

fn resize_post_process_scene(
    device: &wgpu::Device,
    texture_layout: &wgpu::BindGroupLayout,
    post_process: &mut WgpuPostProcess,
    width: u32,
    height: u32,
) {
    let (texture, view, bind_group) = create_post_process_scene_target(
        device,
        texture_layout,
        &post_process.sampler,
        width,
        height,
    );
    post_process.scene_texture = texture;
    post_process.scene_view = view;
    post_process.scene_bind_group = bind_group;
    post_process.width = width.max(1);
    post_process.height = height.max(1);
}

fn readback_texture_rgba8(
    state: &WgpuState,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> MFResult<Vec<u8>> {
    let unpadded_bytes_per_row = width.saturating_mul(4);
    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(alignment) * alignment;
    let buffer_size = u64::from(padded_bytes_per_row) * u64::from(height);
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
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
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
    let mut pixels = vec![0; unpadded_bytes_per_row as usize * height as usize];
    for row in 0..height as usize {
        let source_start = row * padded_bytes_per_row as usize;
        let destination_start = row * unpadded_bytes_per_row as usize;
        pixels[destination_start..destination_start + unpadded_bytes_per_row as usize]
            .copy_from_slice(&mapped[source_start..source_start + unpadded_bytes_per_row as usize]);
    }
    drop(mapped);
    readback.unmap();
    Ok(pixels)
}

#[allow(clippy::too_many_arguments)]
fn create_sampled_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    color_layout: &wgpu::BindGroupLayout,
    normal_layout: &wgpu::BindGroupLayout,
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
        view_formats: &[SPRITE_COLOR_VIEW_FORMAT],
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
    let color_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("MiniForge sRGB color texture view"),
        format: Some(SPRITE_COLOR_VIEW_FORMAT),
        ..Default::default()
    });
    let normal_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("MiniForge linear normal texture view"),
        format: Some(SPRITE_TEXTURE_FORMAT),
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: color_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    let normal_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MiniForge linear normal texture"),
        layout: normal_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&normal_view),
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
        normal_bind_group,
    }
}

struct RenderTargetTextureResources<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    color_layout: &'a wgpu::BindGroupLayout,
    normal_layout: &'a wgpu::BindGroupLayout,
    sampler: &'a wgpu::Sampler,
}

fn create_render_target_texture(
    resources: RenderTargetTextureResources<'_>,
    width: u32,
    height: u32,
    label: &str,
) -> WgpuRenderTarget {
    let RenderTargetTextureResources {
        device,
        queue,
        color_layout,
        normal_layout,
        sampler,
    } = resources;
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[SPRITE_COLOR_VIEW_FORMAT],
    });
    let color_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("MiniForge Render Target 2D sRGB view"),
        format: Some(SPRITE_COLOR_VIEW_FORMAT),
        ..Default::default()
    });
    let normal_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("MiniForge Render Target 2D linear view"),
        format: Some(SPRITE_TEXTURE_FORMAT),
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MiniForge Render Target 2D sampled color"),
        layout: color_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    let normal_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MiniForge Render Target 2D sampled linear"),
        layout: normal_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&normal_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    let text_cache = TextCache::new(device);
    let text_viewport = TextViewport::new(device, &text_cache);
    let mut text_atlas = TextAtlas::new(device, queue, &text_cache, SPRITE_COLOR_VIEW_FORMAT);
    let text_renderer = TextRenderer::new(
        &mut text_atlas,
        device,
        wgpu::MultisampleState::default(),
        None,
    );
    WgpuRenderTarget {
        texture,
        color_view,
        bind_group,
        normal_bind_group,
        text_atlas,
        text_renderer,
        text_viewport,
        width,
        height,
    }
}

fn sprites_to_vertices(commands: &[QueuedSprite]) -> Vec<SpriteVertex> {
    let mut vertices = Vec::with_capacity(commands.len() * 6);
    for queued in commands {
        let command = queued.sprite;
        let width = queued.viewport_size[0];
        let height = queued.viewport_size[1];
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
            material_effect: queued.material_effect as u32,
            effect_strength: f32::from(queued.effect_strength) / 255.0,
            normal_strength: f32::from(queued.normal_strength) / 255.0,
            normal_flip_y: u32::from(queued.normal_flip_y),
            light_direction: queued
                .light_direction
                .map(|axis| f32::from(axis) / f32::from(i16::MAX)),
            light_color: queued.light_color.map(|channel| f32::from(channel) / 255.0),
            ambient_light: f32::from(queued.ambient_light) / 255.0,
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
    fn post_process_uniform_abi_and_validation_are_gpu_safe() {
        assert_eq!(std::mem::size_of::<GpuPostProcessParams>(), 144);
        assert_eq!(std::mem::align_of::<GpuPostProcessParams>(), 16);

        let mut backend = WgpuBackend::default();
        assert!(
            backend
                .set_post_process_2d(PostProcessCommand2D::default())
                .is_err()
        );
        backend.frame_open = true;
        let mut command = PostProcessCommand2D {
            exposure: 99.0,
            gamma: 0.0,
            bloom_intensity: 8.0,
            chromatic_aberration: 2.0,
            tint: [3.0, -1.0, 0.5, 1.0],
            ..PostProcessCommand2D::default()
        };
        backend.set_post_process_2d(command.clone()).unwrap();
        let sanitized = backend.post_process_command.as_ref().unwrap();
        assert_eq!(sanitized.exposure, 8.0);
        assert_eq!(sanitized.gamma, 0.05);
        assert_eq!(sanitized.bloom_intensity, 4.0);
        assert_eq!(sanitized.chromatic_aberration, 0.05);
        assert_eq!(sanitized.tint, [2.0, 0.0, 0.5, 1.0]);

        command.exposure = f32::NAN;
        assert!(backend.set_post_process_2d(command).is_err());
        backend.active_render_target = Some(ActiveRenderTargetPass {
            target_id: 7,
            pass_index: 0,
            first_sprite: 0,
            first_text: 0,
        });
        assert!(
            backend
                .set_post_process_2d(PostProcessCommand2D {
                    bloom_intensity: 1.0,
                    ..PostProcessCommand2D::default()
                })
                .is_err()
        );
    }

    fn queued(texture_id: u64, clip_rect: Option<[u32; 4]>) -> QueuedSprite {
        QueuedSprite {
            sprite: SpriteDrawCommand {
                entity_id: texture_id,
                texture_id,
                x: 10.0,
                y: 10.0,
                width: 16.0,
                height: 16.0,
                rotation: 0.0,
                color: [1.0; 4],
            },
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            clip_rect,
            blend_mode: SpriteBlendMode::Alpha,
            material_effect: SpriteMaterialEffect::None,
            effect_strength: u8::MAX,
            normal_texture_id: BUILTIN_FLAT_NORMAL_TEXTURE_ID,
            normal_strength: 0,
            normal_flip_y: false,
            light_direction: [0, -i16::MAX],
            light_color: [u8::MAX; 3],
            ambient_light: u8::MAX,
            render_target_pass: None,
            viewport_size: [100, 100],
        }
    }

    fn text_command(text: impl Into<String>) -> TextDrawCommand {
        TextDrawCommand {
            text_id: 1,
            text: text.into(),
            font_family: String::new(),
            x: 4.0,
            y: 4.0,
            width: 80.0,
            height: 24.0,
            font_size: 14.0,
            line_height: 18.0,
            color: [255; 4],
            wrap: TextWrapMode::Word,
            clip_rect: None,
        }
    }

    #[test]
    fn gpu_particle_abi_and_submission_are_bounded_before_device_work() {
        assert_eq!(std::mem::size_of::<GpuParticle>(), 32);
        assert_eq!(std::mem::size_of::<GpuParticleParams>(), 96);

        let mut backend = WgpuBackend {
            frame_open: true,
            ..WgpuBackend::default()
        };
        backend
            .draw_particles(ParticleDrawCommand {
                system_id: 42,
                particle_count: usize::MAX,
                lifetime: f32::INFINITY,
                ..ParticleDrawCommand::default()
            })
            .expect_err("non-finite particle values must be rejected");
        backend
            .draw_particles(ParticleDrawCommand {
                system_id: 42,
                particle_count: usize::MAX,
                ..ParticleDrawCommand::default()
            })
            .unwrap();
        assert_eq!(
            backend.particles[0].particle_count,
            MAX_GPU_PARTICLES_PER_SYSTEM
        );
    }

    #[test]
    fn sprite_vertices_cover_requested_pixel_rect() {
        let vertices = sprites_to_vertices(&[QueuedSprite {
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
            blend_mode: SpriteBlendMode::Alpha,
            material_effect: SpriteMaterialEffect::Sepia,
            effect_strength: 128,
            normal_texture_id: BUILTIN_FLAT_NORMAL_TEXTURE_ID,
            normal_strength: 192,
            normal_flip_y: true,
            light_direction: [i16::MAX, 0],
            light_color: [255, 128, 64],
            ambient_light: 32,
            render_target_pass: None,
            viewport_size: [100, 100],
        }]);
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].position, [-1.0, 1.0]);
        assert_eq!(vertices[2].position, [0.0, 0.5]);
        assert_eq!(vertices[0].uv, [0.25, 0.5]);
        assert_eq!(vertices[2].uv, [0.75, 1.0]);
        assert_eq!(
            vertices[0].material_effect,
            SpriteMaterialEffect::Sepia as u32
        );
        assert!((vertices[0].effect_strength - 128.0 / 255.0).abs() < 0.001);
        assert!((vertices[0].normal_strength - 192.0 / 255.0).abs() < 0.001);
        assert_eq!(vertices[0].normal_flip_y, 1);
        assert_eq!(vertices[0].light_direction, [1.0, 0.0]);
        assert_eq!(vertices[0].light_color, [1.0, 128.0 / 255.0, 64.0 / 255.0]);
        assert!((vertices[0].ambient_light - 32.0 / 255.0).abs() < 0.001);
    }

    #[test]
    fn sprite_batches_merge_contiguous_texture_and_clip_runs() {
        let sprites = vec![
            queued(1, None),
            queued(1, None),
            queued(2, None),
            queued(2, Some([0, 0, 50, 50])),
            queued(2, Some([0, 0, 50, 50])),
            queued(1, None),
        ];
        let batches = build_sprite_batches(&sprites);
        assert_eq!(batches.len(), 4);
        assert_eq!(batches[0].sprite_count, 2);
        assert_eq!(batches[1].sprite_count, 1);
        assert_eq!(batches[2].sprite_count, 2);
        assert_eq!(batches[3].sprite_count, 1);
        assert_eq!(texture_bind_changes(&batches), 3);
    }

    #[test]
    fn sprite_batches_preserve_blend_order_and_report_pipeline_changes() {
        let alpha = queued(1, None);
        let mut additive = queued(1, None);
        additive.blend_mode = SpriteBlendMode::Additive;
        let batches = build_sprite_batches(&[alpha, additive, additive, alpha]);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[1].sprite_count, 2);
        assert_eq!(pipeline_changes(&batches), 3);
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.blend_mode)
                .collect::<Vec<_>>(),
            vec![
                SpriteBlendMode::Alpha,
                SpriteBlendMode::Additive,
                SpriteBlendMode::Alpha,
            ]
        );
    }

    #[test]
    fn sprite_batches_split_normal_bindings_without_reordering_color_textures() {
        let first = queued(8, None);
        let mut lit = queued(8, None);
        lit.normal_texture_id = 91;
        let batches = build_sprite_batches(&[first, lit, lit, first]);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[1].sprite_count, 2);
        assert_eq!(texture_bind_changes(&batches), 1);
        assert_eq!(normal_texture_bind_changes(&batches), 3);
    }

    #[test]
    fn sprite_blend_states_cover_alpha_and_effect_workflows() {
        assert_eq!(
            sprite_blend_state(SpriteBlendMode::Alpha),
            wgpu::BlendState::ALPHA_BLENDING
        );
        assert_eq!(
            sprite_blend_state(SpriteBlendMode::PremultipliedAlpha),
            wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING
        );
        let additive = sprite_blend_state(SpriteBlendMode::Additive);
        assert_eq!(additive.color.src_factor, wgpu::BlendFactor::SrcAlpha);
        assert_eq!(additive.color.dst_factor, wgpu::BlendFactor::One);
        let multiply = sprite_blend_state(SpriteBlendMode::Multiply);
        assert_eq!(multiply.color.src_factor, wgpu::BlendFactor::Dst);
        let screen = sprite_blend_state(SpriteBlendMode::Screen);
        assert_eq!(screen.color.dst_factor, wgpu::BlendFactor::OneMinusSrc);
    }

    #[test]
    fn viewport_and_clip_culling_are_conservative_and_safe() {
        let mut sprite = queued(0, None).sprite;
        assert!(sprite_intersects_viewport(&sprite, 100, 100));
        sprite.x = 500.0;
        assert!(!sprite_intersects_viewport(&sprite, 100, 100));
        assert_eq!(
            normalize_clip_rect([90, 90, 20, 20], 100, 100),
            Some([90, 90, 10, 10])
        );
        assert_eq!(normalize_clip_rect([0, 0, 0, 10], 100, 100), None);
    }

    #[test]
    fn text_queue_validates_bounds_metrics_and_memory() {
        let mut backend = WgpuBackend {
            frame_open: true,
            width: 100,
            height: 50,
            ..WgpuBackend::default()
        };
        backend.draw_text(text_command("Unicode ✓")).unwrap();
        assert_eq!(backend.texts.len(), 1);

        let mut outside = text_command("outside");
        outside.x = 500.0;
        backend.draw_text(outside).unwrap();
        assert_eq!(backend.culled_text_areas, 1);

        let mut invalid = text_command("invalid");
        invalid.font_size = f32::NAN;
        assert!(backend.draw_text(invalid).is_err());
        assert!(
            backend
                .draw_text(text_command("x".repeat(MAX_TEXT_BYTES_PER_AREA + 1)))
                .is_err()
        );
    }

    #[test]
    fn text_queue_tracks_render_targets_and_rejects_duplicate_target_writes() {
        let mut backend = WgpuBackend {
            frame_open: true,
            width: 100,
            height: 50,
            ..WgpuBackend::default()
        };
        for texture_id in [7, 8] {
            backend.render_target_descriptors.insert(
                texture_id,
                RenderTargetDescriptor2D {
                    texture_id,
                    width: 32,
                    height: 24,
                    ..RenderTargetDescriptor2D::default()
                },
            );
        }

        backend.begin_render_target_2d(7).unwrap();
        backend.draw_text(text_command("Target A")).unwrap();
        backend.end_render_target_2d().unwrap();
        assert_eq!(backend.texts[0].render_target_pass, Some(0));
        assert_eq!(backend.render_target_passes[0].text_count, 1);
        assert!(backend.begin_render_target_2d(7).is_err());

        backend.begin_render_target_2d(8).unwrap();
        backend.draw_text(text_command("Target B")).unwrap();
        backend.end_render_target_2d().unwrap();
        assert_eq!(backend.texts[1].render_target_pass, Some(1));
        assert_eq!(backend.render_target_passes[1].text_count, 1);
    }

    #[test]
    fn text_bounds_clamp_to_area_clip_and_viewport() {
        let mut command = text_command("bounds");
        command.x = -8.0;
        command.y = 5.0;
        command.width = 30.0;
        command.height = 20.0;
        assert_eq!(
            text_bounds(&command, 100, 50),
            TextBounds {
                left: 0,
                top: 5,
                right: 22,
                bottom: 25,
            }
        );
        command.clip_rect = Some([90, 45, 30, 30]);
        assert_eq!(
            text_bounds(&command, 100, 50),
            TextBounds {
                left: 90,
                top: 45,
                right: 100,
                bottom: 50,
            }
        );
    }

    #[test]
    #[ignore = "requires a physical graphics adapter"]
    fn physical_device_loss_recreates_resources_and_restores_textures() {
        let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
        backend.resize(32, 32).unwrap();
        backend.init().unwrap();
        backend
            .upload_texture_rgba8(7, 2, 2, &[255, 40, 20, 255].repeat(4))
            .unwrap();
        backend
            .create_render_target_2d(RenderTargetDescriptor2D {
                texture_id: 8,
                width: 16,
                height: 16,
                clear_color: [0.0, 0.0, 0.0, 1.0],
                label: "Recovered target".to_string(),
            })
            .unwrap();
        backend.force_device_loss_for_testing().unwrap();
        for _ in 0..8 {
            backend.recover_device_if_needed().unwrap();
            if backend.device_loss_recoveries > 0 {
                break;
            }
        }
        assert_eq!(backend.device_loss_recoveries, 1);
        assert_eq!(backend.texture_count(), 1);
        assert_eq!(backend.render_target_count(), 1);

        backend.begin_frame().unwrap();
        backend.begin_render_target_2d(8).unwrap();
        backend
            .draw_sprite(SpriteDrawCommand {
                entity_id: 8,
                texture_id: 7,
                x: 0.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
                rotation: 0.0,
                color: [1.0; 4],
            })
            .unwrap();
        backend.end_render_target_2d().unwrap();
        backend
            .draw_sprite(SpriteDrawCommand {
                entity_id: 1,
                texture_id: 7,
                x: 0.0,
                y: 0.0,
                width: 32.0,
                height: 32.0,
                rotation: 0.0,
                color: [1.0; 4],
            })
            .unwrap();
        backend
            .draw_text(TextDrawCommand {
                text_id: 2,
                text: "Recovered".to_string(),
                font_family: String::new(),
                x: 1.0,
                y: 1.0,
                width: 30.0,
                height: 14.0,
                font_size: 8.0,
                line_height: 10.0,
                color: [255; 4],
                wrap: TextWrapMode::None,
                clip_rect: None,
            })
            .unwrap();
        backend.end_frame().unwrap();
        let pixels = backend.readback_rgba8().unwrap();
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[0] > 200));
        let target_pixels = backend.readback_render_target_rgba8(8).unwrap();
        assert!(target_pixels.chunks_exact(4).any(|pixel| pixel[0] > 200));
        assert_eq!(backend.last_frame_diagnostics().queued_text_areas, 1);
    }
}
