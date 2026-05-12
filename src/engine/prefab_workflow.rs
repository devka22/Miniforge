use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct PrefabWorkflow;

impl PrefabWorkflow {
    pub fn apply_selected_to_prefab(entity: &mut GameObject) {
        entity.is_prefab_instance = true;
    }

    pub fn revert_selected_prefab(entity: &mut GameObject) {
        entity.is_prefab_instance = false;
    }
}
