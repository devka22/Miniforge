//! MiniForge2D is a UE-inspired 2D layer built on top of the existing
//! GameObject/Component, JSON, macroquad and Rhai architecture.
//!
//! The module is intentionally data-first: every system can be serialized as
//! JSON, validated without opening a window, and then adapted into existing
//! runtime components.

pub mod actor;
pub mod ai;
pub mod animation_blueprint;
pub mod asset_registry2d;
pub mod blueprint;
pub mod blueprint_library;
pub mod console_panel;
pub mod content_browser;
pub mod details_inspector;
pub mod editor_layout;
pub mod editor_tabs;
pub mod examples;
pub mod exporter2d;
pub mod gameplay;
pub mod packaging2d;
pub mod paper2d;
pub mod physics2d;
pub mod plugin_system2d;
pub mod problems_panel;
pub mod project_settings2d;
pub mod rts_tools;
pub mod scene_view;
pub mod sequencer2d;
pub mod toolbar;
pub mod ui_designer;
pub mod ui_framework;
pub mod validation;
pub mod world_outliner;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::miniforge_2d::ai::BehaviorTree2D;
use crate::engine::miniforge_2d::animation_blueprint::AnimationBlueprint2D;
use crate::engine::miniforge_2d::blueprint::graph_from_value;
use crate::engine::miniforge_2d::packaging2d::PackageManifest2D;
use crate::engine::miniforge_2d::sequencer2d::Sequencer2D;
use crate::engine::miniforge_2d::validation::{ValidationReport2D, validate_references};
use crate::entities::game_object::GameObject;

pub use actor::{Actor2DFactory, ActorClass2D, ActorDescriptor2D, Transform2D};
pub use ai::{BehaviorNode2D, BehaviorTree2D as MiniBehaviorTree2D, Blackboard2D};
pub use animation_blueprint::{AnimationBlueprint2D as MiniAnimationBlueprint2D, AnimationState2D};
pub use asset_registry2d::{AssetOperation2D, AssetRegistryReport2D};
pub use blueprint::{BlueprintGraph2D as MiniBlueprintGraph2D, BlueprintNode2D};
pub use blueprint_library::BlueprintLibrary2D;
pub use console_panel::ConsolePanel2D;
pub use content_browser::{ContentAsset2D, ContentBrowserCatalog2D, ContentFilter2D};
pub use details_inspector::DetailsInspector2D;
pub use editor_layout::EditorLayout2D;
pub use editor_tabs::EditorTabSession2D;
pub use examples::{examples_document, minimal_examples};
pub use exporter2d::{ExportLayout2D, ExportValidation2D};
pub use gameplay::{GameFramework2D, GameMode2DConfig};
pub use packaging2d::{PackageManifest2D as MiniPackageManifest2D, PackageProfile2D};
pub use paper2d::{FlipbookAnimation2D, Sprite2DAsset, Tilemap2D, Tileset2D};
pub use physics2d::{Physics2DSettings, Raycast2DQuery};
pub use plugin_system2d::PluginManifest2D;
pub use problems_panel::ProblemsPanel2D;
pub use project_settings2d::ProjectSettings2D;
pub use rts_tools::RtsTools2D;
pub use scene_view::SceneView2D;
pub use sequencer2d::Sequencer2D as MiniSequencer2D;
pub use toolbar::{EditorRunState2D, Toolbar2D};
pub use ui_designer::UiDesigner2D;
pub use ui_framework::{UiCanvas2D, UiWidget2D};
pub use validation::{ValidationIssue2D, ValidationSeverity2D};
pub use world_outliner::WorldOutliner2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureModule2D {
    pub name: String,
    pub priority: usize,
    pub asset_extension: String,
    pub component_types: Vec<String>,
    pub description: String,
}

