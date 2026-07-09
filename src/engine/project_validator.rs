use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_database::{AssetDatabase, stable_guid};
use crate::engine::asset_tools::AssetTools;
use crate::engine::miniforge_2d::blueprint::supported_node_kinds;
use crate::engine::prefab_serializer::PrefabSerializer;
use crate::engine::scene_serializer::SceneSerializer;
use crate::engine::version::ENGINE_VERSION;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct ProjectValidator {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectAutoFixReport {
    pub actions: Vec<String>,
    pub skipped: Vec<String>,
}

impl ProjectAutoFixReport {
    pub fn fixed_count(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty() && self.skipped.is_empty()
    }
}

impl ProjectValidator {
    pub fn validate(&mut self, project_path: impl AsRef<Path>) -> bool {
        self.validate_with_context(project_path, &[], None)
    }

    pub fn auto_fix_safe(
        &mut self,
        project_path: impl AsRef<Path>,
    ) -> std::io::Result<ProjectAutoFixReport> {
        let project_path = project_path.as_ref();
        let mut report = ProjectAutoFixReport::default();

        let paths = AssetTools::ensure_project_folders(project_path)?;
        report.actions.push("Carpetas base verificadas".to_string());

        self.create_config_backups(project_path, &paths, &mut report)?;
        self.restore_scene_autosave(project_path, &paths, &mut report)?;
        self.reset_corrupt_layouts(project_path, &paths, &mut report)?;
        self.regenerate_missing_guids(project_path, &mut report)?;
        self.remove_null_reference_entries(project_path, &mut report)?;
        self.mark_missing_assets(project_path, &mut report)?;
        self.disable_broken_plugins(project_path, &paths.plugins, &mut report)?;

        match AssetDatabase::new(paths.assets.clone(), project_path)
            .and_then(|mut database| database.rebuild_dependency_graph())
        {
            Ok(graph) => report.actions.push(format!(
                "Asset index reconstruido con {} relaciones",
                graph.len()
            )),
            Err(error) => report
                .skipped
                .push(format!("No se pudo reconstruir asset index: {error}")),
        }

        self.validate(project_path);
        Ok(report)
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
        self.validate_engine_config(project_path, &paths.scenes);
        self.validate_json_files(project_path);
        self.validate_program_assets(&paths.scripts);
        self.validate_scenes(&paths.scenes);
        self.validate_prefabs(&paths.prefabs);
        self.validate_2d_documents(project_path);
        self.validate_plugins(project_path);
        self.validate_duplicate_guids(project_path);
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

    fn validate_engine_config(&mut self, project_path: &Path, scenes: &Path) {
        let path = project_path.join("engine_config.json");
        if !path.exists() {
            return;
        }
        let Ok(data) = AssetTools::read_json(&path) else {
            return;
        };
        if let Some(start_scene) = data.get("start_scene").and_then(Value::as_str) {
            if start_scene.trim().is_empty() {
                self.errors
                    .push("engine_config.json: start_scene vacio".to_string());
            } else if !scenes
                .join(crate::engine::scene_manager::normalize_scene_name(
                    start_scene,
                ))
                .exists()
            {
                self.warnings.push(format!(
                    "engine_config.json: start_scene no existe en saves/scenes/: {start_scene}"
                ));
            }
        }
        if let Some(rendering) = data.get("rendering").and_then(Value::as_object) {
            for (key, warning) in [
                (
                    "sprite_batching",
                    "rendering.sprite_batching desactivado: juegos 2D masivos perderan rendimiento",
                ),
                (
                    "view_frustum_culling",
                    "rendering.view_frustum_culling desactivado: se dibujaran objetos fuera de camara",
                ),
                (
                    "occlusion_culling",
                    "rendering.occlusion_culling desactivado: objetos tapados seguiran consumiendo render",
                ),
                (
                    "lod_enabled",
                    "rendering.lod_enabled desactivado: objetos lejanos no bajaran detalle",
                ),
                (
                    "backface_culling",
                    "rendering.backface_culling desactivado: caras traseras 3D pueden costar rendimiento extra",
                ),
            ] {
                if rendering.get(key).and_then(Value::as_bool) == Some(false) {
                    self.warnings.push(warning.to_string());
                }
            }
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
                        let data = match crate::engine::visual_graph_serializer::VisualGraphSerializer::try_migrate(data) {
                            Ok(report) => {
                                if report.changed {
                                    self.warnings.push(format!(
                                        "Graph legacy pendiente de guardar en schema actual: {}",
                                        path.display()
                                    ));
                                }
                                report.data
                            }
                            Err(error) => {
                                self.errors.push(format!(
                                    "Graph incompatible: {} | {error}",
                                    path.display()
                                ));
                                continue;
                            }
                        };
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
                Some("luau") => {
                    let Ok(source) = fs::read_to_string(&path) else {
                        self.errors
                            .push(format!("Script Luau ilegible: {}", path.display()));
                        continue;
                    };
                    let filename = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("script.luau");
                    if let Err(error) =
                        crate::engine::luau_scripting::LuauScriptRuntime::validate_source(
                            &source, filename,
                        )
                    {
                        self.errors.push(format!(
                            "Script Luau invalido: {} | {error}",
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
                Ok(data) => match SceneSerializer::try_migrate(data) {
                    Ok(migration) => {
                        let data = migration.data;
                        if migration.changed {
                            self.warnings.push(format!(
                                "Escena legacy pendiente de guardar en schema actual: {}",
                                path.display()
                            ));
                        }
                        if let Some(entities) = data.get("entities").and_then(Value::as_array)
                            && entities.len() > 3000
                        {
                            self.warnings.push(format!(
                                "Escena masiva sin particion explicita: {} tiene {} entidades; considera WorldPartition2D/RuntimeBudget2D",
                                path.display(),
                                entities.len()
                            ));
                        }
                        self.validate_references_in_value(&data, &path);
                    }
                    Err(error) => self.errors.push(format!(
                        "Esquema de escena invalido: {} | {error}",
                        path.display()
                    )),
                },
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
                Ok(data) => match PrefabSerializer::try_migrate(data) {
                    Ok(migration) => {
                        let data = migration.data;
                        if migration.changed {
                            self.warnings.push(format!(
                                "Prefab legacy pendiente de guardar en schema actual: {}",
                                path.display()
                            ));
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
                    Err(error) => self.errors.push(format!(
                        "Esquema de prefab invalido: {} | {error}",
                        path.display()
                    )),
                },
                Err(error) => self
                    .errors
                    .push(format!("Prefab invalido: {} | {error}", path.display())),
            }
        }
    }

    fn validate_2d_documents(&mut self, project_path: &Path) {
        for path in walk_files(project_path) {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if !matches!(extension, "mfui" | "anim2d" | "seq2d")
                && !name.ends_with(".ui2d.json")
                && !name.ends_with(".anim2d.json")
                && !name.ends_with(".seq2d.json")
                && !name.ends_with(".spriteframes")
            {
                continue;
            }
            match AssetTools::read_json(&path) {
                Ok(data) => {
                    if extension == "mfui" || name.ends_with(".ui2d.json") {
                        if data.get("widgets").and_then(Value::as_array).is_none()
                            && data.get("root").is_none()
                        {
                            self.warnings
                                .push(format!("UI document sin widgets/root: {}", path.display()));
                        }
                        self.validate_references_in_value(&data, &path);
                    }
                    if extension == "seq2d" || name.ends_with(".seq2d.json") {
                        if data.get("tracks").and_then(Value::as_array).is_none() {
                            self.errors
                                .push(format!("Sequencer2D sin tracks: {}", path.display()));
                        }
                        if data
                            .get("duration")
                            .and_then(Value::as_f64)
                            .is_some_and(|duration| duration < 0.0)
                        {
                            self.errors.push(format!(
                                "Sequencer2D con duration negativa: {}",
                                path.display()
                            ));
                        }
                    }
                    if extension == "anim2d" || name.ends_with(".anim2d.json") {
                        let has_frames = data
                            .get("frames")
                            .and_then(Value::as_array)
                            .is_some_and(|frames| !frames.is_empty())
                            || data
                                .get("animations")
                                .and_then(Value::as_array)
                                .is_some_and(|animations| !animations.is_empty());
                        if !has_frames {
                            self.warnings
                                .push(format!("Animacion 2D sin frames: {}", path.display()));
                        }
                    }
                }
                Err(error) => self.errors.push(format!(
                    "Documento 2D invalido: {} | {error}",
                    path.display()
                )),
            }
        }
    }

    fn validate_plugins(&mut self, project_path: &Path) {
        let plugins = project_path.join("plugins");
        if !plugins.exists() {
            return;
        }
        for entry in walk_files(&plugins) {
            if entry.file_name().and_then(|value| value.to_str()) != Some("plugin.json") {
                continue;
            }
            match AssetTools::read_json(&entry) {
                Ok(data) => {
                    for key in ["name", "version", "enabled"] {
                        if data.get(key).is_none() {
                            self.warnings
                                .push(format!("Plugin manifest sin {key}: {}", entry.display()));
                        }
                    }
                    if data.get("min_engine_version").is_none() {
                        self.warnings.push(format!(
                            "Plugin sin min_engine_version: {}",
                            entry.display()
                        ));
                    }
                }
                Err(error) => self.errors.push(format!(
                    "Plugin manifest invalido: {} | {error}",
                    entry.display()
                )),
            }
        }
    }

    fn validate_duplicate_guids(&mut self, project_path: &Path) {
        let mut seen = std::collections::BTreeMap::<String, PathBuf>::new();
        for path in walk_files(project_path) {
            if !is_guid_candidate(project_path, &path) {
                continue;
            }
            let Ok(data) = AssetTools::read_json(&path) else {
                continue;
            };
            for guid in collect_guid_values(&data) {
                if let Some(first) = seen.insert(guid.clone(), path.clone()) {
                    self.errors.push(format!(
                        "GUID duplicado: {guid} en {} y {}",
                        first.display(),
                        path.display()
                    ));
                }
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

    fn create_config_backups(
        &self,
        project_path: &Path,
        paths: &crate::engine::asset_tools::ProjectPaths,
        report: &mut ProjectAutoFixReport,
    ) -> std::io::Result<()> {
        let candidates = [
            project_path.join("engine_config.json"),
            project_path.join("project.json"),
            project_path.join("manifest.json"),
            paths.settings.join("build_settings.json"),
            paths.settings.join("runtime_config.json"),
            paths.settings.join("tags.json"),
            paths.settings.join("layers.json"),
            project_path.join("project").join("project.json"),
            project_path.join("project").join("manifest.json"),
        ];
        for path in candidates {
            if !path.exists() {
                continue;
            }
            let backup = path.with_extension(format!(
                "{}.bak",
                path.extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("json")
            ));
            if backup.exists() {
                continue;
            }
            fs::copy(&path, &backup)?;
            report.actions.push(format!(
                "Backup config creado: {}",
                display_project_path(project_path, &backup)
            ));
        }
        Ok(())
    }

    fn restore_scene_autosave(
        &self,
        project_path: &Path,
        paths: &crate::engine::asset_tools::ProjectPaths,
        report: &mut ProjectAutoFixReport,
    ) -> std::io::Result<()> {
        let autosave = project_path
            .join("saves")
            .join("autosave")
            .join("autosave.scene");
        if !autosave.exists() {
            return Ok(());
        }
        let main_scene = paths.scenes.join("main.scene");
        let main_scene_ok = main_scene.exists() && AssetTools::read_json(&main_scene).is_ok();
        if main_scene_ok {
            return Ok(());
        }
        if main_scene.exists() {
            fs::copy(&main_scene, main_scene.with_extension("scene.corrupt"))?;
        }
        if let Some(parent) = main_scene.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&autosave, &main_scene)?;
        report.actions.push(format!(
            "Autosave restaurado como {}",
            display_project_path(project_path, &main_scene)
        ));
        Ok(())
    }

    fn reset_corrupt_layouts(
        &self,
        project_path: &Path,
        paths: &crate::engine::asset_tools::ProjectPaths,
        report: &mut ProjectAutoFixReport,
    ) -> std::io::Result<()> {
        let candidates = [
            paths.settings.join("editor_layout.json"),
            project_path.join("project").join("editor_layout.json"),
        ];
        for path in candidates {
            if !path.exists() || AssetTools::read_json(&path).is_ok() {
                continue;
            }
            fs::copy(&path, path.with_extension("json.corrupt"))?;
            AssetTools::write_json(
                &path,
                &json!({
                    "version": ENGINE_VERSION,
                    "layout": "default",
                    "panels": [
                        "Scene",
                        "WorldOutliner",
                        "Inspector",
                        "ContentBrowser",
                        "Console",
                        "Problems",
                        "Profiler"
                    ]
                }),
            )?;
            report.actions.push(format!(
                "Layout corrupto reiniciado: {}",
                display_project_path(project_path, &path)
            ));
        }
        Ok(())
    }

    fn regenerate_missing_guids(
        &self,
        project_path: &Path,
        report: &mut ProjectAutoFixReport,
    ) -> std::io::Result<()> {
        for path in walk_files(project_path) {
            if !is_guid_candidate(project_path, &path) {
                continue;
            }
            let Ok(mut data) = AssetTools::read_json(&path) else {
                continue;
            };
            let Some(map) = data.as_object_mut() else {
                continue;
            };
            if map.get("guid").is_some() || map.get("asset_guid").is_some() {
                continue;
            }
            let relative = display_project_path(project_path, &path);
            map.insert("guid".to_string(), Value::String(stable_guid(&relative)));
            AssetTools::write_json(&path, &data)?;
            report
                .actions
                .push(format!("GUID faltante regenerado: {relative}"));
        }
        Ok(())
    }

    fn remove_null_reference_entries(
        &self,
        project_path: &Path,
        report: &mut ProjectAutoFixReport,
    ) -> std::io::Result<()> {
        for path in walk_files(project_path) {
            if !is_json_like(&path) {
                continue;
            }
            let Ok(mut data) = AssetTools::read_json(&path) else {
                continue;
            };
            let removed = remove_null_references_in_value(&mut data);
            if removed == 0 {
                continue;
            }
            AssetTools::write_json(&path, &data)?;
            report.actions.push(format!(
                "Referencias nulas eliminadas: {} ({removed})",
                display_project_path(project_path, &path)
            ));
        }
        Ok(())
    }

    fn mark_missing_assets(
        &self,
        project_path: &Path,
        report: &mut ProjectAutoFixReport,
    ) -> std::io::Result<()> {
        let mut missing = Vec::new();
        for path in walk_files(project_path) {
            if !is_json_like(&path) {
                continue;
            }
            let Ok(data) = AssetTools::read_json(&path) else {
                continue;
            };
            for reference in collect_project_references(&data) {
                if !project_path.join(&reference).exists() {
                    missing.push(json!({
                        "source": display_project_path(project_path, &path),
                        "reference": reference,
                        "status": "missing"
                    }));
                }
            }
        }
        if missing.is_empty() {
            return Ok(());
        }
        missing.sort_by_key(|item| item.to_string());
        missing.dedup();
        let path = project_path.join("project").join("missing_assets.json");
        AssetTools::write_json(
            &path,
            &json!({
                "version": ENGINE_VERSION,
                "missing": missing
            }),
        )?;
        report.actions.push(format!(
            "Assets faltantes marcados: {}",
            display_project_path(project_path, &path)
        ));
        Ok(())
    }

    fn disable_broken_plugins(
        &self,
        project_path: &Path,
        plugins_path: &Path,
        report: &mut ProjectAutoFixReport,
    ) -> std::io::Result<()> {
        if !plugins_path.exists() {
            return Ok(());
        }
        for path in walk_files(plugins_path) {
            if path.file_name().and_then(|value| value.to_str()) != Some("plugin.json") {
                continue;
            }
            let Ok(mut data) = AssetTools::read_json(&path) else {
                report.skipped.push(format!(
                    "Plugin ilegible requiere revision manual: {}",
                    display_project_path(project_path, &path)
                ));
                continue;
            };
            let missing_required = ["name", "version", "min_engine_version"]
                .iter()
                .any(|key| data.get(key).is_none());
            let missing_dependencies = missing_plugin_dependencies(&data, plugins_path);
            if !missing_required && missing_dependencies.is_empty() {
                continue;
            }
            let Some(map) = data.as_object_mut() else {
                continue;
            };
            map.insert("enabled".to_string(), Value::Bool(false));
            if !missing_dependencies.is_empty() {
                map.insert(
                    "disabled_reason".to_string(),
                    Value::String(format!(
                        "Dependencias faltantes: {}",
                        missing_dependencies.join(", ")
                    )),
                );
            } else {
                map.insert(
                    "disabled_reason".to_string(),
                    Value::String("Manifest incompleto".to_string()),
                );
            }
            AssetTools::write_json(&path, &data)?;
            report.actions.push(format!(
                "Plugin roto desactivado: {}",
                display_project_path(project_path, &path)
            ));
        }
        Ok(())
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
            if is_generated_directory(&path) {
                continue;
            }
            files.extend(walk_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}

fn is_generated_directory(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some(
            "build"
                | "builds"
                | "exports"
                | "packages"
                | "target"
                | ".cache"
                | ".git"
                | ".miniforge"
                | "logs"
                | "native"
                | "templates"
                | "tools"
        )
    )
}

fn display_project_path(project_path: &Path, path: &Path) -> String {
    path.strip_prefix(project_path)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn is_json_like(path: &Path) -> bool {
    let extension = path.extension().and_then(|value| value.to_str());
    matches!(
        extension,
        Some(
            "json"
                | "prefab"
                | "scene"
                | "mfgraph"
                | "spriteasset"
                | "spriteframes"
                | "anim2d"
                | "atlas"
                | "mfui"
                | "seq2d"
        )
    )
}

fn is_guid_candidate(project_path: &Path, path: &Path) -> bool {
    if !is_json_like(path) {
        return false;
    }
    let relative = display_project_path(project_path, path);
    relative.starts_with("assets/")
        || relative.starts_with("scripts/visual_graphs/")
        || relative.starts_with("saves/scenes/")
        || relative.ends_with(".prefab")
        || relative.ends_with(".spriteframes")
        || relative.ends_with(".anim2d.json")
        || relative.ends_with(".ui2d.json")
        || relative.ends_with(".seq2d.json")
}

fn remove_null_references_in_value(value: &mut Value) -> usize {
    match value {
        Value::Object(map) => {
            let mut removed = 0;
            for (key, child) in map.iter_mut() {
                if is_reference_collection_key(key)
                    && let Value::Array(items) = child
                {
                    let before = items.len();
                    items.retain(|item| !item.is_null());
                    removed += before.saturating_sub(items.len());
                }
                removed += remove_null_references_in_value(child);
            }
            removed
        }
        Value::Array(items) => items.iter_mut().map(remove_null_references_in_value).sum(),
        _ => 0,
    }
}

fn is_reference_collection_key(key: &str) -> bool {
    matches!(
        key,
        "references"
            | "dependencies"
            | "assets"
            | "scripts"
            | "graphs"
            | "plugins"
            | "scenes"
            | "timelines"
            | "bindings"
    )
}

fn missing_plugin_dependencies(data: &Value, plugins_path: &Path) -> Vec<String> {
    let mut missing = Vec::new();
    let Some(dependencies) = data.get("dependencies").and_then(Value::as_array) else {
        return missing;
    };
    for dependency in dependencies.iter().filter_map(Value::as_str) {
        if !plugin_dependency_exists(dependency, plugins_path) {
            missing.push(dependency.to_string());
        }
    }
    missing.sort();
    missing.dedup();
    missing
}

fn plugin_dependency_exists(dependency: &str, plugins_path: &Path) -> bool {
    if plugins_path.join(dependency).join("plugin.json").exists() {
        return true;
    }
    for path in walk_files(plugins_path) {
        if path.file_name().and_then(|value| value.to_str()) != Some("plugin.json") {
            continue;
        }
        let Ok(data) = AssetTools::read_json(path) else {
            continue;
        };
        if data.get("name").and_then(Value::as_str) == Some(dependency) {
            return true;
        }
    }
    false
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

fn collect_guid_values(value: &Value) -> Vec<String> {
    let mut values = Vec::new();
    collect_guid_values_inner(value, &mut values);
    values
}

fn collect_guid_values_inner(value: &Value, values: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if matches!(key.as_str(), "guid" | "asset_guid" | "id")
                    && let Some(text) = value.as_str()
                    && text.len() >= 8
                    && (text.contains('-') || key != "id")
                {
                    values.push(text.to_string());
                }
                collect_guid_values_inner(value, values);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_guid_values_inner(item, values);
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
