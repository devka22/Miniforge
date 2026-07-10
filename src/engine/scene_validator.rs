use crate::engine::scene_signal::SceneSignalBus;
use crate::engine::scene_tree::SceneTreeIndex;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct SceneValidator {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SceneValidator {
    pub fn validate_entities(&mut self, entities: &[GameObject]) -> bool {
        self.errors.clear();
        self.warnings.clear();
        let tree = SceneTreeIndex::build(entities);
        let mut ids = std::collections::BTreeSet::new();
        for entity in entities {
            if !ids.insert(entity.id) {
                self.errors
                    .push(format!("Duplicate entity id {}", entity.id));
            }
            if entity.name.trim().is_empty() {
                self.errors
                    .push(format!("Entity {} has empty name", entity.id));
            }
            if entity.get_component("Transform").is_none() {
                self.warnings.push(format!(
                    "Entity {} ({}) no tiene Transform; se usaran coordenadas del entity.",
                    entity.id, entity.name
                ));
            }
            if entity.is_prefab_instance && entity.prefab_source.as_deref().unwrap_or("").is_empty()
            {
                self.warnings.push(format!(
                    "Prefab instance {} ({}) no tiene prefab_source.",
                    entity.id, entity.name
                ));
            }
            for component in &entity.components {
                if component.component_type.trim().is_empty() {
                    self.errors.push(format!(
                        "Entity {} ({}) tiene un componente sin tipo.",
                        entity.id, entity.name
                    ));
                }
                if component.enabled && component.data.is_empty() {
                    self.warnings.push(format!(
                        "Componente {} en {} no tiene data serializable.",
                        component.component_type, entity.name
                    ));
                }
                for (key, value) in &component.data {
                    if is_node_path_key(key)
                        && let Some(path) = value.as_str()
                        && !path.trim().is_empty()
                        && tree.resolve_path(Some(entity.id), path).is_none()
                    {
                        self.warnings.push(format!(
                            "NodePath no resuelto en {}.{}.{}: {}",
                            entity.name, component.component_type, key, path
                        ));
                    }
                }
            }
        }

        self.warnings.extend(tree.warnings.clone());
        let signal_bus = SceneSignalBus::from_entities(entities, &tree);
        let signal_report = signal_bus.validate();
        for missing in signal_report.missing_targets {
            self.errors
                .push(format!("SignalEmitter target no resuelto: {missing}"));
        }
        for empty in signal_report.empty_methods {
            self.errors
                .push(format!("SignalEmitter method vacio: {empty}"));
        }
        self.errors.is_empty()
    }
}

fn is_node_path_key(key: &str) -> bool {
    matches!(
        key,
        "node_path" | "target_path" | "root_path" | "owner_path" | "parent_path"
    )
}
