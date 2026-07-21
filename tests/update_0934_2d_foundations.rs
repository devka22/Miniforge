#![cfg(feature = "editor_core")]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_pipeline_2d::{ImportPipeline2D, ImportProfile2D, ReimportReason2D};
use miniforge::engine::editor_python::{PythonAutomationHost, PythonEditorContext};
use miniforge::engine::editor_spatial_tools_2d::{
    AlignMode2D, EditorGroupManager2D, EditorLayerManager2D, EditorSpatialTools2D,
    SmartSnapSettings2D,
};
use miniforge::engine::editor_workflow_2d::{
    EditorContext2D, EditorTransaction2D, EditorWorkflow2D,
};
use miniforge::engine::script_host_2d::{
    ScriptBackendState2D, ScriptCall2D, ScriptCapability2D, ScriptFunction2D, ScriptHost2D,
    ScriptLanguage2D, ScriptModuleManifest2D,
};
use miniforge::engine::tilemap_layers::TilemapLayers;
use miniforge::engine::update_0934::{
    Engine0934FoundationPlan, FOUNDATION_VERSION, FoundationReleaseState,
};
use miniforge::engine::vector_canvas_2d::{VectorPath2D, VectorPoint2D, VectorStyle2D};
use miniforge::engine::version::{DEVELOPMENT_VERSION, ENGINE_VERSION, development_version_label};
use miniforge::engine::{component::default_component, tile_brush::TileBrushMode};
use miniforge::entities::game_object::GameObject;
use serde_json::json;

#[test]
fn final_0934_version_is_visible_and_launchable() {
    let plan = Engine0934FoundationPlan::current();
    assert_eq!(FOUNDATION_VERSION, "0.9.3.4");
    assert_eq!(ENGINE_VERSION, "0.9.3.4");
    assert_eq!(DEVELOPMENT_VERSION, "0.9.3.4");
    assert_eq!(plan.release_state, FoundationReleaseState::Released);
    assert!(plan.launch_allowed);
    assert!(!plan.is_unreleased());
    assert!(development_version_label().contains("released"));
    assert!(
        plan.capabilities
            .iter()
            .any(|item| item.area == "2D asset workflow")
    );
}

#[test]
fn source_imports_are_rebuildable_and_dependency_aware() {
    let root = temp_root("import_pipeline");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("hero.png");
    let imported = root.join(".miniforge/imported/hero.texture2d");
    fs::write(&source, b"sprite-v1").unwrap();

    let mut pipeline = ImportPipeline2D::default();
    let key = source.to_string_lossy().to_string();
    pipeline
        .register_source(&source, &imported, Some(ImportProfile2D::pixel_art()))
        .unwrap();
    assert!(pipeline.add_dependency(&key, "palette:forest"));

    let first_plan = pipeline.plan_reimport();
    assert_eq!(first_plan.jobs.len(), 1);
    assert!(
        first_plan.jobs[0]
            .reasons
            .contains(&ReimportReason2D::NewSource)
    );

    fs::create_dir_all(imported.parent().unwrap()).unwrap();
    fs::write(&imported, b"runtime-texture").unwrap();
    pipeline
        .complete_import(&key, vec![imported.to_string_lossy().to_string()], 10)
        .unwrap();
    assert!(pipeline.plan_reimport().jobs.is_empty());

    pipeline.mark_dependency_changed("palette:forest");
    assert!(
        pipeline.plan_reimport().jobs[0]
            .reasons
            .contains(&ReimportReason2D::DependencyChanged)
    );

    let changes = BTreeMap::from([("filter".to_string(), json!(true))]);
    assert_eq!(
        pipeline.update_profiles_batch(std::slice::from_ref(&key), &changes),
        1
    );
    assert_eq!(
        pipeline.imports[&key].profile.options["filter"],
        json!(true)
    );
    assert!(
        pipeline.plan_reimport().jobs[0]
            .reasons
            .contains(&ReimportReason2D::ImporterChanged)
    );

    let manifest = root.join("imports.json");
    pipeline.save(&manifest).unwrap();
    let loaded = ImportPipeline2D::load(&manifest).unwrap();
    assert_eq!(loaded.imports[&key].dependencies, vec!["palette:forest"]);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn language_host_has_one_abi_and_explicit_backend_readiness() {
    let mut host = ScriptHost2D::foundation();
    assert_eq!(
        host.backends[&ScriptLanguage2D::Luau].state,
        ScriptBackendState2D::BuiltIn
    );
    assert_eq!(
        host.backends[&ScriptLanguage2D::Blueprint].state,
        ScriptBackendState2D::BuiltIn
    );
    assert_eq!(
        host.backends[&ScriptLanguage2D::Python].state,
        ScriptBackendState2D::Available
    );
    assert_eq!(
        host.backends[&ScriptLanguage2D::CSharp].state,
        ScriptBackendState2D::Available
    );
    assert_eq!(host.backends.len(), 4);

    host.register_module(ScriptModuleManifest2D {
        id: "player_controller".to_string(),
        language: ScriptLanguage2D::Luau,
        source: "scripts/player.luau".to_string(),
        api_version: 1,
        functions: vec![ScriptFunction2D {
            name: "move_player".to_string(),
            parameters: vec!["axis".to_string()],
            returns: Some("Vector2".to_string()),
        }],
        capabilities: BTreeSet::from([ScriptCapability2D::Input, ScriptCapability2D::WriteScene]),
        editor_only: false,
    })
    .unwrap();

    let valid_call = ScriptCall2D {
        module: "player_controller".to_string(),
        function: "move_player".to_string(),
        arguments: BTreeMap::from([("axis".to_string(), json!([1.0, 0.0]))]),
    };
    assert_eq!(host.validate_call(&valid_call).unwrap().name, "move_player");

    let incompatible_module = ScriptModuleManifest2D {
        id: "future_luau_module".to_string(),
        language: ScriptLanguage2D::Luau,
        source: "scripts/future.luau".to_string(),
        api_version: 99,
        functions: Vec::new(),
        capabilities: BTreeSet::new(),
        editor_only: false,
    };
    let errors = host.register_module(incompatible_module).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("incompatible")));
}

