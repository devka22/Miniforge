use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

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
        AssetTools::write_json(&self.metadata_file, &json!({"assets": self.assets}))
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
        let mut discovered = BTreeSet::new();
        for root in scan_roots {
            if !root.exists() {
                continue;
            }
            for path in walk_files(&root)? {
                let relative = path
                    .strip_prefix(&self.project_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                discovered.insert(relative.clone());
                let stats = file_stats(&path);
                let detected_type = detect_asset_type(&path);
                let labels = labels_for(&path, &detected_type);
                let compatibility = compatibility_for(&path, &detected_type, stats.0);
                let record = self
                    .assets
                    .entry(relative.clone())
                    .or_insert_with(|| AssetRecord {
                        guid: make_guid(&relative),
                        relative_path: relative.clone(),
                        name: path
                            .file_stem()
                            .and_then(|value| value.to_str())
                            .unwrap_or("asset")
                            .to_string(),
                        asset_type: detected_type.clone(),
                        size_bytes: stats.0,
                        modified_unix: stats.1,
                        import_settings: default_import_settings(&path),
                        labels: labels.clone(),
                        compatibility: compatibility.clone(),
                        dependencies: Vec::new(),
                    });
                record.relative_path = relative;
                record.asset_type = detected_type;
                record.size_bytes = stats.0;
                record.modified_unix = stats.1;
                record.labels = labels;
                record.compatibility = compatibility;
            }
        }
        self.assets
            .retain(|relative, _| discovered.contains(relative));
        self.save_metadata()
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
        let asset_paths: Vec<String> = self.assets.keys().cloned().collect();
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
                .filter(|asset| {
                    let stem = Path::new(asset)
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or(asset);
                    text.contains(asset.as_str()) || text.contains(stem)
                })
                .cloned()
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
}

pub fn stable_guid(seed: &str) -> String {
    format!("{:016x}", hash_seed(seed))
}

fn make_guid(seed: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    format!("{:016x}{:016x}", hash_seed(seed), now)
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
    if filename.ends_with(".atlas.json") {
        return "Atlas".to_string();
    }
    if filename.ends_with(".sprite.json") {
        return "Sprite".to_string();
    }
    if filename.ends_with(".sound.json") {
        return "Audio".to_string();
    }
    if filename.ends_with(".material.json") {
        return "Material".to_string();
    }
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "aseprite" => "Sprite",
        "wav" | "mp3" | "ogg" | "flac" => "Audio",
        "prefab" => "Prefab",
        "scene" => "Scene",
        "mfgraph" => "VisualGraph",
        "mat" | "material" => "Material",
        "glsl" | "wgsl" => "Shader",
        "ttf" | "otf" => "Font",
        "tmx" | "tsx" => "Tilemap",
        "mp4" | "mov" | "webm" => "Video",
        "py" => "LegacyScript",
        "json" | "txt" | "csv" | "ron" | "toml" => "Data",
        _ => "Asset",
    }
    .to_string()
}

fn default_import_settings(path: &Path) -> Value {
    match detect_asset_type(path).as_str() {
        "Sprite" => json!({
            "filter": "nearest",
            "include_in_build": true,
            "pixels_per_unit": 32,
            "generate_mips": false,
            "atlas": null,
        }),
        "Audio" => json!({
            "stream": false,
            "include_in_build": true,
            "bus": "SFX",
            "spatial": false,
            "preload": true,
        }),
        "Material" => json!({"shader": "sprite_default", "include_in_build": true}),
        "VisualGraph" => json!({"runtime": "rust_visual_graph", "include_in_build": true}),
        "SpriteSheet" => json!({"include_in_build": true, "grid": {"w": 32, "h": 32}}),
        "Atlas" => json!({"include_in_build": true, "filter": "nearest"}),
        "Shader" => json!({"target": "macroquad", "include_in_build": true}),
        "LegacyScript" => json!({"runtime": "python_legacy", "include_in_build": false}),
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
    matches!(
        name,
        "__pycache__" | ".git" | "target" | "builds" | ".pytest_cache" | ".mypy_cache"
    )
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
    if path_text.contains("/visual_graphs/") {
        labels.push("in-engine-code".to_string());
    }
    if path_text.contains("/sprites/") {
        labels.push("rendering".to_string());
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
    if asset_type == "LegacyScript" {
        notes.push("Legacy Python asset: migrate to .mfgraph or Rust runtime hooks".to_string());
    }
    if asset_type == "Sprite" && size_bytes > 8 * 1024 * 1024 {
        notes.push("Large sprite: consider atlas/import compression".to_string());
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
