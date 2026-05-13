use serde_json::Value;

use crate::engine::asset_database::{AssetDatabase, AssetRecord};
use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPreviewKind {
    Image,
    Audio,
    Material,
    Prefab,
    Scene,
    VisualGraph,
    RhaiScript,
    Data,
    SpriteSheet,
    Atlas,
    Other,
}

impl AssetPreviewKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "Image",
            Self::Audio => "Audio",
            Self::Material => "Material",
            Self::Prefab => "Prefab",
            Self::Scene => "Scene",
            Self::VisualGraph => "VisualGraph",
            Self::RhaiScript => "RhaiScript",
            Self::Data => "Data",
            Self::SpriteSheet => "SpriteSheet",
            Self::Atlas => "Atlas",
            Self::Other => "Asset",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetPreview {
    pub guid: String,
    pub path: String,
    pub name: String,
    pub asset_type: String,
    pub kind: AssetPreviewKind,
    pub labels: Vec<String>,
    pub import_settings: Value,
    pub dependencies: Vec<String>,
    pub reverse_dependencies: Vec<String>,
    pub warnings: Vec<String>,
    pub details: Vec<String>,
}

impl AssetPreview {
    pub fn from_record(database: &AssetDatabase, record: &AssetRecord) -> Self {
        let mut warnings = record.compatibility.clone();
        let absolute = database.project_root.join(&record.relative_path);
        if !absolute.exists() {
            warnings.push("Asset file missing on disk".to_string());
        }
        if !record
            .import_settings
            .get("include_in_build")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        {
            warnings.push("Excluded from runtime builds".to_string());
        }

        let mut details = Vec::new();
        details.push(format!(
            "{} KB",
            (record.size_bytes as f64 / 1024.0).ceil() as u64
        ));
        details.push(format!("modified {}", record.modified_unix));
        if let Some(filter) = record.import_settings.get("filter").and_then(Value::as_str) {
            details.push(format!("filter {filter}"));
        }
        if let Some(bus) = record.import_settings.get("bus").and_then(Value::as_str) {
            details.push(format!("bus {bus}"));
        }
        if record.asset_type == "SpriteSheet" {
            if let Ok(meta) = AssetTools::read_json(&absolute) {
                let n = meta
                    .get("slices")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                details.push(format!("slices {n}"));
            }
        } else if record.asset_type == "Atlas" {
            if let Ok(meta) = AssetTools::read_json(&absolute) {
                let n = meta
                    .get("regions")
                    .and_then(Value::as_object)
                    .map(|o| o.len())
                    .unwrap_or(0);
                details.push(format!("regions {n}"));
            }
        } else if record.asset_type == "Audio"
            && absolute.extension().and_then(|e| e.to_str()) == Some("wav")
        {
            let cache = crate::engine::asset_importers::WaveformCache::new(&database.project_root);
            if let Ok(peaks) = cache.peaks_for_wav(&absolute, 48) {
                details.push(format!(
                    "waveform preview {} buckets (max {:.2})",
                    peaks.len(),
                    peaks.iter().copied().fold(0.0f32, f32::max)
                ));
            }
        }
        if let Some(shader) = record.import_settings.get("shader").and_then(Value::as_str) {
            details.push(format!("shader {shader}"));
        }

        Self {
            guid: record.guid.clone(),
            path: record.relative_path.clone(),
            name: record.name.clone(),
            asset_type: record.asset_type.clone(),
            kind: preview_kind(&record.asset_type),
            labels: record.labels.clone(),
            import_settings: record.import_settings.clone(),
            dependencies: database.dependencies_for(&record.relative_path),
            reverse_dependencies: database.reverse_dependencies_for(&record.relative_path),
            warnings,
            details,
        }
    }

    pub fn build_included(&self) -> bool {
        self.import_settings
            .get("include_in_build")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }
}

impl AssetDatabase {
    pub fn preview(&self, relative_path: &str) -> Option<AssetPreview> {
        self.assets
            .get(relative_path)
            .map(|record| AssetPreview::from_record(self, record))
    }
}

fn preview_kind(asset_type: &str) -> AssetPreviewKind {
    match asset_type {
        "Sprite" => AssetPreviewKind::Image,
        "Audio" => AssetPreviewKind::Audio,
        "AudioEvent" => AssetPreviewKind::Audio,
        "Material" | "Shader" => AssetPreviewKind::Material,
        "Prefab" => AssetPreviewKind::Prefab,
        "Scene" => AssetPreviewKind::Scene,
        "VisualGraph" => AssetPreviewKind::VisualGraph,
        "RhaiScript" => AssetPreviewKind::RhaiScript,
        "Data" | "Tilemap" | "Font" | "Video" => AssetPreviewKind::Data,
        "SpriteSheet" => AssetPreviewKind::SpriteSheet,
        "Atlas" => AssetPreviewKind::Atlas,
        _ => AssetPreviewKind::Other,
    }
}
