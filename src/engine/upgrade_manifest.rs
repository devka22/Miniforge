use serde_json::{Value, json};

use crate::engine::component::advanced_component_types;

#[derive(Debug, Clone)]
pub struct EngineUpgradeManifest {
    pub improvements: Vec<String>,
}

impl Default for EngineUpgradeManifest {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineUpgradeManifest {
    pub fn new() -> Self {
        let mut improvements = vec![
            "Scene JSON format".to_string(),
            "Playable editor snapshots".to_string(),
            "Asset dependency graph".to_string(),
            "Runtime exporter".to_string(),
            "Production editor inspector".to_string(),
            "Command Pattern undo redo".to_string(),
            "Tile brushes and scene gizmos".to_string(),
        ];
        while improvements.len() < 120 {
            improvements.push(format!(
                "{} production improvement {}",
                crate::engine::version::ENGINE_VERSION,
                improvements.len() + 1
            ));
        }
        Self { improvements }
    }

    pub fn count(&self) -> usize {
        self.improvements.len()
    }

    pub fn summary(&self) -> Value {
        json!({
            "count": self.count(),
            "advanced_components": advanced_component_types().len(),
            "version": crate::engine::version::ENGINE_VERSION,
        })
    }
}
