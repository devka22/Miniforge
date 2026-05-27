use std::fs;

use miniforge::engine::component::default_component;
use miniforge::engine::miniforge_2d::actor::Actor2DFactory;
use miniforge::engine::miniforge_2d::ai::minimal_behavior_tree;
use miniforge::engine::miniforge_2d::animation_blueprint::minimal_animation_blueprint;
use miniforge::engine::miniforge_2d::blueprint::minimal_blueprint_graph;
use miniforge::engine::miniforge_2d::blueprint_library::BlueprintLibrary2D;
use miniforge::engine::miniforge_2d::console_panel::{ConsoleCommandResult2D, ConsolePanel2D};
use miniforge::engine::miniforge_2d::content_browser::{ContentBrowserCatalog2D, ContentFilter2D};
use miniforge::engine::miniforge_2d::details_inspector::DetailsInspector2D;
use miniforge::engine::miniforge_2d::editor_layout::EditorLayout2D;
use miniforge::engine::miniforge_2d::editor_tabs::EditorTabSession2D;
use miniforge::engine::miniforge_2d::examples::minimal_examples;
use miniforge::engine::miniforge_2d::gameplay::GameFramework2D;
use miniforge::engine::miniforge_2d::module_catalog;
use miniforge::engine::miniforge_2d::packaging2d::minimal_package_manifest;
use miniforge::engine::miniforge_2d::paper2d::{
    FlipbookAnimation2D, FlipbookFrame2D, Tilemap2D, TilemapLayer2D,
};
use miniforge::engine::miniforge_2d::physics2d::Physics2DSettings;
use miniforge::engine::miniforge_2d::problems_panel::ProblemsPanel2D;
use miniforge::engine::miniforge_2d::project_settings2d::ProjectSettings2D;
use miniforge::engine::miniforge_2d::rts_tools::RtsTools2D;
use miniforge::engine::miniforge_2d::scene_view::SceneView2D;
use miniforge::engine::miniforge_2d::sequencer2d::minimal_sequencer;
use miniforge::engine::miniforge_2d::toolbar::{EditorRunState2D, Toolbar2D, ToolbarStatus2D};
use miniforge::engine::miniforge_2d::ui_designer::UiDesigner2D;
use miniforge::engine::miniforge_2d::ui_framework::{
    minimal_ui_canvas, supported_ui_events, supported_widget_types,
};
use miniforge::engine::miniforge_2d::validation::ValidationSeverity2D;
use miniforge::engine::miniforge_2d::world_outliner::WorldOutliner2D;
use miniforge::systems::physics_system::PhysicsSystem;
use serde_json::json;

#[test]
fn miniforge_2d_component_defaults_are_registered() {
    for component_type in [
        "Actor2D",
        "GameMode2D",
        "GameState2D",
        "PlayerState2D",
        "Pawn2D",
        "PlayerController2D",
        "AIController2D",
        "TilemapRenderer2D",
        "AnimationBlueprint2D",
        "Animator2D",
        "ScriptComponent",
        "VisualGraphComponent",
        "AudioSource2D",
        "Camera2D",
        "WidgetCanvas2D",
        "Sequencer2D",
        "Trigger2D",
        "StaticBody2D",
        "KinematicBody2D",
        "BehaviorTree2D",
    ] {
        assert!(
            default_component(component_type).is_some(),
            "{component_type} should have defaults"
        );
    }
}

#[test]
fn actor_factory_and_editor_surfaces_use_existing_game_object_model() {
    let mut pawn = Actor2DFactory::pawn("PlayerPawn", 4.0, 5.0);
    let child = Actor2DFactory::actor("WeaponSocket", 4.0, 5.0);
    pawn.parent_id = Some(child.id);

    assert_eq!(pawn.entity_type, "Pawn2D");
    assert!(pawn.get_component("Pawn2D").is_some());
    assert!(pawn.get_component("CharacterController2D").is_some());

    let inspector = DetailsInspector2D::from_entity(&pawn);
    assert!(inspector.editable_field_count() > 5);

    let outliner = WorldOutliner2D::from_entities(&[child.clone(), pawn]);
    assert_eq!(outliner.search("player").len(), 1);

    let framework = GameFramework2D::default();
    assert_eq!(framework.spawn_minimal_entities().len(), 4);
}

#[test]
fn blueprint_animation_ai_and_sequence_validate() {
    let graph = minimal_blueprint_graph();
    assert!(graph.validate().is_valid());
    let component = graph.to_visual_script_component();
    assert_eq!(component.component_type, "VisualScript");
    assert!(
        component
            .get("nodes")
            .and_then(|value| value.as_array())
            .is_some()
    );

    let animation = minimal_animation_blueprint();
    assert!(animation.validate().is_valid());

    let behavior_tree = minimal_behavior_tree();
    assert!(behavior_tree.validate().is_valid());
    assert!(behavior_tree.task_order().contains(&"Attack".to_string()));

    let sequence = minimal_sequencer();
    assert!(sequence.validate());
    assert_eq!(sequence.sample_events(0.0, 2.0).len(), 1);
}

