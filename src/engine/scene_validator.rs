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
            }
        }
        self.errors.is_empty()
    }
}
