use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_database::{AssetDatabase, AssetRecord, stable_guid};
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentAsset2D {
    pub guid: String,
    pub path: String,
    pub name: String,
    pub asset_type: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub preview: AssetPreview2D,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub valid: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPreview2D {
    pub preview_type: String,
    pub thumbnail_path: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentFilter2D {
    pub search: String,
    pub asset_types: BTreeSet<String>,
    pub labels: BTreeSet<String>,
    pub include_invalid: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContentBrowserCatalog2D {
    pub assets: BTreeMap<String, ContentAsset2D>,
    #[serde(default)]
    pub view_mode: ContentBrowserViewMode2D,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentBrowserViewMode2D {
    #[default]
    Grid,
    List,
}

impl ContentBrowserCatalog2D {
    pub fn from_asset_database(database: &AssetDatabase) -> Self {
        Self {
            assets: database
                .assets
                .iter()
                .map(|(path, record)| (path.clone(), asset_from_record(record)))
                .collect(),
            view_mode: ContentBrowserViewMode2D::Grid,
        }
    }

    pub fn insert_json_asset(
        &mut self,
        path: impl Into<String>,
        asset_type: impl Into<String>,
        metadata: Value,
    ) -> String {
        let path = path.into();
        let asset_type = asset_type.into();
        let guid = stable_guid(&path);
        let name = std::path::Path::new(&path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("asset")
            .to_string();
        self.assets.insert(
            path.clone(),
            ContentAsset2D {
                guid: guid.clone(),
                path,
                name,
                asset_type,
                labels: Vec::new(),
                preview: AssetPreview2D {
                    preview_type: "json".to_string(),
                    thumbnail_path: None,
                    summary: "MiniForge2D JSON asset".to_string(),
                },
                dependencies: Vec::new(),
                valid: true,
                metadata,
            },
        );
        guid
    }

    pub fn filter(&self, filter: &ContentFilter2D) -> Vec<&ContentAsset2D> {
        let search = filter.search.to_lowercase();
        self.assets
            .values()
            .filter(|asset| filter.include_invalid || asset.valid)
            .filter(|asset| {
                filter.asset_types.is_empty() || filter.asset_types.contains(&asset.asset_type)
            })
            .filter(|asset| {
                filter.labels.is_empty()
                    || asset
                        .labels
                        .iter()
                        .any(|label| filter.labels.contains(label))
            })
            .filter(|asset| {
                search.is_empty()
                    || asset.name.to_lowercase().contains(&search)
                    || asset.path.to_lowercase().contains(&search)
                    || asset.guid.contains(&search)
            })
            .collect()
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        let paths = self.assets.keys().cloned().collect::<BTreeSet<_>>();
        let mut guids = BTreeSet::new();
        for asset in self.assets.values() {
            if !guids.insert(asset.guid.clone()) {
                report.error(
                    "duplicate_guid",
                    asset.path.clone(),
                    format!("GUID duplicado en Content Browser: {}", asset.guid),
                );
            }
            for dependency in &asset.dependencies {
                if !paths.contains(dependency) {
                    report.warning(
                        "missing_dependency",
                        asset.path.clone(),
                        format!("Dependencia rota: {dependency}"),
                    );
                }
            }
        }
        report
    }

    pub fn to_value(&self) -> Value {
        json!({
            "runtime": "miniforge_content_browser_2d",
            "view_mode": self.view_mode,
            "assets": self.assets
        })
    }
}

pub fn supported_content_asset_types() -> Vec<&'static str> {
    crate::engine::miniforge_2d::asset_registry2d::supported_asset_types()
}

pub fn supported_content_operations() -> Vec<&'static str> {
    vec![
        "folder_view",
        "grid_view",
        "list_view",
        "search",
        "filter_by_type",
        "labels",
        "preview",
        "guid",
        "path",
        "size",
        "dependencies",
        "reverse_dependencies",
        "drag_drop_to_scene",
        "double_click_open",
        "create_asset",
        "rename",
        "duplicate",
        "delete",
        "reimport",
        "broken_references",
        "unused_assets",
        "missing_assets",
    ]
}

fn asset_from_record(record: &AssetRecord) -> ContentAsset2D {
    ContentAsset2D {
        guid: record.guid.clone(),
        path: record.relative_path.clone(),
        name: record.name.clone(),
        asset_type: normalize_type(&record.asset_type),
        labels: record.labels.clone(),
        preview: preview_for(record),
        dependencies: record.dependencies.clone(),
        valid: true,
        metadata: json!({
            "size_bytes": record.size_bytes,
            "modified_unix": record.modified_unix,
            "import_settings": record.import_settings,
            "compatibility": record.compatibility,
        }),
    }
}

fn normalize_type(asset_type: &str) -> String {
    match asset_type {
        "sprite" | "png" | "image" => "Sprite2D",
        "prefab" => "Prefab2D",
        "scene" => "Scene2D",
        "mfgraph" | "visual_graph" => "BlueprintGraph2D",
        "json" => "DataAsset2D",
        other => other,
    }
    .to_string()
}

fn preview_for(record: &AssetRecord) -> AssetPreview2D {
    let preview_type = match normalize_type(&record.asset_type).as_str() {
        "Sprite2D" => "sprite",
        "BlueprintGraph2D" => "graph",
        "Scene2D" => "scene",
        "Prefab2D" => "prefab",
        _ => "metadata",
    };
    AssetPreview2D {
        preview_type: preview_type.to_string(),
        thumbnail_path: None,
        summary: format!("{} | {} bytes", record.asset_type, record.size_bytes),
    }
}