#[test]
fn editor_actions_are_contextual_searchable_and_undoable() {
    let mut workflow = EditorWorkflow2D {
        active_context: EditorContext2D::Sprite,
        ..EditorWorkflow2D::default()
    };
    let matches = workflow.search_actions("socket", 10);
    assert_eq!(matches[0].id, "sprite.edit_sockets");
    assert!(
        workflow
            .actions_for_context(EditorContext2D::Sprite)
            .iter()
            .any(|action| action.id == "asset.reimport")
    );
    assert!(
        !workflow
            .actions_for_context(EditorContext2D::Sprite)
            .iter()
            .any(|action| action.id == "tilemap.paint")
    );

    let transaction = EditorTransaction2D::batch_property(
        "Set pixels per unit",
        [
            ("hero_idle".to_string(), json!(1.0)),
            ("hero_run".to_string(), json!(2.0)),
        ],
        "pixels_per_unit",
        json!(16.0),
    );
    workflow.push_transaction(transaction);
    let undo = workflow.undo().unwrap();
    assert_eq!(undo.edits.len(), 2);
    assert_eq!(undo.edits[0].after, json!(2.0));
    assert_eq!(workflow.redo().unwrap().label, "Set pixels per unit");
}

#[test]
fn lyon_vector_canvas_tessellates_curves_fills_strokes_and_hit_tests() {
    let style = VectorStyle2D {
        fill: Some([40, 120, 240, 90]),
        stroke: Some([110, 210, 255, 255]),
        stroke_width: 2.0,
        ..VectorStyle2D::default()
    };
    let rounded = VectorPath2D::rounded_rectangle(
        VectorPoint2D::new(0.0, 0.0),
        VectorPoint2D::new(100.0, 60.0),
        12.0,
        style.clone(),
    );
    let geometry = rounded.tessellate().unwrap();
    assert!(geometry.fill.as_ref().unwrap().indices.len() >= 6);
    assert!(geometry.stroke.as_ref().unwrap().vertices.len() >= 8);
    assert!(rounded.hit_test_fill(VectorPoint2D::new(50.0, 30.0)));
    assert!(!rounded.hit_test_fill(VectorPoint2D::new(150.0, 30.0)));

    let bezier = VectorPath2D::new(VectorStyle2D {
        fill: None,
        stroke: Some([255, 200, 90, 255]),
        stroke_width: 3.0,
        ..VectorStyle2D::default()
    })
    .move_to(0.0, 0.0)
    .cubic_to(30.0, -40.0, 70.0, 40.0, 100.0, 0.0);
    assert!(bezier.tessellate().unwrap().stroke.unwrap().indices.len() > 12);
}

