use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct SceneValidator {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SceneValidator {
    pub fn validate_entities(&mut self, entities: &[GameObject]) -> bool {
        self.errors.clear();
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
        }
        self.errors.is_empty()
    }
}
