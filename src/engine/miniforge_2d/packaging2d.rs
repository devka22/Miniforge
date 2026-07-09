use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_database::AssetDatabase;
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PackageProfile2D {
    Debug,
    Release,
    Shipping,
    WebFuture,
    MacosAppFuture,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageManifest2D {
    pub game_name: String,
    pub profile: PackageProfile2D,
    pub start_scene: String,
    #[serde(default)]
    pub used_assets: Vec<String>,
    #[serde(default)]
    pub missing_assets: Vec<String>,
    #[serde(default)]
    pub validation_errors: Vec<String>,
    #[serde(default)]
    pub settings: Value,
}

impl PackageManifest2D {
    pub fn from_asset_database(
        game_name: impl Into<String>,
        profile: PackageProfile2D,
        start_scene: impl Into<String>,
        database: &AssetDatabase,
    ) -> Self {
        let used_assets = database.assets.keys().cloned().collect::<Vec<_>>();
        Self {
            game_name: game_name.into(),
            profile,
            start_scene: start_scene.into(),
            used_assets,
            missing_assets: Vec::new(),
            validation_errors: Vec::new(),
            settings: json!({
                "include_debug_tools": matches!(profile, PackageProfile2D::Debug),
                "shipping": matches!(profile, PackageProfile2D::Shipping),
                "asset_manifest": "runtime_manifest.json"
            }),
        }
    }

    pub fn validate(&mut self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        if self.game_name.trim().is_empty() {
            report.error("package_game_name", "game_name", "Package sin game_name.");
        }
        if self.start_scene.trim().is_empty() {
            report.error(
                "package_start_scene",
                "start_scene",
                "Package sin start_scene.",
            );
        }
        for missing in &self.missing_assets {
            report.warning(
                "package_missing_asset",
                missing.clone(),
                format!("Asset usado por build no existe: {missing}"),
            );
        }
        self.validation_errors = report
            .issues
            .iter()
            .filter(|issue| {
                issue.severity
                    == crate::engine::miniforge_2d::validation::ValidationSeverity2D::Error
            })
            .map(|issue| issue.message.clone())
            .collect();
        report
    }

    pub fn to_value(&self) -> Value {
        json!(self)
    }
}

pub fn minimal_package_manifest() -> PackageManifest2D {
    PackageManifest2D {
        game_name: "MiniForge2DDemo".to_string(),
        profile: PackageProfile2D::Debug,
        start_scene: "saves/scenes/main.scene".to_string(),
        used_assets: vec![
            "assets/sprites/player.png".to_string(),
            "assets/ui/hud.ui2d.json".to_string(),
            "scripts/visual_graphs/BP_PlayerPawn2D.mfgraph".to_string(),
        ],
        missing_assets: Vec::new(),
        validation_errors: Vec::new(),
        settings: json!({
            "runtime": "miniforge_runtime",
            "manifest": "runtime_manifest.json",
            "validate_assets": true
        }),
    }
}
