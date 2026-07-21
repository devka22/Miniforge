use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::miniforge_2d::sequencer2d::{
    Sequencer2D, SequencerKeyframe2D, SequencerTrack2D, minimal_sequencer, supported_track_types,
};
use crate::engine::miniforge_2d::tilemap_editor2d::{
    RuleTile2D, TerrainRule2D, TerrainSet2D, TileBrushKind2D, TilemapEditor2D,
};
use crate::engine::miniforge_2d::ui_designer::{UiDesigner2D, UiDesignerTool2D};
use crate::engine::miniforge_2d::ui_framework::{
    UiBinding2D, UiCallback2D, UiCanvas2D, UiWidget2D,
};
use crate::engine::tilemap_layers::TileLayer;

const HISTORY_LIMIT: usize = 64;

#[derive(Debug, Clone)]
struct DocumentHistory<T> {
    document_path: String,
    document: T,
    dirty: bool,
    undo: Vec<T>,
    redo: Vec<T>,
}

impl<T: Clone> DocumentHistory<T> {
    fn new(document_path: impl Into<String>, document: T) -> Self {
        Self {
            document_path: document_path.into(),
            document,
            dirty: false,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.document.clone());
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.document.clone());
        self.document = previous;
        self.dirty = true;
        true
    }

    fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.document.clone());
        self.document = next;
        self.dirty = true;
        true
    }

    fn reset_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SequencerWorkspace {
    sequence: Sequencer2D,
    cursor: f64,
    playing: bool,
    looped: bool,
    selected_track: Option<String>,
    selected_key: Option<usize>,
}

