#![cfg(feature = "editor_core")]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::autosave_manager::{AutosaveDomain, AutosaveManager};
use miniforge::engine::component::default_component;
use miniforge::engine::document_manager::CloseDocumentChoice;
use miniforge::engine::editor_shell::{EditorCommandBus, EditorCommandSource, EditorShellCommand};
use miniforge::engine::event_bus::EventBus;
use miniforge::engine::miniforge_2d::gameplay::GameFramework2D;
use miniforge::engine::miniforge_2d::gameplay_ability::{
    AbilityQueue2D, AttributeSet2D, GameplayAbility2D, GameplayEffect2D, GameplayTag,
    GameplayTagContainer, Targeting2D,
};
use miniforge::engine::miniforge_2d::packaging2d::PackageProfile2D;
use miniforge::engine::miniforge_2d::paper2d::SpriteFrames2D;
use miniforge::engine::miniforge_2d::particles2d::particle_templates;
use miniforge::engine::miniforge_2d::tilemap_editor2d::TilemapEditor2D;
use miniforge::engine::miniforge_2d::ui_designer::UiDesigner2D;
use miniforge::engine::miniforge_2d::ui_framework::minimal_ui_canvas;
use miniforge::engine::miniforge_2d::{exporter2d, sequencer2d};
use miniforge::engine::project_validator::ProjectValidator;
use miniforge::engine::render_2d::{
    AtlasRegion2D, Material2D, MetalFramePlan2D, PostProcessStack2D, RenderGraph2D,
    RenderPipelineCache2D, SpriteBatcher, TextureAtlas2D,
};
use miniforge::engine::render_3d::{
    HybridScene3DStarter, Render3DCompatibilityPlan, RenderGraph3D,
};
use miniforge::engine::safe_mode::SafeModeSettings;
use miniforge::engine::script_editor::ScriptEditor;
use miniforge::engine::service_registry::EngineServiceRegistry;
use miniforge::engine::ui_runtime::{UiEventKind, UiRuntime};
use miniforge::render::backend::{
    CameraCommand3D, GraphicsApi, LightDrawCommand3D, MacroquadBackend, MeshDrawCommand3D,
    RenderBackend, RenderBackendConfig, RenderBackendSelection, SpriteDrawCommand, WgpuBackend,
};

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("miniforge-next-level-{name}-{stamp}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn script_documents_preserve_dirty_buffers_and_close_without_engine_quit() {
    let tmp = temp_dir("documents");
    let first = tmp.join("Player.luau");
    let second = tmp.join("Enemy.luau");
    fs::write(&first, "fn on_start() {}").unwrap();
    fs::write(&second, "fn on_update(dt) {}").unwrap();

    let mut editor = ScriptEditor::default();
    editor.open(first.clone()).unwrap();
    editor.set_text("fn on_start() { print(\"dirty player\"); }");
    editor.open(second.clone()).unwrap();
    editor.set_text("fn on_update(dt) { print(dt); }");

    editor.activate_next_tab(-1).unwrap();
    assert_eq!(editor.document.path, Some(first.clone()));
    assert!(editor.text().contains("dirty player"));
    assert!(editor.is_dirty(&first));

    let cancelled = editor
        .close_current_tab_with_choice(CloseDocumentChoice::Cancel)
        .unwrap();
    assert!(cancelled.cancelled);
    assert_eq!(editor.document.path, Some(first.clone()));
    assert_eq!(editor.tabs.len(), 2);

    let closed = editor
        .close_current_tab_with_choice(CloseDocumentChoice::Save)
        .unwrap();
    assert!(closed.closed);
    assert!(closed.saved);
    assert_eq!(editor.document.path, Some(second.clone()));
    assert!(fs::read_to_string(&first).unwrap().contains("dirty player"));
    assert!(editor.text().contains("print(dt)"));
}

