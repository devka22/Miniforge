use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

/// Import configuration kept next to a source asset. The source file remains
/// the artist-owned truth; generated runtime files can always be rebuilt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportProfile2D {
    pub importer: String,
    pub importer_version: u32,
    pub resource_type: String,
    pub preset: String,
    #[serde(default)]
    pub options: BTreeMap<String, Value>,
}

impl ImportProfile2D {
    pub fn pixel_art() -> Self {
        Self {
            importer: "texture2d".to_string(),
            importer_version: 1,
            resource_type: "Texture2D".to_string(),
            preset: "Pixel Art 2D".to_string(),
            options: BTreeMap::from([
                ("filter".to_string(), json!(false)),
                ("mipmaps".to_string(), json!(false)),
                ("compression".to_string(), json!("lossless")),
                ("color_space".to_string(), json!("srgb")),
                ("alpha_mode".to_string(), json!("straight")),
            ]),
        }
    }

    pub fn smooth_sprite() -> Self {
        Self {
            preset: "Smooth Sprite 2D".to_string(),
            options: BTreeMap::from([
                ("filter".to_string(), json!(true)),
                ("mipmaps".to_string(), json!(true)),
                ("compression".to_string(), json!("lossless")),
                ("color_space".to_string(), json!("srgb")),
                ("alpha_mode".to_string(), json!("premultiplied")),
            ]),
            ..Self::pixel_art()
        }
    }

    pub fn ui_texture() -> Self {
        Self {
            preset: "UI Texture".to_string(),
            options: BTreeMap::from([
                ("filter".to_string(), json!(true)),
                ("mipmaps".to_string(), json!(false)),
                ("compression".to_string(), json!("lossless")),
                ("color_space".to_string(), json!("srgb")),
                ("atlas_group".to_string(), json!("ui")),
            ]),
            ..Self::pixel_art()
        }
    }

    pub fn audio_event() -> Self {
        Self {
            importer: "audio2d".to_string(),
            importer_version: 1,
            resource_type: "Audio2D".to_string(),
            preset: "Game Audio".to_string(),
            options: BTreeMap::from([
                ("stream".to_string(), json!(false)),
                ("normalize".to_string(), json!(false)),
                ("loop".to_string(), json!(false)),
                ("quality".to_string(), json!(0.8)),
            ]),
        }
    }

