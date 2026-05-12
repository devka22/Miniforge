use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::camera::Camera;
use crate::engine::tilemap_layers::TilemapLayers;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EditorCommandKind {
    MoveEntity {
        entity_id: u64,
    },
    EditInspector {
        entity_id: u64,
        target: String,
        field: String,
        before: Value,
        after: Value,
    },
    CreateEntity {
        entity_id: u64,
    },
    DeleteEntity {
        entity_id: u64,
    },
    DuplicateEntity {
        source_id: u64,
        clone_id: u64,
    },
    PaintTilemap {
        layer: usize,
        cells: Vec<(usize, usize, i32, i32)>,
    },
    AddComponent {
        entity_id: u64,
        component_type: String,
    },
    RemoveComponent {
        entity_id: u64,
        component_type: String,
    },
    SceneOperation {
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct EditorSnapshot {
    pub entities: Vec<GameObject>,
    pub tilemap_layers: TilemapLayers,
    pub grid: Grid,
    pub camera: Camera,
}

#[derive(Debug, Clone)]
pub struct EditorCommand {
    pub label: String,
    pub kind: EditorCommandKind,
    pub before: EditorSnapshot,
    pub after: EditorSnapshot,
}

impl EditorSnapshot {
    pub fn capture(
        entities: &[GameObject],
        tilemap_layers: &TilemapLayers,
        grid: &Grid,
        camera: &Camera,
    ) -> Self {
        Self {
            entities: entities.to_vec(),
            tilemap_layers: tilemap_layers.clone(),
            grid: grid.clone(),
            camera: *camera,
        }
    }

    pub fn restore(
        &self,
        entities: &mut Vec<GameObject>,
        tilemap_layers: &mut TilemapLayers,
        grid: &mut Grid,
        camera: &mut Camera,
    ) {
        *entities = self.entities.clone();
        *tilemap_layers = self.tilemap_layers.clone();
        *grid = self.grid.clone();
        *camera = self.camera;
    }
}

impl EditorCommand {
    pub fn new(
        label: impl Into<String>,
        kind: EditorCommandKind,
        before: EditorSnapshot,
        after: EditorSnapshot,
    ) -> Self {
        Self {
            label: label.into(),
            kind,
            before,
            after,
        }
    }
}