#[test]
fn editor_command_bus_prevents_panels_from_quitting_the_engine() {
    let mut bus = EditorCommandBus::default();
    assert!(!bus.emit(EditorCommandSource::Panel, EditorShellCommand::RequestQuit));
    assert_eq!(bus.queue.len(), 0);
    assert_eq!(bus.rejected.len(), 1);

    assert!(bus.emit(
        EditorCommandSource::Panel,
        EditorShellCommand::CloseDocument(PathBuf::from("scripts/Player.luau"))
    ));
    assert_eq!(bus.drain().len(), 1);
}

#[test]
fn crash_recovery_safe_mode_and_validator_cover_next_level_assets() {
    let tmp = temp_dir("recovery");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(tmp.join("project.json"), "{}").unwrap();
    fs::write(tmp.join("manifest.json"), "{}").unwrap();
    fs::write(
        tmp.join("assets").join("a.json"),
        r#"{"guid":"asset-shared-guid"}"#,
    )
    .unwrap();
    fs::write(
        tmp.join("assets").join("b.json"),
        r#"{"guid":"asset-shared-guid"}"#,
    )
    .unwrap();
    fs::create_dir_all(tmp.join("assets").join("ui")).unwrap();
    fs::write(
        tmp.join("assets").join("ui").join("hud.ui2d.json"),
        r#"{"widgets":[]}"#,
    )
    .unwrap();

    let mut autosave = AutosaveManager::new(&tmp, 60);
    let recovered = autosave
        .save_text(AutosaveDomain::Scripts, "Player.luau", "fn on_start() {}")
        .unwrap();
    assert!(recovered.ends_with("scripts/Player.luau"));
    assert!(
        autosave
            .recover_text(AutosaveDomain::Scripts, "Player.luau")
            .unwrap()
            .contains("on_start")
    );
    assert_eq!(autosave.available_recoveries().len(), 1);

    let safe = SafeModeSettings::for_recovery("script crash");
    let report = safe.report();
    assert!(report.active);
    assert!(!safe.allows_scripts());
    assert!(!safe.allows_plugins());

    let mut validator = ProjectValidator::default();
    validator.validate(&tmp);
    assert!(
        validator
            .errors
            .iter()
            .any(|error| error.contains("GUID duplicado"))
    );
}

#[test]
fn project_validator_auto_fix_handles_safe_recovery_tasks() {
    let tmp = temp_dir("autofix");
    let paths = AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(
        paths.assets.join("NeedsGuid.json"),
        r#"{"name":"NeedsGuid","references":[null,"assets/missing.png"]}"#,
    )
    .unwrap();
    fs::write(paths.settings.join("editor_layout.json"), "{ broken").unwrap();
    fs::create_dir_all(tmp.join("plugins").join("Broken")).unwrap();
    fs::write(
        tmp.join("plugins").join("Broken").join("plugin.json"),
        r#"{
            "name":"Broken",
            "version":"1.0.0",
            "enabled":true,
            "min_engine_version":"0.9.2",
            "dependencies":["MissingPlugin"]
        }"#,
    )
    .unwrap();
    fs::create_dir_all(tmp.join("saves").join("autosave")).unwrap();
    fs::write(
        tmp.join("saves").join("autosave").join("autosave.scene"),
        r#"{"version":"0.9.2","entities":[]}"#,
    )
    .unwrap();

    let mut validator = ProjectValidator::default();
    let report = validator.auto_fix_safe(&tmp).unwrap();
    assert!(report.fixed_count() >= 6);
    assert!(
        report
            .actions
            .iter()
            .any(|action| action.contains("GUID faltante regenerado"))
    );
    assert!(
        report
            .actions
            .iter()
            .any(|action| action.contains("Assets faltantes marcados"))
    );

    let asset = AssetTools::read_json(paths.assets.join("NeedsGuid.json")).unwrap();
    assert!(asset.get("guid").is_some());
    assert_eq!(asset["references"].as_array().unwrap().len(), 1);
    assert!(tmp.join("project").join("missing_assets.json").exists());
    assert!(paths.scenes.join("main.scene").exists());
    assert!(paths.settings.join("editor_layout.json.corrupt").exists());

    let plugin =
        AssetTools::read_json(tmp.join("plugins").join("Broken").join("plugin.json")).unwrap();
    assert_eq!(plugin["enabled"], false);
    assert!(
        plugin["disabled_reason"]
            .as_str()
            .unwrap()
            .contains("MissingPlugin")
    );
}

