//! Macroquad player for exported builds.
//!
//! Keeping this small frontend in `runtime` prevents the runtime executable
//! from importing the editor's 2D viewport and UI state.

use std::collections::BTreeMap;
use std::path::PathBuf;

use macroquad::prelude::*;
use serde_json::{Map, Value};

use crate::engine::asset_tools::AssetTools;
use crate::engine::crash_reporter::{CrashReporter, CrashReporterConfig};
use crate::engine::runtime_manifest_loader::RuntimeManifestLoader;
use crate::engine::safe_mode::SafeModeSettings;
use crate::entities::game_object::GameObject;
use crate::runtime::engine_runtime::EngineRuntime;

const LIGHT_FULL_CIRCLE_DEGREES: f32 = 359.0;

#[derive(Default)]
struct RuntimeTextures {
    by_path: BTreeMap<String, Texture2D>,
}

#[derive(Debug, Clone, Copy)]
struct ShadowRect {
    min: Vec2,
    max: Vec2,
}

#[derive(Debug, Clone, Copy)]
struct MinimapMarker {
    color: Color,
    priority: u8,
    distance_sq: f32,
}

#[derive(Debug, Clone, Copy)]
struct ScreenRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Copy)]
struct ScreenOccluder {
    entity_id: u64,
    rect: ScreenRect,
    sorting_order: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityDrawLod {
    Full,
    Simplified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphicsQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl GraphicsQuality {
    fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RuntimeGraphicsSettings {
    quality: GraphicsQuality,
    lighting_enabled: bool,
    shadow_lights_enabled: bool,
    light_sample_budget: usize,
    max_shadow_lights: usize,
    max_source_cores: usize,
    light_caster_padding: f32,
    max_drawn_entities: usize,
    minimap_entity_budget: usize,
    view_frustum_culling: bool,
    occlusion_culling: bool,
    lod_enabled: bool,
    lod_near_pixels: f32,
    lod_far_pixels: f32,
    lod_cull_pixels: f32,
    occlusion_padding: f32,
}

impl RuntimeGraphicsSettings {
    fn for_quality(quality: GraphicsQuality) -> Self {
        match quality {
            GraphicsQuality::Low => Self {
                quality,
                lighting_enabled: true,
                shadow_lights_enabled: false,
                light_sample_budget: 18,
                max_shadow_lights: 0,
                max_source_cores: 24,
                light_caster_padding: 0.0,
                max_drawn_entities: 260,
                minimap_entity_budget: 36,
                view_frustum_culling: true,
                occlusion_culling: true,
                lod_enabled: true,
                lod_near_pixels: 48.0,
                lod_far_pixels: 24.0,
                lod_cull_pixels: 3.0,
                occlusion_padding: 0.0,
            },
            GraphicsQuality::Medium => Self {
                quality,
                lighting_enabled: true,
                shadow_lights_enabled: true,
                light_sample_budget: 28,
                max_shadow_lights: 8,
                max_source_cores: 48,
                light_caster_padding: 0.5,
                max_drawn_entities: 520,
                minimap_entity_budget: 60,
                view_frustum_culling: true,
                occlusion_culling: true,
                lod_enabled: true,
                lod_near_pixels: 48.0,
                lod_far_pixels: 18.0,
                lod_cull_pixels: 3.0,
                occlusion_padding: 0.0,
            },
            GraphicsQuality::High => Self {
                quality,
                lighting_enabled: true,
                shadow_lights_enabled: true,
                light_sample_budget: 44,
                max_shadow_lights: 14,
                max_source_cores: 80,
                light_caster_padding: 1.0,
                max_drawn_entities: 900,
                minimap_entity_budget: 90,
                view_frustum_culling: true,
                occlusion_culling: true,
                lod_enabled: true,
                lod_near_pixels: 48.0,
                lod_far_pixels: 14.0,
                lod_cull_pixels: 2.0,
                occlusion_padding: 0.0,
            },
            GraphicsQuality::Ultra => Self {
                quality,
                lighting_enabled: true,
                shadow_lights_enabled: true,
                light_sample_budget: 72,
                max_shadow_lights: 32,
                max_source_cores: 160,
                light_caster_padding: 1.4,
                max_drawn_entities: 1600,
                minimap_entity_budget: 160,
                view_frustum_culling: true,
                occlusion_culling: true,
                lod_enabled: true,
                lod_near_pixels: 48.0,
                lod_far_pixels: 10.0,
                lod_cull_pixels: 1.0,
                occlusion_padding: 0.0,
            },
        }
    }

    fn from_runtime(runtime: &EngineRuntime, override_quality: Option<GraphicsQuality>) -> Self {
        let quality = override_quality.unwrap_or_else(|| runtime_graphics_quality(runtime));
        let mut settings = Self::for_quality(quality);
        if let Some(graphics) = runtime
            .runtime_config
            .data
            .get("graphics")
            .and_then(Value::as_object)
        {
            settings.apply_runtime_config(graphics, override_quality.is_none());
        }
        settings
    }

    fn apply_runtime_config(&mut self, graphics: &Map<String, Value>, include_top_level: bool) {
        if let Some(profile) = graphics
            .get("profiles")
            .and_then(Value::as_object)
            .and_then(|profiles| profiles.get(self.quality.label()))
            .and_then(Value::as_object)
        {
            self.apply_config_values(profile);
        }
        if include_top_level {
            self.apply_config_values(graphics);
        }
    }

    fn apply_config_values(&mut self, values: &Map<String, Value>) {
        if let Some(value) = values.get("lighting_enabled").and_then(Value::as_bool) {
            self.lighting_enabled = value;
        }
        if let Some(value) = values.get("shadow_lights_enabled").and_then(Value::as_bool) {
            self.shadow_lights_enabled = value;
        }
        if let Some(value) = values.get("light_sample_budget").and_then(Value::as_u64) {
            self.light_sample_budget = (value as usize).clamp(8, 128);
        }
        if let Some(value) = values.get("max_shadow_lights").and_then(Value::as_u64) {
            self.max_shadow_lights = (value as usize).min(256);
        }
        if let Some(value) = values.get("max_source_cores").and_then(Value::as_u64) {
            self.max_source_cores = (value as usize).min(10_000);
        }
        if let Some(value) = values.get("light_caster_padding").and_then(Value::as_f64)
            && value.is_finite()
        {
            self.light_caster_padding = (value as f32).clamp(0.0, 32.0);
        }
        if let Some(value) = values.get("max_drawn_entities").and_then(Value::as_u64) {
            self.max_drawn_entities = (value as usize).clamp(64, 50_000);
        }
        if let Some(value) = values.get("minimap_entity_budget").and_then(Value::as_u64) {
            self.minimap_entity_budget = (value as usize).min(10_000);
        }
        if let Some(value) = values.get("view_frustum_culling").and_then(Value::as_bool) {
            self.view_frustum_culling = value;
        }
        if let Some(value) = values.get("occlusion_culling").and_then(Value::as_bool) {
            self.occlusion_culling = value;
        }
        if let Some(value) = values.get("lod_enabled").and_then(Value::as_bool) {
            self.lod_enabled = value;
        }
        if let Some(value) = values.get("lod_near_pixels").and_then(Value::as_f64)
            && value.is_finite()
        {
            self.lod_near_pixels = (value as f32).clamp(1.0, 512.0);
        }
        if let Some(value) = values.get("lod_far_pixels").and_then(Value::as_f64)
            && value.is_finite()
        {
            self.lod_far_pixels = (value as f32).clamp(1.0, 512.0);
        }
        if let Some(value) = values.get("lod_cull_pixels").and_then(Value::as_f64)
            && value.is_finite()
        {
            self.lod_cull_pixels = (value as f32).clamp(0.0, 64.0);
        }
        if let Some(value) = values.get("occlusion_padding").and_then(Value::as_f64)
            && value.is_finite()
        {
            self.occlusion_padding = (value as f32).clamp(0.0, 64.0);
        }
    }
}

fn runtime_graphics_quality(runtime: &EngineRuntime) -> GraphicsQuality {
    std::env::var("MINIFORGE_GRAPHICS_QUALITY")
        .ok()
        .and_then(|value| parse_graphics_quality(&value))
        .or_else(|| {
            runtime
                .runtime_config
                .data
                .get("graphics")
                .and_then(|graphics| graphics.get("quality"))
                .and_then(serde_json::Value::as_str)
                .and_then(parse_graphics_quality)
        })
        .or_else(|| {
            runtime
                .runtime_config
                .data
                .get("quality_preset")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_graphics_quality)
        })
        .or_else(|| {
            runtime
                .engine_config
                .data
                .get("runtime")
                .and_then(|runtime| runtime.get("graphics_quality"))
                .and_then(serde_json::Value::as_str)
                .and_then(parse_graphics_quality)
        })
        .or_else(|| {
            runtime
                .engine_config
                .data
                .get("runtime")
                .and_then(|runtime| runtime.get("quality_preset"))
                .and_then(serde_json::Value::as_str)
                .and_then(parse_graphics_quality)
        })
        .unwrap_or(GraphicsQuality::High)
}

fn parse_graphics_quality(value: &str) -> Option<GraphicsQuality> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" | "fast" | "performance" | "potato" => Some(GraphicsQuality::Low),
        "medium" | "balanced" | "normal" => Some(GraphicsQuality::Medium),
        "high" | "quality" => Some(GraphicsQuality::High),
        "ultra" | "cinematic" | "max" => Some(GraphicsQuality::Ultra),
        _ => None,
    }
}

