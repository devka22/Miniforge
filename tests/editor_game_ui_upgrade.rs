use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::game_api::GameAPI;
use miniforge::engine::luau_scripting::LuauScriptRuntime;
use miniforge::engine::miniforge_2d::blueprint::{
    BlueprintFunction2D, BlueprintGraph2D, BlueprintPin2D, graph_from_value, search_node_palette,
    search_node_palette_ranked,
};
use miniforge::engine::miniforge_2d::content_browser::{
    ContentBrowserCatalog2D, ContentFilter2D, ContentSortMode2D,
};
use miniforge::engine::miniforge_2d::details_inspector::DetailsInspector2D;
use miniforge::engine::miniforge_2d::editor_layout::EditorLayout2D;
use miniforge::engine::miniforge_2d::ui_designer::UiDesigner2D;
use miniforge::engine::miniforge_2d::ui_framework::{
    dialogue_screen_canvas, game_over_screen_canvas, inventory_screen_canvas,
    level_select_screen_canvas, main_menu_canvas, pause_menu_canvas, settings_screen_canvas,
    standard_screen_manager, supported_screen_types, supported_widget_types,
};
use miniforge::engine::miniforge_2d::world_outliner::WorldOutliner2D;
use miniforge::engine::ui_runtime::{UiEventKind, UiRuntime};
use miniforge::entities::game_object::GameObject;
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-ui-upgrade-{name}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn game_ui_designer_builds_unreal_style_menu_screens() {
    let menu = main_menu_canvas("Iron Skies");
    assert_eq!(menu.name, "MainMenu");
    assert!(menu.find_widget("MenuPanel").is_some());
    assert_eq!(menu.callbacks_for_event("OnClick").len(), 4);
    assert!(menu.validate_navigation_links().is_empty());
    assert_eq!(menu.widget_type_counts()["MenuButton"], 4);

    let pause = pause_menu_canvas();
    assert!(pause.find_widget("ResumeButton").is_some());

    let mut designer = UiDesigner2D::main_menu("Iron Skies");
    assert!(designer.validate().is_valid());
    assert!(!designer.palette_search("inventory").is_empty());
    assert!(designer.create_widget_from_palette("ProgressBar", "LoadingBar", 32.0, 512.0));
    assert!(designer.select("LoadingBar"));
    assert!(designer.duplicate_selected("LoadingBarCopy"));
    assert!(designer.align_selected("center_x"));
    assert!(
        designer
            .hierarchy_rows()
            .iter()
            .any(|(id, kind, _)| id == "LoadingBarCopy" && kind == "ProgressBar")
    );
}

#[test]
fn screen_manager_exposes_complete_game_ui_stack() {
    let widget_types = supported_widget_types();
    for widget_type in [
        "Canvas",
        "Panel",
        "Button",
        "Label",
        "Image",
        "ProgressBar",
        "Slider",
        "Checkbox",
        "TextInput",
        "InventoryGrid",
        "DialogueBox",
    ] {
        assert!(widget_types.contains(&widget_type), "{widget_type}");
    }

    let screen_types = supported_screen_types();
    for screen_type in [
        "UIScreen",
        "ScreenManager",
        "MainMenuScreen",
        "PauseScreen",
        "SettingsScreen",
        "HUDScreen",
        "InventoryScreen",
        "DialogueScreen",
        "GameOverScreen",
        "LevelSelectScreen",
    ] {
        assert!(screen_types.contains(&screen_type), "{screen_type}");
    }

    let mut manager = standard_screen_manager("Iron Skies");
    assert!(manager.validate().is_empty());
    assert!(manager.screens.contains_key("HUDScreen"));
    assert!(manager.screens.contains_key("MainMenuScreen"));
    assert!(manager.gameplay_blocked());
    assert_eq!(
        manager.command_for_widget("MainMenuScreen", "ContinueButton"),
        Some("continue_game".to_string())
    );
    assert!(!manager.close_screen("HUDScreen"));
    assert!(manager.open_screen("InventoryScreen"));
    assert_eq!(manager.top_screen().unwrap().id, "InventoryScreen");
    assert!(manager.active_canvases().len() >= 3);

    assert!(
        settings_screen_canvas()
            .find_widget("MasterVolumeSlider")
            .is_some()
    );
    assert!(
        inventory_screen_canvas()
            .find_widget("InventoryGrid")
            .is_some()
    );
    assert!(
        dialogue_screen_canvas()
            .find_widget("DialogueNextButton")
            .is_some()
    );
    assert!(
        game_over_screen_canvas()
            .find_widget("RetryButton")
            .is_some()
    );
    assert!(
        level_select_screen_canvas()
            .find_widget("LevelTwoButton")
            .is_some()
    );

    let settings = settings_screen_canvas();
    let mut runtime = UiRuntime::default();
    let events = runtime.update_miniforge_canvas_interaction(
        &settings,
        (1280.0, 720.0),
        Some((420.0, 260.0)),
        true,
    );
    assert!(events.iter().any(|event| {
        event.kind == UiEventKind::Click
            && event.element_id == "MasterVolumeSlider"
            && event.command.as_deref() == Some("set_master_volume")
    }));
}

