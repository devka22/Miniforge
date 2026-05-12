use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct EditorTools {
    pub show_grid: bool,
    pub show_chunks: bool,
    pub show_coordinates: bool,
    pub show_brush_preview: bool,
}

impl EditorTools {
    pub fn toggle_selected_visible(selected: &mut [GameObject]) {
        for entity in selected {
            entity.visible = !entity.visible;
        }
    }

    pub fn toggle_selected_locked(selected: &mut [GameObject]) {
        for entity in selected {
            entity.locked = !entity.locked;
        }
    }
}
