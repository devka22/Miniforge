use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::engine::component::Component;
use crate::engine::error_handler::MFResult;
use crate::engine::tilemap_layers::{TileLayer, TilemapLayers};
use crate::engine::ui_canvas::{UiCanvasElement, layout_element_pixels, ui_canvases_from_value};
use crate::engine::world::RuntimeWorld;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;
use crate::runtime::engine_runtime::EngineRuntime;
use crate::systems::particle_system::ParticleSystem;

use super::backend::{
    BUILTIN_RADIAL_LIGHT_TEXTURE_ID, RenderBackend, SpriteBlendMode, SpriteDrawCommand,
    SpriteDrawOptions, SpriteMaterialEffect, SpriteRegionDrawCommand, TextDrawCommand,
    TextWrapMode,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTexture2D {
    pub texture_id: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeScene2DStats {
    pub tile_quads: usize,
    pub tile_layer_quads: usize,
    pub entity_quads: usize,
    pub particle_quads: usize,
    pub light_quads: usize,
    pub ui_quads: usize,
    pub ui_text_areas: usize,
    pub ui_canvas_quads: usize,
    pub ui_canvas_text_areas: usize,
    pub textured_ui_images: usize,
    pub minimap_quads: usize,
    pub textured_entities: usize,
}

/// Draws the currently migrated 2D passes of an exported runtime through any
/// backend.
///
/// Tilemap cells, UI panels and CPU particles intentionally expand to normal
/// sprite quads. This keeps their behavior identical on wgpu and compatibility
/// backends while leaving room for GPU batching behind the backend boundary.
pub fn draw_engine_runtime_scene_2d<B: RenderBackend>(
    backend: &mut B,
    runtime: &EngineRuntime,
    textures: &BTreeMap<String, RuntimeTexture2D>,
    width: f32,
    height: f32,
) -> MFResult<RuntimeScene2DStats> {
    let zoom = runtime.camera.zoom.clamp(0.1, 8.0) as f32;
    let tile = (runtime.grid.tile_size.max(1) as f32 * zoom).max(1.0);
    let origin_x = -runtime.camera.x as f32 * zoom;
    let origin_y = -runtime.camera.y as f32 * zoom;
    let mut stats = RuntimeScene2DStats::default();

    draw_grid_base(
        backend,
        &runtime.grid,
        origin_x,
        origin_y,
        tile,
        width,
        height,
        &mut stats,
    )?;
    draw_entities(
        backend,
        &runtime.runtime_world,
        textures,
        origin_x,
        origin_y,
        tile,
        width,
        height,
        EntityPass::BehindTiles,
        &mut stats,
    )?;
    draw_tile_layers(
        backend,
        &runtime.tilemap_layers,
        origin_x,
        origin_y,
        tile,
        width,
        height,
        false,
        &mut stats,
    )?;
    draw_entities(
        backend,
        &runtime.runtime_world,
        textures,
        origin_x,
        origin_y,
        tile,
        width,
        height,
        EntityPass::InFrontOfTiles,
        &mut stats,
    )?;
    draw_particles(
        backend,
        &runtime.runtime_world,
        &runtime.particle_system,
        origin_x,
        origin_y,
        tile,
        zoom,
        width,
        height,
        &mut stats,
    )?;
    draw_tile_layers(
        backend,
        &runtime.tilemap_layers,
        origin_x,
        origin_y,
        tile,
        width,
        height,
        true,
        &mut stats,
    )?;
    draw_runtime_lights(
        backend,
        &runtime.runtime_world,
        origin_x,
        origin_y,
        tile,
        width,
        height,
        &mut stats,
    )?;
    draw_runtime_ui(
        backend,
        &runtime.runtime_world,
        &runtime.tilemap_layers,
        textures,
        width,
        height,
        &mut stats,
    )?;
    draw_scene_ui_canvases(
        backend,
        &runtime.ui_canvases,
        textures,
        width,
        height,
        &mut stats,
    )?;
    Ok(stats)
}

pub fn scene_ui_sprite_paths(ui_canvases: &serde_json::Value) -> Vec<String> {
    let mut paths = ui_canvases_from_value(ui_canvases)
        .into_iter()
        .flat_map(|canvas| canvas.elements)
        .filter_map(|element| match element {
            UiCanvasElement::Image { sprite_path, .. } if !sprite_path.trim().is_empty() => {
                Some(sprite_path)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

pub fn entity_sprite_path(entity: &GameObject) -> Option<&str> {
    let sprite = entity.get_component("SpriteRenderer")?;
    [
        "_texture_path",
        "texture_path",
        "sprite_path",
        "source_asset",
    ]
    .into_iter()
    .find_map(|key| sprite.get(key).and_then(serde_json::Value::as_str))
    .filter(|path| !path.trim().is_empty())
}

pub fn entity_ui_sprite_path(entity: &GameObject) -> Option<&str> {
    ui_component_sprite_path(entity.get_component("UIElement")?)
}

pub fn draw_runtime_scene_2d<B: RenderBackend>(
    backend: &mut B,
    world: &RuntimeWorld,
    grid: &Grid,
    texture_ids: &BTreeMap<String, u64>,
    width: f32,
    height: f32,
) -> MFResult<RuntimeScene2DStats> {
    let columns = grid.width.max(1);
    let rows = grid.height.max(1);
    let tile = (width.max(1.0) / (columns as f32 + 2.0))
        .min(height.max(1.0) / (rows as f32 + 2.0))
        .max(1.0);
    let origin_x = (width.max(1.0) - columns as f32 * tile) * 0.5;
    let origin_y = (height.max(1.0) - rows as f32 * tile) * 0.5;
    let mut stats = RuntimeScene2DStats::default();

    for row in 0..rows {
        for column in 0..columns {
            let blocked = grid.get_tile(column, row).unwrap_or(0) != 0;
            let checker = (column + row) % 2 == 0;
            let color = if blocked {
                [0.19, 0.21, 0.25, 1.0]
            } else if checker {
                [0.055, 0.075, 0.085, 1.0]
            } else {
                [0.045, 0.065, 0.075, 1.0]
            };
            backend.draw_sprite(SpriteDrawCommand {
                entity_id: 10_000 + (row * columns + column) as u64,
                texture_id: 0,
                x: origin_x + column as f32 * tile,
                y: origin_y + row as f32 * tile,
                width: (tile - 1.0).max(1.0),
                height: (tile - 1.0).max(1.0),
                rotation: 0.0,
                color,
            })?;
            stats.tile_quads += 1;
        }
    }

    let mut entities = world
        .units
        .iter()
        .filter(|entity| entity.visible && entity.enabled && entity.active)
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| {
        entity_sorting_order(left)
            .cmp(&entity_sorting_order(right))
            .then_with(|| left.y.total_cmp(&right.y))
            .then_with(|| left.id.cmp(&right.id))
    });
    for entity in entities {
        let entity_width = (entity.width.abs() as f32 * tile).max(8.0);
        let entity_height = (entity.height.abs() as f32 * tile).max(8.0);
        let center_x = origin_x + entity.x as f32 * tile;
        let center_y = origin_y + entity.y as f32 * tile;
        let texture_id = entity_sprite_path(entity)
            .and_then(|path| texture_ids.get(path))
            .copied()
            .unwrap_or(0);
        backend.draw_sprite(SpriteDrawCommand {
            entity_id: entity.id,
            texture_id,
            x: center_x - entity_width * 0.5,
            y: center_y - entity_height * 0.5,
            width: entity_width,
            height: entity_height,
            rotation: entity.rotation as f32,
            color: entity_tint(entity),
        })?;
        stats.entity_quads += 1;
        stats.textured_entities += usize::from(texture_id != 0);
    }
    Ok(stats)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityPass {
    BehindTiles,
    InFrontOfTiles,
}

#[allow(clippy::too_many_arguments)]
fn draw_grid_base<B: RenderBackend>(
    backend: &mut B,
    grid: &Grid,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    width: f32,
    height: f32,
    stats: &mut RuntimeScene2DStats,
) -> MFResult<()> {
    let (min_x, max_x, min_y, max_y) = visible_tile_bounds(
        origin_x,
        origin_y,
        tile,
        grid.width,
        grid.height,
        width,
        height,
    );
    for row in min_y..max_y {
        for column in min_x..max_x {
            let blocked = grid.get_tile(column, row).unwrap_or(0) != 0;
            let checker = (column + row) % 2 == 0;
            let color = if blocked {
                [0.16, 0.18, 0.21, 1.0]
            } else if checker {
                [0.052, 0.071, 0.078, 1.0]
            } else {
                [0.043, 0.061, 0.069, 1.0]
            };
            draw_quad(
                backend,
                10_000 + (row * grid.width.max(1) + column) as u64,
                0,
                origin_x + column as f32 * tile,
                origin_y + row as f32 * tile,
                tile.ceil(),
                tile.ceil(),
                color,
            )?;
            stats.tile_quads += 1;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_tile_layers<B: RenderBackend>(
    backend: &mut B,
    tilemap: &TilemapLayers,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    width: f32,
    height: f32,
    overlay_pass: bool,
    stats: &mut RuntimeScene2DStats,
) -> MFResult<()> {
    let (min_x, max_x, min_y, max_y) = visible_tile_bounds(
        origin_x,
        origin_y,
        tile,
        tilemap.width,
        tilemap.height,
        width,
        height,
    );
    for (layer_index, layer) in tilemap.layers.iter().enumerate() {
        let overlay = layer.name.eq_ignore_ascii_case("overlay");
        if !layer.visible || overlay != overlay_pass {
            continue;
        }
        for row in min_y..max_y {
            for column in min_x..max_x {
                let value = layer.get(column, row);
                if value == 0 {
                    continue;
                }
                draw_quad(
                    backend,
                    1_000_000
                        + layer_index as u64 * 1_000_000
                        + (row * tilemap.width.max(1) + column) as u64,
                    0,
                    origin_x + column as f32 * tile,
                    origin_y + row as f32 * tile,
                    tile.ceil(),
                    tile.ceil(),
                    tile_color(value, layer, layer_index),
                )?;
                stats.tile_quads += 1;
                stats.tile_layer_quads += 1;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_runtime_lights<B: RenderBackend>(
    backend: &mut B,
    world: &RuntimeWorld,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    screen_width: f32,
    screen_height: f32,
    stats: &mut RuntimeScene2DStats,
) -> MFResult<()> {
    let mut lights = world
        .units
        .iter()
        .filter(|entity| entity.enabled && entity.visible)
        .filter_map(|entity| {
            let light = entity
                .get_component("Light2D")
                .filter(|light| light.enabled)?;
            let intensity = finite_f64_to_f32(light.get_f64("intensity", 1.0)).clamp(0.0, 8.0);
            let radius = finite_f64_to_f32(light.get_f64("radius", 5.0)).clamp(0.1, 1_024.0);
            (intensity > f32::EPSILON).then_some((entity, light, intensity, radius))
        })
        .collect::<Vec<_>>();
    lights.sort_by(|left, right| {
        right
            .2
            .total_cmp(&left.2)
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    for (entity, light, intensity, radius) in lights.into_iter().take(256) {
        let screen_radius = radius * tile;
        let center_x = origin_x + finite_f64_to_f32(entity.x) * tile;
        let center_y = origin_y + finite_f64_to_f32(entity.y) * tile;
        let x = center_x - screen_radius;
        let y = center_y - screen_radius;
        let diameter = screen_radius * 2.0;
        if !rect_intersects_screen(x, y, diameter, diameter, screen_width, screen_height) {
            continue;
        }
        let alpha = (0.18 * intensity).clamp(0.0, 1.0);
        let color = component_color(light.get("color"), [255, 240, 200, 255], alpha);
        backend.draw_sprite_with_options(
            SpriteDrawCommand {
                entity_id: (3u64 << 60).saturating_add(entity.id),
                texture_id: BUILTIN_RADIAL_LIGHT_TEXTURE_ID,
                x,
                y,
                width: diameter,
                height: diameter,
                rotation: 0.0,
                color,
            },
            SpriteDrawOptions {
                blend_mode: SpriteBlendMode::Additive,
                ..SpriteDrawOptions::default()
            },
        )?;
        stats.light_quads += 1;
    }
    Ok(())
}

fn tile_color(value: i32, layer: &TileLayer, layer_index: usize) -> [f32; 4] {
    const PALETTE: [[f32; 3]; 8] = [
        [0.18, 0.32, 0.24],
        [0.30, 0.23, 0.17],
        [0.29, 0.34, 0.40],
        [0.22, 0.40, 0.34],
        [0.44, 0.30, 0.24],
        [0.23, 0.31, 0.46],
        [0.43, 0.42, 0.27],
        [0.36, 0.27, 0.42],
    ];
    let rgb = PALETTE[value.unsigned_abs() as usize % PALETTE.len()];
    let alpha = if layer.name.eq_ignore_ascii_case("ground") || layer_index == 0 {
        1.0
    } else if layer.name.eq_ignore_ascii_case("collision") {
        0.38
    } else {
        0.78
    };
    [rgb[0], rgb[1], rgb[2], alpha]
}

#[allow(clippy::too_many_arguments)]
fn draw_entities<B: RenderBackend>(
    backend: &mut B,
    world: &RuntimeWorld,
    textures: &BTreeMap<String, RuntimeTexture2D>,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    screen_width: f32,
    screen_height: f32,
    pass: EntityPass,
    stats: &mut RuntimeScene2DStats,
) -> MFResult<()> {
    let mut entities = world
        .units
        .iter()
        .filter(|entity| {
            entity.visible
                && entity.enabled
                && entity.active
                && entity.get_component("UIElement").is_none()
                && entity
                    .get_component("SpriteRenderer")
                    .is_none_or(|sprite| sprite.get_bool("visible", true))
        })
        .filter(|entity| match pass {
            EntityPass::BehindTiles => entity_sorting_order(entity) < 0,
            EntityPass::InFrontOfTiles => entity_sorting_order(entity) >= 0,
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| {
        entity_sorting_order(left)
            .cmp(&entity_sorting_order(right))
            .then_with(|| left.y.total_cmp(&right.y))
            .then_with(|| left.id.cmp(&right.id))
    });
    for entity in entities {
        let entity_width =
            (entity.width.abs() as f32 * entity.scale_x.abs() as f32 * tile).max(1.0);
        let entity_height =
            (entity.height.abs() as f32 * entity.scale_y.abs() as f32 * tile).max(1.0);
        let center_x = origin_x + entity.x as f32 * tile;
        let center_y = origin_y + entity.y as f32 * tile;
        let x = center_x - entity_width * 0.5;
        let y = center_y - entity_height * 0.5;
        if !rect_intersects_screen(
            x,
            y,
            entity_width,
            entity_height,
            screen_width,
            screen_height,
        ) {
            continue;
        }
        let binding = entity_sprite_path(entity).and_then(|path| textures.get(path));
        let sprite = SpriteDrawCommand {
            entity_id: entity.id,
            texture_id: binding.map_or(0, |binding| binding.texture_id),
            x,
            y,
            width: entity_width,
            height: entity_height,
            rotation: (entity.rotation as f32).to_radians(),
            color: entity_tint(entity),
        };
        let options = entity_sprite_options(entity);
        if let Some((binding, uv_rect)) = binding
            .and_then(|binding| entity_source_uv(entity, binding).map(|uv_rect| (binding, uv_rect)))
        {
            backend.draw_sprite_region_with_options(
                SpriteRegionDrawCommand {
                    sprite: SpriteDrawCommand {
                        texture_id: binding.texture_id,
                        ..sprite
                    },
                    uv_rect,
                    clip_rect: None,
                },
                options,
            )?;
        } else {
            backend.draw_sprite_with_options(sprite, options)?;
        }
        stats.entity_quads += 1;
        stats.textured_entities += usize::from(binding.is_some());
    }
    Ok(())
}

fn entity_source_uv(entity: &GameObject, texture: &RuntimeTexture2D) -> Option<[f32; 4]> {
    let rect = entity
        .get_component("SpriteRenderer")?
        .get("_source_rect")?
        .as_object()?;
    let x = rect
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0) as f32;
    let y = rect
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0) as f32;
    let width = rect
        .get("width")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(texture.width as f64) as f32;
    let height = rect
        .get("height")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(texture.height as f64) as f32;
    let texture_width = texture.width.max(1) as f32;
    let texture_height = texture.height.max(1) as f32;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let uv_rect = [
        (x / texture_width).clamp(0.0, 1.0),
        (y / texture_height).clamp(0.0, 1.0),
        ((x + width) / texture_width).clamp(0.0, 1.0),
        ((y + height) / texture_height).clamp(0.0, 1.0),
    ];
    (uv_rect[0] < uv_rect[2] && uv_rect[1] < uv_rect[3]).then_some(uv_rect)
}

#[allow(clippy::too_many_arguments)]
fn draw_particles<B: RenderBackend>(
    backend: &mut B,
    world: &RuntimeWorld,
    particles: &ParticleSystem,
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    zoom: f32,
    screen_width: f32,
    screen_height: f32,
    stats: &mut RuntimeScene2DStats,
) -> MFResult<()> {
    for (emitter_id, state) in &particles.emitters {
        let Some(emitter) = world.entity(*emitter_id) else {
            continue;
        };
        let blend_mode = emitter
            .get_component("ParticleEmitter")
            .and_then(|component| component.get("blend_mode"))
            .and_then(serde_json::Value::as_str)
            .and_then(SpriteBlendMode::from_name)
            .unwrap_or(SpriteBlendMode::Alpha);
        for (index, particle) in state.particles.iter().enumerate() {
            let size = (particle.size as f32 * zoom).max(1.0);
            let x = origin_x + emitter.x as f32 * tile + (particle.x - emitter.x) as f32 * zoom
                - size * 0.5;
            let y = origin_y + emitter.y as f32 * tile + (particle.y - emitter.y) as f32 * zoom
                - size * 0.5;
            if !rect_intersects_screen(x, y, size, size, screen_width, screen_height) {
                continue;
            }
            let particle_id = 8_000_000_000u64
                .saturating_add(emitter_id.saturating_mul(1_000_000))
                .saturating_add(index as u64);
            draw_quad_with_options(
                backend,
                particle_id,
                0,
                x,
                y,
                size,
                size,
                particle.color.map(|channel| channel as f32 / 255.0),
                SpriteDrawOptions {
                    blend_mode,
                    ..SpriteDrawOptions::default()
                },
            )?;
            stats.particle_quads += 1;
        }
    }
    Ok(())
}

fn draw_runtime_ui<B: RenderBackend>(
    backend: &mut B,
    world: &RuntimeWorld,
    tilemap_layers: &TilemapLayers,
    textures: &BTreeMap<String, RuntimeTexture2D>,
    screen_width: f32,
    screen_height: f32,
    stats: &mut RuntimeScene2DStats,
) -> MFResult<()> {
    let mut elements = world
        .units
        .iter()
        .filter_map(|entity| entity.get_component("UIElement").map(|ui| (entity, ui)))
        .filter(|(entity, ui)| entity.enabled && entity.visible && ui.get_bool("visible", true))
        .collect::<Vec<_>>();
    elements.sort_by_key(|(entity, ui)| (ui.get_i64("sorting_order", 0), entity.id));
    for (entity, ui) in elements {
        let ui_base = (1u64 << 62).saturating_add(entity.id.saturating_mul(100_000));
        let x = ui.get_f64("x", 0.0) as f32;
        let y = ui.get_f64("y", 0.0) as f32;
        let width = ui.get_f64("width", 160.0).max(0.0) as f32;
        let height = ui.get_f64("height", 36.0).max(0.0) as f32;
        let opacity = ui.get_f64("opacity", 1.0).clamp(0.0, 1.0) as f32;
        if opacity <= 0.01
            || !rect_intersects_screen(x, y, width, height, screen_width, screen_height)
        {
            continue;
        }
        let kind = ui.get_string("element_type", "Label");
        let image_binding = if matches!(kind.as_str(), "Image" | "NineSlice") {
            ui_component_sprite_path(ui).and_then(|path| textures.get(path))
        } else {
            None
        };
        let background = component_color(ui.get("color"), [24, 28, 36, 255], opacity);
        if kind == "NineSlice"
            && let Some(texture) = image_binding
        {
            let slice_quads =
                draw_nine_slice(backend, ui_base, ui, texture, x, y, width, height, opacity)?;
            stats.ui_quads += slice_quads;
            stats.textured_ui_images += 1;
        } else {
            draw_quad(
                backend,
                ui_base,
                image_binding.map_or(0, |texture| texture.texture_id),
                x,
                y,
                width,
                height,
                if image_binding.is_some() {
                    [1.0, 1.0, 1.0, opacity]
                } else {
                    background
                },
            )?;
            stats.ui_quads += 1;
            stats.textured_ui_images += usize::from(image_binding.is_some());

            if !matches!(kind.as_str(), "Image" | "NineSlice") || image_binding.is_none() {
                let border = component_color(ui.get("border_color"), [92, 112, 142, 255], opacity);
                for (index, (bx, by, bw, bh)) in [
                    (x, y, width, 1.0),
                    (x, y + (height - 1.0).max(0.0), width, 1.0),
                    (x, y, 1.0, height),
                    (x + (width - 1.0).max(0.0), y, 1.0, height),
                ]
                .into_iter()
                .enumerate()
                {
                    draw_quad(
                        backend,
                        ui_base.saturating_add(index as u64 + 1),
                        0,
                        bx,
                        by,
                        bw,
                        bh,
                        border,
                    )?;
                    stats.ui_quads += 1;
                }
            }
        }

        if matches!(kind.as_str(), "ProgressBar" | "StatBar") {
            let max = ui.get_f64("max_progress", 1.0).max(0.0001);
            let progress = (ui.get_f64("progress", 0.0) / max).clamp(0.0, 1.0) as f32;
            let fill = if entity.name.contains("Health") {
                [1.0, 0.35, 0.55, opacity]
            } else {
                [0.36, 0.78, 1.0, opacity]
            };
            draw_quad(
                backend,
                ui_base.saturating_add(5),
                0,
                x + 3.0,
                y + 3.0,
                (width - 6.0).max(0.0) * progress,
                (height - 6.0).max(0.0),
                fill,
            )?;
            stats.ui_quads += 1;
        }
        match kind.as_str() {
            "Slider" => {
                let normalized = normalized_ui_slider(ui);
                let track_x = x + 8.0;
                let track_width = (width - 16.0).max(1.0);
                let track_y = y + height * 0.5 - 2.0;
                let accent = component_color(ui.get("accent_color"), [92, 186, 255, 255], opacity);
                draw_quad(
                    backend,
                    ui_base.saturating_add(16),
                    0,
                    track_x,
                    track_y,
                    track_width,
                    4.0,
                    [0.22, 0.25, 0.31, opacity],
                )?;
                draw_quad(
                    backend,
                    ui_base.saturating_add(17),
                    0,
                    track_x,
                    track_y,
                    track_width * normalized,
                    4.0,
                    accent,
                )?;
                draw_quad(
                    backend,
                    ui_base.saturating_add(18),
                    0,
                    track_x + track_width * normalized - 5.0,
                    y + height * 0.5 - 5.0,
                    10.0,
                    10.0,
                    accent,
                )?;
                stats.ui_quads += 3;
            }
            "Checkbox" | "Toggle" => {
                let box_size = (height - 8.0).clamp(8.0, 24.0);
                let box_x = x + 4.0;
                let box_y = y + (height - box_size) * 0.5;
                let accent = component_color(ui.get("accent_color"), [92, 186, 255, 255], opacity);
                draw_quad(
                    backend,
                    ui_base.saturating_add(16),
                    0,
                    box_x,
                    box_y,
                    box_size,
                    box_size,
                    if ui.get_bool("checked", false) {
                        accent
                    } else {
                        [0.16, 0.18, 0.23, opacity]
                    },
                )?;
                stats.ui_quads += 1;
                if ui.get_bool("checked", false) {
                    let mark = [0.96, 0.98, 1.0, opacity];
                    draw_quad(
                        backend,
                        ui_base.saturating_add(17),
                        0,
                        box_x + box_size * 0.22,
                        box_y + box_size * 0.52,
                        box_size * 0.28,
                        2.0,
                        mark,
                    )?;
                    draw_quad(
                        backend,
                        ui_base.saturating_add(18),
                        0,
                        box_x + box_size * 0.43,
                        box_y + box_size * 0.36,
                        box_size * 0.4,
                        2.0,
                        mark,
                    )?;
                    stats.ui_quads += 2;
                }
            }
            "InventoryGrid" | "AbilityBar" => {
                let columns = ui.get_i64("columns", 4).clamp(1, 16) as usize;
                let slot_count = ui
                    .get("slot_count")
                    .or_else(|| ui.get("slots"))
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(columns as i64)
                    .clamp(1, 256) as usize;
                let gap = 2.0;
                let max_slot_width = ((width - 8.0 - gap * (columns.saturating_sub(1)) as f32)
                    / columns as f32)
                    .max(1.0);
                let slot_size = (ui.get_f64("slot_size", 32.0) as f32)
                    .clamp(8.0, 128.0)
                    .min(max_slot_width);
                let items = ui.get("items").and_then(serde_json::Value::as_array);
                for slot in 0..slot_count {
                    let column = slot % columns;
                    let row = slot / columns;
                    let slot_x = x + 4.0 + column as f32 * (slot_size + gap);
                    let slot_y = y + 4.0 + row as f32 * (slot_size + gap);
                    if slot_y + slot_size > y + height - 3.0 {
                        break;
                    }
                    let occupied = items
                        .and_then(|values| values.get(slot))
                        .is_some_and(|value| !value.is_null());
                    draw_quad(
                        backend,
                        ui_base.saturating_add(32 + slot as u64),
                        0,
                        slot_x,
                        slot_y,
                        slot_size,
                        slot_size,
                        if occupied {
                            component_color(ui.get("accent_color"), [82, 150, 216, 255], opacity)
                        } else {
                            [0.12, 0.14, 0.18, opacity]
                        },
                    )?;
                    stats.ui_quads += 1;
                }
            }
            "InputField" | "TextInput" if ui.get_bool("focused", false) => {
                let caret_height = (height - 12.0).clamp(8.0, 28.0);
                draw_quad(
                    backend,
                    ui_base.saturating_add(16),
                    0,
                    x + width - 10.0,
                    y + (height - caret_height) * 0.5,
                    1.0,
                    caret_height,
                    [0.9, 0.94, 1.0, opacity],
                )?;
                stats.ui_quads += 1;
            }
            "Minimap" => {
                let quads = draw_runtime_minimap(
                    backend,
                    ui_base.saturating_add(1_000),
                    ui,
                    world,
                    tilemap_layers,
                    x,
                    y,
                    width,
                    height,
                    opacity,
                )?;
                stats.ui_quads += quads;
                stats.minimap_quads += quads;
            }
            _ => {}
        }

        let authored_text = ui.get_string("text", "");
        let placeholder = ui.get_string("placeholder", "");
        let uses_placeholder = authored_text.trim().is_empty()
            && matches!(kind.as_str(), "InputField" | "TextInput")
            && !placeholder.trim().is_empty();
        let text = if uses_placeholder {
            placeholder
        } else {
            authored_text
        };
        if !text.trim().is_empty() {
            let (font_size, line_height) = resolved_ui_text_metrics(ui);
            let text_opacity = if uses_placeholder {
                opacity * 0.55
            } else {
                opacity
            };
            let text_color =
                component_color(ui.get("text_color"), [235, 240, 248, 255], text_opacity)
                    .map(|channel| (channel * 255.0).round().clamp(0.0, 255.0) as u8);
            let text_left = if matches!(kind.as_str(), "Checkbox" | "Toggle") {
                (height - 8.0).clamp(8.0, 24.0) + 10.0
            } else {
                6.0
            };
            backend.draw_text(TextDrawCommand {
                text_id: ui_base.saturating_add(900),
                text,
                font_family: ui.get_string("font_family", ""),
                x: x + text_left,
                y: y + ((height - line_height) * 0.5).max(2.0),
                width: (width - text_left - 6.0).max(1.0),
                height: (height - 4.0).max(1.0),
                font_size,
                line_height,
                color: text_color,
                wrap: match ui.get_string("wrap", "word").as_str() {
                    "none" | "no_wrap" => TextWrapMode::None,
                    "glyph" | "character" => TextWrapMode::Glyph,
                    _ => TextWrapMode::Word,
                },
                clip_rect: None,
            })?;
            stats.ui_text_areas += 1;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct RuntimeMinimapMarker {
    entity_id: u64,
    x: f32,
    y: f32,
    color: [f32; 4],
    priority: u8,
    distance_squared: f32,
}

#[allow(clippy::too_many_arguments)]
fn draw_runtime_minimap<B: RenderBackend>(
    backend: &mut B,
    minimap_id: u64,
    ui: &Component,
    world: &RuntimeWorld,
    tilemap_layers: &TilemapLayers,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
) -> MFResult<usize> {
    let inner_x = x + 6.0;
    let inner_y = y + 6.0;
    let inner_width = (width - 12.0).max(1.0);
    let inner_height = (height - 12.0).max(1.0);
    let center_x = inner_x + inner_width * 0.5;
    let center_y = inner_y + inner_height * 0.5;
    let player = world
        .units
        .iter()
        .find(|entity| entity.enabled && entity.visible && entity.tag == "Player")
        .map(|entity| (finite_f64_to_f32(entity.x), finite_f64_to_f32(entity.y)))
        .unwrap_or((0.0, 0.0));
    let world_radius = finite_f64_to_f32(ui.get_f64("world_radius", 17.5)).clamp(1.0, 512.0);
    let scale = (inner_width.min(inner_height) * 0.5 / world_radius).max(0.01);
    let mut quads = 0;

    let tile_budget = ui.get_i64("tile_budget", 4_096).clamp(0, 16_384) as usize;
    let road_layer = tilemap_layers
        .layer("Decoration")
        .or_else(|| tilemap_layers.layers.get(1));
    if let Some(layer) = road_layer {
        let min_tile_x = ((player.0 - world_radius).floor() as isize).max(0) as usize;
        let max_tile_x = ((player.0 + world_radius).ceil() as isize)
            .max(0)
            .min(tilemap_layers.width as isize) as usize;
        let min_tile_y = ((player.1 - world_radius).floor() as isize).max(0) as usize;
        let max_tile_y = ((player.1 + world_radius).ceil() as isize)
            .max(0)
            .min(tilemap_layers.height as isize) as usize;
        'tiles: for tile_y in min_tile_y..max_tile_y {
            for tile_x in min_tile_x..max_tile_x {
                if quads >= tile_budget {
                    break 'tiles;
                }
                if layer.get(tile_x, tile_y) == 0 {
                    continue;
                }
                let point_x = center_x + (tile_x as f32 + 0.5 - player.0) * scale;
                let point_y = center_y + (tile_y as f32 + 0.5 - player.1) * scale;
                let tile_size = scale.max(1.0);
                if !rect_inside(
                    point_x - tile_size * 0.5,
                    point_y - tile_size * 0.5,
                    tile_size,
                    tile_size,
                    inner_x,
                    inner_y,
                    inner_width,
                    inner_height,
                ) {
                    continue;
                }
                draw_quad(
                    backend,
                    minimap_id.saturating_add(quads as u64),
                    0,
                    point_x - tile_size * 0.5,
                    point_y - tile_size * 0.5,
                    tile_size,
                    tile_size,
                    [0.55, 0.78, 0.76, opacity * 0.72],
                )?;
                quads += 1;
            }
        }
    }

    let marker_budget = ui.get_i64("marker_budget", 128).clamp(0, 2_048) as usize;
    let mut markers = world
        .units
        .iter()
        .filter(|entity| entity.enabled && entity.visible && entity.tag != "Player")
        .filter_map(|entity| runtime_minimap_marker(entity, player, world_radius, opacity))
        .collect::<Vec<_>>();
    markers.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.distance_squared.total_cmp(&right.distance_squared))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
    });
    for marker in markers.into_iter().take(marker_budget) {
        let marker_x = center_x + (marker.x - player.0) * scale;
        let marker_y = center_y + (marker.y - player.1) * scale;
        let marker_size = 5.0;
        if !rect_inside(
            marker_x - marker_size * 0.5,
            marker_y - marker_size * 0.5,
            marker_size,
            marker_size,
            inner_x,
            inner_y,
            inner_width,
            inner_height,
        ) {
            continue;
        }
        draw_quad(
            backend,
            minimap_id.saturating_add(20_000 + quads as u64),
            0,
            marker_x - marker_size * 0.5,
            marker_y - marker_size * 0.5,
            marker_size,
            marker_size,
            marker.color,
        )?;
        quads += 1;
    }

    backend.draw_sprite(SpriteDrawCommand {
        entity_id: minimap_id.saturating_add(90_000),
        texture_id: 0,
        x: center_x - 4.0,
        y: center_y - 4.0,
        width: 8.0,
        height: 8.0,
        rotation: std::f32::consts::FRAC_PI_4,
        color: [1.0, 0.3, 0.74, opacity],
    })?;
    Ok(quads + 1)
}

fn runtime_minimap_marker(
    entity: &GameObject,
    player: (f32, f32),
    world_radius: f32,
    opacity: f32,
) -> Option<RuntimeMinimapMarker> {
    let (color, priority) = if entity.get_component("ObjectiveMarker").is_some() {
        ([0.35, 0.94, 1.0, opacity], 0)
    } else {
        match entity.tag.as_str() {
            "Police" => ([0.32, 0.61, 1.0, opacity], 1),
            "Enemy" | "Zombie" => ([1.0, 0.28, 0.32, opacity], 2),
            "Contact" | "NPC" => ([0.77, 0.49, 1.0, opacity], 3),
            "Collectible" | "Resource" => ([0.35, 1.0, 0.76, opacity], 4),
            "Vehicle" => ([1.0, 0.8, 0.43, opacity], 5),
            _ => return None,
        }
    };
    let x = finite_f64_to_f32(entity.x);
    let y = finite_f64_to_f32(entity.y);
    let dx = x - player.0;
    let dy = y - player.1;
    let distance_squared = dx * dx + dy * dy;
    (distance_squared <= world_radius * world_radius).then_some(RuntimeMinimapMarker {
        entity_id: entity.id,
        x,
        y,
        color,
        priority,
        distance_squared,
    })
}

fn finite_f64_to_f32(value: f64) -> f32 {
    if value.is_finite() {
        value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
    } else {
        0.0
    }
}

#[allow(clippy::too_many_arguments)]
fn rect_inside(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    bounds_x: f32,
    bounds_y: f32,
    bounds_width: f32,
    bounds_height: f32,
) -> bool {
    x >= bounds_x
        && y >= bounds_y
        && x + width <= bounds_x + bounds_width
        && y + height <= bounds_y + bounds_height
}

fn ui_component_sprite_path(ui: &Component) -> Option<&str> {
    [
        "texture_path",
        "sprite_path",
        "image_path",
        "image_name",
        "source_asset",
    ]
    .into_iter()
    .find_map(|key| ui.get(key).and_then(serde_json::Value::as_str))
    .filter(|path| !path.trim().is_empty())
}

#[allow(clippy::too_many_arguments)]
fn draw_nine_slice<B: RenderBackend>(
    backend: &mut B,
    element_id: u64,
    ui: &Component,
    texture: &RuntimeTexture2D,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
) -> MFResult<usize> {
    let texture_width = texture.width.max(1) as f32;
    let texture_height = texture.height.max(1) as f32;
    let left_source = (ui.get_f64("slice_left", 8.0) as f32).clamp(0.0, texture_width * 0.5);
    let right_source = (ui.get_f64("slice_right", 8.0) as f32).clamp(0.0, texture_width * 0.5);
    let top_source = (ui.get_f64("slice_top", 8.0) as f32).clamp(0.0, texture_height * 0.5);
    let bottom_source = (ui.get_f64("slice_bottom", 8.0) as f32).clamp(0.0, texture_height * 0.5);
    let left = left_source.min(width * 0.5);
    let right = right_source.min(width * 0.5);
    let top = top_source.min(height * 0.5);
    let bottom = bottom_source.min(height * 0.5);
    let xs = [x, x + left, x + width - right, x + width];
    let ys = [y, y + top, y + height - bottom, y + height];
    let us = [
        0.0,
        left_source / texture_width,
        1.0 - right_source / texture_width,
        1.0,
    ];
    let vs = [
        0.0,
        top_source / texture_height,
        1.0 - bottom_source / texture_height,
        1.0,
    ];
    let mut quads = 0;
    for row in 0..3 {
        for column in 0..3 {
            let slice_width = xs[column + 1] - xs[column];
            let slice_height = ys[row + 1] - ys[row];
            if slice_width <= 0.0 || slice_height <= 0.0 {
                continue;
            }
            backend.draw_sprite_region(SpriteRegionDrawCommand {
                sprite: SpriteDrawCommand {
                    entity_id: element_id.saturating_add((row * 3 + column) as u64),
                    texture_id: texture.texture_id,
                    x: xs[column],
                    y: ys[row],
                    width: slice_width,
                    height: slice_height,
                    rotation: 0.0,
                    color: [1.0, 1.0, 1.0, opacity],
                },
                uv_rect: [us[column], vs[row], us[column + 1], vs[row + 1]],
                clip_rect: None,
            })?;
            quads += 1;
        }
    }
    Ok(quads)
}

fn draw_scene_ui_canvases<B: RenderBackend>(
    backend: &mut B,
    ui_canvases: &serde_json::Value,
    textures: &BTreeMap<String, RuntimeTexture2D>,
    screen_width: f32,
    screen_height: f32,
    stats: &mut RuntimeScene2DStats,
) -> MFResult<()> {
    for (canvas_index, canvas) in ui_canvases_from_value(ui_canvases).iter().enumerate() {
        for (element_index, element) in canvas.elements.iter().enumerate() {
            let Some((x, y, width, height)) =
                finite_ui_canvas_rect(canvas, element, screen_width, screen_height)
            else {
                continue;
            };
            if !rect_intersects_screen(x, y, width, height, screen_width, screen_height) {
                continue;
            }
            let element_base = (1u64 << 63)
                .saturating_add((canvas_index as u64).saturating_mul(1_000_000))
                .saturating_add((element_index as u64).saturating_mul(4));
            match element {
                UiCanvasElement::Panel { color, .. } => {
                    draw_quad(
                        backend,
                        element_base,
                        0,
                        x,
                        y,
                        width,
                        height,
                        rgba8_to_float(*color),
                    )?;
                    stats.ui_quads += 1;
                    stats.ui_canvas_quads += 1;
                }
                UiCanvasElement::Button { label, .. } => {
                    draw_quad(
                        backend,
                        element_base,
                        0,
                        x,
                        y,
                        width,
                        height,
                        [0.12, 0.16, 0.22, 0.96],
                    )?;
                    stats.ui_quads += 1;
                    stats.ui_canvas_quads += 1;
                    if !label.trim().is_empty() {
                        draw_canvas_text(
                            backend,
                            element_base.saturating_add(1),
                            label,
                            x,
                            y,
                            width,
                            height,
                            18.0,
                        )?;
                        stats.ui_text_areas += 1;
                        stats.ui_canvas_text_areas += 1;
                    }
                }
                UiCanvasElement::Label {
                    text, font_size, ..
                } => {
                    if !text.trim().is_empty() {
                        draw_canvas_text(
                            backend,
                            element_base,
                            text,
                            x,
                            y,
                            width,
                            height,
                            sanitize_canvas_font_size(*font_size),
                        )?;
                        stats.ui_text_areas += 1;
                        stats.ui_canvas_text_areas += 1;
                    }
                }
                UiCanvasElement::Image { sprite_path, .. } => {
                    let texture_id = textures
                        .get(sprite_path)
                        .map_or(0, |texture| texture.texture_id);
                    draw_quad(
                        backend,
                        element_base,
                        texture_id,
                        x,
                        y,
                        width,
                        height,
                        [1.0; 4],
                    )?;
                    stats.ui_quads += 1;
                    stats.ui_canvas_quads += 1;
                    stats.textured_ui_images += usize::from(texture_id != 0);
                }
            }
        }
    }
    Ok(())
}

fn finite_ui_canvas_rect(
    canvas: &crate::engine::ui_canvas::UiCanvasRoot,
    element: &UiCanvasElement,
    screen_width: f32,
    screen_height: f32,
) -> Option<(f32, f32, f32, f32)> {
    let rect = layout_element_pixels(
        canvas,
        element.rect(),
        screen_width.max(1.0),
        screen_height.max(1.0),
    );
    (rect.0.is_finite()
        && rect.1.is_finite()
        && rect.2.is_finite()
        && rect.3.is_finite()
        && rect.2 > 0.0
        && rect.3 > 0.0)
        .then_some(rect)
}

#[allow(clippy::too_many_arguments)]
fn draw_canvas_text<B: RenderBackend>(
    backend: &mut B,
    text_id: u64,
    text: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    font_size: f32,
) -> MFResult<()> {
    let line_height = font_size * 1.25;
    backend.draw_text(TextDrawCommand {
        text_id,
        text: text.to_string(),
        font_family: String::new(),
        x: x + 6.0,
        y: y + ((height - line_height) * 0.5).max(2.0),
        width: (width - 12.0).max(1.0),
        height: (height - 4.0).max(1.0),
        font_size,
        line_height,
        color: [235, 240, 248, 255],
        wrap: TextWrapMode::Word,
        clip_rect: None,
    })
}

fn sanitize_canvas_font_size(font_size: f32) -> f32 {
    if font_size.is_finite() && font_size > 0.0 {
        font_size.clamp(1.0, 512.0)
    } else {
        18.0
    }
}

fn rgba8_to_float(color: [u8; 4]) -> [f32; 4] {
    color.map(|channel| f32::from(channel) / 255.0)
}

fn resolved_ui_text_metrics(ui: &Component) -> (f32, f32) {
    let authored_font_size = ui.get_f64("font_size", 0.0);
    let font_size = if authored_font_size.is_finite() && authored_font_size > 0.0 {
        authored_font_size.clamp(1.0, 512.0) as f32
    } else {
        16.0
    };
    let authored_line_height = ui.get_f64("line_height", 0.0);
    let line_height = if authored_line_height.is_finite() && authored_line_height > 0.0 {
        authored_line_height.clamp(f64::from(font_size), 1024.0) as f32
    } else {
        font_size * 1.25
    };
    (font_size, line_height)
}

fn normalized_ui_slider(ui: &Component) -> f32 {
    let min = ui.get_f64("min", 0.0);
    let min = if min.is_finite() { min } else { 0.0 };
    let authored_max = ui.get_f64("max", 1.0);
    let max = if authored_max.is_finite() && authored_max > min {
        authored_max
    } else {
        min + 1.0
    };
    let authored_value = ui.get_f64("value", min);
    let value = if authored_value.is_finite() {
        authored_value
    } else {
        min
    };
    ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
}

fn component_color(
    value: Option<&serde_json::Value>,
    fallback: [u8; 4],
    alpha_scale: f32,
) -> [f32; 4] {
    let channels = value.and_then(serde_json::Value::as_array);
    let channel = |index: usize| {
        channels
            .and_then(|values| values.get(index))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value.min(255) as u8)
            .unwrap_or(fallback[index]) as f32
            / 255.0
    };
    [channel(0), channel(1), channel(2), channel(3) * alpha_scale]
}

#[allow(clippy::too_many_arguments)]
fn draw_quad<B: RenderBackend>(
    backend: &mut B,
    entity_id: u64,
    texture_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
) -> MFResult<()> {
    draw_quad_with_options(
        backend,
        entity_id,
        texture_id,
        x,
        y,
        width,
        height,
        color,
        SpriteDrawOptions::default(),
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_quad_with_options<B: RenderBackend>(
    backend: &mut B,
    entity_id: u64,
    texture_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    options: SpriteDrawOptions,
) -> MFResult<()> {
    if width <= 0.0 || height <= 0.0 {
        return Ok(());
    }
    backend.draw_sprite_with_options(
        SpriteDrawCommand {
            entity_id,
            texture_id,
            x,
            y,
            width,
            height,
            rotation: 0.0,
            color,
        },
        options,
    )
}

fn visible_tile_bounds(
    origin_x: f32,
    origin_y: f32,
    tile: f32,
    columns: usize,
    rows: usize,
    width: f32,
    height: f32,
) -> (usize, usize, usize, usize) {
    if columns == 0 || rows == 0 {
        return (0, 0, 0, 0);
    }
    let min_x = ((-origin_x / tile).floor() as isize - 1).clamp(0, columns as isize) as usize;
    let max_x =
        (((width - origin_x) / tile).ceil() as isize + 1).clamp(0, columns as isize) as usize;
    let min_y = ((-origin_y / tile).floor() as isize - 1).clamp(0, rows as isize) as usize;
    let max_y = (((height - origin_y) / tile).ceil() as isize + 1).clamp(0, rows as isize) as usize;
    (min_x, max_x, min_y, max_y)
}

fn rect_intersects_screen(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    screen_width: f32,
    screen_height: f32,
) -> bool {
    x + width >= 0.0 && y + height >= 0.0 && x <= screen_width && y <= screen_height
}

fn entity_sorting_order(entity: &GameObject) -> i64 {
    entity
        .get_component("SpriteRenderer")
        .map(|sprite| sprite.get_i64("sorting_order", 0))
        .unwrap_or(0)
}

fn entity_blend_mode(entity: &GameObject) -> SpriteBlendMode {
    ["Material2D", "SpriteRenderer"]
        .into_iter()
        .filter_map(|component_type| entity.get_component(component_type))
        .find_map(|component| {
            component
                .get("blend_mode")
                .and_then(serde_json::Value::as_str)
                .and_then(SpriteBlendMode::from_name)
        })
        .unwrap_or_default()
}

fn entity_sprite_options(entity: &GameObject) -> SpriteDrawOptions {
    let mut options = SpriteDrawOptions {
        blend_mode: entity_blend_mode(entity),
        ..SpriteDrawOptions::default()
    };
    for component_type in ["Material2D", "SpriteRenderer"] {
        let Some(component) = entity.get_component(component_type) else {
            continue;
        };
        let effect = ["material_effect", "shader"].into_iter().find_map(|key| {
            component
                .get(key)
                .and_then(serde_json::Value::as_str)
                .and_then(SpriteMaterialEffect::from_name)
        });
        if let Some(effect) = effect {
            options.material_effect = effect;
            let strength = component.get_f64("effect_strength", 1.0).clamp(0.0, 1.0);
            options.effect_strength = (strength * 255.0).round() as u8;
            break;
        }
    }
    options
}

fn entity_tint(entity: &GameObject) -> [f32; 4] {
    let fallback = if entity.tag == "Player" {
        [0.28, 0.76, 1.0, 1.0]
    } else if entity.tag == "Enemy" {
        [1.0, 0.4, 0.46, 1.0]
    } else {
        [0.52, 0.91, 0.68, 1.0]
    };
    let Some(tint) = entity
        .get_component("SpriteRenderer")
        .and_then(|sprite| sprite.get("tint"))
        .and_then(serde_json::Value::as_array)
    else {
        return fallback;
    };
    if tint.len() < 3 {
        return fallback;
    }
    [
        normalized_color_channel(tint[0].as_f64()),
        normalized_color_channel(tint[1].as_f64()),
        normalized_color_channel(tint[2].as_f64()),
        normalized_color_channel(tint.get(3).and_then(serde_json::Value::as_f64)),
    ]
}

fn normalized_color_channel(channel: Option<f64>) -> f32 {
    (channel.unwrap_or(255.0) / 255.0).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::component::default_component;
    use crate::engine::ui_canvas::{UiAnchor, UiCanvasRoot, UiRect};
    use crate::entities::game_object::GameObject;
    use crate::render::backend::MacroquadBackend;
    use serde_json::json;

    #[test]
    fn material_and_sprite_components_select_reusable_blend_modes() {
        let mut entity = GameObject::new(0.0, 0.0, Some("Effect".to_string()));
        entity
            .get_component_mut("SpriteRenderer")
            .unwrap()
            .set("blend_mode", json!("screen"));
        assert_eq!(entity_blend_mode(&entity), SpriteBlendMode::Screen);

        let mut material = default_component("Material2D").unwrap();
        material.set("blend_mode", json!("multiply"));
        material.set("shader", json!("sprite_sepia"));
        material.set("effect_strength", json!(0.5));
        entity.add_component(material);
        assert_eq!(entity_blend_mode(&entity), SpriteBlendMode::Multiply);
        let options = entity_sprite_options(&entity);
        assert_eq!(options.material_effect, SpriteMaterialEffect::Sepia);
        assert_eq!(options.effect_strength, 128);
    }

    #[test]
    fn runtime_scene_extraction_is_backend_agnostic_and_deterministic() {
        let mut grid = Grid::new(4, 3, 16, 2);
        grid.set_tile(2, 1, 1);
        let mut player = GameObject::new(1.5, 1.0, Some("Player".to_string()));
        player.tag = "Player".to_string();
        let world = RuntimeWorld::new(vec![player]);
        let mut backend = MacroquadBackend::default();
        backend.init().unwrap();
        backend.begin_frame().unwrap();
        let stats =
            draw_runtime_scene_2d(&mut backend, &world, &grid, &BTreeMap::new(), 320.0, 180.0)
                .unwrap();
        backend.end_frame().unwrap();

        assert_eq!(stats.tile_quads, 12);
        assert_eq!(stats.entity_quads, 1);
        assert_eq!(stats.textured_entities, 0);
        assert_eq!(backend.draw_calls, 13);
    }

    #[test]
    fn full_runtime_pass_expands_layers_particles_atlas_and_ui_to_quads() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("miniforge-wgpu-scene-{unique}"));
        std::fs::create_dir_all(root.join("assets/scenes")).unwrap();
        std::fs::create_dir_all(root.join("settings")).unwrap();
        std::fs::write(
            root.join("project.mforge"),
            r#"{"name":"Render Test","start_scene":"assets/scenes/main.scene.json"}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("assets/scenes/main.scene.json"),
            r#"{"entities":[],"grid":{"width":4,"height":3,"tile_size":16,"chunk_size":2}}"#,
        )
        .unwrap();
        let mut runtime = EngineRuntime::new(&root).unwrap();
        runtime.tilemap_layers.layers[0].set(1, 1, 2);
        runtime.tilemap_layers.layers[1].set(2, 1, 5);

        let mut actor = GameObject::new(1.5, 1.0, Some("Actor".to_string()));
        actor.tag = "Player".to_string();
        actor
            .get_component_mut("SpriteRenderer")
            .unwrap()
            .set("texture_path", json!("assets/actor.png"));
        actor.get_component_mut("SpriteRenderer").unwrap().set(
            "_source_rect",
            json!({"x": 8, "y": 0, "width": 8, "height": 8}),
        );
        actor.add_component(default_component("Light2D").unwrap());
        let mut emitter = GameObject::new(2.0, 1.0, Some("Emitter".to_string()));
        emitter.add_component(default_component("ParticleEmitter").unwrap());
        let mut police = GameObject::new(3.0, 1.0, Some("Police".to_string()));
        police.tag = "Police".to_string();
        let mut ui = GameObject::new(0.0, 0.0, Some("Health".to_string()));
        let mut ui_component = default_component("UIElement").unwrap();
        ui_component.set("element_type", json!("ProgressBar"));
        ui_component.set("text", json!("Health"));
        ui.add_component(ui_component);
        let mut slider = GameObject::new(0.0, 0.0, Some("Volume".to_string()));
        let mut slider_component = default_component("UIElement").unwrap();
        slider_component.set("element_type", json!("Slider"));
        slider_component.set("text", json!(""));
        slider_component.set("min", json!(0.0));
        slider_component.set("max", json!(100.0));
        slider_component.set("value", json!(50.0));
        slider.add_component(slider_component);
        let mut checkbox = GameObject::new(0.0, 0.0, Some("Warm".to_string()));
        let mut checkbox_component = default_component("UIElement").unwrap();
        checkbox_component.set("element_type", json!("Checkbox"));
        checkbox_component.set("checked", json!(true));
        checkbox_component.set("text", json!("Warm"));
        checkbox.add_component(checkbox_component);
        let mut inventory = GameObject::new(0.0, 0.0, Some("Inventory".to_string()));
        let mut inventory_component = default_component("UIElement").unwrap();
        inventory_component.set("element_type", json!("InventoryGrid"));
        inventory_component.set("text", json!(""));
        inventory_component.set("width", json!(100.0));
        inventory_component.set("height", json!(64.0));
        inventory_component.set("columns", json!(3));
        inventory_component.set("slot_count", json!(6));
        inventory_component.set("slot_size", json!(24.0));
        inventory_component.set("items", json!([{"id": "water"}, null]));
        inventory.add_component(inventory_component);
        let mut nine_slice = GameObject::new(0.0, 0.0, Some("Frame".to_string()));
        let mut nine_slice_component = default_component("UIElement").unwrap();
        nine_slice_component.set("element_type", json!("NineSlice"));
        nine_slice_component.set("text", json!(""));
        nine_slice_component.set("image_name", json!("assets/ui/frame.png"));
        nine_slice_component.set("width", json!(96.0));
        nine_slice_component.set("height", json!(64.0));
        nine_slice_component.set("slice_left", json!(6.0));
        nine_slice_component.set("slice_right", json!(6.0));
        nine_slice_component.set("slice_top", json!(6.0));
        nine_slice_component.set("slice_bottom", json!(6.0));
        nine_slice.add_component(nine_slice_component);
        let mut minimap = GameObject::new(0.0, 0.0, Some("Minimap".to_string()));
        let mut minimap_component = default_component("UIElement").unwrap();
        minimap_component.set("element_type", json!("Minimap"));
        minimap_component.set("text", json!(""));
        minimap_component.set("width", json!(100.0));
        minimap_component.set("height", json!(100.0));
        minimap_component.set("world_radius", json!(20.0));
        minimap.add_component(minimap_component);
        runtime.runtime_world.replace_entities(vec![
            actor, emitter, police, ui, slider, checkbox, inventory, nine_slice, minimap,
        ]);
        runtime
            .particle_system
            .update_previews(&runtime.runtime_world.units, 0.0);

        let textures = BTreeMap::from([
            (
                "assets/actor.png".to_string(),
                RuntimeTexture2D {
                    texture_id: 7,
                    width: 16,
                    height: 8,
                },
            ),
            (
                "assets/ui/frame.png".to_string(),
                RuntimeTexture2D {
                    texture_id: 8,
                    width: 24,
                    height: 24,
                },
            ),
        ]);
        let mut backend = MacroquadBackend::default();
        backend.init().unwrap();
        backend.begin_frame().unwrap();
        let stats =
            draw_engine_runtime_scene_2d(&mut backend, &runtime, &textures, 320.0, 180.0).unwrap();
        backend.end_frame().unwrap();

        assert_eq!(stats.tile_layer_quads, 2);
        assert_eq!(stats.entity_quads, 3);
        assert_eq!(stats.textured_entities, 1);
        assert!(stats.particle_quads >= 8);
        assert_eq!(stats.light_quads, 1);
        assert_eq!(stats.ui_quads, 50);
        assert_eq!(stats.ui_text_areas, 2);
        assert_eq!(stats.textured_ui_images, 1);
        assert_eq!(stats.minimap_quads, 3);
        assert_eq!(
            backend.draw_calls,
            stats.tile_quads
                + stats.entity_quads
                + stats.particle_quads
                + stats.light_quads
                + stats.ui_quads
                + stats.ui_text_areas
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn ui_text_zero_metrics_resolve_to_legible_defaults() {
        let mut ui = default_component("UIElement").unwrap();
        ui.set("font_size", json!(0.0));
        ui.set("line_height", json!(0.0));
        assert_eq!(resolved_ui_text_metrics(&ui), (16.0, 20.0));

        ui.set("font_size", json!(24.0));
        ui.set("line_height", json!(12.0));
        assert_eq!(resolved_ui_text_metrics(&ui), (24.0, 24.0));
    }

    #[test]
    fn slider_normalization_repairs_invalid_ranges_and_values() {
        let mut ui = default_component("UIElement").unwrap();
        ui.set("min", json!(10.0));
        ui.set("max", json!(5.0));
        ui.set("value", json!(10.5));
        assert_eq!(normalized_ui_slider(&ui), 0.5);

        ui.set("value", json!(f64::NAN));
        assert_eq!(normalized_ui_slider(&ui), 0.0);
    }

    #[test]
    fn responsive_scene_canvas_draws_panels_buttons_labels_and_images() {
        let rect = |x: f32, y: f32, width: f32, height: f32| UiRect {
            anchor: UiAnchor {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
            },
            pivot_x: 0.0,
            pivot_y: 0.0,
            offset_x: x,
            offset_y: y,
            width,
            height,
        };
        let canvas = UiCanvasRoot {
            id: "hud".to_string(),
            name: "HUD".to_string(),
            reference_width: 320.0,
            reference_height: 180.0,
            elements: vec![
                UiCanvasElement::Panel {
                    id: "panel".to_string(),
                    name: "Panel".to_string(),
                    rect: rect(8.0, 8.0, 120.0, 60.0),
                    color: [24, 28, 36, 220],
                },
                UiCanvasElement::Button {
                    id: "button".to_string(),
                    label: "Continue".to_string(),
                    rect: rect(12.0, 16.0, 96.0, 32.0),
                },
                UiCanvasElement::Label {
                    id: "label".to_string(),
                    text: "Unicode: agua • frío".to_string(),
                    rect: rect(12.0, 72.0, 180.0, 28.0),
                    font_size: 18.0,
                },
                UiCanvasElement::Image {
                    id: "portrait".to_string(),
                    sprite_path: "assets/ui/portrait.png".to_string(),
                    rect: rect(240.0, 8.0, 64.0, 64.0),
                },
            ],
        };
        let canvases = json!([canvas]);
        let textures = BTreeMap::from([(
            "assets/ui/portrait.png".to_string(),
            RuntimeTexture2D {
                texture_id: 41,
                width: 32,
                height: 32,
            },
        )]);
        let mut backend = MacroquadBackend::default();
        backend.init().unwrap();
        backend.begin_frame().unwrap();
        let mut stats = RuntimeScene2DStats::default();

        draw_scene_ui_canvases(&mut backend, &canvases, &textures, 640.0, 360.0, &mut stats)
            .unwrap();
        backend.end_frame().unwrap();

        assert_eq!(stats.ui_canvas_quads, 3);
        assert_eq!(stats.ui_canvas_text_areas, 2);
        assert_eq!(stats.textured_ui_images, 1);
        assert_eq!(stats.ui_quads, 3);
        assert_eq!(stats.ui_text_areas, 2);
        assert_eq!(backend.draw_calls, 5);
        assert_eq!(
            scene_ui_sprite_paths(&canvases),
            vec!["assets/ui/portrait.png".to_string()]
        );
    }
}
