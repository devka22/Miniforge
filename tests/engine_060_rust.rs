use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::core::engine_config::EngineConfig;
use miniforge::core::game::Game;
use miniforge::engine::advanced_prefabs::AdvancedPrefabSystem;
use miniforge::engine::animation_editor::AnimationEditor;
use miniforge::engine::animation_graph::{
    AnimationClip, AnimationGraphLibrary, AnimationKeyframe, AnimationTransition, AnimatorState,
};
use miniforge::engine::archetype_library::ArchetypeLibrary;
use miniforge::engine::asset_database::AssetDatabase;
use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::audio_mixer::AudioMixer;
use miniforge::engine::build_placement::{BuildFootprint, BuildPlacement};
use miniforge::engine::build_profiles::BuildProfiles;
use miniforge::engine::build_report::BuildReport;
use miniforge::engine::camera::Camera;
use miniforge::engine::component::{component_from_data, default_component};
use miniforge::engine::component_registry::ComponentRegistry;
use miniforge::engine::component_validation::ComponentValidation;
use miniforge::engine::content_drag::DragPayload;
use miniforge::engine::developer_console::{ConsoleSeverity, DeveloperConsole};
use miniforge::engine::diagnostics::{Diagnostics, FrameHealth};
use miniforge::engine::docking_panel::{EditorDockTab, EguiDockingWorkspace};
use miniforge::engine::editor_workspace::{EditorPanelKind, EditorWorkspace, WorkspaceMode};
use miniforge::engine::engine_programming::ProgrammingEnvironment;
use miniforge::engine::event_bus::EventBus;
use miniforge::engine::file_browser::FileBrowser;
use miniforge::engine::game_api::GameAPI;
use miniforge::engine::game_clock::GameClock;
use miniforge::engine::hierarchy_manager::HierarchyManager;
use miniforge::engine::input_map::InputMap;
use miniforge::engine::logger::{LogLevel, Logger};
use miniforge::engine::luau_scripting::LuauScriptRuntime;
use miniforge::engine::material_system::{Material2D, MaterialLibrary, TextureSlot2D};
use miniforge::engine::play_mode_manager::PlayModeManager;
use miniforge::engine::plugin_manager::PluginManager;
use miniforge::engine::prefab_manager::PrefabManager;
use miniforge::engine::prefab_overrides::PrefabOverrides;
use miniforge::engine::profiler::Profiler;
use miniforge::engine::project_launcher::{EguiProjectLauncher, LauncherTemplate};
use miniforge::engine::project_package::ProjectPackageManager;
use miniforge::engine::project_templates::ProjectTemplates;
use miniforge::engine::project_validator::ProjectValidator;
use miniforge::engine::resource_manager::{
    ResourceCacheMode, ResourceKind, ResourceLoadStatus, ResourceManager,
};
use miniforge::engine::runtime_exporter::{ExportProfile, RuntimeExporter};
use miniforge::engine::scene_serializer::SceneSerializer;
use miniforge::engine::scene_view_tools::SceneViewTools;
use miniforge::engine::script_debugger::ScriptDebugger;
use miniforge::engine::script_editor::ScriptEditor;
use miniforge::engine::spatial_index::SpatialIndex;
use miniforge::engine::sprite_editor::{SpriteColor, SpriteEditorCanvas};
use miniforge::engine::tile_brush::{TileBrush, TileBrushMode};
use miniforge::engine::tilemap_layers::TilemapLayers;
use miniforge::engine::ui_canvas::UICanvas;
use miniforge::engine::ui_runtime::{UiEventKind, UiRuntime};
use miniforge::engine::upgrade_manifest::EngineUpgradeManifest;
use miniforge::engine::visual_input_editor::VisualInputEditor;
use miniforge::engine::visual_scripting::VisualScriptRuntime;
use miniforge::engine::xcode_integration::XcodeBuildPlan;
use miniforge::entities::game_object::GameObject;
use miniforge::input::input_handler::InputHandler;
use miniforge::map::flow_field::FlowField;
use miniforge::map::grid::Grid;
use miniforge::map::pathfinding::{influence_map, threat_aware_astar};
use miniforge::systems::animation_system::AnimationSystem;
use miniforge::systems::audio_system::{AudioCommandKind, AudioSystem};
use miniforge::systems::camera_system::CameraSystem;
use miniforge::systems::command_system::CommandSystem;
use miniforge::systems::gameplay_system::GameplaySystem;
use miniforge::systems::particle_system::ParticleSystem;
use miniforge::systems::physics_system::{PairType, PhysicsEventPhase, PhysicsSystem};
use miniforge::systems::rapier_physics_bridge::RapierPhysicsBridge;
use miniforge::systems::render_system::RenderSystem;
use miniforge::systems::rts_system::RTSSystem;
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-rust-{name}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn game_object_roundtrip() {
    let mut obj = GameObject::new(3.0, 4.0, Some("Crate".to_string()));
    obj.command = "PATROL".to_string();
    obj.path = vec![(4.0, 4.0), (5.0, 4.0)];
    obj.patrol_points = vec![(3.0, 4.0), (5.0, 4.0)];
    let data = obj.serialize();
    let clone = GameObject::from_data(&data, true);
    assert_eq!(clone.name, "Crate");
    assert_eq!(clone.x, 3.0);
    assert_eq!(clone.command, "PATROL");
    assert_eq!(clone.path.len(), 2);
    assert_eq!(clone.patrol_points.len(), 2);
    assert_eq!(
        clone.get_component("Transform").unwrap().get_f64("x", 0.0),
        3.0
    );
}

#[test]
fn input_map_persists_bindings() {
    let tmp = temp_dir("input");
    let path = tmp.join("input_map.json");
    let mut input_map = InputMap::new(&path).unwrap();
    input_map
        .set_binding("dash", vec!["left_shift".to_string()])
        .unwrap();
    let loaded = InputMap::new(&path).unwrap();
    assert_eq!(loaded.bindings["dash"], vec!["left_shift"]);
}

#[test]
fn build_profiles_cycle() {
    let tmp = temp_dir("profiles");
    let path = tmp.join("build_profiles.json");
    let mut profiles = BuildProfiles::new(&path).unwrap();
    let first = profiles.active.clone();
    let second = profiles.cycle().unwrap();
    assert_ne!(first, second);
    assert!(
        profiles
            .xcode_profiles()
            .contains(&"AppleXcodeDebug".to_string())
    );
}

#[test]
fn hierarchy_parenting() {
    let parent = GameObject::new(10.0, 10.0, Some("Parent".to_string()));
    let mut child = GameObject::new(12.0, 13.0, Some("Child".to_string()));
    HierarchyManager::set_parent(&mut child, &parent);
    assert_eq!(child.parent_id, Some(parent.id));
    assert_eq!(child.local_x, 2.0);
    assert_eq!(child.local_y, 3.0);
}

#[test]
fn prefab_override_diff() {
    let diff = PrefabOverrides::diff_dict("", &json!({"name": "A"}), &json!({"name": "B"}));
    assert_eq!(diff[0]["path"], "name");
}

#[test]
fn script_editor_validates_syntax() {
    let mut editor = ScriptEditor {
        lines: vec!["{\"nodes\": []}".to_string()],
        ..Default::default()
    };
    assert!(!editor.validate());
    assert!(editor.document.syntax_error.is_some());
}

#[test]
fn script_editor_tabs_can_close_and_cycle_between_files() {
    let tmp = temp_dir("script-tabs");
    let first = tmp.join("First.luau");
    let second = tmp.join("Second.luau");
    fs::write(&first, "fn on_start() {}").unwrap();
    fs::write(&second, "fn on_update(dt) {}").unwrap();

    let mut editor = ScriptEditor::default();
    editor.open(first.clone()).unwrap();
    editor.open(second.clone()).unwrap();
    assert_eq!(editor.tabs.len(), 2);
    editor.activate_next_tab(-1).unwrap();
    assert_eq!(editor.document.path, Some(first.clone()));
    let active = editor.close_current_tab().unwrap().unwrap();
    assert_eq!(active, second);
    assert_eq!(editor.tabs.len(), 1);
    assert_eq!(editor.closed_tabs.len(), 1);
}

#[test]
fn visual_input_editor_adds_binding() {
    let tmp = temp_dir("visual-input");
    let mut input = InputMap::new(tmp.join("input_map.json")).unwrap();
    input
        .set_binding("jump", vec!["space".to_string()])
        .unwrap();
    let mut editor = VisualInputEditor::default();
    editor.select("jump");
    editor.add_binding(&mut input, "j").unwrap();
    assert!(input.bindings["jump"].contains(&"j".to_string()));
}

#[test]
fn editor_workspace_modes_surface_useful_panels() {
    let mut workspace = EditorWorkspace::default();
    assert!(
        workspace
            .visible_panels()
            .iter()
            .any(|panel| panel.kind == EditorPanelKind::Hierarchy)
    );
    workspace.apply_mode(WorkspaceMode::Scripting);
    assert_eq!(workspace.focused_panel, EditorPanelKind::Programming);
    assert!(
        workspace
            .visible_panels()
            .iter()
            .any(|panel| panel.kind == EditorPanelKind::AssetGraph)
    );
    assert_eq!(workspace.performance_status(8.0), "Realtime");
}

#[test]
fn godot_core_patterns_improve_registry_resource_loader_signals_and_xcode() {
    let registry = ComponentRegistry::new();
    let sprite = registry.descriptor("SpriteRenderer").unwrap();
    assert_eq!(sprite.category, "Rendering");
    assert!(sprite.creatable);
    assert!(
        registry
            .submenu_model()
            .iter()
            .any(|submenu| submenu.category == "Physics"
                && submenu.component_types.contains(&"Collider2D".to_string()))
    );
    assert!(
        registry
            .search("render")
            .iter()
            .any(|descriptor| descriptor.name == "SpriteRenderer")
    );

    let tmp = temp_dir("resource-loader");
    fs::create_dir_all(tmp.join("sprites")).unwrap();
    fs::write(tmp.join("sprites/hero.png"), [0_u8, 1, 2]).unwrap();
    let mut resources = ResourceManager::new(&tmp);
    resources.scan_all().unwrap();
    let request = resources
        .request_load_by_kind(ResourceKind::Image, "hero", ResourceCacheMode::Reuse)
        .unwrap();
    let processed = resources.process_load_queue();
    assert_eq!(
        resources.load_status(&request.id),
        Some(ResourceLoadStatus::Loaded)
    );
    assert_eq!(processed[0].message, "loaded");
    assert_eq!(
        resources.cached_entries_by_kind(ResourceKind::Image).len(),
        1
    );

    let mut bus = EventBus::default();
    bus.connect("ResourceLoaded", "AssetService");
    bus.emit_signal(
        "ResourceLoaded",
        "ResourceManager",
        json!({"id": request.id}),
    );
    assert_eq!(bus.subscribers_for("ResourceLoaded"), vec!["AssetService"]);
    assert_eq!(bus.count("ResourceLoaded"), 1);

    let xcode = XcodeBuildPlan::macos_debug(&tmp, "MiniForgeGame");
    assert!(xcode.validate().is_empty());
    let args = xcode.command_args();
    assert!(args.contains(&"-workspace".to_string()));
    assert!(args.contains(&"build".to_string()));
    assert_eq!(xcode.open_command()[0], "open");
}

#[test]
fn egui_docking_workspace_tracks_professional_editor_windows() {
    let mut docking = EguiDockingWorkspace::new();
    docking.open_tab(EditorDockTab::ScriptEditor);
    docking.open_tab(EditorDockTab::BlueprintEditor);
    docking.set_floating_visibility("script_editor", true);
    assert!(
        docking
            .dock_state
            .iter_all_tabs()
            .any(|(_, tab)| *tab == EditorDockTab::ScriptEditor)
    );
    assert!(docking.floating_panel("script_editor").is_some());
    assert!(docking.dock_summary().contains("egui_dock"));
}

#[test]
fn programming_environment_creates_graph_assets_and_attaches_without_engine_source_edits() {
    let tmp = temp_dir("programming");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut programming = ProgrammingEnvironment::new();
    let path = programming
        .create_graph_asset(&tmp, "HealthPickup", None)
        .unwrap();
    assert_eq!(
        path.extension().and_then(|value| value.to_str()),
        Some("mfgraph")
    );

    let mut entity = GameObject::new(0.0, 0.0, Some("Pickup".to_string()));
    let graph = programming.attach_template_to_entity(&mut entity, "HealthPickup");
    assert_eq!(graph, "HealthPickup");
    assert!(entity.get_component("VisualScript").is_some());
    assert_eq!(programming.compile_count, 1);
}

#[test]
fn visual_graph_view_supports_node_layout_and_pin_connections() {
    let programming = ProgrammingEnvironment::new();
    let mut graph = programming.template_graph("LogAndMove");
    let view = programming.graph_view(&graph);
    assert!(view.nodes.iter().any(|node| node.id == "start"));
    assert!(view.connections.iter().any(|link| link.from == "start"));

    assert!(ProgrammingEnvironment::move_graph_node(
        &mut graph, "move", 320.0, 180.0
    ));
    assert!(ProgrammingEnvironment::connect_graph_nodes(
        &mut graph, "update", "move"
    ));
    let view = programming.graph_view(&graph);
    let move_node = view.nodes.iter().find(|node| node.id == "move").unwrap();
    assert_eq!((move_node.x, move_node.y), (320.0, 180.0));
    assert!(
        view.connections
            .iter()
            .any(|link| link.from == "update" && link.to == "move")
    );
}

#[test]
fn productivity_blueprint_catalog_search_and_branch_pins_work() {
    let programming = ProgrammingEnvironment::new();
    let catalog = programming.catalog_summary();
    assert!(catalog.node_count >= 40);
    assert!(catalog.categories.iter().any(|category| category == "Flow"));
    assert!(catalog.quick_action_count >= 4);
    assert!(
        programming
            .search_node_catalog("health")
            .iter()
            .any(|node| node.node_type == "SetHealth")
    );
    assert!(
        programming
            .search_templates("player")
            .iter()
            .any(|template| template.name == "PlayerVitalMovement")
    );

    let mut graph = programming.template_graph("HealthCombat");
    assert!(ProgrammingEnvironment::add_graph_node(&mut graph, "DestroySelf").is_some());
    assert!(ProgrammingEnvironment::connect_graph_nodes_on_pin(
        &mut graph,
        "low_check",
        "heal",
        "true"
    ));
    let view = programming.graph_view(&graph);
    let branch = view
        .nodes
        .iter()
        .find(|node| node.id == "low_check")
        .unwrap();
    assert_eq!(branch.output_pins, vec!["true", "false"]);
    assert!(
        view.connections
            .iter()
            .any(|connection| connection.from == "low_check" && connection.pin == "true")
    );

    let action = programming
        .quick_actions()
        .into_iter()
        .find(|action| action.label == "Health branch")
        .unwrap();
    let added = ProgrammingEnvironment::add_quick_action_to_graph(&mut graph, &action);
    assert!(added.len() >= 3);
    let view = programming.graph_view(&graph);
    assert!(
        view.connections
            .iter()
            .any(|connection| added.contains(&connection.from))
    );
}

