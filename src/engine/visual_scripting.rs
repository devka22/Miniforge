use serde_json::Value;

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct VisualScriptRuntime {
    pub graphs: usize,
    pub last_frame_graphs: usize,
    pub executed_nodes: usize,
    pub logs: Vec<String>,
}

impl VisualScriptRuntime {
    pub fn update_entities(&mut self, entities: &mut [GameObject], dt: f64, mode: &str) {
        self.last_frame_graphs = 0;
        self.executed_nodes = 0;
        for entity in entities {
            let Some(script) = entity.get_component("VisualScript").cloned() else {
                continue;
            };
            if mode != "PLAY" && !script.get_bool("run_in_editor", false) {
                continue;
            }
            self.graphs += 1;
            self.last_frame_graphs += 1;
            let nodes = script
                .get("nodes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let start_id = if script.get_bool("_started", false) {
                "update"
            } else {
                "start"
            };
            self.execute_chain(entity, &nodes, start_id, dt);
            if let Some(script_mut) = entity.get_component_mut("VisualScript") {
                script_mut.set("_started", serde_json::json!(true));
            }
        }
    }

    fn execute_chain(&mut self, entity: &mut GameObject, nodes: &[Value], start_id: &str, dt: f64) {
        let mut current = nodes
            .iter()
            .find(|node| node.get("id").and_then(Value::as_str) == Some(start_id))
            .or_else(|| {
                nodes
                    .iter()
                    .find(|node| node.get("type").and_then(Value::as_str) == Some("EventStart"))
            });
        let mut guard = 0;
        while let Some(node) = current {
            guard += 1;
            if guard > 128 {
                break;
            }
            self.executed_nodes += 1;
            match node.get("type").and_then(Value::as_str).unwrap_or("") {
                "Move" => {
                    entity.x += node.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                    entity.y += node.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                    entity.sync_to_components();
                }
                "Log" => {
                    self.logs.push(
                        node.get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("Visual script running")
                            .to_string(),
                    );
                }
                "Damage" => {
                    if let Some(health) = entity.get_component_mut("Health") {
                        health
                            .take_damage(node.get("amount").and_then(Value::as_f64).unwrap_or(0.0));
                    }
                }
                "Heal" => {
                    if let Some(health) = entity.get_component_mut("Health") {
                        health.heal(node.get("amount").and_then(Value::as_f64).unwrap_or(0.0));
                    }
                }
                "SetEnabled" => {
                    entity.enabled = node
                        .get("value")
                        .and_then(Value::as_bool)
                        .unwrap_or(entity.enabled);
                    entity.active = entity.enabled;
                }
                "SetVariable" => {
                    if let Some(script) = entity.get_component_mut("VisualScript") {
                        let mut vars = script
                            .get("variables")
                            .and_then(Value::as_object)
                            .cloned()
                            .unwrap_or_default();
                        if let Some(name) = node.get("name").and_then(Value::as_str) {
                            vars.insert(
                                name.to_string(),
                                node.get("value").cloned().unwrap_or(Value::Null),
                            );
                            script.set("variables", Value::Object(vars));
                        }
                    }
                }
                "Wait" => {
                    let _ = dt;
                }
                _ => {}
            }
            let next = node.get("next").and_then(Value::as_str);
            current = next.and_then(|id| {
                nodes
                    .iter()
                    .find(|node| node.get("id").and_then(Value::as_str) == Some(id))
            });
        }
    }
}
