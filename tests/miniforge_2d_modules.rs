use std::fs;

use miniforge::engine::component::default_component;
use miniforge::engine::miniforge_2d::actor::{Actor2DFactory, Transform2D};
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
use miniforge::engine::miniforge_2d::massive_world2d::{
    ObjectPool2D, RuntimeBudgetStats2D, SaveSharding2D, SpawnDirector2D, SpawnRule2D,
    WorldPartition2D, minimal_massive_world2d,
};
use miniforge::engine::miniforge_2d::module_catalog;
use miniforge::engine::miniforge_2d::packaging2d::minimal_package_manifest;
use miniforge::engine::miniforge_2d::paper2d::{
    FlipbookAnimation2D, FlipbookFrame2D, SpriteFrames2D, Tilemap2D, TilemapLayer2D,
};
use miniforge::engine::miniforge_2d::physics2d::Physics2DSettings;
use miniforge::engine::miniforge_2d::plugin_system2d::{PluginExtensionSlot2D, PluginManifest2D};
use miniforge::engine::miniforge_2d::problems_panel::ProblemsPanel2D;
use miniforge::engine::miniforge_2d::project_settings2d::ProjectSettings2D;
use miniforge::engine::miniforge_2d::rts_tools::RtsTools2D;
use miniforge::engine::miniforge_2d::scene_view::{
    SceneOverlayKind2D, SceneSnapTarget2D, SceneView2D,
};
use miniforge::engine::miniforge_2d::sequencer2d::minimal_sequencer;
use miniforge::engine::miniforge_2d::tilemap_editor2d::TilemapEditor2D;
use miniforge::engine::miniforge_2d::toolbar::{EditorRunState2D, Toolbar2D, ToolbarStatus2D};
use miniforge::engine::miniforge_2d::ui_designer::UiDesigner2D;
use miniforge::engine::miniforge_2d::ui_framework::{
    minimal_ui_canvas, supported_screen_types, supported_ui_events, supported_widget_types,
};
use miniforge::engine::miniforge_2d::validation::ValidationSeverity2D;
use miniforge::engine::miniforge_2d::world_outliner::{OutlinerWarningSeverity2D, WorldOutliner2D};
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
        "Transform3D",
        "MeshRenderer3D",
        "Camera3D",
        "Light3D",
        "Material3D",
        "Billboard3D",
        "HybridScene3D",
        "WorldPartition2D",
        "StreamingChunk2D",
        "RuntimeBudget2D",
        "ObjectPool2D",
        "SpawnDirector2D",
        "SaveShard2D",
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
    assert!(
        layout
            .submenu_entries("Create", "create_2d")
            .iter()
            .any(|item| item.command == "create.sprite2d")
    );
    assert!(
        layout
            .submenu_entries("Tools", "tools_apple")
            .iter()
            .any(|item| item.command == "tools.xcode.debug_plan")
    );

    let toolbar = Toolbar2D::new(ToolbarStatus2D {
        state: EditorRunState2D::Editing,
        ..ToolbarStatus2D::default()
    });
    assert!(toolbar.available_actions().len() >= 10);

    let mut tabs = EditorTabSession2D::default();
    let graph_tab = tabs.open("scripts/visual_graphs/Test.mfgraph");
    tabs.open_in_group("logs/build.log", "output");
    assert_eq!(tabs.tabs_for_group("scripts").len(), 1);
    assert_eq!(tabs.tabs_for_group("output").len(), 1);
    assert!(
        tabs.grouped_tabs()
            .iter()
            .any(|group| group.id == "output" && group.tab_ids.len() == 1)
    );
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
    assert!(supported_widget_types().contains(&"TextInput"));
    assert!(supported_screen_types().contains(&"ScreenManager"));
    assert!(supported_screen_types().contains(&"GameOverScreen"));
    assert!(supported_ui_events().contains(&"OnTextChanged"));

    let rts = RtsTools2D::default();
    assert_eq!(rts.feature_names().len(), 15);
    assert_eq!(rts.minimal_demo_scene().scene_name, "RTS_Minimal_Demo");

    let modules = module_catalog();
    let hybrid = modules
        .iter()
        .find(|module| module.name == "Hybrid 2D + 3D Rendering")
        .expect("hybrid 3d module should be listed");
    assert!(
        hybrid
            .component_types
            .contains(&"MeshRenderer3D".to_string())
    );
    let massive = modules
        .iter()
        .find(|module| module.name == "Massive World 2D")
        .expect("massive world module should be listed");
    assert!(
        massive
            .component_types
            .contains(&"WorldPartition2D".to_string())
    );
}