#[test]
fn project_validator_ignores_generated_build_copies_when_checking_guids() {
    let tmp = temp_dir("validator-generated-output");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(
        tmp.join("assets").join("Unit.prefab"),
        r#"{"guid":"unit-guid","name":"Unit"}"#,
    )
    .unwrap();
    let exported = tmp.join("build").join("debug").join("Game").join("assets");
    fs::create_dir_all(&exported).unwrap();
    fs::write(
        exported.join("Unit.prefab"),
        r#"{"guid":"unit-guid","name":"Unit"}"#,
    )
    .unwrap();

    let mut validator = ProjectValidator::default();
    validator.validate(&tmp);
    assert!(
        validator
            .errors
            .iter()
            .all(|error| !error.contains("GUID duplicado"))
    );
}

#[test]
fn render_backend_and_2d_render_data_are_ready_for_macroquad_and_wgpu() {
    let config = RenderBackendConfig::default();
    assert_eq!(config.backend, "macroquad");
    assert!(config.prefer_metal_on_macos);
    assert_eq!(
        RenderBackendSelection::choose(&config).selected,
        GraphicsApi::Macroquad
    );

    let mut metal_config = RenderBackendConfig {
        backend: "wgpu".to_string(),
        experimental_wgpu: true,
        ..RenderBackendConfig::default()
    };
    metal_config.metal.allow_compute_particles = true;
    let selection = RenderBackendSelection::choose(&metal_config);
    assert!(matches!(
        selection.selected,
        GraphicsApi::WgpuMetal | GraphicsApi::WgpuVulkan | GraphicsApi::WgpuDx12
    ));
    assert_eq!(selection.fallback, Some(GraphicsApi::Macroquad));

    let mut backend = MacroquadBackend::default();
    backend.init().unwrap();
    backend.begin_frame().unwrap();
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 1,
            texture_id: 2,
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
            rotation: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
        })
        .unwrap();
    backend.end_frame().unwrap();
    assert_eq!(backend.draw_calls, 1);

    let mut disabled_wgpu = WgpuBackend::default();
    assert!(disabled_wgpu.init().is_err());

    let mut enabled_wgpu = WgpuBackend {
        enabled: true,
        prefer_metal: true,
        ..Default::default()
    };
    enabled_wgpu.init().unwrap();
    assert!(enabled_wgpu.caps.as_ref().unwrap().supports_compute);

    let mut batcher = SpriteBatcher::default();
    for id in 0..2050 {
        batcher.push(SpriteDrawCommand {
            entity_id: id,
            texture_id: 1,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            rotation: 0.0,
            color: [1.0; 4],
        });
    }
    assert_eq!(batcher.batches(), 2);
    assert!(Material2D::water_example().to_value()["shader"] == "water_wave");
    assert!(PostProcessStack2D::default().effects.len() >= 10);

    let mut cache = RenderPipelineCache2D::default();
    let material = Material2D::water_example();
    let first = cache.pipeline_for_material(&material, 4);
    let second = cache.pipeline_for_material(&material, 4);
    assert_eq!(first, second);
    assert_eq!(cache.pipelines.len(), 1);

    let mut atlas = TextureAtlas2D {
        name: "MainAtlas".to_string(),
        width: 256,
        height: 256,
        regions: Default::default(),
    };
    assert!(atlas.add_region(
        "player_idle",
        AtlasRegion2D {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
            extrude: 2,
        }
    ));
    assert_eq!(atlas.uv_rect("player_idle").unwrap()[2], 0.125);
    assert!(
        RenderGraph2D::default_2d(true)
            .passes
            .iter()
            .any(|pass| pass.name == "post_process")
    );
}

