use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use petgraph::algo::{kosaraju_scc, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

pub const ASSET_METADATA_FORMAT: &str = "miniforge.asset-metadata";
pub const ASSET_METADATA_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetRecord {
    pub guid: String,
    pub relative_path: String,
    pub name: String,
    pub asset_type: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub modified_unix: u64,
    /// Content fingerprint used only to reconcile externally moved files.
    /// References continue to use `guid`, never this value.
    #[serde(default)]
    pub content_hash: String,
    pub import_settings: Value,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub compatibility: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AssetDatabase {
    pub root: PathBuf,
    pub project_root: PathBuf,
    pub metadata_file: PathBuf,
    pub assets: BTreeMap<String, AssetRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetDependencyReport {
    /// Dependencies appear before the assets that consume them.
    pub build_order: Vec<String>,
    pub cycles: Vec<Vec<String>>,
    pub edge_count: usize,
}

#[derive(Debug, Clone)]
struct ScannedAsset {
    relative_path: String,
    name: String,
    asset_type: String,
    size_bytes: u64,
    modified_unix: u64,
    content_hash: String,
    import_settings: Value,
    labels: Vec<String>,
    compatibility: Vec<String>,
}

impl AssetDatabase {
    pub fn new(root: impl AsRef<Path>, project_root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let project_root = project_root.as_ref().to_path_buf();
        let metadata_file = project_root.join("project").join("asset_metadata.json");
        let mut database = Self {
            root,
            project_root,
            metadata_file,
            assets: BTreeMap::new(),
        };
        database.load_metadata()?;
        Ok(database)
    }

    pub fn load_metadata(&mut self) -> io::Result<()> {
        self.assets.clear();
        if self.metadata_file.exists() {
            let value = AssetTools::read_json(&self.metadata_file)?;
            if let Some(format) = value.get("format").and_then(Value::as_str)
                && format != ASSET_METADATA_FORMAT
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("asset metadata format no soportado: {format}"),
                ));
            }
            let schema_version = value
                .get("schema_version")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if schema_version > ASSET_METADATA_SCHEMA_VERSION {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "asset metadata schema {schema_version} is newer than supported {}",
                        ASSET_METADATA_SCHEMA_VERSION
                    ),
                ));
            }
            if let Some(assets) = value.get("assets").and_then(Value::as_object) {
                for (path, record) in assets {
                    if let Ok(record) = serde_json::from_value::<AssetRecord>(record.clone()) {
                        self.assets.insert(path.clone(), record);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn save_metadata(&self) -> io::Result<()> {
        AssetTools::write_json(
            &self.metadata_file,
            &json!({
                "format": ASSET_METADATA_FORMAT,
                "schema_version": ASSET_METADATA_SCHEMA_VERSION,
                "engine_version": crate::engine::version::ENGINE_VERSION,
                "assets": self.assets,
            }),
        )
    }

    pub fn scan(&mut self) -> io::Result<()> {
        let paths = AssetTools::get_project_paths(&self.project_root);
        let scan_roots = [
            self.root.clone(),
            paths.scripts,
            paths.scenes,
            paths.settings,
            paths.prefabs,
        ];
        let mut file_paths = Vec::new();
        for root in scan_roots {
            if !root.exists() {
                continue;
            }
            file_paths.extend(walk_files(&root)?);
        }
        file_paths.sort();
        file_paths.dedup();

        let project_root = self.project_root.clone();
        let scanned = file_paths
            .par_iter()
            .map(|path| {
                let relative_path = path
                    .strip_prefix(&project_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                let (size_bytes, modified_unix) = file_stats(path);
                let content_hash = file_content_hash(path).unwrap_or_default();
                let asset_type = detect_asset_type(path);
                let labels = labels_for(path, &asset_type);
                let compatibility = compatibility_for(path, &asset_type, size_bytes);
                let name = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("asset")
                    .to_string();
                ScannedAsset {
                    relative_path,
                    name,
                    asset_type,
                    size_bytes,
                    modified_unix,
                    content_hash,
                    import_settings: default_import_settings(path),
                    labels,
                    compatibility,
                }
            })
            .collect::<Vec<_>>();

        let live_paths = scanned
            .iter()
            .map(|asset| asset.relative_path.clone())
            .collect::<BTreeSet<_>>();
        let mut previous = std::mem::take(&mut self.assets);
        let mut next = BTreeMap::new();
        let mut moved_paths = BTreeMap::new();
        for scanned in scanned {
            let existing_path = scanned.relative_path.clone();
            let moved_from = if previous.contains_key(&existing_path) {
                None
            } else {
                unique_content_match(&previous, &scanned, &live_paths)
            };
            let mut record = if let Some(record) = previous.remove(&existing_path) {
                record
            } else if let Some(old_path) = moved_from.as_ref() {
                let record = previous.remove(old_path).expect("matched asset exists");
                moved_paths.insert(old_path.clone(), existing_path.clone());
                record
            } else {
                AssetRecord {
                    guid: make_guid(&scanned.relative_path),
                    relative_path: scanned.relative_path.clone(),
                    name: scanned.name.clone(),
                    asset_type: scanned.asset_type.clone(),
                    size_bytes: scanned.size_bytes,
                    modified_unix: scanned.modified_unix,
                    content_hash: scanned.content_hash.clone(),
                    import_settings: scanned.import_settings.clone(),
                    labels: scanned.labels.clone(),
                    compatibility: scanned.compatibility.clone(),
                    dependencies: Vec::new(),
                }
            };
            record.relative_path = scanned.relative_path;
            record.name = scanned.name;
            record.asset_type = scanned.asset_type;
            record.size_bytes = scanned.size_bytes;
            record.modified_unix = scanned.modified_unix;
            record.content_hash = scanned.content_hash;
            record.labels = scanned.labels;
            record.compatibility = scanned.compatibility;
            next.insert(record.relative_path.clone(), record);
        }
        for record in next.values_mut() {
            for dependency in &mut record.dependencies {
                if let Some(next_path) = moved_paths.get(dependency) {
                    *dependency = next_path.clone();
                }
            }
        }
        self.assets = next;
        self.save_metadata()
    }

    /// Resolves a persistent asset identity to its current project-relative path.
    pub fn path_for_guid(&self, guid: &str) -> Option<&str> {
        self.assets
            .values()
            .find(|record| record.guid == guid)
            .map(|record| record.relative_path.as_str())
    }

    pub fn record_for_guid(&self, guid: &str) -> Option<&AssetRecord> {
        self.assets.values().find(|record| record.guid == guid)
    }

    /// Moves an asset and its metadata as one logical operation.
    ///
    /// If persisting metadata fails, both the in-memory table and filesystem
    /// move are rolled back, so references never observe a half-moved asset.
    pub fn move_asset(
        &mut self,
        source_relative: &str,
        target_relative: &str,
    ) -> io::Result<PathBuf> {
        validate_relative_asset_path(source_relative)?;
        validate_relative_asset_path(target_relative)?;
        let source = self.project_root.join(source_relative);
        let target = self.project_root.join(target_relative);
        if !source.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("asset no encontrado: {source_relative}"),
            ));
        }
        if target.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("asset destino ya existe: {target_relative}"),
            ));
        }
        let Some(mut record) = self.assets.get(source_relative).cloned() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("asset sin metadata: {source_relative}"),
            ));
        };
        let previous_assets = self.assets.clone();
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&source, &target)?;
        self.assets.remove(source_relative);
        record.relative_path = target_relative.to_string();
        record.name = target
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("asset")
            .to_string();
        self.assets.insert(target_relative.to_string(), record);
        for candidate in self.assets.values_mut() {
            for dependency in &mut candidate.dependencies {
                if dependency == source_relative {
                    *dependency = target_relative.to_string();
                }
            }
        }
        if let Err(error) = self.save_metadata() {
            self.assets = previous_assets;
            if let Err(rollback_error) = fs::rename(&target, &source) {
                return Err(io::Error::other(format!(
                    "metadata move failed: {error}; filesystem rollback failed: {rollback_error}"
                )));
            }
            return Err(error);
        }
        Ok(target)
    }

    pub fn get_import_settings(&self, relative_path: &str) -> Value {
        self.assets
            .get(relative_path)
            .map(|record| record.import_settings.clone())
            .unwrap_or_else(|| default_import_settings(Path::new(relative_path)))
    }

    pub fn set_import_setting(
        &mut self,
        relative_path: &str,
        key: &str,
        value: Value,
    ) -> io::Result<Value> {
        let record = self
            .assets
            .entry(relative_path.to_string())
            .or_insert_with(|| AssetRecord {
                guid: make_guid(relative_path),
                relative_path: relative_path.to_string(),
                name: Path::new(relative_path)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("asset")
                    .to_string(),
                asset_type: detect_asset_type(Path::new(relative_path)),
                size_bytes: 0,
                modified_unix: 0,
                content_hash: String::new(),
                import_settings: default_import_settings(Path::new(relative_path)),
                labels: labels_for(
                    Path::new(relative_path),
                    &detect_asset_type(Path::new(relative_path)),
                ),
                compatibility: Vec::new(),
                dependencies: Vec::new(),
            });
        if let Some(map) = record.import_settings.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        let settings = record.import_settings.clone();
        self.save_metadata()?;
        Ok(settings)
    }

    pub fn rebuild_dependency_graph(&mut self) -> io::Result<BTreeMap<String, Vec<String>>> {
        self.scan()?;
        let mut graph = BTreeMap::new();
        let project_files = walk_files(&self.project_root)?;
        let asset_paths = self
            .assets
            .iter()
            .map(|(path, record)| (path.clone(), record.guid.clone()))
            .collect::<Vec<_>>();
        for file in project_files {
            let relative = file
                .strip_prefix(&self.project_root)
                .unwrap_or(&file)
                .to_string_lossy()
                .to_string();
            if !(relative.ends_with(".scene")
                || relative.ends_with(".prefab")
                || relative.ends_with(".json")
                || relative.ends_with(".mfgraph")
                || relative.ends_with(".ron"))
            {
                continue;
            }
            let text = fs::read_to_string(&file).unwrap_or_default();
            let deps: Vec<String> = asset_paths
                .iter()
                .filter(|(asset, guid)| {
                    if asset == &relative {
                        return false;
                    }
                    let stem = Path::new(asset)
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or(asset);
                    text.contains(asset.as_str())
                        || (!guid.is_empty() && text.contains(guid))
                        || text.contains(&format!("\"{stem}\""))
                        || text.contains(&format!("'{stem}'"))
                })
                .map(|(path, _)| path.clone())
                .collect();
            if !deps.is_empty() {
                graph.insert(relative, deps);
            }
        }

        for record in self.assets.values_mut() {
            record.dependencies.clear();
        }
        for (owner, deps) in &graph {
            if let Some(record) = self.assets.get_mut(owner) {
                record.dependencies = deps.clone();
            }
        }
        self.save_metadata()?;
        Ok(graph)
    }

    pub fn dependencies_for(&self, relative_path: &str) -> Vec<String> {
        self.assets
            .get(relative_path)
            .map(|record| record.dependencies.clone())
            .unwrap_or_default()
    }

    pub fn reverse_dependencies_for(&self, relative_path: &str) -> Vec<String> {
        self.assets
            .iter()
            .filter_map(|(path, record)| {
                if record.dependencies.iter().any(|dep| dep == relative_path) {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Builds a real directed dependency graph for ordering imports/builds and
    /// surfacing cycles in the editor.
    pub fn dependency_report(&self) -> AssetDependencyReport {
        let mut graph = DiGraph::<String, ()>::new();
        let mut nodes = BTreeMap::<String, NodeIndex>::new();
        for path in self.assets.keys() {
            nodes.insert(path.clone(), graph.add_node(path.clone()));
        }
        let mut edge_count = 0;
        for (owner, record) in &self.assets {
            let Some(&owner_node) = nodes.get(owner) else {
                continue;
            };
            for dependency in &record.dependencies {
                let Some(&dependency_node) = nodes.get(dependency) else {
                    continue;
                };
                // dependency -> consumer ensures topological build order.
                graph.add_edge(dependency_node, owner_node, ());
                edge_count += 1;
            }
        }

        let cycles = kosaraju_scc(&graph)
            .into_iter()
            .filter(|component| component.len() > 1)
            .map(|component| {
                let mut paths = component
                    .into_iter()
                    .map(|node| graph[node].clone())
                    .collect::<Vec<_>>();
                paths.sort();
                paths
            })
            .collect::<Vec<_>>();
        let build_order = toposort(&graph, None)
            .unwrap_or_else(|_| graph.node_indices().collect())
            .into_iter()
            .map(|node| graph[node].clone())
            .collect();
        AssetDependencyReport {
            build_order,
            cycles,
            edge_count,
        }
    }
}

pub fn stable_guid(seed: &str) -> String {
    format!("{:016x}", hash_seed(seed))
}

/// Generates a new persistent identity. Use this at asset creation time and
/// store the result; do not recompute it after a rename.
pub fn new_asset_guid(seed: &str) -> String {
    make_guid(seed)
}

fn make_guid(seed: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    format!("{:016x}{:016x}", hash_seed(seed), now)
}

fn unique_content_match(
    previous: &BTreeMap<String, AssetRecord>,
    scanned: &ScannedAsset,
    live_paths: &BTreeSet<String>,
) -> Option<String> {
    if scanned.content_hash.is_empty() {
        return None;
    }
    let mut matches = previous
        .iter()
        .filter(|(path, record)| {
            !live_paths.contains(*path)
                && record.content_hash == scanned.content_hash
                && record.size_bytes == scanned.size_bytes
                && record.asset_type == scanned.asset_type
        })
        .map(|(path, _)| path.clone());
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn validate_relative_asset_path(path: &str) -> io::Result<()> {
    let path = Path::new(path);
    let invalid = path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
    if invalid || path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "asset path must stay inside the project",
        ));
    }
    Ok(())
}

fn file_content_hash(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = 1469598103934665603_u64;
    let mut length = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        length = length.saturating_add(read as u64);
        for byte in &buffer[..read] {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
    }
    Ok(format!("{hash:016x}-{length:016x}"))
}

fn hash_seed(seed: &str) -> u64 {
    let mut hash: u64 = 1469598103934665603;
    for byte in seed.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn detect_asset_type(path: &Path) -> String {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    if filename.ends_with(".spritesheet.json") {
        return "SpriteSheet".to_string();
    }
    if filename.ends_with(".spriteframes") || filename.ends_with(".spriteframes.json") {
        return "SpriteFrames2D".to_string();
    }
    if filename.ends_with(".anim2d.json") {
        return "AnimationBlueprint2D".to_string();
    }
    if filename.ends_with(".flipbook.json") {
        return "FlipbookAnimation2D".to_string();
    }
    if filename.ends_with(".atlas.json") {
        return "Atlas".to_string();
    }
    if filename.ends_with(".sprite.json") {
        return "Sprite".to_string();
    }
    if filename.ends_with(".sound.json") {
        return "Audio".to_string();
    }
    if filename.ends_with(".audio.json") {
        return "AudioEvent".to_string();
    }
    if filename.ends_with(".material.json") {
        return "Material".to_string();
    }
    if filename.ends_with(".particles.json") {
        return "ParticlePreset".to_string();
    }
    if filename.ends_with(".shader.json") {
        return "Shader".to_string();
    }
    if filename.ends_with(".mpscene.json") {
        return "PackedScene2D".to_string();
    }
    let path_text = path.to_string_lossy().to_lowercase();
    let image_like = matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "aseprite"
    );
    if image_like
        && (path_text.contains("/textures/")
            || path_text.contains("\\textures\\")
            || looks_like_material_texture(&filename))
    {
        return "Texture2D".to_string();
    }
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "aseprite" => "Sprite",
        "svg" | "svgz" => "VectorImage",
        "wav" | "mp3" | "ogg" | "flac" => "Audio",
        "prefab" => "Prefab",
        "scene" => "Scene",
        "luau" => "LuauScript",
        "mfgraph" => "VisualGraph",
        "mat" | "material" => "Material",
        "glsl" | "wgsl" => "Shader",
        "ttf" | "otf" => "Font",
        "tmx" | "tsx" => "Tilemap",
        "mp4" | "mov" | "webm" => "Video",
        "json" | "txt" | "csv" | "ron" | "toml" => "Data",
        _ => "Asset",
    }
    .to_string()
}

