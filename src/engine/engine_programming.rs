use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::component::{Component, default_component};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, PartialEq)]
pub struct VisualGraphNodeView {
    pub id: String,
    pub node_type: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub input_pins: Vec<String>,
    pub output_pins: Vec<String>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualGraphConnection {
    pub from: String,
    pub to: String,
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualGraphView {
    pub name: String,
    pub nodes: Vec<VisualGraphNodeView>,
    pub connections: Vec<VisualGraphConnection>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProgramTemplate {
    pub name: String,
    pub description: String,
    pub graph: Value,
}

#[derive(Debug, Clone)]
pub struct ProgrammingEnvironment {
    pub templates: Vec<ProgramTemplate>,
    pub opened_graphs: Vec<PathBuf>,
    pub compile_count: usize,
    pub last_warnings: Vec<String>,
    pub runtime_events: Vec<String>,
}

impl Default for ProgrammingEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgrammingEnvironment {
    pub fn new() -> Self {
        Self {
            templates: vec![
                ProgramTemplate::new(
                    "LogAndMove",
                    "Evento start, log y movimiento simple para prototipos rapidos.",
                    graph_log_and_move(),
                ),
                ProgramTemplate::new(
                    "ButtonClick",
                    "Flujo de UI para botones y feedback en consola.",
                    graph_button_click(),
                ),
                ProgramTemplate::new(
                    "HealthPickup",
                    "Trigger de pickup que cura y se desactiva.",
                    graph_health_pickup(),
                ),
                ProgramTemplate::new(
                    "RTSOrder",
                    "Orden data-driven para unidades RTS sin escribir codigo del motor.",
                    graph_rts_order(),
                ),
                ProgramTemplate::new(
                    "Spawner",
                    "Spawner temporizado para oleadas o recursos.",
                    graph_spawner(),
                ),
            ],
            opened_graphs: Vec::new(),
            compile_count: 0,
            last_warnings: Vec::new(),
            runtime_events: Vec::new(),
        }
    }

    pub fn template_names(&self) -> Vec<String> {
        self.templates
            .iter()
            .map(|template| template.name.clone())
            .collect()
    }

    pub fn template_graph(&self, name: &str) -> Value {
        self.templates
            .iter()
            .find(|template| template.name.eq_ignore_ascii_case(name))
            .map(|template| template.graph.clone())
            .unwrap_or_else(graph_log_and_move)
    }

    pub fn create_graph_asset(
        &mut self,
        project_path: impl AsRef<Path>,
        template_name: &str,
        filename: Option<&str>,
    ) -> io::Result<PathBuf> {
        let paths = AssetTools::get_project_paths(project_path);
        let graph_folder = paths.scripts.join("visual_graphs");
        fs::create_dir_all(&graph_folder)?;
        let mut file_name = filename
            .map(|value| AssetTools::safe_name(value, "NewGraph"))
            .unwrap_or_else(|| AssetTools::safe_name(template_name, "NewGraph"));
        if !file_name.ends_with(".mfgraph") {
            file_name.push_str(".mfgraph");
        }
        let path = AssetTools::unique_path(&graph_folder, &file_name);
        let graph = self.template_graph(template_name);
        AssetTools::write_json(&path, &graph)?;
        self.open_graph(path.clone());
        Ok(path)
    }

    pub fn open_graph(&mut self, path: PathBuf) {
        if !self.opened_graphs.contains(&path) {
            self.opened_graphs.push(path);
        }
    }

    pub fn compile_to_component(&mut self, template_name: &str) -> Component {
        let graph = self.template_graph(template_name);
        let warnings = self.validate_graph(&graph);
        self.last_warnings = warnings;
        self.compile_count += 1;

        let mut component = default_component("VisualScript").unwrap_or_else(|| {
            let mut fallback = Component::new("VisualScript");
            fallback.set("nodes", json!([]));
            fallback
        });
        component.set(
            "graph_name",
            graph
                .get("name")
                .cloned()
                .unwrap_or_else(|| json!(template_name)),
        );
        component.set(
            "runtime",
            graph
                .get("runtime")
                .cloned()
                .unwrap_or_else(|| json!("rust_visual_graph")),
        );
        component.set(
            "nodes",
            graph
                .get("nodes")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        );
        component.set(
            "variables",
            graph.get("variables").cloned().unwrap_or_else(|| json!({})),
        );
        component.set("source_template", json!(template_name));
        component
    }

    pub fn attach_template_to_entity(
        &mut self,
        entity: &mut GameObject,
        template_name: &str,
    ) -> String {
        let component = self.compile_to_component(template_name);
        let graph_name = component.get_string("graph_name", template_name);
        if entity.get_component("VisualScript").is_some() {
            if let Some(existing) = entity.get_component_mut("VisualScript") {
                existing.merge_data(&component.serialize());
            }
        } else {
            entity.add_component(component);
        }
        self.runtime_events
            .push(format!("{graph_name} attached to {}", entity.name));
        if self.runtime_events.len() > 48 {
            self.runtime_events.remove(0);
        }
        graph_name
    }

    pub fn validate_graph(&self, graph: &Value) -> Vec<String> {
        let mut warnings = Vec::new();
        let nodes = graph
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if nodes.is_empty() {
            warnings.push("Graph has no nodes".to_string());
            return warnings;
        }
        let has_event = nodes.iter().any(|node| {
            node.get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.starts_with("Event"))
        });
        if !has_event {
            warnings.push("Graph needs at least one Event node".to_string());
        }
        for node in &nodes {
            if node.get("id").and_then(Value::as_str).is_none() {
                warnings.push("Node without id".to_string());
            }
            if node.get("type").and_then(Value::as_str).is_none() {
                warnings.push("Node without type".to_string());
            }
        }
        warnings
    }

    pub fn graph_view(&self, graph: &Value) -> VisualGraphView {
        let nodes = graph
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut view_nodes = Vec::new();
        let mut connections = Vec::new();
        let mut warnings = self.validate_graph(graph);
        for (index, node) in nodes.iter().enumerate() {
            let id = node
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("node_{index}"));
            let node_type = node
                .get("type")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| "Unknown".to_string());
            let (x, y) = node_position(node, index);
            let next = node
                .get("next")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            if let Some(to) = &next {
                connections.push(VisualGraphConnection {
                    from: id.clone(),
                    to: to.clone(),
                    pin: "exec".to_string(),
                });
            }
            if next.as_deref().is_some_and(|target| {
                !nodes
                    .iter()
                    .any(|candidate| candidate.get("id").and_then(Value::as_str) == Some(target))
            }) {
                warnings.push(format!("{id} conecta con un nodo inexistente"));
            }
            view_nodes.push(VisualGraphNodeView {
                title: node_title(&node_type),
                id,
                node_type,
                x,
                y,
                input_pins: vec!["exec".to_string()],
                output_pins: vec!["exec".to_string()],
                next,
            });
        }
        VisualGraphView {
            name: graph
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("VisualGraph")
                .to_string(),
            nodes: view_nodes,
            connections,
            warnings,
        }
    }