#[test]
fn spriteframes_sequencer_ability_and_export_profiles_cover_guide_surface() {
    let spriteframes = SpriteFrames2D::grid_slice(
        "PlayerSpriteFrames",
        "assets/player_sheet.png",
        2,
        2,
        32,
        32,
        8.0,
    );
    assert!(spriteframes.validate());
    assert_eq!(spriteframes.animations[0].frames.len(), 4);

    let tracks = sequencer2d::supported_track_types();
    assert!(tracks.contains(&"material_parameter"));
    assert!(tracks.contains(&"gameplay_tag"));

    let owner_tags = GameplayTagContainer::from_tags(&["Character.Player", "Weapon.Sword"]);
    assert!(owner_tags.has(&GameplayTag::new("Character")));
    let ability = GameplayAbility2D {
        name: "Slash".to_string(),
        tags: GameplayTagContainer::from_tags(&["Ability.Melee"]),
        required_tags: GameplayTagContainer::from_tags(&["Weapon.Sword"]),
        blocked_tags: GameplayTagContainer::from_tags(&["State.Stunned"]),
        cooldown_seconds: 0.25,
        cost_attribute: Some("Stamina".to_string()),
        cost_amount: 10.0,
        targeting: Targeting2D::default(),
    };
    let mut attributes = AttributeSet2D {
        name: "PlayerAttributes".to_string(),
        attributes: [("Stamina".to_string(), 25.0), ("Health".to_string(), 100.0)]
            .into_iter()
            .collect(),
    };
    assert!(ability.can_activate(&owner_tags, &attributes));
    GameplayEffect2D {
        name: "Burn".to_string(),
        duration_seconds: 2.0,
        granted_tags: GameplayTagContainer::from_tags(&["State.Burning"]),
        modifiers: vec![
            miniforge::engine::miniforge_2d::gameplay_ability::AttributeModifier2D {
                attribute: "Health".to_string(),
                operation: "add".to_string(),
                magnitude: -5.0,
            },
        ],
    }
    .apply_to(&mut attributes);
    assert_eq!(attributes.attributes["Health"], 95.0);

    let mut queue = AbilityQueue2D {
        max_len: 1,
        ..Default::default()
    };
    queue.queue("Slash");
    queue.queue("Dash");
    assert_eq!(queue.pop_next().as_deref(), Some("Dash"));

    let layout = exporter2d::export_layout(PackageProfile2D::Shipping, "Demo");
    assert!(layout.root.contains("shipping"));
    assert!(layout.folders.contains(&"timelines".to_string()));
    assert!(layout.folders.contains(&"plugins".to_string()));
}

#[test]
fn ui2d_designer_runtime_bindings_and_navigation_support_complex_huds() {
    let canvas = minimal_ui_canvas();
    assert!(canvas.validate_widget_ids());
    assert!(
        canvas
            .binding_paths()
            .contains(&"player.health_percent".to_string())
    );
    assert_eq!(
        canvas.focused_neighbor("StartButton", "down"),
        Some("CoinText")
    );

    let layout = canvas.resolve_layout((1280.0, 720.0));
    assert!(layout.iter().any(|widget| widget.id == "HealthBar"));

    let mut runtime = UiRuntime::default();
    let events = runtime.update_miniforge_canvas_interaction(
        &canvas,
        (1280.0, 720.0),
        Some((30.0, 104.0)),
        true,
    );
    assert!(events.iter().any(|event| event.kind == UiEventKind::Click));
    assert_eq!(
        runtime.move_focus(&canvas, "down").as_deref(),
        Some("CoinText")
    );

    let mut designer = UiDesigner2D::default();
    assert!(designer.select("StartButton"));
    assert!(designer.move_selected(9.0, 7.0));
    assert!(designer.resize_selected(188.0, 44.0));
    assert!(designer.set_selected_property("text", serde_json::json!("Continue")));
    assert!(designer.responsive_preview_count() >= 4);
    assert!(designer.preview_layout().len() >= 3);
}

#[test]
fn complex_game_framework_declares_services_streaming_and_save_paths() {
    let framework = GameFramework2D::default();
    assert!(framework.validate_complex_game_setup().is_empty());
    let order = framework.startup_order();
    assert!(order.contains(&"SaveService".to_string()));
    assert!(order.contains(&"RenderService".to_string()));
    assert!(
        framework
            .scene_streaming
            .preload_assets
            .iter()
            .any(|asset| asset.ends_with(".mfgraph"))
    );
    assert!(
        framework
            .save_game
            .saved_systems
            .contains(&"Inventory".to_string())
    );
}

