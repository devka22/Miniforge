use std::collections::BTreeMap;

use crate::engine::ui_runtime::{UiRuntime, UiRuntimeEvent};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct UISystem {
    pub runtime: UiRuntime,
    pub stats: BTreeMap<String, usize>,
}

impl UISystem {
    pub fn update_entities(
        &mut self,
        entities: &mut [GameObject],
        pointer: Option<(f64, f64)>,
        clicked: bool,
        mode: &str,
    ) -> Vec<UiRuntimeEvent> {
        let events = if mode == "PLAY" {
            pointer
                .map(|pointer| {
                    self.runtime
                        .update_entity_interaction(entities, pointer, clicked)
                })
                .unwrap_or_else(|| {
                    self.runtime.events.clear();
                    Vec::new()
                })
        } else {
            self.runtime.events.clear();
            Vec::new()
        };
        self.stats.insert(
            "ui_elements".to_string(),
            entities
                .iter()
                .filter(|entity| entity.get_component("UIElement").is_some())
                .count(),
        );
        self.stats.insert("events".to_string(), events.len());
        events
    }

    pub fn drain_events(&mut self) -> Vec<UiRuntimeEvent> {
        std::mem::take(&mut self.runtime.events)
    }
}