#[test]
fn paper_ui_physics_content_and_packaging_have_minimal_runtime_paths() {
    let flipbook = FlipbookAnimation2D {
        name: "Blink".to_string(),
        frames_per_second: 2.0,
        looping: true,
        frames: vec![
            FlipbookFrame2D {
                sprite: "A".to_string(),
                duration: 0.5,
            },
            FlipbookFrame2D {
                sprite: "B".to_string(),
                duration: 0.5,
            },
        ],
        frame_events: Vec::new(),
    };
    assert_eq!(flipbook.sprite_at_time(0.75), Some("B"));

    let tilemap = Tilemap2D {
        name: "Map".to_string(),
        tileset: "Demo".to_string(),
        width: 2,
        height: 2,
        tile_width: 16,
        tile_height: 16,
        chunk_width: 16,
        chunk_height: 16,
        layers: vec![TilemapLayer2D {
            name: "Ground".to_string(),
            visible: true,
            collision: false,
            tiles: vec![0, 1, 2, 3],
        }],
        autotiles: Vec::new(),
        animated_tiles: Vec::new(),
    };
    assert_eq!(tilemap.tile_at("Ground", 1, 1), Some(3));

    let ui = minimal_ui_canvas();
    assert!(ui.validate_widget_ids());
    assert!(ui.find_widget("HealthBar").is_some());

    let settings = Physics2DSettings::default();
    let mut physics = PhysicsSystem::new();
    settings.apply_to_system(&mut physics);
    assert_eq!(physics.gravity, (0.0, 18.0));

    let mut catalog = ContentBrowserCatalog2D::default();
    catalog.insert_json_asset(
        "assets/data/item.json",
        "DataAsset2D",
        json!({"name": "Potion"}),
    );
    assert_eq!(catalog.filter(&ContentFilter2D::default()).len(), 1);
    assert!(catalog.validate().is_valid());

    let mut package = minimal_package_manifest();
    assert!(package.validate().is_valid());
}

#[test]
fn guide_editor_productivity_modules_are_represented() {
    let layout = EditorLayout2D::default();
    assert!(layout.validate().is_valid());
    assert!(
        layout
            .panels
            .iter()
            .any(|panel| panel.id == "world_outliner")
    );

    let toolbar = Toolbar2D::new(ToolbarStatus2D {
        state: EditorRunState2D::Editing,
        ..ToolbarStatus2D::default()
    });
    assert!(toolbar.available_actions().len() >= 10);

    let mut tabs = EditorTabSession2D::default();
    let graph_tab = tabs.open("scripts/visual_graphs/Test.mfgraph");
    assert!(tabs.mark_dirty(&graph_tab, true));
    assert!(!tabs.close(&graph_tab, false));
    assert!(tabs.close(&graph_tab, true));
    assert!(tabs.reopen_last_closed().is_some());

    let mut console = ConsolePanel2D::default();
    assert!(matches!(
        console.execute("validate_project"),
        ConsoleCommandResult2D::ValidateProject
    ));
    assert!(
        ConsolePanel2D::commands()
            .iter()
            .any(|cmd| cmd.name == "build_release")
    );

    let mut report = miniforge::engine::miniforge_2d::validation::ValidationReport2D::default();
    report.error("duplicate_guid", "asset", "duplicate");
    let problems = ProblemsPanel2D::from_report(&report);
    assert_eq!(problems.toolbar_counts(), (1, 0));
    assert_eq!(
        problems
            .filter(None, Some(ValidationSeverity2D::Error))
            .len(),
        1
    );

    let settings = ProjectSettings2D::default();
    assert!(settings.validate().is_valid());

    let mut scene_view = SceneView2D::default();
    assert!(scene_view.apply_shortcut("W"));

    let ui_designer = UiDesigner2D::default();
    assert!(ui_designer.validate().is_valid());

    let library = BlueprintLibrary2D::default_library();
    assert!(library.search("RTS").len() >= 3);
    assert!(
        library
            .instantiate("Player TopDown")
            .unwrap()
            .validate()
            .is_valid()
    );

    assert!(supported_widget_types().contains(&"InventorySlot"));
    assert!(supported_ui_events().contains(&"OnTextChanged"));

    let rts = RtsTools2D::default();
    assert_eq!(rts.feature_names().len(), 15);
    assert_eq!(rts.minimal_demo_scene().scene_name, "RTS_Minimal_Demo");
}

#[test]
fn examples_cover_every_requested_module_and_json_file_parses() {
    let examples = minimal_examples();
    for key in [
        "actor_component",
        "editor_layout",
        "toolbar",
        "game_framework",
        "blueprint_graph",
        "content_browser",
        "details_inspector",
        "world_outliner",
        "scene_view_2d",
        "paper2d",
        "animation_blueprint",
        "umg_like_ui",
        "ui_designer",
        "blueprint_library",
        "sequencer2d",
        "physics2d",
        "ai_behavior_tree",
        "rts_tools",
        "asset_registry",
        "plugin_system",
        "project_settings",
        "packaging",
        "play_in_editor",
    ] {
        assert!(examples.contains_key(key), "missing example {key}");
    }
    assert!(module_catalog().len() >= 13);

    let text = fs::read_to_string("examples/miniforge_2d/miniforge_2d_examples.json").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["format"], "miniforge_2d_examples");
}