fn next_graphics_quality(current: GraphicsQuality) -> GraphicsQuality {
    match current {
        GraphicsQuality::Low => GraphicsQuality::Medium,
        GraphicsQuality::Medium => GraphicsQuality::High,
        GraphicsQuality::High => GraphicsQuality::Ultra,
        GraphicsQuality::Ultra => GraphicsQuality::Low,
    }
}

impl RuntimeTextures {
    async fn load(runtime: &EngineRuntime) -> Self {
        let mut textures = Self::default();
        for relative in runtime.asset_database.assets.keys().filter(|path| {
            matches!(
                PathBuf::from(path)
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("png" | "jpg" | "jpeg" | "bmp")
            )
        }) {
            let absolute = runtime.project_path.join(relative);
            let Some(path) = absolute.to_str() else {
                continue;
            };
            if let Ok(texture) = load_texture(path).await {
                texture.set_filter(FilterMode::Nearest);
                textures.by_path.insert(relative.clone(), texture);
            }
        }
        textures
    }

    fn get(&self, path: &str) -> Option<&Texture2D> {
        self.by_path.get(path)
    }
}

pub fn runtime_player_window_conf() -> Conf {
    Conf {
        window_title: "MiniForge Runtime Player".to_string(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 2,
        window_resizable: true,
        ..Default::default()
    }
}

pub fn parse_exported_build_path_from_env() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut resolved = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!(
                    "miniforge_runtime --build <carpeta>\nLa carpeta debe contener runtime_manifest.json"
                );
                std::process::exit(0);
            }
            "--build" => resolved = args.next().map(PathBuf::from),
            value if !value.starts_with('-') && resolved.is_none() => {
                resolved = Some(PathBuf::from(value));
            }
            _ => {}
        }
    }
    resolved.ok_or_else(|| "Indica la carpeta exportada con --build <ruta>".to_string())
}

pub async fn run_exported_runtime_player() {
    let build_root = match parse_exported_build_path_from_env() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    if !build_root.is_dir() {
        eprintln!("La ruta no es una carpeta: {}", build_root.display());
        return;
    }
    if let Err(error) = RuntimeManifestLoader::load(&build_root) {
        eprintln!("Advertencia al validar runtime_manifest.json: {error}");
    }
    if let Err(error) = AssetTools::ensure_project_folders(&build_root) {
        eprintln!("Estructura de proyecto incompleta: {error}");
        return;
    }
    CrashReporter::install(CrashReporterConfig::for_project(
        &build_root,
        "MiniForge Runtime",
    ));
    let safe_mode = if safe_mode_requested_from_environment() {
        SafeModeSettings::for_recovery("solicitado para runtime exportado")
    } else {
        SafeModeSettings::default()
    };
    let mut runtime = match EngineRuntime::from_project_with_safe_mode(&build_root, safe_mode) {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("No se pudo cargar el proyecto exportado: {error}");
            return;
        }
    };
    runtime.run_headless_once(0.0);
    let textures = RuntimeTextures::load(&runtime).await;
    let mut zoom_target = runtime.camera.zoom;
    let mut graphics_override = None;
    let mut graphics_notice_timer = 0.0f32;

    loop {
        if is_key_pressed(KeyCode::F10) {
            break;
        }
        let dt = get_frame_time() as f64;
        handle_character_input(&mut runtime);
        if is_key_pressed(KeyCode::F1) {
            let current = graphics_override.unwrap_or_else(|| runtime_graphics_quality(&runtime));
            graphics_override = Some(next_graphics_quality(current));
            graphics_notice_timer = 2.0;
        }
        graphics_notice_timer = (graphics_notice_timer - dt as f32).max(0.0);
        handle_camera_zoom(&mut runtime, &mut zoom_target, dt);
        runtime.run_headless_once(dt);
        draw_runtime(
            &runtime,
            &textures,
            graphics_override,
            graphics_notice_timer,
        );
        next_frame().await;
    }
}

fn handle_character_input(runtime: &mut EngineRuntime) {
    let left = is_key_down(KeyCode::A) || is_key_down(KeyCode::Left);
    let right = is_key_down(KeyCode::D) || is_key_down(KeyCode::Right);
    let up = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
    let down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
    let movement = (
        right as i32 as f64 - left as i32 as f64,
        down as i32 as f64 - up as i32 as f64,
    );
    let jump_pressed = is_key_pressed(KeyCode::Space);
    let jump_held = is_key_down(KeyCode::Space);
    let run_pressed = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let fire_pressed = is_mouse_button_down(MouseButton::Left)
        || is_key_down(KeyCode::LeftControl)
        || is_key_down(KeyCode::RightControl);
    let interact_pressed = is_key_down(KeyCode::E) || is_key_down(KeyCode::Enter);
    let pause_pressed = is_key_down(KeyCode::Escape);
    let vehicle_pressed = is_key_down(KeyCode::F);
    let dash_pressed = is_key_pressed(KeyCode::X);
    runtime.set_character_input_for_tag(
        "Player",
        movement,
        jump_pressed,
        jump_held,
        run_pressed,
        dash_pressed,
    );
    for (key, pressed) in [
        ("A", left),
        ("D", right),
        ("W", up),
        ("S", down),
        ("Space", jump_held),
        ("Shift", run_pressed),
        ("run", run_pressed),
        ("MouseLeft", fire_pressed),
        ("Ctrl", fire_pressed),
        ("fire", fire_pressed),
        ("E", interact_pressed),
        ("interact", interact_pressed),
        ("Escape", pause_pressed),
        ("pause", pause_pressed),
        ("F", vehicle_pressed),
        ("enter_vehicle", vehicle_pressed),
        ("1", is_key_down(KeyCode::Key1)),
        ("2", is_key_down(KeyCode::Key2)),
    ] {
        runtime.set_script_input_pressed(key, pressed);
    }
    for (key_code, key) in [
        (KeyCode::Space, "Space"),
        (KeyCode::X, "X"),
        (KeyCode::Enter, "Enter"),
        (KeyCode::E, "E"),
        (KeyCode::Key1, "1"),
        (KeyCode::Key2, "2"),
    ] {
        if is_key_pressed(key_code) {
            runtime.dispatch_script_key_down(key);
        }
    }
    if is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Enter) {
        let _ = runtime.interact();
    }
    if is_key_pressed(KeyCode::Key1) {
        let _ = runtime.choose_dialogue(0);
    }
    if is_key_pressed(KeyCode::Key2) {
        let _ = runtime.choose_dialogue(1);
    }
}

