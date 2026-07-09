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
    pub component_count: usize,
    pub tag: String,
    pub layer: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldOutliner2D {
    pub items: Vec<OutlinerItem2D>,
    #[serde(default)]
    pub selected_ids: Vec<u64>,
    #[serde(default)]
    pub warnings: Vec<OutlinerWarning2D>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutlinerSummary2D {
    pub total: usize,
    pub visible: usize,
    pub hidden: usize,
    pub locked: usize,
    pub selected: usize,
    pub roots: usize,
    pub warnings: usize,
    pub by_layer: BTreeMap<String, usize>,
    pub by_tag: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutlinerWarningSeverity2D {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutlinerWarning2D {
    pub entity_id: u64,
    pub code: String,
    pub severity: OutlinerWarningSeverity2D,
    pub message: String,
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
            warnings: collect_warnings(entities),
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

    pub fn summary(&self) -> OutlinerSummary2D {
        let mut summary = OutlinerSummary2D {
            total: self.items.len(),
            selected: self.selected_ids.len(),
            warnings: self.warnings.len(),
            roots: self
                .items
                .iter()
                .filter(|item| item.parent_id.is_none())
                .count(),
            ..Default::default()
        };
        for item in &self.items {
            if item.visible {
                summary.visible += 1;
            } else {
                summary.hidden += 1;
            }
            if item.locked {
                summary.locked += 1;
            }
            *summary.by_layer.entry(item.layer.clone()).or_insert(0) += 1;
            *summary.by_tag.entry(item.tag.clone()).or_insert(0) += 1;
        }
        summary
    }

    pub fn warnings_for(&self, id: u64) -> Vec<&OutlinerWarning2D> {
        self.warnings
            .iter()
            .filter(|warning| warning.entity_id == id)
            .collect()
    }

    pub fn warning_count_for(&self, id: u64) -> usize {
        self.warnings_for(id).len()
    }

    pub fn visible_items(&self) -> Vec<&OutlinerItem2D> {
        self.items
            .iter()
            .filter(|item| item.enabled && item.visible)
            .collect()
    }

    pub fn hidden_items(&self) -> Vec<&OutlinerItem2D> {
        self.items.iter().filter(|item| !item.visible).collect()
    }

    pub fn locked_items(&self) -> Vec<&OutlinerItem2D> {
        self.items.iter().filter(|item| item.locked).collect()
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

    pub fn select_many(&mut self, ids: impl IntoIterator<Item = u64>, replace: bool) {
        if replace {
            self.selected_ids.clear();
        }
        for id in ids {
            if self.items.iter().any(|item| item.id == id) && !self.selected_ids.contains(&id) {
                self.selected_ids.push(id);
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_ids.clear();
    }

    pub fn item_path(&self, id: u64) -> Option<String> {
        let item = self.items.iter().find(|item| item.id == id)?;
        let mut names = vec![item.name.clone()];
        let mut parent_id = item.parent_id;
        while let Some(parent) = parent_id {
            let parent_item = self.items.iter().find(|item| item.id == parent)?;
            names.push(parent_item.name.clone());
            parent_id = parent_item.parent_id;
        }
        names.reverse();
        Some(names.join("/"))
    }

    pub fn descendants_of(&self, id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        let mut stack = self
            .items
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.children.clone())
            .unwrap_or_default();
        while let Some(child_id) = stack.pop() {
            result.push(child_id);
            if let Some(child) = self.items.iter().find(|item| item.id == child_id) {
                stack.extend(child.children.iter().copied());
            }
        }
        result
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

    pub fn delete_recursive(entities: &mut Vec<GameObject>, id: u64) -> usize {
        let mut ids = BTreeSet::from([id]);
        let mut changed = true;
        while changed {
            changed = false;
            for entity in entities.iter() {
                if entity.parent_id.is_some_and(|parent| ids.contains(&parent))
                    && ids.insert(entity.id)
                {
                    changed = true;
                }
            }
        }
        let before = entities.len();
        entities.retain(|entity| !ids.contains(&entity.id));
        before.saturating_sub(entities.len())
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

    pub fn set_locked_recursive(entities: &mut [GameObject], id: u64, locked: bool) -> usize {
        let mut ids = BTreeSet::from([id]);
        let mut changed = true;
        while changed {
            changed = false;
            for entity in entities.iter() {
                if entity.parent_id.is_some_and(|parent| ids.contains(&parent))
                    && ids.insert(entity.id)
                {
                    changed = true;
                }
            }
        }
        let mut count = 0;
        for entity in entities
            .iter_mut()
            .filter(|entity| ids.contains(&entity.id))
        {
            entity.locked = locked;
            count += 1;
        }
        count
    }

    pub fn rename(entities: &mut [GameObject], id: u64, name: &str) -> bool {
        let Some(entity) = entities.iter_mut().find(|entity| entity.id == id) else {
            return false;
        };
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        entity.name = name.to_string();
        true
    }

    pub fn move_to_layer(entities: &mut [GameObject], id: u64, layer: &str) -> bool {
        let Some(entity) = entities.iter_mut().find(|entity| entity.id == id) else {
            return false;
        };
        let layer = layer.trim();
        if layer.is_empty() {
            return false;
        }
        entity.layer = layer.to_string();
        true
    }

    pub fn set_visible_recursive(entities: &mut [GameObject], id: u64, visible: bool) -> usize {
        let mut ids = BTreeSet::from([id]);
        let mut changed = true;
        while changed {
            changed = false;
            for entity in entities.iter() {
                if entity.parent_id.is_some_and(|parent| ids.contains(&parent))
                    && ids.insert(entity.id)
                {
                    changed = true;
                }
            }
        }
        let mut count = 0;
        for entity in entities
            .iter_mut()
            .filter(|entity| ids.contains(&entity.id))
        {
            entity.visible = visible;
            count += 1;
        }
        count
    }

    pub fn context_actions_for(&self, id: u64) -> Vec<&'static str> {
        let Some(item) = self.items.iter().find(|item| item.id == id) else {
            return Vec::new();
        };
        let mut actions = vec!["rename", "duplicate", "focus", "create_child"];
        if item.visible {
            actions.push("hide");
        } else {
            actions.push("show");
        }
        if item.locked {
            actions.push("unlock");
        } else {
            actions.push("lock");
        }
        if !item.children.is_empty() {
            actions.push("collapse_children");
            actions.push("hide_recursive");
            actions.push("lock_recursive");
            actions.push("delete_recursive");
        }
        if self.warning_count_for(id) > 0 {
            actions.push("show_warnings");
        }
        actions.push("select_children");
        actions.push("copy_path");
        actions
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
        component_count: entity.components.len(),
        tag: entity.tag.clone(),
        layer: entity.layer.clone(),
    });
    for child_id in child_ids {
        if let Some(child) = entities.iter().find(|candidate| candidate.id == child_id) {
            push_item(child, entities, children, depth + 1, visited, ordered);
        }
    }
}

fn collect_warnings(entities: &[GameObject]) -> Vec<OutlinerWarning2D> {
    let ids = entities
        .iter()
        .map(|entity| entity.id)
        .collect::<BTreeSet<_>>();
    let mut warnings = Vec::new();
    for entity in entities {
        if entity.parent_id.is_none() && root_has_transform(entity) && !is_ui_root(entity) {
            warnings.push(warning(
                entity.id,
                "root_transform",
                OutlinerWarningSeverity2D::Warning,
                "El root de una escena 2D conviene dejarlo sin transform para que las instancias no hereden offsets inesperados.",
            ));
        }
        if let Some(parent_id) = entity.parent_id {
            if parent_id == entity.id {
                warnings.push(warning(
                    entity.id,
                    "self_parent",
                    OutlinerWarningSeverity2D::Error,
                    "La entidad esta parentada a si misma.",
                ));
            } else if !ids.contains(&parent_id) {
                warnings.push(warning(
                    entity.id,
                    "missing_parent",
                    OutlinerWarningSeverity2D::Error,
                    "La entidad referencia un parent que no existe en la escena.",
                ));
            }
            if let Some(parent) = entities.iter().find(|candidate| candidate.id == parent_id)
                && !parent.visible
                && entity.visible
            {
                warnings.push(warning(
                    entity.id,
                    "visible_child_hidden_parent",
                    OutlinerWarningSeverity2D::Info,
                    "La entidad esta visible, pero su parent esta oculto.",
                ));
            }
        }
        if has_parent_cycle(entity.id, entities) {
            warnings.push(warning(
                entity.id,
                "parent_cycle",
                OutlinerWarningSeverity2D::Error,
                "La jerarquia contiene un ciclo de parents.",
            ));
        }
    }
    warnings
}

fn warning(
    entity_id: u64,
    code: &str,
    severity: OutlinerWarningSeverity2D,
    message: &str,
) -> OutlinerWarning2D {
    OutlinerWarning2D {
        entity_id,
        code: code.to_string(),
        severity,
        message: message.to_string(),
    }
}

fn root_has_transform(entity: &GameObject) -> bool {
    entity.x.abs() > f64::EPSILON
        || entity.y.abs() > f64::EPSILON
        || entity.rotation.abs() > f64::EPSILON
        || (entity.scale_x - 1.0).abs() > f64::EPSILON
        || (entity.scale_y - 1.0).abs() > f64::EPSILON
}

fn is_ui_root(entity: &GameObject) -> bool {
    entity.entity_type.contains("Widget") || entity.entity_type.contains("UI")
}

fn has_parent_cycle(entity_id: u64, entities: &[GameObject]) -> bool {
    let mut seen = BTreeSet::new();
    let mut current = Some(entity_id);
    while let Some(id) = current {
        if !seen.insert(id) {
            return true;
        }
        current = entities
            .iter()
            .find(|entity| entity.id == id)
            .and_then(|entity| entity.parent_id);
    }
    false
}
