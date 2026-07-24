use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::engine::node_path::{NodePath, node_path_segment};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneTreeNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub path: String,
    pub depth: usize,
    pub child_count: usize,
    pub groups: Vec<String>,
    pub component_types: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneTreeIndex {
    pub roots: Vec<u64>,
    pub nodes: BTreeMap<u64, SceneTreeNode>,
    pub children: BTreeMap<u64, Vec<u64>>,
    pub groups: BTreeMap<String, Vec<u64>>,
    pub warnings: Vec<String>,
}

impl SceneTreeIndex {
    pub fn build(entities: &[GameObject]) -> Self {
        let ids = entities
            .iter()
            .map(|entity| entity.id)
            .collect::<BTreeSet<_>>();
        let entity_by_id = entities
            .iter()
            .map(|entity| (entity.id, entity))
            .collect::<BTreeMap<_, _>>();
        let mut roots = Vec::new();
        let mut children = BTreeMap::<u64, Vec<u64>>::new();
        let mut warnings = Vec::new();

        for entity in entities {
            match entity.parent_id {
                Some(parent_id) if ids.contains(&parent_id) => {
                    children.entry(parent_id).or_default().push(entity.id);
                }
                Some(parent_id) => {
                    warnings.push(format!(
                        "Entity {} ({}) references missing parent {} and is treated as root",
                        entity.id, entity.name, parent_id
                    ));
                    roots.push(entity.id);
                }
                None => roots.push(entity.id),
            }
        }

        warnings.extend(duplicate_sibling_name_warnings(entities));

        let duplicate_names = duplicate_sibling_names(entities);
        let mut nodes = BTreeMap::new();
        let mut groups = BTreeMap::<String, BTreeSet<u64>>::new();
        let mut visited = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        for root_id in roots.iter().copied() {
            walk_tree(
                root_id,
                None,
                0,
                &entity_by_id,
                &children,
                &duplicate_names,
                &mut nodes,
                &mut groups,
                &mut visited,
                &mut visiting,
                &mut warnings,
            );
        }

        for entity in entities {
            if !visited.contains(&entity.id) {
                warnings.push(format!(
                    "Entity {} ({}) was not reachable from a root; indexed as detached",
                    entity.id, entity.name
                ));
                walk_tree(
                    entity.id,
                    None,
                    0,
                    &entity_by_id,
                    &children,
                    &duplicate_names,
                    &mut nodes,
                    &mut groups,
                    &mut visited,
                    &mut visiting,
                    &mut warnings,
                );
            }
        }

        let groups = groups
            .into_iter()
            .map(|(group, ids)| (group, ids.into_iter().collect()))
            .collect();

        Self {
            roots,
            nodes,
            children,
            groups,
            warnings,
        }
    }

    pub fn node(&self, id: u64) -> Option<&SceneTreeNode> {
        self.nodes.get(&id)
    }

