use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::version::ENGINE_VERSION;

pub const DEFAULT_PROJECT_NAME: &str = "DefaultProject";

#[derive(Debug, Clone)]
pub struct ProjectPaths {
    pub project: PathBuf,
    pub assets: PathBuf,
    pub sprites: PathBuf,
    pub audio: PathBuf,
    pub data: PathBuf,
    pub prefabs: PathBuf,
    pub scripts: PathBuf,
    pub visual_graphs: PathBuf,
    pub components: PathBuf,
    pub systems: PathBuf,
    pub scenes: PathBuf,
    pub root_scenes: PathBuf,
    pub settings: PathBuf,
    pub logs: PathBuf,
    pub templates: PathBuf,
    pub plugins: PathBuf,
    pub builds: PathBuf,
}

impl ProjectPaths {
    pub fn as_map(&self) -> BTreeMap<String, PathBuf> {
        BTreeMap::from([
            ("project".to_string(), self.project.clone()),
            ("assets".to_string(), self.assets.clone()),
            ("sprites".to_string(), self.sprites.clone()),
            ("audio".to_string(), self.audio.clone()),
            ("data".to_string(), self.data.clone()),
            ("prefabs".to_string(), self.prefabs.clone()),
            ("scripts".to_string(), self.scripts.clone()),
            ("visual_graphs".to_string(), self.visual_graphs.clone()),
            ("components".to_string(), self.components.clone()),
            ("systems".to_string(), self.systems.clone()),
            ("scenes".to_string(), self.scenes.clone()),
            ("root_scenes".to_string(), self.root_scenes.clone()),
            ("settings".to_string(), self.settings.clone()),
            ("logs".to_string(), self.logs.clone()),
            ("templates".to_string(), self.templates.clone()),
            ("plugins".to_string(), self.plugins.clone()),
            ("builds".to_string(), self.builds.clone()),
        ])
    }
}

pub struct AssetTools;

