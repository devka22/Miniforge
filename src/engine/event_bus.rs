use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct EventBus {
    pub events: Vec<(String, Value)>,
    pub counters: BTreeMap<String, usize>,
}

impl EventBus {
    pub fn emit(&mut self, event: &str, payload: Value) {
        *self.counters.entry(event.to_string()).or_insert(0) += 1;
        self.events.push((event.to_string(), payload));
    }

    pub fn drain(&mut self) -> Vec<(String, Value)> {
        std::mem::take(&mut self.events)
    }

    pub fn drain_named(&mut self, event: &str) -> Vec<Value> {
        let mut matched = Vec::new();
        let mut retained = Vec::new();
        for (name, payload) in self.events.drain(..) {
            if name == event {
                matched.push(payload);
            } else {
                retained.push((name, payload));
            }
        }
        self.events = retained;
        matched
    }

    pub fn count(&self, event: &str) -> usize {
        self.counters.get(event).copied().unwrap_or(0)
    }
}
