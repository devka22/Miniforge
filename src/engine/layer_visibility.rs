use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct LayerVisibility {
    pub layers: BTreeMap<String, bool>,
}

impl LayerVisibility {
    pub fn new(layer_names: &[String]) -> Self {
        let mut visibility = Self::default();
        for layer in layer_names {
            visibility.layers.insert(layer.clone(), true);
        }
        visibility
    }

    pub fn is_visible(&self, layer: &str) -> bool {
        self.layers.get(layer).copied().unwrap_or(true)
    }

    pub fn set_visible(&mut self, layer: &str, visible: bool) {
        self.layers.insert(layer.to_string(), visible);
    }

    pub fn toggle(&mut self, layer: &str) -> bool {
        let next = !self.is_visible(layer);
        self.set_visible(layer, next);
        next
    }
}