    pub fn connect_graph_nodes(graph: &mut Value, from: &str, to: &str) -> bool {
        if from == to || !graph_has_node(graph, to) {
            return false;
        }
        let Some(nodes) = graph.get_mut("nodes").and_then(Value::as_array_mut) else {
            return false;
        };
        let Some(node) = nodes
            .iter_mut()
            .find(|node| node.get("id").and_then(Value::as_str) == Some(from))
        else {
            return false;
        };
        if let Some(map) = node.as_object_mut() {
            map.insert("next".to_string(), json!(to));
            return true;
        }
        false
    }

    pub fn move_graph_node(graph: &mut Value, node_id: &str, x: f64, y: f64) -> bool {
        let Some(nodes) = graph.get_mut("nodes").and_then(Value::as_array_mut) else {
            return false;
        };
        let Some(node) = nodes
            .iter_mut()
            .find(|node| node.get("id").and_then(Value::as_str) == Some(node_id))
        else {
            return false;
        };
        if let Some(map) = node.as_object_mut() {
            map.insert("position".to_string(), json!({"x": x, "y": y}));
            return true;
        }
        false
    }

    pub fn add_graph_node(graph: &mut Value, node_type: &str) -> Option<String> {
        let nodes = graph.get_mut("nodes").and_then(Value::as_array_mut)?;
        let safe_type = AssetTools::safe_name(node_type, "Log");
        let base = safe_type.to_ascii_lowercase();
        let mut index = nodes.len() + 1;
        let id = loop {
            let candidate = format!("{base}_{index}");
            if !nodes
                .iter()
                .any(|node| node.get("id").and_then(Value::as_str) == Some(candidate.as_str()))
            {
                break candidate;
            }
            index += 1;
        };
        nodes.push(json!({
            "id": id,
            "type": safe_type,
            "message": "New node",
            "next": null,
            "position": {"x": 120.0 + (index as f64 * 36.0), "y": 110.0 + (index as f64 * 22.0)}
        }));
        Some(id)
    }

    pub fn summary(&self) -> String {
        format!(
            "{} templates | {} open graphs | {} compiles | {} warnings",
            self.templates.len(),
            self.opened_graphs.len(),
            self.compile_count,
            self.last_warnings.len()
        )
    }
}