#[test]
fn rust_game_indexes_visual_graph_assets() {
    let tmp = temp_dir("graph-index");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    let graph_path = game.create_program_asset("LogAndMove").unwrap();
    let alias_path = AssetTools::create_script(&tmp, "AliasGraph").unwrap();
    game.refresh_assets().unwrap();
    assert!(game.asset_database.assets.values().any(|asset| {
        asset.asset_type == "VisualGraph"
            && graph_path.to_string_lossy().ends_with(&asset.relative_path)
    }));
    assert_eq!(
        alias_path.extension().and_then(|value| value.to_str()),
        Some("mfgraph")
    );
    assert!(game.visual_graph_asset_count() >= 2);
    let manifest = game.build_manifest().unwrap();
    assert!(
        manifest["scripts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry.as_str().unwrap_or_default().ends_with(".mfgraph"))
    );
    assert!(manifest.get("legacy_runtime_scripts").is_none());
}

#[test]
fn rigidbody_roundtrip_and_force() {
    let mut body = default_component("Rigidbody2D").unwrap();
    body.add_force(4.0, 2.0, true);
    let clone = component_from_data(&body.serialize()).unwrap();
    assert_eq!(clone.component_type, "Rigidbody2D");
    assert_eq!(clone.get_f64("velocity_x", 0.0), 4.0);
    assert_eq!(clone.get_f64("velocity_y", 0.0), 2.0);
}

#[test]
fn tilemap_layers_paint_and_serialize() {
    let mut tilemap = TilemapLayers::new(8, 8);
    tilemap.set_tile(2, 3, 4);
    tilemap.cycle_layer();
    tilemap.fill_active(0, 0, 2, 2, 1);
    let mut clone = TilemapLayers::new(1, 1);
    clone.deserialize(&tilemap.serialize());
    assert_eq!(clone.layer("Ground").unwrap().get(2, 3), 4);
    assert_eq!(clone.stats()["layers"], 4);
}

#[test]
fn audio_mixer_serializes_buses() {
    let mut mixer = AudioMixer::new();
    mixer.set_bus_volume("SFX", 0.25);
    let data = mixer.serialize();
    let mut clone = AudioMixer::new();
    clone.deserialize(&data);
    assert_eq!(clone.buses["SFX"].volume, 0.25);
}

#[test]
fn physics_integrates_rigidbody() {
    let mut entity = GameObject::new(0.0, 0.0, Some("Body".to_string()));
    let mut body = default_component("Rigidbody2D").unwrap();
    body.set("use_gravity", json!(false));
    body.set_f64("velocity_x", 10.0);
    entity.add_component(body);
    let mut entities = vec![entity];
    PhysicsSystem::new().update_entities(&mut entities, 0.1, "PLAY");
    assert!(entities[0].x > 0.0);
}

#[test]
fn animation_system_applies_tint() {
    let mut entity = GameObject::new(0.0, 0.0, Some("Animated".to_string()));
    entity.add_component(default_component("Animator").unwrap());
    let mut entities = vec![entity];
    AnimationSystem.update_entities(&mut entities, &AnimationGraphLibrary::new(), 0.6, "PLAY");
    assert_ne!(
        entities[0]
            .get_component("SpriteRenderer")
            .unwrap()
            .get("tint")
            .unwrap(),
        &json!([255, 255, 255])
    );
}

#[test]
fn ui_canvas_hit_test() {
    let mut entity = GameObject::new(0.0, 0.0, Some("Button".to_string()));
    let mut ui = default_component("UIElement").unwrap();
    ui.set("element_type", json!("Button"));
    ui.set_f64("x", 10.0);
    ui.set_f64("y", 10.0);
    ui.set_f64("width", 100.0);
    ui.set_f64("height", 40.0);
    entity.add_component(ui);
    let entities = vec![entity];
    let hit = UICanvas::default().hit_test(&entities, (20.0, 20.0));
    assert_eq!(hit.unwrap().0.name, "Button");
}

#[test]
fn visual_scripting_moves_entity() {
    let mut entity = GameObject::new(0.0, 0.0, Some("Scripted".to_string()));
    let mut script = default_component("VisualScript").unwrap();
    script.set(
        "nodes",
        json!([
            {"id": "start", "type": "EventStart", "next": "move"},
            {"id": "move", "type": "Move", "x": 3, "y": 4, "next": null}
        ]),
    );
    entity.add_component(script);
    let mut entities = vec![entity];
    VisualScriptRuntime::default().update_entities(&mut entities, 0.016, "PLAY");
    assert_eq!(entities[0].x, 3.0);
    assert_eq!(entities[0].y, 4.0);
}

#[test]
fn visual_scripting_productivity_nodes_drive_gameplay_components() {
    let mut entity = GameObject::new(0.0, 0.0, Some("BlueprintHero".to_string()));
    let mut script = default_component("VisualScript").unwrap();
    script.set(
        "nodes",
        json!([
            {"id": "start", "type": "EventStart", "next": "health"},
            {"id": "health", "type": "SetHealth", "health": 40.0, "max_health": 50.0, "next": "damage"},
            {"id": "damage", "type": "Damage", "amount": 25.0, "next": "branch"},
            {"id": "branch", "type": "BranchHealth", "operator": "<=", "value": 20.0, "true_next": "heal", "false_next": "velocity"},
            {"id": "heal", "type": "Heal", "amount": 20.0, "next": "velocity"},
            {"id": "velocity", "type": "SetVelocity", "x": 3.0, "y": -1.0, "next": "blackboard"},
            {"id": "blackboard", "type": "SetBlackboard", "key": "state", "value": "Ready", "next": null}
        ]),
    );
    entity.add_component(script);
    let mut entities = vec![entity];
    let mut runtime = VisualScriptRuntime::default();
    runtime.update_entities(&mut entities, 0.016, "PLAY");
    let entity = &entities[0];
    assert_eq!(
        entity
            .get_component("Health")
            .unwrap()
            .get_f64("health", 0.0),
        35.0
    );
    assert_eq!(
        entity
            .get_component("Rigidbody2D")
            .unwrap()
            .get_f64("velocity_x", 0.0),
        3.0
    );
    assert_eq!(
        entity
            .get_component("Blackboard")
            .unwrap()
            .blackboard_get("state", json!("")),
        json!("Ready")
    );
}

#[test]
fn visual_scripting_wait_node_pauses_chain_until_timer_finishes() {
    let mut entity = GameObject::new(0.0, 0.0, Some("TimerGraph".to_string()));
    let mut script = default_component("VisualScript").unwrap();
    script.set(
        "nodes",
        json!([
            {"id": "start", "type": "EventStart", "next": "wait"},
            {"id": "wait", "type": "Wait", "seconds": 0.1, "next": "log"},
            {"id": "log", "type": "Log", "message": "done", "next": null}
        ]),
    );
    entity.add_component(script);
    let mut entities = vec![entity];
    let mut runtime = VisualScriptRuntime::default();
    runtime.update_entities(&mut entities, 0.02, "PLAY");
    assert!(runtime.logs.is_empty());
    runtime.update_entities(&mut entities, 0.12, "PLAY");
    assert_eq!(runtime.logs, vec!["done".to_string()]);
}

#[test]
fn luau_scripts_drive_gameplay_events_and_hot_reload() {
    let tmp = temp_dir("luau-runtime");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let script = AssetTools::create_luau_script(&tmp, "PlayerController").unwrap();
    fs::write(
        &script,
        r#"
function on_start()
    set_position(2.0, 3.0)
    ui_text("ready")
    set_blackboard("mood", "hope")
    add_quest("letters", "Letters", "meet", "Meet Mara", 1)
    quest_progress("letters", "meet", 1)
    recharge_ability(1)
    trigger_ability()
end

function on_update(dt: number)
    if input_pressed("D") then move(10.0 * dt, 0.0) end
end

function on_key_down(key: string)
    if key == "Space" then
        spawn("Bullet", 7.0, 8.0)
        play_sound("jump")
        load_scene("Arena.scene")
    end
end

function on_collision_enter(other: string)
    set_ui_text("Hud", "hit " .. other)
end

function on_destroy()
    spawn("Poof", 1.0, 1.0)
end
"#,
    )
    .unwrap();

    let mut player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    player.script = Some("PlayerController.luau".to_string());
    let player_id = player.id;
    let mut hud = GameObject::new(0.0, 0.0, Some("Hud".to_string()));
    hud.add_component(default_component("UIElement").unwrap());
    let mut entities = vec![player, hud];

    let mut runtime = LuauScriptRuntime::new(&tmp);
    runtime.set_input_pressed("D", true);
    let update_report = runtime.update_entities(&mut entities, 0.5, "PLAY");
    assert!(
        update_report.errors.is_empty(),
        "{:?}",
        update_report.errors
    );
    assert_eq!(entities[0].x, 7.0);
    assert_eq!(entities[0].y, 3.0);
    assert_eq!(
        entities[0]
            .get_component("UIElement")
            .unwrap()
            .get("text")
            .and_then(|value| value.as_str()),
        Some("ready")
    );
    assert_eq!(
        GameAPI::get_blackboard(&entities[0], "mood", json!("")).as_str(),
        Some("hope")
    );
    assert!(entities[0].get_component("QuestLog").is_some());
    assert!(entities[0].get_component("Ability").is_some());

    let key_report = runtime.run_key_down(&mut entities, "Space");
    assert_eq!(key_report.spawned.len(), 1);
    assert!(key_report.sounds.contains(&"jump".to_string()));
    assert!(
        key_report
            .scene_requests
            .contains(&"Arena.scene".to_string())
    );
    assert!(entities.iter().any(|entity| entity.name == "Bullet"));
    assert!(
        entities
            .iter()
            .any(|entity| entity.name == "Audio_jump"
                && entity.get_component("AudioSource").is_some())
    );

    let collision_report = runtime.run_collision_enter(&mut entities, player_id, "Wall");
    assert_eq!(collision_report.ui_updates, 1);
    let hud = entities.iter().find(|entity| entity.name == "Hud").unwrap();
    assert_eq!(
        hud.get_component("UIElement")
            .unwrap()
            .get("text")
            .and_then(|value| value.as_str()),
        Some("hit Wall")
    );

    let destroy_report = runtime.run_destroy(&mut entities, player_id);
    assert!(destroy_report.destroyed.contains(&player_id));
    assert!(!entities.iter().any(|entity| entity.id == player_id));
    assert!(entities.iter().any(|entity| entity.name == "Poof"));

    runtime.mark_script_changed(&script);
    assert_eq!(runtime.poll_hot_reload(), 1);
    assert_eq!(runtime.reload_count, 1);
}

#[test]
fn game_loop_runs_luau_and_records_profiler_counters() {
    let tmp = temp_dir("game-luau");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(
        tmp.join("scripts").join("Mover.luau"),
        r#"function on_update(dt: number) if input_pressed("D") then move(4.0 * dt, 0.0) end end"#,
    )
    .unwrap();
    let mut game = Game::from_project(&tmp, true).unwrap();
    let mut entity = GameObject::new(0.0, 0.0, Some("Mover".to_string()));
    entity.script = Some("Mover.luau".to_string());
    game.units = vec![entity];
    game.set_script_input_pressed("D", true);
    game.run_headless_once(0.25);
    assert_eq!(game.units[0].x, 1.0);
    assert!(
        game.profiler
            .counters
            .get("LuauScripts")
            .copied()
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn production_polish_animation_particles_materials_ui_and_debugger() {
    let mut library = AnimationGraphLibrary::new();
    let controller = library.controllers.get_mut("Default").unwrap();
    controller.clips.insert(
        "Run".to_string(),
        AnimationClip {
            name: "Run".to_string(),
            duration: 0.5,
            tint_from: (255, 255, 255),
            tint_to: (255, 120, 90),
            keyframes: vec![
                AnimationKeyframe {
                    time: 0.0,
                    sprite_name: Some("run_0".to_string()),
                    x: Some(0.0),
                    y: Some(0.0),
                    rotation: Some(0.0),
                    scale_x: Some(1.0),
                    scale_y: Some(1.0),
                    tint: Some((255, 255, 255)),
                },
                AnimationKeyframe {
                    time: 0.5,
                    sprite_name: Some("run_1".to_string()),
                    x: Some(2.0),
                    y: Some(0.0),
                    rotation: Some(15.0),
                    scale_x: Some(1.2),
                    scale_y: Some(1.0),
                    tint: Some((255, 120, 90)),
                },
            ],
            events: Vec::new(),
        },
    );
    controller.states.insert(
        "Run".to_string(),
        AnimatorState {
            name: "Run".to_string(),
            clip: "Run".to_string(),
            speed: 1.0,
            looped: true,
            preview_time: 0.0,
        },
    );
    controller.add_transition(AnimationTransition {
        from: "Idle".to_string(),
        to: "Run".to_string(),
        parameter: Some("moving".to_string()),
        equals: json!(true),
        exit_time: Some(0.0),
        duration: 0.1,
    });

    let mut editor = AnimationEditor::new(controller.clone());
    assert!(editor.select_state("Run"));
    editor.timeline.playing = true;
    let preview = editor.tick(0.25);
    assert_eq!(preview.clip, "Run");
    assert!(preview.sample.x.unwrap() > 0.0);
    assert!(editor.validate());

    let mut animated = GameObject::new(0.0, 0.0, Some("Animated".to_string()));
    let mut animator = default_component("Animator").unwrap();
    animator.set("parameters", json!({"moving": true}));
    animated.add_component(animator);
    let mut entities = vec![animated];
    AnimationSystem.update_entities(&mut entities, &library, 0.016, "PLAY");
    assert_eq!(
        entities[0]
            .get_component("Animator")
            .unwrap()
            .get_string("current_state", ""),
        "Run"
    );

    entities[0].add_component(default_component("ParticleEmitter").unwrap());
    let mut particles = ParticleSystem::default();
    particles.update_entities(&entities, 0.25, "PLAY");
    assert!(particles.preview(entities[0].id).unwrap().particle_count > 0);

    let mut materials = MaterialLibrary::default();
    materials.upsert_material(Material2D::lit_fog("LitFog"));
    let material_preview = materials.apply_to_entity(&mut entities[0], "LitFog");
    assert_eq!(material_preview.shader, "sprite_lit_fog");
    assert!(material_preview.warnings.is_empty());
    let material_preview = materials.assign_texture_to_material(
        "LitFog",
        TextureSlot2D::Normal,
        "assets/textures/hero_normal.png",
    );
    assert_eq!(
        material_preview
            .texture_slots
            .get("normal")
            .map(String::as_str),
        Some("assets/textures/hero_normal.png")
    );

    let mut ui_runtime = UiRuntime::default();
    let canvas = miniforge::engine::ui_canvas::UiCanvasRoot::default_hud();
    let events =
        ui_runtime.update_canvas_interaction(&canvas, (1920.0, 1080.0), Some((24.0, 24.0)), true);
    assert!(events.iter().all(|event| event.kind != UiEventKind::Click));

    let mut button = GameObject::new(0.0, 0.0, Some("Button".to_string()));
    let mut ui = default_component("UIElement").unwrap();
    ui.set("element_type", json!("Button"));
    ui.set("interactable", json!(true));
    ui.set("on_click", json!("start_game"));
    ui.set_f64("x", 0.0);
    ui.set_f64("y", 0.0);
    ui.set_f64("width", 100.0);
    ui.set_f64("height", 40.0);
    button.add_component(ui);
    let mut ui_entities = vec![button];
    let events = ui_runtime.update_entity_interaction(&mut ui_entities, (10.0, 10.0), true);
    assert!(events.iter().any(|event| event.kind == UiEventKind::Click));
}

#[test]
fn production_polish_launcher_export_autosave_debugger_and_demo() {
    let tmp = temp_dir("polish-demo");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let created = ProjectTemplates::create(&tmp, "complete_demo").unwrap();
    assert!(
        created
            .iter()
            .any(|path| path.to_string_lossy().contains("Demo_Game"))
    );
    assert!(tmp.join("scripts").join("DemoPlayer.luau").exists());
    assert!(
        tmp.join("assets")
            .join("data")
            .join("ImpactSparks.particles.json")
            .exists()
    );

    let mut game = Game::from_project(&tmp, true).unwrap();
    game.load_scene("Demo_Game.scene").unwrap();
    assert!(
        game.units
            .iter()
            .any(|entity| entity.get_component("ParticleEmitter").is_some())
    );
    game.run_headless_once(1.0 / 60.0);
    assert!(game.profiler.counters.contains_key("Particles"));
    assert!(
        game.script_debugger
            .active_scripts
            .iter()
            .any(|script| script.ends_with("DemoPlayer.luau"))
    );

    let mut debugger = ScriptDebugger::default();
    debugger.refresh(&game.luau_script_runtime, &tmp, &game.units);
    assert!(!debugger.active_scripts.is_empty());
    assert!(!debugger.has_errors());

    let mut launcher = EguiProjectLauncher::new(temp_dir("launcher-polish"));
    launcher.settings.validate_on_open = true;
    let repair_notes = launcher.repair_project(&tmp).unwrap();
    assert!(!repair_notes.is_empty());
    let report =
        RuntimeExporter::export_with_profile(&tmp, tmp.join("exports"), ExportProfile::Release)
            .unwrap();
    assert!(report.release_optimized);
    assert!(
        report.validation_errors.is_empty(),
        "{:?}",
        report.validation_errors
    );

    assert!(matches!(game.autosave_manager.health(), "empty" | "ready"));
    game.autosave_manager
        .save(&mut game.runtime_world.units)
        .unwrap();
    assert_eq!(game.autosave_manager.health(), "ready");
    assert!(!game.autosave_manager.recover_entities().unwrap().is_empty());

    game.audio_mixer.set_bus_fade("Music", 0.25, 0.5);
    game.audio_mixer.tick_fades(0.5);
    assert!(game.audio_mixer.buses["Music"].volume <= 1.0);
    game.diagnostics.push_warning("asset fallback used");
    assert!(game.diagnostics.stability_score() <= 1.0);
}

#[test]
fn advanced_prefab_variant_diff() {
    let tmp = temp_dir("prefabs");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut entity = GameObject::new(0.0, 0.0, Some("VariantSource".to_string()));
    let mut system = AdvancedPrefabSystem::default();
    let prefab = system
        .create_prefab_from_entity(&tmp, &mut entity, false, vec![])
        .unwrap();
    assert!(prefab.exists());
    assert!(entity.is_prefab_instance);
    let path = system
        .create_variant_from_entity(&tmp, &mut entity)
        .unwrap();
    assert!(path.exists());
    assert_eq!(system.variants_created, 1);
    let report = system.analyze_instance(&entity, None);
    assert!(report.component_count >= 1);
}

#[test]
fn profiler_records_scheduler_time() {
    let mut profiler = Profiler::new();
    profiler.record_system("SystemA", 1.5);
    profiler.begin_frame();
    profiler.end_frame();
    assert!(
        profiler
            .rows()
            .contains(&("SystemA".to_string(), "1.5 ms".to_string()))
    );
}

#[test]
fn diagnostics_and_event_bus_track_runtime_health() {
    let mut diagnostics = Diagnostics::default();
    diagnostics.update_with_budget(0.016, 8.33);
    diagnostics.update_with_budget(0.032, 8.33);
    assert_eq!(diagnostics.frames, 2);
    assert_eq!(diagnostics.over_budget_frames, 2);
    assert_eq!(diagnostics.dropped_frames, 1);
    assert!(diagnostics.average_frame_time_ms > diagnostics.min_frame_time_ms);
    assert!(diagnostics.max_frame_time_ms >= diagnostics.average_frame_time_ms);

    let mut clock = GameClock::new(1.0 / 60.0);
    clock.configure_fixed_step(1.0 / 60.0, 2);
    let advance = clock.advance(0.25);
    diagnostics.record_frame_runtime(
        advance,
        clock.fixed_delta,
        42,
        12.0,
        Some(("Physics".to_string(), 9.5)),
    );
    assert_eq!(diagnostics.last_frame.health, FrameHealth::Saturated);
    assert_eq!(diagnostics.last_frame.entity_count, 42);
    assert_eq!(
        diagnostics.last_frame.slowest_system.as_deref(),
        Some("Physics")
    );
    assert!(
        diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("fisica/scripts"))
    );
    assert!(
        diagnostics
            .health_summary()
            .contains("fixed step saturated")
    );

    let mut events = EventBus::default();
    events.emit("unit_spawned", json!({"id": 1}));
    events.emit("unit_spawned", json!({"id": 2}));
    events.emit("resource_changed", json!({"Gold": 50}));
    assert_eq!(events.count("unit_spawned"), 2);
    assert_eq!(events.drain_named("unit_spawned").len(), 2);
    assert_eq!(events.drain().len(), 1);
}

#[test]
fn component_validation_repairs_ranges() {
    let mut audio = component_from_data(&json!({
        "component_type": "AudioSource",
        "volume": 5,
        "spatial_blend": -1
    }))
    .unwrap();
    assert!(ComponentValidation::repair_component(&mut audio));
    assert_eq!(audio.get_f64("volume", 0.0), 1.0);
    assert_eq!(audio.get_f64("spatial_blend", 1.0), 0.0);

    let mut site = component_from_data(&json!({
        "component_type": "ConstructionSite",
        "build_time": -3.0,
        "build_rate": -1.0,
        "progress": 99.0
    }))
    .unwrap();
    assert!(ComponentValidation::repair_component(&mut site));
    assert!(site.get_f64("build_time", 0.0) > 0.0);
    assert_eq!(site.get_f64("build_rate", 1.0), 0.0);
}

#[test]
fn scene_serializer_migrates_old_data() {
    let data = SceneSerializer::migrate(json!({"objects": [{"name": "Old"}]}));
    assert_eq!(data["version"], miniforge::ENGINE_VERSION);
    assert_eq!(data["entities"][0]["name"], "Old");
    assert_eq!(data["brush_size"], 1);
}

#[test]
fn resource_manager_recursive_scan() {
    let tmp = temp_dir("resources");
    let sprites = tmp.join("sprites").join("nested");
    fs::create_dir_all(&sprites).unwrap();
    fs::write(sprites.join("hero.png"), b"not-a-real-image").unwrap();
    let mut manager = ResourceManager::new(&tmp);
    manager.scan_sprites().unwrap();
    assert_eq!(
        manager.images["hero"],
        PathBuf::from("sprites/nested/hero.png")
    );
}

#[test]
fn upgrade_manifest_tracks_more_than_100_improvements() {
    let summary = EngineUpgradeManifest::new().summary();
    assert!(summary["count"].as_u64().unwrap() >= 100);
    assert!(summary["advanced_components"].as_u64().unwrap() >= 30);
}

#[test]
fn advanced_components_roundtrip_inventory_and_ai() {
    let mut inventory = component_from_data(&json!({
        "component_type": "Inventory",
        "capacity": 2,
        "items": []
    }))
    .unwrap();
    let added = inventory.inventory_add_item("potion", 3, json!({}));
    let clone = component_from_data(&inventory.serialize()).unwrap();
    let ai = component_from_data(&json!({
        "component_type": "AIController",
        "behavior": "attack",
        "target_tags": ["Player"]
    }))
    .unwrap();
    assert_eq!(added, 3);
    assert!(clone.inventory_has_item("potion", 3));
    assert_eq!(ai.get_string("behavior", ""), "attack");
    assert_eq!(ai.get_string_list("target_tags"), vec!["Player"]);
}

#[test]
fn gameplay_system_lifetime_destroys_entity() {
    let mut entity = GameObject::new(0.0, 0.0, Some("Temp".to_string()));
    let mut lifetime = default_component("Lifetime").unwrap();
    lifetime.set_f64("duration", 0.01);
    entity.add_component(lifetime);
    let mut entities = vec![entity];
    GameplaySystem::default().update_entities(&mut entities, 0.02, "PLAY");
    assert!(entities.is_empty());
}

#[test]
fn gameplay_ai_damages_target() {
    let mut attacker = GameObject::new(0.0, 0.0, Some("Attacker".to_string()));
    let mut ai = default_component("AIController").unwrap();
    ai.set("behavior", json!("attack"));
    ai.set("target_tags", json!(["Enemy"]));
    ai.set_f64("attack_radius", 2.0);
    attacker.add_component(ai);
    let mut damage = default_component("DamageDealer").unwrap();
    damage.set_f64("damage", 12.0);
    damage.set_f64("cooldown", 0.0);
    damage.set("target_tags", json!(["Enemy"]));
    attacker.add_component(damage);

    let mut target = GameObject::new(1.0, 0.0, Some("Target".to_string()));
    target.tag = "Enemy".to_string();
    let mut health = default_component("Health").unwrap();
    health.set_f64("max_health", 50.0);
    health.set_f64("health", 50.0);
    target.add_component(health);

    let mut entities = vec![attacker, target];
    GameplaySystem::default().update_entities(&mut entities, 0.05, "PLAY");
    assert!(
        entities[1]
            .get_component("Health")
            .unwrap()
            .get_f64("health", 50.0)
            < 50.0
    );
}

#[test]
fn game_api_inventory_cooldown_and_tween_helpers() {
    let mut entity = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    GameAPI::add_item(&mut entity, "key", 2);
    GameAPI::start_cooldown(&mut entity, "dash", 1.0);
    GameAPI::tween(&mut entity, "x", 10.0, 0.5);
    assert_eq!(GameAPI::item_count(&entity, "key"), 2);
    assert!(!GameAPI::cooldown_ready(&entity, "dash"));
    assert!(
        entity
            .get_component("Tween")
            .unwrap()
            .get_bool("active", false)
    );
}

#[test]
fn beta_entity_serializes_standard_fields() {
    let mut entity = GameObject::new(5.0, 7.0, Some("Player".to_string()));
    entity.rotation = 30.0;
    entity.scale_x = 2.0;
    entity.scale_y = 3.0;
    entity.width = 32.0;
    entity.height = 48.0;
    entity.script = Some("player.mfgraph".to_string());
    entity.sync_to_components();
    let data = entity.serialize();
    assert_eq!(data["position"], json!([5.0, 7.0]));
    assert_eq!(data["scale"], json!([2.0, 3.0]));
    assert_eq!(data["size"], json!([32.0, 48.0]));
    assert_eq!(data["script"], "player.mfgraph");
    assert_eq!(data["active"], true);
}

#[test]
fn beta_scene_template_has_required_json_shape() {
    let data = AssetTools::template_scene("main_scene");
    assert_eq!(data["scene_name"], "main_scene");
    assert_eq!(data["engine_version"], miniforge::ENGINE_VERSION);
    assert!(data.get("entities").is_some());
    assert!(data.get("tiles").is_some());
    assert!(data.get("camera").is_some());
    assert!(data.get("settings").is_some());
}

#[test]
fn beta_project_files_include_engine_config() {
    let tmp = temp_dir("project-files");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    assert!(tmp.join("project.json").exists());
    assert!(tmp.join("engine_config.json").exists());
    assert!(tmp.join("logs").exists());
    assert!(tmp.join("saves").join("scenes").exists());
}

#[test]
fn beta_play_mode_restores_snapshot() {
    let entity = GameObject::new(1.0, 1.0, Some("Player".to_string()));
    let mut entities = vec![entity];
    let mut mode = "EDITOR".to_string();
    let mut manager = PlayModeManager::default();
    manager.enter_play_mode(&entities, &mut mode);
    entities[0].x = 99.0;
    manager.tick_frame();
    manager.exit_play_mode(&mut entities, &mut mode, "test");
    assert_eq!(mode, "EDITOR");
    assert_eq!(entities[0].x, 1.0);
    assert_eq!(manager.frame_count, 0);
    assert_eq!(manager.last_session_frames, 1);
    assert_eq!(manager.last_session_entity_count, 1);
    assert_eq!(manager.last_exit_reason, "test");
}

#[test]
fn asset_database_import_settings_and_dependencies() {
    let tmp = temp_dir("asset-db");
    let assets = tmp.join("assets").join("data");
    let scenes = tmp.join("saves").join("scenes");
    fs::create_dir_all(&assets).unwrap();
    fs::create_dir_all(&scenes).unwrap();
    fs::write(assets.join("Items.json"), "{}").unwrap();
    fs::write(scenes.join("main.scene"), "{\"uses\": \"Items\"}").unwrap();
    let mut database = AssetDatabase::new(tmp.join("assets"), &tmp).unwrap();
    database
        .set_import_setting("assets/data/Items.json", "include_in_build", json!(false))
        .unwrap();
    let graph = database.rebuild_dependency_graph().unwrap();
    assert!(
        !database.get_import_settings("assets/data/Items.json")["include_in_build"]
            .as_bool()
            .unwrap()
    );
    assert!(graph["saves/scenes/main.scene"].contains(&"assets/data/Items.json".to_string()));
}

#[test]
fn file_browser_and_asset_tools_manage_game_assets() {
    let tmp = temp_dir("file-browser");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(tmp.join("assets").join("sprites").join("hero.png"), b"png").unwrap();
    fs::write(tmp.join("assets").join("audio").join("hit.wav"), b"wav").unwrap();

    let mut browser = FileBrowser::new(&tmp);
    let sprite_import = browser
        .create_sprite_import("HeroSprite", "assets/sprites/hero.png")
        .unwrap();
    let sound_cue = browser
        .create_sound_cue("HitCue", "assets/audio/hit.wav")
        .unwrap();
    let material = browser.create_material("HeroMaterial").unwrap();
    assert!(sprite_import.exists());
    assert!(sound_cue.exists());
    assert!(material.exists());

    let folder = browser.create_folder("assets", "Imported").unwrap();
    assert!(folder.exists());
    browser.select_asset_by_path(&sprite_import);
    let renamed = browser.rename_selected_asset("HeroIdle").unwrap().unwrap();
    assert!(
        renamed
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("HeroIdle")
    );
    let moved = browser
        .move_selected_asset("assets/Imported")
        .unwrap()
        .unwrap();
    assert!(moved.starts_with(tmp.join("assets").join("Imported")));

    let stats = browser.stats().unwrap();
    assert!(stats.sprites >= 2);
    assert!(stats.audio >= 2);
    assert!(
        browser
            .scan_entries()
            .unwrap()
            .iter()
            .any(|entry| entry.asset_type == "Material")
    );
}

#[test]
fn asset_browser_templates_launcher_and_kira_audio_cover_game_creation_flow() {
    let tmp = temp_dir("creation-flow");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let browser = FileBrowser::new(&tmp);

    let luau = browser.create_script("EnemyBrain").unwrap();
    let graph = browser.create_visual_graph("SpawnGraph").unwrap();
    let enemy = browser.create_enemy("Slime").unwrap();
    let ui = browser.create_ui("ScoreLabel").unwrap();
    let audio_event = browser.create_audio_event("Explosion").unwrap();
    assert_eq!(
        luau.extension().and_then(|value| value.to_str()),
        Some("luau")
    );
    assert_eq!(
        graph.extension().and_then(|value| value.to_str()),
        Some("mfgraph")
    );
    assert!(
        enemy
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".prefab")
    );
    assert!(
        ui.file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".ui.prefab")
    );
    assert!(
        audio_event
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".audio.json")
    );

    let mut database = AssetDatabase::new(tmp.join("assets"), &tmp).unwrap();
    database.scan().unwrap();
    assert!(
        database
            .assets
            .values()
            .any(|record| record.asset_type == "LuauScript")
    );
    assert!(
        database
            .assets
            .values()
            .any(|record| record.asset_type == "AudioEvent")
    );

    let created = ProjectTemplates::create(&tmp, "Platformer").unwrap();
    assert!(created.iter().any(|path| {
        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension == "luau")
    }));

    let workspace = temp_dir("launcher-workspace");
    let mut launcher = EguiProjectLauncher::new(&workspace);
    launcher.project_name = "LauncherGame".to_string();
    launcher.selected_template = LauncherTemplate::Rts;
    let project = launcher.create_new_project().unwrap();
    assert!(project.join("project.json").exists());
    assert_eq!(launcher.recent_projects.first(), Some(&project));
    let export = launcher.export_game(&project).unwrap();
    assert!(export.output_path.exists());

    let mut audio = AudioSystem::default();
    audio.play_music("theme", 0.8, 1.25);
    audio.play_sfx("click", 0.5);
    audio.set_volume("SFX", 0.4);
    let tween = audio.fade("Music", 0.2, 2.0);
    assert_eq!(tween.duration.as_secs_f64(), 2.0);
    assert!(
        audio
            .command_log
            .iter()
            .any(|command| command.kind == AudioCommandKind::Music && command.looped)
    );
    assert!(
        audio
            .command_log
            .iter()
            .any(|command| command.kind == AudioCommandKind::Fade && command.bus == "Music")
    );
    assert!(AudioSystem::volume_to_decibels(0.0).0 <= -60.0);
}