#[test]
fn editor_main_menu_blueprint_palette_and_layout_are_connected() {
    let layout = EditorLayout2D::default();
    assert!(layout.validate().is_valid());
    assert!(
        layout
            .command_palette_entries()
            .iter()
            .any(|entry| entry.contains("create.ui.main_menu"))
    );
    assert!(
        layout
            .visible_panels_by_region("bottom")
            .iter()
            .any(|panel| panel.id == "content_browser")
    );

    let palette = search_node_palette("menu");
    assert!(palette.iter().any(|item| item.kind == "OpenMenu"));
    assert!(palette.iter().any(|item| item.kind == "CreateMainMenu"));

    let mut graph = BlueprintGraph2D::default();
    let menu_node = graph
        .add_node(
            "OpenMenu",
            "Open Main Menu",
            600.0,
            20.0,
            json!({"menu": "MainMenu"}),
        )
        .unwrap();
    assert!(graph.connect_nodes("print_ready", "then", &menu_node, "exec"));
    graph.auto_layout();
    let summary = graph.compile_summary();
    assert!(summary.runtime_ready);
    assert!(summary.node_count >= 3);
}

#[test]
fn blueprint_graph_analysis_tracks_flow_and_editor_actions() {
    let mut graph = BlueprintGraph2D::default();
    let orphan = graph
        .add_node(
            "PrintString",
            "Loose Debug",
            500.0,
            240.0,
            json!({"message": "loose"}),
        )
        .unwrap();
    let getter = graph
        .add_node(
            "GetVariable",
            "Read Speed",
            320.0,
            160.0,
            json!({"name": "MoveSpeed"}),
        )
        .unwrap();
    assert!(graph.connect_nodes("print_ready", "then", &getter, "exec"));

    let analysis = graph.analyze();
    assert!(analysis.reachable_node_ids.contains(&getter));
    assert!(analysis.orphan_node_ids.contains(&orphan));
    assert!(analysis.variable_reads.contains(&"MoveSpeed".to_string()));
    assert!(
        analysis
            .recommended_actions
            .contains(&"connect_or_remove_orphans".to_string())
    );

    let duplicate = graph.duplicate_node(&getter, 30.0, 30.0).unwrap();
    assert!(graph.node_by_id(&duplicate).is_some());
    assert!(graph.remove_node(&orphan));
    assert!(!graph.analyze().orphan_node_ids.contains(&orphan));

    let ranked = search_node_palette_ranked("menu");
    assert!(ranked.first().is_some_and(|result| result.score < 50));
}

