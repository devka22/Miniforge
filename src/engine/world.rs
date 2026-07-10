use std::collections::{BTreeMap, BTreeSet};

use crate::engine::packed_scene::PackedScene2D;
use crate::engine::scene_signal::SceneSignalBus;
use crate::engine::scene_tree::SceneTreeIndex;
use crate::engine::spatial_index::{SpatialEntry, SpatialIndex};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldValidationReport {
    pub entity_count: usize,
    pub duplicate_ids: Vec<u64>,
    pub dangling_parent_ids: Vec<(u64, u64)>,
    pub hierarchy_cycles: Vec<Vec<u64>>,
}

impl WorldValidationReport {
    pub fn is_valid(&self) -> bool {
        self.duplicate_ids.is_empty()
            && self.dangling_parent_ids.is_empty()
            && self.hierarchy_cycles.is_empty()
    }
}

/// Canonical runtime owner for entities and their acceleration structures.
///
/// `units` keeps its legacy name during the compatibility window. It is the
/// only entity vector; the previous cloned `World.entities` snapshot has been
/// removed.
#[derive(Debug, Clone)]
pub struct RuntimeWorld {
    pub units: Vec<GameObject>,
    pub spatial_index: SpatialIndex,
    pub structural_revision: u64,
    pub indexed_revision: u64,
}

impl Default for RuntimeWorld {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl RuntimeWorld {
    pub fn new(units: Vec<GameObject>) -> Self {
        let mut world = Self {
            units,
            spatial_index: SpatialIndex::default(),
            structural_revision: 1,
            indexed_revision: 0,
        };
        world.rebuild_index();
        world
    }

    pub fn entities(&self) -> &[GameObject] {
        &self.units
    }

    pub fn entities_mut(&mut self) -> &mut [GameObject] {
        self.mark_changed();
        &mut self.units
    }

    pub fn entity(&self, entity_id: u64) -> Option<&GameObject> {
        self.units.iter().find(|entity| entity.id == entity_id)
    }

    pub fn entity_mut(&mut self, entity_id: u64) -> Option<&mut GameObject> {
        let index = self
            .units
            .iter()
            .position(|entity| entity.id == entity_id)?;
        self.mark_changed();
        self.units.get_mut(index)
    }

    pub fn replace_entities(&mut self, units: Vec<GameObject>) {
        self.units = units;
        self.mark_changed();
        self.rebuild_index();
    }

    pub fn push(&mut self, entity: GameObject) -> Result<u64, String> {
        if self.entity(entity.id).is_some() {
            return Err(format!("duplicate runtime entity id {}", entity.id));
        }
        let id = entity.id;
        self.units.push(entity);
        self.mark_changed();
        if let Some(entity) = self.units.last() {
            self.spatial_index.insert(entity);
            self.indexed_revision = self.structural_revision;
        }
        Ok(id)
    }

    pub fn remove(&mut self, entity_id: u64) -> Option<GameObject> {
        let index = self
            .units
            .iter()
            .position(|entity| entity.id == entity_id)?;
        let removed = self.units.remove(index);
        self.spatial_index.remove(entity_id);
        self.mark_changed();
        self.indexed_revision = self.structural_revision;
        Some(removed)
    }

    pub fn mark_changed(&mut self) {
        self.structural_revision = self.structural_revision.saturating_add(1);
    }

    pub fn rebuild_index(&mut self) {
        self.spatial_index.rebuild(&self.units);
        self.indexed_revision = self.structural_revision;
    }

    pub fn index_is_current(&self) -> bool {
        self.indexed_revision == self.structural_revision
    }

