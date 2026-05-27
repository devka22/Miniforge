use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};
use crate::engine::miniforge_2d::validation::{ValidationReport2D, require_keys};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintGraph2D {
    pub name: String,
    pub runtime: String,
    #[serde(default)]
    pub variables: BTreeMap<String, BlueprintVariable2D>,
    #[serde(default)]
    pub functions: BTreeMap<String, BlueprintFunction2D>,
    #[serde(default)]
    pub nodes: Vec<BlueprintNode2D>,
    #[serde(default)]
    pub edges: Vec<BlueprintEdge2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintVariable2D {
    pub value_type: String,
    #[serde(default)]
    pub default_value: Value,
    #[serde(default)]
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintFunction2D {
    #[serde(default)]
    pub inputs: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub outputs: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub entry_node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintNode2D {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub pins: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintPin2D {
    pub name: String,
    pub pin_type: String,
    #[serde(default)]
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintEdge2D {
    pub from: String,
    pub from_pin: String,
    pub to: String,
    pub to_pin: String,
}

impl Default for BlueprintGraph2D {
    fn default() -> Self {
        minimal_blueprint_graph()
    }
}

impl BlueprintGraph2D {
    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        let supported = supported_node_kinds();
        let mut ids = BTreeSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() {
                report.error("node_id_empty", "nodes", "Nodo Blueprint sin id.");
            }
            if !ids.insert(node.id.clone()) {
                report.error(
                    "duplicate_node",
                    format!("nodes.{}", node.id),
                    format!("Nodo duplicado: {}", node.id),
                );
            }
            if !supported.contains(node.kind.as_str()) {
                report.error(
                    "invalid_node_kind",
                    format!("nodes.{}", node.id),
                    format!("Nodo no soportado por MiniForge2D: {}", node.kind),
                );
            }
            validate_required_node_data(node, &mut report);
        }
        for edge in &self.edges {
            if !ids.contains(&edge.from) {
                report.error(
                    "edge_missing_from",
                    format!("edges.{}->{}", edge.from, edge.to),
                    format!("Conexion sale de un nodo inexistente: {}", edge.from),
                );
            }
            if !ids.contains(&edge.to) {
                report.error(
                    "edge_missing_to",
                    format!("edges.{}->{}", edge.from, edge.to),
                    format!("Conexion apunta a un nodo inexistente: {}", edge.to),
                );
            }
            if edge.from_pin.trim().is_empty() || edge.to_pin.trim().is_empty() {
                report.warning(
                    "edge_pin_empty",
                    format!("edges.{}->{}", edge.from, edge.to),
                    "Conexion con pin vacio; el editor intentara inferir exec.",
                );
            }
        }
        if !self.nodes.iter().any(|node| node.kind.starts_with("Event")) {
            report.warning(
                "missing_event",
                "nodes",
                "Graph sin evento de entrada; no se ejecutara automaticamente.",
            );
        }
        report
    }

    pub fn to_visual_script_component(&self) -> Component {
        let mut component =
            default_component("VisualScript").unwrap_or_else(|| Component::new("VisualScript"));
        component.set("graph_name", json!(self.name));
        component.set("variables", json!(self.variable_defaults()));
        component.set("nodes", Value::Array(self.visual_script_nodes()));
        component
    }

    pub fn variable_defaults(&self) -> BTreeMap<String, Value> {
        self.variables
            .iter()
            .map(|(name, variable)| (name.clone(), variable.default_value.clone()))
            .collect()
    }

    fn visual_script_nodes(&self) -> Vec<Value> {
        self.nodes
            .iter()
            .map(|node| {
                let next = self
                    .edges
                    .iter()
                    .find(|edge| edge.from == node.id && edge.from_pin == "then")
                    .map(|edge| edge.to.clone());
                let mut data = node.data.clone();
                if !data.is_object() {
                    data = json!({});
                }
                let mut map = data.as_object().cloned().unwrap_or_default();
                map.insert("id".to_string(), json!(node.id));
                map.insert("type".to_string(), json!(visual_runtime_kind(&node.kind)));
                if let Some(next) = next {
                    map.insert("next".to_string(), json!(next));
                }
                Value::Object(map)
            })
            .collect()
    }
}

pub fn supported_node_kinds() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "EventBeginPlay",
        "EventTick",
        "EventInput",
        "EventCollision",
        "EventTrigger",
        "EventDamage",
        "EventDeath",
        "EventAnimationFrame",
        "EventUIButtonClicked",
        "CustomEvent",
        "FunctionEntry",
        "CallFunction",
        "MacroEntry",
        "Comment",
        "Reroute",
        "Branch",
        "Delay",
        "Sequence",
        "SetVariable",
        "GetVariable",
        "SpawnActor2D",
        "DestroyActor",
        "DestroySelf",
        "SetTransform2D",
        "AddForce2D",
        "SetVelocity2D",
        "PlayAnimation2D",
        "PlaySound",
        "ApplyDamage",
        "SetAnimationParameter",
        "SetUiText",
        "CreateWidget",
        "LoadScene",
        "SaveGame",
        "FindEntityByTag",
        "GetComponent",
        "SetBlackboardValue",
        "GetBlackboardValue",
        "AIMoveTo",
        "Patrol",
        "RunBehaviorTree",
        "InventoryAdd",
        "SetPhysicsEnabled",
        "Raycast2D",
        "PrintString",
    ])
}