#[test]
fn launcher_creates_projects_in_free_locations_and_surfaces_patch_notes() {
    let workspace = temp_dir("launcher-free-root");
    let free_location = temp_dir("launcher-free-location");
    let mut launcher = EguiProjectLauncher::new(&workspace);
    launcher.project_location = free_location.display().to_string();
    launcher.project_name = "Free Location Game".to_string();
    launcher.selected_template = LauncherTemplate::TopDown;

    let project = launcher.create_new_project().unwrap();
    assert!(project.starts_with(&free_location));
    assert!(project.join("project.json").exists());
    assert!(launcher.backend_plan.is_some());
    assert!(launcher.backend_summary.contains("readiness"));
    assert!(!launcher.backend_actions.is_empty());
    assert!(
        launcher
            .active_patch_note()
            .unwrap()
            .highlights
            .iter()
            .any(|line| line.contains("Blueprint"))
    );
    assert!(launcher.discover_recent_projects().unwrap() >= 1);

    launcher.open_path = project.display().to_string();
    let summary = launcher.refresh_typed_project_status().unwrap();
    assert!(summary.contains("editor="));

    let repair_notes = launcher.repair_project(&project).unwrap();
    assert!(repair_notes.iter().any(|note| note.contains("Manifest")));
    assert!(repair_notes.iter().any(|note| note.starts_with("Next:")));
    assert_eq!(launcher.last_repair_notes, repair_notes);

    launcher.settings.validate_on_open = false;
    launcher.settings.analyze_before_export = false;
    launcher.export_profile = ExportProfile::Release;
    launcher.save_state().unwrap();

    let restored = EguiProjectLauncher::new(&workspace);
    assert_eq!(restored.recent_projects.first(), Some(&project));
    assert!(!restored.settings.validate_on_open);
    assert!(!restored.settings.analyze_before_export);
    assert_eq!(restored.export_profile, ExportProfile::Release);
}