    pub fn for_source(path: impl AsRef<Path>) -> Self {
        match path
            .as_ref()
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "png" | "jpg" | "jpeg" | "webp" | "bmp" => Self::pixel_art(),
            "wav" | "ogg" | "mp3" | "flac" => Self::audio_event(),
            extension => Self {
                importer: "copy".to_string(),
                importer_version: 1,
                resource_type: "DataAsset2D".to_string(),
                preset: "Keep Source".to_string(),
                options: BTreeMap::from([("extension".to_string(), json!(extension))]),
            },
        }
    }

    pub fn apply_options(&mut self, changes: &BTreeMap<String, Value>) -> usize {
        let mut changed = 0;
        for (key, value) in changes {
            if self.options.get(key) != Some(value) {
                self.options.insert(key.clone(), value.clone());
                changed += 1;
            }
        }
        changed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetImportMetadata2D {
    pub source_path: String,
    pub source_fingerprint: String,
    pub imported_path: String,
    pub profile: ImportProfile2D,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub generated_files: Vec<String>,
    #[serde(default)]
    pub last_import_unix: u64,
    #[serde(default)]
    pub profile_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReimportReason2D {
    NewSource,
    SourceChanged,
    ImporterChanged,
    MissingGeneratedFile,
    DependencyChanged,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReimportJob2D {
    pub source_path: String,
    pub imported_path: String,
    pub reasons: BTreeSet<ReimportReason2D>,
    pub priority: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReimportPlan2D {
    pub jobs: Vec<ReimportJob2D>,
    pub unchanged: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportPipeline2D {
    #[serde(default)]
    pub imports: BTreeMap<String, AssetImportMetadata2D>,
    #[serde(default)]
    pub changed_dependencies: BTreeSet<String>,
}

impl ImportPipeline2D {
    pub fn register_source(
        &mut self,
        source_path: impl AsRef<Path>,
        imported_path: impl AsRef<Path>,
        profile: Option<ImportProfile2D>,
    ) -> io::Result<&AssetImportMetadata2D> {
        let source_path = source_path.as_ref();
        let source_key = normalize_path(source_path);
        let metadata = AssetImportMetadata2D {
            source_path: source_key.clone(),
            source_fingerprint: fingerprint_file(source_path)?,
            imported_path: normalize_path(imported_path.as_ref()),
            profile: profile.unwrap_or_else(|| ImportProfile2D::for_source(source_path)),
            dependencies: Vec::new(),
            generated_files: Vec::new(),
            last_import_unix: 0,
            profile_dirty: true,
        };
        self.imports.insert(source_key.clone(), metadata);
        Ok(self
            .imports
            .get(&source_key)
            .expect("inserted import metadata"))
    }

    pub fn add_dependency(&mut self, source_path: &str, dependency: &str) -> bool {
        let Some(metadata) = self.imports.get_mut(source_path) else {
            return false;
        };
        if metadata.dependencies.iter().any(|item| item == dependency) {
            return false;
        }
        metadata.dependencies.push(dependency.to_string());
        metadata.dependencies.sort();
        true
    }

    pub fn mark_dependency_changed(&mut self, dependency: impl Into<String>) {
        self.changed_dependencies.insert(dependency.into());
    }

    pub fn update_profiles_batch(
        &mut self,
        source_paths: &[String],
        changes: &BTreeMap<String, Value>,
    ) -> usize {
        let mut changed = 0;
        for path in source_paths {
            if let Some(metadata) = self.imports.get_mut(path) {
                let profile_changes = metadata.profile.apply_options(changes);
                if profile_changes > 0 {
                    metadata.profile_dirty = true;
                    changed += profile_changes;
                }
            }
        }
        changed
    }

    pub fn plan_reimport(&self) -> ReimportPlan2D {
        let mut plan = ReimportPlan2D::default();
        for (source, metadata) in &self.imports {
            let mut reasons = BTreeSet::new();
            let source_path = Path::new(source);
            match fingerprint_file(source_path) {
                Ok(fingerprint) if fingerprint != metadata.source_fingerprint => {
                    reasons.insert(ReimportReason2D::SourceChanged);
                }
                Err(_) => {
                    reasons.insert(ReimportReason2D::NewSource);
                }
                _ => {}
            }
            if metadata.last_import_unix == 0 {
                reasons.insert(ReimportReason2D::NewSource);
            }
            if metadata.profile_dirty && metadata.last_import_unix > 0 {
                reasons.insert(ReimportReason2D::ImporterChanged);
            }
            if metadata
                .generated_files
                .iter()
                .any(|path| !Path::new(path).exists())
            {
                reasons.insert(ReimportReason2D::MissingGeneratedFile);
            }
            if metadata
                .dependencies
                .iter()
                .any(|dependency| self.changed_dependencies.contains(dependency))
            {
                reasons.insert(ReimportReason2D::DependencyChanged);
            }

            if reasons.is_empty() {
                plan.unchanged.push(source.clone());
            } else {
                let priority = if reasons.contains(&ReimportReason2D::MissingGeneratedFile) {
                    100
                } else if reasons.contains(&ReimportReason2D::SourceChanged) {
                    80
                } else {
                    50
                };
                plan.jobs.push(ReimportJob2D {
                    source_path: source.clone(),
                    imported_path: metadata.imported_path.clone(),
                    reasons,
                    priority,
                });
            }
        }
        plan.jobs.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.source_path.cmp(&right.source_path))
        });
        plan.unchanged.sort();
        plan
    }

    pub fn dependency_impact(&self, dependency: &str) -> Vec<String> {
        self.imports
            .iter()
            .filter_map(|(source, metadata)| {
                metadata
                    .dependencies
                    .iter()
                    .any(|item| item == dependency)
                    .then_some(source.clone())
            })
            .collect()
    }

    pub fn complete_import(
        &mut self,
        source_path: &str,
        generated_files: Vec<String>,
        timestamp: u64,
    ) -> io::Result<()> {
        let metadata = self
            .imports
            .get_mut(source_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "source is not registered"))?;
        metadata.source_fingerprint = fingerprint_file(Path::new(source_path))?;
        metadata.generated_files = generated_files;
        metadata.last_import_unix = timestamp;
        metadata.profile_dirty = false;
        Ok(())
    }

    pub fn clear_dependency_changes(&mut self) {
        self.changed_dependencies.clear();
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        AssetTools::write_json(path, &serde_json::to_value(self).unwrap_or_default())
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        serde_json::from_value(AssetTools::read_json(path)?).map_err(io::Error::other)
    }

    pub fn sidecar_path(source_path: impl AsRef<Path>) -> PathBuf {
        let source = source_path.as_ref();
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("asset");
        source.with_file_name(format!("{file_name}.mfimport.json"))
    }
}

pub fn fingerprint_file(path: impl AsRef<Path>) -> io::Result<String> {
    let bytes = fs::read(path)?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/")
}