impl Default for SequencerWorkspace {
    fn default() -> Self {
        let sequence = minimal_sequencer();
        Self {
            selected_track: sequence.tracks.first().map(|track| track.id.clone()),
            sequence,
            cursor: 0.0,
            playing: false,
            looped: true,
            selected_key: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditorToolSessions {
    project_path: Option<PathBuf>,
    sequencer: DocumentHistory<SequencerWorkspace>,
    tilemap: DocumentHistory<TilemapEditor2D>,
    ui_designer: DocumentHistory<UiDesigner2D>,
}

impl Default for EditorToolSessions {
    fn default() -> Self {
        Self {
            project_path: None,
            sequencer: DocumentHistory::new(
                "assets/animations/Timeline.seq2d",
                SequencerWorkspace::default(),
            ),
            tilemap: DocumentHistory::new(
                "assets/tilemaps/World.tilemap2d.json",
                TilemapEditor2D::new(24, 16),
            ),
            ui_designer: DocumentHistory::new("assets/ui/hud.mfui", UiDesigner2D::default()),
        }
    }
}

impl EditorToolSessions {
    pub fn open_project(&mut self, project_path: impl AsRef<Path>) -> Result<(), String> {
        let project_path = project_path.as_ref().to_path_buf();
        self.project_path = Some(project_path.clone());
        self.sequencer = DocumentHistory::new(
            "assets/animations/Timeline.seq2d",
            load_json::<Sequencer2D>(&project_path.join("assets/animations/Timeline.seq2d"))
                .map(|sequence| SequencerWorkspace {
                    selected_track: sequence.tracks.first().map(|track| track.id.clone()),
                    sequence,
                    cursor: 0.0,
                    playing: false,
                    looped: true,
                    selected_key: None,
                })
                .unwrap_or_default(),
        );
        self.tilemap = DocumentHistory::new(
            "assets/tilemaps/World.tilemap2d.json",
            load_json::<TilemapEditor2D>(
                &project_path.join("assets/tilemaps/World.tilemap2d.json"),
            )
            .unwrap_or_else(|| TilemapEditor2D::new(24, 16)),
        );
        let ui_path = "assets/ui/hud.mfui";
        let designer = load_json::<UiCanvas2D>(&project_path.join(ui_path))
            .map(|canvas| UiDesigner2D {
                document_path: ui_path.to_string(),
                animation_timeline: canvas.animations.clone(),
                canvas,
                ..UiDesigner2D::default()
            })
            .unwrap_or_default();
        self.ui_designer = DocumentHistory::new(ui_path, designer);
        Ok(())
    }

    pub fn state(&self, tool: &str) -> Result<Value, String> {
        self.ensure_project()?;
        match tool {
            "sequencer" => Ok(self.sequencer_state()),
            "tilemap" => Ok(self.tilemap_state()),
            "ui_designer" => Ok(self.ui_designer_state()),
            _ => Err(format!("Unknown editor tool: {tool}")),
        }
    }

    pub fn action(&mut self, tool: &str, action: &str, payload: &Value) -> Result<Value, String> {
        self.ensure_project()?;
        if !payload.is_object() {
            return Err("Editor tool action payload must be a JSON object".to_string());
        }
        match tool {
            "sequencer" => self.sequencer_action(action, payload)?,
            "tilemap" => self.tilemap_action(action, payload)?,
            "ui_designer" => self.ui_designer_action(action, payload)?,
            _ => return Err(format!("Unknown editor tool: {tool}")),
        }
        self.state(tool)
    }

    fn ensure_project(&self) -> Result<&Path, String> {
        self.project_path
            .as_deref()
            .ok_or_else(|| "No project is open".to_string())
    }

    fn common_action(&mut self, tool: &str, action: &str) -> Result<bool, String> {
        match (tool, action) {
            ("sequencer", "undo") => {
                self.sequencer.undo();
                Ok(true)
            }
            ("sequencer", "redo") => {
                self.sequencer.redo();
                Ok(true)
            }
            ("tilemap", "undo") => {
                self.tilemap.undo();
                Ok(true)
            }
            ("tilemap", "redo") => {
                self.tilemap.redo();
                Ok(true)
            }
            ("ui_designer", "undo") => {
                self.ui_designer.undo();
                Ok(true)
            }
            ("ui_designer", "redo") => {
                self.ui_designer.redo();
                Ok(true)
            }
            (_, "save") => {
                self.save(tool)?;
                Ok(true)
            }
            (_, "reload") => {
                self.reload(tool)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn save(&mut self, tool: &str) -> Result<(), String> {
        let project_path = self.ensure_project()?.to_path_buf();
        match tool {
            "sequencer" => {
                atomic_write_json(
                    &project_path.join(&self.sequencer.document_path),
                    &self.sequencer.document.sequence,
                )?;
                self.sequencer.dirty = false;
            }
            "tilemap" => {
                atomic_write_json(
                    &project_path.join(&self.tilemap.document_path),
                    &self.tilemap.document,
                )?;
                self.tilemap.dirty = false;
            }
            "ui_designer" => {
                atomic_write_json(
                    &project_path.join(&self.ui_designer.document_path),
                    &self.ui_designer.document.canvas,
                )?;
                self.ui_designer.dirty = false;
            }
            _ => return Err(format!("Unknown editor tool: {tool}")),
        }
        Ok(())
    }

    fn reload(&mut self, tool: &str) -> Result<(), String> {
        let project_path = self.ensure_project()?.to_path_buf();
        match tool {
            "sequencer" => {
                let sequence =
                    load_json::<Sequencer2D>(&project_path.join(&self.sequencer.document_path))
                        .ok_or_else(|| "Sequencer document does not exist".to_string())?;
                self.sequencer.document = SequencerWorkspace {
                    selected_track: sequence.tracks.first().map(|track| track.id.clone()),
                    sequence,
                    cursor: 0.0,
                    playing: false,
                    looped: true,
                    selected_key: None,
                };
                self.sequencer.reset_history();
            }
            "tilemap" => {
                self.tilemap.document =
                    load_json::<TilemapEditor2D>(&project_path.join(&self.tilemap.document_path))
                        .ok_or_else(|| "Tilemap document does not exist".to_string())?;
                self.tilemap.reset_history();
            }
            "ui_designer" => {
                let canvas =
                    load_json::<UiCanvas2D>(&project_path.join(&self.ui_designer.document_path))
                        .ok_or_else(|| "UI document does not exist".to_string())?;
                self.ui_designer.document.canvas = canvas;
                self.ui_designer.document.selected_widget = None;
                self.ui_designer.reset_history();
            }
            _ => return Err(format!("Unknown editor tool: {tool}")),
        }
        Ok(())
    }

    fn sequencer_action(&mut self, action: &str, payload: &Value) -> Result<(), String> {
        if self.common_action("sequencer", action)? {
            return Ok(());
        }
        match action {
            "new" => {
                self.sequencer.checkpoint();
                let name = string_field(payload, "name").unwrap_or("Timeline");
                let duration = number_field(payload, "duration").unwrap_or(6.0).max(0.01);
                let frame_rate = number_field(payload, "frame_rate").unwrap_or(30.0).max(1.0);
                self.sequencer.document = SequencerWorkspace {
                    sequence: Sequencer2D {
                        name: name.to_string(),
                        duration,
                        frame_rate,
                        tracks: Vec::new(),
                    },
                    cursor: 0.0,
                    playing: false,
                    looped: true,
                    selected_track: None,
                    selected_key: None,
                };
            }
            "select_track" => {
                self.sequencer.document.selected_track =
                    string_field(payload, "track_id").map(ToString::to_string);
                self.sequencer.document.selected_key = None;
            }
            "select_key" => {
                self.sequencer.document.selected_track =
                    Some(required_string(payload, "track_id")?.to_string());
                self.sequencer.document.selected_key = Some(required_usize(payload, "index")?);
            }
            "add_track" => {
                let id = required_string(payload, "id")?.trim();
                let track_type = required_string(payload, "track_type")?.trim();
                if id.is_empty()
                    || self
                        .sequencer
                        .document
                        .sequence
                        .tracks
                        .iter()
                        .any(|track| track.id == id)
                {
                    return Err("Sequencer track id must be non-empty and unique".to_string());
                }
                if !supported_track_types().contains(&track_type) {
                    return Err(format!("Unsupported sequencer track type: {track_type}"));
                }
                self.sequencer.checkpoint();
                self.sequencer
                    .document
                    .sequence
                    .tracks
                    .push(SequencerTrack2D {
                        id: id.to_string(),
                        target: string_field(payload, "target")
                            .filter(|value| !value.trim().is_empty())
                            .map(ToString::to_string),
                        track_type: track_type.to_string(),
                        keyframes: Vec::new(),
                    });
                self.sequencer.document.selected_track = Some(id.to_string());
            }
            "remove_track" => {
                let id = required_string(payload, "track_id")?;
                self.sequencer.checkpoint();
                let before = self.sequencer.document.sequence.tracks.len();
                self.sequencer
                    .document
                    .sequence
                    .tracks
                    .retain(|track| track.id != id);
                if before == self.sequencer.document.sequence.tracks.len() {
                    return Err(format!("Sequencer track not found: {id}"));
                }
                self.sequencer.document.selected_track = None;
                self.sequencer.document.selected_key = None;
            }
            "add_keyframe" => {
                let track_id = required_string(payload, "track_id")?;
                let duration = self.sequencer.document.sequence.duration;
                let time = required_number(payload, "time")?.clamp(0.0, duration);
                let easing = string_field(payload, "easing")
                    .unwrap_or("linear")
                    .to_string();
                let value = payload.get("value").cloned().unwrap_or_else(|| json!({}));
                self.sequencer.checkpoint();
                let track = sequence_track_mut(&mut self.sequencer.document.sequence, track_id)?;
                track.keyframes.push(SequencerKeyframe2D {
                    time,
                    easing,
                    value,
                });
                track.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
            }
            "update_keyframe" | "move_keyframe" => {
                let track_id = required_string(payload, "track_id")?;
                let index = required_usize(payload, "index")?;
                let duration = self.sequencer.document.sequence.duration;
                self.sequencer.checkpoint();
                let track = sequence_track_mut(&mut self.sequencer.document.sequence, track_id)?;
                let key = track
                    .keyframes
                    .get_mut(index)
                    .ok_or_else(|| "Keyframe index out of range".to_string())?;
                if let Some(time) = number_field(payload, "time") {
                    key.time = time.clamp(0.0, duration);
                }
                if let Some(easing) = string_field(payload, "easing") {
                    key.easing = easing.to_string();
                }
                if let Some(value) = payload.get("value") {
                    key.value = value.clone();
                }
                track.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
            }
            "set_tangents" => {
                let track_id = required_string(payload, "track_id")?;
                let index = required_usize(payload, "index")?;
                let in_tangent = number_field(payload, "in_tangent").unwrap_or(0.0);
                let out_tangent = number_field(payload, "out_tangent").unwrap_or(0.0);
                if !in_tangent.is_finite() || !out_tangent.is_finite() {
                    return Err("Curve tangents must be finite".to_string());
                }
                self.sequencer.checkpoint();
                let track = sequence_track_mut(&mut self.sequencer.document.sequence, track_id)?;
                let key = track
                    .keyframes
                    .get_mut(index)
                    .ok_or_else(|| "Keyframe index out of range".to_string())?;
                set_key_curve_metadata(&mut key.value, in_tangent, out_tangent);
            }
            "remove_keyframe" => {
                let track_id = required_string(payload, "track_id")?;
                let index = required_usize(payload, "index")?;
                self.sequencer.checkpoint();
                let track = sequence_track_mut(&mut self.sequencer.document.sequence, track_id)?;
                if index >= track.keyframes.len() {
                    return Err("Keyframe index out of range".to_string());
                }
                track.keyframes.remove(index);
                self.sequencer.document.selected_key = None;
            }
            "set_duration" => {
                let duration = required_number(payload, "duration")?.max(0.01);
                self.sequencer.checkpoint();
                self.sequencer.document.sequence.duration = duration;
                for key in self
                    .sequencer
                    .document
                    .sequence
                    .tracks
                    .iter_mut()
                    .flat_map(|track| &mut track.keyframes)
                {
                    key.time = key.time.clamp(0.0, duration);
                }
                self.sequencer.document.cursor = self.sequencer.document.cursor.min(duration);
            }
            "set_frame_rate" => {
                let frame_rate = required_number(payload, "frame_rate")?;
                if frame_rate <= 0.0 {
                    return Err("Sequencer frame rate must be greater than zero".to_string());
                }
                self.sequencer.checkpoint();
                self.sequencer.document.sequence.frame_rate = frame_rate;
            }
            "set_cursor" => {
                self.sequencer.document.cursor = required_number(payload, "cursor")?
                    .clamp(0.0, self.sequencer.document.sequence.duration);
            }
            "set_playing" => {
                self.sequencer.document.playing = required_bool(payload, "value")?;
            }
            "set_looped" => {
                self.sequencer.document.looped = required_bool(payload, "value")?;
            }
            "tick" => {
                let delta = required_number(payload, "delta")?.max(0.0);
                if self.sequencer.document.playing {
                    self.sequencer.document.cursor += delta;
                    let duration = self.sequencer.document.sequence.duration.max(0.01);
                    if self.sequencer.document.cursor > duration {
                        self.sequencer.document.cursor = if self.sequencer.document.looped {
                            self.sequencer.document.cursor % duration
                        } else {
                            self.sequencer.document.playing = false;
                            duration
                        };
                    }
                }
            }
            "validate" => {
                if !self.sequencer.document.sequence.validate() {
                    return Err(
                        "Sequencer contains invalid duration, frame rate, or keyframe times"
                            .to_string(),
                    );
                }
            }
            _ => return Err(format!("Unknown sequencer action: {action}")),
        }
        Ok(())
    }

    fn tilemap_action(&mut self, action: &str, payload: &Value) -> Result<(), String> {
        if self.common_action("tilemap", action)? {
            return Ok(());
        }
        match action {
            "new" => {
                let width = required_usize(payload, "width")?.clamp(1, 256);
                let height = required_usize(payload, "height")?.clamp(1, 256);
                self.tilemap.checkpoint();
                self.tilemap.document = TilemapEditor2D::new(width, height);
            }
            "set_layer" => {
                let layer = required_usize(payload, "layer")?;
                if layer >= self.tilemap.document.tilemap.layers.len() {
                    return Err("Tilemap layer index out of range".to_string());
                }
                self.tilemap.document.active_layer = layer;
                self.tilemap.document.tilemap.active_layer = layer;
            }
            "set_tile" => self.tilemap.document.palette.selected = required_i32(payload, "tile")?,
            "set_brush" => {
                self.tilemap.document.active_brush =
                    parse_brush(required_string(payload, "brush")?)?;
            }
            "paint_cells" => {
                let layer =
                    usize_field(payload, "layer").unwrap_or(self.tilemap.document.active_layer);
                let value =
                    i32_field(payload, "value").unwrap_or(self.tilemap.document.palette.selected);
                let cells = payload
                    .get("cells")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "paint_cells requires cells".to_string())?
                    .clone();
                self.tilemap.checkpoint();
                for cell in cells {
                    if let (Some(x), Some(y)) = (usize_field(&cell, "x"), usize_field(&cell, "y")) {
                        self.tilemap.document.paint_cell(layer, x, y, value);
                    }
                }
            }
            "line" => {
                let (layer, start, end, value) =
                    tile_region_payload(&self.tilemap.document, payload)?;
                self.tilemap.checkpoint();
                self.tilemap.document.apply_line(layer, start, end, value);
            }
            "rectangle" | "fill" => {
                let (layer, start, end, value) =
                    tile_region_payload(&self.tilemap.document, payload)?;
                let filled = action == "fill"
                    || payload
                        .get("filled")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                self.tilemap.checkpoint();
                self.tilemap
                    .document
                    .apply_rectangle(layer, start, end, value, filled);
            }
            "flood_fill" => {
                let layer =
                    usize_field(payload, "layer").unwrap_or(self.tilemap.document.active_layer);
                let origin = required_point(payload, "origin")?;
                let value =
                    i32_field(payload, "value").unwrap_or(self.tilemap.document.palette.selected);
                let map = &self.tilemap.document.tilemap;
                let target_layer = map
                    .layers
                    .get(layer)
                    .ok_or_else(|| "Tilemap layer index out of range".to_string())?;
                if target_layer.locked || origin.0 >= map.width || origin.1 >= map.height {
                    return Err("Flood fill origin is outside an editable layer".to_string());
                }
                let previous = target_layer.get(origin.0, origin.1);
                if previous == value {
                    return Ok(());
                }
                self.tilemap.checkpoint();
                flood_fill_layer(
                    &mut self.tilemap.document.tilemap.layers[layer],
                    self.tilemap.document.tilemap.width,
                    self.tilemap.document.tilemap.height,
                    origin,
                    previous,
                    value,
                );
            }
            "rules" | "terrain" => {
                let layer =
                    usize_field(payload, "layer").unwrap_or(self.tilemap.document.active_layer);
                self.tilemap.checkpoint();
                self.tilemap.document.apply_rule_tiles(layer);
            }
            "add_terrain_rule" => {
                let set_name = string_field(payload, "terrain_set").unwrap_or("Terrain");
                let name = required_string(payload, "name")?.trim();
                if name.is_empty() {
                    return Err("Terrain rule name cannot be empty".to_string());
                }
                self.tilemap.checkpoint();
                let sets = &mut self.tilemap.document.terrain_sets;
                let set_index = sets
                    .iter()
                    .position(|set| set.name == set_name)
                    .unwrap_or_else(|| {
                        sets.push(TerrainSet2D {
                            name: set_name.to_string(),
                            rules: Vec::new(),
                        });
                        sets.len() - 1
                    });
                let rule = TerrainRule2D {
                    name: name.to_string(),
                    center_tile: i32_field(payload, "center_tile")
                        .unwrap_or(self.tilemap.document.palette.selected),
                    neighbors: Default::default(),
                    output_tile: i32_field(payload, "output_tile")
                        .unwrap_or(self.tilemap.document.palette.selected),
                    priority: usize_field(payload, "priority").unwrap_or(0),
                };
                if let Some(existing) = sets[set_index]
                    .rules
                    .iter_mut()
                    .find(|existing| existing.name == name)
                {
                    *existing = rule;
                } else {
                    sets[set_index].rules.push(rule);
                }
            }
            "remove_terrain_rule" => {
                let set_name = required_string(payload, "terrain_set")?;
                let name = required_string(payload, "name")?;
                let Some(set_index) = self
                    .tilemap
                    .document
                    .terrain_sets
                    .iter()
                    .position(|set| set.name == set_name)
                else {
                    return Err("Terrain set not found".to_string());
                };
                self.tilemap.checkpoint();
                self.tilemap.document.terrain_sets[set_index]
                    .rules
                    .retain(|rule| rule.name != name);
            }
            "add_rule_tile" => {
                let name = required_string(payload, "name")?.trim();
                if name.is_empty() {
                    return Err("Rule tile name cannot be empty".to_string());
                }
                self.tilemap.checkpoint();
                let rule = RuleTile2D {
                    name: name.to_string(),
                    output_tile: i32_field(payload, "output_tile")
                        .unwrap_or(self.tilemap.document.palette.selected),
                    probability_percent: usize_field(payload, "probability_percent")
                        .unwrap_or(100)
                        .clamp(0, 100) as u8,
                    required_neighbors: Default::default(),
                };
                if let Some(existing) = self
                    .tilemap
                    .document
                    .rule_tiles
                    .iter_mut()
                    .find(|existing| existing.name == name)
                {
                    *existing = rule;
                } else {
                    self.tilemap.document.rule_tiles.push(rule);
                }
            }
            "remove_rule_tile" => {
                let name = required_string(payload, "name")?;
                self.tilemap.checkpoint();
                self.tilemap
                    .document
                    .rule_tiles
                    .retain(|rule| rule.name != name);
            }
            "select" => {
                let layer =
                    usize_field(payload, "layer").unwrap_or(self.tilemap.document.active_layer);
                let start = required_point(payload, "start")?;
                let end = required_point(payload, "end")?;
                self.tilemap.document.select_rectangle(layer, start, end);
            }
            "clear_selection" => self.tilemap.document.clear_selection(),
            "copy" => {
                let name = string_field(payload, "name").unwrap_or("Tile Selection");
                self.tilemap
                    .document
                    .copy_selection(name)
                    .ok_or_else(|| "No tile selection to copy".to_string())?;
            }
            "paste" => {
                let layer =
                    usize_field(payload, "layer").unwrap_or(self.tilemap.document.active_layer);
                let origin = required_point(payload, "origin")?;
                self.tilemap.checkpoint();
                self.tilemap.document.paste_clipboard(layer, origin);
            }
            "add_layer" => {
                let name = required_string(payload, "name")?.trim();
                if name.is_empty() {
                    return Err("Layer name cannot be empty".to_string());
                }
                self.tilemap.checkpoint();
                let map = &mut self.tilemap.document.tilemap;
                map.layers.push(TileLayer::new(name, map.width, map.height));
            }
            "remove_layer" => {
                let layer = required_usize(payload, "layer")?;
                if self.tilemap.document.tilemap.layers.len() <= 1
                    || layer >= self.tilemap.document.tilemap.layers.len()
                {
                    return Err("Tilemap must keep at least one valid layer".to_string());
                }
                self.tilemap.checkpoint();
                self.tilemap.document.tilemap.layers.remove(layer);
                self.tilemap.document.active_layer = self
                    .tilemap
                    .document
                    .active_layer
                    .min(self.tilemap.document.tilemap.layers.len() - 1);
                self.tilemap.document.tilemap.active_layer = self.tilemap.document.active_layer;
            }
            "rename_layer" => {
                let layer = required_usize(payload, "layer")?;
                let name = required_string(payload, "name")?;
                self.tilemap.checkpoint();
                let target = self
                    .tilemap
                    .document
                    .tilemap
                    .layers
                    .get_mut(layer)
                    .ok_or_else(|| "Tilemap layer index out of range".to_string())?;
                target.name = name.to_string();
            }
            "set_layer_visible" | "set_layer_locked" => {
                let layer = required_usize(payload, "layer")?;
                let value = required_bool(payload, "value")?;
                self.tilemap.checkpoint();
                let target = self
                    .tilemap
                    .document
                    .tilemap
                    .layers
                    .get_mut(layer)
                    .ok_or_else(|| "Tilemap layer index out of range".to_string())?;
                if action == "set_layer_visible" {
                    target.visible = value;
                } else {
                    target.locked = value;
                }
            }
            "validate" => {
                let issues = self.tilemap.document.validate();
                if !issues.is_empty() {
                    return Err(issues.join(" | "));
                }
            }
            _ => return Err(format!("Unknown tilemap action: {action}")),
        }
        Ok(())
    }

    fn ui_designer_action(&mut self, action: &str, payload: &Value) -> Result<(), String> {
        if self.common_action("ui_designer", action)? {
            return Ok(());
        }
        match action {
            "new" => {
                let template = string_field(payload, "template").unwrap_or("hud");
                self.ui_designer.checkpoint();
                self.ui_designer.document = match template {
                    "main_menu" => UiDesigner2D::main_menu(
                        string_field(payload, "title").unwrap_or("MiniForge Game"),
                    ),
                    "pause" => UiDesigner2D::pause_menu(),
                    "settings" => UiDesigner2D::settings_menu(),
                    _ => UiDesigner2D::default(),
                };
                self.ui_designer.document_path = self.ui_designer.document.document_path.clone();
            }
            "select" => {
                let id = required_string(payload, "widget_id")?;
                if !self.ui_designer.document.select(id) {
                    return Err(format!("UI widget not found: {id}"));
                }
            }
            "select_point" => {
                let x = required_number(payload, "x")? as f32;
                let y = required_number(payload, "y")? as f32;
                self.ui_designer
                    .document
                    .select_at_preview_point(x, y)
                    .ok_or_else(|| "No UI widget at preview point".to_string())?;
            }
            "add_widget" => {
                let widget_type = required_string(payload, "widget_type")?;
                let id = required_string(payload, "id")?;
                let x = number_field(payload, "x").unwrap_or(32.0) as f32;
                let y = number_field(payload, "y").unwrap_or(32.0) as f32;
                self.ui_designer.checkpoint();
                if !self
                    .ui_designer
                    .document
                    .create_widget_from_palette(widget_type, id, x, y)
                {
                    return Err(format!("Could not create UI widget type {widget_type}"));
                }
                self.ui_designer.document.selected_widget = Some(id.to_string());
            }
            "duplicate" => {
                let id = required_string(payload, "id")?;
                self.ui_designer.checkpoint();
                if !self.ui_designer.document.duplicate_selected(id) {
                    return Err("No selected UI widget to duplicate".to_string());
                }
            }
            "delete" => {
                let id = self
                    .ui_designer
                    .document
                    .selected_widget
                    .clone()
                    .ok_or_else(|| "No selected UI widget".to_string())?;
                self.ui_designer.checkpoint();
                if !remove_ui_widget(&mut self.ui_designer.document.canvas.widgets, &id) {
                    return Err(format!("UI widget not found: {id}"));
                }
                self.ui_designer.document.selected_widget = None;
            }
            "move" => {
                self.ui_designer.checkpoint();
                if !self.ui_designer.document.move_selected(
                    required_number(payload, "dx")? as f32,
                    required_number(payload, "dy")? as f32,
                ) {
                    return Err("No selected UI widget to move".to_string());
                }
            }
            "resize" => {
                self.ui_designer.checkpoint();
                if !self.ui_designer.document.resize_selected(
                    required_number(payload, "width")? as f32,
                    required_number(payload, "height")? as f32,
                ) {
                    return Err("No selected UI widget to resize".to_string());
                }
            }
            "property" => {
                let key = required_string(payload, "key")?;
                let value = payload
                    .get("value")
                    .cloned()
                    .ok_or_else(|| "property action requires value".to_string())?;
                self.ui_designer.checkpoint();
                if !self.ui_designer.document.set_selected_property(key, value) {
                    return Err("No selected UI widget to edit".to_string());
                }
            }
            "reparent" => {
                let widget_id = string_field(payload, "widget_id")
                    .map(ToString::to_string)
                    .or_else(|| self.ui_designer.document.selected_widget.clone())
                    .ok_or_else(|| "No selected UI widget to reparent".to_string())?;
                let parent_id = payload
                    .get("parent_id")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty());
                let mut next = self.ui_designer.document.clone();
                if next.reparent_widget(&widget_id, parent_id)? {
                    self.ui_designer.checkpoint();
                    self.ui_designer.document = next;
                }
            }
            "upsert_binding" => {
                let widget_id = string_field(payload, "widget_id")
                    .map(ToString::to_string)
                    .or_else(|| self.ui_designer.document.selected_widget.clone())
                    .ok_or_else(|| "No selected UI widget for binding".to_string())?;
                let binding = UiBinding2D {
                    property: required_string(payload, "property")?.to_string(),
                    source_path: required_string(payload, "source_path")?.to_string(),
                    fallback: payload.get("fallback").cloned().unwrap_or(Value::Null),
                };
                let mut next = self.ui_designer.document.clone();
                if next.upsert_widget_binding(&widget_id, binding)? {
                    self.ui_designer.checkpoint();
                    self.ui_designer.document = next;
                }
            }
            "remove_binding" => {
                let widget_id = string_field(payload, "widget_id")
                    .map(ToString::to_string)
                    .or_else(|| self.ui_designer.document.selected_widget.clone())
                    .ok_or_else(|| "No selected UI widget for binding".to_string())?;
                let mut next = self.ui_designer.document.clone();
                if next.remove_widget_binding(&widget_id, required_string(payload, "property")?)? {
                    self.ui_designer.checkpoint();
                    self.ui_designer.document = next;
                }
            }
            "upsert_callback" => {
                let widget_id = string_field(payload, "widget_id")
                    .map(ToString::to_string)
                    .or_else(|| self.ui_designer.document.selected_widget.clone())
                    .ok_or_else(|| "No selected UI widget for callback".to_string())?;
                let callback = UiCallback2D {
                    event: required_string(payload, "event")?.to_string(),
                    graph: string_field(payload, "graph")
                        .filter(|value| !value.trim().is_empty())
                        .map(ToString::to_string),
                    function: string_field(payload, "function")
                        .filter(|value| !value.trim().is_empty())
                        .map(ToString::to_string),
                    payload: payload.get("payload").cloned().unwrap_or_else(|| json!({})),
                };
                let mut next = self.ui_designer.document.clone();
                if next.upsert_widget_callback(&widget_id, callback)? {
                    self.ui_designer.checkpoint();
                    self.ui_designer.document = next;
                }
            }
            "remove_callback" => {
                let widget_id = string_field(payload, "widget_id")
                    .map(ToString::to_string)
                    .or_else(|| self.ui_designer.document.selected_widget.clone())
                    .ok_or_else(|| "No selected UI widget for callback".to_string())?;
                let mut next = self.ui_designer.document.clone();
                if next.remove_widget_callback(&widget_id, required_string(payload, "event")?)? {
                    self.ui_designer.checkpoint();
                    self.ui_designer.document = next;
                }
            }
            "align" => {
                self.ui_designer.checkpoint();
                if !self
                    .ui_designer
                    .document
                    .align_selected(required_string(payload, "alignment")?)
                {
                    return Err("Could not align selected UI widget".to_string());
                }
            }
            "resolution" => {
                let width = required_usize(payload, "width")?.clamp(1, 7680) as u32;
                let height = required_usize(payload, "height")?.clamp(1, 4320) as u32;
                self.ui_designer.document.preview_resolution = (width, height);
            }
            "snap" => self.ui_designer.document.snap = required_bool(payload, "value")?,
            "guides" => self.ui_designer.document.guides = required_bool(payload, "value")?,
            "safe_area" => {
                self.ui_designer.document.show_safe_area = required_bool(payload, "value")?
            }
            "tool" => {
                self.ui_designer.document.active_tool =
                    parse_ui_tool(required_string(payload, "tool")?)?
            }
            "validate" => {
                let report = self.ui_designer.document.validate();
                let error_count = report.error_count();
                if error_count > 0 {
                    let details = report
                        .issues
                        .iter()
                        .filter(|issue| {
                            issue.severity
                                == crate::engine::miniforge_2d::validation::ValidationSeverity2D::Error
                        })
                        .map(|issue| format!("{}: {}", issue.path, issue.message))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    return Err(format!(
                        "UI validation failed with {error_count} errors: {details}"
                    ));
                }
            }
            _ => return Err(format!("Unknown UI Designer action: {action}")),
        }
        Ok(())
    }

    fn sequencer_state(&self) -> Value {
        json!({
            "tool": "sequencer",
            "document_path": self.sequencer.document_path,
            "dirty": self.sequencer.dirty,
            "can_undo": !self.sequencer.undo.is_empty(),
            "can_redo": !self.sequencer.redo.is_empty(),
            "sequence": self.sequencer.document.sequence,
            "cursor": self.sequencer.document.cursor,
            "playing": self.sequencer.document.playing,
            "looped": self.sequencer.document.looped,
            "selected_track": self.sequencer.document.selected_track,
            "selected_key": self.sequencer.document.selected_key,
            "track_types": supported_track_types(),
            "waveforms": self.sequencer_waveforms(),
            "valid": self.sequencer.document.sequence.validate(),
        })
    }

    fn tilemap_state(&self) -> Value {
        json!({
            "tool": "tilemap",
            "document_path": self.tilemap.document_path,
            "dirty": self.tilemap.dirty,
            "can_undo": !self.tilemap.undo.is_empty(),
            "can_redo": !self.tilemap.redo.is_empty(),
            "editor": self.tilemap.document,
            "atlas": self.tilemap_atlas_state(),
            "issues": self.tilemap.document.validate(),
        })
    }

    fn ui_designer_state(&self) -> Value {
        let hierarchy = self.ui_designer.document.hierarchy_rows().into_iter().map(|(id, widget_type, depth)| {
            json!({"id": id, "widget_type": widget_type, "depth": depth})
        }).collect::<Vec<_>>();
        let selected_widget_data = self
            .ui_designer
            .document
            .selected_widget
            .as_deref()
            .and_then(|id| self.ui_designer.document.canvas.find_widget(id))
            .cloned();
        json!({
            "tool": "ui_designer",
            "document_path": self.ui_designer.document_path,
            "dirty": self.ui_designer.dirty,
            "can_undo": !self.ui_designer.undo.is_empty(),
            "can_redo": !self.ui_designer.redo.is_empty(),
            "designer": self.ui_designer.document,
            "hierarchy": hierarchy,
            "preview": self.ui_designer.document.preview_layout(),
            "selected_widget_data": selected_widget_data,
            "validation": self.ui_designer.document.validate(),
        })
    }

    fn sequencer_waveforms(&self) -> Vec<Value> {
        let Some(project_path) = self.project_path.as_ref() else {
            return Vec::new();
        };
        self.sequencer
            .document
            .sequence
            .tracks
            .iter()
            .filter(|track| track.track_type == "audio")
            .filter_map(|track| {
                let asset = track
                    .keyframes
                    .iter()
                    .find_map(|key| audio_asset_path(&key.value))?;
                let samples = waveform_peaks_from_wav(&project_path.join(&asset), 256).ok()?;
                Some(json!({
                    "track_id": track.id,
                    "asset": asset,
                    "samples": samples,
                }))
            })
            .collect()
    }

    fn tilemap_atlas_state(&self) -> Value {
        let Some(project_path) = self.project_path.as_ref() else {
            return json!({"source":"", "relative_path":"", "tile_size":16});
        };
        let atlas = find_first_image(&project_path.join("assets/tilesets"))
            .or_else(|| find_first_image(&project_path.join("assets/sprites")));
        let Some(atlas) = atlas else {
            return json!({"source":"", "relative_path":"", "tile_size":16});
        };
        let relative_path = atlas
            .strip_prefix(project_path)
            .unwrap_or(&atlas)
            .to_string_lossy()
            .replace('\\', "/");
        let source = format!("file://{}", atlas.to_string_lossy().replace(' ', "%20"));
        json!({
            "source": source,
            "relative_path": relative_path,
            "tile_size": 16,
        })
    }
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn atomic_write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let temp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("json")
    ));
    fs::write(&temp, bytes).map_err(|error| error.to_string())?;
    fs::rename(&temp, path).map_err(|error| error.to_string())
}

fn sequence_track_mut<'a>(
    sequence: &'a mut Sequencer2D,
    id: &str,
) -> Result<&'a mut SequencerTrack2D, String> {
    sequence
        .tracks
        .iter_mut()
        .find(|track| track.id == id)
        .ok_or_else(|| format!("Sequencer track not found: {id}"))
}