#[test]
fn godot_inspired_transform_snap_and_tile_patterns_work_as_data() {
    let parent = Transform2D {
        x: 10.0,
        y: 5.0,
        rotation: 90.0,
        scale_x: 2.0,
        scale_y: 2.0,
    };
    let child = Transform2D {
        x: 1.0,
        y: 0.0,
        rotation: 15.0,
        scale_x: 0.5,
        scale_y: 1.0,
    };
    let global = Transform2D::combine(parent, child);
    assert!((global.x - 10.0).abs() < 0.001);
    assert!((global.y - 7.0).abs() < 0.001);
    assert_eq!(global.rotation, 105.0);
    let local = Transform2D::local_from_global(global, parent).unwrap();
    assert!((local.x - child.x).abs() < 0.001);
    assert!((local.y - child.y).abs() < 0.001);
    assert!((parent.local_to_global((2.0, 1.0)).0 - 8.0).abs() < 0.001);

    let mut scene_view = SceneView2D {
        snap: true,
        snap_size: 8.0,
        grid_offset_x: 2.0,
        grid_offset_y: 2.0,
        pixel_snap: true,
        smart_snap: true,
        smart_snap_tolerance: 3.0,
        ..SceneView2D::default()
    };
    let guide_entity = Actor2DFactory::actor("Guide", 18.0, 34.0);
    scene_view.rebuild_guides_from_entities(&[guide_entity]);
    let snapped = scene_view.snap_point((17.2, 35.1));
    assert_eq!(snapped.point, (18.0, 34.0));
    assert_eq!(snapped.target, SceneSnapTarget2D::GuideBoth);
    assert_eq!(
        scene_view.screen_to_world(
            scene_view.world_to_screen((5.0, 9.0), (100.0, 50.0)),
            (100.0, 50.0)
        ),
        (5.0, 9.0)
    );

    let mut editor = TilemapEditor2D::new(4, 4);
    editor.paint_cell(0, 0, 0, 1);
    editor.paint_cell(0, 1, 0, 2);
    editor.paint_cell(0, 0, 1, 3);
    editor.paint_cell(0, 1, 1, 4);
    let selection = editor.select_rectangle(0, (0, 0), (1, 1));
    assert_eq!(selection.cells.len(), 4);
    let pattern = editor.copy_selection("Corner").unwrap();
    assert_eq!(pattern.width, 2);
    assert_eq!(pattern.height, 2);
    let rotated = pattern.rotated_right();
    editor.paste_pattern(0, &rotated, (3, 0));
    assert_eq!(editor.tilemap.layers[0].get(2, 0), 3);
    assert_eq!(editor.tilemap.layers[0].get(3, 0), 1);
    let flipped = pattern.flipped_h();
    editor.paste_pattern(0, &flipped, (3, 2));
    assert_eq!(editor.tilemap.layers[0].get(2, 2), 2);
    assert_eq!(editor.tilemap.layers[0].get(3, 2), 1);
    assert!(editor.validate().is_empty());
}

#[test]
fn godot_editor_productivity_contracts_keep_one_click_paths() {
    let plugin = PluginManifest2D::rts_tools_demo();
    assert!(plugin.validate().is_valid());
    assert_eq!(
        plugin
            .extension_points_for(PluginExtensionSlot2D::SceneOverlay)
            .len(),
        1
    );
    assert!(plugin.canvas_input_forwarding);
    assert!(plugin.canvas_overlay_forwarding);

    let mut root = Actor2DFactory::actor("SceneRoot", 12.0, 0.0);
    root.visible = false;
    let mut child = Actor2DFactory::actor("VisibleChild", 0.0, 0.0);
    child.parent_id = Some(root.id);
    let outliner = WorldOutliner2D::from_entities(&[root.clone(), child.clone()]);
    assert!(outliner.summary().warnings >= 2);
    assert!(
        outliner
            .warnings_for(root.id)
            .iter()
            .any(|warning| warning.code == "root_transform"
                && warning.severity == OutlinerWarningSeverity2D::Warning)
    );
    assert!(
        outliner
            .warnings_for(child.id)
            .iter()
            .any(|warning| warning.code == "visible_child_hidden_parent")
    );
    assert!(
        outliner
            .context_actions_for(root.id)
            .contains(&"show_warnings")
    );

    let mut overlay_view = SceneView2D {
        smart_snap: true,
        show_colliders: true,
        box_selection: Some((4.0, 4.0, 8.0, 8.0)),
        selected_ids: vec![child.id],
        ..SceneView2D::default()
    };
    child.visible = true;
    overlay_view.rebuild_guides_from_entities(&[child.clone()]);
    let overlays = overlay_view.overlay_commands(&[child.clone()]);
    assert!(
        overlays
            .iter()
            .any(|overlay| overlay.kind == SceneOverlayKind2D::SelectionRect)
    );
    assert!(
        overlays
            .iter()
            .any(|overlay| overlay.kind == SceneOverlayKind2D::GuideLine)
    );
    assert!(
        overlays
            .iter()
            .any(|overlay| overlay.kind == SceneOverlayKind2D::ColliderOutline)
    );
    assert!(
        overlays
            .iter()
            .any(|overlay| overlay.kind == SceneOverlayKind2D::Pivot)
    );

    let mut spriteframes =
        SpriteFrames2D::grid_slice("Hero", "assets/hero.png", 3, 1, 16, 16, 10.0);
    assert!(spriteframes.duplicate_animation("default", "attack"));
    assert_eq!(spriteframes.animation_names(), vec!["default", "attack"]);
    let attack_index = spriteframes.animation_index("attack").unwrap();
    spriteframes.animations[attack_index].ping_pong = true;
    assert_eq!(
        spriteframes.set_frame_duration("attack", &[0, 1, 1], 0.2),
        2
    );
    assert!(spriteframes.move_frame("attack", 0, 2));
    assert_eq!(
        spriteframes.frame_at_time("attack", 0.0).unwrap().rect.x,
        16
    );
    assert_eq!(spriteframes.toggle_loop("attack"), Some(false));
    assert!(spriteframes.validate());
}

