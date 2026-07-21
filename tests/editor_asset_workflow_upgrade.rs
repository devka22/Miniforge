#![cfg(feature = "editor_core")]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use image::{ImageBuffer, Rgb, Rgba};
use miniforge::core::game::Game;
use miniforge::engine::asset_database::AssetDatabase;
use miniforge::engine::asset_importers::SpriteSheetImporter;
use miniforge::engine::content_drag::{DragAssetKind, DragPayload, DropOutcome};
use miniforge::engine::editor_asset_connector::EditorAssetConnector;
use miniforge::engine::editor_command::EditorCommandKind;
use miniforge::engine::inspector_editor::InspectorEditor;
use miniforge::engine::miniforge_2d::blueprint::BlueprintGraph2D;
use miniforge::engine::miniforge_2d::content_browser::ContentBrowserCatalog2D;
use miniforge::engine::miniforge_2d::details_inspector::DetailsInspector2D;
use miniforge::engine::miniforge_2d::ui_designer::UiDesigner2D;
use miniforge::engine::packaging_manager::{
    InstallerPlatform, InstallerSigningConfig, PackagingManager,
};
use miniforge::engine::scene_view_tools::SceneViewTools;
use miniforge::engine::script_editor::ScriptEditor;
use miniforge::engine::sprite_editor::{SpriteColor, SpriteEditorCanvas};
use miniforge::engine::ui_canvas::{UiCanvasGizmoHandleKind, UiCanvasRoot, ui_canvases_from_value};
use miniforge::entities::game_object::GameObject;
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-editor-workflow-{name}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn sprite_sheet_importer_reads_jpeg_and_webp_metadata() {
    let tmp = temp_dir("sheet-formats");
    let jpg = tmp.join("sheet.jpg");
    let webp = tmp.join("sheet.webp");
    let rgb = ImageBuffer::from_pixel(4, 2, Rgb([220u8, 40, 80]));
    rgb.save(&jpg).unwrap();
    let rgba = ImageBuffer::from_pixel(4, 2, Rgba([30u8, 160, 240, 255]));
    rgba.save(&webp).unwrap();

    assert!(SpriteSheetImporter::supports_image(&jpg));
    assert!(SpriteSheetImporter::supports_image(&webp));
    let jpg_meta = SpriteSheetImporter::build_metadata(&jpg, 2, 1, 0, 0).unwrap();
    let webp_meta = SpriteSheetImporter::build_metadata(&webp, 2, 1, 0, 0).unwrap();
    assert_eq!(jpg_meta.source_format, "jpg");
    assert_eq!(webp_meta.source_format, "webp");
    assert_eq!(jpg_meta.slices.len(), 4);
    assert_eq!(webp_meta.slices.len(), 4);
    assert!(!jpg_meta.import_warnings.is_empty());
}

#[test]
fn scene_view_can_select_move_resize_and_edit_ui_canvas_elements() {
    let mut root = UiCanvasRoot::default_hud();
    let tools = SceneViewTools {
        grid_snapping: true,
        snap_size: 8.0,
        ..Default::default()
    };
    let hit = tools
        .select_ui_element_at(&root, 1920.0, 1080.0, (30.0, 30.0))
        .unwrap();
    assert_eq!(hit, "label_hint");
    let move_report = tools.drag_ui_element(&mut root, &hit, 1920.0, 1080.0, 16.0, 24.0);
    assert!(move_report.changed);
    assert_eq!(
        root.find_element("label_hint").unwrap().rect().offset_x,
        40.0
    );
    let resize_report = tools.resize_ui_element(&mut root, "label_hint", 260.0, 48.0);
    assert!(resize_report.changed);
    let gizmo = root
        .gizmo_for_element("label_hint", 1920.0, 1080.0)
        .expect("selected UI element should expose resize handles");
    assert_eq!(gizmo.handles.len(), 8);
    assert!(
        gizmo
            .handles
            .iter()
            .any(|handle| handle.kind == UiCanvasGizmoHandleKind::BottomRight)
    );
    let handle = root
        .hit_test_gizmo_handle(1920.0, 1080.0, (304.0, 96.0))
        .expect("bottom-right handle should be hittable");
    assert_eq!(handle.kind, UiCanvasGizmoHandleKind::BottomRight);
    let handle_resize = tools.resize_ui_element_from_handle(
        &mut root,
        "label_hint",
        UiCanvasGizmoHandleKind::BottomRight,
        1920.0,
        1080.0,
        40.0,
        16.0,
    );
    assert!(handle_resize.changed);
    assert_eq!(root.find_element("label_hint").unwrap().rect().width, 304.0);
    assert!(root.set_element_text("label_hint", "HUD listo"));
}