impl ProgramTemplate {
    pub fn new(name: &str, description: &str, graph: Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            graph,
        }
    }
}

fn graph_base(name: &str, nodes: Value, variables: Value) -> Value {
    let nodes = with_auto_node_layout(nodes);
    json!({
        "version": crate::engine::version::ENGINE_VERSION,
        "kind": "MiniForgeVisualGraph",
        "runtime": "rust_visual_graph",
        "name": name,
        "variables": variables,
        "editor": {
            "schema": 2,
            "canvas": {"x": 0, "y": 0, "zoom": 1.0}
        },
        "nodes": nodes,
    })
}

fn with_auto_node_layout(nodes: Value) -> Value {
    let Some(items) = nodes.as_array() else {
        return nodes;
    };
    Value::Array(
        items
            .iter()
            .enumerate()
            .map(|(index, node)| {
                let mut node = node.clone();
                if node.get("position").is_none()
                    && let Some(map) = node.as_object_mut()
                {
                    map.insert(
                        "position".to_string(),
                        json!({
                            "x": 48.0 + (index as f64 * 178.0),
                            "y": 46.0 + ((index % 2) as f64 * 96.0)
                        }),
                    );
                }
                node
            })
            .collect(),
    )
}

fn node_position(node: &Value, index: usize) -> (f64, f64) {
    if let Some(position) = node.get("position") {
        if let Some(array) = position.as_array()
            && array.len() >= 2
        {
            return (
                array[0].as_f64().unwrap_or(0.0),
                array[1].as_f64().unwrap_or(0.0),
            );
        }
        if let Some(object) = position.as_object() {
            return (
                object.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                object.get("y").and_then(Value::as_f64).unwrap_or(0.0),
            );
        }
    }
    (
        48.0 + (index as f64 * 178.0),
        46.0 + ((index % 2) as f64 * 96.0),
    )
}

fn node_title(node_type: &str) -> String {
    match node_type {
        "EventStart" => "Event Start",
        "EventUpdate" => "Event Update",
        "EventClick" => "Event Click",
        "EventTrigger" => "Event Trigger",
        "SetVariable" => "Set Variable",
        "SetEnabled" => "Set Enabled",
        other => other,
    }
    .to_string()
}

fn graph_has_node(graph: &Value, id: &str) -> bool {
    graph
        .get("nodes")
        .and_then(Value::as_array)
        .is_some_and(|nodes| {
            nodes
                .iter()
                .any(|node| node.get("id").and_then(Value::as_str) == Some(id))
        })
}

fn graph_log_and_move() -> Value {
    graph_base(
        "LogAndMove",
        json!([
            {"id": "start", "type": "EventStart", "next": "log"},
            {"id": "update", "type": "EventUpdate", "next": null},
            {"id": "log", "type": "Log", "message": "Visual graph running in Rust", "next": "move"},
            {"id": "move", "type": "Move", "x": 1.0, "y": 0.0, "next": null}
        ]),
        json!({"speed": 1.0}),
    )
}

fn graph_button_click() -> Value {
    graph_base(
        "ButtonClick",
        json!([
            {"id": "click", "type": "EventClick", "next": "log"},
            {"id": "start", "type": "EventStart", "next": null},
            {"id": "log", "type": "Log", "message": "Button clicked", "next": null}
        ]),
        json!({"click_count": 0}),
    )
}

fn graph_health_pickup() -> Value {
    graph_base(
        "HealthPickup",
        json!([
            {"id": "start", "type": "EventStart", "next": "log"},
            {"id": "log", "type": "Log", "message": "Health pickup ready", "next": null},
            {"id": "touch", "type": "EventTrigger", "next": "heal"},
            {"id": "heal", "type": "Heal", "amount": 25.0, "next": "disable"},
            {"id": "disable", "type": "SetEnabled", "value": false, "next": null}
        ]),
        json!({"heal_amount": 25.0}),
    )
}

fn graph_rts_order() -> Value {
    graph_base(
        "RTSOrder",
        json!([
            {"id": "start", "type": "EventStart", "next": "set"},
            {"id": "set", "type": "SetVariable", "name": "order", "value": "Move", "next": "log"},
            {"id": "log", "type": "Log", "message": "RTS order graph armed", "next": null}
        ]),
        json!({"order": "Idle", "formation": "square"}),
    )
}

fn graph_spawner() -> Value {
    graph_base(
        "Spawner",
        json!([
            {"id": "start", "type": "EventStart", "next": "timer"},
            {"id": "timer", "type": "Wait", "seconds": 2.0, "next": "log"},
            {"id": "log", "type": "Log", "message": "Spawner tick", "next": null}
        ]),
        json!({"prefab": "Enemy", "interval": 2.0}),
    )
}
