use std::collections::BTreeSet;

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct SelectionManager {
    pub selected_ids: BTreeSet<u64>,
}

impl SelectionManager {
    pub fn select(&mut self, entity: &mut GameObject, additive: bool) {
        if !additive {
            self.selected_ids.clear();
        }
        entity.selected = true;
        self.selected_ids.insert(entity.id);
    }

    pub fn clear(&mut self, entities: &mut [GameObject]) {
        for entity in entities {
            entity.selected = false;
        }
        self.selected_ids.clear();
    }

    pub fn selected<'a>(&self, entities: &'a [GameObject]) -> Vec<&'a GameObject> {
        entities
            .iter()
            .filter(|entity| self.selected_ids.contains(&entity.id))
            .collect()
    }
}
