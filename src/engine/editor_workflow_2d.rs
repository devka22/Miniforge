use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::editor_ui::fuzzy_rank;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EditorContext2D {
    #[default]
    Global,
    Scene,
    Sprite,
    Tilemap,
    Animation,
    UserInterface,
    Script,
    PlayMode,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneTool2D {
    #[default]
    Select,
    Move,
    Rotate,
    Scale,
    Pivot,
    Measure,
    Paint,
    Erase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorAction2D {
    pub id: String,
    pub title: String,
    pub category: String,
    pub shortcut: Option<String>,
    #[serde(default)]
    pub contexts: BTreeSet<EditorContext2D>,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub undoable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionMatch2D {
    pub id: String,
    pub title: String,
    pub category: String,
    pub shortcut: Option<String>,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyEdit2D {
    pub target: String,
    pub property: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorTransaction2D {
    pub label: String,
    pub edits: Vec<PropertyEdit2D>,
}

impl EditorTransaction2D {
    pub fn batch_property(
        label: impl Into<String>,
        targets: impl IntoIterator<Item = (String, Value)>,
        property: impl Into<String>,
        after: Value,
    ) -> Self {
        let property = property.into();
        Self {
            label: label.into(),
            edits: targets
                .into_iter()
                .map(|(target, before)| PropertyEdit2D {
                    target,
                    property: property.clone(),
                    before,
                    after: after.clone(),
                })
                .collect(),
        }
    }

    pub fn inverse(&self) -> Self {
        Self {
            label: format!("Undo {}", self.label),
            edits: self
                .edits
                .iter()
                .rev()
                .map(|edit| PropertyEdit2D {
                    target: edit.target.clone(),
                    property: edit.property.clone(),
                    before: edit.after.clone(),
                    after: edit.before.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorWorkflow2D {
    pub active_context: EditorContext2D,
    pub active_tool: SceneTool2D,
    pub grid_size: [f32; 2],
    pub grid_snap: bool,
    pub pixel_snap: bool,
    #[serde(default)]
    pub selection: BTreeSet<String>,
    #[serde(default)]
    pub actions: BTreeMap<String, EditorAction2D>,
    #[serde(default)]
    pub recent_actions: Vec<String>,
    #[serde(default)]
    pub undo_stack: Vec<EditorTransaction2D>,
    #[serde(default)]
    pub redo_stack: Vec<EditorTransaction2D>,
}

impl Default for EditorWorkflow2D {
    fn default() -> Self {
        let actions = default_actions_2d()
            .into_iter()
            .map(|action| (action.id.clone(), action))
            .collect();
        Self {
            active_context: EditorContext2D::Scene,
            active_tool: SceneTool2D::Select,
            grid_size: [16.0, 16.0],
            grid_snap: true,
            pixel_snap: true,
            selection: BTreeSet::new(),
            actions,
            recent_actions: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

impl EditorWorkflow2D {
    pub fn actions_for_context(&self, context: EditorContext2D) -> Vec<&EditorAction2D> {
        self.actions
            .values()
            .filter(|action| {
                action.contexts.is_empty()
                    || action.contexts.contains(&EditorContext2D::Global)
                    || action.contexts.contains(&context)
            })
            .collect()
    }

    pub fn search_actions(&self, query: &str, limit: usize) -> Vec<ActionMatch2D> {
        let actions = self.actions_for_context(self.active_context);
        let searchable = actions
            .iter()
            .map(|action| {
                format!(
                    "{} {} {} {}",
                    action.title,
                    action.category,
                    action.id,
                    action.keywords.join(" ")
                )
            })
            .collect::<Vec<_>>();
        let mut matches = fuzzy_rank(query, &searchable, actions.len())
            .into_iter()
            .map(|result| {
                let action = actions[result.index];
                ActionMatch2D {
                    id: action.id.clone(),
                    title: action.title.clone(),
                    category: action.category.clone(),
                    shortcut: action.shortcut.clone(),
                    score: result.score.min(i32::MAX as u32) as i32
                        + if self.recent_actions.iter().any(|id| id == &action.id) {
                            20
                        } else {
                            0
                        },
                }
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.title.cmp(&right.title))
        });
        matches.truncate(limit);
        matches
    }

    pub fn trigger_action(&mut self, id: &str) -> bool {
        if !self.actions.contains_key(id) {
            return false;
        }
        self.recent_actions.retain(|recent| recent != id);
        self.recent_actions.insert(0, id.to_string());
        self.recent_actions.truncate(12);
        true
    }

    pub fn set_selection(&mut self, selection: impl IntoIterator<Item = String>) {
        self.selection = selection.into_iter().collect();
    }

    pub fn push_transaction(&mut self, transaction: EditorTransaction2D) {
        if transaction.edits.is_empty() {
            return;
        }
        self.undo_stack.push(transaction);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> Option<EditorTransaction2D> {
        let transaction = self.undo_stack.pop()?;
        let inverse = transaction.inverse();
        self.redo_stack.push(transaction);
        Some(inverse)
    }

    pub fn redo(&mut self) -> Option<EditorTransaction2D> {
        let transaction = self.redo_stack.pop()?;
        self.undo_stack.push(transaction.clone());
        Some(transaction)
    }
}

pub fn default_actions_2d() -> Vec<EditorAction2D> {
    vec![
        action(
            "project.quick_open",
            "Quick Open Asset",
            "Project",
            Some("Ctrl+P"),
            &[EditorContext2D::Global],
            &["find", "asset", "scene"],
            false,
        ),
        action(
            "editor.command_palette",
            "Command Palette",
            "Editor",
            Some("Ctrl+Shift+P"),
            &[EditorContext2D::Global],
            &["actions", "search"],
            false,
        ),
        action(
            "scene.frame_selection",
            "Frame Selection",
            "Scene 2D",
            Some("F"),
            &[EditorContext2D::Scene],
            &["focus", "camera"],
            false,
        ),
        action(
            "scene.toggle_pixel_snap",
            "Toggle Pixel Snap",
            "Scene 2D",
            None,
            &[
                EditorContext2D::Scene,
                EditorContext2D::Sprite,
                EditorContext2D::Tilemap,
            ],
            &["grid", "sharp", "pixel art"],
            false,
        ),
        action(
            "scene.toggle_collision",
            "Show 2D Collision",
            "Debug",
            None,
            &[EditorContext2D::Scene, EditorContext2D::PlayMode],
            &["physics", "overlay", "shapes"],
            false,
        ),
        action(
            "scene.smart_snap",
            "Toggle Smart Snapping",
            "Scene 2D",
            None,
            &[EditorContext2D::Scene],
            &["guide", "align", "edge", "center"],
            false,
        ),
        action(
            "scene.align_selection",
            "Align Selected Objects",
            "Scene 2D",
            None,
            &[EditorContext2D::Scene, EditorContext2D::UserInterface],
            &["left", "center", "right", "distribute"],
            true,
        ),
        action(
            "scene.group_selection",
            "Group Selected Objects",
            "Scene 2D",
            Some("Ctrl+Shift+G"),
            &[EditorContext2D::Scene],
            &["group", "selection", "organize"],
            true,
        ),
        action(
            "scene.edit_pivot",
            "Edit Object Pivot",
            "Scene 2D",
            Some("6"),
            &[EditorContext2D::Scene, EditorContext2D::Sprite],
            &["origin", "anchor", "pivot"],
            true,
        ),
        action(
            "scene.edit_collision",
            "Edit Collision Vertices",
            "Scene 2D",
            Some("7"),
            &[EditorContext2D::Scene, EditorContext2D::Sprite],
            &["collider", "polygon", "vertex"],
            true,
        ),
        action(
            "scene.camera_frame",
            "Toggle Camera Frame",
            "Scene 2D",
            None,
            &[EditorContext2D::Scene],
            &["safe area", "aspect", "preview"],
            false,
        ),
        action(
            "asset.reimport",
            "Reimport Selected Assets",
            "Assets",
            Some("Ctrl+R"),
            &[EditorContext2D::Global],
            &["import", "refresh", "source"],
            true,
        ),
        action(
            "asset.reimport_changed",
            "Reimport Changed Sources",
            "Assets",
            None,
            &[EditorContext2D::Global],
            &["automatic", "dirty", "pipeline"],
            true,
        ),
        action(
            "sprite.extract_sheet",
            "Extract Sprite Sheet",
            "Sprite",
            None,
            &[EditorContext2D::Sprite],
            &["slice", "atlas", "frames"],
            true,
        ),
        action(
            "sprite.edit_collision",
            "Edit Sprite Collision",
            "Sprite",
            None,
            &[EditorContext2D::Sprite],
            &["polygon", "circle", "box"],
            true,
        ),
        action(
            "sprite.edit_sockets",
            "Edit Sprite Sockets",
            "Sprite",
            None,
            &[EditorContext2D::Sprite],
            &["attach", "weapon", "effects"],
            true,
        ),
        action(
            "tilemap.paint",
            "Tilemap Paint Tool",
            "Tilemap",
            Some("B"),
            &[EditorContext2D::Tilemap],
            &["brush", "terrain", "stamp"],
            false,
        ),
        action(
            "tilemap.line",
            "Tilemap Line Tool",
            "Tilemap",
            None,
            &[EditorContext2D::Tilemap],
            &["bresenham", "wall", "path"],
            true,
        ),
        action(
            "tilemap.create_pattern",
            "Create Pattern from Selection",
            "Tilemap",
            None,
            &[EditorContext2D::Tilemap],
            &["clipboard", "stamp", "reuse"],
            true,
        ),
        action(
            "animation.preview",
            "Preview Animation",
            "Animation",
            Some("Space"),
            &[EditorContext2D::Animation],
            &["flipbook", "frames", "play"],
            false,
        ),
        action(
            "ui.preview_responsive",
            "Preview Responsive Layout",
            "UI",
            None,
            &[EditorContext2D::UserInterface],
            &["phone", "desktop", "anchors"],
            false,
        ),
        action(
            "script.build",
            "Build Current Script",
            "Script",
            Some("Ctrl+B"),
            &[EditorContext2D::Script],
            &["compile", "validate", "luau", "rust"],
            false,
        ),
        action(
            "tools.python_scene_report",
            "Run Python Scene Report",
            "Tools",
            None,
            &[EditorContext2D::Global],
            &["python", "automation", "production"],
            false,
        ),
        action(
            "play.current_scene",
            "Play Current Scene",
            "Run",
            Some("F6"),
            &[EditorContext2D::Global],
            &["pie", "test", "game"],
            false,
        ),
    ]
}

fn action(
    id: &str,
    title: &str,
    category: &str,
    shortcut: Option<&str>,
    contexts: &[EditorContext2D],
    keywords: &[&str],
    undoable: bool,
) -> EditorAction2D {
    EditorAction2D {
        id: id.to_string(),
        title: title.to_string(),
        category: category.to_string(),
        shortcut: shortcut.map(str::to_string),
        contexts: contexts.iter().copied().collect(),
        keywords: keywords
            .iter()
            .map(|keyword| (*keyword).to_string())
            .collect(),
        undoable,
    }
}
