use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::component::{Component, default_component};
use crate::entities::game_object::GameObject;

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
    json!({
        "version": crate::engine::version::ENGINE_VERSION,
        "kind": "MiniForgeVisualGraph",
        "runtime": "rust_visual_graph",
        "name": name,
        "variables": variables,
        "nodes": nodes,
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