#[test]
fn massive_world2d_streaming_budgets_pooling_spawns_and_saves() {
    let (mut partition, budget, mut pool) = minimal_massive_world2d();
    for chunk in partition
        .chunks
        .iter_mut()
        .filter(|chunk| chunk.cell_x < -1)
    {
        chunk.loaded = true;
    }
    let plan = partition.streaming_plan(0.0, 0.0);
    assert_eq!(plan.focus_cell, (0, 0));
    assert!(!plan.load.is_empty());
    assert!(plan.unload.iter().any(|action| action.cell_x == -2));
    assert!(partition.validate().is_empty());

    let issues = budget.assess(&RuntimeBudgetStats2D {
        entities: budget.max_entities + 1,
        visible_sprites: budget.max_visible_sprites * 3,
        particles: 100,
        draw_calls: 10,
        loaded_chunks: 5,
        script_ms: budget.max_script_ms + 0.5,
        physics_ms: 1.0,
        ui_ms: 1.0,
        memory_mb: 128.0,
    });
    assert!(issues.iter().any(|issue| issue.metric == "entities"));
    assert!(
        issues
            .iter()
            .any(|issue| issue.metric == "visible_sprites" && issue.severity == "critical")
    );

    let first = pool.acquire("assets/prefabs/projectile.prefab");
    assert!(first.allowed);
    assert!(first.reused);
    assert!(pool.release("assets/prefabs/projectile.prefab"));

    let mut tight_pool = ObjectPool2D::with_bucket("assets/prefabs/bullet.prefab", 0, 1);
    assert!(tight_pool.acquire("assets/prefabs/bullet.prefab").allowed);
    assert!(!tight_pool.acquire("assets/prefabs/bullet.prefab").allowed);

    let mut spawns = SpawnDirector2D {
        max_spawn_per_tick: 2,
        rules: vec![SpawnRule2D {
            prefab: "assets/prefabs/enemy.prefab".to_string(),
            tag: "Enemy".to_string(),
            min_distance_from_camera: 10.0,
            max_distance_from_camera: 20.0,
            max_alive: 5,
            weight: 1.0,
            cooldown_frames: 1,
            last_spawn_frame: 0,
        }],
    };
    let requests = spawns.requests(10, 0.0, 0.0, |_| 0);
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].prefab, "assets/prefabs/enemy.prefab");

    let mut sharding = SaveSharding2D::new(4, "saves/profile/global.json");
    sharding.mark_position_dirty(&WorldPartition2D::default(), 10.0, 10.0);
    sharding.mark_dirty(7, -1);
    let flush = sharding.flush_plan();
    assert_eq!(flush.global_save_path, "saves/profile/global.json");
    assert!(
        flush
            .shard_paths
            .iter()
            .any(|path| path.contains("shard_1_-1"))
    );
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
        "hybrid_3d_rendering",
        "paper2d",
        "animation_blueprint",
        "umg_like_ui",
        "ui_designer",
        "blueprint_library",
        "sequencer2d",
        "physics2d",
        "ai_behavior_tree",
        "rts_tools",
        "massive_world2d",
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