pub fn minimal_blueprint_graph() -> BlueprintGraph2D {
    BlueprintGraph2D {
        name: "BP_PlayerPawn2D".to_string(),
        runtime: "miniforge_visual_script_2d".to_string(),
        variables: BTreeMap::from([(
            "MoveSpeed".to_string(),
            BlueprintVariable2D {
                value_type: "float".to_string(),
                default_value: json!(5.0),
                editable: true,
            },
        )]),
        functions: BTreeMap::new(),
        nodes: vec![
            BlueprintNode2D {
                id: "begin_play".to_string(),
                kind: "EventBeginPlay".to_string(),
                title: "Begin Play".to_string(),
                x: 0.0,
                y: 0.0,
                pins: exec_out("then"),
                data: json!({}),
            },
            BlueprintNode2D {
                id: "print_ready".to_string(),
                kind: "PrintString".to_string(),
                title: "Print Ready".to_string(),
                x: 260.0,
                y: 0.0,
                pins: exec_in_out("exec", "then"),
                data: json!({"message": "MiniForge2D Blueprint ready"}),
            },
        ],
        edges: vec![BlueprintEdge2D {
            from: "begin_play".to_string(),
            from_pin: "then".to_string(),
            to: "print_ready".to_string(),
            to_pin: "exec".to_string(),
        }],
    }
}

pub fn graph_from_value(value: &Value) -> Result<BlueprintGraph2D, serde_json::Error> {
    serde_json::from_value(value.clone())
}

fn exec_out(pin: &str) -> Vec<BlueprintPin2D> {
    vec![BlueprintPin2D {
        name: pin.to_string(),
        pin_type: "exec".to_string(),
        direction: "out".to_string(),
    }]
}

fn exec_in_out(input: &str, output: &str) -> Vec<BlueprintPin2D> {
    vec![
        BlueprintPin2D {
            name: input.to_string(),
            pin_type: "exec".to_string(),
            direction: "in".to_string(),
        },
        BlueprintPin2D {
            name: output.to_string(),
            pin_type: "exec".to_string(),
            direction: "out".to_string(),
        },
    ]
}

fn validate_required_node_data(node: &BlueprintNode2D, report: &mut ValidationReport2D) {
    match node.kind.as_str() {
        "EventInput" => require_keys(
            &node.data,
            &format!("nodes.{}", node.id),
            &["action"],
            report,
        ),
        "Delay" => require_keys(
            &node.data,
            &format!("nodes.{}", node.id),
            &["seconds"],
            report,
        ),
        "SpawnActor2D" => require_keys(
            &node.data,
            &format!("nodes.{}", node.id),
            &["prefab"],
            report,
        ),
        "SetVariable" | "GetVariable" => {
            require_keys(&node.data, &format!("nodes.{}", node.id), &["name"], report)
        }
        "PlayAnimation2D" => require_keys(
            &node.data,
            &format!("nodes.{}", node.id),
            &["state"],
            report,
        ),
        "SetUiText" => require_keys(&node.data, &format!("nodes.{}", node.id), &["text"], report),
        _ => {}
    }
}

fn visual_runtime_kind(kind: &str) -> &str {
    match kind {
        "EventBeginPlay" => "EventStart",
        "EventTick" => "EventUpdate",
        "EventCollision" => "EventTrigger",
        "EventDamage" | "EventDeath" | "EventAnimationFrame" | "EventUIButtonClicked" => {
            "CustomEvent"
        }
        "Branch" => "BranchVariable",
        "Delay" => "Wait",
        "PrintString" => "Log",
        "SetTransform2D" => "SetPosition",
        "SetVelocity2D" => "SetVelocity",
        "AddForce2D" => "AddForce",
        "PlayAnimation2D" => "SetAnimation",
        "ApplyDamage" => "Damage",
        "SetBlackboardValue" => "SetBlackboard",
        "Patrol" | "RunBehaviorTree" | "SaveGame" | "FindEntityByTag" | "GetComponent" => "Log",
        other => other,
    }
}
