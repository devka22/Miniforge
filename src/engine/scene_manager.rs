use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::camera::Camera;
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
use crate::engine::scene_serializer::SceneSerializer;
use crate::engine::tilemap_layers::TilemapLayers;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;

#[derive(Debug, Clone)]
pub struct SceneManager {
    pub project_path: PathBuf,
    pub current_scene: String,
    pub loaded_scenes: Vec<String>,
    pub scene_stack: Vec<String>,
    pub transition: Option<SceneTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneTransition {
    pub from_scene: String,
    pub to_scene: String,
    pub kind: String,
    pub duration: f64,
    pub elapsed: f64,
    pub complete: bool,
}

impl SceneManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self::new_with_start_scene(project_path, "main.scene")
    }

    pub fn new_with_start_scene(project_path: impl AsRef<Path>, start_scene: &str) -> Self {
        let current_scene = normalize_scene_name(start_scene);
        Self {
            project_path: project_path.as_ref().to_path_buf(),
            loaded_scenes: vec![current_scene.clone()],
            scene_stack: vec![current_scene.clone()],
            current_scene,
            transition: None,
        }
    }

    pub fn scene_path(&self) -> PathBuf {
        self.scene_path_for(&self.current_scene)
    }

    pub fn scene_path_for(&self, name: &str) -> PathBuf {
        let scene_name = normalize_scene_name(name);
        AssetTools::get_project_paths(&self.project_path)
            .scenes
            .join(scene_name)
    }

    pub fn create_new_scene(&mut self, name: &str) -> io::Result<PathBuf> {
        let path = AssetTools::create_scene(&self.project_path, name)?;
        self.current_scene = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("main.scene")
            .to_string();
        self.set_single_loaded_scene(self.current_scene.clone());
        Ok(path)
    }

    pub fn open_scene(&mut self, name: &str) -> io::Result<bool> {
        let scene_name = normalize_scene_name(name);
        let path = self.scene_path_for(&scene_name);
        if !path.exists() {
            return Ok(false);
        }
        self.set_single_loaded_scene(scene_name);
        Ok(true)
    }

    pub fn duplicate_current_scene(&mut self, new_name: &str) -> io::Result<PathBuf> {
        let source = self.scene_path();
        let mut filename = AssetTools::safe_name(new_name, "SceneCopy");
        if !filename.ends_with(".scene") {
            filename.push_str(".scene");
        }
        let scenes = AssetTools::get_project_paths(&self.project_path).scenes;
        let target = AssetTools::unique_path(scenes, &filename);
        if source.exists() {
            fs::copy(source, &target)?;
        } else {
            AssetTools::create_json_file(
                &target,
                &AssetTools::template_scene(
                    target
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("SceneCopy"),
                ),
                true,
            )?;
        }
        self.current_scene = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("main.scene")
            .to_string();
        self.set_single_loaded_scene(self.current_scene.clone());
        Ok(target)
    }

    pub fn scene_metadata(&self) -> io::Result<Value> {
        let path = self.scene_path();
        let entity_count = self
            .load_current_scene_data()?
            .get("entities")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        Ok(json!({
            "current_scene": self.current_scene,
            "path": path,
            "exists": path.exists(),
            "entity_count": entity_count,
        }))
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
        let data = SceneSerializer::stamp(json!({
            "version": crate::engine::version::ENGINE_VERSION,
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
            "ui_canvases": json!([]),
        }))
        .map_err(io::Error::from)?;
        let path = self.scene_path();
        let backup = path.with_extension("scene.bak");
        ProjectStorage::write_json_atomic_with_backup(
            &path,
            &data,
            BackupPolicy::new(backup, DEFAULT_BACKUP_GENERATIONS),
        )
        .map(|_| ())
        .map_err(io::Error::from)
    }

    pub fn load_current_scene_data(&self) -> io::Result<Value> {
        self.load_scene_data(&self.current_scene)
    }

    pub fn load_scene_data(&self, name: &str) -> io::Result<Value> {
        let path = self.scene_path_for(name);
        if !path.exists() {
            return Ok(json!({}));
        }
        load_scene_document_with_backup(&path)
    }

    pub fn load_current_scene(&self) -> io::Result<Vec<GameObject>> {
        self.load_scene_entities(&self.current_scene)
    }

    pub fn load_scene_entities(&self, name: &str) -> io::Result<Vec<GameObject>> {
        let scene_name = normalize_scene_name(name);
        let data = self.load_scene_data(&scene_name)?;
        let entities = data
            .get("entities")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let mut entity = GameObject::from_data(item, true);
                        entity.scene_name = Some(scene_name.clone());
                        entity
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(entities)
    }

    pub fn load_scene(
        &mut self,
        name: &str,
        current_entities: &[GameObject],
    ) -> io::Result<Vec<GameObject>> {
        let scene_name = normalize_scene_name(name);
        let mut next_entities = preserve_dont_destroy_entities(current_entities);
        next_entities.extend(self.load_scene_entities(&scene_name)?);
        self.set_single_loaded_scene(scene_name);
        Ok(next_entities)
    }

    pub fn load_scene_additive(
        &mut self,
        name: &str,
        entities: &mut Vec<GameObject>,
    ) -> io::Result<usize> {
        let scene_name = normalize_scene_name(name);
        let mut loaded = self.load_scene_entities(&scene_name)?;
        let before = entities.len();
        loaded.retain(|entity| !entities.iter().any(|existing| existing.id == entity.id));
        entities.extend(loaded);
        if !self.loaded_scenes.iter().any(|scene| scene == &scene_name) {
            self.loaded_scenes.push(scene_name.clone());
        }
        self.current_scene = scene_name;
        Ok(entities.len() - before)
    }

    pub fn unload_scene(&mut self, name: &str, entities: &mut Vec<GameObject>) -> usize {
        let scene_name = normalize_scene_name(name);
        let before = entities.len();
        entities.retain(|entity| {
            entity.scene_name.as_deref() != Some(scene_name.as_str())
                || entity_survives_scene_load(entity)
        });
        self.loaded_scenes.retain(|scene| scene != &scene_name);
        if self.current_scene == scene_name {
            self.current_scene = self
                .loaded_scenes
                .last()
                .cloned()
                .unwrap_or_else(|| "main.scene".to_string());
        }
        before - entities.len()
    }

    pub fn restart_scene(
        &mut self,
        current_entities: &[GameObject],
    ) -> io::Result<Vec<GameObject>> {
        let current = self.current_scene.clone();
        self.load_scene(&current, current_entities)
    }

    pub fn push_scene(
        &mut self,
        name: &str,
        current_entities: &[GameObject],
    ) -> io::Result<Vec<GameObject>> {
        let previous_stack = if self.scene_stack.is_empty() {
            vec![self.current_scene.clone()]
        } else {
            self.scene_stack.clone()
        };
        let scene_name = normalize_scene_name(name);
        let entities = self.load_scene(&scene_name, current_entities)?;
        self.scene_stack = previous_stack;
        if self.scene_stack.last() != Some(&scene_name) {
            self.scene_stack.push(scene_name);
        }
        Ok(entities)
    }

    pub fn pop_scene(
        &mut self,
        current_entities: &[GameObject],
    ) -> io::Result<Option<Vec<GameObject>>> {
        if self.scene_stack.len() <= 1 {
            return Ok(None);
        }
        let mut next_stack = self.scene_stack.clone();
        next_stack.pop();
        let Some(target) = next_stack.last().cloned() else {
            return Ok(None);
        };
        let entities = self.load_scene(&target, current_entities)?;
        self.scene_stack = next_stack;
        Ok(Some(entities))
    }

    pub fn transition_to_scene(
        &mut self,
        name: &str,
        kind: &str,
        duration: f64,
        current_entities: &[GameObject],
    ) -> io::Result<Vec<GameObject>> {
        let target = normalize_scene_name(name);
        self.transition = Some(SceneTransition {
            from_scene: self.current_scene.clone(),
            to_scene: target.clone(),
            kind: kind.to_string(),
            duration: duration.max(0.0),
            elapsed: 0.0,
            complete: duration <= 0.0,
        });
        self.load_scene(&target, current_entities)
    }

    pub fn update_transition(&mut self, dt: f64) -> Option<SceneTransition> {
        let transition = self.transition.as_mut()?;
        transition.elapsed = (transition.elapsed + dt.max(0.0)).min(transition.duration);
        transition.complete =
            transition.duration <= 0.0 || transition.elapsed >= transition.duration;
        if transition.complete {
            return self.transition.take();
        }
        Some(transition.clone())
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
        self.set_single_loaded_scene(self.current_scene.clone());
        Ok(Some(self.current_scene.clone()))
    }

    fn set_single_loaded_scene(&mut self, scene_name: String) {
        self.current_scene = scene_name.clone();
        self.loaded_scenes = vec![scene_name.clone()];
        self.scene_stack = vec![scene_name];
    }
}

