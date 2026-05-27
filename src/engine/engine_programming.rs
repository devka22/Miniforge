use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use strsim::jaro_winkler;

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

#[derive(Debug, Clone, PartialEq)]
pub struct VisualGraphNodeDefinition {
    pub node_type: String,
    pub label: String,
    pub category: String,
    pub description: String,
    pub default_node: Value,
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
                    "PlayerVitalMovement",
                    "Vida, movimiento por velocidad y rama de muerte para un jugador basico.",
                    graph_player_vital_movement(),
                ),
                ProgramTemplate::new(
                    "HealthCombat",
                    "Flujo de daño, curacion y comparacion de vida listo para combates.",
                    graph_health_combat(),
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
                ProgramTemplate::new(
                    "BlueprintCommunication",
                    "Eventos custom, broadcast, gate y flip-flop para flujos estilo Blueprint.",
                    graph_blueprint_communication(),
                ),
                ProgramTemplate::new(
                    "InventoryEconomyLoop",
                    "Inventario, economia, compra y ramas de recursos para RPG, survival o RTS.",
                    graph_inventory_economy_loop(),
                ),
                ProgramTemplate::new(
                    "QuestAbilityLoop",
                    "Quest con objetivo, habilidad con cargas y feedback de estado.",
                    graph_quest_ability_loop(),
                ),
                ProgramTemplate::new(
                    "RTSProductionEconomy",
                    "Wallet, receta, produccion preferida y cola base para RTS 2D.",
                    graph_rts_production_economy(),
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

    pub fn search_templates(&self, query: &str) -> Vec<ProgramTemplate> {
        let query = query.trim().to_lowercase();
        let mut scored = self
            .templates
            .iter()
            .filter_map(|template| {
                let score = fuzzy_score(&query, &[&template.name, &template.description]);
                if query.is_empty() || score >= 0.62 {
                    Some((score, template.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.name.cmp(&b.1.name))
        });
        scored.into_iter().map(|(_, template)| template).collect()
    }

    pub fn node_catalog(&self) -> Vec<VisualGraphNodeDefinition> {
        node_catalog()
    }

    pub fn search_node_catalog(&self, query: &str) -> Vec<VisualGraphNodeDefinition> {
        search_node_catalog(query)
    }

    pub fn node_definition(node_type: &str) -> Option<VisualGraphNodeDefinition> {
        node_catalog()
            .into_iter()
            .find(|definition| definition.node_type.eq_ignore_ascii_case(node_type))
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
        use std::collections::BTreeSet;

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
        let known_types = self
            .node_catalog()
            .into_iter()
            .map(|definition| definition.node_type)
            .collect::<BTreeSet<_>>();
        let mut ids = BTreeSet::new();
        for node in &nodes {
            let Some(id) = node.get("id").and_then(Value::as_str) else {
                warnings.push("Node without id".to_string());
                continue;
            };
            if !ids.insert(id.to_string()) {
                warnings.push(format!("Duplicated node id: {id}"));
            }
            let Some(node_type) = node.get("type").and_then(Value::as_str) else {
                warnings.push(format!("{id}: node without type"));
                continue;
            };
            if !known_types.contains(node_type) {
                warnings.push(format!("{id}: unknown node type {node_type}"));
            }
            for key in [
                "next",
                "true_next",
                "false_next",
                "then_0",
                "then_1",
                "a_next",
                "b_next",
            ] {
                if let Some(target) = node.get(key).and_then(Value::as_str)
                    && !nodes.iter().any(|candidate| {
                        candidate.get("id").and_then(Value::as_str) == Some(target)
                    })
                {
                    warnings.push(format!("{id}: {key} points to missing node {target}"));
                }
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
            for (key, pin) in graph_branch_keys() {
                if let Some(to) = node.get(key).and_then(Value::as_str) {
                    connections.push(VisualGraphConnection {
                        from: id.clone(),
                        to: to.to_string(),
                        pin: pin.to_string(),
                    });
                }
            }
            if next.as_deref().is_some_and(|target| {
                !nodes
                    .iter()
                    .any(|candidate| candidate.get("id").and_then(Value::as_str) == Some(target))
            }) {
                warnings.push(format!("{id} conecta con un nodo inexistente"));
            }
            for (key, pin) in graph_branch_keys() {
                if let Some(target) = node.get(key).and_then(Value::as_str)
                    && !nodes.iter().any(|candidate| {
                        candidate.get("id").and_then(Value::as_str) == Some(target)
                    })
                {
                    warnings.push(format!("{id} pin {pin} conecta con un nodo inexistente"));
                }
            }
            view_nodes.push(VisualGraphNodeView {
                title: node_title(&node_type),
                id,
                input_pins: vec!["exec".to_string()],
                output_pins: output_pins_for(&node_type),
                node_type,
                x,
                y,
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
        Self::connect_graph_nodes_on_pin(graph, from, to, "exec")
    }

    pub fn connect_graph_nodes_on_pin(graph: &mut Value, from: &str, to: &str, pin: &str) -> bool {
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
            let key = match pin {
                "true" | "true_next" => "true_next",
                "false" | "false_next" => "false_next",
                "then_0" => "then_0",
                "then_1" => "then_1",
                "a" | "a_next" => "a_next",
                "b" | "b_next" => "b_next",
                _ => "next",
            };
            map.insert(key.to_string(), json!(to));
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
        let definition = Self::node_definition(node_type);
        let safe_type = definition
            .as_ref()
            .map(|definition| definition.node_type.clone())
            .unwrap_or_else(|| AssetTools::safe_name(node_type, "Log"));
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
        let mut node = definition
            .map(|definition| definition.default_node)
            .unwrap_or_else(|| json!({"type": safe_type, "message": "New node", "next": null}));
        if let Some(map) = node.as_object_mut() {
            map.insert("id".to_string(), json!(id.clone()));
            map.insert(
                "position".to_string(),
                json!({"x": 120.0 + (index as f64 * 36.0), "y": 110.0 + (index as f64 * 22.0)}),
            );
        }
        nodes.push(node);
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
        "ConstructionScript" => "Construction Script",
        "CustomEvent" => "Custom Event",
        "CallEvent" => "Call Event",
        "BroadcastEvent" => "Broadcast Event",
        "Sequence" => "Sequence",
        "DoOnce" => "Do Once",
        "ResetDoOnce" => "Reset DoOnce",
        "Gate" => "Gate",
        "OpenGate" => "Open Gate",
        "CloseGate" => "Close Gate",
        "ToggleGate" => "Toggle Gate",
        "FlipFlop" => "Flip Flop",
        "SetVariable" => "Set Variable",
        "AddVariable" => "Add Variable",
        "ToggleVariable" => "Toggle Variable",
        "SetEnabled" => "Set Enabled",
        "MoveTowards" => "Move Towards",
        "SetVelocity" => "Set Velocity",
        "AddForce" => "Add Force",
        "StopMovement" => "Stop Movement",
        "SetSpeed" => "Set Speed",
        "SetPosition" => "Set Position",
        "SetRotation" => "Set Rotation",
        "SetScale" => "Set Scale",
        "SetHealth" => "Set Health",
        "BranchHealth" => "Branch Health",
        "BranchVariable" => "Branch Variable",
        "SetBlackboard" => "Set Blackboard",
        "ConfigureSpawner" => "Configure Spawner",
        "SetAnimation" => "Set Animation",
        "SetUiText" => "Set UI Text",
        "InventoryAdd" => "Inventory Add",
        "InventoryRemove" => "Inventory Remove",
        "BranchItem" => "Branch Item",
        "EquipItem" => "Equip Item",
        "EconomyAdd" => "Economy Add",
        "EconomySpend" => "Economy Spend",
        "BranchResource" => "Branch Resource",
        "AddProductionRecipe" => "Add Production Recipe",
        "SetPreferredRecipe" => "Set Preferred Recipe",
        "QueuePreferredRecipe" => "Queue Preferred Recipe",
        "AddQuest" => "Add Quest",
        "QuestProgress" => "Quest Progress",
        "TriggerAbility" => "Trigger Ability",
        "RechargeAbility" => "Recharge Ability",
        "StartCooldown" => "Start Cooldown",
        "SetState" => "Set State",
        "AddStatusEffect" => "Add Status Effect",
        "CompleteQuest" => "Complete Quest",
        "SetTag" => "Set Tag",
        "AddComponent" => "Add Component",
        "SetComponentNumber" => "Set Component Number",
        "DestroySelf" => "Destroy Self",
        other => other,
    }
    .to_string()
}

fn output_pins_for(node_type: &str) -> Vec<String> {
    match node_type {
        "BranchHealth" | "BranchVariable" | "BranchItem" | "BranchResource" | "EconomySpend"
        | "InventoryRemove" | "TriggerAbility" => {
            vec!["true".to_string(), "false".to_string()]
        }
        "Sequence" => vec!["then_0".to_string(), "then_1".to_string()],
        "FlipFlop" => vec!["a".to_string(), "b".to_string()],
        "DestroySelf" => Vec::new(),
        _ => vec!["exec".to_string()],
    }
}

fn graph_branch_keys() -> [(&'static str, &'static str); 6] {
    [
        ("true_next", "true"),
        ("false_next", "false"),
        ("then_0", "then_0"),
        ("then_1", "then_1"),
        ("a_next", "a"),
        ("b_next", "b"),
    ]
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

fn node_definition(
    node_type: &str,
    label: &str,
    category: &str,
    description: &str,
    default_node: Value,
) -> VisualGraphNodeDefinition {
    VisualGraphNodeDefinition {
        node_type: node_type.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        description: description.to_string(),
        default_node,
    }
}

fn node_catalog() -> Vec<VisualGraphNodeDefinition> {
    vec![
        node_definition(
            "EventStart",
            "Event Start",
            "Events",
            "Primer flujo cuando el blueprint arranca.",
            json!({"type": "EventStart", "next": null}),
        ),
        node_definition(
            "EventUpdate",
            "Event Update",
            "Events",
            "Flujo por frame para logica continua.",
            json!({"type": "EventUpdate", "next": null}),
        ),
        node_definition(
            "EventClick",
            "Event Click",
            "Events",
            "Entrada para botones o UI interactiva.",
            json!({"type": "EventClick", "next": null}),
        ),
        node_definition(
            "EventTrigger",
            "Event Trigger",
            "Events",
            "Entrada para colisiones o triggers.",
            json!({"type": "EventTrigger", "next": null}),
        ),
        node_definition(
            "ConstructionScript",
            "Construction Script",
            "Events",
            "Inicializacion estilo Unreal justo antes de Event Start.",
            json!({"type": "ConstructionScript", "next": null}),
        ),
        node_definition(
            "CustomEvent",
            "Custom Event",
            "Events",
            "Punto de entrada invocable por Call Event.",
            json!({"type": "CustomEvent", "event": "OnUsed", "next": null}),
        ),
        node_definition(
            "CallEvent",
            "Call Event",
            "Functions",
            "Salta a otro evento o subgrafo del blueprint.",
            json!({"type": "CallEvent", "event": "OnUsed", "next": null}),
        ),
        node_definition(
            "BroadcastEvent",
            "Broadcast Event",
            "Events",
            "Ejecuta todos los Custom Event con el mismo nombre y luego continua.",
            json!({"type": "BroadcastEvent", "event": "OnUsed", "next": null}),
        ),
        node_definition(
            "Sequence",
            "Sequence",
            "Flow",
            "Ejecuta dos salidas ordenadas como el nodo Sequence de Unreal.",
            json!({"type": "Sequence", "then_0": null, "then_1": null, "next": null}),
        ),
        node_definition(
            "DoOnce",
            "Do Once",
            "Flow",
            "Deja pasar la ejecucion una sola vez hasta Reset DoOnce.",
            json!({"type": "DoOnce", "key": "default", "next": null}),
        ),
        node_definition(
            "ResetDoOnce",
            "Reset DoOnce",
            "Flow",
            "Resetea una compuerta Do Once.",
            json!({"type": "ResetDoOnce", "key": "default", "next": null}),
        ),
        node_definition(
            "Gate",
            "Gate",
            "Flow",
            "Deja pasar la ejecucion solo cuando su compuerta esta abierta.",
            json!({"type": "Gate", "key": "main", "open": true, "next": null}),
        ),
        node_definition(
            "OpenGate",
            "Open Gate",
            "Flow",
            "Abre una compuerta Gate nombrada.",
            json!({"type": "OpenGate", "key": "main", "next": null}),
        ),
        node_definition(
            "CloseGate",
            "Close Gate",
            "Flow",
            "Cierra una compuerta Gate nombrada.",
            json!({"type": "CloseGate", "key": "main", "next": null}),
        ),
        node_definition(
            "ToggleGate",
            "Toggle Gate",
            "Flow",
            "Alterna una compuerta Gate nombrada.",
            json!({"type": "ToggleGate", "key": "main", "next": null}),
        ),
        node_definition(
            "FlipFlop",
            "Flip Flop",
            "Flow",
            "Alterna entre las salidas A y B en cada ejecucion.",
            json!({"type": "FlipFlop", "key": "main", "a_next": null, "b_next": null, "next": null}),
        ),
        node_definition(
            "Log",
            "Log",
            "Debug",
            "Escribe un mensaje en el runtime de visual scripting.",
            json!({"type": "Log", "message": "Blueprint running", "next": null}),
        ),
        node_definition(
            "Move",
            "Move",
            "Movement",
            "Mueve la entidad en X/Y; puede escalar por delta time.",
            json!({"type": "Move", "x": 1.0, "y": 0.0, "use_dt": false, "next": null}),
        ),
        node_definition(
            "MoveTowards",
            "Move Towards",
            "Movement",
            "Avanza hacia una posicion con velocidad controlada.",
            json!({"type": "MoveTowards", "target_x": 0.0, "target_y": 0.0, "speed": 4.0, "next": null}),
        ),
        node_definition(
            "SetVelocity",
            "Set Velocity",
            "Movement",
            "Crea/usa Rigidbody2D y asigna velocidad.",
            json!({"type": "SetVelocity", "x": 0.0, "y": 0.0, "next": null}),
        ),
        node_definition(
            "AddForce",
            "Add Force",
            "Movement",
            "Aplica fuerza o impulso al Rigidbody2D.",
            json!({"type": "AddForce", "x": 1.0, "y": 0.0, "impulse": true, "next": null}),
        ),
        node_definition(
            "StopMovement",
            "Stop Movement",
            "Movement",
            "Limpia rutas y velocidades para detener la entidad.",
            json!({"type": "StopMovement", "next": null}),
        ),
        node_definition(
            "SetSpeed",
            "Set Speed",
            "Movement",
            "Ajusta la velocidad usada por movimiento RTS y controladores.",
            json!({"type": "SetSpeed", "speed": 4.5, "next": null}),
        ),
        node_definition(
            "SetPosition",
            "Set Position",
            "Transform",
            "Coloca la entidad en una posicion exacta.",
            json!({"type": "SetPosition", "x": 0.0, "y": 0.0, "next": null}),
        ),
        node_definition(
            "SetRotation",
            "Set Rotation",
            "Transform",
            "Define la rotacion de la entidad.",
            json!({"type": "SetRotation", "rotation": 0.0, "next": null}),
        ),
        node_definition(
            "SetScale",
            "Set Scale",
            "Transform",
            "Define escala X/Y de la entidad.",
            json!({"type": "SetScale", "x": 1.0, "y": 1.0, "next": null}),
        ),
        node_definition(
            "Damage",
            "Damage",
            "Health",
            "Resta vida al componente Health de la entidad.",
            json!({"type": "Damage", "amount": 10.0, "next": null}),
        ),
        node_definition(
            "Heal",
            "Heal",
            "Health",
            "Cura la entidad hasta su max_health.",
            json!({"type": "Heal", "amount": 10.0, "next": null}),
        ),
        node_definition(
            "SetHealth",
            "Set Health",
            "Health",
            "Define vida actual y crea Health si hace falta.",
            json!({"type": "SetHealth", "health": 100.0, "max_health": 100.0, "next": null}),
        ),
        node_definition(
            "BranchHealth",
            "Branch Health",
            "Flow",
            "Elige true/false segun una comparacion de vida.",
            json!({"type": "BranchHealth", "operator": "<=", "value": 0.0, "true_next": null, "false_next": null}),
        ),
        node_definition(
            "BranchVariable",
            "Branch Variable",
            "Flow",
            "Elige true/false segun una variable del blueprint.",
            json!({"type": "BranchVariable", "name": "is_ready", "operator": "==", "value": true, "true_next": null, "false_next": null}),
        ),
        node_definition(
            "Wait",
            "Wait",
            "Flow",
            "Pausa la cadena hasta que pasen los segundos indicados.",
            json!({"type": "Wait", "seconds": 1.0, "next": null}),
        ),
        node_definition(
            "SetVariable",
            "Set Variable",
            "Variables",
            "Guarda un valor en variables del blueprint.",
            json!({"type": "SetVariable", "name": "score", "value": 0.0, "next": null}),
        ),
        node_definition(
            "AddVariable",
            "Add Variable",
            "Variables",
            "Suma un numero a una variable.",
            json!({"type": "AddVariable", "name": "score", "amount": 1.0, "next": null}),
        ),
        node_definition(
            "ToggleVariable",
            "Toggle Variable",
            "Variables",
            "Invierte una variable booleana.",
            json!({"type": "ToggleVariable", "name": "enabled", "next": null}),
        ),
        node_definition(
            "SetBlackboard",
            "Set Blackboard",
            "Gameplay",
            "Escribe un dato en el componente Blackboard.",
            json!({"type": "SetBlackboard", "key": "state", "value": "Ready", "next": null}),
        ),
        node_definition(
            "ConfigureSpawner",
            "Configure Spawner",
            "Gameplay",
            "Crea/configura un Spawner para oleadas o recursos.",
            json!({"type": "ConfigureSpawner", "prefab": "Enemy", "interval": 2.0, "radius": 2.0, "max_alive": 3, "spawn_on_start": true, "next": null}),
        ),
        node_definition(
            "SetAnimation",
            "Set Animation",
            "Presentation",
            "Cambia current_state en Animator.",
            json!({"type": "SetAnimation", "state": "Idle", "next": null}),
        ),
        node_definition(
            "SetUiText",
            "Set UI Text",
            "UI",
            "Actualiza el texto de UIElement en la entidad.",
            json!({"type": "SetUiText", "text": "Ready", "next": null}),
        ),
        node_definition(
            "InventoryAdd",
            "Inventory Add",
            "Gameplay",
            "Agrega items a Inventory.",
            json!({"type": "InventoryAdd", "item": "potion", "quantity": 1, "next": null}),
        ),
        node_definition(
            "InventoryRemove",
            "Inventory Remove",
            "Gameplay",
            "Quita items y bifurca si la operacion alcanza la cantidad pedida.",
            json!({"type": "InventoryRemove", "item": "potion", "quantity": 1, "true_next": null, "false_next": null}),
        ),
        node_definition(
            "BranchItem",
            "Branch Item",
            "Gameplay",
            "Comprueba si Inventory tiene un item y sale por true/false.",
            json!({"type": "BranchItem", "item": "potion", "quantity": 1, "true_next": null, "false_next": null}),
        ),
        node_definition(
            "EquipItem",
            "Equip Item",
            "Gameplay",
            "Equipa un item en Equipment y registra bonuses de stats.",
            json!({"type": "EquipItem", "slot": "weapon", "item": "iron_sword", "bonuses": {"attack": 4.0}, "next": null}),
        ),
        node_definition(
            "EconomyAdd",
            "Economy Add",
            "Economy",
            "Agrega recursos a EconomyWallet para oro, madera, comida u otros.",
            json!({"type": "EconomyAdd", "resource": "Gold", "amount": 100.0, "next": null}),
        ),
        node_definition(
            "EconomySpend",
            "Economy Spend",
            "Economy",
            "Gasta recursos y bifurca si alcanza o no alcanza.",
            json!({"type": "EconomySpend", "resource": "Gold", "amount": 50.0, "true_next": null, "false_next": null}),
        ),
        node_definition(
            "BranchResource",
            "Branch Resource",
            "Economy",
            "Comprueba recursos disponibles en EconomyWallet.",
            json!({"type": "BranchResource", "resource": "Gold", "amount": 50.0, "true_next": null, "false_next": null}),
        ),
        node_definition(
            "AddProductionRecipe",
            "Add Production Recipe",
            "RTS",
            "Registra o actualiza una receta de produccion RTS en la entidad.",
            json!({"type": "AddProductionRecipe", "unit": "Worker", "display": "Worker", "build_time": 3.0, "cost": {"Gold": 50.0}, "next": null}),
        ),
        node_definition(
            "SetPreferredRecipe",
            "Set Preferred Recipe",
            "RTS",
            "Define que receta usara Queue Preferred Recipe.",
            json!({"type": "SetPreferredRecipe", "unit": "Worker", "next": null}),
        ),
        node_definition(
            "QueuePreferredRecipe",
            "Queue Preferred Recipe",
            "RTS",
            "Encola la receta preferida en ProductionQueue.",
            json!({"type": "QueuePreferredRecipe", "next": null}),
        ),
        node_definition(
            "AddQuest",
            "Add Quest",
            "Narrative",
            "Agrega una quest activa con objetivos data-driven.",
            json!({"type": "AddQuest", "quest": "quest_01", "title": "New Quest", "objectives": [{"id": "objective_01", "text": "Do something", "progress": 0, "target": 1}], "next": null}),
        ),
        node_definition(
            "QuestProgress",
            "Quest Progress",
            "Narrative",
            "Actualiza progreso de un objetivo de quest.",
            json!({"type": "QuestProgress", "quest": "quest_01", "objective": "objective_01", "progress": 1, "next": null}),
        ),
        node_definition(
            "TriggerAbility",
            "Trigger Ability",
            "Gameplay",
            "Dispara Ability con cooldown/cargas y bifurca por exito.",
            json!({"type": "TriggerAbility", "true_next": null, "false_next": null}),
        ),
        node_definition(
            "RechargeAbility",
            "Recharge Ability",
            "Gameplay",
            "Recarga cargas de Ability sin escribir codigo.",
            json!({"type": "RechargeAbility", "amount": 1, "next": null}),
        ),
        node_definition(
            "StartCooldown",
            "Start Cooldown",
            "Gameplay",
            "Inicia un cooldown nombrado.",
            json!({"type": "StartCooldown", "name": "dash", "duration": 1.0, "next": null}),
        ),
        node_definition(
            "SetState",
            "Set State",
            "Gameplay",
            "Cambia entity.state y StateMachine.current_state si existe.",
            json!({"type": "SetState", "state": "Active", "next": null}),
        ),
        node_definition(
            "AddStatusEffect",
            "Add Status Effect",
            "Gameplay",
            "Agrega un efecto temporal de estado.",
            json!({"type": "AddStatusEffect", "name": "Burn", "duration": 2.0, "damage_per_second": 1.0, "next": null}),
        ),
        node_definition(
            "CompleteQuest",
            "Complete Quest",
            "Gameplay",
            "Marca una quest como completada en QuestLog.",
            json!({"type": "CompleteQuest", "quest": "quest_01", "next": null}),
        ),
        node_definition(
            "SetEnabled",
            "Set Enabled",
            "Entity",
            "Activa o desactiva la entidad.",
            json!({"type": "SetEnabled", "value": true, "next": null}),
        ),
        node_definition(
            "SetTag",
            "Set Tag",
            "Entity",
            "Cambia el tag de la entidad.",
            json!({"type": "SetTag", "tag": "Player", "next": null}),
        ),
        node_definition(
            "AddComponent",
            "Add Component",
            "Entity",
            "Agrega un componente built-in si no existe.",
            json!({"type": "AddComponent", "component": "Health", "next": null}),
        ),
        node_definition(
            "SetComponentNumber",
            "Set Component Number",
            "Entity",
            "Edita un campo numerico de un componente.",
            json!({"type": "SetComponentNumber", "component": "Stats", "field": "attack", "value": 10.0, "next": null}),
        ),
        node_definition(
            "DestroySelf",
            "Destroy Self",
            "Entity",
            "Desactiva y oculta la entidad actual.",
            json!({"type": "DestroySelf", "next": null}),
        ),
    ]
}

fn search_node_catalog(query: &str) -> Vec<VisualGraphNodeDefinition> {
    let query = query.trim().to_lowercase();
    let mut scored = node_catalog()
        .into_iter()
        .filter_map(|definition| {
            let score = fuzzy_score(
                &query,
                &[
                    &definition.node_type,
                    &definition.label,
                    &definition.category,
                    &definition.description,
                ],
            );
            if query.is_empty() || score >= 0.62 {
                Some((score, definition))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.category.cmp(&b.1.category))
            .then_with(|| a.1.label.cmp(&b.1.label))
    });
    scored
        .into_iter()
        .map(|(_, definition)| definition)
        .collect()
}

fn fuzzy_score(query: &str, fields: &[&str]) -> f64 {
    if query.is_empty() {
        return 1.0;
    }
    fields
        .iter()
        .map(|field| {
            let field = field.to_lowercase();
            if field.contains(query) {
                1.0
            } else {
                jaro_winkler(&field, query)
            }
        })
        .fold(0.0, f64::max)
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

fn graph_player_vital_movement() -> Value {
    graph_base(
        "PlayerVitalMovement",
        json!([
            {"id": "start", "type": "EventStart", "next": "health"},
            {"id": "health", "type": "SetHealth", "health": 100.0, "max_health": 100.0, "next": "speed"},
            {"id": "speed", "type": "SetSpeed", "speed": 5.0, "next": "log"},
            {"id": "log", "type": "Log", "message": "Player blueprint ready", "next": null},
            {"id": "update", "type": "EventUpdate", "next": "velocity"},
            {"id": "velocity", "type": "SetVelocity", "x": 0.0, "y": 0.0, "next": "alive"},
            {"id": "alive", "type": "BranchHealth", "operator": "<=", "value": 0.0, "true_next": "dead", "false_next": null},
            {"id": "dead", "type": "DestroySelf", "next": null}
        ]),
        json!({"speed": 5.0, "health": 100.0}),
    )
}

fn graph_health_combat() -> Value {
    graph_base(
        "HealthCombat",
        json!([
            {"id": "start", "type": "EventStart", "next": "set_health"},
            {"id": "set_health", "type": "SetHealth", "health": 75.0, "max_health": 100.0, "next": "log"},
            {"id": "log", "type": "Log", "message": "Combat graph armed", "next": null},
            {"id": "trigger", "type": "EventTrigger", "next": "damage"},
            {"id": "damage", "type": "Damage", "amount": 15.0, "next": "low_check"},
            {"id": "low_check", "type": "BranchHealth", "operator": "<=", "value": 25.0, "true_next": "heal", "false_next": null},
            {"id": "heal", "type": "Heal", "amount": 10.0, "next": null}
        ]),
        json!({"damage": 15.0, "low_health": 25.0}),
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
            {"id": "start", "type": "EventStart", "next": "setup"},
            {"id": "setup", "type": "ConfigureSpawner", "prefab": "Enemy", "interval": 2.0, "radius": 2.0, "max_alive": 3, "spawn_on_start": true, "next": "timer"},
            {"id": "timer", "type": "Wait", "seconds": 2.0, "next": "log"},
            {"id": "log", "type": "Log", "message": "Spawner tick", "next": null}
        ]),
        json!({"prefab": "Enemy", "interval": 2.0}),
    )
}

fn graph_blueprint_communication() -> Value {
    graph_base(
        "BlueprintCommunication",
        json!([
            {"id": "construction", "type": "ConstructionScript", "next": "setup_gate"},
            {"id": "setup_gate", "type": "OpenGate", "key": "ready", "next": null},
            {"id": "start", "type": "EventStart", "next": "broadcast"},
            {"id": "broadcast", "type": "BroadcastEvent", "event": "OnReady", "next": "gate"},
            {"id": "on_ready", "type": "CustomEvent", "event": "OnReady", "next": "ready_log"},
            {"id": "ready_log", "type": "Log", "message": "Custom event OnReady", "next": null},
            {"id": "gate", "type": "Gate", "key": "ready", "open": true, "next": "flip"},
            {"id": "flip", "type": "FlipFlop", "key": "step", "a_next": "state_a", "b_next": "state_b"},
            {"id": "state_a", "type": "SetState", "state": "StateA", "next": null},
            {"id": "state_b", "type": "SetState", "state": "StateB", "next": null}
        ]),
        json!({"ready": true, "step": false}),
    )
}

fn graph_inventory_economy_loop() -> Value {
    graph_base(
        "InventoryEconomyLoop",
        json!([
            {"id": "start", "type": "EventStart", "next": "seed_gold"},
            {"id": "seed_gold", "type": "EconomyAdd", "resource": "Gold", "amount": 120.0, "next": "add_potions"},
            {"id": "add_potions", "type": "InventoryAdd", "item": "potion", "quantity": 3, "next": "can_buy"},
            {"id": "can_buy", "type": "BranchResource", "resource": "Gold", "amount": 50.0, "true_next": "spend_gold", "false_next": "no_gold"},
            {"id": "spend_gold", "type": "EconomySpend", "resource": "Gold", "amount": 50.0, "true_next": "equip", "false_next": "no_gold"},
            {"id": "equip", "type": "EquipItem", "slot": "weapon", "item": "iron_sword", "bonuses": {"attack": 4.0}, "next": "log_success"},
            {"id": "no_gold", "type": "Log", "message": "Not enough Gold", "next": null},
            {"id": "log_success", "type": "Log", "message": "Inventory/economy loop ready", "next": null},
            {"id": "update", "type": "EventUpdate", "next": "has_potion"},
            {"id": "has_potion", "type": "BranchItem", "item": "potion", "quantity": 1, "true_next": "remove_potion", "false_next": null},
            {"id": "remove_potion", "type": "InventoryRemove", "item": "potion", "quantity": 1, "true_next": null, "false_next": null}
        ]),
        json!({"Gold": 120.0, "potion": 3}),
    )
}

fn graph_quest_ability_loop() -> Value {
    graph_base(
        "QuestAbilityLoop",
        json!([
            {"id": "start", "type": "EventStart", "next": "add_quest"},
            {"id": "add_quest", "type": "AddQuest", "quest": "first_steps", "title": "First Steps", "objectives": [{"id": "cast", "text": "Use an ability", "progress": 0, "target": 1}], "next": "setup_status"},
            {"id": "setup_status", "type": "SetState", "state": "QuestReady", "next": null},
            {"id": "update", "type": "EventUpdate", "next": "trigger"},
            {"id": "trigger", "type": "TriggerAbility", "true_next": "progress", "false_next": "recharge"},
            {"id": "progress", "type": "QuestProgress", "quest": "first_steps", "objective": "cast", "progress": 1, "next": "done"},
            {"id": "done", "type": "CompleteQuest", "quest": "first_steps", "next": null},
            {"id": "recharge", "type": "RechargeAbility", "amount": 1, "next": null}
        ]),
        json!({"quest": "first_steps", "ability_ready": true}),
    )
}

fn graph_rts_production_economy() -> Value {
    graph_base(
        "RTSProductionEconomy",
        json!([
            {"id": "start", "type": "EventStart", "next": "wallet"},
            {"id": "wallet", "type": "EconomyAdd", "resource": "Gold", "amount": 240.0, "next": "wood"},
            {"id": "wood", "type": "EconomyAdd", "resource": "Wood", "amount": 80.0, "next": "recipe_worker"},
            {"id": "recipe_worker", "type": "AddProductionRecipe", "unit": "Worker", "display": "Worker", "build_time": 3.0, "cost": {"Gold": 50.0}, "next": "recipe_soldier"},
            {"id": "recipe_soldier", "type": "AddProductionRecipe", "unit": "Soldier", "display": "Soldier", "build_time": 5.0, "cost": {"Gold": 85.0, "Wood": 25.0}, "next": "prefer"},
            {"id": "prefer", "type": "SetPreferredRecipe", "unit": "Worker", "next": "queue"},
            {"id": "queue", "type": "QueuePreferredRecipe", "next": "log"},
            {"id": "log", "type": "Log", "message": "RTS production economy ready", "next": null},
            {"id": "update", "type": "EventUpdate", "next": null}
        ]),
        json!({"Gold": 240.0, "Wood": 80.0}),
    )
}
