use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::engine::error_handler::MFResult;
use crate::engine::world::RuntimeWorld;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;

use super::backend::{RenderBackend, SpriteDrawCommand};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeScene2DStats {
    pub tile_quads: usize,
    pub entity_quads: usize,
    pub textured_entities: usize,
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
    use crate::entities::game_object::GameObject;
    use crate::render::backend::MacroquadBackend;

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
}