fn default_import_settings(path: &Path) -> Value {
    match detect_asset_type(path).as_str() {
        "Sprite" | "VectorImage" => json!({
            "filter": "nearest",
            "include_in_build": true,
            "pixels_per_unit": 32,
            "generate_mips": false,
            "atlas": null,
            "rasterize_on_import": detect_asset_type(path) == "VectorImage",
        }),
        "Texture2D" => json!({
            "filter": "linear",
            "include_in_build": true,
            "generate_mips": true,
            "usage": "material_slot",
            "slot_hint": texture_slot_hint(path),
            "srgb": texture_slot_hint(path) == "base_color",
        }),
        "Audio" => json!({
            "stream": false,
            "include_in_build": true,
            "bus": "SFX",
            "spatial": false,
            "preload": true,
        }),
        "AudioEvent" => json!({
            "runtime": "kira",
            "include_in_build": true,
            "bus": "SFX",
            "preload": true,
        }),
        "Material" => json!({"shader": "sprite_default", "include_in_build": true}),
        "ParticlePreset" => json!({"include_in_build": true, "runtime": "particle_system"}),
        "LuauScript" => json!({"runtime": "luau", "include_in_build": true, "hot_reload": true}),
        "VisualGraph" => json!({"runtime": "rust_visual_graph", "include_in_build": true}),
        "SpriteSheet" => json!({"include_in_build": true, "grid": {"w": 32, "h": 32}}),
        "SpriteFrames2D" => json!({"include_in_build": true, "fps": 8, "runtime": "flipbook"}),
        "PackedScene2D" => json!({"include_in_build": true, "editable_children": true}),
        "AnimationBlueprint2D" | "FlipbookAnimation2D" => {
            json!({"include_in_build": true, "runtime": "animator2d"})
        }
        "Atlas" => json!({"include_in_build": true, "filter": "nearest"}),
        "Shader" => json!({"target": "macroquad", "include_in_build": true}),
        _ => json!({"include_in_build": true}),
    }
}