#[test]
fn blueprint_graph_supports_unreal_like_authoring_stack() {
    let mut graph = BlueprintGraph2D::default();
    assert_eq!(graph.asset_kind, "BlueprintClass");
    assert_eq!(graph.parent_class, "Actor2D");
    assert_eq!(graph.graph_type, "EventGraph");
    assert!(graph.add_component("Sprite", "SpriteRenderer2D", "Root"));
    assert!(graph.add_variable_with_properties("Health", "float", json!(100.0), true, "Combat"));
    assert!(graph.add_function(
        "GetHealthPercent",
        vec![BlueprintPin2D {
            name: "max_health".to_string(),
            pin_type: "float".to_string(),
            direction: "in".to_string(),
            ..Default::default()
        }],
        vec![BlueprintPin2D {
            name: "percent".to_string(),
            pin_type: "float".to_string(),
            direction: "out".to_string(),
            ..Default::default()
        }],
        true,
    ));
    assert!(graph.add_macro(
        "GateOnce",
        vec![BlueprintPin2D {
            name: "exec".to_string(),
            pin_type: "exec".to_string(),
            direction: "in".to_string(),
            ..Default::default()
        }],
        vec![BlueprintPin2D {
            name: "then".to_string(),
            pin_type: "exec".to_string(),
            direction: "out".to_string(),
            ..Default::default()
        }],
    ));
    assert!(graph.add_event_dispatcher(
        "OnDamaged",
        vec![BlueprintPin2D {
            name: "amount".to_string(),
            pin_type: "float".to_string(),
            direction: "in".to_string(),
            ..Default::default()
        }],
    ));
    assert!(graph.implement_interface(
        "BPI_Interactable",
        vec![BlueprintFunction2D {
            inputs: vec![BlueprintPin2D {
                name: "instigator".to_string(),
                pin_type: "object".to_string(),
                direction: "in".to_string(),
                ..Default::default()
            }],
            access: "Public".to_string(),
            category: "Interface".to_string(),
            ..Default::default()
        }],
    ));
    let cast = graph
        .add_node(
            "CastTo",
            "Cast To Enemy",
            320.0,
            120.0,
            json!({"class": "Enemy"}),
        )
        .unwrap();
    assert!(graph.promote_pin_to_variable(&cast, "as_type", "EnemyRef"));
    let branch = graph
        .add_node("Branch", "Can Damage?", 520.0, 120.0, json!({}))
        .unwrap();
    assert!(
        graph
            .connect_nodes_checked("begin_play", "then", &branch, "exec")
            .unwrap()
    );
    assert!(
        graph
            .connect_nodes_checked("begin_play", "then", &branch, "condition")
            .is_err()
    );

    let analysis = graph.analyze();
    assert_eq!(analysis.component_count, 2);
    assert_eq!(analysis.dispatcher_count, 1);
    assert_eq!(analysis.macro_count, 1);
    assert_eq!(analysis.interface_count, 1);
    let parity = graph.unreal_parity_summary();
    assert_eq!(parity.pure_functions, 1);
    assert_eq!(parity.dispatcher_count, 1);
    assert!(
        !parity
            .missing_unreal_like_features
            .contains(&"interfaces".to_string())
    );
}

#[test]
fn blueprint_graph_deserializes_older_assets_with_unreal_defaults() {
    let graph = graph_from_value(&json!({
        "name": "BP_Old",
        "runtime": "miniforge_visual_script_2d",
        "nodes": [
            {"id": "begin", "kind": "EventBeginPlay", "title": "Begin", "pins": [{"name": "then", "pin_type": "exec", "direction": "out"}]}
        ],
        "edges": []
    }))
    .unwrap();

    assert_eq!(graph.asset_kind, "BlueprintClass");
    assert_eq!(graph.parent_class, "Actor2D");
    assert_eq!(graph.graph_type, "EventGraph");
    assert!(graph.validate().is_valid());
}