fn load_scene_document_with_backup(path: &Path) -> io::Result<Value> {
    let primary_error = match AssetTools::read_json(path) {
        Ok(data) => match SceneSerializer::try_migrate(data) {
            Ok(report) => return Ok(report.data),
            Err(error) if error.is_future_version() => return Err(io::Error::from(error)),
            Err(error) => error.to_string(),
        },
        Err(error) => error.to_string(),
    };
    let backup = path.with_extension("scene.bak");
    if !backup.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Escena invalida: {} | {primary_error}", path.display()),
        ));
    }
    let backup_data = AssetTools::read_json(&backup).map_err(|backup_error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Escena invalida y backup ilegible: {} | {primary_error}; backup: {backup_error}",
                path.display()
            ),
        )
    })?;
    SceneSerializer::try_migrate(backup_data)
        .map(|report| report.data)
        .map_err(|backup_error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Escena invalida y backup incompatible: {} | {primary_error}; backup: {backup_error}",
                    path.display()
                ),
            )
        })
}

pub fn normalize_scene_name(name: &str) -> String {
    let mut scene_name = AssetTools::safe_name(name, "main");
    if !scene_name.ends_with(".scene") {
        scene_name.push_str(".scene");
    }
    scene_name
}

pub fn entity_survives_scene_load(entity: &GameObject) -> bool {
    entity
        .get_component("DontDestroyOnLoad")
        .is_some_and(|component| component.enabled && component.get_bool("preserve", true))
}

fn preserve_dont_destroy_entities(entities: &[GameObject]) -> Vec<GameObject> {
    entities
        .iter()
        .filter(|entity| entity_survives_scene_load(entity))
        .cloned()
        .collect()
}