fn remove_ui_widget(widgets: &mut Vec<UiWidget2D>, id: &str) -> bool {
    let before = widgets.len();
    widgets.retain(|widget| widget.id != id);
    if widgets.len() != before {
        return true;
    }
    widgets
        .iter_mut()
        .any(|widget| remove_ui_widget(&mut widget.children, id))
}

fn set_key_curve_metadata(value: &mut Value, in_tangent: f64, out_tangent: f64) {
    if !value.is_object() {
        let previous = std::mem::replace(value, Value::Null);
        *value = json!({"value": previous});
    }
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "__curve".to_string(),
            json!({"in_tangent": in_tangent, "out_tangent": out_tangent}),
        );
    }
}

fn flood_fill_layer(
    layer: &mut TileLayer,
    width: usize,
    height: usize,
    origin: (usize, usize),
    previous: i32,
    value: i32,
) {
    let mut queue = VecDeque::from([origin]);
    let mut visited = vec![false; width.saturating_mul(height)];
    while let Some((x, y)) = queue.pop_front() {
        if x >= width || y >= height {
            continue;
        }
        let index = y * width + x;
        if visited[index] || layer.get(x, y) != previous {
            continue;
        }
        visited[index] = true;
        layer.set(x, y, value);
        if x > 0 {
            queue.push_back((x - 1, y));
        }
        if x + 1 < width {
            queue.push_back((x + 1, y));
        }
        if y > 0 {
            queue.push_back((x, y - 1));
        }
        if y + 1 < height {
            queue.push_back((x, y + 1));
        }
    }
}

