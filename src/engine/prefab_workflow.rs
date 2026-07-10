use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct PrefabWorkflow;

impl PrefabWorkflow {
    pub fn apply_selected_to_prefab(entity: &mut GameObject) {
        entity.is_prefab_instance = true;
    }

    pub fn mark_prefab_instance(
        entity: &mut GameObject,
        source: impl Into<String>,
        guid: Option<String>,
    ) {
        entity.prefab_source = Some(source.into());
        entity.prefab_guid = guid;
        entity.is_prefab_instance = true;
    }

    pub fn revert_selected_prefab(entity: &mut GameObject) {
        entity.is_prefab_instance = false;
        entity.prefab_source = None;
        entity.prefab_guid = None;
    }
}
