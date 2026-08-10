use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};
use crate::engine::miniforge_2d::validation::{ValidationReport2D, require_keys};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintGraph2D {
    pub name: String,
    pub runtime: String,
    #[serde(default = "default_blueprint_asset_kind")]
    pub asset_kind: String,
    #[serde(default = "default_parent_class")]
    pub parent_class: String,
    #[serde(default = "default_event_graph_type")]
    pub graph_type: String,
    #[serde(default)]
    pub class_settings: BlueprintClassSettings2D,
    #[serde(default)]
    pub components: Vec<BlueprintComponent2D>,
    #[serde(default)]
    pub interfaces: Vec<BlueprintInterfaceImplementation2D>,
    #[serde(default)]
    pub event_dispatchers: BTreeMap<String, BlueprintEventDispatcher2D>,
    #[serde(default)]
    pub macros: BTreeMap<String, BlueprintMacro2D>,
    #[serde(default)]
    pub variables: BTreeMap<String, BlueprintVariable2D>,
    #[serde(default)]
    pub functions: BTreeMap<String, BlueprintFunction2D>,
    #[serde(default)]
    pub nodes: Vec<BlueprintNode2D>,
    #[serde(default)]
    pub edges: Vec<BlueprintEdge2D>,
    #[serde(default)]
    pub comments: Vec<BlueprintCommentBox2D>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintVariable2D {
    pub value_type: String,
    #[serde(default)]
    pub default_value: Value,
    #[serde(default)]
    pub editable: bool,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tooltip: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub expose_on_spawn: bool,
    #[serde(default)]
    pub expose_to_cinematics: bool,
    #[serde(default)]
    pub replication: String,
    #[serde(default)]
    pub save_game: bool,
    #[serde(default)]
    pub transient: bool,
    #[serde(default)]
    pub advanced_display: bool,
    #[serde(default)]
    pub deprecated_message: String,
    #[serde(default)]
    pub slider_range: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintFunction2D {
    #[serde(default)]
    pub inputs: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub outputs: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub entry_node: String,
    #[serde(default = "default_public_access")]
    pub access: String,
    #[serde(default)]
    pub pure: bool,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintPin2D {
    pub name: String,
    pub pin_type: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub default_value: Value,
    #[serde(default)]
    pub by_ref: bool,
    #[serde(default)]
    pub pass_by_ref: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintEdge2D {
    pub from: String,
    pub from_pin: String,
    pub to: String,
    pub to_pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintCommentBox2D {
    pub id: String,
    pub title: String,
    pub body: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintClassSettings2D {
    #[serde(default)]
    pub blueprint_type: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tick_enabled: bool,
    #[serde(default)]
    pub replicated: bool,
    #[serde(default)]
    pub abstract_class: bool,
    #[serde(default)]
    pub deprecated_message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintComponent2D {
    pub name: String,
    pub component_type: String,
    #[serde(default)]
    pub parent: String,
    #[serde(default)]
    pub editable: bool,
    #[serde(default)]
    pub exposed_as_variable: bool,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintInterfaceImplementation2D {
    pub name: String,
    #[serde(default)]
    pub functions: Vec<BlueprintFunction2D>,
    #[serde(default)]
    pub replicates: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintEventDispatcher2D {
    #[serde(default)]
    pub inputs: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tooltip: String,
    #[serde(default)]
    pub copy_signature_from: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintMacro2D {
    #[serde(default)]
    pub inputs: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub outputs: Vec<BlueprintPin2D>,
    #[serde(default)]
    pub entry_node: String,
    #[serde(default)]
    pub exit_node: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instance_color: String,
    #[serde(default)]
    pub library: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintNodePaletteItem2D {
    pub kind: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub color: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintCompileSummary2D {
    pub node_count: usize,
    pub edge_count: usize,
    pub variable_count: usize,
    pub function_count: usize,
    #[serde(default)]
    pub component_count: usize,
    #[serde(default)]
    pub interface_count: usize,
    #[serde(default)]
    pub dispatcher_count: usize,
    #[serde(default)]
    pub macro_count: usize,
    #[serde(default)]
    pub comment_count: usize,
    #[serde(default)]
    pub reachable_node_count: usize,
    #[serde(default)]
    pub orphan_node_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintCompilerDiagnostic2D {
    pub severity: String,
    pub code: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintImplicitConversion2D {
    pub edge: String,
    pub from_type: String,
    pub to_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledBlueprint2D {
    pub compiler_version: u32,
    pub graph_name: String,
    pub runtime: String,
    pub entry_points: BTreeMap<String, String>,
    pub execution_order: Vec<String>,
    pub implicit_conversions: Vec<BlueprintImplicitConversion2D>,
    pub diagnostics: Vec<BlueprintCompilerDiagnostic2D>,
    pub valid: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintEditorSummary2D {
    pub compile: BlueprintCompileSummary2D,
    pub searchable_nodes: usize,
    pub selected_node_ids: Vec<String>,
    pub recommended_actions: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub orphan_node_ids: Vec<String>,
    #[serde(default)]
    pub event_node_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintGraphAnalysis2D {
    pub node_count: usize,
    pub edge_count: usize,
    pub component_count: usize,
    pub interface_count: usize,
    pub dispatcher_count: usize,
    pub macro_count: usize,
    pub comment_count: usize,
    pub event_node_ids: Vec<String>,
    pub reachable_node_ids: Vec<String>,
    pub orphan_node_ids: Vec<String>,
    pub variable_reads: Vec<String>,
    pub variable_writes: Vec<String>,
    pub unsupported_node_ids: Vec<String>,
    pub incompatible_edge_ids: Vec<String>,
    pub max_exec_depth: usize,
    pub palette_categories: Vec<String>,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintUnrealParitySummary2D {
    pub asset_kind: String,
    pub parent_class: String,
    pub graph_type: String,
    pub has_class_defaults: bool,
    pub public_variables: usize,
    pub private_variables: usize,
    pub exposed_on_spawn: usize,
    pub pure_functions: usize,
    pub impure_functions: usize,
    pub macro_count: usize,
    pub dispatcher_count: usize,
    pub interface_count: usize,
    pub component_count: usize,
    pub missing_unreal_like_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintPaletteSearchResult2D {
    pub item: BlueprintNodePaletteItem2D,
    pub score: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintConnectionSuggestion2D {
    pub node_kind: String,
    pub node_title: String,
    pub category: String,
    pub compatible_pin: String,
    pub pin_type: String,
    pub direction: String,
    pub score: usize,
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
        if self.asset_kind.trim().is_empty() {
            report.warning(
                "blueprint_asset_kind_empty",
                "asset_kind",
                "Blueprint sin tipo de asset; se asumira BlueprintClass.",
            );
        }
        if self.asset_kind == "BlueprintClass" && self.parent_class.trim().is_empty() {
            report.error(
                "parent_class_empty",
                "parent_class",
                "Blueprint Class necesita parent_class.",
            );
        }
        let mut component_names = BTreeSet::new();
        for component in &self.components {
            if component.name.trim().is_empty() || component.component_type.trim().is_empty() {
                report.error(
                    "component_invalid",
                    "components",
                    "Componente Blueprint sin nombre o tipo.",
                );
            }
            if !component_names.insert(component.name.clone()) {
                report.error(
                    "duplicate_component",
                    format!("components.{}", component.name),
                    format!("Componente duplicado: {}", component.name),
                );
            }
        }
        for (name, dispatcher) in &self.event_dispatchers {
            if name.trim().is_empty() {
                report.error(
                    "dispatcher_empty",
                    "event_dispatchers",
                    "Dispatcher sin nombre.",
                );
            }
            for pin in &dispatcher.inputs {
                if pin.name.trim().is_empty() || pin.pin_type.trim().is_empty() {
                    report.error(
                        "dispatcher_pin_invalid",
                        format!("event_dispatchers.{name}"),
                        "Dispatcher con parametro incompleto.",
                    );
                }
            }
        }
        for (name, macro_graph) in &self.macros {
            if macro_graph.entry_node.trim().is_empty() || macro_graph.exit_node.trim().is_empty() {
                report.warning(
                    "macro_tunnels_missing",
                    format!("macros.{name}"),
                    "Macro sin entry/exit tunnel; el editor no podra expandirla visualmente.",
                );
            }
        }
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
            if let (Some(from_node), Some(to_node)) =
                (self.node_by_id(&edge.from), self.node_by_id(&edge.to))
            {
                let from_pin = pin_by_name(from_node, &edge.from_pin);
                let to_pin = pin_by_name(to_node, &edge.to_pin);
                match (from_pin, to_pin) {
                    (Some(from_pin), Some(to_pin)) if !pins_are_compatible(from_pin, to_pin) => {
                        report.error(
                            "edge_pin_incompatible",
                            format!(
                                "edges.{}:{}->{}:{}",
                                edge.from, edge.from_pin, edge.to, edge.to_pin
                            ),
                            format!(
                                "Pin {} ({}) no es compatible con {} ({}).",
                                edge.from_pin, from_pin.pin_type, edge.to_pin, to_pin.pin_type
                            ),
                        );
                    }
                    (None, _) | (_, None) => {
                        report.error(
                            "edge_pin_missing",
                            format!(
                                "edges.{}:{}->{}:{}",
                                edge.from, edge.from_pin, edge.to, edge.to_pin
                            ),
                            "La conexión referencia un pin que no existe.",
                        );
                    }
                    _ => {}
                }
            }
        }
        for edge in duplicate_data_input_edges(self) {
            report.error(
                "multiple_data_sources",
                format!("edges.{}:{}", edge.to, edge.to_pin),
                format!(
                    "El pin de datos {}.{} tiene más de una fuente.",
                    edge.to, edge.to_pin
                ),
            );
        }
        if let Some(cycle) = self.exec_cycle() {
            report.error(
                "exec_cycle",
                "edges",
                format!(
                    "Ciclo de ejecución explícito detectado: {}",
                    cycle.join(" -> ")
                ),
            );
        }
        if !self.nodes.iter().any(|node| is_entry_node_kind(&node.kind)) {
            report.warning(
                "missing_event",
                "nodes",
                "Graph sin evento de entrada; no se ejecutara automaticamente.",
            );
        }
        report
    }

    pub fn compile_summary(&self) -> BlueprintCompileSummary2D {
        let report = self.validate();
        let analysis = self.analyze();
        BlueprintCompileSummary2D {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            variable_count: self.variables.len(),
            function_count: self.functions.len(),
            component_count: self.components.len(),
            interface_count: self.interfaces.len(),
            dispatcher_count: self.event_dispatchers.len(),
            macro_count: self.macros.len(),
            comment_count: self.comments.len(),
            reachable_node_count: analysis.reachable_node_ids.len(),
            orphan_node_count: analysis.orphan_node_ids.len(),
            error_count: report.error_count(),
            warning_count: report.warning_count(),
            runtime_ready: report.is_valid(),
        }
    }

    pub fn compile(&self) -> CompiledBlueprint2D {
        let validation = self.validate();
        let mut diagnostics = validation
            .issues
            .iter()
            .map(|issue| BlueprintCompilerDiagnostic2D {
                severity: format!("{:?}", issue.severity).to_ascii_lowercase(),
                code: issue.code.clone(),
                subject: issue.path.clone(),
                message: issue.message.clone(),
            })
            .collect::<Vec<_>>();
        let entry_points = self
            .nodes
            .iter()
            .filter(|node| is_entry_node_kind(&node.kind))
            .map(|node| (node.kind.clone(), node.id.clone()))
            .collect::<BTreeMap<_, _>>();
        let execution_order = self.execution_order();
        let mut implicit_conversions = Vec::new();
        for edge in &self.edges {
            let Some(from) = self
                .node_by_id(&edge.from)
                .and_then(|node| pin_by_name(node, &edge.from_pin))
            else {
                continue;
            };
            let Some(to) = self
                .node_by_id(&edge.to)
                .and_then(|node| pin_by_name(node, &edge.to_pin))
            else {
                continue;
            };
            if from.pin_type != to.pin_type && pins_are_compatible(from, to) {
                implicit_conversions.push(BlueprintImplicitConversion2D {
                    edge: format!(
                        "{}:{}->{}:{}",
                        edge.from, edge.from_pin, edge.to, edge.to_pin
                    ),
                    from_type: from.pin_type.clone(),
                    to_type: to.pin_type.clone(),
                });
            }
        }
        for node in self
            .nodes
            .iter()
            .filter(|node| !execution_order.contains(&node.id))
        {
            diagnostics.push(BlueprintCompilerDiagnostic2D {
                severity: "warning".to_string(),
                code: "node_not_scheduled".to_string(),
                subject: node.id.clone(),
                message: "El nodo no es alcanzable desde un punto de entrada.".to_string(),
            });
        }
        let valid = !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error");
        CompiledBlueprint2D {
            compiler_version: 2,
            graph_name: self.name.clone(),
            runtime: "miniforge_blueprint_vm_v2".to_string(),
            entry_points,
            execution_order,
            implicit_conversions,
            diagnostics,
            valid,
        }
    }

    pub fn add_node(
        &mut self,
        kind: &str,
        title: impl Into<String>,
        x: f64,
        y: f64,
        data: Value,
    ) -> Option<String> {
        if !supported_node_kinds().contains(kind) {
            return None;
        }
        let id = unique_node_id(&self.nodes, kind);
        self.nodes.push(BlueprintNode2D {
            id: id.clone(),
            kind: kind.to_string(),
            title: title.into(),
            x,
            y,
            pins: default_pins_for_kind(kind),
            data,
        });
        Some(id)
    }

    pub fn add_comment_box(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        x: f64,
        y: f64,
        node_ids: Vec<String>,
    ) -> String {
        let id = unique_comment_id(&self.comments);
        self.comments.push(BlueprintCommentBox2D {
            id: id.clone(),
            title: title.into(),
            body: body.into(),
            x,
            y,
            width: 360.0,
            height: 180.0,
            color: "#334155".to_string(),
            node_ids,
        });
        id
    }

    pub fn remove_comment_box(&mut self, comment_id: &str) -> bool {
        let before = self.comments.len();
        self.comments.retain(|comment| comment.id != comment_id);
        self.comments.len() != before
    }

    pub fn connect_nodes(&mut self, from: &str, from_pin: &str, to: &str, to_pin: &str) -> bool {
        if !self.nodes.iter().any(|node| node.id == from)
            || !self.nodes.iter().any(|node| node.id == to)
        {
            return false;
        }
        if self.edges.iter().any(|edge| {
            edge.from == from && edge.from_pin == from_pin && edge.to == to && edge.to_pin == to_pin
        }) {
            return true;
        }
        self.edges.push(BlueprintEdge2D {
            from: from.to_string(),
            from_pin: from_pin.to_string(),
            to: to.to_string(),
            to_pin: to_pin.to_string(),
        });
        true
    }

    pub fn connect_nodes_checked(
        &mut self,
        from: &str,
        from_pin: &str,
        to: &str,
        to_pin: &str,
    ) -> Result<bool, String> {
        let from_node = self
            .node_by_id(from)
            .ok_or_else(|| format!("Nodo origen inexistente: {from}"))?;
        let to_node = self
            .node_by_id(to)
            .ok_or_else(|| format!("Nodo destino inexistente: {to}"))?;
        let from_pin_def = pin_by_name(from_node, from_pin)
            .ok_or_else(|| format!("Pin origen inexistente: {from}.{from_pin}"))?;
        let to_pin_def = pin_by_name(to_node, to_pin)
            .ok_or_else(|| format!("Pin destino inexistente: {to}.{to_pin}"))?;
        if from_pin_def.direction == "in" || to_pin_def.direction == "out" {
            return Err(
                "Las conexiones deben ir de un pin de salida a uno de entrada.".to_string(),
            );
        }
        if !pins_are_compatible(from_pin_def, to_pin_def) {
            return Err(format!(
                "{}:{} ({}) no conecta con {}:{} ({}).",
                from, from_pin, from_pin_def.pin_type, to, to_pin, to_pin_def.pin_type
            ));
        }
        if to_pin_def.pin_type != "exec"
            && self
                .edges
                .iter()
                .any(|edge| edge.to == to && edge.to_pin == to_pin)
        {
            return Err(format!(
                "El pin de datos {to}.{to_pin} ya tiene una fuente."
            ));
        }
        let from_is_exec = from_pin_def.pin_type == "exec";
        let connected = self.connect_nodes(from, from_pin, to, to_pin);
        if connected && from_is_exec && self.exec_cycle().is_some() {
            self.edges.retain(|edge| {
                !(edge.from == from
                    && edge.from_pin == from_pin
                    && edge.to == to
                    && edge.to_pin == to_pin)
            });
            return Err("La conexión crearía un ciclo explícito de ejecución.".to_string());
        }
        Ok(connected)
    }

    /// Removes one exact connection. This is the primitive used by an
    /// advanced graph editor for Alt-click disconnect and undoable rewiring.
    pub fn disconnect_nodes(&mut self, from: &str, from_pin: &str, to: &str, to_pin: &str) -> bool {
        let before = self.edges.len();
        self.edges.retain(|edge| {
            !(edge.from == from
                && edge.from_pin == from_pin
                && edge.to == to
                && edge.to_pin == to_pin)
        });
        self.edges.len() != before
    }

    /// Breaks every link touching a pin and returns how many links were
    /// removed. It works for both input and output pins.
    pub fn break_pin_links(&mut self, node_id: &str, pin_name: &str) -> usize {
        let before = self.edges.len();
        self.edges.retain(|edge| {
            !((edge.from == node_id && edge.from_pin == pin_name)
                || (edge.to == node_id && edge.to_pin == pin_name))
        });
        before - self.edges.len()
    }

    /// Produces context-sensitive palette entries for a dragged pin. Results
    /// only include node pins that can be connected in the correct direction.
    pub fn connection_suggestions(
        &self,
        node_id: &str,
        pin_name: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<BlueprintConnectionSuggestion2D>, String> {
        let source_node = self
            .node_by_id(node_id)
            .ok_or_else(|| format!("Nodo inexistente: {node_id}"))?;
        let source_pin = pin_by_name(source_node, pin_name)
            .ok_or_else(|| format!("Pin inexistente: {node_id}.{pin_name}"))?;
        let mut suggestions = Vec::new();
        let mut seen = BTreeSet::new();
        for result in search_node_palette_ranked(query) {
            for candidate in default_pins_for_kind(&result.item.kind) {
                let compatible = if source_pin.direction == "out" && candidate.direction == "in" {
                    pins_are_compatible(source_pin, &candidate)
                } else if source_pin.direction == "in" && candidate.direction == "out" {
                    pins_are_compatible(&candidate, source_pin)
                } else {
                    false
                };
                if !compatible || !seen.insert((result.item.kind.clone(), candidate.name.clone())) {
                    continue;
                }
                let type_penalty = usize::from(source_pin.pin_type != candidate.pin_type) * 5;
                suggestions.push(BlueprintConnectionSuggestion2D {
                    node_kind: result.item.kind.clone(),
                    node_title: result.item.title.clone(),
                    category: result.item.category.clone(),
                    compatible_pin: candidate.name,
                    pin_type: candidate.pin_type,
                    direction: candidate.direction,
                    score: result.score + type_penalty,
                });
            }
        }
        suggestions.sort_by(|left, right| {
            left.score
                .cmp(&right.score)
                .then_with(|| left.node_title.cmp(&right.node_title))
                .then_with(|| left.compatible_pin.cmp(&right.compatible_pin))
        });
        suggestions.truncate(limit);
        Ok(suggestions)
    }

    pub fn add_component(
        &mut self,
        name: impl Into<String>,
        component_type: impl Into<String>,
        parent: impl Into<String>,
    ) -> bool {
        let name = name.into();
        if name.trim().is_empty()
            || self
                .components
                .iter()
                .any(|component| component.name == name)
        {
            return false;
        }
        self.components.push(BlueprintComponent2D {
            name,
            component_type: component_type.into(),
            parent: parent.into(),
            editable: true,
            exposed_as_variable: true,
            properties: BTreeMap::new(),
        });
        true
    }

    pub fn add_variable_with_properties(
        &mut self,
        name: impl Into<String>,
        value_type: impl Into<String>,
        default_value: Value,
        editable: bool,
        category: impl Into<String>,
    ) -> bool {
        let name = name.into();
        if name.trim().is_empty() || self.variables.contains_key(&name) {
            return false;
        }
        self.variables.insert(
            name,
            BlueprintVariable2D {
                value_type: value_type.into(),
                default_value,
                editable,
                category: category.into(),
                ..Default::default()
            },
        );
        true
    }

    pub fn promote_pin_to_variable(
        &mut self,
        node_id: &str,
        pin_name: &str,
        variable_name: impl Into<String>,
    ) -> bool {
        let Some(node) = self.node_by_id(node_id) else {
            return false;
        };
        let Some(pin) = pin_by_name(node, pin_name) else {
            return false;
        };
        let variable_name = variable_name.into();
        self.add_variable_with_properties(
            variable_name,
            pin.pin_type.clone(),
            pin.default_value.clone(),
            true,
            "Promoted Pins",
        )
    }

    pub fn add_function(
        &mut self,
        name: impl Into<String>,
        inputs: Vec<BlueprintPin2D>,
        outputs: Vec<BlueprintPin2D>,
        pure: bool,
    ) -> bool {
        let name = name.into();
        if name.trim().is_empty() || self.functions.contains_key(&name) {
            return false;
        }
        self.functions.insert(
            name,
            BlueprintFunction2D {
                inputs,
                outputs,
                pure,
                access: default_public_access(),
                category: "Functions".to_string(),
                ..Default::default()
            },
        );
        true
    }

    pub fn add_macro(
        &mut self,
        name: impl Into<String>,
        inputs: Vec<BlueprintPin2D>,
        outputs: Vec<BlueprintPin2D>,
    ) -> bool {
        let name = name.into();
        if name.trim().is_empty() || self.macros.contains_key(&name) {
            return false;
        }
        self.macros.insert(
            name.clone(),
            BlueprintMacro2D {
                inputs,
                outputs,
                entry_node: format!("{name}_entry"),
                exit_node: format!("{name}_exit"),
                category: "Macros".to_string(),
                instance_color: "gray".to_string(),
                ..Default::default()
            },
        );
        true
    }

    pub fn add_event_dispatcher(
        &mut self,
        name: impl Into<String>,
        inputs: Vec<BlueprintPin2D>,
    ) -> bool {
        let name = name.into();
        if name.trim().is_empty() || self.event_dispatchers.contains_key(&name) {
            return false;
        }
        self.event_dispatchers.insert(
            name,
            BlueprintEventDispatcher2D {
                inputs,
                category: "Dispatchers".to_string(),
                tooltip: "Multicast event dispatcher.".to_string(),
                copy_signature_from: String::new(),
            },
        );
        true
    }

    pub fn implement_interface(
        &mut self,
        name: impl Into<String>,
        functions: Vec<BlueprintFunction2D>,
    ) -> bool {
        let name = name.into();
        if name.trim().is_empty()
            || self
                .interfaces
                .iter()
                .any(|interface| interface.name == name)
        {
            return false;
        }
        self.interfaces.push(BlueprintInterfaceImplementation2D {
            name,
            functions,
            replicates: false,
        });
        true
    }

    pub fn node_by_id(&self, node_id: &str) -> Option<&BlueprintNode2D> {
        self.nodes.iter().find(|node| node.id == node_id)
    }

    pub fn node_by_id_mut(&mut self, node_id: &str) -> Option<&mut BlueprintNode2D> {
        self.nodes.iter_mut().find(|node| node.id == node_id)
    }

    pub fn disconnect_node(&mut self, node_id: &str) -> usize {
        let before = self.edges.len();
        self.edges
            .retain(|edge| edge.from != node_id && edge.to != node_id);
        before - self.edges.len()
    }

    pub fn remove_node(&mut self, node_id: &str) -> bool {
        let before = self.nodes.len();
        self.nodes.retain(|node| node.id != node_id);
        if self.nodes.len() == before {
            return false;
        }
        self.disconnect_node(node_id);
        true
    }

    pub fn duplicate_node(
        &mut self,
        node_id: &str,
        offset_x: f64,
        offset_y: f64,
    ) -> Option<String> {
        let source = self.node_by_id(node_id)?.clone();
        let new_id = unique_node_id(&self.nodes, &source.kind);
        let mut duplicate = source;
        duplicate.id = new_id.clone();
        duplicate.title = if duplicate.title.ends_with(" Copy") {
            duplicate.title
        } else {
            format!("{} Copy", duplicate.title)
        };
        duplicate.x += offset_x;
        duplicate.y += offset_y;
        self.nodes.push(duplicate);
        Some(new_id)
    }

    pub fn quick_add_node(&mut self, query: &str, x: f64, y: f64) -> Option<String> {
        let item = search_node_palette(query).into_iter().next()?;
        self.add_node(&item.kind, item.title, x, y, json!({}))
    }

    pub fn connect_exec_chain(&mut self, node_ids: &[String]) -> usize {
        let mut connected = 0usize;
        for pair in node_ids.windows(2) {
            if self.connect_nodes(&pair[0], "then", &pair[1], "exec") {
                connected += 1;
            }
        }
        connected
    }

    pub fn editor_summary(&self, selected_node_ids: Vec<String>) -> BlueprintEditorSummary2D {
        let compile = self.compile_summary();
        let analysis = self.analyze();
        let mut recommended_actions = Vec::new();
        if !compile.runtime_ready {
            recommended_actions.push("open_problems_panel".to_string());
        }
        if !self.nodes.iter().any(|node| is_entry_node_kind(&node.kind)) {
            recommended_actions.push("add_event_node".to_string());
        }
        if !analysis.orphan_node_ids.is_empty() {
            recommended_actions.push("review_orphan_nodes".to_string());
        }
        if self.edges.is_empty() && self.nodes.len() > 1 {
            recommended_actions.push("connect_exec_chain".to_string());
        }
        recommended_actions.push("auto_layout".to_string());
        recommended_actions.push("add_comment_box".to_string());
        recommended_actions.push("attach_to_selected_actor".to_string());
        BlueprintEditorSummary2D {
            compile,
            searchable_nodes: blueprint_node_palette().len(),
            selected_node_ids,
            recommended_actions,
            categories: analysis.palette_categories,
            orphan_node_ids: analysis.orphan_node_ids,
            event_node_ids: analysis.event_node_ids,
        }
    }

    pub fn analyze(&self) -> BlueprintGraphAnalysis2D {
        let supported = supported_node_kinds();
        let event_node_ids = self
            .nodes
            .iter()
            .filter(|node| is_entry_node_kind(&node.kind))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let reachable = self.reachable_node_ids();
        let reachable_set = reachable.iter().cloned().collect::<BTreeSet<_>>();
        let orphan_node_ids = self
            .nodes
            .iter()
            .filter(|node| !reachable_set.contains(&node.id))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let unsupported_node_ids = self
            .nodes
            .iter()
            .filter(|node| !supported.contains(node.kind.as_str()))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let incompatible_edge_ids = self
            .edges
            .iter()
            .filter_map(|edge| {
                let from = self.node_by_id(&edge.from)?;
                let to = self.node_by_id(&edge.to)?;
                let from_pin = pin_by_name(from, &edge.from_pin)?;
                let to_pin = pin_by_name(to, &edge.to_pin)?;
                (!pins_are_compatible(from_pin, to_pin)).then(|| {
                    format!(
                        "{}:{}->{}:{}",
                        edge.from, edge.from_pin, edge.to, edge.to_pin
                    )
                })
            })
            .collect::<Vec<_>>();
        let (variable_reads, variable_writes) = self.variable_references();
        let mut recommended_actions = Vec::new();
        if event_node_ids.is_empty() {
            recommended_actions.push("add_event_node".to_string());
        }
        if !orphan_node_ids.is_empty() {
            recommended_actions.push("connect_or_remove_orphans".to_string());
        }
        if !unsupported_node_ids.is_empty() {
            recommended_actions.push("replace_unsupported_nodes".to_string());
        }
        if !incompatible_edge_ids.is_empty() {
            recommended_actions.push("fix_incompatible_pins".to_string());
        }
        if self.edges.is_empty() && self.nodes.len() > 1 {
            recommended_actions.push("connect_exec_chain".to_string());
        }
        if self.class_settings.category.is_empty() {
            recommended_actions.push("set_blueprint_category".to_string());
        }
        if self.comments.is_empty() && self.nodes.len() > 4 {
            recommended_actions.push("add_comment_box".to_string());
        }
        recommended_actions.push("auto_layout".to_string());
        BlueprintGraphAnalysis2D {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            component_count: self.components.len(),
            interface_count: self.interfaces.len(),
            dispatcher_count: self.event_dispatchers.len(),
            macro_count: self.macros.len(),
            comment_count: self.comments.len(),
            event_node_ids,
            reachable_node_ids: reachable,
            orphan_node_ids,
            variable_reads,
            variable_writes,
            unsupported_node_ids,
            incompatible_edge_ids,
            max_exec_depth: self.max_exec_depth(),
            palette_categories: palette_categories(),
            recommended_actions,
        }
    }

    pub fn reachable_node_ids(&self) -> Vec<String> {
        let mut visited = BTreeSet::<String>::new();
        let mut stack = self
            .nodes
            .iter()
            .filter(|node| is_entry_node_kind(&node.kind))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        while let Some(node_id) = stack.pop() {
            if !visited.insert(node_id.clone()) {
                continue;
            }
            for edge in self.edges.iter().filter(|edge| edge.from == node_id) {
                stack.push(edge.to.clone());
            }
        }
        visited.into_iter().collect()
    }

    pub fn execution_order(&self) -> Vec<String> {
        let mut order = Vec::new();
        let mut visited = BTreeSet::new();
        let mut entries = self
            .nodes
            .iter()
            .filter(|node| is_entry_node_kind(&node.kind))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            self.append_execution_order(&entry, &mut visited, &mut order);
        }
        order
    }

    fn append_execution_order(
        &self,
        node_id: &str,
        visited: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) {
        if !visited.insert(node_id.to_string()) {
            return;
        }
        order.push(node_id.to_string());
        let mut targets = self
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .filter(|edge| {
                self.node_by_id(&edge.from)
                    .and_then(|node| pin_by_name(node, &edge.from_pin))
                    .is_some_and(|pin| pin.pin_type == "exec")
            })
            .map(|edge| edge.to.clone())
            .collect::<Vec<_>>();
        targets.sort();
        for target in targets {
            self.append_execution_order(&target, visited, order);
        }
    }

    pub fn exec_cycle(&self) -> Option<Vec<String>> {
        fn visit(
            graph: &BlueprintGraph2D,
            node: &str,
            visiting: &mut Vec<String>,
            done: &mut BTreeSet<String>,
        ) -> Option<Vec<String>> {
            if let Some(index) = visiting.iter().position(|current| current == node) {
                let mut cycle = visiting[index..].to_vec();
                cycle.push(node.to_string());
                return Some(cycle);
            }
            if done.contains(node) {
                return None;
            }
            visiting.push(node.to_string());
            for edge in graph.edges.iter().filter(|edge| edge.from == node) {
                let is_exec = graph
                    .node_by_id(&edge.from)
                    .and_then(|from| pin_by_name(from, &edge.from_pin))
                    .is_some_and(|pin| pin.pin_type == "exec");
                if is_exec && let Some(cycle) = visit(graph, &edge.to, visiting, done) {
                    return Some(cycle);
                }
            }
            visiting.pop();
            done.insert(node.to_string());
            None
        }
        let mut done = BTreeSet::new();
        for node in &self.nodes {
            if let Some(cycle) = visit(self, &node.id, &mut Vec::new(), &mut done) {
                return Some(cycle);
            }
        }
        None
    }

    pub fn variable_references(&self) -> (Vec<String>, Vec<String>) {
        let mut reads = BTreeSet::new();
        let mut writes = BTreeSet::new();
        for node in &self.nodes {
            let Some(name) = node.data.get("name").and_then(Value::as_str) else {
                continue;
            };
            match node.kind.as_str() {
                "GetVariable" | "GetBlackboardValue" => {
                    reads.insert(name.to_string());
                }
                "SetVariable" | "SetBlackboardValue" => {
                    writes.insert(name.to_string());
                }
                _ => {}
            }
        }
        (reads.into_iter().collect(), writes.into_iter().collect())
    }

    pub fn max_exec_depth(&self) -> usize {
        let starts = self
            .nodes
            .iter()
            .filter(|node| is_entry_node_kind(&node.kind))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let mut best = 0usize;
        for start in starts {
            let mut stack = vec![(start, 0usize)];
            let mut seen = BTreeSet::<String>::new();
            while let Some((node_id, depth)) = stack.pop() {
                best = best.max(depth);
                if depth >= self.nodes.len() || !seen.insert(node_id.clone()) {
                    continue;
                }
                for edge in self.edges.iter().filter(|edge| edge.from == node_id) {
                    stack.push((edge.to.clone(), depth + 1));
                }
            }
        }
        best
    }

    pub fn unreal_parity_summary(&self) -> BlueprintUnrealParitySummary2D {
        let public_variables = self
            .variables
            .values()
            .filter(|variable| !variable.private)
            .count();
        let private_variables = self
            .variables
            .values()
            .filter(|variable| variable.private)
            .count();
        let exposed_on_spawn = self
            .variables
            .values()
            .filter(|variable| variable.expose_on_spawn)
            .count();
        let pure_functions = self
            .functions
            .values()
            .filter(|function| function.pure)
            .count();
        let impure_functions = self.functions.len().saturating_sub(pure_functions);
        let mut missing_unreal_like_features = Vec::new();
        if self.parent_class.trim().is_empty() {
            missing_unreal_like_features.push("parent_class".to_string());
        }
        if self.components.is_empty() {
            missing_unreal_like_features.push("components_panel".to_string());
        }
        if self.class_settings.category.is_empty() {
            missing_unreal_like_features.push("class_settings_category".to_string());
        }
        if self.functions.is_empty() {
            missing_unreal_like_features.push("functions".to_string());
        }
        if self.event_dispatchers.is_empty() {
            missing_unreal_like_features.push("event_dispatchers".to_string());
        }
        if self.interfaces.is_empty() {
            missing_unreal_like_features.push("interfaces".to_string());
        }
        BlueprintUnrealParitySummary2D {
            asset_kind: self.asset_kind.clone(),
            parent_class: self.parent_class.clone(),
            graph_type: self.graph_type.clone(),
            has_class_defaults: !self.class_settings.description.is_empty()
                || self.class_settings.tick_enabled
                || self.class_settings.replicated,
            public_variables,
            private_variables,
            exposed_on_spawn,
            pure_functions,
            impure_functions,
            macro_count: self.macros.len(),
            dispatcher_count: self.event_dispatchers.len(),
            interface_count: self.interfaces.len(),
            component_count: self.components.len(),
            missing_unreal_like_features,
        }
    }

    pub fn auto_layout(&mut self) {
        let mut depths = BTreeMap::<String, usize>::new();
        for node in &self.nodes {
            if is_entry_node_kind(&node.kind) {
                depths.insert(node.id.clone(), 0);
            }
        }
        let mut changed = true;
        while changed {
            changed = false;
            for edge in &self.edges {
                let Some(from_depth) = depths.get(&edge.from).copied() else {
                    continue;
                };
                let target_depth = from_depth + 1;
                if depths.get(&edge.to).copied().unwrap_or(0) < target_depth {
                    depths.insert(edge.to.clone(), target_depth);
                    changed = true;
                }
            }
        }
        let mut rows = BTreeMap::<usize, usize>::new();
        for (index, node) in self.nodes.iter_mut().enumerate() {
            let depth = depths.get(&node.id).copied().unwrap_or(index);
            let row = rows.entry(depth).or_insert(0);
            node.x = depth as f64 * 320.0;
            node.y = *row as f64 * 150.0;
            *row += 1;
        }
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
        "ConstructionScript",
        "CustomEvent",
        "FunctionEntry",
        "CallFunction",
        "MacroEntry",
        "MacroExit",
        "MacroInstance",
        "Comment",
        "Reroute",
        "Branch",
        "Delay",
        "Sequence",
        "ForLoop",
        "ForEachLoop",
        "SetVariable",
        "GetVariable",
        "Self",
        "ParentFunctionCall",
        "CastTo",
        "MakeStruct",
        "BreakStruct",
        "SpawnActor2D",
        "DestroyActor",
        "DestroySelf",
        "AddComponent",
        "SetTransform2D",
        "SetActorLocation2D",
        "SetSprite2D",
        "SetMaterial2D",
        "SetTextureSlot2D",
        "AddForce2D",
        "SetVelocity2D",
        "PlayAnimation2D",
        "PlaySound",
        "PlaySound2D",
        "ApplyDamage",
        "SetAnimationParameter",
        "SetUiText",
        "SetWidgetText",
        "SetWidgetVisibility",
        "SetWidgetPercent",
        "OpenMenu",
        "CloseMenu",
        "PushMenu",
        "PopMenu",
        "SetMenuState",
        "CreateWidget",
        "OpenWidget",
        "CreateMainMenu",
        "CallInterfaceFunction",
        "BlueprintInterfaceCall",
        "CallEventDispatcher",
        "BindEventDispatcher",
        "UnbindEventDispatcher",
        "AssignEventDispatcher",
        "LoadScene",
        "SaveGame",
        "LoadGame",
        "SaveCheckpoint",
        "LoadCheckpoint",
        "FindEntityByTag",
        "GetComponent",
        "SetComponentField",
        "SetBlackboardValue",
        "GetBlackboardValue",
        "AIMoveTo",
        "Patrol",
        "RunBehaviorTree",
        "InventoryAdd",
        "SetPhysicsEnabled",
        "Raycast2D",
        "LineTrace2D",
        "OverlapCircle2D",
        "ForEachEntityWithTag",
        "ForEachComponent",
        "SpawnProjectile2D",
        "PlayTimeline2D",
        "SetMaterialParameter2D",
        "StartParticleSystem2D",
        "BindUiEvent",
        "BindInputAction",
        "CallLuauFunction",
        "AsyncLoadScene",
        "WaitUntil",
        "SetTimer",
        "ClearTimer",
        "Timeline",
        "TimelineUpdate",
        "TweenPosition2D",
        "TweenColor",
        "GameplayTagHas",
        "ApplyGameplayEffect2D",
        "Breakpoint",
        "ProfilerMarker",
        "PrintString",
    ])
}

pub fn blueprint_node_palette() -> Vec<BlueprintNodePaletteItem2D> {
    [
        (
            "EventBeginPlay",
            "Events",
            "Begin Play",
            "Runs once when actor starts.",
            "green",
            &["start", "ready"][..],
        ),
        (
            "EventTick",
            "Events",
            "Tick",
            "Runs every frame with delta time.",
            "green",
            &["update", "frame"][..],
        ),
        (
            "EventInput",
            "Events",
            "Input Action",
            "Responds to an input action.",
            "green",
            &["key", "button", "controls"][..],
        ),
        (
            "ConstructionScript",
            "Events",
            "Construction Script",
            "Runs in editor when the actor is constructed.",
            "green",
            &["construction", "editor", "defaults"][..],
        ),
        (
            "CustomEvent",
            "Events",
            "Custom Event",
            "Creates a named event entry point.",
            "green",
            &["event", "call", "dispatch"][..],
        ),
        (
            "Branch",
            "Flow",
            "Branch",
            "Executes true or false flow.",
            "blue",
            &["if", "condition"][..],
        ),
        (
            "Sequence",
            "Flow",
            "Sequence",
            "Runs multiple outputs in order.",
            "blue",
            &["then", "flow"][..],
        ),
        (
            "ForLoop",
            "Flow",
            "For Loop",
            "Runs an exec body for a numeric range.",
            "blue",
            &["loop", "index", "range"][..],
        ),
        (
            "ForEachLoop",
            "Flow",
            "For Each Loop",
            "Runs once per item in an array.",
            "blue",
            &["loop", "array", "foreach"][..],
        ),
        (
            "Delay",
            "Flow",
            "Delay",
            "Waits before continuing execution.",
            "blue",
            &["timer", "wait"][..],
        ),
        (
            "MacroInstance",
            "Flow",
            "Macro Instance",
            "Expands a reusable graph macro.",
            "blue",
            &["macro", "tunnel", "collapse"][..],
        ),
        (
            "SetTimer",
            "Flow",
            "Set Timer",
            "Schedules delayed or looping execution.",
            "blue",
            &["timer", "loop", "delegate"][..],
        ),
        (
            "Timeline",
            "Flow",
            "Timeline",
            "Drives values over time.",
            "blue",
            &["curve", "animation", "update"][..],
        ),
        (
            "SpawnActor2D",
            "Actor",
            "Spawn Actor 2D",
            "Spawns a prefab or actor.",
            "orange",
            &["spawn", "prefab"][..],
        ),
        (
            "DestroyActor",
            "Actor",
            "Destroy Actor",
            "Destroys an entity.",
            "orange",
            &["delete", "remove"][..],
        ),
        (
            "SetTransform2D",
            "Actor",
            "Set Transform",
            "Moves or rotates an entity.",
            "orange",
            &["move", "position"][..],
        ),
        (
            "SetActorLocation2D",
            "Actor",
            "Set Actor Location",
            "Moves an actor to a 2D location.",
            "orange",
            &["move", "location", "position"][..],
        ),
        (
            "SetSprite2D",
            "Actor",
            "Set Sprite",
            "Swaps the SpriteRenderer asset.",
            "orange",
            &["sprite", "texture", "visual"][..],
        ),
        (
            "SetMaterial2D",
            "Actor",
            "Set Material",
            "Assigns a material to the actor renderer.",
            "orange",
            &["material", "renderer", "visual"][..],
        ),
        (
            "SetTextureSlot2D",
            "Actor",
            "Set Texture Slot",
            "Writes a texture into a Material2D slot.",
            "orange",
            &["texture", "normal", "roughness", "slot"][..],
        ),
        (
            "AddComponent",
            "Actor",
            "Add Component",
            "Adds a component at runtime.",
            "orange",
            &["component", "actor", "construct"][..],
        ),
        (
            "CastTo",
            "Actor",
            "Cast To",
            "Casts an object to another Blueprint type.",
            "orange",
            &["cast", "class", "type"][..],
        ),
        (
            "ParentFunctionCall",
            "Actor",
            "Call Parent",
            "Calls parent class implementation.",
            "orange",
            &["super", "parent", "override"][..],
        ),
        (
            "MakeStruct",
            "Data",
            "Make Struct",
            "Builds a struct-like value.",
            "violet",
            &["struct", "make", "data"][..],
        ),
        (
            "BreakStruct",
            "Data",
            "Break Struct",
            "Splits a struct-like value.",
            "violet",
            &["struct", "break", "data"][..],
        ),
        (
            "SetUiText",
            "UI",
            "Set UI Text",
            "Updates a UI text value.",
            "cyan",
            &["hud", "label"][..],
        ),
        (
            "SetWidgetVisibility",
            "UI",
            "Set Widget Visibility",
            "Shows or hides a widget.",
            "cyan",
            &["visible", "menu"][..],
        ),
        (
            "OpenMenu",
            "UI",
            "Open Menu",
            "Opens a named UI menu screen.",
            "cyan",
            &["main menu", "pause"][..],
        ),
        (
            "OpenWidget",
            "UI",
            "Open Widget",
            "Creates and opens a widget screen.",
            "cyan",
            &["widget", "umg", "screen"][..],
        ),
        (
            "CreateMainMenu",
            "UI",
            "Create Main Menu",
            "Creates a main menu canvas.",
            "cyan",
            &["umg", "unity ui"][..],
        ),
        (
            "CallInterfaceFunction",
            "Blueprint",
            "Interface Call",
            "Calls a function through a Blueprint interface.",
            "teal",
            &["interface", "message", "contract"][..],
        ),
        (
            "BindEventDispatcher",
            "Blueprint",
            "Bind Dispatcher",
            "Binds an event to a dispatcher.",
            "teal",
            &["dispatcher", "delegate", "bind"][..],
        ),
        (
            "CallEventDispatcher",
            "Blueprint",
            "Call Dispatcher",
            "Broadcasts a dispatcher.",
            "teal",
            &["dispatcher", "broadcast", "event"][..],
        ),
        (
            "InventoryAdd",
            "Gameplay",
            "Add Item",
            "Adds an item to inventory.",
            "purple",
            &["item", "loot"][..],
        ),
        (
            "EquipInventoryItem",
            "Survival",
            "Equip Inventory Item",
            "Atomically equips an inventory item into compatible slots.",
            "purple",
            &["equipment", "inventory", "loadout", "slot"][..],
        ),
        (
            "UnequipToInventory",
            "Survival",
            "Unequip to Inventory",
            "Returns equipped gear to inventory without item loss.",
            "purple",
            &["equipment", "inventory", "slot"][..],
        ),
        (
            "ApplyInjury",
            "Survival",
            "Apply Injury",
            "Adds a body-region injury with bleeding and infection risk.",
            "red",
            &["injury", "body", "bleeding", "survival"][..],
        ),
        (
            "TreatInjury",
            "Survival",
            "Treat Injury",
            "Consumes a data-driven treatment item on an injury.",
            "green",
            &["injury", "bandage", "medical", "survival"][..],
        ),
        (
            "SetCrouching",
            "Stealth",
            "Set Crouching",
            "Changes crouch state and its movement, noise and visibility multipliers.",
            "teal",
            &["stealth", "crouch", "movement", "survival"][..],
        ),
        (
            "EmitNoise",
            "Stealth",
            "Emit Noise",
            "Emits a world-space noise stimulus for hearing sensors.",
            "orange",
            &["noise", "hearing", "stimulus", "stealth"][..],
        ),
        (
            "DoorAction",
            "World Interaction",
            "Door Action",
            "Opens, closes, locks, unlocks or toggles a Door2D.",
            "blue",
            &["door", "open", "lock", "interaction"][..],
        ),
        (
            "AddBarricadeLayer",
            "World Interaction",
            "Add Barricade Layer",
            "Adds a durable layer to a barricadable object.",
            "brown",
            &["barricade", "build", "fortify", "survival"][..],
        ),
        (
            "DamageBarricade",
            "World Interaction",
            "Damage Barricade",
            "Applies resistance-aware damage to a barricade.",
            "red",
            &["barricade", "damage", "breach", "survival"][..],
        ),
        (
            "BranchAlertness",
            "Stealth",
            "Branch Alertness",
            "Branches on the current Senses2D alertness value.",
            "red",
            &["alert", "sense", "ai", "stealth"][..],
        ),
        (
            "ApplyGameplayEffect2D",
            "Gameplay",
            "Apply Effect",
            "Applies a gameplay effect.",
            "purple",
            &["ability", "status"][..],
        ),
        (
            "AIMoveTo",
            "AI",
            "AI Move To",
            "Orders an AI movement.",
            "red",
            &["nav", "enemy"][..],
        ),
        (
            "RunBehaviorTree",
            "AI",
            "Run Behavior Tree",
            "Starts AI logic.",
            "red",
            &["bt", "enemy"][..],
        ),
        (
            "PlaySound",
            "Audio",
            "Play Sound",
            "Plays an audio event.",
            "yellow",
            &["sfx", "music"][..],
        ),
        (
            "PlaySound2D",
            "Audio",
            "Play Sound 2D",
            "Plays a non-spatial 2D sound.",
            "yellow",
            &["sfx", "music", "ui"][..],
        ),
        (
            "SaveGame",
            "Persistence",
            "Save Game",
            "Writes save data.",
            "gray",
            &["save", "slot"][..],
        ),
        (
            "LoadGame",
            "Persistence",
            "Load Game",
            "Reads save data from a slot.",
            "gray",
            &["load", "save", "slot"][..],
        ),
        (
            "LineTrace2D",
            "Physics",
            "Line Trace 2D",
            "Traces a line and returns the first hit.",
            "red",
            &["raycast", "trace", "hit"][..],
        ),
        (
            "PrintString",
            "Debug",
            "Print String",
            "Logs a message.",
            "gray",
            &["log", "debug"][..],
        ),
    ]
    .into_iter()
    .map(
        |(kind, category, title, description, color, keywords)| BlueprintNodePaletteItem2D {
            kind: kind.to_string(),
            category: category.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            color: color.to_string(),
            keywords: keywords
                .iter()
                .map(|keyword| (*keyword).to_string())
                .collect(),
        },
    )
    .collect()
}

pub fn search_node_palette(query: &str) -> Vec<BlueprintNodePaletteItem2D> {
    let query = query.to_lowercase();
    let mut items = blueprint_node_palette()
        .into_iter()
        .filter(|item| {
            query.is_empty()
                || item.kind.to_lowercase().contains(&query)
                || item.title.to_lowercase().contains(&query)
                || item.category.to_lowercase().contains(&query)
                || item.keywords.iter().any(|keyword| keyword.contains(&query))
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|item| {
        let haystack = format!(
            "{} {} {} {}",
            item.title.to_lowercase(),
            item.kind.to_lowercase(),
            item.category.to_lowercase(),
            item.keywords.join(" ").to_lowercase()
        );
        palette_score(&query, item, &haystack)
    });
    items
}

pub fn search_node_palette_ranked(query: &str) -> Vec<BlueprintPaletteSearchResult2D> {
    let query = query.to_lowercase();
    let mut results = blueprint_node_palette()
        .into_iter()
        .filter_map(|item| {
            let haystack = format!(
                "{} {} {} {}",
                item.title.to_lowercase(),
                item.kind.to_lowercase(),
                item.category.to_lowercase(),
                item.keywords.join(" ").to_lowercase()
            );
            if query.is_empty() || haystack.contains(&query) {
                Some(BlueprintPaletteSearchResult2D {
                    score: palette_score(&query, &item, &haystack),
                    item,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    results.sort_by_key(|result| result.score);
    results
}

pub fn palette_categories() -> Vec<String> {
    blueprint_node_palette()
        .into_iter()
        .map(|item| item.category)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn minimal_blueprint_graph() -> BlueprintGraph2D {
    BlueprintGraph2D {
        name: "BP_PlayerPawn2D".to_string(),
        runtime: "miniforge_visual_script_2d".to_string(),
        asset_kind: default_blueprint_asset_kind(),
        parent_class: "Actor2D".to_string(),
        graph_type: default_event_graph_type(),
        class_settings: BlueprintClassSettings2D {
            blueprint_type: "Normal".to_string(),
            category: "Player".to_string(),
            description: "Playable 2D actor Blueprint.".to_string(),
            tick_enabled: true,
            ..Default::default()
        },
        components: vec![BlueprintComponent2D {
            name: "Root".to_string(),
            component_type: "Transform2D".to_string(),
            editable: true,
            exposed_as_variable: true,
            ..Default::default()
        }],
        interfaces: Vec::new(),
        event_dispatchers: BTreeMap::new(),
        macros: BTreeMap::new(),
        variables: BTreeMap::from([(
            "MoveSpeed".to_string(),
            BlueprintVariable2D {
                value_type: "float".to_string(),
                default_value: json!(5.0),
                editable: true,
                category: "Movement".to_string(),
                tooltip: "Units per second used by movement scripts.".to_string(),
                slider_range: Some((0.0, 50.0)),
                ..Default::default()
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
        comments: Vec::new(),
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
        ..Default::default()
    }]
}

fn exec_in_out(input: &str, output: &str) -> Vec<BlueprintPin2D> {
    vec![
        BlueprintPin2D {
            name: input.to_string(),
            pin_type: "exec".to_string(),
            direction: "in".to_string(),
            ..Default::default()
        },
        BlueprintPin2D {
            name: output.to_string(),
            pin_type: "exec".to_string(),
            direction: "out".to_string(),
            ..Default::default()
        },
    ]
}

fn pin(name: &str, pin_type: &str, direction: &str) -> BlueprintPin2D {
    BlueprintPin2D {
        name: name.to_string(),
        pin_type: pin_type.to_string(),
        direction: direction.to_string(),
        ..Default::default()
    }
}

fn default_pins_for_kind(kind: &str) -> Vec<BlueprintPin2D> {
    match kind {
        kind if kind.starts_with("Event")
            || kind == "FunctionEntry"
            || kind == "ConstructionScript" =>
        {
            exec_out("then")
        }
        "MacroEntry" => exec_out("then"),
        "MacroExit" => vec![pin("exec", "exec", "in")],
        "Reroute" => vec![
            pin("input", "wildcard", "in"),
            pin("output", "wildcard", "out"),
        ],
        "Branch" => vec![
            pin("exec", "exec", "in"),
            pin("condition", "bool", "in"),
            pin("true", "exec", "out"),
            pin("false", "exec", "out"),
        ],
        "Sequence" => vec![
            pin("exec", "exec", "in"),
            pin("then_0", "exec", "out"),
            pin("then_1", "exec", "out"),
        ],
        "ForLoop" => vec![
            pin("exec", "exec", "in"),
            pin("first_index", "int", "in"),
            pin("last_index", "int", "in"),
            pin("loop_body", "exec", "out"),
            pin("index", "int", "out"),
            pin("completed", "exec", "out"),
        ],
        "ForEachLoop" => vec![
            pin("exec", "exec", "in"),
            pin("array", "array", "in"),
            pin("loop_body", "exec", "out"),
            pin("array_element", "variant", "out"),
            pin("array_index", "int", "out"),
            pin("completed", "exec", "out"),
        ],
        "GetVariable" => vec![pin("value", "variant", "out")],
        "MakeStruct" => vec![pin("value", "struct", "out")],
        "BreakStruct" => vec![pin("value", "struct", "in")],
        "CastTo" => vec![
            pin("exec", "exec", "in"),
            pin("object", "object", "in"),
            pin("cast_failed", "exec", "out"),
            pin("cast_succeeded", "exec", "out"),
            pin("as_type", "object", "out"),
        ],
        "CallInterfaceFunction" | "BlueprintInterfaceCall" => vec![
            pin("exec", "exec", "in"),
            pin("target", "object", "in"),
            pin("then", "exec", "out"),
        ],
        "CallEventDispatcher"
        | "BindEventDispatcher"
        | "UnbindEventDispatcher"
        | "AssignEventDispatcher" => vec![
            pin("exec", "exec", "in"),
            pin("target", "object", "in"),
            pin("then", "exec", "out"),
        ],
        "Timeline" => vec![
            pin("exec", "exec", "in"),
            pin("update", "exec", "out"),
            pin("finished", "exec", "out"),
            pin("alpha", "float", "out"),
        ],
        "SetActorLocation2D" => vec![
            pin("exec", "exec", "in"),
            pin("target", "object", "in"),
            pin("x", "float", "in"),
            pin("y", "float", "in"),
            pin("then", "exec", "out"),
        ],
        "SetSprite2D" | "SetMaterial2D" => vec![
            pin("exec", "exec", "in"),
            pin("target", "object", "in"),
            pin("asset_path", "asset", "in"),
            pin("then", "exec", "out"),
        ],
        "SetTextureSlot2D" => vec![
            pin("exec", "exec", "in"),
            pin("target", "object", "in"),
            pin("slot", "string", "in"),
            pin("texture_path", "asset", "in"),
            pin("then", "exec", "out"),
        ],
        "LineTrace2D" | "Raycast2D" => vec![
            pin("exec", "exec", "in"),
            pin("start", "vector2", "in"),
            pin("end", "vector2", "in"),
            pin("then", "exec", "out"),
            pin("hit", "bool", "out"),
            pin("hit_actor", "object", "out"),
        ],
        "OpenWidget" | "PlaySound2D" | "LoadGame" => vec![
            pin("exec", "exec", "in"),
            pin("asset_or_slot", "string", "in"),
            pin("then", "exec", "out"),
        ],
        _ => exec_in_out("exec", "then"),
    }
}

fn unique_node_id(nodes: &[BlueprintNode2D], kind: &str) -> String {
    let base = kind
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    if !nodes.iter().any(|node| node.id == base) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}_{index}");
        if !nodes.iter().any(|node| node.id == candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn unique_comment_id(comments: &[BlueprintCommentBox2D]) -> String {
    let base = "comment".to_string();
    if !comments.iter().any(|comment| comment.id == base) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}_{index}");
        if !comments.iter().any(|comment| comment.id == candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn default_blueprint_asset_kind() -> String {
    "BlueprintClass".to_string()
}

fn default_parent_class() -> String {
    "Actor2D".to_string()
}

fn default_event_graph_type() -> String {
    "EventGraph".to_string()
}

fn default_public_access() -> String {
    "Public".to_string()
}

fn pin_by_name<'a>(node: &'a BlueprintNode2D, pin_name: &str) -> Option<&'a BlueprintPin2D> {
    node.pins.iter().find(|pin| pin.name == pin_name)
}

fn pins_are_compatible(from_pin: &BlueprintPin2D, to_pin: &BlueprintPin2D) -> bool {
    if from_pin.direction == "in" || to_pin.direction == "out" {
        return false;
    }
    from_pin.pin_type == to_pin.pin_type
        || from_pin.pin_type == "wildcard"
        || to_pin.pin_type == "wildcard"
        || from_pin.pin_type == "variant"
        || to_pin.pin_type == "variant"
        || (from_pin.pin_type == "int" && to_pin.pin_type == "float")
        || (from_pin.pin_type == "object" && to_pin.pin_type.ends_with("Object"))
        || (from_pin.pin_type.ends_with("Object") && to_pin.pin_type == "object")
}

fn duplicate_data_input_edges(graph: &BlueprintGraph2D) -> Vec<&BlueprintEdge2D> {
    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    for edge in &graph.edges {
        let is_data = graph
            .node_by_id(&edge.to)
            .and_then(|node| pin_by_name(node, &edge.to_pin))
            .is_some_and(|pin| pin.pin_type != "exec");
        if is_data && !seen.insert((edge.to.as_str(), edge.to_pin.as_str())) {
            duplicates.push(edge);
        }
    }
    duplicates
}

fn is_entry_node_kind(kind: &str) -> bool {
    kind.starts_with("Event") || kind == "FunctionEntry" || kind == "ConstructionScript"
}

fn palette_score(query: &str, item: &BlueprintNodePaletteItem2D, haystack: &str) -> usize {
    if query.is_empty() {
        return 100 + item.title.len();
    }
    let title = item.title.to_lowercase();
    let kind = item.kind.to_lowercase();
    let category = item.category.to_lowercase();
    if title == query || kind == query {
        0
    } else if title.starts_with(query) || kind.starts_with(query) {
        10
    } else if item
        .keywords
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case(query))
    {
        20
    } else if category == query {
        30
    } else if haystack.contains(query) {
        50 + haystack.find(query).unwrap_or(50)
    } else {
        usize::MAX
    }
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
        "SetUiText" | "SetWidgetText" => {
            require_keys(&node.data, &format!("nodes.{}", node.id), &["text"], report)
        }
        "OpenMenu" | "CloseMenu" | "PushMenu" | "SetMenuState" => {
            require_keys(&node.data, &format!("nodes.{}", node.id), &["menu"], report)
        }
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
        "SetTransform2D" | "SetActorLocation2D" => "SetPosition",
        "SetVelocity2D" => "SetVelocity",
        "AddForce2D" => "AddForce",
        "PlayAnimation2D" => "SetAnimation",
        "ApplyDamage" => "Damage",
        "SetWidgetText" => "SetUiText",
        "SetWidgetVisibility"
        | "SetWidgetPercent"
        | "OpenMenu"
        | "OpenWidget"
        | "CloseMenu"
        | "PushMenu" => "Log",
        "PopMenu" | "SetMenuState" | "CreateMainMenu" => "Log",
        "SaveCheckpoint" | "LoadCheckpoint" | "SaveGame" | "LoadGame" | "SetComponentField"
        | "BindInputAction" => "Log",
        "SetBlackboardValue" => "SetBlackboard",
        "Patrol" | "RunBehaviorTree" | "FindEntityByTag" | "GetComponent" => "Log",
        "ForLoop" | "ForEachLoop" | "ForEachEntityWithTag" | "ForEachComponent" => "Log",
        "Raycast2D" | "LineTrace2D" | "OverlapCircle2D" | "SpawnProjectile2D"
        | "PlayTimeline2D" => "Log",
        "SetMaterialParameter2D" | "StartParticleSystem2D" | "BindUiEvent" => "Log",
        "SetSprite2D" | "SetMaterial2D" | "SetTextureSlot2D" | "PlaySound2D" => "Log",
        "CallLuauFunction" | "AsyncLoadScene" | "WaitUntil" => "Log",
        "TweenPosition2D" | "TweenColor" | "GameplayTagHas" | "ApplyGameplayEffect2D" => "Log",
        "Breakpoint" | "ProfilerMarker" => "Log",
        other => other,
    }
}
