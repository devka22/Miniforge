use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::miniforge_2d::module_catalog;
use crate::engine::plugin_manager::{PluginLoadPlan, PluginManager};
use crate::engine::project_validator::ProjectValidator;
use crate::engine::resource_manager::{ResourceManager, ResourceReport};
use crate::engine::runtime_config::{HardwareProfile, RuntimeConfig, RuntimeTuning};
use crate::engine::service_registry::EngineServiceRegistry;
use crate::engine::system_audit::SystemReadinessReport;
use crate::engine::update_093::Engine093UpgradePlan;
use crate::engine::update_0934::Engine0934FoundationPlan;
use crate::engine::version::ENGINE_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineBackendPlan {
    pub engine_version: String,
    pub project_name: String,
    pub project_path: String,
    #[serde(default)]
    pub service_startup_order: Vec<String>,
    #[serde(default)]
    pub service_health: BTreeMap<String, usize>,
    #[serde(default)]
    pub service_issues: Vec<String>,
    pub plugins: PluginLoadPlan,
    pub resources: ResourceReport,
    pub runtime_tuning: RuntimeTuning,
    pub hardware_profile: HardwareProfile,
    #[serde(default)]
    pub feature_modules: Vec<BackendFeatureModule>,
    #[serde(default)]
    pub validation_errors: Vec<String>,
    #[serde(default)]
    pub validation_warnings: Vec<String>,
    #[serde(default)]
    pub recommendations: Vec<String>,
    pub system_audit: SystemReadinessReport,
    #[serde(default)]
    pub update_093: Engine093UpgradePlan,
    #[serde(default)]
    pub update_0934: Engine0934FoundationPlan,
    pub editor_ready: bool,
    pub runtime_ready: bool,
    pub export_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendFeatureModule {
    pub name: String,
    pub priority: usize,
    pub asset_extension: String,
    #[serde(default)]
    pub component_types: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EngineBackend;

impl EngineBackend {
    pub fn plan_project(project_path: impl AsRef<Path>) -> io::Result<EngineBackendPlan> {
        let project_path = project_path.as_ref();
        let project_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("MiniForgeGame")
            .to_string();

        let registry = EngineServiceRegistry::default_miniforge_2d();
        let service_issues = registry.validate();
        let service_health = registry
            .health_summary()
            .into_iter()
            .map(|(health, count)| (format!("{health:?}"), count))
            .collect::<BTreeMap<_, _>>();

        let mut plugins = PluginManager::new(project_path);
        let plugin_plan = plugins.load_plan()?;

        let resources = ResourceManager::scan_project_resources(project_path)?.report();

        let hardware_profile = HardwareProfile::detect();
        let runtime_tuning = read_runtime_tuning(project_path);
        let mut validator = ProjectValidator::default();
        validator.validate(project_path);
        let system_audit = SystemReadinessReport::audit_project(project_path)?;
        let update_093 = Engine093UpgradePlan::current();
        let update_0934 = Engine0934FoundationPlan::current();

        let mut recommendations = runtime_tuning.warnings();
        recommendations.extend(runtime_tuning.hardware_recommendations(&hardware_profile));
        for blocked in &plugin_plan.blocked_plugins {
            recommendations.push(format!(
                "Plugin {} bloqueado por {} ({})",
                blocked.plugin, blocked.dependency, blocked.reason
            ));
        }
        if !resources.duplicates.is_empty() {
            recommendations.push(format!(
                "{} nombres de recursos duplicados detectados; usa rutas/GUIDs para referencias",
                resources.duplicates.len()
            ));
        }
        if resources.counts.get("script").copied().unwrap_or(0) == 0
            && resources.counts.get("visual_graph").copied().unwrap_or(0) == 0
        {
            recommendations.push(
                "No hay scripts Luau ni visual graphs: agrega logica runtime antes de exportar una demo jugable"
                    .to_string(),
            );
        }
        for action in system_audit.top_actions(8) {
            recommendations.push(format!("System audit: {action}"));
        }
        recommendations.sort();
        recommendations.dedup();

        let editor_ready = validator.errors.is_empty()
            && service_issues.is_empty()
            && system_audit.total_score >= 50;
        let runtime_ready = editor_ready && plugin_plan.blocked_plugins.is_empty();
        let export_ready = runtime_ready && system_audit.total_score >= 60;

        Ok(EngineBackendPlan {
            engine_version: ENGINE_VERSION.to_string(),
            project_name,
            project_path: project_path.display().to_string(),
            service_startup_order: registry.startup_order(),
            service_health,
            service_issues,
            plugins: plugin_plan,
            resources,
            runtime_tuning,
            hardware_profile,
            feature_modules: module_catalog()
                .into_iter()
                .map(|module| BackendFeatureModule {
                    name: module.name,
                    priority: module.priority,
                    asset_extension: module.asset_extension,
                    component_types: module.component_types,
                })
                .collect(),
            validation_errors: validator.errors,
            validation_warnings: validator.warnings,
            recommendations,
            system_audit,
            update_093,
            update_0934,
            editor_ready,
            runtime_ready,
            export_ready,
        })
    }
}

impl EngineBackendPlan {
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|error| {
            json!({
                "engine_version": self.engine_version,
                "project_name": self.project_name,
                "serialization_error": error.to_string(),
            })
        })
    }

    pub fn complex_game_ready(&self) -> bool {
        self.runtime_ready
            && self.runtime_tuning.complex_game_ready()
            && self
                .feature_modules
                .iter()
                .any(|module| module.name == "Massive World 2D")
    }
}

fn read_runtime_tuning(project_path: &Path) -> RuntimeTuning {
    let path = AssetTools::get_project_paths(project_path)
        .settings
        .join("runtime_config.json");
    let data = AssetTools::read_json(path).unwrap_or_else(|_| RuntimeConfig::default_data());
    RuntimeTuning::from_value(&data)
}
