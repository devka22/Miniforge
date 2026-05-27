use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::engine::asset_database::AssetDatabase;
use crate::engine::asset_tools::AssetTools;
use crate::engine::miniforge_2d::blueprint::supported_node_kinds;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct ProjectValidator {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ProjectValidator {
    pub fn validate(&mut self, project_path: impl AsRef<Path>) -> bool {
        self.validate_with_context(project_path, &[], None)
    }

    pub fn validate_with_context(
        &mut self,
        project_path: impl AsRef<Path>,
        entities: &[GameObject],
        asset_database: Option<&AssetDatabase>,
    ) -> bool {
        self.errors.clear();
        self.warnings.clear();
        let project_path = project_path.as_ref();
        let paths = AssetTools::get_project_paths(project_path);
        self.validate_folders(&paths);
        self.validate_project_files(project_path);
        self.validate_json_files(project_path);
        self.validate_program_assets(&paths.scripts);
        self.validate_scenes(&paths.scenes);
        self.validate_prefabs(&paths.prefabs);
        self.validate_asset_references(entities, asset_database);
        self.validate_build_settings(&paths.settings, &paths.scenes);
        self.errors.is_empty()
    }

    fn validate_folders(&mut self, paths: &crate::engine::asset_tools::ProjectPaths) {
        for path in paths.as_map().values() {
            if !path.exists() {
                self.errors
                    .push(format!("Falta carpeta: {}", path.display()));
            }
        }
    }

    fn validate_project_files(&mut self, project_path: &Path) {
        if !project_path.join("project.json").exists()
            && !project_path.join("project").join("project.json").exists()
        {
            self.errors.push("Falta project.json".to_string());
        }
        if !project_path.join("manifest.json").exists()
            && !project_path.join("project").join("manifest.json").exists()
        {
            self.warnings.push("Falta manifest.json".to_string());
        }
        if !project_path.join("engine_config.json").exists() {
            self.warnings.push("Falta engine_config.json".to_string());
        }
    }

    fn validate_json_files(&mut self, root: &Path) {
        for path in walk_files(root) {
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            if let Err(error) = AssetTools::read_json(&path) {
                self.errors
                    .push(format!("JSON invalido: {} | {error}", path.display()));
            }
        }
    }

    fn validate_program_assets(&mut self, root: &Path) {
        for path in walk_files(root) {
            match path.extension().and_then(|value| value.to_str()) {
                Some("mfgraph") => match AssetTools::read_json(&path) {
                    Ok(data) => {
                        if data.get("nodes").and_then(Value::as_array).is_none() {
                            self.errors
                                .push(format!("Graph sin nodes: {}", path.display()));
                        }
                        self.validate_graph_nodes(&data, &path);
                        if data
                            .get("runtime")
                            .and_then(Value::as_str)
                            .is_some_and(|runtime| runtime != "rust_visual_graph")
                        {
                            self.warnings
                                .push(format!("Graph con runtime desconocido: {}", path.display()));
                        }
                    }
                    Err(error) => self
                        .errors
                        .push(format!("Graph invalido: {} | {error}", path.display())),
                },
                Some("rhai") => {
                    let Ok(source) = fs::read_to_string(&path) else {
                        self.errors
                            .push(format!("Script Rhai ilegible: {}", path.display()));
                        continue;
                    };
                    if let Err(error) = rhai::Engine::new().compile(rewrite_spawn_api(&source)) {
                        self.errors.push(format!(
                            "Script Rhai invalido: {} | {error}",
                            path.display()
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    fn validate_scenes(&mut self, scenes: &Path) {
        if !scenes.exists() {
            return;
        }
        let scene_files = walk_files(scenes);
        if !scene_files
            .iter()
            .any(|path| path.extension().and_then(|value| value.to_str()) == Some("scene"))
        {
            self.warnings
                .push("No hay escenas .scene en el proyecto".to_string());
        }
        for path in scene_files {
            if path.extension().and_then(|value| value.to_str()) != Some("scene") {
                continue;
            }
            match AssetTools::read_json(&path) {
                Ok(data) => {
                    if data.get("entities").and_then(Value::as_array).is_none()
                        && data.get("objects").and_then(Value::as_array).is_none()
                    {
                        self.warnings
                            .push(format!("Escena sin entities: {}", path.display()));
                    }
                    if data.get("engine_version").is_none() && data.get("version").is_none() {
                        self.warnings.push(format!(
                            "Escena sin version; se migrara al abrir: {}",
                            path.display()
                        ));
                    }
                    self.validate_references_in_value(&data, &path);
                }
                Err(error) => {
                    let backup = path.with_extension("scene.bak");
                    if backup.exists() {
                        self.warnings.push(format!(
                            "Escena invalida pero tiene backup: {} | {error}",
                            path.display()
                        ));
                    } else {
                        self.errors
                            .push(format!("Escena invalida: {} | {error}", path.display()));
                    }
                }
            }
        }
    }

    fn validate_prefabs(&mut self, prefabs: &Path) {
        for path in walk_files(prefabs) {
            if path.extension().and_then(|value| value.to_str()) != Some("prefab") {
                continue;
            }
            match AssetTools::read_json(&path) {
                Ok(data) => {
                    if data.get("entity").is_none() {
                        self.warnings
                            .push(format!("Prefab sin entity: {}", path.display()));
                    } else if let Some(entity) = data.get("entity") {
                        if entity.get("components").and_then(Value::as_array).is_none() {
                            self.warnings
                                .push(format!("Prefab sin components: {}", path.display()));
                        }
                        if entity
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .trim()
                            .is_empty()
                        {
                            self.errors
                                .push(format!("Prefab con entity sin nombre: {}", path.display()));
                        }
                    }
                    if data
                        .get("variant")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        && data.get("base_prefab").is_none()
                    {
                        self.warnings.push(format!(
                            "Prefab variant sin base_prefab: {}",
                            path.display()
                        ));
                    }
                    self.validate_references_in_value(&data, &path);
                }
                Err(error) => self
                    .errors
                    .push(format!("Prefab invalido: {} | {error}", path.display())),
            }
        }
    }

    fn validate_asset_references(
        &mut self,
        entities: &[GameObject],
        asset_database: Option<&AssetDatabase>,
    ) {
        let Some(database) = asset_database else {
            return;
        };
        for entity in entities {
            let Some(sprite) = &entity.sprite_name else {
                continue;
            };
            let exists = database.assets.values().any(|asset| asset.name == *sprite);
            if !exists {
                self.warnings.push(format!(
                    "Sprite referenciado no existe en asset database: {sprite}"
                ));
            }
        }
        for entity in entities {
            if let Some(script) = &entity.script {
                let exists = database
                    .assets
                    .values()
                    .any(|asset| asset.relative_path.ends_with(script));
                if !exists {
                    self.warnings
                        .push(format!("Script referenciado no existe: {script}"));
                }
            }
        }
    }

    fn validate_build_settings(&mut self, settings: &Path, scenes: &Path) {
        let path = settings.join("build_settings.json");
        if !path.exists() {
            return;
        }
        let Ok(data) = AssetTools::read_json(&path) else {
            return;
        };
        if data
            .get("game_name")
            .and_then(Value::as_str)
            .unwrap_or("MiniForgeGame")
            .trim()
            .is_empty()
        {
            self.errors
                .push("Build Settings: game_name vacio".to_string());
        }
        if let Some(start_scene) = data.get("start_scene").and_then(Value::as_str)
            && !scenes.join(start_scene).exists()
        {
            self.warnings.push(format!(
                "Build Settings: start_scene no existe en scenes/: {start_scene}"
            ));
        }
    }

    fn validate_references_in_value(&mut self, data: &Value, source: &Path) {
        let project_root = source
            .ancestors()
            .find(|path| path.join("engine_config.json").exists())
            .unwrap_or_else(|| Path::new(""));
        for reference in collect_project_references(data) {
            if !project_root.join(&reference).exists() {
                self.warnings.push(format!(
                    "Referencia rota en {}: {}",
                    source.display(),
                    reference
                ));
            }
        }
    }

    fn validate_graph_nodes(&mut self, data: &Value, path: &Path) {
        let Some(nodes) = data.get("nodes").and_then(Value::as_array) else {
            return;
        };
        let mut ids = std::collections::BTreeSet::new();
        for node in nodes {
            if let Some(id) = node.get("id").and_then(Value::as_str) {
                if !ids.insert(id.to_string()) {
                    self.errors.push(format!(
                        "Graph con node id duplicado: {} | {id}",
                        path.display()
                    ));
                }
            } else {
                self.errors
                    .push(format!("Graph con node sin id: {}", path.display()));
            }
        }
        let visual_nodes = allowed_visual_script_nodes();
        let blueprint_nodes = supported_node_kinds();
        for node in nodes {
            let id = node.get("id").and_then(Value::as_str).unwrap_or("<sin id>");
            if let Some(kind) = node.get("kind").and_then(Value::as_str)
                && !blueprint_nodes.contains(kind)
            {
                self.errors.push(format!(
                    "Graph con node kind invalido: {} | {id}:{kind}",
                    path.display()
                ));
            }
            if let Some(node_type) = node.get("type").and_then(Value::as_str)
                && !visual_nodes.contains(node_type)
            {
                self.warnings.push(format!(
                    "Graph con node type no reconocido por runtime actual: {} | {id}:{node_type}",
                    path.display()
                ));
            }
            for key in [
                "next",
                "true_next",
                "false_next",
                "then_0",
                "then_1",
                "a_next",
                "b_next",
            ] {
                let Some(target) = node.get(key).and_then(Value::as_str) else {
                    continue;
                };
                if !ids.contains(target) {
                    self.errors.push(format!(
                        "Graph con referencia de exec rota: {} | {id}.{key} -> {target}",
                        path.display()
                    ));
                }
            }
        }
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !root.exists() {
        return files;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}

fn rewrite_spawn_api(source: &str) -> String {
    source.replace("spawn(", "spawn_entity(")
}

fn collect_project_references(value: &Value) -> Vec<String> {
    let mut references = Vec::new();
    collect_project_references_inner(value, &mut references);
    references.sort();
    references.dedup();
    references
}

fn collect_project_references_inner(value: &Value, references: &mut Vec<String>) {
    match value {
        Value::String(text)
            if (text.starts_with("assets/")
                || text.starts_with("scripts/")
                || text.starts_with("saves/"))
                && text.contains('.') =>
        {
            references.push(text.clone());
        }
        Value::Array(items) => {
            for item in items {
                collect_project_references_inner(item, references);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_project_references_inner(value, references);
            }
        }
        _ => {}
    }
}

fn allowed_visual_script_nodes() -> std::collections::BTreeSet<&'static str> {
    std::collections::BTreeSet::from([
        "EventStart",
        "EventUpdate",
        "EventClick",
        "EventTrigger",
        "ConstructionScript",
        "CustomEvent",
        "CallEvent",
        "BroadcastEvent",
        "Sequence",
        "DoOnce",
        "ResetDoOnce",
        "Gate",
        "OpenGate",
        "CloseGate",
        "ToggleGate",
        "FlipFlop",
        "Move",
        "MoveTowards",
        "SetVelocity",
        "AddForce",
        "StopMovement",
        "SetSpeed",
        "SetPosition",
        "SetRotation",
        "SetScale",
        "Log",
        "Damage",
        "Heal",
        "SetHealth",
        "BranchHealth",
        "BranchVariable",
        "SetEnabled",
        "SetTag",
        "SetVariable",
        "AddVariable",
        "ToggleVariable",
        "SetBlackboard",
        "Wait",
        "ConfigureSpawner",
        "SetAnimation",
        "SetUiText",
        "InventoryAdd",
        "InventoryRemove",
        "BranchItem",
        "EquipItem",
        "EconomyAdd",
        "EconomySpend",
        "BranchResource",
        "AddProductionRecipe",
        "SetPreferredRecipe",
        "QueuePreferredRecipe",
        "AddQuest",
        "QuestProgress",
        "TriggerAbility",
        "RechargeAbility",
        "StartCooldown",
        "SetState",
        "AddStatusEffect",
        "CompleteQuest",
        "AddComponent",
        "SetComponentNumber",
        "DestroySelf",
    ])
}