fn handle_camera_zoom(runtime: &mut EngineRuntime, target: &mut f64, dt: f64) {
    if is_key_down(KeyCode::Q) {
        *target -= dt * 1.4;
    }
    if is_key_down(KeyCode::R) {
        *target += dt * 1.4;
    }
    let (_, wheel_y) = mouse_wheel();
    if wheel_y != 0.0 {
        *target += wheel_y as f64 * 0.08;
    }
    *target = target.clamp(0.1, 8.0);
    let blend = 1.0 - (-14.0 * dt).exp();
    runtime
        .camera
        .set_zoom(runtime.camera.zoom + (*target - runtime.camera.zoom) * blend);
}

fn draw_runtime(
    runtime: &EngineRuntime,
    textures: &RuntimeTextures,
    graphics_override: Option<GraphicsQuality>,
    graphics_notice_timer: f32,
) {
    let graphics = RuntimeGraphicsSettings::from_runtime(runtime, graphics_override);
    clear_background(Color::from_rgba(13, 18, 27, 255));
    let tile = runtime.grid.tile_size as f32 * runtime.camera.zoom as f32;
    let origin_x = -runtime.camera.x as f32 * runtime.camera.zoom as f32;
    let origin_y = -runtime.camera.y as f32 * runtime.camera.zoom as f32;
    let mut world_entities = runtime
        .runtime_world
        .units
        .iter()
        .filter(|entity| {
            entity.enabled && entity.visible && entity.get_component("UIElement").is_none()
        })
        .filter(|entity| {
            entity
                .get_component("SpriteRenderer")
                .is_none_or(|sprite| sprite.get_bool("visible", true))
        })
        .filter(|entity| {
            !graphics.view_frustum_culling
                || entity_intersects_screen(entity, origin_x, origin_y, tile, tile * 8.0)
        })
        .collect::<Vec<_>>();
    world_entities.sort_by_key(|entity| {
        entity
            .get_component("SpriteRenderer")
            .map(|sprite| sprite.get_i64("sorting_order", 0))
            .unwrap_or(0)
    });
    let occluders = if graphics.occlusion_culling {
        screen_occluders(&world_entities, origin_x, origin_y, tile)
    } else {
        Vec::new()
    };
    for (entity, lod) in world_entities
        .iter()
        .copied()
        .filter(|entity| {
            entity
                .get_component("SpriteRenderer")
                .is_some_and(|sprite| sprite.get_i64("sorting_order", 0) < 0)
        })
        .filter_map(|entity| {
            entity_draw_lod(entity, &graphics, &occluders, origin_x, origin_y, tile)
                .map(|lod| (entity, lod))
        })
        .take(graphics.max_drawn_entities)
    {
        draw_entity(entity, origin_x, origin_y, tile, textures, lod);
    }
    draw_tiles(runtime, origin_x, origin_y, tile);
    draw_grid(runtime, origin_x, origin_y, tile);
    for (entity, lod) in world_entities
        .into_iter()
        .filter(|entity| {
            entity
                .get_component("SpriteRenderer")
                .is_none_or(|sprite| sprite.get_i64("sorting_order", 0) >= 0)
        })
        .filter_map(|entity| {
            entity_draw_lod(entity, &graphics, &occluders, origin_x, origin_y, tile)
                .map(|lod| (entity, lod))
        })
        .take(graphics.max_drawn_entities)
    {
        draw_entity(entity, origin_x, origin_y, tile, textures, lod);
    }
    draw_lighting(runtime, origin_x, origin_y, tile, &graphics);
    draw_ui(runtime, &graphics);
    if graphics_notice_timer > 0.0 {
        draw_text_ex(
            &format!("Graficos: {}  (F1)", graphics.quality.label()),
            24.0,
            34.0,
            TextParams {
                font_size: 22,
                color: Color::from_rgba(160, 235, 255, 230),
                ..Default::default()
            },
        );
    }
}

fn draw_tiles(runtime: &EngineRuntime, origin_x: f32, origin_y: f32, tile: f32) {
    let palette = [
        Color::from_rgba(32, 43, 61, 255),
        Color::from_rgba(46, 62, 80, 255),
        Color::from_rgba(74, 91, 112, 255),
        Color::from_rgba(48, 72, 67, 255),
        Color::from_rgba(96, 71, 91, 255),
        Color::from_rgba(107, 91, 64, 255),
    ];
    let has_backdrop = runtime.runtime_world.units.iter().any(|entity| {
        entity.enabled
            && entity.visible
            && entity
                .get_component("SpriteRenderer")
                .is_some_and(|sprite| sprite.get_i64("sorting_order", 0) < -50)
    });
    if has_backdrop {
        return;
    }
    let (min_x, max_x, min_y, max_y) = visible_tile_bounds(
        origin_x,
        origin_y,
        tile,
        runtime.tilemap_layers.width,
        runtime.tilemap_layers.height,
        1,
    );
    for (layer_index, layer) in runtime.tilemap_layers.layers.iter().enumerate() {
        if !layer.visible || (has_backdrop && layer_index == 0) {
            continue;
        }
        for y in min_y..max_y {
            for x in min_x..max_x {
                let value = layer.get(x, y);
                if value == 0 {
                    continue;
                }
                let mut color = palette[value.unsigned_abs() as usize % palette.len()];
                color.a = if layer_index == 0 { 1.0 } else { 0.72 };
                draw_rectangle(
                    origin_x + x as f32 * tile,
                    origin_y + y as f32 * tile,
                    tile.ceil(),
                    tile.ceil(),
                    color,
                );
            }
        }
    }
}

fn draw_grid(runtime: &EngineRuntime, origin_x: f32, origin_y: f32, tile: f32) {
    if std::env::var("MINIFORGE_RUNTIME_DEBUG_GRID").as_deref() != Ok("1") {
        return;
    }
    if tile < 5.0 {
        return;
    }
    let color = Color::from_rgba(65, 78, 98, 70);
    let (min_x, max_x, min_y, max_y) = visible_tile_bounds(
        origin_x,
        origin_y,
        tile,
        runtime.grid.width,
        runtime.grid.height,
        1,
    );
    for x in min_x..=max_x {
        let px = origin_x + x as f32 * tile;
        draw_line(
            px,
            origin_y + min_y as f32 * tile,
            px,
            origin_y + max_y as f32 * tile,
            1.0,
            color,
        );
    }
    for y in min_y..=max_y {
        let py = origin_y + y as f32 * tile;
        draw_line(
            origin_x + min_x as f32 * tile,
            py,
            origin_x + max_x as f32 * tile,
            py,
            1.0,
            color,
        );
    }
}

