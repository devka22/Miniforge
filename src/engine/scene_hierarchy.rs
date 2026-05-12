use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct SceneHierarchy {
    pub expanded: std::collections::BTreeSet<u64>,
}

impl SceneHierarchy {
    pub fn roots<'a>(&self, entities: &'a [GameObject]) -> Vec<&'a GameObject> {
        entities
            .iter()
            .filter(|entity| entity.parent_id.is_none())
            .collect()
    }

    pub fn children<'a>(&self, entities: &'a [GameObject], parent_id: u64) -> Vec<&'a GameObject> {
        entities
            .iter()
            .filter(|entity| entity.parent_id == Some(parent_id))
            .collect()
    }
}
