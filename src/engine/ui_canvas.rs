use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct UICanvas {
    pub elements: usize,
}

impl UICanvas {
    pub fn hit_test<'a>(
        &self,
        entities: &'a [GameObject],
        point: (f64, f64),
    ) -> Option<(&'a GameObject, &'a crate::engine::component::Component)> {
        let (px, py) = point;
        let mut hits = Vec::new();
        for entity in entities {
            if let Some(ui) = entity.get_component("UIElement") {
                let x = ui.get_f64("x", 0.0);
                let y = ui.get_f64("y", 0.0);
                let width = ui.get_f64("width", 0.0);
                let height = ui.get_f64("height", 0.0);
                if px >= x && py >= y && px <= x + width && py <= y + height {
                    hits.push((ui.get_i64("sorting_order", 0), entity, ui));
                }
            }
        }
        hits.sort_by_key(|(order, _, _)| *order);
        hits.pop().map(|(_, entity, ui)| (entity, ui))
    }
}