#[test]
fn plugin_manager_emits_hooks() {
    let tmp = temp_dir("plugins");
    let plugin_dir = tmp.join("plugins").join("hello");
    fs::create_dir_all(&plugin_dir).unwrap();
    fs::write(
        plugin_dir.join("plugin.json"),
        "{\"name\":\"hello\",\"enabled\":true,\"hooks\":[\"on_editor_start\"]}",
    )
    .unwrap();
    let mut manager = PluginManager::new(&tmp);
    assert_eq!(manager.emit_hook("on_editor_start").unwrap(), 1);
}

#[test]
fn scene_view_tools_drag_moves_selected() {
    let entity = GameObject::new(1.0, 1.0, Some("Mover".to_string()));
    let mut selected = vec![entity];
    let tools = SceneViewTools {
        grid_snapping: false,
        ..Default::default()
    };
    tools.apply_screen_drag(&mut selected, 32.0, 0.0, "Move");
    assert_eq!((selected[0].x * 1000.0).round() / 1000.0, 2.0);
}

#[test]
fn build_report_summary() {
    let tmp = temp_dir("report");
    fs::write(tmp.join("runtime_manifest.json"), "{}").unwrap();
    let report = BuildReport::generate(&tmp).unwrap();
    assert!(report["summary"]["files"].as_u64().unwrap() >= 1);
    assert!(tmp.join("build_report.json").exists());
}

#[test]
fn advanced_component_behaviors_are_live() {
    let mut stats = default_component("Stats").unwrap();
    assert_eq!(stats.stats_add_experience(150.0), 1);
    assert_eq!(stats.get_i64("level", 1), 2);
    assert!(stats.stats_effective_attack() > 10.0);

    let mut equipment = default_component("Equipment").unwrap();
    assert!(equipment.equipment_equip("weapon", Some("sword"), json!({"attack": 5.0})));
    assert_eq!(equipment.equipment_total_bonus("attack"), 5.0);
    assert_eq!(equipment.equipment_unequip("weapon").unwrap(), "sword");

    let mut ability = default_component("Ability").unwrap();
    assert!(ability.ability_trigger(10.0));
    assert!(!ability.ability_is_ready(10.1));
    ability.ability_recharge(1);
    assert_eq!(ability.get_i64("current_charges", 0), 1);

    let mut quest = default_component("QuestLog").unwrap();
    assert!(quest.quest_add("q1", "Find key", json!([{"id": "key", "progress": 0}])));
    assert!(quest.quest_set_objective_progress("q1", "key", json!(1)));
    assert!(quest.quest_complete("q1"));

    let mut dialogue = default_component("Dialogue").unwrap();
    assert_eq!(dialogue.dialogue_current_line(), "Hello.");
    assert!(!dialogue.dialogue_advance());
}

#[test]
fn gameplay_updates_status_regen_state_nav_and_spawner() {
    let mut player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    player.tag = "Player".to_string();
    let mut health = default_component("Health").unwrap();
    health.set_f64("health", 50.0);
    health.set_f64("max_health", 100.0);
    player.add_component(health);
    let mut stats = default_component("Stats").unwrap();
    stats.set_f64("regen_per_second", 10.0);
    player.add_component(stats);
    GameAPI::add_status_effect(
        &mut player,
        "poison",
        1.0,
        1,
        json!({"damage_per_second": 4.0}),
    );
    let mut machine = default_component("StateMachine").unwrap();
    machine.set(
        "transitions",
        json!([{"from": "Idle", "after": 0.1, "to": "Run"}]),
    );
    player.add_component(machine);

    let mut navigator = GameObject::new(0.0, 0.0, Some("Navigator".to_string()));
    let mut nav = default_component("NavAgent").unwrap();
    nav.nav_set_destination(2.0, 0.0);
    nav.set("avoid_obstacles", json!(false));
    navigator.add_component(nav);

    let mut spawner_entity = GameObject::new(5.0, 5.0, Some("Spawner".to_string()));
    let mut spawner = default_component("Spawner").unwrap();
    spawner.set("spawn_on_start", json!(true));
    spawner.set("prefab_name", json!("Minion"));
    spawner_entity.add_component(spawner);

    let mut entities = vec![player, navigator, spawner_entity];
    let mut gameplay = GameplaySystem::default();
    gameplay.update_entities(&mut entities, 0.05, "PLAY");
    gameplay.update_entities(&mut entities, 0.05, "PLAY");
    gameplay.update_entities(&mut entities, 0.05, "PLAY");

    let player = entities
        .iter()
        .find(|entity| entity.name == "Player")
        .unwrap();
    assert!(
        player
            .get_component("Health")
            .unwrap()
            .get_f64("health", 0.0)
            > 50.0
    );
    assert_eq!(
        player
            .get_component("StateMachine")
            .unwrap()
            .get_string("current_state", ""),
        "Run"
    );
    let navigator = entities
        .iter()
        .find(|entity| entity.name == "Navigator")
        .unwrap();
    assert!(navigator.x > 0.0);
    assert!(entities.iter().any(|entity| entity.name == "Minion"));
    assert_eq!(gameplay.stats["spawners"], 1);
}

#[test]
fn command_system_supports_rts_orders_and_path_cleanup() {
    let mut grid = Grid::new(6, 6, 32, 8);
    grid.set_tile(5, 5, 1);
    let mut unit = GameObject::new_unit(0.0, 0.0, Some("Worker".to_string()));
    unit.add_component(default_component("Worker").unwrap());
    let mut target = GameObject::new(5.0, 5.0, Some("Gold".to_string()));
    target.add_component(default_component("ResourceNode").unwrap());

    assert!(CommandSystem::move_unit_to_grid(&grid, &mut unit, (99, 99)));
    assert_eq!(unit.command, "MOVE");
    assert!(!unit.path.is_empty());

    let mut units = vec![unit];
    assert_eq!(
        CommandSystem::gather_units(Some(&grid), &mut units, &target),
        1
    );
    assert_eq!(units[0].command, "GATHER");
    assert_eq!(units[0].gather_target_id, Some(target.id));

    CommandSystem::patrol_units(Some(&grid), &mut units, (2.0, 2.0));
    assert_eq!(units[0].command, "PATROL");
    assert_eq!(units[0].patrol_points.len(), 2);

    CommandSystem::cancel_units(&mut units);
    assert_eq!(units[0].command, "IDLE");

    let slots = CommandSystem::formation_targets(Some(&grid), 4, (5.0, 5.0), "line", 1.0);
    assert_eq!(slots.len(), 4);
    assert!(
        slots
            .iter()
            .all(|(x, y)| grid.is_walkable(*x as i32, *y as i32))
    );
}

#[test]
fn spatial_index_accelerates_runtime_queries() {
    let mut player = GameObject::new_unit(2.0, 2.0, Some("Player".to_string()));
    player.tag = "Player".to_string();
    player.layer = "Units".to_string();
    let mut enemy = GameObject::new_unit(8.0, 2.0, Some("Enemy".to_string()));
    enemy.tag = "Enemy".to_string();
    enemy.layer = "Units".to_string();
    let resource = GameObject::new(2.6, 2.2, Some("Gold".to_string()));

    let mut index = SpatialIndex::new(2.0);
    index.rebuild(&[player.clone(), enemy, resource]);
    assert_eq!(
        index
            .nearest(2.0, 2.0, 1.0, Some("Player"), Some("Units"))
            .unwrap()
            .entity_id,
        player.id
    );
    assert_eq!(index.query_radius(2.0, 2.0, 1.5, None, None).len(), 2);
    assert!(index.stats()["cells"] >= 1);
}

#[test]
fn game_clock_produces_fixed_ticks_and_clamps_spikes() {
    let mut clock = GameClock::new(0.02);
    let first = clock.advance(0.01);
    assert_eq!(first.fixed_steps, 0);
    let second = clock.advance(0.05);
    assert_eq!(second.fixed_steps, 3);
    assert_eq!(clock.tick, 3);
    clock.max_steps_per_frame = 2;
    let spike = clock.advance(1.0);
    assert_eq!(spike.fixed_steps, 2);
    assert!(spike.dropped_time >= 0.0);
}

#[test]
fn game_clock_uses_runtime_tuning_and_reports_pacing() {
    let mut clock = GameClock::new(1.0 / 60.0);
    clock.configure_fixed_step(1.0 / 120.0, 6);
    clock.configure_frame_budget(120);

    assert!((clock.fixed_delta - 1.0 / 120.0).abs() < 0.000001);
    assert_eq!(clock.max_steps_per_frame, 6);
    assert!(clock.frame_budget_ms() < 8.4);

    let frame = clock.advance(1.0 / 60.0);
    assert_eq!(frame.fixed_steps, 2);
    assert!(frame.over_budget);
    assert!(!frame.saturated_fixed_steps);
    assert!(frame.interpolation_alpha < 0.01);

    clock.max_steps_per_frame = 1;
    let spike = clock.advance(0.25);
    assert_eq!(spike.fixed_steps, 1);
    assert!(spike.saturated_fixed_steps);
    assert!(spike.dropped_time > 0.0);
}

