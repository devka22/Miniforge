use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutlinerItem2D {
    pub id: u64,
    pub name: String,
    pub entity_type: String,
    pub parent_id: Option<u64>,
    pub depth: usize,
    pub enabled: bool,
    pub visible: bool,
    pub locked: bool,
    pub children: Vec<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldOutliner2D {
    pub items: Vec<OutlinerItem2D>,
    #[serde(default)]
    pub selected_ids: Vec<u64>,
}

impl WorldOutliner2D {
    pub fn from_entities(entities: &[GameObject]) -> Self {
        let children = children_by_parent(entities);
        let mut ordered = Vec::new();
        let mut roots = entities
            .iter()
            .filter(|entity| entity.parent_id.is_none())
            .collect::<Vec<_>>();
        roots.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        for root in roots {
            push_item(
                root,
                entities,
                &children,
                0,
                &mut BTreeSet::new(),
                &mut ordered,
            );
        }
        for entity in entities {
            if !ordered
                .iter()
                .any(|item: &OutlinerItem2D| item.id == entity.id)
            {
                push_item(
                    entity,
                    entities,
                    &children,
                    0,
                    &mut BTreeSet::new(),
                    &mut ordered,
                );
            }
        }
        Self {
            items: ordered,
            selected_ids: Vec::new(),
        }
    }

    pub fn search(&self, query: &str) -> Vec<&OutlinerItem2D> {
        let query = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                query.is_empty()
                    || item.name.to_lowercase().contains(&query)
                    || item.entity_type.to_lowercase().contains(&query)
                    || item.id.to_string().contains(&query)
            })
            .collect()
    }

    pub fn filter<'a>(
        &'a self,
        entities: &'a [GameObject],
        tag: Option<&str>,
        layer: Option<&str>,
        component: Option<&str>,
    ) -> Vec<&'a OutlinerItem2D> {
        self.items
            .iter()
            .filter(|item| {
                let Some(entity) = entities.iter().find(|entity| entity.id == item.id) else {
                    return false;
                };
                tag.is_none_or(|tag| entity.tag == tag)
                    && layer.is_none_or(|layer| entity.layer == layer)
                    && component.is_none_or(|component| entity.get_component(component).is_some())
            })
            .collect()
    }

    pub fn select(&mut self, id: u64, multi: bool) {
        if !multi {
            self.selected_ids.clear();
        }
        if !self.selected_ids.contains(&id) {
            self.selected_ids.push(id);
        }
    }

    pub fn duplicate(entities: &mut Vec<GameObject>, id: u64) -> Option<u64> {
        let mut clone = entities.iter().find(|entity| entity.id == id)?.clone();
        clone.id = crate::engine::entity_id::generate_entity_id();
        clone.name = format!("{}_Copy", clone.name);
        let id = clone.id;
        entities.push(clone);
        Some(id)
    }

    pub fn delete(entities: &mut Vec<GameObject>, id: u64) -> bool {
        let before = entities.len();
        entities.retain(|entity| entity.id != id);
        entities
            .iter_mut()
            .filter(|entity| entity.parent_id == Some(id))
            .for_each(|entity| entity.parent_id = None);
        before != entities.len()
    }

    pub fn set_enabled(entities: &mut [GameObject], id: u64, enabled: bool) -> bool {
        let Some(entity) = entities.iter_mut().find(|entity| entity.id == id) else {
            return false;
        };
        entity.enabled = enabled;
        entity.active = enabled;
        true
    }

    pub fn reparent(entities: &mut [GameObject], child_id: u64, parent_id: Option<u64>) -> bool {
        if parent_id == Some(child_id) {
            return false;
        }
        if let Some(parent_id) = parent_id
            && !entities.iter().any(|entity| entity.id == parent_id)
        {
            return false;
        }
        let Some(child) = entities.iter_mut().find(|entity| entity.id == child_id) else {
            return false;
        };
        child.parent_id = parent_id;
        true
    }

    pub fn set_visible(entities: &mut [GameObject], id: u64, visible: bool) -> bool {
        let Some(entity) = entities.iter_mut().find(|entity| entity.id == id) else {
            return false;
        };
        entity.visible = visible;
        true
    }

    pub fn set_locked(entities: &mut [GameObject], id: u64, locked: bool) -> bool {
        let Some(entity) = entities.iter_mut().find(|entity| entity.id == id) else {
            return false;
        };
        entity.locked = locked;
        true
    }
}

fn children_by_parent(entities: &[GameObject]) -> BTreeMap<u64, Vec<u64>> {
    let mut map: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for entity in entities {
        if let Some(parent) = entity.parent_id {
            map.entry(parent).or_default().push(entity.id);
        }
    }
    for children in map.values_mut() {
        children.sort();
    }
    map
}

fn push_item(
    entity: &GameObject,
    entities: &[GameObject],
    children: &BTreeMap<u64, Vec<u64>>,
    depth: usize,
    visited: &mut BTreeSet<u64>,
    ordered: &mut Vec<OutlinerItem2D>,
) {
    if !visited.insert(entity.id) {
        return;
    }
    let child_ids = children.get(&entity.id).cloned().unwrap_or_default();
    ordered.push(OutlinerItem2D {
        id: entity.id,
        name: entity.name.clone(),
        entity_type: entity.entity_type.clone(),
        parent_id: entity.parent_id,
        depth,
        enabled: entity.enabled,
        visible: entity.visible,
        locked: entity.locked,
        children: child_ids.clone(),
    });
    for child_id in child_ids {
        if let Some(child) = entities.iter().find(|candidate| candidate.id == child_id) {
            push_item(child, entities, children, depth + 1, visited, ordered);
        }
    }
}
