use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::engine::asset_database::AssetDatabase;
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetOperation2D {
    Reimport,
    RenameSafe { to: String },
    MoveSafe { to: String },
    DeleteSafe,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetRegistryReport2D {
    pub missing_assets: Vec<String>,
    pub duplicate_guids: Vec<String>,
    pub unused_assets: Vec<String>,
}

pub fn supported_asset_types() -> Vec<&'static str> {
    vec![
        "textures",
        "sprites",
        "sprite_atlases",
        "tilemaps",
        "tilesets",
        "sounds",
        "music",
        "scenes",
        "prefabs",
        "luau_scripts",
        "visual_graphs",
        "materials",
        "ui_documents",
        "timelines",
        "json_configs",
        "fonts",
        "shaders",
    ]
}

pub fn analyze_asset_database(database: &AssetDatabase) -> AssetRegistryReport2D {
    let mut by_guid: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (path, record) in &database.assets {
        by_guid
            .entry(record.guid.clone())
            .or_default()
            .push(path.clone());
    }
    let duplicate_guids = by_guid
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(guid, _)| guid)
        .collect();
    let known = database.assets.keys().cloned().collect::<BTreeSet<_>>();
    let mut referenced = BTreeSet::new();
    let mut missing_assets = Vec::new();
    for record in database.assets.values() {
        for dependency in &record.dependencies {
            referenced.insert(dependency.clone());
            if !known.contains(dependency) {
                missing_assets.push(dependency.clone());
            }
        }
    }
    missing_assets.sort();
    missing_assets.dedup();
    let unused_assets = known
        .difference(&referenced)
        .filter(|asset| !asset.ends_with(".scene") && !asset.ends_with("project.json"))
        .cloned()
        .collect();
    AssetRegistryReport2D {
        missing_assets,
        duplicate_guids,
        unused_assets,
    }
}

impl AssetRegistryReport2D {
    pub fn to_validation_report(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        for asset in &self.missing_assets {
            report.warning(
                "missing_asset",
                asset.clone(),
                format!("Asset faltante en dependencias: {asset}"),
            );
        }
        for guid in &self.duplicate_guids {
            report.error(
                "duplicate_guid",
                guid.clone(),
                format!("GUID duplicado: {guid}"),
            );
        }
        for asset in &self.unused_assets {
            report.warning(
                "unused_asset",
                asset.clone(),
                format!("Asset no usado detectado: {asset}"),
            );
        }
        report
    }
}