#[test]
fn game_initializes_clock_from_runtime_config() {
    let tmp = temp_dir("clock-runtime-config");
    let paths = AssetTools::ensure_project_folders(&tmp).unwrap();
    AssetTools::write_json(
        paths.settings.join("runtime_config.json"),
        &serde_json::json!({
            "target_fps": 144,
            "fixed_timestep": 1.0 / 120.0,
            "max_frame_steps": 7,
            "quality_preset": "balanced",
            "performance_class": "desktop"
        }),
    )
    .unwrap();

    let mut game = Game::from_project(&tmp, true).unwrap();
    assert!((game.clock.fixed_delta - 1.0 / 120.0).abs() < 0.000001);
    assert_eq!(game.clock.max_steps_per_frame, 7);
    assert!(game.clock.frame_budget_ms() < 7.0);

    game.run_headless_once(1.0 / 60.0);
    assert!(game.profiler.metrics.contains_key("FrameBudgetMs"));
    assert_eq!(game.profiler.counters["FrameOverBudget"], 1);
    assert_eq!(game.diagnostics.last_frame.health, FrameHealth::OverBudget);
    assert!(game.diagnostics.health_summary().contains("over budget"));
}

#[test]
fn flow_field_builds_shared_paths_for_rts_squads() {
    let mut grid = Grid::new(8, 8, 32, 8);
    for y in 0..8 {
        if y != 4 {
            grid.set_tile(3, y, 1);
        }
    }
    let flow = FlowField::build(&grid, (7, 7), 200).unwrap();
    let path = flow.path_from(&grid, (0, 0), 32);
    assert!(!path.is_empty());
    assert_eq!(*path.last().unwrap(), flow.goal);
    assert!(
        path.iter()
            .any(|point| *point == (3, 4) || *point == (4, 4))
    );

    let mut units = vec![
        GameObject::new_unit(0.0, 0.0, Some("A".to_string())),
        GameObject::new_unit(0.0, 1.0, Some("B".to_string())),
    ];
    CommandSystem::flow_field_move_units(&grid, &mut units, (7, 7));
    assert_eq!(units[0].command, "FLOW_FIELD_MOVE");
    assert!(!units[0].path.is_empty());
    assert!(!units[1].path.is_empty());
}

#[test]
fn rts_navigation_avoids_threats_and_builds_influence_maps() {
    let mut grid = Grid::new(8, 5, 32, 8);
    assert!(grid.line_of_sight((0, 0), (7, 0)));
    grid.set_tile(3, 0, 1);
    assert!(!grid.line_of_sight((0, 0), (7, 0)));
    assert!(grid.reachable_area((0, 2), 10).len() > 8);
    grid.set_tile(3, 0, 0);
    let threats = vec![((2, 2), 100), ((3, 2), 100), ((4, 2), 100), ((5, 2), 100)];
    let path = threat_aware_astar(&grid, (0, 2), (7, 2), &threats, 30);
    assert!(!path.is_empty());
    assert!(
        !path
            .iter()
            .any(|point| threats.iter().any(|(threat, _)| threat == point))
    );

    let mut units = vec![GameObject::new_unit(0.0, 2.0, Some("Scout".to_string()))];
    CommandSystem::threat_aware_move_units(&grid, &mut units, (7, 2), &threats);
    assert_eq!(units[0].command, "THREAT_AWARE_MOVE");
    assert!(!units[0].path.is_empty());

    let influence = influence_map(&grid, &[((0, 0), 10), ((7, 4), -10)], 2);
    assert!(influence.get(&(0, 0)).copied().unwrap_or_default() > 0);
    assert!(influence.get(&(7, 4)).copied().unwrap_or_default() < 0);
    assert!(influence.get(&(3, 2)).copied().unwrap_or_default().abs() < 10);
}

#[test]
fn build_placement_validates_footprints_and_reserves_tiles() {
    let mut grid = Grid::new(10, 10, 32, 8);
    grid.set_tile(4, 4, 1);
    let mut blocking = GameObject::new(2.0, 2.0, Some("Blocker".to_string()));
    blocking.width = 2.0;
    blocking.height = 2.0;
    blocking.layer = "Buildings".to_string();
    let footprint = BuildFootprint {
        width: 2,
        height: 2,
        clearance: 0,
    };

    assert!(!BuildPlacement::validate(&grid, &[blocking.clone()], (4, 4), &footprint, None).valid);
    assert!(!BuildPlacement::validate(&grid, &[blocking], (1, 1), &footprint, None).valid);
    let valid =
        BuildPlacement::find_nearest_valid(&grid, &[], (4, 4), &footprint, 4, None).unwrap();
    assert!(valid.valid);
    BuildPlacement::reserve_on_grid(&mut grid, valid.cell, &footprint, 1);
    assert!(!grid.is_walkable(valid.cell.0, valid.cell.1));
}

#[test]
fn rts_system_runs_economy_production_construction_and_fog() {
    let mut base = GameObject::new(0.0, 0.0, Some("CommandCenter".to_string()));
    base.tag = "Player".to_string();
    base.add_component(default_component("Team").unwrap());
    base.add_component(default_component("EconomyWallet").unwrap());
    base.add_component(default_component("ProductionQueue").unwrap());
    base.add_component(default_component("Vision").unwrap());
    if let Some(team) = base.get_component_mut("Team") {
        team.set("team_id", json!(1));
    }
    if let Some(wallet) = base.get_component_mut("EconomyWallet") {
        wallet.set("resources", json!({"Gold": 200.0, "Wood": 0.0}));
    }
    if let Some(queue) = base.get_component_mut("ProductionQueue") {
        queue.set_f64("rally_x", 2.0);
        queue.set_f64("rally_y", 0.0);
    }
    assert!(RTSSystem::enqueue_production(
        &mut base,
        "Worker",
        "Worker",
        0.05,
        json!({"Gold": 50.0})
    ));

    let mut worker = GameObject::new_unit(1.0, 0.0, Some("Harvester".to_string()));
    worker.tag = "Player".to_string();
    worker.add_component(default_component("Team").unwrap());
    worker.add_component(default_component("Worker").unwrap());
    worker.add_component(default_component("Vision").unwrap());
    if let Some(team) = worker.get_component_mut("Team") {
        team.set("team_id", json!(1));
    }

    let mut gold = GameObject::new(1.5, 0.0, Some("Gold".to_string()));
    gold.tag = "Resource".to_string();
    gold.add_component(default_component("ResourceNode").unwrap());
    worker.gather_target_id = Some(gold.id);
    if let Some(worker_component) = worker.get_component_mut("Worker") {
        worker_component.set("gather_target_id", json!(gold.id));
    }

    let mut site = GameObject::new(4.0, 0.0, Some("Barracks_Site".to_string()));
    site.add_component(default_component("ConstructionSite").unwrap());
    if let Some(construction) = site.get_component_mut("ConstructionSite") {
        construction.set("target_name", json!("Barracks"));
        construction.set_f64("build_time", 0.05);
        construction.set("finished_components", json!(["Health", "ProductionQueue"]));
    }

    let mut controller = GameObject::new(0.0, 0.0, Some("RTSController".to_string()));
    controller.visible = false;
    controller.add_component(default_component("Team").unwrap());
    controller.add_component(default_component("FogOfWar").unwrap());
    if let Some(team) = controller.get_component_mut("Team") {
        team.set("team_id", json!(1));
    }
    if let Some(fog) = controller.get_component_mut("FogOfWar") {
        fog.set("team_id", json!(1));
        fog.set("map_width", json!(16));
        fog.set("map_height", json!(16));
    }

    let mut entities = vec![base, worker, gold, site, controller];
    let mut rts = RTSSystem::default();
    rts.update_entities(&mut entities, 0.1, "PLAY");

    assert!(entities.iter().any(|entity| entity.name == "Worker"));
    assert!(
        entities
            .iter()
            .find(|entity| entity.name == "Barracks")
            .unwrap()
            .get_component("ProductionQueue")
            .is_some()
    );
    let base = entities
        .iter()
        .find(|entity| entity.name == "CommandCenter")
        .unwrap();
    assert!(
        base.get_component("EconomyWallet")
            .unwrap()
            .get("resources")
            .unwrap()["Gold"]
            .as_f64()
            .unwrap()
            > 150.0
    );
    let controller = entities
        .iter()
        .find(|entity| entity.name == "RTSController")
        .unwrap();
    assert!(
        controller
            .get_component("FogOfWar")
            .unwrap()
            .get("visible_tiles")
            .unwrap()
            .as_array()
            .unwrap()
            .len()
            > 1
    );
    assert_eq!(rts.stats["produced"], 1);
    assert_eq!(rts.stats["completed_constructions"], 1);
}

#[test]
fn archetype_library_spawns_ready_to_use_game_entities() {
    let library = ArchetypeLibrary::with_defaults();
    assert!(library.keys().contains(&"rts_soldier".to_string()));
    assert!(!library.by_tag("Player").is_empty());

    let soldier = library
        .instantiate("rts_soldier", 4.0, 5.0, Some(2))
        .unwrap();
    assert_eq!(soldier.tag, "Enemy");
    assert_eq!(soldier.layer, "Units");
    assert!(soldier.get_component("Team").is_some());
    assert_eq!(
        soldier.get_component("Team").unwrap().get_i64("team_id", 0),
        2
    );
    assert!(soldier.get_component("DamageDealer").is_some());
    assert!(soldier.get_component("CombatTarget").is_some());
    assert!(soldier.get_component("SquadMember").is_some());

    let mut entities = Vec::new();
    let worker_id =
        GameAPI::spawn_archetype(&mut entities, &library, "rts_worker", 1.0, 2.0, Some(1)).unwrap();
    let worker = entities
        .iter_mut()
        .find(|entity| entity.id == worker_id)
        .unwrap();
    assert!(GameAPI::assign_squad(worker, "alpha", 3, "builder"));
    assert_eq!(
        worker
            .get_component("SquadMember")
            .unwrap()
            .get_string("squad_id", ""),
        "alpha"
    );
    GameAPI::issue_attack_move(worker, 8.0, 2.0);
    assert_eq!(worker.command, "ATTACK_MOVE");
    assert_eq!(worker.attack_move_target, Some((8.0, 2.0)));
}

#[test]
fn rts_system_executes_tactical_combat_and_auto_queue() {
    let library = ArchetypeLibrary::with_defaults();
    let mut attacker = library
        .instantiate("rts_soldier", 0.0, 0.0, Some(1))
        .unwrap();
    let mut target = library
        .instantiate("rts_soldier", 1.0, 0.0, Some(2))
        .unwrap();
    if let Some(damage) = attacker.get_component_mut("DamageDealer") {
        damage.set_f64("damage", 120.0);
        damage.set_f64("cooldown", 0.01);
        damage.set_f64("range", 2.0);
    }
    if let Some(health) = target.get_component_mut("Health") {
        health.set_f64("health", 40.0);
    }

    let mut base = library
        .instantiate("rts_command_center", 5.0, 0.0, Some(1))
        .unwrap();
    if let Some(book) = base.get_component_mut("ProductionRecipeBook") {
        book.set("auto_queue", json!(true));
        book.set("preferred_recipe", json!("Worker"));
    }
    if let Some(queue) = base.get_component_mut("ProductionQueue") {
        queue.set_f64("rally_x", 6.0);
        queue.set_f64("rally_y", 0.0);
    }

    let mut entities = vec![attacker, target, base];
    let mut rts = RTSSystem::default();
    rts.update_entities(&mut entities, 0.1, "PLAY");

    assert!(rts.stats["combat_events"] >= 1);
    assert_eq!(rts.stats["destroyed"], 1);
    assert!(!entities.iter().any(|entity| entity.tag == "Enemy"));
    assert_eq!(rts.stats["auto_queued"], 1);
    assert!(
        entities
            .iter()
            .find(|entity| entity.name == "CommandCenter")
            .unwrap()
            .get_component("ProductionQueue")
            .unwrap()
            .get("queue")
            .unwrap()
            .as_array()
            .unwrap()
            .len()
            == 1
    );
}

#[test]
fn physics_resolves_collisions_and_honors_layer_matrix() {
    let mut first = GameObject::new(0.0, 0.0, Some("First".to_string()));
    first.add_component(default_component("Rigidbody2D").unwrap());
    let mut second = GameObject::new(0.4, 0.0, Some("Second".to_string()));
    second.add_component(default_component("Rigidbody2D").unwrap());
    let mut entities = vec![first, second];
    let mut physics = PhysicsSystem::new();
    physics.gravity = (0.0, 0.0);
    physics.update_entities_mut(&mut entities, 0.016, "PLAY");
    assert!(physics.stats["contacts"] >= 1);
    assert!(entities[0].x < 0.0 || entities[1].x > 0.4);

    let mut first = GameObject::new(0.0, 0.0, Some("First".to_string()));
    first.layer = "A".to_string();
    first.add_component(default_component("Rigidbody2D").unwrap());
    let mut second = GameObject::new(0.4, 0.0, Some("Second".to_string()));
    second.layer = "B".to_string();
    second.add_component(default_component("Rigidbody2D").unwrap());
    let mut entities = vec![first, second];
    let mut physics = PhysicsSystem::new();
    physics.gravity = (0.0, 0.0);
    physics.set_layer_collision("A", "B", false);
    physics.update_entities_mut(&mut entities, 0.016, "PLAY");
    assert_eq!(physics.stats["contacts"], 0);
}

