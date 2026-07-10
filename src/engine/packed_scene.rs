use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::entity_id::generate_entity_id;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackedScene2D {
    pub format: String,
    pub schema_version: u64,
    pub root_id: u64,
    pub root_name: String,
    pub entities: Vec<GameObject>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackedSceneInstantiateOptions {
    pub root_x: f64,
    pub root_y: f64,
    #[serde(default)]
    pub name_prefix: Option<String>,
    #[serde(default)]
    pub preserve_scene_name: bool,
}

impl Default for PackedSceneInstantiateOptions {
    fn default() -> Self {
        Self {
            root_x: 0.0,
            root_y: 0.0,
            name_prefix: None,
            preserve_scene_name: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackedSceneInstance {
    pub root_id: u64,
    pub id_map: BTreeMap<u64, u64>,
    pub entities: Vec<GameObject>,
}

impl PackedScene2D {
    pub const FORMAT: &'static str = "miniforge.packed-scene-2d";
    pub const SCHEMA_VERSION: u64 = 1;

    pub fn pack_from_root(entities: &[GameObject], root_id: u64) -> Result<Self, String> {
        let root = entities
            .iter()
            .find(|entity| entity.id == root_id)
            .ok_or_else(|| format!("root entity not found: {root_id}"))?;
        let descendants = collect_descendants(entities, root_id);
        let packed_ids = descendants.iter().copied().collect::<BTreeSet<_>>();
        let mut packed_entities = entities
            .iter()
            .filter(|entity| packed_ids.contains(&entity.id))
            .cloned()
            .collect::<Vec<_>>();
        packed_entities.sort_by_key(|entity| descendants.iter().position(|id| *id == entity.id));

        Ok(Self {
            format: Self::FORMAT.to_string(),
            schema_version: Self::SCHEMA_VERSION,
            root_id,
            root_name: root.name.clone(),
            entities: packed_entities,
            metadata: BTreeMap::new(),
        })
    }

    pub fn instantiate(&self, options: PackedSceneInstantiateOptions) -> PackedSceneInstance {
        let mut id_map = BTreeMap::new();
        for entity in &self.entities {
            id_map.insert(entity.id, generate_entity_id());
        }

        let (source_root_x, source_root_y) = self
            .entities
            .iter()
            .find(|entity| entity.id == self.root_id)
            .map(|entity| (entity.x, entity.y))
            .unwrap_or((0.0, 0.0));
        let offset_x = options.root_x - source_root_x;
        let offset_y = options.root_y - source_root_y;

        let mut entities = Vec::new();
        let mut root_id = 0;
        for source in &self.entities {
            let mut entity = source.clone();
            let old_id = entity.id;
            entity.id = id_map[&old_id];
            if old_id == self.root_id {
                root_id = entity.id;
                entity.parent_id = None;
            } else {
                entity.parent_id = entity
                    .parent_id
                    .and_then(|parent| id_map.get(&parent).copied());
            }
            entity.x += offset_x;
            entity.y += offset_y;
            if !options.preserve_scene_name {
                entity.scene_name = None;
            }
            if let Some(prefix) = options
                .name_prefix
                .as_deref()
                .filter(|prefix| !prefix.is_empty())
            {
                entity.name = format!("{prefix}{}", entity.name);
            }
            entity.sync_to_components();
            entities.push(entity);
        }

        PackedSceneInstance {
            root_id,
            id_map,
            entities,
        }
    }
}

fn collect_descendants(entities: &[GameObject], root_id: u64) -> Vec<u64> {
    let mut children = BTreeMap::<u64, Vec<u64>>::new();
    for entity in entities {
        if let Some(parent_id) = entity.parent_id {
            children.entry(parent_id).or_default().push(entity.id);
        }
    }
    let mut ordered = Vec::new();
    let mut stack = vec![root_id];
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        ordered.push(id);
        if let Some(child_ids) = children.get(&id) {
            for child_id in child_ids.iter().rev() {
                stack.push(*child_id);
            }
        }
    }
    ordered
}

#[cfg(test)]
mod tests {
    use crate::engine::packed_scene::{PackedScene2D, PackedSceneInstantiateOptions};
    use crate::entities::game_object::GameObject;

    #[test]
    fn packs_descendants_and_instantiates_with_new_ids() {
        let root = GameObject::new(10.0, 10.0, Some("Root".to_string()));
        let mut child = GameObject::new(12.0, 14.0, Some("Child".to_string()));
        child.parent_id = Some(root.id);
        child.local_x = 2.0;
        child.local_y = 4.0;
        let packed =
            PackedScene2D::pack_from_root(&[root.clone(), child.clone()], root.id).unwrap();

        let instance = packed.instantiate(PackedSceneInstantiateOptions {
            root_x: 100.0,
            root_y: 200.0,
            name_prefix: Some("A_".to_string()),
            preserve_scene_name: false,
        });

        assert_eq!(instance.entities.len(), 2);
        assert_ne!(instance.root_id, root.id);
        assert!(
            instance
                .entities
                .iter()
                .any(|entity| entity.name == "A_Root")
        );
        let instanced_child = instance
            .entities
            .iter()
            .find(|entity| entity.name == "A_Child")
            .unwrap();
        assert_eq!(instanced_child.parent_id, Some(instance.root_id));
        assert_eq!(instanced_child.x, 102.0);
        assert_eq!(instanced_child.y, 204.0);
    }
}
