use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Blackboard2D {
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorTree2D {
    pub name: String,
    pub root: String,
    #[serde(default)]
    pub nodes: Vec<BehaviorNode2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorNode2D {
    pub id: String,
    pub node_type: String,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub condition: Option<BehaviorCondition2D>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorCondition2D {
    pub key: String,
    pub operator: String,
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BehaviorStatus2D {
    Success,
    Failure,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorTaskExecution2D {
    pub node_id: String,
    pub task: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorTick2D {
    pub status: BehaviorStatus2D,
    #[serde(default)]
    pub visited_nodes: Vec<String>,
    #[serde(default)]
    pub executed_tasks: Vec<BehaviorTaskExecution2D>,
    pub active_node: Option<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

impl Default for BehaviorTick2D {
    fn default() -> Self {
        Self {
            status: BehaviorStatus2D::Failure,
            visited_nodes: Vec::new(),
            executed_tasks: Vec::new(),
            active_node: None,
            errors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RtsCommand2D {
    pub command: String,
    pub target: Option<(f64, f64)>,
    #[serde(default)]
    pub units: Vec<u64>,
    #[serde(default)]
    pub payload: Value,
}

impl BehaviorTree2D {
    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        let mut id_counts = BTreeMap::<String, usize>::new();
        let mut lookup = BTreeMap::<String, &BehaviorNode2D>::new();
        for node in &self.nodes {
            *id_counts.entry(node.id.clone()).or_default() += 1;
            lookup.entry(node.id.clone()).or_insert(node);
        }
        let ids = id_counts.keys().cloned().collect::<BTreeSet<_>>();
        if self.name.trim().is_empty() {
            report.warning(
                "missing_behavior_name",
                "name",
                "Behavior Tree sin nombre; el debugger no podra identificarlo.",
            );
        }
        if self.root.trim().is_empty() || !ids.contains(&self.root) {
            report.error(
                "missing_root",
                "root",
                format!("Behavior Tree root inexistente: {}", self.root),
            );
        }
        for (id, count) in &id_counts {
            if id.trim().is_empty() {
                report.error(
                    "empty_node_id",
                    "nodes",
                    "Behavior Tree contiene un nodo sin id.",
                );
            }
            if *count > 1 {
                report.error(
                    "duplicate_node_id",
                    id.clone(),
                    format!("Behavior Tree contiene {count} nodos con id `{id}`."),
                );
            }
        }
        for node in &self.nodes {
            if !matches!(
                node.node_type.as_str(),
                "selector" | "sequence" | "task" | "condition" | "decorator"
            ) {
                report.error(
                    "invalid_behavior_node",
                    node.id.clone(),
                    format!("Tipo de nodo BT invalido: {}", node.node_type),
                );
            }
            for child in &node.children {
                if !ids.contains(child) {
                    report.error(
                        "missing_child",
                        node.id.clone(),
                        format!("Nodo BT referencia child inexistente: {child}"),
                    );
                }
            }
            match node.node_type.as_str() {
                "selector" | "sequence" if node.children.is_empty() => report.error(
                    "empty_composite",
                    node.id.clone(),
                    "Selector/Sequence debe contener al menos un child.",
                ),
                "decorator" if node.children.len() != 1 => report.error(
                    "invalid_decorator_children",
                    node.id.clone(),
                    "Decorator debe contener exactamente un child.",
                ),
                "condition" if node.condition.is_none() => report.error(
                    "missing_condition",
                    node.id.clone(),
                    "Nodo condition requiere una condicion.",
                ),
                "task" if !node.children.is_empty() => report.warning(
                    "task_children_ignored",
                    node.id.clone(),
                    "Los children de un nodo task no se ejecutan.",
                ),
                _ => {}
            }
            if let Some(condition) = &node.condition {
                if condition.key.trim().is_empty() {
                    report.error(
                        "empty_condition_key",
                        node.id.clone(),
                        "Condition requiere una key de blackboard.",
                    );
                }
                if !matches!(
                    condition.operator.as_str(),
                    "==" | "!=" | ">" | ">=" | "<" | "<=" | "exists" | "not_exists" | "contains"
                ) {
                    report.error(
                        "invalid_condition_operator",
                        node.id.clone(),
                        format!("Operador BT no soportado: {}", condition.operator),
                    );
                }
            }
            if node.node_type == "task"
                && !matches!(
                    node.task.as_deref(),
                    Some(
                        "Patrol"
                            | "Chase"
                            | "Attack"
                            | "Flee"
                            | "Wander"
                            | "Guard"
                            | "FindTarget"
                            | "MoveTo"
                            | "Wait"
                            | "RTSCommand"
                    )
                )
            {
                report.error(
                    "invalid_task",
                    node.id.clone(),
                    format!("Task BT no soportada: {:?}", node.task),
                );
            }
        }

        for cycle in behavior_cycles(&lookup) {
            report.error(
                "behavior_cycle",
                cycle
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "nodes".to_string()),
                format!("Ciclo detectado en Behavior Tree: {}", cycle.join(" -> ")),
            );
        }
        if ids.contains(&self.root) {
            let mut reachable = BTreeSet::new();
            collect_reachable(&self.root, &lookup, &mut reachable);
            for id in ids.difference(&reachable) {
                report.warning(
                    "unreachable_node",
                    id.clone(),
                    format!(
                        "Nodo BT `{id}` no es alcanzable desde root `{}`.",
                        self.root
                    ),
                );
            }
        }
        report
    }

    pub fn task_order(&self) -> Vec<String> {
        let mut tasks = Vec::new();
        let lookup = self
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<BTreeMap<_, _>>();
        collect_tasks(&self.root, &lookup, &mut BTreeSet::new(), &mut tasks);
        tasks
    }

    /// Executes one behavior-tree tick against a blackboard.
    ///
    /// The task callback owns game-specific side effects and returns whether a
    /// task completed, failed, or remains active. Structural cycles and invalid
    /// references fail safely and are recorded in the returned trace.
    pub fn tick<F>(&self, blackboard: &Blackboard2D, mut run_task: F) -> BehaviorTick2D
    where
        F: FnMut(&BehaviorNode2D, &Blackboard2D) -> BehaviorStatus2D,
    {
        let mut lookup = BTreeMap::new();
        for node in &self.nodes {
            lookup.entry(node.id.as_str()).or_insert(node);
        }
        let mut trace = BehaviorTick2D::default();
        let mut stack = BTreeSet::new();
        let visit_budget = self.nodes.len().saturating_mul(4).max(16);
        trace.status = evaluate_node(
            &self.root,
            &lookup,
            blackboard,
            &mut run_task,
            &mut trace,
            &mut stack,
            visit_budget,
        );
        trace
    }

    /// Chooses the first runnable task using the tree's selector/sequence and
    /// blackboard conditions. Useful for deterministic NPC planning and editor
    /// previews that do not execute gameplay side effects.
    pub fn select_task(&self, blackboard: &Blackboard2D) -> BehaviorTick2D {
        self.tick(blackboard, |_node, _blackboard| BehaviorStatus2D::Success)
    }
}

impl Blackboard2D {
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.values.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        blackboard_value(&self.values, key)
    }

    pub fn condition_matches(&self, condition: &BehaviorCondition2D) -> bool {
        condition_matches(self.get(&condition.key), condition)
    }
}

fn evaluate_node<F>(
    id: &str,
    lookup: &BTreeMap<&str, &BehaviorNode2D>,
    blackboard: &Blackboard2D,
    run_task: &mut F,
    trace: &mut BehaviorTick2D,
    stack: &mut BTreeSet<String>,
    visit_budget: usize,
) -> BehaviorStatus2D
where
    F: FnMut(&BehaviorNode2D, &Blackboard2D) -> BehaviorStatus2D,
{
    if trace.visited_nodes.len() >= visit_budget {
        trace
            .errors
            .push(format!("Behavior Tree visit budget exceeded at `{id}`"));
        return BehaviorStatus2D::Failure;
    }
    if !stack.insert(id.to_string()) {
        trace
            .errors
            .push(format!("Behavior Tree cycle reached at `{id}`"));
        return BehaviorStatus2D::Failure;
    }
    let Some(node) = lookup.get(id).copied() else {
        trace
            .errors
            .push(format!("Behavior Tree node not found: `{id}`"));
        stack.remove(id);
        return BehaviorStatus2D::Failure;
    };
    trace.visited_nodes.push(id.to_string());

    if node
        .condition
        .as_ref()
        .is_some_and(|condition| !blackboard.condition_matches(condition))
    {
        stack.remove(id);
        return BehaviorStatus2D::Failure;
    }

    let status = match node.node_type.as_str() {
        "selector" => {
            let mut status = BehaviorStatus2D::Failure;
            for child in &node.children {
                status = evaluate_node(
                    child,
                    lookup,
                    blackboard,
                    run_task,
                    trace,
                    stack,
                    visit_budget,
                );
                if status != BehaviorStatus2D::Failure {
                    break;
                }
            }
            status
        }
        "sequence" => {
            let mut status = BehaviorStatus2D::Success;
            for child in &node.children {
                status = evaluate_node(
                    child,
                    lookup,
                    blackboard,
                    run_task,
                    trace,
                    stack,
                    visit_budget,
                );
                if status != BehaviorStatus2D::Success {
                    break;
                }
            }
            status
        }
        "condition" => node
            .condition
            .as_ref()
            .map(|condition| {
                if blackboard.condition_matches(condition) {
                    BehaviorStatus2D::Success
                } else {
                    BehaviorStatus2D::Failure
                }
            })
            .unwrap_or(BehaviorStatus2D::Failure),
        "decorator" => {
            let Some(child) = node.children.first() else {
                trace.errors.push(format!("Decorator `{id}` has no child"));
                stack.remove(id);
                return BehaviorStatus2D::Failure;
            };
            let child_status = evaluate_node(
                child,
                lookup,
                blackboard,
                run_task,
                trace,
                stack,
                visit_budget,
            );
            match node
                .data
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("invert")
            {
                "invert" => match child_status {
                    BehaviorStatus2D::Success => BehaviorStatus2D::Failure,
                    BehaviorStatus2D::Failure => BehaviorStatus2D::Success,
                    BehaviorStatus2D::Running => BehaviorStatus2D::Running,
                },
                "force_success" => match child_status {
                    BehaviorStatus2D::Running => BehaviorStatus2D::Running,
                    _ => BehaviorStatus2D::Success,
                },
                "force_failure" => match child_status {
                    BehaviorStatus2D::Running => BehaviorStatus2D::Running,
                    _ => BehaviorStatus2D::Failure,
                },
                mode => {
                    trace
                        .errors
                        .push(format!("Decorator `{id}` uses unsupported mode `{mode}`"));
                    BehaviorStatus2D::Failure
                }
            }
        }
        "task" => {
            let Some(task) = node.task.clone() else {
                trace.errors.push(format!("Task node `{id}` has no task"));
                stack.remove(id);
                return BehaviorStatus2D::Failure;
            };
            trace.executed_tasks.push(BehaviorTaskExecution2D {
                node_id: id.to_string(),
                task,
                data: node.data.clone(),
            });
            let status = run_task(node, blackboard);
            if status == BehaviorStatus2D::Running {
                trace.active_node = Some(id.to_string());
            }
            status
        }
        node_type => {
            trace.errors.push(format!(
                "Behavior node `{id}` has unsupported type `{node_type}`"
            ));
            BehaviorStatus2D::Failure
        }
    };
    stack.remove(id);
    status
}

fn blackboard_value<'a>(values: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a Value> {
    let mut segments = key.split('.');
    let first = segments.next()?;
    let mut value = values.get(first)?;
    for segment in segments {
        value = value.as_object()?.get(segment)?;
    }
    Some(value)
}

fn condition_matches(actual: Option<&Value>, condition: &BehaviorCondition2D) -> bool {
    match condition.operator.as_str() {
        "exists" => actual.is_some_and(|value| !value.is_null()),
        "not_exists" => actual.is_none_or(Value::is_null),
        "==" => actual == Some(&condition.value),
        "!=" => actual != Some(&condition.value),
        ">" | ">=" | "<" | "<=" => {
            let Some(actual) = actual.and_then(Value::as_f64) else {
                return false;
            };
            let Some(expected) = condition.value.as_f64() else {
                return false;
            };
            match condition.operator.as_str() {
                ">" => actual > expected,
                ">=" => actual >= expected,
                "<" => actual < expected,
                "<=" => actual <= expected,
                _ => false,
            }
        }
        "contains" => match actual {
            Some(Value::Array(items)) => items.contains(&condition.value),
            Some(Value::String(text)) => condition
                .value
                .as_str()
                .is_some_and(|needle| text.contains(needle)),
            Some(Value::Object(map)) => condition
                .value
                .as_str()
                .is_some_and(|key| map.contains_key(key)),
            _ => false,
        },
        _ => false,
    }
}

fn collect_reachable(
    id: &str,
    lookup: &BTreeMap<String, &BehaviorNode2D>,
    reachable: &mut BTreeSet<String>,
) {
    if !reachable.insert(id.to_string()) {
        return;
    }
    let Some(node) = lookup.get(id) else {
        return;
    };
    for child in &node.children {
        collect_reachable(child, lookup, reachable);
    }
}

fn behavior_cycles(lookup: &BTreeMap<String, &BehaviorNode2D>) -> Vec<Vec<String>> {
    fn visit(
        id: &str,
        lookup: &BTreeMap<String, &BehaviorNode2D>,
        visiting: &mut Vec<String>,
        visited: &mut BTreeSet<String>,
        cycles: &mut BTreeSet<String>,
        result: &mut Vec<Vec<String>>,
    ) {
        if let Some(start) = visiting.iter().position(|current| current == id) {
            let mut cycle = visiting[start..].to_vec();
            cycle.push(id.to_string());
            let key = cycle.join(" -> ");
            if cycles.insert(key) {
                result.push(cycle);
            }
            return;
        }
        if !visited.insert(id.to_string()) {
            return;
        }
        let Some(node) = lookup.get(id) else {
            return;
        };
        visiting.push(id.to_string());
        for child in &node.children {
            visit(child, lookup, visiting, visited, cycles, result);
        }
        visiting.pop();
    }

    let mut visiting = Vec::new();
    let mut visited = BTreeSet::new();
    let mut cycle_keys = BTreeSet::new();
    let mut cycles = Vec::new();
    for id in lookup.keys() {
        visit(
            id,
            lookup,
            &mut visiting,
            &mut visited,
            &mut cycle_keys,
            &mut cycles,
        );
    }
    cycles
}

pub fn minimal_behavior_tree() -> BehaviorTree2D {
    BehaviorTree2D {
        name: "BT_EnemyBasic2D".to_string(),
        root: "root_selector".to_string(),
        nodes: vec![
            BehaviorNode2D {
                id: "root_selector".to_string(),
                node_type: "selector".to_string(),
                task: None,
                condition: None,
                children: vec![
                    "attack_if_close".to_string(),
                    "chase_if_seen".to_string(),
                    "flee_if_low_health".to_string(),
                    "patrol".to_string(),
                    "wait".to_string(),
                ],
                data: json!({}),
            },
            BehaviorNode2D {
                id: "attack_if_close".to_string(),
                node_type: "task".to_string(),
                task: Some("Attack".to_string()),
                condition: Some(BehaviorCondition2D {
                    key: "target_distance".to_string(),
                    operator: "<=".to_string(),
                    value: json!(1.25),
                }),
                children: Vec::new(),
                data: json!({"cooldown": 0.8}),
            },
            BehaviorNode2D {
                id: "chase_if_seen".to_string(),
                node_type: "task".to_string(),
                task: Some("Chase".to_string()),
                condition: Some(BehaviorCondition2D {
                    key: "has_target".to_string(),
                    operator: "==".to_string(),
                    value: json!(true),
                }),
                children: Vec::new(),
                data: json!({"speed": 3.5}),
            },
            BehaviorNode2D {
                id: "flee_if_low_health".to_string(),
                node_type: "task".to_string(),
                task: Some("Flee".to_string()),
                condition: Some(BehaviorCondition2D {
                    key: "health_percent".to_string(),
                    operator: "<".to_string(),
                    value: json!(0.25),
                }),
                children: Vec::new(),
                data: json!({"distance": 5.0}),
            },
            BehaviorNode2D {
                id: "patrol".to_string(),
                node_type: "task".to_string(),
                task: Some("Patrol".to_string()),
                condition: None,
                children: Vec::new(),
                data: json!({"points": [[0, 0], [4, 0], [4, 4], [0, 4]]}),
            },
            BehaviorNode2D {
                id: "wait".to_string(),
                node_type: "task".to_string(),
                task: Some("Wait".to_string()),
                condition: None,
                children: Vec::new(),
                data: json!({"seconds": 0.5}),
            },
        ],
    }
}

fn collect_tasks<'a>(
    id: &str,
    lookup: &BTreeMap<&'a str, &'a BehaviorNode2D>,
    visited: &mut BTreeSet<String>,
    tasks: &mut Vec<String>,
) {
    if !visited.insert(id.to_string()) {
        return;
    }
    let Some(node) = lookup.get(id) else {
        return;
    };
    if let Some(task) = &node.task {
        tasks.push(task.clone());
    }
    for child in &node.children {
        collect_tasks(child, lookup, visited, tasks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_tree_selects_task_from_blackboard_conditions() {
        let tree = minimal_behavior_tree();
        assert!(tree.validate().is_valid());
        let mut blackboard = Blackboard2D::default();
        blackboard.set("target_distance", json!(0.75));
        blackboard.set("has_target", json!(true));
        blackboard.set("health_percent", json!(1.0));

        let tick = tree.select_task(&blackboard);

        assert_eq!(tick.status, BehaviorStatus2D::Success);
        assert_eq!(
            tick.executed_tasks.first().map(|task| task.task.as_str()),
            Some("Attack")
        );
        assert!(tick.errors.is_empty());
    }

    #[test]
    fn running_task_is_exposed_as_active_leaf() {
        let tree = minimal_behavior_tree();
        let blackboard = Blackboard2D::default();

        let tick = tree.tick(&blackboard, |node, _| {
            if node.task.as_deref() == Some("Patrol") {
                BehaviorStatus2D::Running
            } else {
                BehaviorStatus2D::Failure
            }
        });

        assert_eq!(tick.status, BehaviorStatus2D::Running);
        assert_eq!(tick.active_node.as_deref(), Some("patrol"));
        assert!(
            tick.visited_nodes
                .iter()
                .any(|node| node == "root_selector")
        );
    }

    #[test]
    fn validator_reports_duplicate_cycle_and_unreachable_nodes() {
        let tree = BehaviorTree2D {
            name: "Broken".to_string(),
            root: "root".to_string(),
            nodes: vec![
                BehaviorNode2D {
                    id: "root".to_string(),
                    node_type: "selector".to_string(),
                    task: None,
                    condition: None,
                    children: vec!["loop".to_string()],
                    data: json!({}),
                },
                BehaviorNode2D {
                    id: "loop".to_string(),
                    node_type: "sequence".to_string(),
                    task: None,
                    condition: None,
                    children: vec!["root".to_string()],
                    data: json!({}),
                },
                BehaviorNode2D {
                    id: "loop".to_string(),
                    node_type: "task".to_string(),
                    task: Some("Wait".to_string()),
                    condition: None,
                    children: Vec::new(),
                    data: json!({}),
                },
                BehaviorNode2D {
                    id: "orphan".to_string(),
                    node_type: "task".to_string(),
                    task: Some("Wait".to_string()),
                    condition: None,
                    children: Vec::new(),
                    data: json!({}),
                },
            ],
        };

        let report = tree.validate();

        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "duplicate_node_id")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "behavior_cycle")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "unreachable_node")
        );
    }

    #[test]
    fn blackboard_supports_nested_comparison_and_contains() {
        let mut blackboard = Blackboard2D::default();
        blackboard.set(
            "target",
            json!({"distance": 2.5, "tags": ["Player", "Visible"]}),
        );

        assert!(blackboard.condition_matches(&BehaviorCondition2D {
            key: "target.distance".to_string(),
            operator: "<=".to_string(),
            value: json!(3.0),
        }));
        assert!(blackboard.condition_matches(&BehaviorCondition2D {
            key: "target.tags".to_string(),
            operator: "contains".to_string(),
            value: json!("Player"),
        }));
    }

    #[test]
    fn cyclic_tree_tick_fails_without_recursing_forever() {
        let tree = BehaviorTree2D {
            name: "Cycle".to_string(),
            root: "a".to_string(),
            nodes: vec![
                BehaviorNode2D {
                    id: "a".to_string(),
                    node_type: "selector".to_string(),
                    task: None,
                    condition: None,
                    children: vec!["b".to_string()],
                    data: json!({}),
                },
                BehaviorNode2D {
                    id: "b".to_string(),
                    node_type: "sequence".to_string(),
                    task: None,
                    condition: None,
                    children: vec!["a".to_string()],
                    data: json!({}),
                },
            ],
        };

        let tick = tree.select_task(&Blackboard2D::default());

        assert_eq!(tick.status, BehaviorStatus2D::Failure);
        assert!(tick.errors.iter().any(|error| error.contains("cycle")));
    }
}