#[test]
fn physics_supports_body_types_materials_events_and_raycast() {
    let mut ball = GameObject::new(0.0, 0.0, Some("Ball".to_string()));
    let mut ball_body = default_component("Rigidbody2D").unwrap();
    ball_body.set_f64("velocity_x", 5.0);
    ball_body.set_f64("velocity_y", 2.0);
    ball_body.set_f64("bounciness", 1.0);
    ball_body.set_f64("friction", 0.5);
    ball.add_component(ball_body);
    if let Some(collider) = ball.get_component_mut("Collider2D") {
        collider.set("shape", json!("circle"));
        collider.set_f64("radius", 0.55);
    }

    let wall = GameObject::new(0.4, 0.0, Some("Wall".to_string()));
    let mut entities = vec![ball, wall];
    let mut physics = PhysicsSystem::new();
    physics.gravity = (0.0, 0.0);
    physics.update_entities_mut(&mut entities, 0.0, "PLAY");
    let velocity_x = entities[0]
        .get_component("Rigidbody2D")
        .unwrap()
        .get_f64("velocity_x", 0.0);
    let velocity_y = entities[0]
        .get_component("Rigidbody2D")
        .unwrap()
        .get_f64("velocity_y", 0.0);
    assert!(velocity_x < 0.0);
    assert!(velocity_y.abs() < 2.0);
    assert!(physics.events.iter().any(|event| {
        event.phase == PhysicsEventPhase::Enter && event.pair_type == PairType::Collision
    }));

    let mut kinematic = GameObject::new(0.0, 4.0, Some("Mover".to_string()));
    let mut kin_body = default_component("Rigidbody2D").unwrap();
    kin_body.set("body_type", json!("kinematic"));
    kin_body.set_f64("velocity_x", 10.0);
    kinematic.add_component(kin_body);
    let mut static_body = GameObject::new(0.0, 8.0, Some("Static".to_string()));
    let mut st_body = default_component("Rigidbody2D").unwrap();
    st_body.set("body_type", json!("static"));
    st_body.set_f64("velocity_x", 10.0);
    static_body.add_component(st_body);
    let mut bodies = vec![kinematic, static_body];
    physics.update_entities_mut(&mut bodies, 0.1, "PLAY");
    assert!(bodies[0].x > 0.4);
    assert_eq!(bodies[1].x, 0.0);

    let mut sensor = GameObject::new(3.0, 0.0, Some("Sensor".to_string()));
    sensor.layer = "Sensors".to_string();
    if let Some(collider) = sensor.get_component_mut("Collider2D") {
        collider.set("shape", json!("polygon"));
        collider.set(
            "points",
            json!([[-0.75, -0.75], [0.75, -0.75], [0.75, 0.75], [-0.75, 0.75]]),
        );
        collider.set("is_trigger", json!(true));
        collider.set("collision_layer", json!("Sensors"));
    }
    let layers = vec!["Sensors".to_string()];
    assert!(
        physics
            .raycast_filtered(&[sensor.clone()], (0.0, 0.0), (1.0, 0.0), 10.0, false, None)
            .is_none()
    );
    let hit = physics
        .raycast_filtered(&[sensor], (0.0, 0.0), (1.0, 0.0), 10.0, true, Some(&layers))
        .unwrap();
    assert_eq!(hit.entity_name, "Sensor");
    assert!(hit.is_trigger);
}

#[test]
fn rapier_bridge_validates_miniforge_2d_physics_shapes() {
    let mut dynamic = GameObject::new(0.0, 0.0, Some("DynamicBall".to_string()));
    dynamic.add_component(default_component("Rigidbody2D").unwrap());
    if let Some(collider) = dynamic.get_component_mut("Collider2D") {
        collider.set("shape", json!("circle"));
        collider.set_f64("radius", 0.75);
        collider.set("is_trigger", json!(true));
    }
    let mut polygon = GameObject::new(2.0, 0.0, Some("Polygon".to_string()));
    if let Some(collider) = polygon.get_component_mut("Collider2D") {
        collider.set("shape", json!("polygon"));
        collider.set("points", json!([[-0.5, -0.5], [0.5, -0.5], [0.25, 0.75]]));
    }
    let report = RapierPhysicsBridge::inspect_scene(&[dynamic, polygon], (0.0, 9.8));
    assert_eq!(report.colliders, 2);
    assert_eq!(report.dynamic_bodies, 1);
    assert_eq!(report.sensors, 1);
    assert!(report.broadphase_aabb_area > 0.0);
    assert!(report.status_line().contains("Rapier2D"));
}

#[test]
fn physics_reports_trigger_enter_stay_and_exit() {
    let mut player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    player.add_component(default_component("Rigidbody2D").unwrap());
    let mut sensor = GameObject::new(0.0, 0.0, Some("Sensor".to_string()));
    if let Some(collider) = sensor.get_component_mut("Collider2D") {
        collider.set("is_trigger", json!(true));
    }

    let mut entities = vec![player, sensor];
    let mut physics = PhysicsSystem::new();
    physics.gravity = (0.0, 0.0);
    physics.update_entities_mut(&mut entities, 0.016, "PLAY");
    assert!(physics.events.iter().any(|event| {
        event.phase == PhysicsEventPhase::Enter && event.pair_type == PairType::Trigger
    }));
    physics.update_entities_mut(&mut entities, 0.016, "PLAY");
    assert!(
        physics
            .events
            .iter()
            .any(|event| event.phase == PhysicsEventPhase::Stay)
    );
    entities[0].x = 10.0;
    entities[0].sync_to_components();
    physics.update_entities_mut(&mut entities, 0.016, "PLAY");
    assert!(
        physics
            .events
            .iter()
            .any(|event| event.phase == PhysicsEventPhase::Exit)
    );
}

#[test]
fn scene_manager_loads_additive_restarts_transitions_and_stack() {
    let tmp = temp_dir("scene-flow");
    AssetTools::ensure_project_folders(&tmp).unwrap();

    fn write_scene(project: &std::path::Path, name: &str, entity_name: &str, x: f64) {
        let mut entity = GameObject::new(x, 0.0, Some(entity_name.to_string()));
        entity.scene_name = Some(format!("{name}.scene"));
        let data = json!({
            "version": miniforge::ENGINE_VERSION,
            "engine_version": miniforge::ENGINE_VERSION,
            "scene_name": name,
            "entities": [entity.serialize()],
            "tiles": [],
            "tilemap_layers": TilemapLayers::new(2, 2).serialize(),
            "camera": {"x": x, "y": 0.0, "zoom": 1.0},
            "settings": {},
            "ui_canvases": [],
        });
        AssetTools::write_json(
            project
                .join("saves")
                .join("scenes")
                .join(format!("{name}.scene")),
            &data,
        )
        .unwrap();
    }

    write_scene(&tmp, "main", "MainEntity", 1.0);
    write_scene(&tmp, "battle", "BattleEntity", 2.0);
    write_scene(&tmp, "hud", "HudEntity", 3.0);
    write_scene(&tmp, "menu", "MenuEntity", 4.0);

    let mut game = Game::from_project(&tmp, false).unwrap();
    let mut global = GameObject::new(99.0, 0.0, Some("GlobalAudio".to_string()));
    global.add_component(default_component("DontDestroyOnLoad").unwrap());
    game.units.push(global);
    let loaded = game.load_scene("battle").unwrap();
    assert_eq!(loaded, 2);
    assert!(game.units.iter().any(|entity| entity.name == "GlobalAudio"));
    assert!(
        game.units
            .iter()
            .any(|entity| entity.name == "BattleEntity")
    );

    assert_eq!(game.load_scene_additive("hud").unwrap(), 1);
    assert!(
        game.scene_manager
            .loaded_scenes
            .contains(&"hud.scene".to_string())
    );
    assert_eq!(game.unload_scene("hud"), 1);
    assert!(!game.units.iter().any(|entity| entity.name == "HudEntity"));

    game.units
        .iter_mut()
        .find(|entity| entity.name == "BattleEntity")
        .unwrap()
        .x = 42.0;
    game.restart_scene().unwrap();
    assert_eq!(
        game.units
            .iter()
            .find(|entity| entity.name == "BattleEntity")
            .unwrap()
            .x,
        2.0
    );

    game.push_scene("menu").unwrap();
    assert_eq!(game.scene_manager.current_scene, "menu.scene");
    assert!(game.scene_manager.scene_stack.len() >= 2);
    game.pop_scene().unwrap();
    assert_eq!(game.scene_manager.current_scene, "battle.scene");

    game.transition_to_scene("menu", "fade", 0.25).unwrap();
    assert_eq!(game.scene_manager.current_scene, "menu.scene");
    assert_eq!(
        game.scene_manager.transition.as_ref().unwrap().from_scene,
        "battle.scene"
    );
    assert!(game.scene_manager.update_transition(0.25).unwrap().complete);
}

#[test]
fn game_api_covers_queries_resources_blackboard_and_save_state() {
    let tmp = temp_dir("api-save");
    let mut entity = GameObject::new(1.0, 2.0, Some("Hero".to_string()));
    entity.tag = "Player".to_string();
    GameAPI::add_component(&mut entity, "Saveable", Some(json!({"save_key": "hero"})));
    assert_eq!(GameAPI::add_resource(&mut entity, "Gold", 25.0), Some(25.0));
    assert!(GameAPI::spend_resource(&mut entity, "Gold", 10.0));
    assert!(GameAPI::set_blackboard(&mut entity, "has_key", json!(true)));
    assert_eq!(
        GameAPI::get_blackboard(&entity, "has_key", json!(false)),
        true
    );

    let mut entities = vec![entity];
    assert_eq!(GameAPI::find(&entities, "Hero").unwrap().tag, "Player");
    assert_eq!(GameAPI::find_with_tag(&entities, "Player").len(), 1);
    assert_eq!(
        GameAPI::query_radius(&entities, 1.0, 2.0, 0.5, Some("Player"), None).len(),
        1
    );

    let save_path = tmp.join("savegame.json");
    GameAPI::save_game_state(&mut entities, &save_path).unwrap();
    entities[0].x = 99.0;
    assert!(GameAPI::load_game_state(&mut entities, &save_path).unwrap());
    assert_eq!(entities[0].x, 1.0);

    GameAPI::set_x(&mut entities[0], 11.0);
    GameAPI::move_y(&mut entities[0], 4.0);
    GameAPI::set_scale(&mut entities[0], 2.0, 3.0);
    GameAPI::rotate_by(&mut entities[0], 45.0);
    GameAPI::set_size(&mut entities[0], 1.5, 2.0);
    assert_eq!(entities[0].x, 11.0);
    assert_eq!(entities[0].y, 6.0);
    assert_eq!(entities[0].scale_x, 2.0);
    assert_eq!(entities[0].width, 1.5);
    assert!(GameAPI::set_component_value(
        &mut entities[0],
        "Transform",
        "x",
        json!(12.0)
    ));
    assert_eq!(
        GameAPI::get_component_value(&entities[0], "Transform", "x").unwrap(),
        json!(12.0)
    );
    assert!(GameAPI::add_audio_source(&mut entities[0], "HitCue", true));
    assert_eq!(
        entities[0]
            .get_component("AudioSource")
            .unwrap()
            .get_string("audio_name", ""),
        "HitCue"
    );
    let sprite_id =
        GameAPI::spawn_sprite_entity(&mut entities, "SpriteEntity", "HeroSprite", 3.0, 4.0);
    assert!(
        entities
            .iter()
            .find(|entity| entity.id == sprite_id)
            .unwrap()
            .sprite_name
            .as_deref()
            == Some("HeroSprite")
    );
}

