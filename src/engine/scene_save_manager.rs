//! Incremental scene persistence: backup, atomic write, merge unchanged entity JSON blobs.

use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io;

use crate::engine::asset_tools::AssetTools;
use crate::engine::camera::Camera;
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
use crate::engine::scene_manager::SceneManager;
use crate::engine::scene_serializer::SceneSerializer;
use crate::engine::tilemap_layers::TilemapLayers;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;

#[derive(Debug, Clone, Default)]
pub struct SceneSaveManager {
    entity_hashes: HashMap<u64, u64>,
    last_tilemap_hash: u64,
    /// Explicit dirty entity ids (inspector edits, etc.)
    pub dirty_entities: HashSet<u64>,
    pub tilemap_dirty: bool,
}

impl SceneSaveManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bootstrap_from_scene(&mut self, entities: &mut [GameObject], tilemap: &TilemapLayers) {
        self.entity_hashes.clear();
        for e in entities.iter_mut() {
            self.entity_hashes.insert(e.id, stable_hash(&e.serialize()));
        }
        self.last_tilemap_hash = stable_hash(&tilemap.serialize());
        self.dirty_entities.clear();
        self.tilemap_dirty = false;
    }

    pub fn note_entity_dirty(&mut self, entity_id: u64) {
        self.dirty_entities.insert(entity_id);
    }

    pub fn note_tilemap_dirty(&mut self) {
        self.tilemap_dirty = true;
    }

    fn entity_changed(&mut self, entity: &mut GameObject) -> bool {
        let v = entity.serialize();
        let h = stable_hash(&v);
        if self.dirty_entities.contains(&entity.id) {
            return true;
        }
        self.entity_hashes.get(&entity.id).copied() != Some(h)
    }

    fn tilemap_changed(&self, tilemap: &TilemapLayers) -> bool {
        self.tilemap_dirty || stable_hash(&tilemap.serialize()) != self.last_tilemap_hash
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_scene(
        &mut self,
        scene_manager: &SceneManager,
        entities: &mut [GameObject],
        tilemap_layers: &TilemapLayers,
        camera: &Camera,
        mode: &str,
        active_tool: &str,
        tile_brush: i32,
        brush_size: usize,
        grid: &Grid,
        ui_canvases: &Value,
    ) -> io::Result<()> {
        let path = scene_manager.scene_path();
        let backup = path.with_extension("scene.bak");

        let old_data = if path.exists() {
            SceneSerializer::try_migrate(AssetTools::read_json(&path)?)
                .map_err(io::Error::from)?
                .data
        } else {
            json!({})
        };

        let old_entities: HashMap<u64, Value> = old_data
            .get("entities")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let id = v.get("id").and_then(Value::as_u64)?;
                        Some((id, v.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let use_merge = !old_entities.is_empty();
        let mut merged_entities: Vec<Value> = Vec::new();
        for entity in entities.iter_mut() {
            if entity.scene_name.is_none() {
                entity.scene_name = Some(scene_manager.current_scene.clone());
            }
            if use_merge
                && !self.entity_changed(entity)
                && let Some(prev) = old_entities.get(&entity.id)
            {
                merged_entities.push(prev.clone());
                continue;
            }
            merged_entities.push(entity.serialize());
        }

        let tiles = if self.tilemap_changed(tilemap_layers) {
            tilemap_layers.serialize()
        } else {
            old_data
                .get("tilemap_layers")
                .cloned()
                .or_else(|| old_data.get("tiles").cloned())
                .unwrap_or_else(|| tilemap_layers.serialize())
        };

        let data = SceneSerializer::stamp(json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "engine_version": crate::engine::version::ENGINE_VERSION,
            "scene_name": scene_manager.current_scene.trim_end_matches(".scene"),
            "mode": mode,
            "active_tool": active_tool,
            "tile_brush": tile_brush,
            "brush_size": brush_size,
            "entities": merged_entities,
            "tiles": tiles.clone(),
            "tilemap_layers": tiles,
            "camera": {"x": camera.x, "y": camera.y, "zoom": camera.zoom},
            "grid": {
                "width": grid.width,
                "height": grid.height,
                "tile_size": grid.tile_size,
                "chunk_size": grid.chunk_size,
            },
            "settings": old_data.get("settings").cloned().unwrap_or(json!({})),
            "ui_canvases": ui_canvases,
            "control_groups": old_data.get("control_groups").cloned().unwrap_or(json!({})),
            "editor_view_settings": old_data.get("editor_view_settings").cloned().unwrap_or(json!({})),
        }))
        .map_err(io::Error::from)?;

        ProjectStorage::write_json_atomic_with_backup(
            &path,
            &data,
            BackupPolicy::new(backup, DEFAULT_BACKUP_GENERATIONS),
        )
        .map_err(io::Error::from)?;
        self.bootstrap_from_scene(entities, tilemap_layers);
        Ok(())
    }
}

fn stable_hash(value: &Value) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    // Deterministic string for hashing
    let s = serde_json::to_string(value).unwrap_or_default();
    s.hash(&mut hasher);
    hasher.finish()
}