fn draw_entity(
    entity: &GameObject,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    textures: &RuntimeTextures,
    lod: EntityDrawLod,
) {
    if entity
        .get_component("SpriteRenderer")
        .is_some_and(|sprite| !sprite.get_bool("visible", true))
    {
        return;
    }
    let x = origin_x + entity.x as f32 * tile;
    let y = origin_y + entity.y as f32 * tile;
    let width = (entity.width as f32 * tile).max(8.0);
    let height = (entity.height as f32 * tile).max(8.0);
    let fallback_color = if entity.tag == "Player" {
        Color::from_rgba(72, 195, 255, 255)
    } else if entity.tag == "Enemy" {
        Color::from_rgba(255, 101, 118, 255)
    } else {
        Color::from_rgba(132, 232, 174, 255)
    };
    let sprite = entity.get_component("SpriteRenderer");
    let color = sprite
        .and_then(|sprite| sprite.get("tint"))
        .map(|tint| color_from_component(Some(tint), [255, 255, 255], 1.0))
        .unwrap_or(fallback_color);
    let texture_path = sprite
        .and_then(|component| component.get("_texture_path"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            sprite
                .and_then(|component| component.get("texture_path"))
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| {
            sprite
                .and_then(|component| component.get("source_asset"))
                .and_then(serde_json::Value::as_str)
        });
    if lod == EntityDrawLod::Full
        && let Some(texture) = texture_path.and_then(|path| textures.get(path))
    {
        let source = sprite
            .and_then(|component| component.get("_source_rect"))
            .and_then(serde_json::Value::as_object)
            .map(|rect| {
                Rect::new(
                    rect.get("x")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0) as f32,
                    rect.get("y")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0) as f32,
                    rect.get("width")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(texture.width() as f64) as f32,
                    rect.get("height")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(texture.height() as f64) as f32,
                )
            });
        draw_texture_ex(
            texture,
            x - width * 0.5,
            y - height * 0.5,
            color,
            DrawTextureParams {
                dest_size: Some(vec2(width, height)),
                source,
                rotation: entity.rotation.to_radians() as f32,
                flip_x: sprite.is_some_and(|sprite| sprite.get_bool("flip_x", false)),
                flip_y: sprite.is_some_and(|sprite| sprite.get_bool("flip_y", false)),
                pivot: Some(vec2(x, y)),
            },
        );
    } else {
        draw_rectangle(x - width * 0.5, y - height * 0.5, width, height, color);
    }
    let show_label = lod == EntityDrawLod::Full
        && sprite
            .map(|sprite| {
                sprite.get_bool(
                    "show_label",
                    entity.tag != "Effects" && entity.tag != "Trigger",
                )
            })
            .unwrap_or(entity.tag != "Effects" && entity.tag != "Trigger");
    if show_label {
        draw_text(
            &entity.name,
            x - width * 0.5,
            y - height * 0.65,
            13.0,
            WHITE,
        );
    }
}

fn draw_ui(runtime: &EngineRuntime, graphics: &RuntimeGraphicsSettings) {
    let mut elements = runtime
        .runtime_world
        .units
        .iter()
        .filter_map(|entity| entity.get_component("UIElement").map(|ui| (entity, ui)))
        .filter(|(entity, ui)| entity.visible && ui.get_bool("visible", true))
        .collect::<Vec<_>>();
    elements.sort_by_key(|(_, ui)| ui.get_i64("sorting_order", 0));
    for (entity, ui) in elements {
        let x = ui.get_f64("x", 0.0) as f32;
        let y = ui.get_f64("y", 0.0) as f32;
        let width = ui.get_f64("width", 160.0) as f32;
        let height = ui.get_f64("height", 36.0) as f32;
        let kind = ui.get_string("element_type", "Label");
        let opacity = ui.get_f64("opacity", 1.0).clamp(0.0, 1.0) as f32;
        if opacity <= 0.01 {
            continue;
        }
        if kind == "Minimap" {
            draw_minimap(runtime, x, y, width, height, opacity, graphics);
            continue;
        }
        if kind == "WantedStars" {
            draw_wanted_stars(runtime, x, y, width, height, opacity);
            continue;
        }
        let background = color_from_component(ui.get("color"), [24, 28, 36], opacity);
        let foreground = color_from_component(ui.get("text_color"), [246, 240, 231], 1.0);
        draw_ui_panel(x, y, width, height, background, opacity);
        draw_rectangle_lines(
            x,
            y,
            width,
            height,
            1.0,
            color_from_component(ui.get("border_color"), [92, 112, 142], opacity),
        );
        if kind == "ProgressBar" || kind == "StatBar" {
            let max = ui.get_f64("max_progress", 1.0).max(0.0001);
            let progress = (ui.get_f64("progress", 0.0) / max).clamp(0.0, 1.0) as f32;
            let fill = if entity.name.contains("Armor") {
                Color::from_rgba(125, 190, 255, 230)
            } else if entity.name.contains("Health") {
                Color::from_rgba(255, 90, 142, 230)
            } else {
                Color::from_rgba(230, 92, 151, 230)
            };
            draw_rectangle(
                x + 3.0,
                y + 3.0,
                (width - 6.0) * progress,
                height - 6.0,
                fill,
            );
        }
        let text = ui.get_string("text", "");
        if text.is_empty() {
            continue;
        }
        let font_size = (height * 0.42).clamp(13.0, 22.0) as u16;
        let baseline = y + (height + font_size as f32) * 0.5 - 2.0;
        draw_text_ex(
            &text,
            x + ui.get_f64("padding", 9.0) as f32,
            baseline,
            TextParams {
                font_size,
                color: foreground,
                ..Default::default()
            },
        );
    }
}

fn draw_ui_panel(x: f32, y: f32, width: f32, height: f32, background: Color, opacity: f32) {
    draw_rectangle(
        x + 3.0,
        y + 4.0,
        width,
        height,
        Color::from_rgba(0, 0, 0, (80.0 * opacity) as u8),
    );
    draw_rectangle(x, y, width, height, background);
    draw_rectangle(
        x,
        y,
        width,
        2.0,
        Color::from_rgba(135, 235, 255, (120.0 * opacity) as u8),
    );
    draw_rectangle(
        x,
        y + height - 2.0,
        width,
        2.0,
        Color::from_rgba(255, 112, 184, (75.0 * opacity) as u8),
    );
}

