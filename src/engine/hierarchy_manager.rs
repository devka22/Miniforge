use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct HierarchyManager;

impl HierarchyManager {
    pub fn set_parent(child: &mut GameObject, parent: &GameObject) {
        child.parent_id = Some(parent.id);
        child.local_x = child.x - parent.x;
        child.local_y = child.y - parent.y;
    }

    pub fn clear_parent(child: &mut GameObject) {
        child.parent_id = None;
        child.local_x = 0.0;
        child.local_y = 0.0;
    }

    pub fn sync_child_world_transforms(entities: &mut [GameObject]) {
        let snapshot: Vec<(u64, f64, f64)> = entities.iter().map(|e| (e.id, e.x, e.y)).collect();
        for entity in entities {
            let Some(parent_id) = entity.parent_id else {
                continue;
            };
            if let Some((_, px, py)) = snapshot.iter().find(|(id, _, _)| *id == parent_id) {
                entity.x = px + entity.local_x;
                entity.y = py + entity.local_y;
                entity.sync_to_components();
            }
        }
    }
}
