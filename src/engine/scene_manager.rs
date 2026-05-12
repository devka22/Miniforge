use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::camera::Camera;
use crate::engine::scene_serializer::SceneSerializer;
use crate::engine::tilemap_layers::TilemapLayers;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;

#[derive(Debug, Clone)]
pub struct SceneManager {
    pub project_path: PathBuf,
    pub current_scene: String,
}

impl SceneManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            project_path: project_path.as_ref().to_path_buf(),
            current_scene: "main.scene".to_string(),
        }
    }

    pub fn scene_path(&self) -> PathBuf {
        AssetTools::get_project_paths(&self.project_path)
            .scenes
            .join(&self.current_scene)
    }

    pub fn create_new_scene(&mut self, name: &str) -> io::Result<PathBuf> {
        let path = AssetTools::create_scene(&self.project_path, name)?;
        self.current_scene = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("main.scene")
            .to_string();
        Ok(path)
    }

    pub fn save_current_scene(&self, entities: &mut [GameObject]) -> io::Result<()> {
        let tilemap = TilemapLayers::new(0, 0);
        let camera = Camera::default();
        let grid = Grid::new(0, 0, 32, 0);
        self.save_current_scene_with_editor_state(
            entities, &tilemap, &camera, "EDITOR", "Select", 0, 1, &grid,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_current_scene_with_editor_state(
        &self,
        entities: &mut [GameObject],
        tilemap_layers: &TilemapLayers,
        camera: &Camera,
        mode: &str,
        active_tool: &str,
        tile_brush: i32,
        brush_size: usize,
        grid: &Grid,
    ) -> io::Result<()> {
        let data = json!({
            "version": "0.6.0",
            "engine_version": crate::engine::version::ENGINE_VERSION,
            "scene_name": self.current_scene.trim_end_matches(".scene"),
            "mode": mode,
            "active_tool": active_tool,
            "tile_brush": tile_brush,
            "brush_size": brush_size,
            "entities": entities.iter_mut().map(GameObject::serialize).collect::<Vec<_>>(),
            "tiles": tilemap_layers.serialize(),
            "tilemap_layers": tilemap_layers.serialize(),
            "camera": {"x": camera.x, "y": camera.y, "zoom": camera.zoom},
            "grid": {
                "width": grid.width,
                "height": grid.height,
                "tile_size": grid.tile_size,
                "chunk_size": grid.chunk_size,
            },
            "settings": {},
        });
        AssetTools::write_json(self.scene_path(), &data)
    }

    pub fn load_current_scene_data(&self) -> io::Result<Value> {
        let path = self.scene_path();
        if !path.exists() {
            return Ok(json!({}));
        }
        Ok(SceneSerializer::migrate(AssetTools::read_json(path)?))
    }

    pub fn load_current_scene(&self) -> io::Result<Vec<GameObject>> {
        let data = self.load_current_scene_data()?;
        let entities = data
            .get("entities")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| GameObject::from_data(item, true))
                    .collect()
            })
            .unwrap_or_default();
        Ok(entities)
    }

    pub fn list_scenes(&self) -> io::Result<Vec<String>> {
        let scenes = AssetTools::get_project_paths(&self.project_path).scenes;
        let mut names = Vec::new();
        if scenes.exists() {
            for entry in fs::read_dir(scenes)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) == Some("scene")
                    && let Some(name) = path.file_name().and_then(|value| value.to_str())
                {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn next_scene(&mut self) -> io::Result<Option<String>> {
        let scenes = self.list_scenes()?;
        if scenes.is_empty() {
            return Ok(None);
        }
        let index = scenes
            .iter()
            .position(|name| name == &self.current_scene)
            .unwrap_or(0);
        self.current_scene = scenes[(index + 1) % scenes.len()].clone();
        Ok(Some(self.current_scene.clone()))
    }
}