#[test]
fn editor_history_restores_ui_canvas_edits() {
    let tmp = temp_dir("ui-history");
    let mut game = Game::from_project(&tmp, false).unwrap();
    game.ensure_default_ui_canvas_scene_data();
    let before = game.capture_editor_snapshot();

    let mut roots = ui_canvases_from_value(&game.ui_canvases);
    roots[0].move_element("label_hint", 32.0, 0.0, None);
    game.ui_canvases = serde_json::to_value(&roots).unwrap();
    game.mark_scene_dirty("Edit UI Canvas");
    game.push_editor_command(
        "Edit UI Canvas",
        EditorCommandKind::SceneOperation {
            name: "UI Canvas Test".to_string(),
        },
        before,
    );

    let moved = ui_canvases_from_value(&game.ui_canvases);
    assert_eq!(
        moved[0].find_element("label_hint").unwrap().rect().offset_x,
        56.0
    );
    assert!(game.undo_editor_command().is_some());
    let reverted = ui_canvases_from_value(&game.ui_canvases);
    assert_eq!(
        reverted[0]
            .find_element("label_hint")
            .unwrap()
            .rect()
            .offset_x,
        24.0
    );
    assert!(game.redo_editor_command().is_some());
    let redone = ui_canvases_from_value(&game.ui_canvases);
    assert_eq!(
        redone[0]
            .find_element("label_hint")
            .unwrap()
            .rect()
            .offset_x,
        56.0
    );
}