#[test]
fn massive_particle_templates_bridge_to_runtime_and_gpu_planning() {
    let templates = particle_templates();
    assert!(templates.len() >= 10);
    assert!(
        templates
            .iter()
            .all(|template| template.system.validate().is_empty())
    );

    let rain = templates
        .iter()
        .find(|template| template.name == "Rain2D")
        .unwrap();
    assert!(rain.system.gpu_recommended());
    assert!(rain.system.estimate_max_particles() >= 4_096);

    let explosion = templates
        .iter()
        .find(|template| template.name == "Explosion2D")
        .unwrap();
    let config = explosion.system.to_runtime_emitter_config().unwrap();
    assert!(config.burst_count >= 48);
    assert!(config.max_particles >= config.burst_count);
    assert_eq!(config.color[3], 255);
}

#[test]
fn massive_tilemap_editor_supports_lines_stamps_random_rules_and_objects() {
    let mut editor = TilemapEditor2D::new(10, 10);
    assert!(editor.validate().is_empty());

    let line = editor.apply_line(0, (0, 0), (5, 0), 2);
    assert_eq!(line.changes.len(), 6);
    assert_eq!(editor.tilemap.layers[0].get(5, 0), 2);

    let stamp = editor.apply_stamp(0, "ThreeByThreeRoom", (4, 4));
    assert_eq!(stamp.changes.len(), 9);
    assert_eq!(editor.tilemap.layers[0].get(4, 4), 1);

    editor.random_seed = 42;
    let random = editor.apply_random(0, (0, 1), (3, 1), &[(7, 1), (8, 3)]);
    assert_eq!(random.changes.len(), 4);
    assert!(random.changes.iter().any(|change| change.after == 8));

    editor.tilemap.layers[0].set(1, 2, 4);
    let rule = editor.apply_rule_tiles(0);
    assert!(rule.changes.iter().any(|change| change.after == 5));

    editor.random_seed = 1;
    let objects = editor.place_objects(0, "CoinScatter", (0, 0), (4, 4));
    assert!(!objects.is_empty());
    assert!(
        objects
            .iter()
            .all(|object| object.prefab.ends_with("coin.prefab"))
    );
}

#[test]
fn service_registry_routes_editor_runtime_events_without_taking_down_engine() {
    let mut registry = EngineServiceRegistry::default_miniforge_2d();
    assert!(registry.validate().is_empty());
    assert_eq!(
        registry.startup_order().first().unwrap(),
        "DiagnosticsService"
    );

    let mut events = EventBus::default();
    events.emit(
        "ScriptSaved",
        serde_json::json!({"path": "scripts/player.luau"}),
    );
    events.emit(
        "ParticlePresetChanged",
        serde_json::json!({"name": "FX_Fire2D"}),
    );

    let delivered = registry.dispatch_bus_events(&mut events);
    assert!(delivered.iter().any(|message| {
        message.service == "ScriptService" && message.event == "ScriptSaved" && message.accepted
    }));
    assert!(delivered.iter().any(|message| {
        message.service == "RenderService"
            && message.event == "ParticlePresetChanged"
            && message.accepted
    }));

    registry.set_enabled("AssetService", false);
    let issues = registry.validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("ProjectService depende de AssetService"))
    );
    assert!(registry.manifest()["startup_order"].is_array());
}

