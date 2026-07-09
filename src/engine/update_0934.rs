use serde::{Deserialize, Serialize};

use crate::engine::script_host_2d::{ScriptBackend2D, ScriptHost2D};

pub const FOUNDATION_VERSION: &str = "0.9.3.4";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FoundationReleaseState {
    Development,
    ReleaseCandidate,
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationCapability0934 {
    pub area: String,
    pub foundation: String,
    pub inspiration: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Engine0934FoundationPlan {
    pub version: String,
    pub release_state: FoundationReleaseState,
    pub launch_allowed: bool,
    pub focus: String,
    pub capabilities: Vec<FoundationCapability0934>,
    pub language_backends: Vec<ScriptBackend2D>,
}

impl Engine0934FoundationPlan {
    pub fn current() -> Self {
        Self {
            version: FOUNDATION_VERSION.to_string(),
            release_state: FoundationReleaseState::Released,
            launch_allowed: true,
            focus: "2D-first editor, workflow, asset pipeline and scripting interoperability"
                .to_string(),
            capabilities: vec![
                capability(
                    "2D asset workflow",
                    "Source fingerprints, import presets, batch options, dependency-aware reimport and rebuildable generated files",
                    "Godot import metadata/reimport + Unreal Content Browser asset actions",
                ),
                capability(
                    "2D editor workflow",
                    "Context actions, fuzzy command palette, tool modes, multi-selection property transactions and undo/redo",
                    "Godot canvas workflow + Unreal editor modes and Property Matrix",
                ),
                capability(
                    "Sprite production",
                    "First-class actions for sheet extraction, collision editing, sockets, pixel snap and animation preview",
                    "Unreal Paper 2D Sprite Editor + Godot SpriteFrames editor",
                ),
                capability(
                    "Language bridge",
                    "Versioned JSON-value call ABI with explicit capabilities and backend readiness",
                    "Godot cross-language calls/GDExtension, adapted to a Rust-owned core",
                ),
                capability(
                    "Vector editor rendering",
                    "Lyon-backed Bezier paths, smooth strokes, polygon fills, selection outlines and reusable gizmo meshes",
                    "Godot Path2D, Line2D and polygon editor workflows",
                ),
                capability(
                    "Spatial authoring",
                    "Smart edge/center snapping, alignment, distribution, groups, layer state, pivots and editable collision polygons",
                    "Godot CanvasItem editor guides and 2D editor plugins",
                ),
                capability(
                    "Python production tools",
                    "Trusted editor-only subprocess tools with a versioned JSON protocol, timeout and validated operations",
                    "Production automation without putting Python in exported gameplay",
                ),
            ],
            language_backends: ScriptHost2D::foundation().language_matrix(),
        }
    }

    pub fn development() -> Self {
        Self::current()
    }

    pub fn is_unreleased(&self) -> bool {
        !self.launch_allowed || self.release_state != FoundationReleaseState::Released
    }
}

impl Default for Engine0934FoundationPlan {
    fn default() -> Self {
        Self::current()
    }
}

fn capability(area: &str, foundation: &str, inspiration: &str) -> FoundationCapability0934 {
    FoundationCapability0934 {
        area: area.to_string(),
        foundation: foundation.to_string(),
        inspiration: inspiration.to_string(),
        status: "foundation-ready".to_string(),
    }
}