#[test]
fn runtime_2d_controller_camera_checkpoint_and_tile_collision_are_live() {
    let tmp = temp_dir("runtime-2d");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    game.create_platformer_starter();
    let player_id = game
        .units
        .iter()
        .find(|entity| entity.name == "PlatformerPlayer")
        .unwrap()
        .id;
    let start_x = game.get_entity_by_id(player_id).unwrap().x;

    game.enter_play_mode();
    assert!(game.set_character_input(player_id, (1.0, 0.0), false, false, false, false));
    game.run_headless_once(0.2);
    assert!(game.get_entity_by_id(player_id).unwrap().x > start_x);
    assert!(
        game.profiler
            .counters
            .get("Runtime2DControllers")
            .copied()
            .unwrap_or(0)
            >= 1
    );

    let checkpoint_id = game
        .units
        .iter()
        .find(|entity| entity.name == "Checkpoint_A")
        .unwrap()
        .id;
    GameAPI::set_position(game.get_entity_by_id_mut(player_id).unwrap(), 44.0, 10.0);
    game.run_headless_once(1.0 / 60.0);
    assert!(
        game.get_entity_by_id(checkpoint_id)
            .unwrap()
            .get_component("Checkpoint")
            .unwrap()
            .get_bool("active", false)
    );

    let fall_y = game
        .get_entity_by_id(player_id)
        .unwrap()
        .get_component("CharacterController2D")
        .unwrap()
        .get_f64("fall_death_y", 9999.0);
    game.get_entity_by_id_mut(player_id).unwrap().y = fall_y + 1.0;
    game.run_headless_once(1.0 / 60.0);
    let player = game.get_entity_by_id(player_id).unwrap();
    assert!(player.y <= 10.1);
    assert!(
        game.profiler
            .counters
            .get("Runtime2DRespawns")
            .copied()
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn gameplay_destroyed_enemies_drop_loot_pickups() {
    let mut attacker = GameObject::new(0.0, 0.0, Some("Attacker".to_string()));
    attacker.tag = "Enemy".to_string();
    let mut ai = default_component("AIController").unwrap();
    ai.set("behavior", json!("attack"));
    ai.set("target_tags", json!(["Victim"]));
    ai.set_f64("attack_radius", 4.0);
    attacker.add_component(ai);
    let mut damage = default_component("DamageDealer").unwrap();
    damage.set_f64("damage", 25.0);
    damage.set_f64("cooldown", 0.0);
    attacker.add_component(damage);

    let mut victim = GameObject::new(1.0, 0.0, Some("Victim".to_string()));
    victim.tag = "Victim".to_string();
    let mut health = default_component("Health").unwrap();
    health.set_f64("health", 5.0);
    victim.add_component(health);
    let mut loot = default_component("LootTable").unwrap();
    loot.set(
        "entries",
        json!([{"id": "coin", "weight": 1.0, "min": 2, "max": 2}]),
    );
    victim.add_component(loot);

    let mut entities = vec![attacker, victim];
    let mut gameplay = GameplaySystem::default();
    gameplay.update_entities(&mut entities, 1.0 / 60.0, "PLAY");

    assert!(!entities.iter().any(|entity| entity.name == "Victim"));
    let loot = entities
        .iter()
        .find(|entity| entity.name == "Loot_coin")
        .unwrap();
    assert_eq!(
        loot.get_component("Blackboard")
            .unwrap()
            .blackboard_get("quantity", json!(0)),
        json!(2)
    );
    assert_eq!(gameplay.stats["loot_drops"], 1);
}

#[test]
fn save_state_restores_components_and_can_recreate_persistent_entities() {
    let tmp = temp_dir("api-save-v2");
    let mut hero = GameObject::new(1.0, 2.0, Some("Hero".to_string()));
    GameAPI::add_component(
        &mut hero,
        "Saveable",
        Some(json!({"save_key": "hero", "persistent": true})),
    );
    hero.add_component(default_component("Health").unwrap());
    GameAPI::add_item(&mut hero, "potion", 2);
    let save_path = tmp.join("savegame.json");
    let mut entities = vec![hero];

    GameAPI::save_game_state(&mut entities, &save_path).unwrap();
    entities[0].x = 50.0;
    entities[0]
        .get_component_mut("Health")
        .unwrap()
        .take_damage(40.0);
    entities[0]
        .get_component_mut("Inventory")
        .unwrap()
        .inventory_remove_item("potion", 2);

    assert!(GameAPI::load_game_state(&mut entities, &save_path).unwrap());
    assert_eq!(entities[0].x, 1.0);
    assert_eq!(
        entities[0]
            .get_component("Health")
            .unwrap()
            .get_f64("health", 0.0),
        100.0
    );
    assert_eq!(GameAPI::item_count(&entities[0], "potion"), 2);

    let mut recreated = Vec::new();
    assert_eq!(
        GameAPI::load_game_state_into(&mut recreated, &save_path).unwrap(),
        1
    );
    assert_eq!(recreated[0].name, "Hero");
    assert_eq!(GameAPI::item_count(&recreated[0], "potion"), 2);
}

#[test]
fn peripheral_systems_have_runtime_state() {
    let tmp = temp_dir("peripheral-input");
    let mut input_map = InputMap::new(tmp.join("input_map.json")).unwrap();
    input_map
        .set_binding("jump", vec!["space".to_string()])
        .unwrap();
    let mut input = InputHandler::default();
    input.set_pressed("space", true);
    assert!(input.action_pressed(&input_map, "jump"));

    let mut camera = Camera::default();
    camera.set_bounds(-1000.0, -1000.0, 1000.0, 1000.0);
    CameraSystem::pan(&mut camera, (1.0, 0.0), 0.1);
    assert!(camera.x > 0.0);
    CameraSystem::zoom(&mut camera, 1.0, 0.5);
    assert!(camera.zoom > 1.0);

    let mut entity = GameObject::new(0.0, 0.0, Some("Speaker".to_string()));
    let mut source = default_component("AudioSource").unwrap();
    source.set("audio_name", json!("click"));
    source.set("play_on_start", json!(true));
    entity.add_component(source);
    let mut entities = vec![entity];
    let mut audio = AudioSystem::default();
    audio.update_entities(&mut entities, &AudioMixer::new(), "PLAY");
    assert_eq!(audio.stats["voices"], 1);

    let mut render = RenderSystem::default();
    render.draw_entities(&entities);
    assert_eq!(render.renderer.last_visible_entities, 1);
}

#[test]
fn templates_validator_and_prefabs_are_real_backend() {
    let tmp = temp_dir("templates-validator");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let created = ProjectTemplates::create(&tmp, "ActionRPG").unwrap();
    assert!(created.len() >= 8);
    assert!(
        tmp.join("saves")
            .join("scenes")
            .join("ActionRPG_Level.scene")
            .exists()
    );

    let mut entity = GameObject::new(3.0, 4.0, Some("Hero Prefab!".to_string()));
    entity.tag = "Player".to_string();
    let manager = PrefabManager::new(&tmp);
    let prefab_path = manager.save_prefab(&mut entity, None).unwrap();
    assert_eq!(
        prefab_path.file_name().and_then(|value| value.to_str()),
        Some("heroprefab.prefab")
    );

    let mut entities = Vec::new();
    let id = manager
        .instantiate_prefab(&mut entities, &prefab_path, 9.0, 10.0)
        .unwrap()
        .unwrap();
    assert_eq!(entities[0].id, id);
    assert!(entities[0].is_prefab_instance);
    assert_eq!(entities[0].x, 9.0);

    let mut validator = ProjectValidator::default();
    assert!(validator.validate_with_context(&tmp, &entities, None));
    assert!(validator.errors.is_empty());
}

#[test]
fn rust_editor_backend_spawns_edits_tiles_assets_and_templates() {
    let tmp = temp_dir("rust-editor-backend");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    let initial = game.units.len();

    let unit_id = game.spawn_unit("Hero", 2.0, 3.0);
    let unit = game.get_entity_by_id(unit_id).unwrap();
    assert_eq!(unit.tag, "Player");
    assert!(unit.get_component("Inventory").is_some());
    assert!(unit.get_component("NavAgent").is_some());
    assert!(unit.get_component("Worker").is_some());

    let enemy_id = game.spawn_enemy(5.0, 3.0);
    let enemy = game.get_entity_by_id(enemy_id).unwrap();
    assert_eq!(enemy.tag, "Enemy");
    assert!(enemy.get_component("AIController").is_some());
    assert!(enemy.get_component("DamageDealer").is_some());

    let resource_id = game.spawn_resource(7.0, 4.0);
    assert!(
        game.get_entity_by_id(resource_id)
            .unwrap()
            .get_component("ResourceNode")
            .is_some()
    );
    let site_id = game
        .try_place_rts_building("Barracks", (12, 12), 1, vec![unit_id])
        .unwrap();
    assert!(
        game.get_entity_by_id(site_id)
            .unwrap()
            .get_component("ConstructionSite")
            .is_some()
    );

    assert!(game.add_component_to_entity(unit_id, "Light2D"));
    assert!(
        game.get_entity_by_id(unit_id)
            .unwrap()
            .get_component("Light2D")
            .is_some()
    );
    let copy_id = game.duplicate_entity(unit_id).unwrap();
    assert_ne!(copy_id, unit_id);
    assert!(game.delete_entity(enemy_id));
    assert!(game.units.len() >= initial + 3);

    assert!(game.paint_tile(1, 1, 4));
    assert_eq!(game.tilemap_layers.layer("Ground").unwrap().get(1, 1), 4);
    assert_eq!(game.cycle_tilemap_layer(), "Decoration");
    assert!(game.paint_tile(2, 2, 6));
    assert_eq!(
        game.tilemap_layers.layer("Decoration").unwrap().get(2, 2),
        6
    );
    game.camera.x = 64.0;
    game.camera.y = 32.0;
    game.camera.set_zoom(1.35);
    game.save_scene().unwrap();
    game.save_project().unwrap();
    assert!(tmp.join("project").join("project_state.json").exists());
    let loaded = Game::from_project(&tmp, false).unwrap();
    assert_eq!(
        loaded.tilemap_layers.layer("Decoration").unwrap().get(2, 2),
        6
    );
    assert_eq!(loaded.camera.x, 64.0);
    assert_eq!(loaded.camera.zoom, 1.35);

    assert!(game.create_project_template("Survival").unwrap() >= 8);
    assert!(game.refresh_assets().unwrap() > 0);
    let manifest = game.build_manifest().unwrap();
    assert!(manifest["scenes"].as_array().unwrap().iter().any(|entry| {
        entry
            .as_str()
            .unwrap_or_default()
            .contains("Survival_Map.scene")
    }));
    assert!(game.validate_project());

    let duplicated = game
        .scene_manager
        .duplicate_current_scene("CopiedScene")
        .unwrap();
    assert!(duplicated.exists());
    assert!(game.scene_manager.open_scene("CopiedScene").unwrap());
    assert!(
        game.scene_manager
            .scene_metadata()
            .unwrap()
            .get("exists")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    );
}

#[test]
fn rust_game_starters_create_functional_2d_scenes() {
    let tmp = temp_dir("game-starters");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();

    let topdown_count = game.create_topdown_starter();
    assert!(topdown_count >= 10);
    let hero = game
        .units
        .iter()
        .find(|entity| entity.name == "Hero")
        .unwrap();
    assert!(hero.get_component("CharacterController2D").is_some());
    assert!(hero.get_component("CameraFollow").is_some());
    assert!(hero.get_component("QuestLog").is_some());
    assert!(
        game.units
            .iter()
            .any(|entity| entity.get_component("Dialogue").is_some())
    );
    assert!(
        game.units
            .iter()
            .any(|entity| entity.get_component("VisualScript").is_some())
    );
    assert_eq!(game.tilemap_layers.layer("Collision").unwrap().get(0, 0), 4);

    let platformer_count = game.create_platformer_starter();
    assert!(platformer_count >= 12);
    let player = game
        .units
        .iter()
        .find(|entity| entity.name == "PlatformerPlayer")
        .unwrap();
    assert!(player.get_component("Rigidbody2D").is_some());
    assert!(player.get_component("CharacterController2D").is_some());
    assert!(player.get_component("Checkpoint").is_some());
    assert!(
        game.units
            .iter()
            .any(|entity| entity.get_component("TilemapCollider").is_some())
    );
    assert!(
        game.units
            .iter()
            .filter(|entity| entity.name.starts_with("Coin_"))
            .count()
            >= 6
    );
    assert!(
        game.grid
            .get_tile(0, game.grid.height - 1)
            .is_some_and(|tile| tile == 1)
    );
}

#[test]
fn production_editor_inspector_commands_undo_redo_and_components() {
    let tmp = temp_dir("production-editor");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    let id = game.spawn_game_object("Editable", 1.0, 2.0);

    game.edit_inspector_value(id, "Transform", "x", json!(9.0))
        .unwrap();
    assert_eq!(game.get_entity_by_id(id).unwrap().x, 9.0);
    assert!(game.undo_editor_command().is_some());
    assert_eq!(game.get_entity_by_id(id).unwrap().x, 1.0);
    assert!(game.redo_editor_command().is_some());
    assert_eq!(game.get_entity_by_id(id).unwrap().x, 9.0);

    assert!(game.add_component_to_entity(id, "Stats"));
    assert!(
        game.get_entity_by_id(id)
            .unwrap()
            .get_component("Stats")
            .is_some()
    );
    game.remove_component_from_entity(id, "Stats").unwrap();
    assert!(
        game.get_entity_by_id(id)
            .unwrap()
            .get_component("Stats")
            .is_none()
    );
    assert!(game.undo_editor_command().is_some());
    assert!(
        game.get_entity_by_id(id)
            .unwrap()
            .get_component("Stats")
            .is_some()
    );
}

#[test]
fn production_tile_brushes_cover_rectangle_fill_collision_and_undo() {
    let mut tilemap = TilemapLayers::new(6, 6);
    let rect = TileBrush::apply(&mut tilemap, TileBrushMode::Rectangle, (1, 1), (2, 2), 3);
    assert_eq!(rect.changes.len(), 4);
    assert_eq!(tilemap.layer("Ground").unwrap().get(2, 2), 3);

    let fill = TileBrush::apply(&mut tilemap, TileBrushMode::Fill, (0, 0), (0, 0), 1);
    assert!(!fill.changes.is_empty());
    assert_eq!(tilemap.layer("Ground").unwrap().get(0, 0), 1);

    let collision = TileBrush::apply(&mut tilemap, TileBrushMode::Collision, (4, 4), (4, 4), 7);
    assert_eq!(tilemap.layer("Collision").unwrap().get(4, 4), 7);
    assert_eq!(collision.layer, 2);

    let tmp = temp_dir("tile-brush-game");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    assert!(game.paint_tile_brush(TileBrushMode::Rectangle, (0, 0), (1, 1), 5));
    assert_eq!(game.tilemap_layers.layer("Ground").unwrap().get(1, 1), 5);
    assert!(game.undo_editor_command().is_some());
    assert_eq!(game.tilemap_layers.layer("Ground").unwrap().get(1, 1), 0);
}

#[test]
fn production_assets_preview_drag_drop_and_runtime_export() {
    let tmp = temp_dir("production-assets");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(tmp.join("assets").join("sprites").join("hero.png"), b"png").unwrap();
    fs::write(tmp.join("assets").join("audio").join("hit.wav"), b"wav").unwrap();

    let mut game = Game::from_project(&tmp, false).unwrap();
    game.refresh_assets().unwrap();
    let sprite = game
        .asset_database
        .assets
        .values()
        .find(|asset| asset.relative_path.ends_with("hero.png"))
        .cloned()
        .unwrap();
    let preview = game.asset_database.preview(&sprite.relative_path).unwrap();
    assert_eq!(preview.kind.label(), "Image");
    assert!(preview.labels.contains(&"sprite".to_string()));

    let payload = DragPayload::from_asset(&sprite);
    let outcome = game.drop_asset_to_scene(&payload, 4.0, 5.0).unwrap();
    assert!(matches!(
        outcome,
        miniforge::engine::content_drag::DropOutcome::SpawnedEntity(_)
    ));
    assert!(
        game.units
            .iter()
            .any(|entity| entity.sprite_guid.as_deref() == Some(sprite.guid.as_str()))
    );

    let report =
        RuntimeExporter::export_with_profile(&tmp, tmp.join("exports"), ExportProfile::Release)
            .unwrap();
    assert!(report.manifest_path.exists());
    assert_eq!(report.profile, ExportProfile::Release);
    assert!(report.copied_files > 0);
}

#[test]
fn production_input_map_has_visual_actions_and_devices() {
    let tmp = temp_dir("production-input");
    let mut input_map = InputMap::new(tmp.join("input_map.json")).unwrap();
    for action in [
        "Move",
        "Attack",
        "Jump",
        "Interact",
        "Pause",
        "Select",
        "Command",
        "CameraPan",
    ] {
        assert!(input_map.bindings.contains_key(action));
    }
    input_map.add_binding("Attack", "keyboard:f").unwrap();
    input_map
        .set_action_binding("Move", 0, "keyboard:custom_move")
        .unwrap();
    input_map.remove_binding("Attack", "keyboard:f").unwrap();
    let infos = input_map.action_infos();
    assert!(infos.iter().any(|action| action.name == "CameraPan"));
    assert_eq!(input_map.bindings["Move"][0], "keyboard:custom_move");
}

#[test]
fn stability_config_recovers_corrupt_file_and_logger_writes_levels() {
    let tmp = temp_dir("stability-config");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    AssetTools::write_json(
        tmp.join("engine_config.json.bak"),
        &json!({
            "engine_name": "MiniForge",
            "engine_alt_name": "LegacyForge",
            "project_name": "Recovered",
            "start_scene": "main.scene"
        }),
    )
    .unwrap();
    fs::write(tmp.join("engine_config.json"), "{ broken json").unwrap();

    let config = EngineConfig::new(&tmp).unwrap();
    assert!(config.status.recovered_from_backup);
    assert_eq!(
        config
            .get("engine_alt_name")
            .and_then(|value| value.as_str()),
        Some("MiniForge")
    );
    assert!(tmp.join("engine_config.json.corrupt").exists());

    let logger = Logger::new(tmp.join("logs").join("miniforge.log"));
    logger
        .log_level(LogLevel::Info, "TEST", "developer stability")
        .unwrap();
    logger.warning("TEST", "recoverable warning").unwrap();
    let log_text = fs::read_to_string(tmp.join("logs").join("miniforge.log")).unwrap();
    assert!(log_text.contains("[info][TEST] developer stability"));
    assert!(log_text.contains("[warning][TEST] recoverable warning"));
}

#[test]
fn stability_editor_can_create_open_edit_save_and_run_luau_without_restart() {
    let tmp = temp_dir("stability-live-edit");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    let script_path = game.create_luau_script_asset("PlayerController").unwrap();
    game.edit_open_file(
        r#"
function on_start()
    set_position(3.0, 4.0)
end

function on_update(dt: number)
    move(2.0 * dt, 0.0)
end
"#,
    );
    assert!(game.save_open_file().unwrap());
    assert!(game.script_editor.document.syntax_error.is_none());

    let relative = script_path
        .strip_prefix(&tmp)
        .unwrap()
        .to_string_lossy()
        .to_string();
    game.units[0].scripts = vec![json!({"runtime": "luau", "path": relative})];
    game.enter_play_mode();
    game.run_headless_once(0.5);
    assert!(game.units[0].x > 3.0);
    assert!(game.luau_script_runtime.last_errors.is_empty());
}

#[test]
fn stability_create_empty_scene_opens_blank_viewport_immediately() {
    let tmp = temp_dir("stability-empty-scene");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    game.spawn_unit("TemporaryUnit", 8.0, 8.0);
    assert!(!game.units.is_empty());

    let path = game.create_empty_scene("BlankGameplay").unwrap();

    assert_eq!(game.units.len(), 0);
    assert_eq!(game.runtime_world.entities().len(), 0);
    assert!(game.selected_units.is_empty());
    assert_eq!(
        game.scene_manager.loaded_scenes,
        vec![game.scene_manager.current_scene.clone()]
    );
    assert_eq!(
        game.scene_manager.scene_stack,
        vec![game.scene_manager.current_scene.clone()]
    );
    let saved = AssetTools::read_json(path).unwrap();
    assert_eq!(
        saved
            .get("entities")
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        saved
            .get("ui_canvases")
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(0)
    );
}

#[test]
fn stability_visual_graph_errors_are_reported_without_panics() {
    let mut entity = GameObject::new(0.0, 0.0, Some("GraphHost".to_string()));
    let mut graph = default_component("VisualScript").unwrap();
    graph.set(
        "nodes",
        json!([
            {"id": "start", "type": "UnknownNode", "next": "missing"}
        ]),
    );
    entity.add_component(graph);
    let mut entities = vec![entity];
    let mut runtime = VisualScriptRuntime::default();
    runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
    assert!(!runtime.last_errors.is_empty());
    assert!(
        runtime
            .last_errors
            .iter()
            .any(|error| error.contains("UnknownNode") || error.contains("missing"))
    );
}

#[test]
fn stability_scenes_and_prefabs_validate_and_recover_from_backups() {
    let tmp = temp_dir("stability-scene-prefab");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let scene = tmp.join("saves").join("scenes").join("broken.scene");
    fs::write(&scene, "{ broken").unwrap();
    AssetTools::write_json(
        scene.with_extension("scene.bak"),
        &AssetTools::template_scene("broken"),
    )
    .unwrap();

    let mut validator = ProjectValidator::default();
    validator.validate(&tmp);
    assert!(
        validator
            .warnings
            .iter()
            .any(|warning| warning.contains("backup"))
    );

    let manager = PrefabManager::new(&tmp);
    let mut entity = GameObject::new(1.0, 2.0, Some("StablePrefab".to_string()));
    let prefab = manager
        .save_prefab(&mut entity, Some("StablePrefab"))
        .unwrap();
    fs::copy(&prefab, prefab.with_extension("prefab.bak")).unwrap();
    fs::write(&prefab, "{ broken").unwrap();
    let loaded = manager.load_prefab(&prefab).unwrap().unwrap();
    assert_eq!(loaded.name, "StablePrefab_Instance");
}

#[test]
fn update_091_project_packages_sprite_editor_and_console_are_connected() {
    let tmp = temp_dir("update-091-package");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(tmp.join("assets").join("data").join("settings.json"), "{}").unwrap();

    let archive = tmp.join("builds").join("package.mfpkg.zip");
    let report = ProjectPackageManager::export_project(&tmp, &archive).unwrap();
    assert!(archive.exists());
    assert!(report.files > 0);

    let import_root = temp_dir("update-091-package-import");
    let imported = ProjectPackageManager::import_project(&archive, &import_root).unwrap();
    assert!(imported.project_path.join("project.json").exists());
    assert!(
        imported
            .project_path
            .join("assets")
            .join("data")
            .join("settings.json")
            .exists()
    );

    let mut sprite = SpriteEditorCanvas::new(8, 8);
    let red = SpriteColor {
        r: 255,
        g: 32,
        b: 32,
        a: 255,
    };
    sprite.fill_rect(1, 1, 3, 3, red);
    sprite.draw_line((0, 0), (7, 7), SpriteColor::WHITE);
    let sprite_path = tmp.join("assets").join("sprites").join("painted.png");
    sprite.save_png(&sprite_path).unwrap();
    let loaded = SpriteEditorCanvas::load_png(&sprite_path).unwrap();
    assert_eq!(loaded.width, 8);
    assert_eq!(loaded.get_pixel(1, 2), Some(red));

    let mut console = DeveloperConsole::default();
    console.log("ready", "ENGINE");
    console.warning("careful", "WARNING");
    console.error("broken", "ERROR");
    let serious = console.search("", ConsoleSeverity::Warning);
    assert_eq!(serious.len(), 2);
    assert!(console.summary().contains("1 warnings"));
    console.clear_channel("ERROR");
    assert_eq!(console.error_count, 0);
}

#[test]
fn update_091_blueprint_flow_nodes_drive_gameplay_state() {
    let mut entity = GameObject::new(0.0, 0.0, Some("BlueprintActor".to_string()));
    let mut graph = default_component("VisualScript").unwrap();
    graph.set(
        "nodes",
        json!([
            {"id": "construction", "type": "ConstructionScript", "next": "set_pos"},
            {"id": "set_pos", "type": "SetPosition", "x": 7.0, "y": 9.0, "next": null},
            {"id": "start", "type": "EventStart", "next": "broadcast"},
            {"id": "broadcast", "type": "BroadcastEvent", "event": "OnReady", "next": "flip"},
            {"id": "on_ready", "type": "CustomEvent", "event": "OnReady", "next": "inventory"},
            {"id": "inventory", "type": "InventoryAdd", "item": "potion", "quantity": 2, "next": "cooldown"},
            {"id": "cooldown", "type": "StartCooldown", "name": "dash", "duration": 3.0, "next": null},
            {"id": "flip", "type": "FlipFlop", "key": "state", "a_next": "state_a", "b_next": "state_b"},
            {"id": "state_a", "type": "SetState", "state": "StateA", "next": null},
            {"id": "state_b", "type": "SetState", "state": "StateB", "next": null},
            {"id": "update", "type": "EventUpdate", "next": "gate"},
            {"id": "gate", "type": "Gate", "key": "ready", "open": true, "next": "move"},
            {"id": "move", "type": "Move", "x": 1.0, "y": 0.0, "use_dt": true, "next": null}
        ]),
    );
    graph.set("variables", json!({"ready": true}));
    entity.add_component(graph);
    let mut entities = vec![entity];
    let mut runtime = VisualScriptRuntime::default();

    runtime.update_entities(&mut entities, 0.5, "PLAY");
    assert_eq!(entities[0].x, 7.0);
    assert_eq!(entities[0].y, 9.0);
    assert_eq!(entities[0].state, "StateA");
    assert!(entities[0].get_component("Inventory").is_some());
    assert!(entities[0].get_component("Cooldown").is_some());

    runtime.update_entities(&mut entities, 0.5, "PLAY");
    assert!(entities[0].x > 7.0);

    let env = ProgrammingEnvironment::new();
    assert!(
        env.template_names()
            .contains(&"BlueprintCommunication".to_string())
    );
    assert!(
        env.search_node_catalog("flip")
            .iter()
            .any(|node| node.node_type == "FlipFlop")
    );
    assert!(
        env.search_node_catalog("broadcast")
            .iter()
            .any(|node| node.node_type == "BroadcastEvent")
    );
}

#[test]
fn update_091_hierarchy_and_prefab_actions_work_from_game_api() {
    let tmp = temp_dir("update-091-prefab-hierarchy");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let mut game = Game::from_project(&tmp, false).unwrap();
    let parent = game.spawn_game_object("Parent", 1.0, 1.0);
    let child = game.spawn_game_object("Child", 2.0, 2.0);

    assert!(game.set_entity_parent(child, parent));
    assert_eq!(
        game.get_entity_by_id(child).unwrap().parent_id,
        Some(parent)
    );
    assert!(game.move_entity_in_hierarchy(child, -1));
    assert!(game.clear_entity_parent(child));
    assert_eq!(game.get_entity_by_id(child).unwrap().parent_id, None);

    game.select_entity(child);
    let prefab = game.save_selected_as_prefab().unwrap().unwrap();
    assert!(prefab.exists());
    assert!(game.apply_selected_to_prefab_source().unwrap());
    assert!(game.detach_selected_prefab_instance());
    let entity = game.get_entity_by_id(child).unwrap();
    assert!(!entity.is_prefab_instance);
    assert!(entity.prefab_source.is_none());
}

#[test]
fn update_092_game_api_supports_inventory_economy_rts_quests_and_abilities() {
    let mut player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    let mut chest = GameObject::new(1.0, 0.0, Some("Chest".to_string()));
    assert_eq!(GameAPI::add_item(&mut player, "potion", 5), 5);
    assert!(GameAPI::has_item(&player, "potion", 3));
    assert_eq!(
        GameAPI::transfer_item(&mut player, &mut chest, "potion", 2),
        2
    );
    assert_eq!(GameAPI::item_count(&chest, "potion"), 2);
    assert!(GameAPI::equip_item(
        &mut player,
        "weapon",
        "iron_sword",
        json!({"attack": 4.0})
    ));
    assert!(player.get_component("Equipment").is_some());

    GameAPI::add_resources(&mut player, &json!({"Gold": 180.0, "Wood": 60.0}));
    assert!(GameAPI::can_afford(
        &player,
        &json!({"Gold": 80.0, "Wood": 20.0})
    ));
    assert!(GameAPI::spend_cost(
        &mut player,
        &json!({"Gold": 80.0, "Wood": 20.0})
    ));
    assert_eq!(GameAPI::resource_amount(&player, "Gold"), 100.0);

    let mut base = GameObject::new(3.0, 3.0, Some("Base".to_string()));
    GameAPI::add_resources(&mut base, &json!({"Gold": 250.0, "Wood": 90.0}));
    assert!(GameAPI::add_production_recipe(
        &mut base,
        "Soldier",
        "Soldier",
        5.0,
        json!({"Gold": 85.0, "Wood": 25.0})
    ));
    assert!(GameAPI::set_preferred_recipe(&mut base, "Soldier"));
    assert!(GameAPI::enqueue_preferred_recipe(&mut base));
    assert!(
        base.get_component("ProductionQueue")
            .and_then(|queue| queue.get("queue"))
            .and_then(|queue| queue.as_array())
            .is_some_and(|queue| queue.len() == 1)
    );

    let mut worker = GameObject::new(4.0, 3.0, Some("Worker".to_string()));
    let mut node = GameObject::new(5.0, 3.0, Some("GoldNode".to_string()));
    node.add_component(default_component("ResourceNode").unwrap());
    assert_eq!(GameAPI::gather_resource(&mut worker, &mut node, 20.0), 20.0);
    assert_eq!(GameAPI::deposit_worker_cargo(&mut worker, &mut base), 20.0);
    assert!(GameAPI::resource_amount(&base, "Gold") >= 185.0);

    assert!(GameAPI::add_quest(
        &mut player,
        "tutorial",
        "Tutorial",
        json!([{"id": "collect", "progress": 0, "target": 1}])
    ));
    assert!(GameAPI::set_quest_objective_progress(
        &mut player,
        "tutorial",
        "collect",
        json!(1)
    ));
    assert!(GameAPI::complete_quest(&mut player, "tutorial"));

    assert!(GameAPI::trigger_ability(&mut player, 10.0));
    assert!(!GameAPI::trigger_ability(&mut player, 10.1));
    assert!(GameAPI::recharge_ability(&mut player, 1));
    assert!(GameAPI::trigger_ability(&mut player, 11.2));
}

#[test]
fn update_092_blueprint_nodes_cover_complex_gameplay_programming() {
    let mut entity = GameObject::new(0.0, 0.0, Some("GameplayBlueprint".to_string()));
    let mut graph = default_component("VisualScript").unwrap();
    graph.set(
        "nodes",
        json!([
            {"id": "start", "type": "EventStart", "next": "gold"},
            {"id": "gold", "type": "EconomyAdd", "resource": "Gold", "amount": 150.0, "next": "has_gold"},
            {"id": "has_gold", "type": "BranchResource", "resource": "Gold", "amount": 100.0, "true_next": "item", "false_next": "failed"},
            {"id": "item", "type": "InventoryAdd", "item": "potion", "quantity": 2, "next": "has_item"},
            {"id": "has_item", "type": "BranchItem", "item": "potion", "quantity": 1, "true_next": "quest", "false_next": "failed"},
            {"id": "quest", "type": "AddQuest", "quest": "first_steps", "title": "First Steps", "objectives": [{"id": "collect", "progress": 0, "target": 1}], "next": "progress"},
            {"id": "progress", "type": "QuestProgress", "quest": "first_steps", "objective": "collect", "progress": 1, "next": "ability"},
            {"id": "ability", "type": "TriggerAbility", "true_next": "fired", "false_next": "recharge"},
            {"id": "fired", "type": "SetState", "state": "AbilityFired", "next": null},
            {"id": "recharge", "type": "RechargeAbility", "amount": 1, "next": null},
            {"id": "failed", "type": "SetState", "state": "MissingResources", "next": null}
        ]),
    );
    graph.set("variables", json!({}));
    entity.add_component(graph);
    let mut entities = vec![entity];
    let mut runtime = VisualScriptRuntime::default();

    runtime.update_entities(&mut entities, 0.25, "PLAY");
    assert!(runtime.last_errors.is_empty());
    assert_eq!(entities[0].state, "AbilityFired");
    assert_eq!(GameAPI::resource_amount(&entities[0], "Gold"), 150.0);
    assert_eq!(GameAPI::item_count(&entities[0], "potion"), 2);
    assert!(entities[0].get_component("QuestLog").is_some());
    assert!(entities[0].get_component("Ability").is_some());

    let env = ProgrammingEnvironment::new();
    assert!(
        env.search_node_catalog("inventry")
            .iter()
            .any(|node| node.node_type == "InventoryAdd")
    );
    assert!(
        env.search_node_catalog("economy spend")
            .iter()
            .any(|node| node.node_type == "EconomySpend")
    );
    assert!(
        env.search_templates("quest abilty")
            .iter()
            .any(|template| template.name == "QuestAbilityLoop")
    );
}