fn audio_asset_path(value: &Value) -> Option<String> {
    match value {
        Value::String(path) if path.to_ascii_lowercase().ends_with(".wav") => Some(path.clone()),
        Value::Object(object) => ["asset", "path", "audio", "clip", "source"]
            .iter()
            .find_map(|key| object.get(*key).and_then(audio_asset_path)),
        Value::Array(values) => values.iter().find_map(audio_asset_path),
        _ => None,
    }
}

fn waveform_peaks_from_wav(path: &Path, maximum_peaks: usize) -> Result<Vec<f32>, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Audio clip is not a RIFF/WAVE file".to_string());
    }
    let mut offset = 12usize;
    let mut format = 0u16;
    let mut channels = 0usize;
    let mut bits = 0usize;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end >= start + 16 {
            format = u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
            channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()) as usize;
            bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()) as usize;
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = start.saturating_add(size).saturating_add(size % 2);
    }
    if format != 1 || channels == 0 || !matches!(bits, 8 | 16) {
        return Err("Waveform preview supports PCM 8-bit or 16-bit WAV files".to_string());
    }
    let data = data.ok_or_else(|| "WAV file has no data chunk".to_string())?;
    let bytes_per_sample = bits / 8;
    let frame_stride = channels * bytes_per_sample;
    let frame_count = data.len() / frame_stride;
    if frame_count == 0 {
        return Ok(Vec::new());
    }
    let peak_count = maximum_peaks.max(1).min(frame_count);
    let mut peaks = vec![0.0f32; peak_count];
    for frame in 0..frame_count {
        let mut amplitude = 0.0f32;
        for channel in 0..channels {
            let sample_offset = frame * frame_stride + channel * bytes_per_sample;
            let sample = if bits == 8 {
                ((data[sample_offset] as f32 - 128.0) / 128.0).abs()
            } else {
                let raw =
                    i16::from_le_bytes(data[sample_offset..sample_offset + 2].try_into().unwrap());
                (raw as f32 / i16::MAX as f32).abs()
            };
            amplitude = amplitude.max(sample);
        }
        let bucket = frame * peak_count / frame_count;
        peaks[bucket] = peaks[bucket].max(amplitude.min(1.0));
    }
    Ok(peaks)
}