fn walk_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if ignored_dir(&path) {
                continue;
            }
            files.extend(walk_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

fn ignored_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    matches!(name, ".git" | "target" | "builds" | ".cache")
}

fn file_stats(path: &Path) -> (u64, u64) {
    let Ok(metadata) = fs::metadata(path) else {
        return (0, 0);
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    (metadata.len(), modified)
}

fn labels_for(path: &Path, asset_type: &str) -> Vec<String> {
    let mut labels = vec![asset_type.to_lowercase()];
    let path_text = path.to_string_lossy().to_lowercase();
    if path_text.contains("/prefabs/") {
        labels.push("prefab-library".to_string());
    }
    if asset_type == "PackedScene2D" {
        labels.push("scene-instancing".to_string());
        labels.push("2d".to_string());
    }
    if path_text.contains("/visual_graphs/") {
        labels.push("in-engine-code".to_string());
    }
    if asset_type == "LuauScript" {
        labels.push("gameplay-code".to_string());
        labels.push("hot-reload".to_string());
    }
    if path_text.contains("/sprites/") {
        labels.push("rendering".to_string());
    }
    if asset_type == "SpriteFrames2D"
        || asset_type == "AnimationBlueprint2D"
        || asset_type == "FlipbookAnimation2D"
    {
        labels.push("animation".to_string());
        labels.push("2d".to_string());
    }
    if asset_type == "Texture2D" {
        labels.push("material-texture".to_string());
        labels.push(texture_slot_hint(path).to_string());
    }
    if path_text.contains("/audio/") {
        labels.push("audio".to_string());
    }
    labels.sort();
    labels.dedup();
    labels
}

fn compatibility_for(path: &Path, asset_type: &str, size_bytes: u64) -> Vec<String> {
    let mut notes = Vec::new();
    if asset_type == "Sprite" && size_bytes > 8 * 1024 * 1024 {
        notes.push("Large sprite: consider atlas/import compression".to_string());
    }
    if asset_type == "Texture2D" && size_bytes > 16 * 1024 * 1024 {
        notes.push("Large texture: consider mips/compression before shipping".to_string());
    }
    if asset_type == "Audio" && size_bytes > 16 * 1024 * 1024 {
        notes.push("Large audio: enable streaming for runtime builds".to_string());
    }
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.contains(' '))
    {
        notes.push(
            "Filename contains spaces; build tools handle it, but GUID references are safer"
                .to_string(),
        );
    }
    notes
}

