use crate::engine::camera::Camera;
use crate::engine::editor_command::EditorCommand;
use crate::engine::tilemap_layers::TilemapLayers;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct EditorHistory {
    pub undo: Vec<(String, Vec<GameObject>)>,
    pub redo: Vec<(String, Vec<GameObject>)>,
    pub command_undo: Vec<EditorCommand>,
    pub command_redo: Vec<EditorCommand>,
    pub max_entries: usize,
}

impl EditorHistory {
    pub fn take_snapshot(&mut self, label: &str, entities: &[GameObject]) {
        self.undo.push((label.to_string(), entities.to_vec()));
        self.redo.clear();
        self.trim();
    }

    pub fn undo(&mut self, current: &mut Vec<GameObject>) -> Option<String> {
        let (label, snapshot) = self.undo.pop()?;
        self.redo.push((label.clone(), current.clone()));
        *current = snapshot;
        Some(label)
    }

    pub fn redo(&mut self, current: &mut Vec<GameObject>) -> Option<String> {
        let (label, snapshot) = self.redo.pop()?;
        self.undo.push((label.clone(), current.clone()));
        *current = snapshot;
        Some(label)
    }

    pub fn push_command(&mut self, command: EditorCommand) {
        self.command_undo.push(command);
        self.command_redo.clear();
        self.trim();
    }

    pub fn undo_command(
        &mut self,
        entities: &mut Vec<GameObject>,
        tilemap_layers: &mut TilemapLayers,
        grid: &mut Grid,
        camera: &mut Camera,
        ui_canvases: &mut Value,
    ) -> Option<String> {
        let command = self.command_undo.pop()?;
        command
            .before
            .restore(entities, tilemap_layers, grid, camera, ui_canvases);
        let label = command.label.clone();
        self.command_redo.push(command);
        Some(label)
    }

    pub fn redo_command(
        &mut self,
        entities: &mut Vec<GameObject>,
        tilemap_layers: &mut TilemapLayers,
        grid: &mut Grid,
        camera: &mut Camera,
        ui_canvases: &mut Value,
    ) -> Option<String> {
        let command = self.command_redo.pop()?;
        command
            .after
            .restore(entities, tilemap_layers, grid, camera, ui_canvases);
        let label = command.label.clone();
        self.command_undo.push(command);
        Some(label)
    }

    fn trim(&mut self) {
        let limit = if self.max_entries == 0 {
            128
        } else {
            self.max_entries
        };
        if self.undo.len() > limit {
            self.undo.drain(0..self.undo.len() - limit);
        }
        if self.command_undo.len() > limit {
            self.command_undo.drain(0..self.command_undo.len() - limit);
        }
    }
}
