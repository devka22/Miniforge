use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::scene_tree::SceneTreeIndex;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignalConnection {
    pub source_id: u64,
    pub signal: String,
    #[serde(default)]
    pub target_id: Option<u64>,
    #[serde(default)]
    pub target_path: Option<String>,
    pub method: String,
    #[serde(default)]
    pub binds: Vec<Value>,
    #[serde(default)]
    pub oneshot: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneSignal {
    pub source_id: u64,
    pub signal: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneSignalDispatch {
    pub source_id: u64,
    pub target_id: u64,
    pub signal: String,
    pub method: String,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SceneSignalBus {
    pub connections: Vec<SignalConnection>,
    pub emitted: Vec<SceneSignal>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignalValidationReport {
    pub missing_targets: Vec<String>,
    pub empty_methods: Vec<String>,
    pub disconnected_emitters: Vec<u64>,
}

impl SignalValidationReport {
    pub fn is_valid(&self) -> bool {
        self.missing_targets.is_empty() && self.empty_methods.is_empty()
    }
}

impl SceneSignalBus {
    pub fn from_entities(entities: &[GameObject], tree: &SceneTreeIndex) -> Self {
        let mut connections = Vec::new();
        for entity in entities {
            let Some(emitter) = entity.get_component("SignalEmitter") else {
                continue;
            };
            let Some(items) = emitter.get("connections").and_then(Value::as_array) else {
                continue;
            };
            for item in items {
                let Some(signal) = item.get("signal").and_then(Value::as_str) else {
                    continue;
                };
                let method = item
                    .get("method")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let target_id = item
                    .get("target_id")
                    .and_then(Value::as_u64)
                    .or_else(|| {
                        item.get("target_path")
                            .and_then(Value::as_str)
                            .and_then(|path| tree.resolve_path(Some(entity.id), path))
                    })
                    .filter(|id| tree.node(*id).is_some());
                connections.push(SignalConnection {
                    source_id: entity.id,
                    signal: signal.to_string(),
                    target_id,
                    target_path: item
                        .get("target_path")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    method,
                    binds: item
                        .get("binds")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                    oneshot: item
                        .get("oneshot")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    enabled: item.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                });
            }
        }
        Self {
            connections,
            emitted: Vec::new(),
        }
    }

    pub fn emit(
        &mut self,
        source_id: u64,
        signal: &str,
        payload: Value,
    ) -> Vec<SceneSignalDispatch> {
        self.emitted.push(SceneSignal {
            source_id,
            signal: signal.to_string(),
            payload: payload.clone(),
        });

        let mut dispatches = Vec::new();
        let mut disable_indices = Vec::new();
        for (index, connection) in self.connections.iter().enumerate() {
            if !connection.enabled
                || connection.source_id != source_id
                || connection.signal != signal
            {
                continue;
            }
            let Some(target_id) = connection.target_id else {
                continue;
            };
            let mut args = Vec::with_capacity(1 + connection.binds.len());
            args.push(payload.clone());
            args.extend(connection.binds.iter().cloned());
            dispatches.push(SceneSignalDispatch {
                source_id,
                target_id,
                signal: signal.to_string(),
                method: connection.method.clone(),
                args,
            });
            if connection.oneshot {
                disable_indices.push(index);
            }
        }
        for index in disable_indices {
            if let Some(connection) = self.connections.get_mut(index) {
                connection.enabled = false;
            }
        }
        dispatches
    }

    pub fn validate(&self) -> SignalValidationReport {
        let mut report = SignalValidationReport::default();
        for connection in &self.connections {
            if connection.target_id.is_none() {
                report.missing_targets.push(format!(
                    "{}:{} -> {:?}",
                    connection.source_id, connection.signal, connection.target_path
                ));
            }
            if connection.method.trim().is_empty() {
                report.empty_methods.push(format!(
                    "{}:{} has empty method",
                    connection.source_id, connection.signal
                ));
            }
        }
        report.missing_targets.sort();
        report.empty_methods.sort();
        report
    }
}

fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::engine::component::default_component;
    use crate::engine::scene_signal::SceneSignalBus;
    use crate::engine::scene_tree::SceneTreeIndex;
    use crate::entities::game_object::GameObject;

    #[test]
    fn resolves_signal_target_paths_and_emits_dispatches() {
        let source = GameObject::new(0.0, 0.0, Some("Button".to_string()));
        let target = GameObject::new(0.0, 0.0, Some("Panel".to_string()));
        let mut source = source;
        let mut emitter = default_component("SignalEmitter").unwrap();
        emitter.set(
            "connections",
            json!([
                {
                    "signal": "pressed",
                    "target_path": "/Panel",
                    "method": "show",
                    "binds": ["fast"]
                }
            ]),
        );
        source.add_component(emitter);
        let target_id = target.id;
        let entities = vec![source.clone(), target];
        let tree = SceneTreeIndex::build(&entities);
        let mut bus = SceneSignalBus::from_entities(&entities, &tree);

        let dispatches = bus.emit(source.id, "pressed", json!({"button": "ok"}));

        assert_eq!(dispatches.len(), 1);
        assert_eq!(dispatches[0].target_id, target_id);
        assert_eq!(dispatches[0].method, "show");
        assert_eq!(dispatches[0].args.len(), 2);
    }
}