#[test]
fn complex_foundations_action_builds_player_systems_ui_and_undo() {
    let tmp = temp_dir("complex-foundations");
    let mut game = Game::from_project(&tmp, false).unwrap();
    let initial_units = game.units.len();

    let report = game.prepare_complex_game_foundations();

    assert!(report.changed);
    assert!(report.created_player);
    assert!(report.created_systems_entity);
    assert!(report.created_ui_canvas);
    assert_eq!(game.selected_units, vec![report.target_entity_id]);
    assert!(
        report
            .added_player_components
            .contains(&"CharacterController2D".to_string())
    );
    assert!(
        report
            .added_player_components
            .contains(&"QuestLog".to_string())
    );
    assert!(
        report
            .added_system_components
            .contains(&"RuntimeBudget2D".to_string())
    );
    assert!(
        report
            .added_system_components
            .contains(&"SaveShard2D".to_string())
    );

    let player = game.get_entity_by_id(report.target_entity_id).unwrap();
    assert_eq!(player.tag, "Player");
    assert_eq!(player.layer, "Units");
    assert!(player.get_component("Health").is_some());
    assert!(player.get_component("Inventory").is_some());
    assert!(player.get_component("Ability").is_some());
    assert!(player.get_component("VisualScript").is_some());
    assert_eq!(
        player
            .get_component("Saveable")
            .unwrap()
            .get("save_key")
            .and_then(|value| value.as_str()),
        Some("player")
    );

    let systems = game
        .get_entity_by_id(report.systems_entity_id.unwrap())
        .unwrap();
    assert_eq!(systems.name, "GameSystems");
    assert_eq!(systems.tag, "System");
    assert!(systems.locked);
    assert!(!systems.visible);
    assert!(systems.get_component("WorldPartition2D").is_some());
    assert!(systems.get_component("ObjectPool2D").is_some());
    assert!(systems.get_component("SpawnDirector2D").is_some());
    assert_eq!(
        systems
            .get_component("SpawnDirector2D")
            .unwrap()
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert!(!ui_canvases_from_value(&game.ui_canvases).is_empty());

    assert!(game.undo_editor_command().is_some());
    assert_eq!(game.units.len(), initial_units);
    assert!(game.get_entity_by_id(report.target_entity_id).is_none());
    assert!(ui_canvases_from_value(&game.ui_canvases).is_empty());
}

#[test]
fn packaging_writes_signed_installer_plans_per_platform() {
    let tmp = temp_dir("installer-plan");
    let mac_plan = PackagingManager::write_installer_plan(
        &tmp,
        "Iron Skies",
        InstallerPlatform::Macos,
        InstallerSigningConfig {
            identity: Some("Developer ID Application: MiniForge".to_string()),
            team_id: Some("TEAM12345".to_string()),
            notarize: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(mac_plan.signing_ready);
    assert!(mac_plan.commands.iter().any(|cmd| cmd.contains("codesign")));
    assert!(
        mac_plan
            .commands
            .iter()
            .any(|cmd| cmd.contains("notarytool"))
    );
    assert!(tmp.join("installer_manifest.json").exists());

    let win_plan = PackagingManager::write_installer_plan(
        &tmp,
        "Iron Skies",
        InstallerPlatform::Windows,
        InstallerSigningConfig::default(),
    )
    .unwrap();
    assert!(!win_plan.signing_ready);
    assert!(win_plan.commands.iter().any(|cmd| cmd.contains("signtool")));
    assert!(!win_plan.warnings.is_empty());
}

#[test]
fn content_browser_inspector_and_connector_assign_assets_to_scene_objects() {
    let mut catalog = ContentBrowserCatalog2D::default();
    catalog.insert_json_asset(
        "assets/sprites/hero.webp",
        "Sprite2D",
        json!({"size_bytes": 128}),
    );
    catalog.insert_json_asset(
        "assets/animations/ABP_Hero.anim2d.json",
        "AnimationBlueprint2D",
        json!({}),
    );
    catalog.insert_json_asset(
        "scripts/visual_graphs/BP_Hero.mfgraph",
        "BlueprintGraph2D",
        json!({}),
    );
    catalog.insert_json_asset(
        "assets/materials/hero.material.json",
        "Material2D",
        json!({}),
    );
    catalog.insert_json_asset("assets/textures/hero_normal.png", "Texture2D", json!({}));
    assert!(catalog.select_asset("assets/sprites/hero.webp"));
    let drag = catalog
        .drag_payload_for_scene("assets/sprites/hero.webp")
        .unwrap();
    assert_eq!(drag.drop_action, "assign_sprite");
    assert!(drag.can_spawn);
    assert!(drag.can_assign_to_selection);
    assert_eq!(drag.target_component.as_deref(), Some("SpriteRenderer"));
    assert!(
        catalog
            .drop_intents_for_selection("assets/sprites/hero.webp", 2)
            .iter()
            .any(|intent| intent.drop_action == "assign_sprite_to_selection")
    );

    let mut entity = GameObject::new(0.0, 0.0, Some("Hero".to_string()));
    let sprite_asset = catalog.selected_asset().unwrap().clone();
    let report = EditorAssetConnector::apply_content_asset(&mut entity, &sprite_asset);
    assert!(
        report
            .updated_fields
            .contains(&"SpriteRenderer.sprite_path".to_string())
    );
    assert_eq!(
        entity
            .get_component("SpriteRenderer")
            .unwrap()
            .get("sprite_path")
            .and_then(|value| value.as_str()),
        Some("assets/sprites/hero.webp")
    );

    let animation = catalog
        .assets
        .get("assets/animations/ABP_Hero.anim2d.json")
        .unwrap()
        .clone();
    let animation_report = EditorAssetConnector::apply_content_asset(&mut entity, &animation);
    assert!(
        animation_report
            .updated_fields
            .contains(&"Animator2D.animation_blueprint".to_string())
    );

    let graph = catalog
        .assets
        .get("scripts/visual_graphs/BP_Hero.mfgraph")
        .unwrap()
        .clone();
    let graph_report = EditorAssetConnector::apply_content_asset(&mut entity, &graph);
    assert!(graph_report.updated_fields.contains(&"scripts".to_string()));

    let material = catalog
        .assets
        .get("assets/materials/hero.material.json")
        .unwrap()
        .clone();
    let material_report = EditorAssetConnector::apply_content_asset(&mut entity, &material);
    assert!(
        material_report
            .updated_fields
            .contains(&"Material2D.material_path".to_string())
    );
    let texture = catalog
        .assets
        .get("assets/textures/hero_normal.png")
        .unwrap()
        .clone();
    let texture_report = EditorAssetConnector::apply_content_asset(&mut entity, &texture);
    assert!(
        texture_report
            .updated_fields
            .contains(&"Material2D.normal_texture".to_string())
    );
    assert_eq!(
        entity
            .get_component("Material2D")
            .unwrap()
            .get("normal_texture")
            .and_then(|value| value.as_str()),
        Some("assets/textures/hero_normal.png")
    );

    let inspector = DetailsInspector2D::from_entity(&entity);
    assert!(
        inspector
            .asset_picker_for("components.SpriteRenderer.sprite_path")
            .is_some()
    );
    assert!(
        inspector
            .asset_picker_for("components.Material2D.normal_texture")
            .is_some()
    );
    assert!(
        inspector
            .recommended_actions
            .contains(&"assign_animation_blueprint".to_string())
    );
    assert_eq!(inspector.target_kind, "entity");
    assert!(
        inspector
            .editable_paths()
            .iter()
            .any(|path| path == "components.SpriteRenderer.sprite_path")
    );

    let scene_inspector = DetailsInspector2D::from_scene("main.scene", &[entity.clone()]);
    assert_eq!(scene_inspector.target_kind, "scene");
    assert!(scene_inspector.section("Components").is_some());
    assert!(
        scene_inspector
            .search_fields("SpriteRenderer")
            .iter()
            .any(|field| field.path == "components.SpriteRenderer")
    );
}

#[test]
fn asset_database_marks_texture_folder_images_as_texture2d() {
    let tmp = temp_dir("asset-textures");
    let assets = tmp.join("assets");
    let sprites = assets.join("sprites");
    let textures = assets.join("textures");
    fs::create_dir_all(&sprites).unwrap();
    fs::create_dir_all(&textures).unwrap();
    fs::write(sprites.join("hero.png"), b"png").unwrap();
    fs::write(textures.join("hero_normal.png"), b"png").unwrap();

    let mut database = AssetDatabase::new(&assets, &tmp).unwrap();
    database.scan().unwrap();
    assert_eq!(
        database
            .assets
            .get("assets/sprites/hero.png")
            .unwrap()
            .asset_type,
        "Sprite"
    );
    let normal = database
        .assets
        .get("assets/textures/hero_normal.png")
        .unwrap();
    assert_eq!(normal.asset_type, "Texture2D");
    assert_eq!(
        normal
            .import_settings
            .get("slot_hint")
            .and_then(|value| value.as_str()),
        Some("normal")
    );
    assert!(normal.labels.contains(&"material-texture".to_string()));
}

#[test]
fn content_drag_applies_to_selection_and_spawns_when_no_selection() {
    let tmp = temp_dir("content-drag-fullstack");
    let mut game = Game::from_project(&tmp, false).unwrap();
    game.clear_selection();
    let target_id = game.spawn_game_object("DropTarget", 2.0, 3.0);
    let script_payload = DragPayload {
        relative_path: "scripts/PlayerController.luau".to_string(),
        name: "PlayerController".to_string(),
        asset_type: "LuauScript".to_string(),
        guid: "script-guid".to_string(),
        kind: DragAssetKind::Script,
    };

    let before_count = game.units.len();
    let outcome = game.drop_asset_to_scene(&script_payload, 8.0, 9.0).unwrap();
    assert_eq!(outcome, DropOutcome::AppliedToEntity(target_id));
    assert_eq!(game.units.len(), before_count);
    let target = game.get_entity_by_id(target_id).unwrap();
    assert_eq!(
        target
            .get_component("ScriptComponent")
            .and_then(|component| component.get("path"))
            .and_then(|value| value.as_str()),
        Some("scripts/PlayerController.luau")
    );

    let material_payload = DragPayload {
        relative_path: "assets/materials/water.material.json".to_string(),
        name: "water".to_string(),
        asset_type: "Material".to_string(),
        guid: "material-guid".to_string(),
        kind: DragAssetKind::Material,
    };
    let outcome = game
        .drop_asset_to_scene(&material_payload, 8.0, 9.0)
        .unwrap();
    assert_eq!(outcome, DropOutcome::AppliedToEntity(target_id));
    assert!(
        game.get_entity_by_id(target_id)
            .unwrap()
            .get_component("Material2D")
            .is_some()
    );

    let texture_payload = DragPayload {
        relative_path: "assets/textures/water_normal.png".to_string(),
        name: "water_normal".to_string(),
        asset_type: "Texture2D".to_string(),
        guid: "texture-guid".to_string(),
        kind: DragAssetKind::Texture,
    };
    let preview = texture_payload.preview();
    assert!(preview.detail.contains("normal_texture"));
    assert!(
        preview
            .compatible_targets
            .contains(&"Actor.Material2D.normal_texture".to_string())
    );
    let outcome = game
        .drop_asset_to_scene(&texture_payload, 8.0, 9.0)
        .unwrap();
    assert_eq!(outcome, DropOutcome::AppliedToEntity(target_id));
    assert_eq!(
        game.get_entity_by_id(target_id)
            .unwrap()
            .get_component("Material2D")
            .unwrap()
            .get("normal_texture")
            .and_then(|value| value.as_str()),
        Some("assets/textures/water_normal.png")
    );

    game.clear_selection();
    let particle_payload = DragPayload {
        relative_path: "assets/fx/spark.particles.json".to_string(),
        name: "spark".to_string(),
        asset_type: "ParticlePreset".to_string(),
        guid: "particle-guid".to_string(),
        kind: DragAssetKind::ParticlePreset,
    };
    let outcome = game
        .drop_asset_to_scene(&particle_payload, 10.0, 11.0)
        .unwrap();
    let DropOutcome::SpawnedEntity(spawned_id) = outcome else {
        panic!("particle payload should spawn an entity without selection");
    };
    let spawned = game.get_entity_by_id(spawned_id).unwrap();
    assert!(spawned.get_component("ParticleEmitter").is_some());
    assert_eq!(spawned.layer, "Effects");
}

#[test]
fn inspector_editor_edits_component_enabled_and_resets_transform() {
    let mut entity = GameObject::new(5.0, 6.0, Some("Inspectable".to_string()));
    entity.rotation = 45.0;
    entity.scale_x = 2.0;
    entity.scale_y = 3.0;
    entity.sync_to_components();

    let previous = InspectorEditor::set_component_value(
        &mut entity,
        "SpriteRenderer",
        "enabled",
        json!(false),
    )
    .unwrap();
    assert_eq!(previous, json!(true));
    assert!(!entity.get_component("SpriteRenderer").unwrap().enabled);
    assert!(
        InspectorEditor::component_summary(&entity)
            .iter()
            .any(|line| line.starts_with("SpriteRenderer:off"))
    );

    InspectorEditor::reset_transform(&mut entity);
    assert_eq!(entity.x, 0.0);
    assert_eq!(entity.y, 0.0);
    assert_eq!(entity.rotation, 0.0);
    assert_eq!(entity.scale_x, 1.0);

    let actions = InspectorEditor::quick_actions(&entity);
    assert!(actions.iter().any(|action| action.id == "assign_sprite"));
    assert!(
        actions
            .iter()
            .any(|action| action.id == "assign_texture_slot")
    );
    let material_report = InspectorEditor::assign_material_asset(
        &mut entity,
        "assets/materials/hero.material.json",
        Some("mat-guid"),
    );
    assert!(
        material_report
            .updated_fields
            .contains(&"Material2D.material_path".to_string())
    );
    let texture_report =
        InspectorEditor::assign_texture_asset(&mut entity, "assets/textures/hero_roughness.png");
    assert!(
        texture_report
            .updated_fields
            .contains(&"Material2D.roughness_texture".to_string())
    );
}

#[test]
fn sprite_editor_creates_pixels_outlines_crops_and_animation_drafts() {
    let mut sprite = SpriteEditorCanvas::new(6, 4);
    let red = SpriteColor {
        r: 220,
        g: 40,
        b: 30,
        a: 255,
    };
    let blue = SpriteColor {
        r: 30,
        g: 60,
        b: 220,
        a: 255,
    };
    sprite.fill_rect(2, 1, 2, 2, red);
    assert_eq!(sprite.replace_color(red, blue), 4);
    let outline_count = sprite.outline_alpha(SpriteColor::WHITE);
    assert!(outline_count > 0);
    assert!(sprite.crop_to_content(0));
    let draft = sprite.animation_clip_draft("HeroIdle", 2, 2, 8.0);
    assert!(!draft.frames.is_empty());
    assert_eq!(draft.frames[0].duration, 0.125);
    assert!(draft.is_timeline_ready());
    let timeline = draft.timeline_preview();
    assert_eq!(timeline.frame_count, draft.frames.len());
    assert_eq!(timeline.markers[0].label, "F1");
    assert!(timeline.warnings.is_empty());
    let sample = draft.sample_at(0.2, true).unwrap();
    assert!(sample.frame_index < draft.frames.len());
    assert!(sample.normalized_time > 0.0);
    let mut fill_test = SpriteEditorCanvas::new(2, 2);
    assert_eq!(fill_test.bucket_fill(0, 0, blue), 4);
}

#[test]
fn script_and_blueprint_editors_offer_productive_scene_actions() {
    let mut editor = ScriptEditor::default();
    editor.set_text("// player script");
    assert!(
        editor
            .code_actions()
            .iter()
            .any(|action| action.kind == "luau.template")
    );
    assert!(editor.insert_luau_event_template("on_update"));
    assert!(editor.find_symbols("on_update").len() == 1);

    let mut graph = BlueprintGraph2D::default();
    let print_node = graph.quick_add_node("print", 100.0, 80.0).unwrap();
    let event_node = graph.nodes[0].id.clone();
    assert_eq!(
        graph.connect_exec_chain(&[event_node.clone(), print_node.clone()]),
        1
    );
    graph.auto_layout();
    let comment_id = graph.add_comment_box(
        "Startup",
        "Initial gameplay flow",
        0.0,
        -220.0,
        vec![event_node],
    );
    assert!(!graph.remove_comment_box("missing"));
    assert_eq!(comment_id, "comment");
    let texture_node = graph.quick_add_node("texture slot", 400.0, 80.0).unwrap();
    assert!(
        graph
            .node_by_id(&texture_node)
            .unwrap()
            .pins
            .iter()
            .any(|pin| pin.name == "texture_path")
    );
    let summary = graph.editor_summary(vec![print_node]);
    assert!(summary.searchable_nodes >= 20);
    assert_eq!(summary.compile.comment_count, 1);
    assert!(
        summary
            .recommended_actions
            .contains(&"add_comment_box".to_string())
    );
    assert!(
        summary
            .recommended_actions
            .contains(&"attach_to_selected_actor".to_string())
    );

    let mut designer = UiDesigner2D::main_menu("MiniForge");
    let selected = designer.select_at_preview_point(640.0, 360.0);
    assert!(selected.is_some());
    assert!(
        designer
            .scene_editing_actions()
            .contains(&"move_widget_gizmo")
    );
}