    pub fn children_of(&self, id: u64) -> &[u64] {
        self.children
            .get(&id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn path_for(&self, id: u64) -> Option<&str> {
        self.nodes.get(&id).map(|node| node.path.as_str())
    }

    pub fn ids_in_group(&self, group: &str) -> Vec<u64> {
        self.groups.get(group).cloned().unwrap_or_default()
    }

    pub fn contains_group(&self, entity_id: u64, group: &str) -> bool {
        self.groups
            .get(group)
            .is_some_and(|ids| ids.contains(&entity_id))
    }

    pub fn resolve_path(&self, current_id: Option<u64>, path: &str) -> Option<u64> {
        let path = path.trim();
        if let Some(id) = path.strip_prefix('#').and_then(|value| value.parse().ok()) {
            return self.nodes.contains_key(&id).then_some(id);
        }
        let node_path = NodePath::parse(path).ok()?;
        if node_path.is_absolute() {
            return self.resolve_absolute_segments(node_path.segments());
        }
        if node_path.is_current() {
            return current_id;
        }

        let mut absolute_segments = current_id
            .and_then(|id| self.path_for(id))
            .map(split_absolute_path)
            .unwrap_or_default();
        for segment in node_path.segments() {
            match segment.as_str() {
                "." => {}
                ".." => {
                    absolute_segments.pop();
                }
                _ => absolute_segments.push(segment.clone()),
            }
        }
        self.resolve_absolute_segments(&absolute_segments)
    }

    fn resolve_absolute_segments(&self, segments: &[String]) -> Option<u64> {
        if segments.is_empty() {
            return None;
        }
        let path = format!("/{}", segments.join("/"));
        self.nodes
            .values()
            .find(|node| node.path == path)
            .map(|node| node.id)
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_tree(
    entity_id: u64,
    parent_path: Option<&str>,
    depth: usize,
    entity_by_id: &BTreeMap<u64, &GameObject>,
    children: &BTreeMap<u64, Vec<u64>>,
    duplicate_names: &BTreeSet<(Option<u64>, String)>,
    nodes: &mut BTreeMap<u64, SceneTreeNode>,
    groups: &mut BTreeMap<String, BTreeSet<u64>>,
    visited: &mut BTreeSet<u64>,
    visiting: &mut BTreeSet<u64>,
    warnings: &mut Vec<String>,
) {
    if visited.contains(&entity_id) {
        return;
    }
    if !visiting.insert(entity_id) {
        warnings.push(format!("Cycle detected while indexing entity {entity_id}"));
        return;
    }
    let Some(entity) = entity_by_id.get(&entity_id).copied() else {
        visiting.remove(&entity_id);
        return;
    };
    let mut segment = node_path_segment(&entity.name, entity.id);
    if duplicate_names.contains(&(entity.parent_id, segment.clone())) {
        segment = format!("{segment}#{}", entity.id);
    }
    let path = match parent_path {
        Some(parent) if parent != "/" => format!("{parent}/{segment}"),
        Some(_) | None => format!("/{segment}"),
    };
    let entity_groups = groups_for_entity(entity);
    for group in &entity_groups {
        groups.entry(group.clone()).or_default().insert(entity.id);
    }
    let child_count = children.get(&entity_id).map(Vec::len).unwrap_or(0);
    nodes.insert(
        entity_id,
        SceneTreeNode {
            id: entity.id,
            parent_id: entity.parent_id,
            name: entity.name.clone(),
            path: path.clone(),
            depth,
            child_count,
            groups: entity_groups,
            component_types: entity.component_types(),
        },
    );

    for child_id in children.get(&entity_id).into_iter().flatten().copied() {
        walk_tree(
            child_id,
            Some(&path),
            depth + 1,
            entity_by_id,
            children,
            duplicate_names,
            nodes,
            groups,
            visited,
            visiting,
            warnings,
        );
    }
    visiting.remove(&entity_id);
    visited.insert(entity_id);
}

fn groups_for_entity(entity: &GameObject) -> Vec<String> {
    let mut groups = BTreeSet::new();
    if let Some(group) = entity
        .editor_group
        .as_deref()
        .filter(|group| !group.is_empty())
    {
        groups.insert(group.to_string());
    }
    if !entity.tag.trim().is_empty() && entity.tag != "Untagged" {
        groups.insert(format!("tag:{}", entity.tag));
    }
    if !entity.layer.trim().is_empty() && entity.layer != "Default" {
        groups.insert(format!("layer:{}", entity.layer));
    }
    if let Some(membership) = entity.get_component("GroupMembership") {
        for group in membership.get_string_list("groups") {
            if !group.trim().is_empty() {
                groups.insert(group);
            }
        }
    }
    groups.into_iter().collect()
}

fn duplicate_sibling_name_warnings(entities: &[GameObject]) -> Vec<String> {
    duplicate_sibling_names(entities)
        .into_iter()
        .map(|(parent_id, name)| match parent_id {
            Some(parent_id) => format!(
                "Duplicate sibling node name '{}' under parent {}; paths use #id suffixes",
                name, parent_id
            ),
            None => format!(
                "Duplicate root node name '{}'; paths use #id suffixes",
                name
            ),
        })
        .collect()
}

fn duplicate_sibling_names(entities: &[GameObject]) -> BTreeSet<(Option<u64>, String)> {
    let mut counts = BTreeMap::<(Option<u64>, String), usize>::new();
    for entity in entities {
        let segment = node_path_segment(&entity.name, entity.id);
        *counts.entry((entity.parent_id, segment)).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(key, count)| (count > 1).then_some(key))
        .collect()
}

fn split_absolute_path(path: &str) -> Vec<String> {
    path.trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::engine::component::default_component;
    use crate::engine::scene_tree::SceneTreeIndex;
    use crate::entities::game_object::GameObject;

    #[test]
    fn indexes_absolute_and_relative_node_paths() {
        let parent = GameObject::new(0.0, 0.0, Some("Root".to_string()));
        let mut child = GameObject::new(0.0, 0.0, Some("Player".to_string()));
        let child_id = child.id;
        child.parent_id = Some(parent.id);
        let mut camera = GameObject::new(0.0, 0.0, Some("Camera".to_string()));
        let camera_id = camera.id;
        camera.parent_id = Some(parent.id);
        let index = SceneTreeIndex::build(&[parent, child, camera]);

        assert_eq!(index.resolve_path(None, "/Root/Camera"), Some(camera_id));
        assert_eq!(
            index.resolve_path(Some(child_id), "../Camera"),
            Some(camera_id)
        );
        assert_eq!(index.resolve_path(Some(child_id), "."), Some(child_id));
    }

    #[test]
    fn collects_editor_and_component_groups() {
        let mut entity = GameObject::new(0.0, 0.0, Some("Enemy".to_string()));
        entity.tag = "Enemy".to_string();
        let mut groups = default_component("GroupMembership").unwrap();
        groups.set("groups", serde_json::json!(["damageable", "ai"]));
        entity.add_component(groups);
        let id = entity.id;
        let index = SceneTreeIndex::build(&[entity]);

        assert_eq!(index.ids_in_group("damageable"), vec![id]);
        assert_eq!(index.ids_in_group("tag:Enemy"), vec![id]);
    }
}