fn looks_like_material_texture(filename: &str) -> bool {
    [
        "normal",
        "nrm",
        "roughness",
        "rough",
        "metallic",
        "metalness",
        "emissive",
        "emission",
        "glow",
        "albedo",
        "basecolor",
        "base_color",
    ]
    .iter()
    .any(|needle| filename.contains(needle))
}

fn texture_slot_hint(path: &Path) -> &'static str {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    if filename.contains("normal") || filename.contains("nrm") {
        "normal"
    } else if filename.contains("roughness") || filename.contains("rough") {
        "roughness"
    } else if filename.contains("metallic")
        || filename.contains("metalness")
        || filename.contains("metal")
    {
        "metallic"
    } else if filename.contains("emissive")
        || filename.contains("emission")
        || filename.contains("glow")
    {
        "emissive"
    } else {
        "base_color"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    fn test_project(label: &str) -> PathBuf {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "miniforge_asset_identity_{label}_{}_{}",
            std::process::id(),
            sequence
        ))
    }

    fn record(path: &str, dependencies: &[&str]) -> AssetRecord {
        AssetRecord {
            guid: stable_guid(path),
            relative_path: path.to_string(),
            name: path.to_string(),
            asset_type: "Data".to_string(),
            size_bytes: 0,
            modified_unix: 0,
            content_hash: String::new(),
            import_settings: json!({}),
            labels: Vec::new(),
            compatibility: Vec::new(),
            dependencies: dependencies
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        }
    }

    #[test]
    fn petgraph_report_orders_dependencies_and_detects_cycles() {
        let database = AssetDatabase {
            root: PathBuf::new(),
            project_root: PathBuf::new(),
            metadata_file: PathBuf::new(),
            assets: BTreeMap::from([
                ("a".to_string(), record("a", &["b"])),
                ("b".to_string(), record("b", &[])),
                ("c".to_string(), record("c", &["d"])),
                ("d".to_string(), record("d", &["c"])),
            ]),
        };
        let report = database.dependency_report();
        assert_eq!(report.edge_count, 3);
        assert_eq!(report.cycles, vec![vec!["c".to_string(), "d".to_string()]]);
        assert!(report.build_order.contains(&"a".to_string()));
        assert!(report.build_order.contains(&"b".to_string()));
    }

    #[test]
    fn managed_move_preserves_guid_and_updates_resolver() {
        let project = test_project("managed_move");
        let paths = AssetTools::ensure_project_folders(&project).expect("project folders");
        let source = paths.assets.join("hero.txt");
        fs::write(&source, "persistent hero asset").expect("source asset");
        let mut database = AssetDatabase::new(&paths.assets, &project).expect("database");
        database.scan().expect("initial scan");
        let original_guid = database.assets["assets/hero.txt"].guid.clone();

        let target = database
            .move_asset("assets/hero.txt", "assets/characters/hero.txt")
            .expect("managed move");

        assert!(target.is_file());
        assert!(!source.exists());
        assert_eq!(
            database.path_for_guid(&original_guid),
            Some("assets/characters/hero.txt")
        );
        let reloaded = AssetDatabase::new(&paths.assets, &project).expect("reload database");
        assert_eq!(
            reloaded.path_for_guid(&original_guid),
            Some("assets/characters/hero.txt")
        );
        let metadata = AssetTools::read_json(&database.metadata_file).expect("metadata");
        assert_eq!(metadata["format"], ASSET_METADATA_FORMAT);
        assert_eq!(metadata["schema_version"], ASSET_METADATA_SCHEMA_VERSION);
        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn scan_reconciles_unique_external_rename_by_content() {
        let project = test_project("external_move");
        let paths = AssetTools::ensure_project_folders(&project).expect("project folders");
        let source = paths.assets.join("portrait.dat");
        let target = paths.assets.join("portraits").join("hero.dat");
        fs::write(&source, "unique portrait bytes 4adf8").expect("source asset");
        let mut database = AssetDatabase::new(&paths.assets, &project).expect("database");
        database.scan().expect("initial scan");
        let original_guid = database.assets["assets/portrait.dat"].guid.clone();

        fs::create_dir_all(target.parent().expect("target parent")).expect("target folder");
        fs::rename(&source, &target).expect("external rename");
        database.scan().expect("reconcile scan");

        assert_eq!(
            database.path_for_guid(&original_guid),
            Some("assets/portraits/hero.dat")
        );
        assert_eq!(
            database
                .assets
                .values()
                .filter(|record| record.guid == original_guid)
                .count(),
            1
        );
        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn scan_duplicate_does_not_steal_the_original_assets_guid() {
        let project = test_project("external_duplicate");
        let paths = AssetTools::ensure_project_folders(&project).expect("project folders");
        let source = paths.sprites.join("hero.png");
        fs::write(&source, "same bytes for duplicate").expect("source asset");
        let mut database = AssetDatabase::new(&paths.assets, &project).expect("database");
        database.scan().expect("initial scan");
        let original_guid = database.assets["assets/sprites/hero.png"].guid.clone();

        let duplicate = paths.audio.join("hero.png");
        fs::copy(&source, &duplicate).expect("duplicate asset");
        database.scan().expect("duplicate scan");

        assert_eq!(
            database.assets["assets/sprites/hero.png"].guid, original_guid,
            "a byte-identical copy in an earlier sort position must not inherit the source identity"
        );
        assert_ne!(database.assets["assets/audio/hero.png"].guid, original_guid);
        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn future_asset_metadata_schema_is_rejected() {
        let project = test_project("future_schema");
        let paths = AssetTools::ensure_project_folders(&project).expect("project folders");
        let metadata = project.join("project/asset_metadata.json");
        AssetTools::write_json(
            &metadata,
            &json!({
                "format": ASSET_METADATA_FORMAT,
                "schema_version": ASSET_METADATA_SCHEMA_VERSION + 1,
                "assets": {},
            }),
        )
        .expect("future metadata");

        let error = AssetDatabase::new(&paths.assets, &project).expect_err("future schema");
        assert!(error.to_string().contains("newer than supported"));
        let _ = fs::remove_dir_all(project);
    }
}