#[test]
fn metal_frame_plan_models_compute_without_breaking_macroquad_fallback() {
    let mut config = RenderBackendConfig {
        backend: "wgpu".to_string(),
        experimental_wgpu: true,
        gpu_particles: true,
        ..RenderBackendConfig::default()
    };
    config.metal.allow_compute_particles = true;
    config.metal.allow_compute_tile_visibility = true;
    config.metal.allow_compute_flow_fields = true;

    let plan =
        MetalFramePlan2D::from_config(&config, RenderGraph2D::default_2d(true), 8_000, 128, 4_096);
    assert!(plan.has_enabled_compute());
    assert!(
        plan.compute_jobs
            .iter()
            .any(|job| job.name == "gpu_particle_simulation" && job.enabled)
    );
    assert!(
        plan.compute_jobs
            .iter()
            .any(|job| job.name == "flow_field_update")
    );

    let stable = MetalFramePlan2D::from_config(
        &RenderBackendConfig::default(),
        RenderGraph2D::default_2d(true),
        100,
        4,
        64,
    );
    assert!(!stable.has_enabled_compute());
    assert!(stable.fallback_passes().contains(&"tilemaps".to_string()));
}

#[test]
fn graphics_3d_compatibility_has_safe_hybrid_path() {
    for component_type in [
        "Transform3D",
        "MeshRenderer3D",
        "Camera3D",
        "Light3D",
        "Material3D",
        "Billboard3D",
        "HybridScene3D",
    ] {
        assert!(
            default_component(component_type).is_some(),
            "{component_type} should have 3D defaults"
        );
    }

    let disabled = Render3DCompatibilityPlan::from_config(&RenderBackendConfig::default());
    assert!(!disabled.enabled);
    assert!(
        disabled
            .warnings
            .iter()
            .any(|warning| warning.contains("3D desactivado"))
    );

    let macroquad_3d = Render3DCompatibilityPlan::from_config(&RenderBackendConfig {
        enable_3d: true,
        hybrid_2d_3d: true,
        depth_buffer: true,
        ..RenderBackendConfig::default()
    });
    assert!(macroquad_3d.can_preview_3d());
    assert!(macroquad_3d.hybrid_2d_3d);
    assert!(
        macroquad_3d
            .supported_features
            .contains(&"sprite_billboards".to_string())
    );
    assert!(
        macroquad_3d
            .deferred_features
            .contains(&"pbr_materials".to_string())
    );
    assert!(!macroquad_3d.is_large_3d_game_ready());

    let wgpu_3d = Render3DCompatibilityPlan::from_config(&RenderBackendConfig {
        backend: "wgpu".to_string(),
        experimental_wgpu: true,
        enable_3d: true,
        hybrid_2d_3d: true,
        depth_buffer: true,
        mesh_batching: true,
        shadow_maps_3d: true,
        ..RenderBackendConfig::default()
    });
    assert!(wgpu_3d.can_preview_3d());
    assert!(
        wgpu_3d
            .supported_features
            .contains(&"mesh_batching".to_string())
    );

    let graph = RenderGraph3D::default_hybrid_2d3d(true);
    assert!(graph.passes.iter().any(|pass| pass.name == "overlay_2d_ui"));
    assert!(
        graph
            .passes
            .iter()
            .any(|pass| pass.name == "sprite_billboards_3d")
    );

    let starter = HybridScene3DStarter::minimal();
    assert!(starter.validate().is_empty());
    assert_eq!(starter.meshes.len(), 2);

    let mut backend = MacroquadBackend::default();
    backend.init().unwrap();
    backend.begin_frame().unwrap();
    backend
        .set_camera_3d(CameraCommand3D {
            entity_id: 1,
            position: [0.0, 4.0, 8.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_y_degrees: 60.0,
            near: 0.05,
            far: 500.0,
        })
        .unwrap();
    backend
        .draw_mesh_3d(MeshDrawCommand3D {
            entity_id: 2,
            mesh_id: 7,
            material_id: Some(9),
            position: [0.0, 0.0, 0.0],
            rotation_euler: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        })
        .unwrap();
    backend
        .draw_light_3d(LightDrawCommand3D {
            entity_id: 3,
            light_type: "directional".to_string(),
            position: [0.0, 6.0, 0.0],
            direction: [-0.4, -1.0, -0.3],
            color: [1.0, 0.96, 0.88, 1.0],
            intensity: 1.0,
            range: 64.0,
            casts_shadows: false,
        })
        .unwrap();
    backend.end_frame().unwrap();
    assert_eq!(backend.draw_calls, 1);
}
