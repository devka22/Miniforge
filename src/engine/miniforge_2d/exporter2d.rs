use serde::{Deserialize, Serialize};

use crate::engine::miniforge_2d::asset_registry2d::AssetRegistryReport2D;
use crate::engine::miniforge_2d::packaging2d::{PackageManifest2D, PackageProfile2D};
use crate::engine::miniforge_2d::validation::{ValidationReport2D, ValidationSeverity2D};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportLayout2D {
    pub profile: PackageProfile2D,
    pub game_name: String,
    pub root: String,
    pub runtime_manifest: String,
    pub build_info: String,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportValidation2D {
    pub blocked: bool,
    pub report: ValidationReport2D,
}

pub fn export_layout(profile: PackageProfile2D, game_name: &str) -> ExportLayout2D {
    let profile_dir = match profile {
        PackageProfile2D::Debug => "debug",
        PackageProfile2D::Release => "release",
    };
    let root = format!("build/{profile_dir}/{game_name}");
    ExportLayout2D {
        profile,
        game_name: game_name.to_string(),
        runtime_manifest: format!("{root}/runtime_manifest.json"),
        build_info: format!("{root}/build_info.json"),
        root,
        folders: ["assets", "scenes", "scripts", "graphs", "config"]
            .iter()
            .map(|folder| folder.to_string())
            .collect(),
    }
}

pub fn validate_before_export(
    manifest: &mut PackageManifest2D,
    assets: &AssetRegistryReport2D,
    plugins_available: bool,
) -> ExportValidation2D {
    let mut report = manifest.validate();
    report.merge(assets.to_validation_report());
    if !plugins_available {
        report.error(
            "missing_plugin",
            "plugins",
            "Hay plugins requeridos no disponibles.",
        );
    }
    let blocked = report
        .issues
        .iter()
        .any(|issue| issue.severity == ValidationSeverity2D::Error);
    ExportValidation2D { blocked, report }
}
