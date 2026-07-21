use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiProjectContext {
    pub project_path: Option<PathBuf>,
    pub active_scene: String,
    pub entities: Vec<AiEntityContext>,
    pub assets: Vec<AiAssetContext>,
    pub scripts: Vec<String>,
    pub visual_graphs: Vec<String>,
    pub prefabs: Vec<String>,
    pub input_actions: Vec<String>,
    pub physics_summary: AiPhysicsContext,
    pub logs: Vec<String>,
    pub config_summary: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiEntityContext {
    pub id: u64,
    pub name: String,
    pub scene: String,
    pub components: Vec<AiComponentContext>,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
    pub tags: Vec<String>,
    pub layer: String,
    pub visible: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiComponentContext {
    pub component_type: String,
    pub enabled: bool,
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiAssetContext {
    pub guid: String,
    pub relative_path: String,
    pub name: String,
    pub asset_type: String,
    pub size_bytes: u64,
    pub labels: Vec<String>,
    pub dependency_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiPhysicsContext {
    pub rigidbody_count: usize,
    pub collider_count: usize,
    pub trigger_count: usize,
    pub layers: Vec<String>,
}

impl Default for AiProjectContext {
    fn default() -> Self {
        Self {
            project_path: None,
            active_scene: "active".to_string(),
            entities: Vec::new(),
            assets: Vec::new(),
            scripts: Vec::new(),
            visual_graphs: Vec::new(),
            prefabs: Vec::new(),
            input_actions: Vec::new(),
            physics_summary: AiPhysicsContext::default(),
            logs: Vec::new(),
            config_summary: BTreeMap::new(),
        }
    }
}

impl AiProjectContext {
    pub fn entity_by_name(&self, name: &str) -> Option<&AiEntityContext> {
        self.entities
            .iter()
            .find(|entity| entity.name.eq_ignore_ascii_case(name))
    }

    pub fn entity_by_id(&self, id: u64) -> Option<&AiEntityContext> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    pub fn summary(&self) -> String {
        format!(
            "{} entities, {} assets, {} scripts, {} prefabs, {} graphs",
            self.entities.len(),
            self.assets.len(),
            self.scripts.len(),
            self.prefabs.len(),
            self.visual_graphs.len()
        )
    }

    pub fn component_count(&self, component_type: &str) -> usize {
        self.entities
            .iter()
            .filter(|entity| {
                entity
                    .components
                    .iter()
                    .any(|component| component.component_type == component_type)
            })
            .count()
    }

    pub fn rebuild_indexes(&mut self) {
        let mut children = self
            .entities
            .iter()
            .map(|entity| (entity.id, Vec::new()))
            .collect::<BTreeMap<_, _>>();
        for entity in &self.entities {
            if let Some(parent) = entity.parent {
                children.entry(parent).or_default().push(entity.id);
            }
        }
        for entity in &mut self.entities {
            entity.children = children.remove(&entity.id).unwrap_or_default();
        }
        self.physics_summary = AiPhysicsContext {
            rigidbody_count: self.component_count("Rigidbody2D"),
            collider_count: self.component_count("Collider2D"),
            trigger_count: self.component_count("Trigger2D"),
            layers: sorted_layers(&self.entities),
        };
    }

    #[cfg(feature = "editor_core")]
    pub fn from_editor_core(
        core: &crate::engine::editor_core::EditorCore,
    ) -> Result<Self, crate::engine::editor_core::EditorCoreError> {
        let mut context = Self {
            project_path: core.project_path().map(|path| path.to_path_buf()),
            ..Self::default()
        };

        let entity_count = core.entity_count()?;
        let mut rows = Vec::new();
        for index in 0..entity_count {
            rows.push(core.entity_at(index)?);
        }
        for row in rows {
            let fields = core.inspector_fields(row.id)?;
            let mut components = BTreeMap::<String, AiComponentContext>::new();
            for field in fields {
                let value = serde_json::from_str::<Value>(&field.value_json).unwrap_or(Value::Null);
                let component =
                    components
                        .entry(field.target.clone())
                        .or_insert_with(|| AiComponentContext {
                            component_type: field.target.clone(),
                            enabled: true,
                            properties: BTreeMap::new(),
                        });
                if field.key == "enabled" {
                    component.enabled = value.as_bool().unwrap_or(true);
                }
                component.properties.insert(field.key, value);
            }
            context.entities.push(AiEntityContext {
                id: row.id,
                name: row.name,
                scene: context.active_scene.clone(),
                components: components.into_values().collect(),
                parent: row.parent_id,
                children: Vec::new(),
                tags: tags_from_row(&row.tag),
                layer: row.layer,
                visible: row.visible,
                enabled: row.enabled,
            });
        }

        let asset_count = core.asset_count()?;
        for index in 0..asset_count {
            let row = core.asset_at(index)?;
            let relative_path = row.relative_path.clone();
            let asset_type = row.asset_type.clone();
            if relative_path.ends_with(".luau") || asset_type == "LuauScript" {
                context.scripts.push(relative_path.clone());
            }
            if relative_path.ends_with(".mfgraph") || asset_type == "BlueprintGraph2D" {
                context.visual_graphs.push(relative_path.clone());
            }
            if relative_path.ends_with(".prefab") || asset_type == "Prefab2D" {
                context.prefabs.push(relative_path.clone());
            }
            context.assets.push(AiAssetContext {
                guid: row.guid,
                relative_path,
                name: row.name,
                asset_type,
                size_bytes: row.size_bytes,
                labels: row.labels,
                dependency_count: row.dependency_count,
            });
        }
        context.rebuild_indexes();
        Ok(context)
    }
}

fn sorted_layers(entities: &[AiEntityContext]) -> Vec<String> {
    let mut layers = entities
        .iter()
        .map(|entity| entity.layer.clone())
        .filter(|layer| !layer.trim().is_empty())
        .collect::<Vec<_>>();
    layers.sort();
    layers.dedup();
    layers
}

#[cfg(feature = "editor_core")]
fn tags_from_row(tag: &str) -> Vec<String> {
    if tag.trim().is_empty() || tag == "Untagged" {
        Vec::new()
    } else {
        vec![tag.to_string()]
    }
}