    pub fn query_radius(
        &self,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> Vec<SpatialEntry> {
        self.spatial_index.query_radius(x, y, radius, tag, layer)
    }

    pub fn scene_tree(&self) -> SceneTreeIndex {
        SceneTreeIndex::build(&self.units)
    }

    pub fn node_path_for(&self, entity_id: u64) -> Option<String> {
        self.scene_tree()
            .path_for(entity_id)
            .map(ToString::to_string)
    }

    pub fn resolve_node_path(&self, current_id: Option<u64>, path: &str) -> Option<u64> {
        self.scene_tree().resolve_path(current_id, path)
    }

    pub fn entities_in_group(&self, group: &str) -> Vec<u64> {
        self.scene_tree().ids_in_group(group)
    }

    pub fn signal_bus(&self) -> SceneSignalBus {
        let tree = self.scene_tree();
        SceneSignalBus::from_entities(&self.units, &tree)
    }

    pub fn pack_scene_from_root(&self, root_id: u64) -> Result<PackedScene2D, String> {
        PackedScene2D::pack_from_root(&self.units, root_id)
    }

    pub fn validate(&self) -> WorldValidationReport {
        let mut ids = BTreeSet::new();
        let mut duplicate_ids = Vec::new();
        for entity in &self.units {
            if !ids.insert(entity.id) {
                duplicate_ids.push(entity.id);
            }
        }
        duplicate_ids.sort_unstable();
        duplicate_ids.dedup();

        let mut dangling_parent_ids = self
            .units
            .iter()
            .filter_map(|entity| {
                let parent_id = entity.parent_id?;
                (!ids.contains(&parent_id)).then_some((entity.id, parent_id))
            })
            .collect::<Vec<_>>();
        dangling_parent_ids.sort_unstable();

        let parents = self
            .units
            .iter()
            .filter_map(|entity| entity.parent_id.map(|parent| (entity.id, parent)))
            .collect::<BTreeMap<_, _>>();
        let mut unique_cycles = BTreeSet::new();
        for start in parents.keys().copied() {
            let mut path = Vec::new();
            let mut positions = BTreeMap::new();
            let mut current = start;
            loop {
                if let Some(position) = positions.get(&current).copied() {
                    let mut cycle = path[position..].to_vec();
                    cycle.sort_unstable();
                    cycle.dedup();
                    if !cycle.is_empty() {
                        unique_cycles.insert(cycle);
                    }
                    break;
                }
                positions.insert(current, path.len());
                path.push(current);
                let Some(parent) = parents.get(&current).copied() else {
                    break;
                };
                current = parent;
            }
        }

        WorldValidationReport {
            entity_count: self.units.len(),
            duplicate_ids,
            dangling_parent_ids,
            hierarchy_cycles: unique_cycles.into_iter().collect(),
        }
    }
}

/// Compatibility name for older imports. New code should use `RuntimeWorld`.
#[deprecated(note = "use RuntimeWorld; cloned World snapshots were removed")]
pub type World = RuntimeWorld;

#[cfg(test)]
mod tests {
    use super::RuntimeWorld;
    use crate::entities::game_object::GameObject;

    #[test]
    fn runtime_world_owns_entities_and_keeps_index_consistent() {
        let near = GameObject::new(1.0, 1.0, Some("Near".to_string()));
        let near_id = near.id;
        let far = GameObject::new(40.0, 40.0, Some("Far".to_string()));
        let mut world = RuntimeWorld::new(vec![near, far]);

        assert!(world.index_is_current());
        assert_eq!(world.query_radius(0.0, 0.0, 4.0, None, None).len(), 1);
        world.entity_mut(near_id).expect("near entity").x = 30.0;
        assert!(!world.index_is_current());
        world.rebuild_index();
        assert!(world.query_radius(0.0, 0.0, 4.0, None, None).is_empty());
    }

    #[test]
    fn runtime_world_rejects_duplicate_push_and_reports_broken_hierarchy() {
        let parent = GameObject::new(0.0, 0.0, Some("Parent".to_string()));
        let mut child = GameObject::new(0.0, 0.0, Some("Child".to_string()));
        child.parent_id = Some(u64::MAX - 1);
        let duplicate = parent.clone();
        let mut world = RuntimeWorld::new(vec![parent, child]);

        assert!(world.push(duplicate).is_err());
        let report = world.validate();
        assert!(!report.is_valid());
        assert_eq!(report.dangling_parent_ids.len(), 1);
    }

    #[test]
    fn runtime_world_detects_parent_cycles() {
        let mut first = GameObject::new(0.0, 0.0, Some("First".to_string()));
        let mut second = GameObject::new(0.0, 0.0, Some("Second".to_string()));
        first.parent_id = Some(second.id);
        second.parent_id = Some(first.id);
        let world = RuntimeWorld::new(vec![first, second]);

        let report = world.validate();
        assert_eq!(report.hierarchy_cycles.len(), 1);
        assert!(!report.is_valid());
    }
}