impl AssetTools {
    pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().components().collect()
    }

    pub fn default_project_path() -> PathBuf {
        PathBuf::from("projects").join(DEFAULT_PROJECT_NAME)
    }

    pub fn ensure_folder(path: impl AsRef<Path>) -> io::Result<PathBuf> {
        fs::create_dir_all(path.as_ref())?;
        Ok(path.as_ref().to_path_buf())
    }

    pub fn get_project_paths(project_path: impl AsRef<Path>) -> ProjectPaths {
        let project = project_path.as_ref().to_path_buf();
        ProjectPaths {
            assets: project.join("assets"),
            sprites: project.join("assets").join("sprites"),
            audio: project.join("assets").join("audio"),
            data: project.join("assets").join("data"),
            prefabs: project.join("assets").join("prefabs"),
            scripts: project.join("scripts"),
            visual_graphs: project.join("scripts").join("visual_graphs"),
            components: project.join("components"),
            systems: project.join("systems"),
            scenes: project.join("saves").join("scenes"),
            root_scenes: project.join("scenes"),
            settings: project.join("settings"),
            logs: project.join("logs"),
            templates: project.join("templates"),
            plugins: project.join("plugins"),
            builds: project.join("builds"),
            project,
        }
    }

    pub fn ensure_project_folders(project_path: impl AsRef<Path>) -> io::Result<ProjectPaths> {
        let paths = Self::get_project_paths(project_path);
        for path in paths.as_map().values() {
            fs::create_dir_all(path)?;
        }
        Self::ensure_project_files(&paths.project)?;
        Ok(paths)
    }

    pub fn ensure_project_files(project_path: impl AsRef<Path>) -> io::Result<()> {
        let project_path = project_path.as_ref();
        let paths = Self::get_project_paths(project_path);
        let project_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(DEFAULT_PROJECT_NAME);

        Self::write_json_if_missing(
            project_path.join("project.json"),
            &json!({
                "project_name": project_name,
                "engine_version": ENGINE_VERSION,
                "start_scene": "main.scene",
                "author": "",
                "license": "GPL-3.0",
                "description": "MiniForge 0.7.0 Production Editor project",
            }),
        )?;
        Self::write_json_if_missing(
            project_path.join("engine_config.json"),
            &json!({
                "engine_name": "MiniForge",
                "engine_alt_name": "Mini Forte",
                "engine_version": ENGINE_VERSION,
                "project_name": project_name,
                "start_scene": "main.scene",
                "autosave": true,
                "autosave_interval_seconds": 60,
                "safe_mode": true,
                "logs": {"engine": "logs/engine.log", "error": "logs/error.log"},
            }),
        )?;
        Self::write_json_if_missing(
            project_path.join("manifest.json"),
            &json!({
                "engine_version": ENGINE_VERSION,
                "assets": {},
                "scenes": [],
                "scripts": [],
                "components": [],
                "systems": [],
            }),
        )?;
        Self::write_json_if_missing(
            paths.settings.join("runtime_config.json"),
            &json!({
                "game_name": project_name,
                "start_scene": "main.scene",
                "window_width": 1100,
                "window_height": 740,
                "fullscreen": false,
                "target_fps": 60,
                "debug": true,
            }),
        )?;
        Self::write_json_if_missing(
            paths.settings.join("build_settings.json"),
            &json!({
                "game_name": project_name,
                "start_scene": "main.scene",
                "target_fps": 60,
                "window_width": 1100,
                "window_height": 740,
                "fullscreen": false,
                "debug_mode": true,
                "export_folder": "builds",
            }),
        )?;
        Self::write_json_if_missing(
            paths.settings.join("tags.json"),
            &json!({"items": [
                "Untagged", "Player", "Enemy", "Resource", "Building", "Projectile", "Neutral"
            ]}),
        )?;
        Self::write_json_if_missing(
            paths.settings.join("layers.json"),
            &json!({"items": [
                "Default", "Ground", "Units", "Buildings", "UI", "Effects",
                "IgnoreSelection", "EditorOnly"
            ]}),
        )?;

        let readme = project_path.join("README.md");
        if !readme.exists() {
            fs::write(
                readme,
                format!(
                    "# {project_name}\n\nProyecto creado con MiniForge {ENGINE_VERSION}.\n\n## Carpetas\n\n- assets/sprites\n- assets/audio\n- assets/data\n- assets/prefabs\n- scripts/visual_graphs\n- components\n- systems\n- scenes\n- settings\n"
                ),
            )?;
        }
        Ok(())
    }

    pub fn write_json(path: impl AsRef<Path>, data: &Value) -> io::Result<()> {
        if let Some(folder) = path.as_ref().parent() {
            fs::create_dir_all(folder)?;
        }
        let bytes = serde_json::to_vec_pretty(data).map_err(io::Error::other)?;
        fs::write(path, bytes)
    }

    fn write_json_if_missing(path: impl AsRef<Path>, data: &Value) -> io::Result<()> {
        if !path.as_ref().exists() {
            Self::write_json(path, data)?;
        }
        Ok(())
    }

    pub fn read_json(path: impl AsRef<Path>) -> io::Result<Value> {
        let text = fs::read_to_string(path)?;
        serde_json::from_str(&text).map_err(io::Error::other)
    }

    pub fn safe_name(name: &str, fallback: &str) -> String {
        let mut name = name.trim().to_string();
        if name.is_empty() {
            name = fallback.to_string();
        }
        for invalid in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
            name = name.replace(invalid, "_");
        }
        name
    }

    pub fn unique_path(folder: impl AsRef<Path>, filename: &str) -> PathBuf {
        let folder = folder.as_ref();
        let path = folder.join(filename);
        if !path.exists() {
            return path;
        }
        let stem = Path::new(filename)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(filename);
        let ext = Path::new(filename)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!(".{value}"))
            .unwrap_or_default();
        for index in 1.. {
            let candidate = folder.join(format!("{stem}_{index}{ext}"));
            if !candidate.exists() {
                return candidate;
            }
        }
        unreachable!()
    }

    pub fn create_file(
        path: impl AsRef<Path>,
        content: &str,
        overwrite: bool,
    ) -> io::Result<PathBuf> {
        let mut path = path.as_ref().to_path_buf();
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        if path.exists() && !overwrite {
            let folder = path.parent().unwrap_or_else(|| Path::new("."));
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("NewFile");
            path = Self::unique_path(folder, filename);
        }
        let mut file = fs::File::create(&path)?;
        file.write_all(content.as_bytes())?;
        Ok(path)
    }

    pub fn create_json_file(
        path: impl AsRef<Path>,
        data: &Value,
        overwrite: bool,
    ) -> io::Result<PathBuf> {
        let mut path = path.as_ref().to_path_buf();
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        if path.exists() && !overwrite {
            let folder = path.parent().unwrap_or_else(|| Path::new("."));
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("NewData.json");
            path = Self::unique_path(folder, filename);
        }
        Self::write_json(&path, data)?;
        Ok(path)
    }

    pub fn template_visual_graph(graph_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeVisualGraph",
            "runtime": "rust_visual_graph",
            "name": graph_name,
            "variables": {},
            "nodes": [
                {"id": "start", "type": "EventStart", "next": "log"},
                {"id": "update", "type": "EventUpdate", "next": null},
                {"id": "log", "type": "Log", "message": format!("{graph_name} started"), "next": null}
            ],
        })
    }

    pub fn template_rhai_script(script_name: &str) -> String {
        format!(
            r#"// MiniForge Rhai script: {script_name}
// Attach this file to an entity with script = "{script_name}.rhai".

fn on_start() {{
    // play_sound("spawn");
}}

fn on_update(dt) {{
    if input_pressed("A") {{
        move(-4.0 * dt, 0.0);
    }}
    if input_pressed("D") {{
        move(4.0 * dt, 0.0);
    }}
}}

fn on_key_down(key) {{
    if key == "Space" {{
        play_sound("jump");
    }}
}}

fn on_collision_enter(other) {{
    // ui_text("Hit " + other);
}}

fn on_destroy() {{
}}
"#
        )
    }

    pub fn template_component(component_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeComponentDefinition",
            "runtime": "rust_component_data",
            "component_type": component_name,
            "enabled": true,
            "fields": {
                "enabled": {"type": "bool", "default": true}
            },
            "editor": {
                "category": "Custom",
                "icon": "component"
            }
        })
    }

    pub fn template_system(system_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeSystemDefinition",
            "runtime": "rust_system_data",
            "system": system_name,
            "enabled": true,
            "run_in_editor": false,
            "run_in_play": true,
            "reads": [],
            "writes": [],
            "schedule": {"phase": "Gameplay", "order": 100}
        })
    }

    pub fn template_scene(scene_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "engine_version": ENGINE_VERSION,
            "scene_name": scene_name,
            "mode": "EDITOR",
            "active_tool": "Select",
            "tile_brush": 0,
            "brush_size": 1,
            "camera": {"x": 0, "y": 0, "zoom": 1.0},
            "control_groups": {},
            "grid": null,
            "tiles": [],
            "settings": {},
            "entities": [],
            "editor_view_settings": {},
        })
    }

    pub fn template_prefab(prefab_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "engine_version": ENGINE_VERSION,
            "prefab_name": prefab_name,
            "entity": {
                "type": "Unit",
                "name": prefab_name,
                "enabled": true,
                "visible": true,
                "locked": false,
                "x": 0,
                "y": 0,
                "speed": 3.5,
                "radius": 0.45,
                "sprite_name": null,
                "tag": "Untagged",
                "layer": "Default",
                "state": "IDLE",
                "command": "IDLE",
                "prefab_source": null,
                "is_prefab_instance": false,
                "components": [],
                "scripts": [],
            },
        })
    }

    pub fn template_sprite_import(sprite_name: &str, source_path: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeSpriteImport",
            "name": sprite_name,
            "source": source_path,
            "pixels_per_unit": 32,
            "filter": "nearest",
            "pivot": [0.5, 0.5],
            "atlas": null,
            "animations": [],
            "collision": {"generate": false, "shape": "rect"},
        })
    }

    pub fn template_sound_cue(cue_name: &str, source_path: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeSoundCue",
            "name": cue_name,
            "source": source_path,
            "bus": "SFX",
            "volume": 1.0,
            "pitch": 1.0,
            "stream": false,
            "loop": false,
            "spatial": {"enabled": false, "min_distance": 4.0, "max_distance": 18.0},
        })
    }

    pub fn template_audio_event(event_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeAudioEvent",
            "runtime": "kira",
            "name": event_name,
            "bus": "SFX",
            "volume": 1.0,
            "fade_seconds": 0.0,
            "actions": [
                {"type": "play_sfx", "cue": event_name, "loop": false}
            ],
        })
    }

    pub fn template_enemy_prefab(enemy_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "engine_version": ENGINE_VERSION,
            "prefab_name": enemy_name,
            "entity": {
                "type": "Unit",
                "name": enemy_name,
                "enabled": true,
                "visible": true,
                "locked": false,
                "x": 0,
                "y": 0,
                "speed": 2.8,
                "radius": 0.45,
                "sprite_name": null,
                "script": "EnemyBrain.rhai",
                "tag": "Enemy",
                "layer": "Units",
                "state": "IDLE",
                "command": "IDLE",
                "components": [
                    {"component_type": "Transform", "x": 0.0, "y": 0.0, "rotation": 0.0, "scale_x": 1.0, "scale_y": 1.0},
                    {"component_type": "Health", "health": 60.0, "max_health": 60.0, "alive": true},
                    {"component_type": "Stats", "attack": 8.0, "defense": 1.0, "speed": 2.8},
                    {"component_type": "Collider2D", "shape": "circle", "radius": 0.45, "collision_layer": "Enemy", "collision_mask": ["Default", "Player", "Projectile"]},
                    {"component_type": "Rigidbody2D", "body_type": "dynamic", "use_gravity": false, "friction": 0.4}
                ],
                "scripts": [{"runtime": "rhai", "path": "EnemyBrain.rhai"}],
            },
        })
    }

    pub fn template_ui_asset(ui_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeUIPrefab",
            "name": ui_name,
            "entity": {
                "type": "GameObject",
                "name": ui_name,
                "tag": "UI",
                "layer": "UI",
                "visible": true,
                "components": [
                    {"component_type": "Transform", "x": 24.0, "y": 24.0, "rotation": 0.0, "scale_x": 1.0, "scale_y": 1.0},
                    {"component_type": "UIElement", "element_type": "Label", "text": ui_name, "width": 180.0, "height": 42.0, "interactable": false}
                ],
                "scripts": [],
            },
        })
    }

    pub fn template_material(material_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeMaterial2D",
            "name": material_name,
            "shader": "sprite_default",
            "tint": [255, 255, 255, 255],
            "blend_mode": "alpha",
            "render_queue": 0,
            "texture": null,
            "lighting": false,
            "fog": false,
            "roughness": 0.5,
            "emission": [0, 0, 0],
            "fallback": "Default",
        })
    }

    pub fn template_particle_preset(name: &str) -> Value {
        let mut preset = crate::systems::particle_system::default_particle_emitter();
        if let Some(map) = preset.as_object_mut() {
            map.insert("version".to_string(), json!(ENGINE_VERSION));
            map.insert("kind".to_string(), json!("MiniForgeParticlePreset"));
            map.insert("name".to_string(), json!(name));
        }
        preset
    }

    pub fn template_shader(shader_name: &str) -> Value {
        json!({
            "version": ENGINE_VERSION,
            "kind": "MiniForgeShader2D",
            "name": shader_name,
            "source": format!("builtin://{shader_name}"),
            "supports_lighting": shader_name.contains("lit"),
            "supports_fog": shader_name.contains("fog"),
            "fallback": "sprite_default",
        })
    }

    fn logical_name_from_file_name(name: &str, fallback: &str) -> String {
        let file_name = Path::new(name)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(name);
        let stem = file_name
            .strip_suffix(".component.json")
            .or_else(|| file_name.strip_suffix(".system.json"))
            .or_else(|| file_name.strip_suffix(".mfgraph"))
            .or_else(|| file_name.strip_suffix(".rhai"))
            .or_else(|| {
                Path::new(file_name)
                    .file_stem()
                    .and_then(|value| value.to_str())
            })
            .unwrap_or(file_name);
        let mut asset_name = String::new();
        for part in stem.replace(['-', '.', ' '], "_").split('_') {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                asset_name.extend(first.to_uppercase());
                asset_name.push_str(chars.as_str());
            }
        }
        if asset_name.is_empty() {
            fallback.to_string()
        } else {
            asset_name
        }
    }

    pub fn create_script(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let mut filename = Self::safe_name(name, "NewScript");
        if Path::new(&filename)
            .extension()
            .and_then(|value| value.to_str())
            != Some("mfgraph")
        {
            let stem = Path::new(&filename)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("NewScript");
            filename = format!("{stem}.mfgraph");
        }
        let graph_name = Self::logical_name_from_file_name(&filename, "NewScript");
        let path = Self::unique_path(paths.visual_graphs, &filename);
        Self::create_json_file(path, &Self::template_visual_graph(&graph_name), true)
    }

    pub fn create_visual_graph(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let mut filename = Self::safe_name(name, "NewGraph");
        if !filename.ends_with(".mfgraph") {
            filename.push_str(".mfgraph");
        }
        let graph_name = Path::new(&filename)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("NewGraph");
        let path = Self::unique_path(paths.visual_graphs, &filename);
        Self::create_json_file(path, &Self::template_visual_graph(graph_name), true)
    }

    pub fn create_rhai_script(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let mut filename = Self::safe_name(name, "NewScript");
        if !filename.ends_with(".rhai") {
            let stem = Path::new(&filename)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("NewScript");
            filename = format!("{stem}.rhai");
        }
        let script_name = Self::logical_name_from_file_name(&filename, "NewScript");
        let path = Self::unique_path(paths.scripts, &filename);
        Self::create_file(path, &Self::template_rhai_script(&script_name), true)
    }

    pub fn create_component(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let mut filename = Self::safe_name(name, "NewComponent");
        if !filename.ends_with(".component.json") {
            filename = format!(
                "{}.component.json",
                Path::new(&filename)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("NewComponent")
            );
        }
        let component_name = Self::logical_name_from_file_name(&filename, "NewComponent");
        let path = Self::unique_path(paths.components, &filename);
        Self::create_json_file(path, &Self::template_component(&component_name), true)
    }

    pub fn create_system(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let mut filename = Self::safe_name(name, "NewSystem");
        if !filename.ends_with(".system.json") {
            filename = format!(
                "{}.system.json",
                Path::new(&filename)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("NewSystem")
            );
        }
        let system_name = Self::logical_name_from_file_name(&filename, "NewSystem");
        let path = Self::unique_path(paths.systems, &filename);
        Self::create_json_file(path, &Self::template_system(&system_name), true)
    }

    pub fn create_json(
        project_path: impl AsRef<Path>,
        folder: Option<&Path>,
        name: &str,
    ) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let target_folder = folder.unwrap_or(&paths.data);
        let mut filename = Self::safe_name(name, "NewData");
        if !filename.ends_with(".json") {
            filename.push_str(".json");
        }
        let path = Self::unique_path(target_folder, &filename);
        Self::create_json_file(
            path,
            &json!({"created_by": "MiniForge", "version": ENGINE_VERSION}),
            true,
        )
    }

    pub fn create_txt(
        project_path: impl AsRef<Path>,
        folder: Option<&Path>,
        name: &str,
    ) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let target_folder = folder.unwrap_or(&paths.data);
        let mut filename = Self::safe_name(name, "NewText");
        if !filename.ends_with(".txt") {
            filename.push_str(".txt");
        }
        let path = Self::unique_path(target_folder, &filename);
        Self::create_file(path, "New text file\n", true)
    }

    pub fn create_scene(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let mut filename = Self::safe_name(name, "main");
        if !filename.ends_with(".scene") {
            filename.push_str(".scene");
        }
        let scene_name = Path::new(&filename)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("main");
        let path = Self::unique_path(paths.scenes, &filename);
        Self::create_json_file(path, &Self::template_scene(scene_name), true)
    }

    pub fn create_prefab(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let mut filename = Self::safe_name(name, "NewPrefab");
        if !filename.ends_with(".prefab") {
            filename.push_str(".prefab");
        }
        let prefab_name = Path::new(&filename)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("NewPrefab");
        let path = Self::unique_path(paths.prefabs, &filename);
        Self::create_json_file(path, &Self::template_prefab(prefab_name), true)
    }

    pub fn create_enemy_prefab(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "Enemy");
        let path = Self::unique_path(&paths.prefabs, &format!("{safe}.prefab"));
        Self::create_json_file(path, &Self::template_enemy_prefab(&safe), true)
    }

    pub fn create_ui_asset(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "NewUI");
        let path = Self::unique_path(&paths.prefabs, &format!("{safe}.ui.prefab"));
        Self::create_json_file(path, &Self::template_ui_asset(&safe), true)
    }

    pub fn create_sprite_import(
        project_path: impl AsRef<Path>,
        name: &str,
        source_path: &str,
    ) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "Sprite");
        let path = Self::unique_path(paths.sprites, &format!("{safe}.sprite.json"));
        Self::create_json_file(
            path,
            &Self::template_sprite_import(&safe, source_path),
            true,
        )
    }

    pub fn create_sound_cue(
        project_path: impl AsRef<Path>,
        name: &str,
        source_path: &str,
    ) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "SoundCue");
        let path = Self::unique_path(paths.audio, &format!("{safe}.sound.json"));
        Self::create_json_file(path, &Self::template_sound_cue(&safe, source_path), true)
    }

    pub fn create_audio_event(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "AudioEvent");
        let path = Self::unique_path(paths.audio, &format!("{safe}.audio.json"));
        Self::create_json_file(path, &Self::template_audio_event(&safe), true)
    }

    pub fn create_material(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "Material");
        let path = Self::unique_path(paths.data, &format!("{safe}.material.json"));
        Self::create_json_file(path, &Self::template_material(&safe), true)
    }

    pub fn create_particle_preset(
        project_path: impl AsRef<Path>,
        name: &str,
    ) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "Particles");
        let path = Self::unique_path(paths.data, &format!("{safe}.particles.json"));
        Self::create_json_file(path, &Self::template_particle_preset(&safe), true)
    }

    pub fn create_shader(project_path: impl AsRef<Path>, name: &str) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let safe = Self::safe_name(name, "sprite_lit_fog");
        let path = Self::unique_path(paths.data, &format!("{safe}.shader.json"));
        Self::create_json_file(path, &Self::template_shader(&safe), true)
    }

    pub fn create_special_folder(
        project_path: impl AsRef<Path>,
        folder_type: &str,
    ) -> io::Result<PathBuf> {
        let paths = Self::get_project_paths(project_path);
        let target = match folder_type.to_lowercase().as_str() {
            "sprites" => paths.sprites,
            "audio" => paths.audio,
            "data" => paths.data,
            "prefabs" => paths.prefabs,
            "scripts" => paths.scripts,
            "visual_graphs" | "graphs" => paths.visual_graphs,
            "components" => paths.components,
            "systems" => paths.systems,
            "scenes" => paths.scenes,
            "settings" => paths.settings,
            "plugins" => paths.plugins,
            other => paths.project.join(other),
        };
        fs::create_dir_all(&target)?;
        Ok(target)
    }

    pub fn safe_copy_to_folder(
        source_path: impl AsRef<Path>,
        target_folder: impl AsRef<Path>,
    ) -> io::Result<PathBuf> {
        fs::create_dir_all(target_folder.as_ref())?;
        let filename = source_path
            .as_ref()
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Asset");
        let target = Self::unique_path(target_folder, filename);
        fs::copy(source_path, &target)?;
        Ok(target)
    }
}