fn draw_minimap(
    runtime: &EngineRuntime,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
    graphics: &RuntimeGraphicsSettings,
) {
    let radius = width.min(height) * 0.5;
    let center = vec2(x + width * 0.5, y + height * 0.5);
    draw_circle(
        center.x + 3.0,
        center.y + 4.0,
        radius,
        Color::from_rgba(0, 0, 0, (90.0 * opacity) as u8),
    );
    draw_circle(
        center.x,
        center.y,
        radius,
        Color::from_rgba(9, 18, 28, (218.0 * opacity) as u8),
    );
    draw_circle_lines(
        center.x,
        center.y,
        radius,
        3.0,
        Color::from_rgba(255, 112, 214, (220.0 * opacity) as u8),
    );
    draw_circle_lines(
        center.x,
        center.y,
        radius - 4.0,
        1.0,
        Color::from_rgba(115, 236, 255, (160.0 * opacity) as u8),
    );

    let player = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.enabled && entity.tag == "Player")
        .map(|entity| vec2(entity.x as f32, entity.y as f32))
        .unwrap_or_else(|| vec2(0.0, 0.0));
    let world_radius = 17.5;
    let scale = (radius - 11.0) / world_radius;
    let min_tile_x = (player.x - world_radius).floor().max(0.0) as usize;
    let max_tile_x = (player.x + world_radius)
        .ceil()
        .min(runtime.tilemap_layers.width as f32) as usize;
    let min_tile_y = (player.y - world_radius).floor().max(0.0) as usize;
    let max_tile_y = (player.y + world_radius)
        .ceil()
        .min(runtime.tilemap_layers.height as f32) as usize;
    let road_layer = runtime
        .tilemap_layers
        .layer("Decoration")
        .or_else(|| runtime.tilemap_layers.layers.get(1));
    if let Some(layer) = road_layer {
        for y_tile in min_tile_y..max_tile_y {
            for x_tile in min_tile_x..max_tile_x {
                if layer.get(x_tile, y_tile) == 0 {
                    continue;
                }
                let dx = x_tile as f32 + 0.5 - player.x;
                let dy = y_tile as f32 + 0.5 - player.y;
                let point = center + vec2(dx * scale, dy * scale);
                if point.distance(center) > radius - 9.0 {
                    continue;
                }
                draw_rectangle(
                    point.x - scale * 0.55,
                    point.y - scale * 0.55,
                    scale * 1.1,
                    scale * 1.1,
                    Color::from_rgba(190, 238, 235, (185.0 * opacity) as u8),
                );
            }
        }
    }

    let mut markers = runtime
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.enabled && entity.visible)
        .filter_map(|entity| {
            minimap_marker(entity, player, world_radius, opacity).map(|marker| (entity, marker))
        })
        .collect::<Vec<_>>();
    markers.sort_by(|(_, a), (_, b)| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.distance_sq.total_cmp(&b.distance_sq))
    });

    for (entity, marker) in markers.into_iter().take(graphics.minimap_entity_budget) {
        let dx = entity.x as f32 - player.x;
        let dy = entity.y as f32 - player.y;
        let point = center + vec2(dx * scale, dy * scale);
        if point.distance(center) > radius - 10.0 {
            continue;
        }
        draw_circle(point.x, point.y, 3.1, marker.color);
    }
    draw_triangle(
        center + vec2(0.0, -8.5),
        center + vec2(6.5, 7.0),
        center + vec2(-6.5, 7.0),
        Color::from_rgba(255, 76, 190, (245.0 * opacity) as u8),
    );
    draw_circle_lines(
        center.x,
        center.y,
        9.0,
        1.0,
        Color::from_rgba(255, 255, 255, (150.0 * opacity) as u8),
    );
}

fn minimap_marker(
    entity: &GameObject,
    player: Vec2,
    world_radius: f32,
    opacity: f32,
) -> Option<MinimapMarker> {
    let (color, priority) = match entity.tag.as_str() {
        "Police" => (Color::from_rgba(82, 154, 255, (230.0 * opacity) as u8), 0),
        "Contact" => (Color::from_rgba(196, 126, 255, (230.0 * opacity) as u8), 1),
        "Collectible" => (Color::from_rgba(90, 255, 218, (230.0 * opacity) as u8), 2),
        "Vehicle" => (Color::from_rgba(255, 205, 110, (210.0 * opacity) as u8), 3),
        _ => return None,
    };
    let dx = entity.x as f32 - player.x;
    let dy = entity.y as f32 - player.y;
    let distance_sq = dx * dx + dy * dy;
    (distance_sq <= world_radius * world_radius).then_some(MinimapMarker {
        color,
        priority,
        distance_sq,
    })
}

