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
        let ids = self
            .nodes
            .iter()
            .map(|node| &node.id)
            .collect::<BTreeSet<_>>();
        if !ids.contains(&self.root) {
            report.error(
                "missing_root",
                "root",
                format!("Behavior Tree root inexistente: {}", self.root),
            );
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