pub fn module_catalog() -> Vec<FeatureModule2D> {
    vec![
        feature(
            "Editor Layout",
            0,
            "editor_layout.json",
            &[],
            "UE-style dark dock layout with menu, toolbar, outliner, inspector and bottom tabs.",
        ),
        feature(
            "Toolbar",
            0,
            ".toolbar2d.json",
            &[],
            "Save, Play, Build, Export, Project Settings and validation status controls.",
        ),
        feature(
            "Actor Component System 2D",
            1,
            ".scene",
            &["Actor2D", "Transform", "SpriteRenderer", "Collider2D"],
            "Actor base + JSON components compatible with existing GameObject.",
        ),
        feature(
            "Game Framework 2D",
            2,
            ".game2d.json",
            &[
                "GameMode2D",
                "Pawn2D",
                "PlayerController2D",
                "AIController2D",
            ],
            "GameMode, Pawn and Controllers for Play-In-Editor and runtime.",
        ),
        feature(
            "Blueprint Graph 2D",
            3,
            ".mfgraph",
            &["VisualScript"],
            "Exec pins, variables, functions, branches, delay and gameplay nodes.",
        ),
        feature(
            "Content Browser 2D",
            4,
            ".asset_metadata.json",
            &["AssetIdentity2D"],
            "GUIDs, previews, filters, labels and dependency validation.",
        ),
        feature(
            "Details Inspector 2D",
            5,
            ".details2d.json",
            &[],
            "Editable sections for entities, components and assets.",
        ),
        feature(
            "World Outliner 2D",
            6,
            ".outliner2d.json",
            &[],
            "Hierarchy, search, enabled state and reparenting helpers.",
        ),
        feature(
            "Scene View 2D",
            6,
            ".sceneview2d.json",
            &[],
            "Zoom, pan, grid, snap, selection, gizmos, overlays and debug draw state.",
        ),
        feature(
            "Paper2D-like",
            7,
            ".paper2d.json",
            &[
                "SpriteRenderer",
                "TilemapRenderer2D",
                "Tileset2D",
                "FlipbookAnimation2D",
            ],
            "Sprites, tilemaps, tilesets, flipbooks and collision tiles.",
        ),
        feature(
            "Animation Blueprint 2D",
            8,
            ".anim2d.json",
            &["AnimationBlueprint2D", "Animator"],
            "Animation states, transitions, parameters and frame events.",
        ),
        feature(
            "UMG-like UI",
            9,
            ".ui2d.json",
            &["WidgetCanvas2D", "UIElement"],
            "Canvas, panel, text, button, image and progress bar widgets.",
        ),
        feature(
            "Sequencer2D",
            10,
            ".seq2d.json",
            &["Sequencer2D"],
            "Timeline tracks for camera, audio, events and dialogue.",
        ),
        feature(
            "Physics2D",
            11,
            ".physics2d.json",
            &["Rigidbody2D", "Collider2D", "Trigger2D"],
            "Rigid bodies, colliders, triggers, layers, raycasts and debug draw.",
        ),
        feature(
            "AI Behavior Trees",
            12,
            ".bt2d.json",
            &["Blackboard", "BehaviorTree2D", "AIController2D"],
            "Blackboard + BT tasks for Patrol, Chase, Attack, Flee and RTS commands.",
        ),
        feature(
            "Packaging",
            13,
            ".package2d.json",
            &[],
            "Debug/release manifest, used assets and validation summary.",
        ),
        feature(
            "UI Designer",
            14,
            ".mfui",
            &["WidgetCanvas2D"],
            "Visual UI document editor state with widget hierarchy, selection, snap and preview.",
        ),
        feature(
            "Blueprint Library",
            15,
            ".mfgraph",
            &["VisualScript"],
            "Searchable graph templates for players, enemies, pickups, UI, RTS and save points.",
        ),
        feature(
            "RTS Tools",
            20,
            ".rts2d.json",
            &["RTSController", "Commandable", "ProductionQueue"],
            "Selection, command system, production, squads, flow fields, influence and fog data.",
        ),
    ]
}

pub fn install_recommended_components(entity: &mut GameObject) {
    actor::ensure_component(entity, "Actor2D");
    actor::ensure_component(entity, "Transform");
    actor::ensure_component(entity, "SpriteRenderer");
    actor::ensure_component(entity, "Collider2D");
}

pub fn validate_miniforge_2d_value(value: &Value) -> ValidationReport2D {
    let mut report = ValidationReport2D::default();
    if value.get("nodes").is_some() && value.get("edges").is_some() {
        match graph_from_value(value) {
            Ok(graph) => report.merge(graph.validate()),
            Err(error) => report.error("blueprint_parse", "blueprint", error.to_string()),
        }
    }
    if let Some(animation) = value.get("animation_blueprint") {
        match serde_json::from_value::<AnimationBlueprint2D>(animation.clone()) {
            Ok(animation) => report.merge(animation.validate()),
            Err(error) => report.error("animation_blueprint_parse", "animation", error.to_string()),
        }
    }
    if let Some(tree) = value.get("behavior_tree") {
        match serde_json::from_value::<BehaviorTree2D>(tree.clone()) {
            Ok(tree) => report.merge(tree.validate()),
            Err(error) => report.error("behavior_tree_parse", "ai", error.to_string()),
        }
    }
    if let Some(sequence) = value.get("sequencer2d") {
        match serde_json::from_value::<Sequencer2D>(sequence.clone()) {
            Ok(sequence) if !sequence.validate() => report.error(
                "sequencer_invalid",
                "sequencer2d",
                "Sequencer2D tiene keyframes fuera de rango o frame_rate invalido.",
            ),
            Ok(_) => {}
            Err(error) => report.error("sequencer_parse", "sequencer2d", error.to_string()),
        }
    }
    if let Some(package) = value.get("packaging") {
        match serde_json::from_value::<PackageManifest2D>(package.clone()) {
            Ok(mut package) => report.merge(package.validate()),
            Err(error) => report.error("package_parse", "packaging", error.to_string()),
        }
    }
    report.merge(validate_references(
        value,
        &Default::default(),
        &Default::default(),
    ));
    report
}

fn feature(
    name: &str,
    priority: usize,
    extension: &str,
    components: &[&str],
    description: &str,
) -> FeatureModule2D {
    FeatureModule2D {
        name: name.to_string(),
        priority,
        asset_extension: extension.to_string(),
        component_types: components.iter().map(|item| item.to_string()).collect(),
        description: description.to_string(),
    }
}