fn draw_wanted_stars(
    runtime: &EngineRuntime,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
) {
    let wanted = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == "CityDirector")
        .and_then(|entity| entity.get_component("Blackboard"))
        .and_then(|blackboard| blackboard.get("values"))
        .and_then(|values| values.get("wanted"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let active = wanted.round().clamp(0.0, 6.0) as usize;
    let count = 6;
    let step = width / count as f32;
    let radius = height.min(step) * 0.35;
    draw_ui_panel(
        x,
        y,
        width,
        height,
        Color::from_rgba(16, 18, 30, (190.0 * opacity) as u8),
        opacity,
    );
    for index in 0..count {
        let center = vec2(x + step * (index as f32 + 0.5), y + height * 0.52);
        let filled = index < active;
        let color = if filled {
            Color::from_rgba(65, 176, 255, (245.0 * opacity) as u8)
        } else {
            Color::from_rgba(47, 69, 98, (175.0 * opacity) as u8)
        };
        draw_star(center, radius, color);
    }
}

fn draw_star(center: Vec2, radius: f32, color: Color) {
    let mut points = Vec::with_capacity(10);
    for index in 0..10 {
        let r = if index % 2 == 0 {
            radius
        } else {
            radius * 0.45
        };
        let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::PI / 5.0;
        points.push(center + vec2(angle.cos() * r, angle.sin() * r));
    }
    for index in 1..(points.len() - 1) {
        draw_triangle(points[0], points[index], points[index + 1], color);
    }
}

fn entity_draw_lod(
    entity: &GameObject,
    graphics: &RuntimeGraphicsSettings,
    occluders: &[ScreenOccluder],
    origin_x: f32,
    origin_y: f32,
    tile: f32,
) -> Option<EntityDrawLod> {
    if graphics.occlusion_culling
        && entity_is_occluded(
            entity,
            occluders,
            origin_x,
            origin_y,
            tile,
            graphics.occlusion_padding,
        )
    {
        return None;
    }
    if !graphics.lod_enabled || entity.tag == "Player" {
        return Some(EntityDrawLod::Full);
    }
    let rect = entity_screen_rect(entity, origin_x, origin_y, tile);
    let max_dimension = rect.width.max(rect.height);
    if max_dimension <= graphics.lod_cull_pixels {
        return None;
    }
    if max_dimension <= graphics.lod_far_pixels {
        return Some(EntityDrawLod::Simplified);
    }
    Some(EntityDrawLod::Full)
}

fn screen_occluders(
    entities: &[&GameObject],
    origin_x: f32,
    origin_y: f32,
    tile: f32,
) -> Vec<ScreenOccluder> {
    entities
        .iter()
        .filter_map(|entity| screen_occluder(entity, origin_x, origin_y, tile))
        .collect()
}

fn screen_occluder(
    entity: &GameObject,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
) -> Option<ScreenOccluder> {
    let has_dedicated_occluder = entity.get_component("Occluder2D").is_some_and(|component| {
        component.enabled && component.get_bool("occludes_rendering", true)
    });
    let has_shadow_occluder = entity
        .get_component("ShadowCaster2D")
        .is_some_and(|component| {
            component.enabled && component.get_bool("occludes_rendering", true)
        });
    if !has_dedicated_occluder && !has_shadow_occluder {
        return None;
    }
    let rect = entity_screen_rect(entity, origin_x, origin_y, tile);
    let min_area = (tile * tile * 0.75).max(16.0);
    if rect.width * rect.height < min_area {
        return None;
    }
    Some(ScreenOccluder {
        entity_id: entity.id,
        rect,
        sorting_order: entity_sorting_order(entity),
    })
}

fn entity_is_occluded(
    entity: &GameObject,
    occluders: &[ScreenOccluder],
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    padding: f32,
) -> bool {
    let rect = entity_screen_rect(entity, origin_x, origin_y, tile);
    let sorting_order = entity_sorting_order(entity);
    occluders.iter().any(|occluder| {
        occluder.entity_id != entity.id
            && occluder.sorting_order >= sorting_order
            && screen_rect_contains(occluder.rect, rect, padding)
    })
}

fn entity_screen_rect(entity: &GameObject, origin_x: f32, origin_y: f32, tile: f32) -> ScreenRect {
    let center_x = origin_x + entity.x as f32 * tile;
    let center_y = origin_y + entity.y as f32 * tile;
    let width = (entity.width as f32 * entity.scale_x.abs() as f32 * tile).max(1.0);
    let height = (entity.height as f32 * entity.scale_y.abs() as f32 * tile).max(1.0);
    ScreenRect {
        x: center_x - width * 0.5,
        y: center_y - height * 0.5,
        width,
        height,
    }
}

fn screen_rect_contains(outer: ScreenRect, inner: ScreenRect, padding: f32) -> bool {
    inner.x >= outer.x - padding
        && inner.y >= outer.y - padding
        && inner.x + inner.width <= outer.x + outer.width + padding
        && inner.y + inner.height <= outer.y + outer.height + padding
}

fn entity_sorting_order(entity: &GameObject) -> i64 {
    entity
        .get_component("SpriteRenderer")
        .map(|sprite| sprite.get_i64("sorting_order", 0))
        .unwrap_or(0)
}

fn entity_intersects_screen(
    entity: &GameObject,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    padding: f32,
) -> bool {
    entity_intersects_screen_for_viewport(
        entity.x as f32,
        entity.y as f32,
        entity.width as f32,
        entity.height as f32,
        origin_x,
        origin_y,
        tile,
        screen_width(),
        screen_height(),
        padding,
    )
}

#[allow(clippy::too_many_arguments)]
fn entity_intersects_screen_for_viewport(
    entity_x: f32,
    entity_y: f32,
    entity_width: f32,
    entity_height: f32,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    screen_width: f32,
    screen_height: f32,
    padding: f32,
) -> bool {
    if tile <= f32::EPSILON {
        return false;
    }
    let x = origin_x + entity_x * tile;
    let y = origin_y + entity_y * tile;
    let width = (entity_width * tile).max(1.0);
    let height = (entity_height * tile).max(1.0);
    rect_intersects_viewport(
        x - width * 0.5,
        y - height * 0.5,
        width,
        height,
        screen_width,
        screen_height,
        padding,
    )
}

fn rect_intersects_viewport(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    screen_width: f32,
    screen_height: f32,
    padding: f32,
) -> bool {
    x + width >= -padding
        && y + height >= -padding
        && x <= screen_width + padding
        && y <= screen_height + padding
}

fn visible_tile_bounds(
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    map_width: usize,
    map_height: usize,
    padding: isize,
) -> (usize, usize, usize, usize) {
    visible_tile_bounds_for_viewport(
        origin_x,
        origin_y,
        tile,
        map_width,
        map_height,
        screen_width(),
        screen_height(),
        padding,
    )
}

#[allow(clippy::too_many_arguments)]
fn visible_tile_bounds_for_viewport(
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    map_width: usize,
    map_height: usize,
    screen_width: f32,
    screen_height: f32,
    padding: isize,
) -> (usize, usize, usize, usize) {
    if tile <= f32::EPSILON || map_width == 0 || map_height == 0 {
        return (0, 0, 0, 0);
    }
    let pad = padding.max(0) as f32;
    let min_x = (((-origin_x) / tile).floor() - pad)
        .max(0.0)
        .min(map_width as f32) as usize;
    let max_x = (((screen_width - origin_x) / tile).ceil() + pad)
        .max(0.0)
        .min(map_width as f32) as usize;
    let min_y = (((-origin_y) / tile).floor() - pad)
        .max(0.0)
        .min(map_height as f32) as usize;
    let max_y = (((screen_height - origin_y) / tile).ceil() + pad)
        .max(0.0)
        .min(map_height as f32) as usize;
    (min_x, max_x, min_y, max_y)
}

fn draw_lighting(
    runtime: &EngineRuntime,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    graphics: &RuntimeGraphicsSettings,
) {
    if !graphics.lighting_enabled {
        return;
    }
    let casters = runtime
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.enabled)
        .filter(|entity| entity_intersects_screen(entity, origin_x, origin_y, tile, tile * 20.0))
        .filter(|entity| {
            entity
                .get_component("ShadowCaster2D")
                .is_some_and(|caster| caster.enabled)
        })
        .map(shadow_rect)
        .collect::<Vec<_>>();
    let mut lights = runtime
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.enabled)
        .filter_map(|entity| entity.get_component("Light2D").map(|light| (entity, light)))
        .filter(|(_, light)| light.enabled)
        .filter_map(|(entity, light)| {
            let radius = light.get_f64("radius", 5.0).max(0.1) as f32;
            let screen_radius = radius * tile;
            let center = vec2(entity.x as f32, entity.y as f32);
            let screen_center = world_to_screen(center, origin_x, origin_y, tile);
            circle_intersects_screen(screen_center, screen_radius).then_some((
                entity,
                light,
                screen_center,
                radius,
            ))
        })
        .collect::<Vec<_>>();
    if lights.is_empty() {
        return;
    }
    lights.sort_by(|a, b| {
        let ai = a.1.get_f64("intensity", 1.0);
        let bi = b.1.get_f64("intensity", 1.0);
        bi.total_cmp(&ai)
    });

    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(5, 8, 14, 145),
    );

    let mut shadow_light_count = 0usize;
    let mut source_core_count = 0usize;
    for (entity, light, screen_center, radius) in lights {
        let center = vec2(entity.x as f32, entity.y as f32);

        let intensity = light.get_f64("intensity", 1.0).max(0.0) as f32;
        if intensity <= f32::EPSILON {
            continue;
        }
        let wants_shadows = light.get_bool("casts_shadows", true);
        let casts_shadows = wants_shadows
            && graphics.shadow_lights_enabled
            && shadow_light_count < graphics.max_shadow_lights;
        if casts_shadows {
            shadow_light_count += 1;
        }
        let angle_degrees = light.get_f64("angle", 360.0).clamp(1.0, 360.0) as f32;
        let direction = (light.get_f64("direction", 0.0) as f32).to_radians();
        let color_alpha = if casts_shadows {
            (0.08 + intensity * 0.08).clamp(0.06, 0.24)
        } else {
            (intensity * 0.055).clamp(0.025, 0.12)
        };
        let light_color = color_from_component(light.get("color"), [255, 240, 200], color_alpha);
        let relevant_casters = casters
            .iter()
            .copied()
            .filter(|caster| {
                distance_to_rect(center, *caster) <= radius + graphics.light_caster_padding
            })
            .collect::<Vec<_>>();

        draw_light_fan(
            center,
            radius,
            direction,
            angle_degrees,
            casts_shadows,
            &relevant_casters,
            origin_x,
            origin_y,
            tile,
            light_color,
            graphics.light_sample_budget,
        );
        if casts_shadows {
            draw_shadow_wedges(
                center,
                radius,
                &relevant_casters,
                origin_x,
                origin_y,
                tile,
                (intensity * 0.12).clamp(0.06, 0.22),
            );
        }
        if entity.visible && source_core_count < graphics.max_source_cores {
            source_core_count += 1;
            draw_circle(
                screen_center.x,
                screen_center.y,
                (tile * 0.12).clamp(2.0, 10.0),
                color_from_component(
                    light.get("color"),
                    [255, 240, 200],
                    (intensity * 0.42).clamp(0.12, 0.7),
                ),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_light_fan(
    center: Vec2,
    radius: f32,
    direction: f32,
    angle_degrees: f32,
    casts_shadows: bool,
    casters: &[ShadowRect],
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    color: Color,
    sample_budget: usize,
) {
    let full_circle = angle_degrees >= LIGHT_FULL_CIRCLE_DEGREES;
    let angle_radians = angle_degrees.to_radians();
    let sample_budget = sample_budget.clamp(8, 160);
    let samples = if full_circle {
        sample_budget
    } else {
        ((angle_degrees / 360.0) * sample_budget as f32)
            .ceil()
            .clamp(8.0, sample_budget as f32) as usize
    };
    let start = if full_circle {
        0.0
    } else {
        direction - angle_radians * 0.5
    };
    let step = if full_circle {
        std::f32::consts::TAU / samples as f32
    } else {
        angle_radians / samples.saturating_sub(1).max(1) as f32
    };
    let mut points = Vec::with_capacity(samples + usize::from(full_circle));
    for index in 0..samples {
        let angle = start + step * index as f32;
        let direction = vec2(angle.cos(), angle.sin());
        let distance = if casts_shadows {
            raycast_shadow_rects(center, direction, radius, casters).unwrap_or(radius)
        } else {
            radius
        };
        points.push(world_to_screen(
            center + direction * distance,
            origin_x,
            origin_y,
            tile,
        ));
    }
    if full_circle && let Some(first) = points.first().copied() {
        points.push(first);
    }

    let screen_center = world_to_screen(center, origin_x, origin_y, tile);
    for pair in points.windows(2) {
        draw_triangle(screen_center, pair[0], pair[1], color);
    }
}

fn draw_shadow_wedges(
    center: Vec2,
    radius: f32,
    casters: &[ShadowRect],
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    alpha: f32,
) {
    let color = Color::new(0.0, 0.0, 0.0, alpha);
    for caster in casters {
        if rect_contains(*caster, center) || distance_to_rect(center, *caster) > radius {
            continue;
        }
        let Some((near_a, near_b, angle_a, angle_b)) = silhouette_edges(center, *caster) else {
            continue;
        };
        let far_a = center + vec2(angle_a.cos(), angle_a.sin()) * radius;
        let far_b = center + vec2(angle_b.cos(), angle_b.sin()) * radius;
        let near_a = world_to_screen(near_a, origin_x, origin_y, tile);
        let near_b = world_to_screen(near_b, origin_x, origin_y, tile);
        let far_a = world_to_screen(far_a, origin_x, origin_y, tile);
        let far_b = world_to_screen(far_b, origin_x, origin_y, tile);
        draw_triangle(near_a, near_b, far_b, color);
        draw_triangle(near_a, far_b, far_a, color);
    }
}

fn shadow_rect(entity: &GameObject) -> ShadowRect {
    let half_w = (entity.width as f32 * entity.scale_x.abs() as f32).max(0.1) * 0.5;
    let half_h = (entity.height as f32 * entity.scale_y.abs() as f32).max(0.1) * 0.5;
    ShadowRect {
        min: vec2(entity.x as f32 - half_w, entity.y as f32 - half_h),
        max: vec2(entity.x as f32 + half_w, entity.y as f32 + half_h),
    }
}

fn world_to_screen(point: Vec2, origin_x: f32, origin_y: f32, tile: f32) -> Vec2 {
    vec2(origin_x + point.x * tile, origin_y + point.y * tile)
}

fn circle_intersects_screen(center: Vec2, radius: f32) -> bool {
    center.x + radius >= 0.0
        && center.y + radius >= 0.0
        && center.x - radius <= screen_width()
        && center.y - radius <= screen_height()
}

fn raycast_shadow_rects(
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
    casters: &[ShadowRect],
) -> Option<f32> {
    casters
        .iter()
        .filter(|rect| !rect_contains(**rect, origin))
        .filter_map(|rect| raycast_rect(origin, direction, max_distance, *rect))
        .min_by(|a, b| a.total_cmp(b))
}

fn raycast_rect(origin: Vec2, direction: Vec2, max_distance: f32, rect: ShadowRect) -> Option<f32> {
    let mut t_min: f32 = 0.0;
    let mut t_max = max_distance;
    for (origin_axis, direction_axis, min_axis, max_axis) in [
        (origin.x, direction.x, rect.min.x, rect.max.x),
        (origin.y, direction.y, rect.min.y, rect.max.y),
    ] {
        if direction_axis.abs() < 0.0001 {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }
        let inv = 1.0 / direction_axis;
        let mut t1 = (min_axis - origin_axis) * inv;
        let mut t2 = (max_axis - origin_axis) * inv;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }
    (t_min >= 0.0 && t_min <= max_distance).then_some(t_min)
}

fn rect_contains(rect: ShadowRect, point: Vec2) -> bool {
    point.x >= rect.min.x && point.x <= rect.max.x && point.y >= rect.min.y && point.y <= rect.max.y
}

fn distance_to_rect(point: Vec2, rect: ShadowRect) -> f32 {
    let dx = if point.x < rect.min.x {
        rect.min.x - point.x
    } else if point.x > rect.max.x {
        point.x - rect.max.x
    } else {
        0.0
    };
    let dy = if point.y < rect.min.y {
        rect.min.y - point.y
    } else if point.y > rect.max.y {
        point.y - rect.max.y
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn silhouette_edges(center: Vec2, rect: ShadowRect) -> Option<(Vec2, Vec2, f32, f32)> {
    let corners = [
        vec2(rect.min.x, rect.min.y),
        vec2(rect.max.x, rect.min.y),
        vec2(rect.max.x, rect.max.y),
        vec2(rect.min.x, rect.max.y),
    ];
    let mut angles = corners
        .iter()
        .map(|corner| {
            let angle = (corner.y - center.y).atan2(corner.x - center.x);
            (angle, *corner)
        })
        .collect::<Vec<_>>();
    angles.sort_by(|a, b| a.0.total_cmp(&b.0));
    if angles.len() < 2 {
        return None;
    }

    let mut largest_gap = f32::MIN;
    let mut gap_index = 0;
    for index in 0..angles.len() {
        let current = angles[index].0;
        let next = if index + 1 < angles.len() {
            angles[index + 1].0
        } else {
            angles[0].0 + std::f32::consts::TAU
        };
        let gap = next - current;
        if gap > largest_gap {
            largest_gap = gap;
            gap_index = index;
        }
    }

    let start_index = (gap_index + 1) % angles.len();
    let end_index = gap_index;
    let start = angles[start_index];
    let mut end = angles[end_index];
    if end.0 < start.0 {
        end.0 += std::f32::consts::TAU;
    }
    Some((start.1, end.1, start.0, end.0))
}

fn color_from_component(value: Option<&serde_json::Value>, fallback: [u8; 3], alpha: f32) -> Color {
    let channels = value.and_then(serde_json::Value::as_array);
    let channel = |index: usize, default: u8| {
        channels
            .and_then(|values| values.get(index))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value.min(255) as u8)
            .unwrap_or(default)
    };
    Color::from_rgba(
        channel(0, fallback[0]),
        channel(1, fallback[1]),
        channel(2, fallback[2]),
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

fn safe_mode_requested_from_environment() -> bool {
    std::env::args().any(|argument| argument == "--safe-mode")
        || std::env::var("MINIFORGE_SAFE_MODE")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_raycast_hits_nearest_shadow_rect() {
        let casters = [
            ShadowRect {
                min: vec2(3.0, -1.0),
                max: vec2(4.0, 1.0),
            },
            ShadowRect {
                min: vec2(7.0, -1.0),
                max: vec2(8.0, 1.0),
            },
        ];

        let hit = raycast_shadow_rects(vec2(0.0, 0.0), vec2(1.0, 0.0), 10.0, &casters);
        assert_eq!(hit, Some(3.0));
    }

    #[test]
    fn light_raycast_ignores_rects_outside_ray() {
        let casters = [ShadowRect {
            min: vec2(3.0, 2.0),
            max: vec2(4.0, 3.0),
        }];

        let hit = raycast_shadow_rects(vec2(0.0, 0.0), vec2(1.0, 0.0), 10.0, &casters);
        assert_eq!(hit, None);
    }

    #[test]
    fn visible_tile_bounds_only_cover_viewport_plus_padding() {
        let bounds = visible_tile_bounds_for_viewport(-160.0, -80.0, 16.0, 96, 64, 320.0, 240.0, 1);
        assert_eq!(bounds, (9, 31, 4, 21));
    }

    #[test]
    fn entity_screen_culling_keeps_large_backdrops_and_skips_far_entities() {
        assert!(entity_intersects_screen_for_viewport(
            48.0, 32.0, 96.0, 64.0, -768.0, -512.0, 16.0, 1280.0, 720.0, 0.0,
        ));
        assert!(!entity_intersects_screen_for_viewport(
            500.0, 500.0, 1.0, 1.0, 0.0, 0.0, 16.0, 1280.0, 720.0, 16.0,
        ));
    }

    #[test]
    fn graphics_quality_parses_aliases_and_cycles() {
        assert_eq!(
            parse_graphics_quality("performance"),
            Some(GraphicsQuality::Low)
        );
        assert_eq!(
            parse_graphics_quality("balanced"),
            Some(GraphicsQuality::Medium)
        );
        assert_eq!(
            parse_graphics_quality("cinematic"),
            Some(GraphicsQuality::Ultra)
        );
        assert_eq!(
            next_graphics_quality(GraphicsQuality::Low),
            GraphicsQuality::Medium
        );
        assert_eq!(
            next_graphics_quality(GraphicsQuality::Ultra),
            GraphicsQuality::Low
        );
    }

    #[test]
    fn graphics_settings_apply_named_profile_before_top_level_overrides() {
        let data = serde_json::json!({
            "profiles": {
                "low": {
                    "light_sample_budget": 7,
                    "light_caster_padding": 99.0,
                    "max_drawn_entities": 120,
                    "max_source_cores": 3,
                    "minimap_entity_budget": 9,
                    "view_frustum_culling": false,
                    "occlusion_culling": true,
                    "lod_enabled": true,
                    "lod_far_pixels": 22.0,
                    "shadow_lights_enabled": false
                }
            },
            "max_drawn_entities": 900,
            "minimap_entity_budget": 40
        });
        let graphics = data.as_object().expect("graphics map");

        let mut override_settings = RuntimeGraphicsSettings::for_quality(GraphicsQuality::Low);
        override_settings.apply_runtime_config(graphics, false);
        assert_eq!(override_settings.light_sample_budget, 8);
        assert_eq!(override_settings.light_caster_padding, 32.0);
        assert_eq!(override_settings.max_drawn_entities, 120);
        assert_eq!(override_settings.max_source_cores, 3);
        assert_eq!(override_settings.minimap_entity_budget, 9);
        assert!(!override_settings.view_frustum_culling);
        assert!(override_settings.occlusion_culling);
        assert_eq!(override_settings.lod_far_pixels, 22.0);
        assert!(!override_settings.shadow_lights_enabled);

        let mut active_settings = RuntimeGraphicsSettings::for_quality(GraphicsQuality::Low);
        active_settings.apply_runtime_config(graphics, true);
        assert_eq!(active_settings.max_drawn_entities, 900);
        assert_eq!(active_settings.minimap_entity_budget, 40);
    }

    #[test]
    fn minimap_markers_prioritize_important_nearby_world_entities() {
        let player = vec2(10.0, 10.0);
        let mut police = GameObject::new(23.0, 10.0, Some("patrol".to_string()));
        police.tag = "Police".to_string();
        let mut vehicle = GameObject::new(11.0, 10.0, Some("taxi".to_string()));
        vehicle.tag = "Vehicle".to_string();
        let mut far_contact = GameObject::new(40.0, 10.0, Some("contact".to_string()));
        far_contact.tag = "Contact".to_string();

        let police_marker = minimap_marker(&police, player, 17.5, 1.0).expect("police marker");
        let vehicle_marker = minimap_marker(&vehicle, player, 17.5, 1.0).expect("vehicle marker");
        assert!(police_marker.priority < vehicle_marker.priority);
        assert!(vehicle_marker.distance_sq < police_marker.distance_sq);
        assert!(minimap_marker(&far_contact, player, 17.5, 1.0).is_none());
    }

    #[test]
    fn occlusion_culling_skips_entities_fully_covered_by_later_occluder() {
        let mut hidden = GameObject::new(5.0, 5.0, Some("HiddenCar".to_string()));
        hidden.width = 1.0;
        hidden.height = 1.0;
        let mut building = GameObject::new(5.0, 5.0, Some("Building".to_string()));
        building.width = 4.0;
        building.height = 4.0;
        building.add_component(crate::engine::component::Component::new("Occluder2D"));
        if let Some(sprite) = building.get_component_mut("SpriteRenderer") {
            sprite.set("sorting_order", serde_json::json!(4));
        }
        let occluders = screen_occluders(&[&hidden, &building], 0.0, 0.0, 16.0);
        assert_eq!(occluders.len(), 1);

        let graphics = RuntimeGraphicsSettings::for_quality(GraphicsQuality::Medium);
        assert_eq!(
            entity_draw_lod(&hidden, &graphics, &occluders, 0.0, 0.0, 16.0),
            None
        );
    }

    #[test]
    fn lod_simplifies_tiny_entities_but_keeps_player_full_detail() {
        let mut tiny = GameObject::new(5.0, 5.0, Some("TinyProp".to_string()));
        tiny.width = 0.25;
        tiny.height = 0.25;
        let mut player = tiny.clone();
        player.tag = "Player".to_string();
        let graphics = RuntimeGraphicsSettings::for_quality(GraphicsQuality::Medium);

        assert_eq!(
            entity_draw_lod(&tiny, &graphics, &[], 0.0, 0.0, 32.0),
            Some(EntityDrawLod::Simplified)
        );
        assert_eq!(
            entity_draw_lod(&player, &graphics, &[], 0.0, 0.0, 32.0),
            Some(EntityDrawLod::Full)
        );
    }
}
