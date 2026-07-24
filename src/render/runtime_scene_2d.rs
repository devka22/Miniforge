use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::engine::error_handler::MFResult;
use crate::engine::tilemap_layers::{TileLayer, TilemapLayers};
use crate::engine::world::RuntimeWorld;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;
use crate::runtime::engine_runtime::EngineRuntime;
use crate::systems::particle_system::ParticleSystem;

use super::backend::{RenderBackend, SpriteDrawCommand, SpriteRegionDrawCommand};

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
    pub ui_quads: usize,
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
    draw_runtime_ui(backend, &runtime.runtime_world, width, height, &mut stats)?;
    Ok(stats)
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
        if let Some((binding, uv_rect)) = binding
            .and_then(|binding| entity_source_uv(entity, binding).map(|uv_rect| (binding, uv_rect)))
        {
            backend.draw_sprite_region(SpriteRegionDrawCommand {
                sprite: SpriteDrawCommand {
                    texture_id: binding.texture_id,
                    ..sprite
                },
                uv_rect,
                clip_rect: None,
            })?;
        } else {
            backend.draw_sprite(sprite)?;
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
            draw_quad(
                backend,
                particle_id,
                0,
                x,
                y,
                size,
                size,
                particle.color.map(|channel| channel as f32 / 255.0),
            )?;
            stats.particle_quads += 1;
        }
    }
    Ok(())
}

fn draw_runtime_ui<B: RenderBackend>(
    backend: &mut B,
    world: &RuntimeWorld,
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
        let ui_base = 9_000_000_000u64.saturating_add(entity.id.saturating_mul(8));
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
        let background = component_color(ui.get("color"), [24, 28, 36, 255], opacity);
        draw_quad(backend, ui_base, 0, x, y, width, height, background)?;
        stats.ui_quads += 1;

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

        let kind = ui.get_string("element_type", "Label");
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
    }
    Ok(())
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
    if width <= 0.0 || height <= 0.0 {
        return Ok(());
    }
    backend.draw_sprite(SpriteDrawCommand {
        entity_id,
        texture_id,
        x,
        y,
        width,
        height,
        rotation: 0.0,
        color,
    })
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
    use crate::entities::game_object::GameObject;
    use crate::render::backend::MacroquadBackend;
    use serde_json::json;

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
        runtime.tilemap_layers.layers[3].set(2, 1, 5);

        let mut actor = GameObject::new(1.5, 1.0, Some("Actor".to_string()));
        actor
            .get_component_mut("SpriteRenderer")
            .unwrap()
            .set("texture_path", json!("assets/actor.png"));
        actor.get_component_mut("SpriteRenderer").unwrap().set(
            "_source_rect",
            json!({"x": 8, "y": 0, "width": 8, "height": 8}),
        );
        let mut emitter = GameObject::new(2.0, 1.0, Some("Emitter".to_string()));
        emitter.add_component(default_component("ParticleEmitter").unwrap());
        let mut ui = GameObject::new(0.0, 0.0, Some("Health".to_string()));
        let mut ui_component = default_component("UIElement").unwrap();
        ui_component.set("element_type", json!("ProgressBar"));
        ui.add_component(ui_component);
        runtime
            .runtime_world
            .replace_entities(vec![actor, emitter, ui]);
        runtime
            .particle_system
            .update_previews(&runtime.runtime_world.units, 0.0);

        let textures = BTreeMap::from([(
            "assets/actor.png".to_string(),
            RuntimeTexture2D {
                texture_id: 7,
                width: 16,
                height: 8,
            },
        )]);
        let mut backend = MacroquadBackend::default();
        backend.init().unwrap();
        backend.begin_frame().unwrap();
        let stats =
            draw_engine_runtime_scene_2d(&mut backend, &runtime, &textures, 320.0, 180.0).unwrap();
        backend.end_frame().unwrap();

        assert_eq!(stats.tile_layer_quads, 2);
        assert_eq!(stats.entity_quads, 2);
        assert_eq!(stats.textured_entities, 1);
        assert!(stats.particle_quads >= 8);
        assert_eq!(stats.ui_quads, 6);
        assert_eq!(
            backend.draw_calls,
            stats.tile_quads + stats.entity_quads + stats.particle_quads + stats.ui_quads
        );
        std::fs::remove_dir_all(root).ok();
    }
}