#[test]
fn browser_outliner_and_details_panel_are_cleaner_and_more_actionable() {
    let mut catalog = ContentBrowserCatalog2D::default();
    catalog.insert_json_asset(
        "assets/ui/main_menu.mfui",
        "DataAsset2D",
        json!({"size_bytes": 100}),
    );
    catalog.insert_json_asset(
        "scripts/visual_graphs/MainMenu.mfgraph",
        "BlueprintGraph2D",
        json!({"size_bytes": 200}),
    );
    catalog.insert_json_asset(
        "assets/prefabs/player.prefab",
        "Prefab2D",
        json!({"size_bytes": 50}),
    );

    let tree = catalog.folder_tree();
    assert!(tree.children.contains_key("assets"));
    assert!(tree.children.contains_key("scripts"));

    let filtered = catalog.filter_sorted(
        &ContentFilter2D {
            folder: Some("assets".to_string()),
            include_invalid: true,
            ..Default::default()
        },
        ContentSortMode2D::Path,
    );
    assert_eq!(filtered.len(), 2);
    assert!(
        ContentBrowserCatalog2D::quick_actions_for(filtered[0])
            .iter()
            .any(|action| action.contains("open"))
    );

    let parent = GameObject::new(0.0, 0.0, Some("MenuRoot".to_string()));
    let mut child = GameObject::new(1.0, 2.0, Some("StartButton".to_string()));
    child.parent_id = Some(parent.id);
    child.tag = "UI".to_string();
    child.layer = "UI".to_string();
    let mut entities = vec![parent.clone(), child.clone()];
    let mut outliner = WorldOutliner2D::from_entities(&entities);
    outliner.select(child.id, false);
    let summary = outliner.summary();
    assert_eq!(summary.total, 2);
    assert_eq!(summary.selected, 1);
    assert_eq!(summary.by_layer["UI"], 1);
    assert!(
        outliner
            .context_actions_for(parent.id)
            .contains(&"delete_recursive")
    );
    assert_eq!(
        WorldOutliner2D::set_visible_recursive(&mut entities, parent.id, false),
        2
    );

    let details = DetailsInspector2D::from_entity(&child);
    assert!(details.section_titles().contains(&"Transform".to_string()));
    assert!(!details.search_fields("layer").is_empty());
}

#[test]
fn luau_gameplay_api_can_drive_ui_state_inventory_resources_and_components() {
    let tmp = temp_dir("luau-ui-api");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let script = AssetTools::create_luau_script(&tmp, "MenuController").unwrap();
    fs::write(
        &script,
        r#"
function on_start()
    set_tag("Player")
    set_layer("Gameplay")
    set_visible(true)
    ui_text("Ready")
    set_ui_progress(4.0, 10.0)
    set_component_number("Transform", "x", 12.0)
    set_component_text("Transform", "debug_label", "from luau")
    add_item("potion", 2)
    add_resource("Gold", 15.0)
end
"#,
    )
    .unwrap();

    let mut entity = GameObject::new(0.0, 0.0, Some("MenuActor".to_string()));
    entity.script = Some("MenuController.luau".to_string());
    let mut entities = vec![entity];

    let mut runtime = LuauScriptRuntime::new(&tmp);
    let report = runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(report.commands_applied >= 8);
    assert_eq!(entities[0].tag, "Player");
    assert_eq!(entities[0].layer, "Gameplay");
    assert_eq!(entities[0].x, 12.0);
    assert_eq!(GameAPI::item_count(&entities[0], "potion"), 2);
    assert_eq!(GameAPI::resource_amount(&entities[0], "Gold"), 15.0);
    let ui = entities[0].get_component("UIElement").unwrap();
    assert_eq!(ui.get_f64("progress", 0.0), 4.0);
    assert_eq!(ui.get_f64("max_progress", 0.0), 10.0);
}

#[test]
fn luau_sprite_helpers_update_sprite_renderer_for_game_scripts() {
    let tmp = temp_dir("luau-sprite-api");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    let script = AssetTools::create_luau_script(&tmp, "SpriteController").unwrap();
    fs::write(
        &script,
        r#"
function on_start()
    set_sprite("assets/sprites/LoveLabCharacters.sprite.json")
    play_sprite_animation("assets/animations/LoveLabCharacters.spriteframes", "idle")
    face_left()
end
"#,
    )
    .unwrap();

    let mut entity = GameObject::new(0.0, 0.0, Some("Sol".to_string()));
    entity.script = Some("SpriteController.luau".to_string());
    let mut entities = vec![entity];

    let mut runtime = LuauScriptRuntime::new(&tmp);
    let report = runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(report.commands_applied >= 3);
    let sprite = entities[0].get_component("SpriteRenderer").unwrap();
    assert_eq!(
        sprite.get("sprite_path").and_then(|value| value.as_str()),
        Some("assets/sprites/LoveLabCharacters.sprite.json")
    );
    assert_eq!(
        sprite.get("sprite_frames").and_then(|value| value.as_str()),
        Some("assets/animations/LoveLabCharacters.spriteframes")
    );
    assert_eq!(
        sprite
            .get("active_animation")
            .and_then(|value| value.as_str()),
        Some("idle")
    );
    assert_eq!(
        sprite.get("flip_x").and_then(|value| value.as_bool()),
        Some(true)
    );
}