#[test]
fn smart_snap_alignment_groups_layers_pivots_and_collisions_work_together() {
    let mut moving = GameObject::new(0.0, 0.0, Some("Moving".to_string()));
    let target = GameObject::new(3.0, 0.0, Some("Target".to_string()));
    let settings = SmartSnapSettings2D {
        grid_enabled: false,
        ..SmartSnapSettings2D::default()
    };
    let snap = EditorSpatialTools2D::smart_snap(
        &moving,
        (1.98, 0.02),
        &[moving.clone(), target.clone()],
        1.0,
        &settings,
    );
    assert!(snap.snapped_x && snap.snapped_y);
    assert!((snap.point.0 - 2.0).abs() < 0.001);

    let mut third = GameObject::new(8.0, 4.0, Some("Third".to_string()));
    let ids = vec![moving.id, target.id, third.id];
    let mut entities = vec![moving.clone(), target.clone(), third.clone()];
    assert_eq!(
        EditorSpatialTools2D::align(&mut entities, &ids, AlignMode2D::CenterY).len(),
        3
    );
    assert!(entities.iter().all(|entity| entity.y == entities[0].y));

    let mut groups = EditorGroupManager2D::default();
    let group_id = groups
        .group_entities(&mut entities, &ids, "Gameplay")
        .unwrap();
    assert_eq!(groups.selection_for(&group_id).len(), 3);
    assert!(entities.iter().all(|entity| entity.editor_group.is_some()));

    entities[0].layer = "Actors".to_string();
    let mut layers = EditorLayerManager2D::from_entities(&entities);
    assert!(layers.set_locked("Actors", true));
    layers.apply(&mut entities);
    assert!(entities[0].locked);

    moving.add_component(default_component("SpriteRenderer").unwrap());
    assert!(EditorSpatialTools2D::set_pivot(
        &mut moving,
        (0.25, 0.75),
        false
    ));
    assert_eq!(EditorSpatialTools2D::pivot(&moving), (0.25, 0.75));
    assert!(EditorSpatialTools2D::move_collision_vertex(
        &mut moving,
        0,
        (-0.75, -0.5),
        Some(0.25),
    ));
    assert_eq!(
        EditorSpatialTools2D::collision_points(&moving)[0],
        (-0.75, -0.5)
    );

    third.locked = true;
}

#[test]
fn line_tile_tool_paints_a_continuous_diagonal() {
    let mut tilemap = TilemapLayers::new(8, 8);
    let stroke = miniforge::engine::tile_brush::TileBrush::apply(
        &mut tilemap,
        TileBrushMode::Line,
        (1, 1),
        (5, 5),
        7,
    );
    assert_eq!(stroke.changes.len(), 5);
    for coordinate in 1..=5 {
        assert_eq!(tilemap.layers[0].get(coordinate, coordinate), 7);
    }
}

#[test]
fn python_is_editor_only_trusted_and_uses_the_json_protocol() {
    let root = temp_root("python_tools");
    fs::create_dir_all(&root).unwrap();
    let host = PythonAutomationHost::new(&root);
    if host.interpreter_version().is_err() {
        fs::remove_dir_all(root).unwrap();
        return;
    }
    host.install_builtin_tools().unwrap();
    let manifest = host
        .discover()
        .unwrap()
        .into_iter()
        .find(|tool| tool.id == "scene_report")
        .unwrap();
    let result = host
        .run(
            &manifest,
            PythonEditorContext {
                active_scene: Some("main.scene".to_string()),
                selected_entity_ids: vec![10, 20],
                assets: vec!["hero.png".to_string()],
                ..PythonEditorContext::default()
            },
        )
        .unwrap();
    assert!(result.success, "{}\n{}", result.stdout, result.stderr);
    assert!(result.message.contains("selection=2"));
    assert!(
        result
            .operations
            .iter()
            .all(|operation| operation.operation == "log")
    );
    fs::remove_dir_all(root).unwrap();
}

fn temp_root(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("miniforge_0934_{label}_{nonce}"))
}
