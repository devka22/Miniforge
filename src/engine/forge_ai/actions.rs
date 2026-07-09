use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::component::default_component;
use crate::engine::forge_ai::{AiError, AiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AiAction {
    CreateEntity {
        action_id: String,
        name: String,
        x: f64,
        y: f64,
        components: Vec<String>,
        tags: Vec<String>,
    },
    DeleteEntity {
        action_id: String,
        entity_id: u64,
        require_confirmation: bool,
    },
    DuplicateEntity {
        action_id: String,
        source_id: u64,
        clone_name: Option<String>,
    },
    AddComponent {
        action_id: String,
        entity_id: Option<u64>,
        entity_name: Option<String>,
        component_type: String,
        properties: BTreeMap<String, Value>,
    },
    RemoveComponent {
        action_id: String,
        entity_id: u64,
        component_type: String,
    },
    SetComponentProperty {
        action_id: String,
        entity_id: Option<u64>,
        entity_name: Option<String>,
        component_type: String,
        key: String,
        value: Value,
    },
    CreatePrefab {
        action_id: String,
        entity_id: Option<u64>,
        entity_name: Option<String>,
        prefab_name: String,
    },
    InstantiatePrefab {
        action_id: String,
        relative_path: String,
        x: f64,
        y: f64,
    },
    CreateScene {
        action_id: String,
        scene_name: String,
    },
    ModifyScene {
        action_id: String,
        scene_name: String,
        description: String,
    },
    CreateLuauScript {
        action_id: String,
        relative_path: String,
        source: String,
        attach_to_entity_name: Option<String>,
    },
    ModifyLuauScript {
        action_id: String,
        relative_path: String,
        source: String,
    },
    CreateVisualGraph {
        action_id: String,
        relative_path: String,
        graph: Value,
    },
    ModifyVisualGraph {
        action_id: String,
        relative_path: String,
        graph: Value,
    },
    ImportAsset {
        action_id: String,
        source_path: String,
        destination_path: String,
    },
    ConfigureInputAction {
        action_id: String,
        action_name: String,
        binding: String,
    },
    ConfigurePhysicsLayer {
        action_id: String,
        first_layer: String,
        second_layer: String,
        enabled: bool,
    },
    RunProject {
        action_id: String,
    },
    RunTests {
        action_id: String,
        suites: Vec<String>,
    },
    AnalyzePerformance {
        action_id: String,
    },
    ValidateProject {
        action_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiActionPreview {
    pub action_id: String,
    pub action_type: String,
    pub summary: String,
    pub affected_files: Vec<String>,
    pub reversible: bool,
}

impl AiAction {
    pub fn action_id(&self) -> &str {
        match self {
            Self::CreateEntity { action_id, .. }
            | Self::DeleteEntity { action_id, .. }
            | Self::DuplicateEntity { action_id, .. }
            | Self::AddComponent { action_id, .. }
            | Self::RemoveComponent { action_id, .. }
            | Self::SetComponentProperty { action_id, .. }
            | Self::CreatePrefab { action_id, .. }
            | Self::InstantiatePrefab { action_id, .. }
            | Self::CreateScene { action_id, .. }
            | Self::ModifyScene { action_id, .. }
            | Self::CreateLuauScript { action_id, .. }
            | Self::ModifyLuauScript { action_id, .. }
            | Self::CreateVisualGraph { action_id, .. }
            | Self::ModifyVisualGraph { action_id, .. }
            | Self::ImportAsset { action_id, .. }
            | Self::ConfigureInputAction { action_id, .. }
            | Self::ConfigurePhysicsLayer { action_id, .. }
            | Self::RunProject { action_id }
            | Self::RunTests { action_id, .. }
            | Self::AnalyzePerformance { action_id }
            | Self::ValidateProject { action_id } => action_id,
        }
    }

    pub fn action_type(&self) -> &'static str {
        match self {
            Self::CreateEntity { .. } => "CreateEntity",
            Self::DeleteEntity { .. } => "DeleteEntity",
            Self::DuplicateEntity { .. } => "DuplicateEntity",
            Self::AddComponent { .. } => "AddComponent",
            Self::RemoveComponent { .. } => "RemoveComponent",
            Self::SetComponentProperty { .. } => "SetComponentProperty",
            Self::CreatePrefab { .. } => "CreatePrefab",
            Self::InstantiatePrefab { .. } => "InstantiatePrefab",
            Self::CreateScene { .. } => "CreateScene",
            Self::ModifyScene { .. } => "ModifyScene",
            Self::CreateLuauScript { .. } => "CreateLuauScript",
            Self::ModifyLuauScript { .. } => "ModifyLuauScript",
            Self::CreateVisualGraph { .. } => "CreateVisualGraph",
            Self::ModifyVisualGraph { .. } => "ModifyVisualGraph",
            Self::ImportAsset { .. } => "ImportAsset",
            Self::ConfigureInputAction { .. } => "ConfigureInputAction",
            Self::ConfigurePhysicsLayer { .. } => "ConfigurePhysicsLayer",
            Self::RunProject { .. } => "RunProject",
            Self::RunTests { .. } => "RunTests",
            Self::AnalyzePerformance { .. } => "AnalyzePerformance",
            Self::ValidateProject { .. } => "ValidateProject",
        }
    }

    pub fn reversible(&self) -> bool {
        !matches!(
            self,
            Self::DeleteEntity { .. } | Self::RunProject { .. } | Self::AnalyzePerformance { .. }
        )
    }

    pub fn affected_files(&self) -> Vec<String> {
        match self {
            Self::CreateLuauScript { relative_path, .. }
            | Self::ModifyLuauScript { relative_path, .. }
            | Self::CreateVisualGraph { relative_path, .. }
            | Self::ModifyVisualGraph { relative_path, .. } => vec![relative_path.clone()],
            Self::CreatePrefab { prefab_name, .. } => vec![format!("assets/prefabs/{prefab_name}")],
            Self::InstantiatePrefab { relative_path, .. } => vec![relative_path.clone()],
            Self::ImportAsset {
                destination_path, ..
            } => vec![destination_path.clone()],
            _ => Vec::new(),
        }
    }

    pub fn validate(&self) -> AiResult<()> {
        if self.action_id().trim().is_empty() {
            return Err(AiError::validation("action_id is required"));
        }
        match self {
            Self::CreateEntity {
                name, components, ..
            } => {
                require_non_empty("entity name", name)?;
                for component in components {
                    validate_component_type(component)?;
                }
            }
            Self::AddComponent {
                component_type,
                entity_id,
                entity_name,
                ..
            } => {
                validate_entity_target(*entity_id, entity_name.as_deref())?;
                validate_component_type(component_type)?;
            }
            Self::SetComponentProperty {
                entity_id,
                entity_name,
                component_type,
                key,
                ..
            } => {
                validate_entity_target(*entity_id, entity_name.as_deref())?;
                if !matches!(component_type.as_str(), "Transform" | "Identity" | "Assets") {
                    validate_component_type(component_type)?;
                }
                require_non_empty("property key", key)?;
            }
            Self::CreateLuauScript {
                relative_path,
                source,
                ..
            }
            | Self::ModifyLuauScript {
                relative_path,
                source,
                ..
            } => {
                validate_project_relative_path(relative_path)?;
                if !relative_path.ends_with(".luau") {
                    return Err(AiError::validation(format!(
                        "Luau script must end with .luau: {relative_path}"
                    )));
                }
                require_non_empty("Luau source", source)?;
            }
            Self::CreatePrefab {
                entity_id,
                entity_name,
                prefab_name,
                ..
            } => {
                validate_entity_target(*entity_id, entity_name.as_deref())?;
                require_non_empty("prefab name", prefab_name)?;
            }
            Self::RunTests { suites, .. } if suites.is_empty() => {
                return Err(AiError::validation("at least one test suite is required"));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn preview(&self) -> AiActionPreview {
        AiActionPreview {
            action_id: self.action_id().to_string(),
            action_type: self.action_type().to_string(),
            summary: self.summary(),
            affected_files: self.affected_files(),
            reversible: self.reversible(),
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::CreateEntity {
                name, components, ..
            } => format!("Create entity {name} with {} components", components.len()),
            Self::DeleteEntity { entity_id, .. } => format!("Delete entity #{entity_id}"),
            Self::DuplicateEntity { source_id, .. } => format!("Duplicate entity #{source_id}"),
            Self::AddComponent {
                component_type,
                entity_id,
                entity_name,
                ..
            } => format!(
                "Add {component_type} to {}",
                target_label(*entity_id, entity_name.as_deref())
            ),
            Self::RemoveComponent {
                entity_id,
                component_type,
                ..
            } => format!("Remove {component_type} from #{entity_id}"),
            Self::SetComponentProperty {
                component_type,
                key,
                value,
                entity_id,
                entity_name,
                ..
            } => format!(
                "Set {component_type}.{key} on {} to {value}",
                target_label(*entity_id, entity_name.as_deref())
            ),
            Self::CreatePrefab {
                prefab_name,
                entity_id,
                entity_name,
                ..
            } => format!(
                "Save {} as prefab {prefab_name}",
                target_label(*entity_id, entity_name.as_deref())
            ),
            Self::InstantiatePrefab {
                relative_path,
                x,
                y,
                ..
            } => format!("Instantiate prefab {relative_path} at ({x}, {y})"),
            Self::CreateScene { scene_name, .. } => format!("Create scene {scene_name}"),
            Self::ModifyScene {
                scene_name,
                description,
                ..
            } => format!("Modify scene {scene_name}: {description}"),
            Self::CreateLuauScript { relative_path, .. } => {
                format!("Create Luau script {relative_path}")
            }
            Self::ModifyLuauScript { relative_path, .. } => {
                format!("Modify Luau script {relative_path}")
            }
            Self::CreateVisualGraph { relative_path, .. } => {
                format!("Create visual graph {relative_path}")
            }
            Self::ModifyVisualGraph { relative_path, .. } => {
                format!("Modify visual graph {relative_path}")
            }
            Self::ImportAsset {
                source_path,
                destination_path,
                ..
            } => format!("Import {source_path} to {destination_path}"),
            Self::ConfigureInputAction {
                action_name,
                binding,
                ..
            } => format!("Bind input action {action_name} to {binding}"),
            Self::ConfigurePhysicsLayer {
                first_layer,
                second_layer,
                enabled,
                ..
            } => format!("Set physics collision {first_layer}<->{second_layer} to {enabled}"),
            Self::RunProject { .. } => "Run project".to_string(),
            Self::RunTests { suites, .. } => format!("Run AI test suites: {}", suites.join(", ")),
            Self::AnalyzePerformance { .. } => "Analyze performance".to_string(),
            Self::ValidateProject { .. } => "Validate project".to_string(),
        }
    }
}

pub fn validate_project_relative_path(path: &str) -> AiResult<()> {
    require_non_empty("relative path", path)?;
    if path.starts_with('/')
        || path.starts_with('\\')
        || path.contains("..")
        || path.contains(':')
        || path.contains('\0')
    {
        return Err(AiError::validation(format!(
            "path must stay inside the project: {path}"
        )));
    }
    Ok(())
}

fn validate_entity_target(entity_id: Option<u64>, entity_name: Option<&str>) -> AiResult<()> {
    if entity_id.is_none() && entity_name.unwrap_or_default().trim().is_empty() {
        return Err(AiError::validation("entity target is required"));
    }
    Ok(())
}

fn validate_component_type(component_type: &str) -> AiResult<()> {
    require_non_empty("component type", component_type)?;
    if default_component(component_type).is_none() {
        return Err(AiError::validation(format!(
            "unknown component type: {component_type}"
        )));
    }
    Ok(())
}

fn require_non_empty(label: &str, value: &str) -> AiResult<()> {
    if value.trim().is_empty() {
        return Err(AiError::validation(format!("{label} is required")));
    }
    Ok(())
}

fn target_label(entity_id: Option<u64>, entity_name: Option<&str>) -> String {
    entity_id
        .map(|id| format!("#{id}"))
        .or_else(|| entity_name.map(|name| format!("'{name}'")))
        .unwrap_or_else(|| "<unresolved>".to_string())
}