fn find_first_image(root: &Path) -> Option<PathBuf> {
    let mut entries = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if let Some(found) = find_first_image(&path) {
                return Some(found);
            }
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "png" | "jpg" | "jpeg"
                )
            })
        {
            return Some(path);
        }
    }
    None
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    string_field(value, key).ok_or_else(|| format!("Missing string field: {key}"))
}

fn number_field(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn required_number(value: &Value, key: &str) -> Result<f64, String> {
    number_field(value, key)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("Missing finite number field: {key}"))
}

fn usize_field(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn required_usize(value: &Value, key: &str) -> Result<usize, String> {
    usize_field(value, key).ok_or_else(|| format!("Missing non-negative integer field: {key}"))
}

fn i32_field(value: &Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn required_i32(value: &Value, key: &str) -> Result<i32, String> {
    i32_field(value, key).ok_or_else(|| format!("Missing integer field: {key}"))
}

fn required_bool(value: &Value, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("Missing bool field: {key}"))
}

fn required_point(value: &Value, key: &str) -> Result<(usize, usize), String> {
    let point = value
        .get(key)
        .ok_or_else(|| format!("Missing point field: {key}"))?;
    Ok((required_usize(point, "x")?, required_usize(point, "y")?))
}

fn tile_region_payload(
    editor: &TilemapEditor2D,
    payload: &Value,
) -> Result<(usize, (usize, usize), (usize, usize), i32), String> {
    Ok((
        usize_field(payload, "layer").unwrap_or(editor.active_layer),
        required_point(payload, "start")?,
        required_point(payload, "end")?,
        i32_field(payload, "value").unwrap_or(editor.palette.selected),
    ))
}

fn parse_brush(value: &str) -> Result<TileBrushKind2D, String> {
    match value.to_ascii_lowercase().as_str() {
        "pencil" => Ok(TileBrushKind2D::Pencil),
        "eraser" => Ok(TileBrushKind2D::Eraser),
        "fill" => Ok(TileBrushKind2D::Fill),
        "rectangle" => Ok(TileBrushKind2D::Rectangle),
        "line" => Ok(TileBrushKind2D::Line),
        "random" => Ok(TileBrushKind2D::Random),
        "terrain" => Ok(TileBrushKind2D::Terrain),
        "collision" => Ok(TileBrushKind2D::Collision),
        "object" => Ok(TileBrushKind2D::Object),
        "stamp" => Ok(TileBrushKind2D::Stamp),
        "rule" => Ok(TileBrushKind2D::Rule),
        _ => Err(format!("Unknown tile brush: {value}")),
    }
}

fn parse_ui_tool(value: &str) -> Result<UiDesignerTool2D, String> {
    match value.to_ascii_lowercase().as_str() {
        "select" => Ok(UiDesignerTool2D::Select),
        "move" => Ok(UiDesignerTool2D::Move),
        "scale" => Ok(UiDesignerTool2D::Scale),
        "addwidget" | "add_widget" => Ok(UiDesignerTool2D::AddWidget),
        _ => Err(format!("Unknown UI Designer tool: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "miniforge_tool_sessions_{name}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn sequencer_actions_undo_redo_and_persist() {
        let root = temp_project("sequencer");
        let mut sessions = EditorToolSessions::default();
        sessions.open_project(&root).unwrap();
        sessions
            .action(
                "sequencer",
                "new",
                &json!({"name":"Cutscene","duration":8.0,"frame_rate":24.0}),
            )
            .unwrap();
        sessions
            .action(
                "sequencer",
                "add_track",
                &json!({"id":"hero","track_type":"transform","target":"Hero"}),
            )
            .unwrap();
        sessions
            .action(
                "sequencer",
                "add_keyframe",
                &json!({"track_id":"hero","time":2.0,"value":{"x":4}}),
            )
            .unwrap();
        assert_eq!(
            sessions.state("sequencer").unwrap()["sequence"]["tracks"][0]["keyframes"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        sessions.action("sequencer", "undo", &json!({})).unwrap();
        assert_eq!(
            sessions.state("sequencer").unwrap()["sequence"]["tracks"][0]["keyframes"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        sessions.action("sequencer", "redo", &json!({})).unwrap();
        sessions
            .action(
                "sequencer",
                "set_tangents",
                &json!({"track_id":"hero","index":0,"in_tangent":-0.5,"out_tangent":0.75}),
            )
            .unwrap();
        assert_eq!(
            sessions.state("sequencer").unwrap()["sequence"]["tracks"][0]["keyframes"][0]["value"]
                ["__curve"]["out_tangent"],
            0.75
        );
        sessions.action("sequencer", "save", &json!({})).unwrap();
        assert!(root.join("assets/animations/Timeline.seq2d").exists());
        assert!(
            !sessions.state("sequencer").unwrap()["dirty"]
                .as_bool()
                .unwrap()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn wav_waveform_preview_extracts_bounded_pcm_peaks() {
        let root = temp_project("waveform");
        let path = root.join("assets/audio/test.wav");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let samples = [0i16, 8_000, -16_000, i16::MAX, -4_000, 0];
        let data_size = (samples.len() * 2) as u32;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_size).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&44_100u32.to_le_bytes());
        wav.extend_from_slice(&(44_100u32 * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(&path, wav).unwrap();
        let peaks = waveform_peaks_from_wav(&path, 4).unwrap();
        assert_eq!(peaks.len(), 4);
        assert!(peaks.iter().copied().fold(0.0f32, f32::max) > 0.95);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn tilemap_stroke_undo_and_persist() {
        let root = temp_project("tilemap");
        let mut sessions = EditorToolSessions::default();
        sessions.open_project(&root).unwrap();
        sessions
            .action(
                "tilemap",
                "paint_cells",
                &json!({"cells":[{"x":1,"y":2},{"x":2,"y":2}],"value":9}),
            )
            .unwrap();
        assert_eq!(
            sessions.state("tilemap").unwrap()["editor"]["tilemap"]["layers"][0]["tiles"][2][1],
            9
        );
        sessions.action("tilemap", "undo", &json!({})).unwrap();
        assert_eq!(
            sessions.state("tilemap").unwrap()["editor"]["tilemap"]["layers"][0]["tiles"][2][1],
            0
        );
        sessions.action("tilemap", "redo", &json!({})).unwrap();
        sessions
            .action(
                "tilemap",
                "flood_fill",
                &json!({"origin":{"x":0,"y":0},"value":4}),
            )
            .unwrap();
        assert_eq!(
            sessions.state("tilemap").unwrap()["editor"]["tilemap"]["layers"][0]["tiles"][0][0],
            4
        );
        sessions
            .action(
                "tilemap",
                "add_terrain_rule",
                &json!({"terrain_set":"Ground","name":"GrassEdge","center_tile":1,"output_tile":2,"priority":3}),
            )
            .unwrap();
        assert!(
            sessions
                .tilemap
                .document
                .terrain_sets
                .iter()
                .flat_map(|set| &set.rules)
                .any(|rule| rule.name == "GrassEdge")
        );
        sessions.action("tilemap", "save", &json!({})).unwrap();
        assert!(root.join("assets/tilemaps/World.tilemap2d.json").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ui_designer_actions_undo_validate_and_persist_canvas() {
        let root = temp_project("ui");
        let mut sessions = EditorToolSessions::default();
        sessions.open_project(&root).unwrap();
        sessions
            .action(
                "ui_designer",
                "add_widget",
                &json!({"widget_type":"Button","id":"TestButton","x":100,"y":80}),
            )
            .unwrap();
        sessions
            .action("ui_designer", "move", &json!({"dx":16,"dy":8}))
            .unwrap();
        assert_eq!(
            sessions.state("ui_designer").unwrap()["designer"]["selected_widget"],
            "TestButton"
        );
        sessions.action("ui_designer", "undo", &json!({})).unwrap();
        sessions.action("ui_designer", "redo", &json!({})).unwrap();
        sessions
            .action(
                "ui_designer",
                "reparent",
                &json!({"widget_id":"TestButton","parent_id":"HealthPanel"}),
            )
            .unwrap();
        sessions
            .action(
                "ui_designer",
                "upsert_binding",
                &json!({"widget_id":"TestButton","property":"text","source_path":"quest.active_title","fallback":"Start"}),
            )
            .unwrap();
        sessions
            .action(
                "ui_designer",
                "upsert_callback",
                &json!({"widget_id":"TestButton","event":"click","graph":"scripts/visual_graphs/Start.mfgraph","payload":{"screen":"game"}}),
            )
            .unwrap();
        let ui_state = sessions.state("ui_designer").unwrap();
        assert_eq!(
            ui_state["selected_widget_data"]["bindings"][0]["property"],
            "text"
        );
        assert_eq!(
            ui_state["selected_widget_data"]["callbacks"][0]["event"],
            "click"
        );
        sessions
            .action("ui_designer", "validate", &json!({}))
            .unwrap();
        sessions.action("ui_designer", "save", &json!({})).unwrap();
        let saved: Value =
            serde_json::from_slice(&fs::read(root.join("assets/ui/hud.mfui")).unwrap()).unwrap();
        assert!(saved.get("widgets").is_some());
        let _ = fs::remove_dir_all(root);
    }
}
