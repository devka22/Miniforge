use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use miniforge::core::engine_config::EngineConfig;
use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::diagnostics::FrameHealth;
use miniforge::engine::engine_backend::EngineBackend;
use miniforge::engine::input_map::InputMap;
use miniforge::engine::manifest_builder::ManifestBuilder;
use miniforge::engine::runtime_config::{HardwareProfile, RuntimeConfig, RuntimeTuning};
use miniforge::engine::system_scheduler::{ScheduledSystem, SystemScheduler};
use miniforge::engine::update_093::Engine093UpgradePlan;
use miniforge::engine::update_0934::FoundationReleaseState;
use miniforge::engine::version::{ENGINE_STREAM_VERSION, ENGINE_VERSION, version_label};
use miniforge::entities::game_object::GameObject;
use miniforge::input::input_handler::InputHandler;
use miniforge::map::grid::Grid;
use miniforge::map::pathfinding::astar_report;
use miniforge::render::renderer::Renderer;
use miniforge::runtime::game_runner::{RuntimeRunOptions, run_with_options};
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-093-{name}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn update_093_manifest_and_project_defaults_are_visible() {
    assert_eq!(ENGINE_VERSION, "0.9.3.4");
    assert_eq!(ENGINE_STREAM_VERSION, "0.9.3.4");
    assert!(version_label().contains("2D Workflow Foundations"));

    let upgrade = Engine093UpgradePlan::current();
    for system in [
        "Backend",
        "Core",
        "Docs",
        "Input",
        "Packaging",
        "Pathfinding",
        "Render",
        "Runtime",
        "Systems",
        "UI",
    ] {
        assert!(upgrade.systems().contains(&system.to_string()));
    }

    let tmp = temp_dir("defaults");
    let paths = AssetTools::ensure_project_folders(&tmp).unwrap();
    let project = AssetTools::read_json(tmp.join("project.json")).unwrap();
    let seeded_manifest = AssetTools::read_json(tmp.join("manifest.json")).unwrap();
    assert_eq!(
        project["description"],
        "MiniForge 0.9.3.4 2D Workflow Foundations project"
    );
    assert_eq!(seeded_manifest["engine_stream_version"], "0.9.3.4");
    assert_eq!(seeded_manifest["update_093"]["version"], "0.9.3.4");

    let engine_config = EngineConfig::new(&tmp).unwrap();
    assert_eq!(engine_config.data["config_version"], 3);
    assert_eq!(engine_config.data["runtime"]["scheduler_budget_ms"], 16.67);

    let runtime_config = RuntimeConfig::new(paths.settings.join("runtime_config.json")).unwrap();
    assert_eq!(runtime_config.tuning().quality_preset, "balanced");
    assert_eq!(runtime_config.tuning().performance_class, "desktop");
    assert_eq!(
        runtime_config.data["graphics"]["view_frustum_culling"],
        true
    );
    assert_eq!(runtime_config.data["graphics"]["occlusion_culling"], true);
    assert_eq!(runtime_config.data["graphics"]["lod_enabled"], true);
    assert_eq!(runtime_config.data["graphics"]["backface_culling_3d"], true);

    let manifest = ManifestBuilder::build_manifest(&tmp).unwrap();
    assert_eq!(manifest["update_093"]["version"], "0.9.3.4");
    assert_eq!(manifest["update_0934"]["version"], "0.9.3.4");
    assert_eq!(manifest["update_0934"]["release_state"], "Released");
    assert!(
        manifest["update_093"]["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["system"] == "Packaging")
    );
}

#[test]
fn backend_runtime_tuning_and_runner_report_next_pass_state() {
    let tmp = temp_dir("backend-runtime");
    let paths = AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(
        paths.scripts.join("Player.luau"),
        "function on_update(dt) end",
    )
    .unwrap();
    fs::write(
        paths.scenes.join("main.scene"),
        r#"{"version":"0.9.3.4","entities":[]}"#,
    )
    .unwrap();
    AssetTools::write_json(
        paths.settings.join("runtime_config.json"),
        &json!({
            "target_fps": 120,
            "fixed_timestep": 0.008333333,
            "max_frame_steps": 6,
            "max_entities": 20000,
            "max_particles": 100000,
            "streaming_enabled": true,
            "asset_hot_reload": true,
            "quality_preset": "high",
            "performance_class": "workstation"
        }),
    )
    .unwrap();

    let plan = EngineBackend::plan_project(&tmp).unwrap();
    assert_eq!(plan.update_093.version, "0.9.3.4");
    assert_eq!(
        plan.update_0934.release_state,
        FoundationReleaseState::Released
    );
    assert!(plan.update_0934.launch_allowed);
    assert!(plan.update_093.systems().contains(&"Runtime".to_string()));
    assert!(plan.runtime_tuning.complex_game_ready());
    assert_eq!(plan.runtime_tuning.recommended_worker_threads(), 8);
    assert!(plan.hardware_profile.logical_cpus >= 1);
    assert!(plan.runtime_tuning.frame_budget_ms() < 9.0);
    assert!(plan.system_audit.areas.contains_key("Runtime"));

    let (game, report) = run_with_options(
        &tmp,
        RuntimeRunOptions {
            fixed_dt: 0.02,
            steps: 3,
            runtime_mode: true,
        },
    )
    .unwrap();
    assert_eq!(report.steps, 3);
    assert!((report.simulated_seconds - 0.06).abs() < 0.0001);
    assert_eq!(game.mode, "PLAY");
    assert!((game.clock.fixed_delta - 0.008333333).abs() < 0.000001);
    assert_eq!(game.clock.max_steps_per_frame, 6);
    assert!(game.clock.frame_budget_ms() < 9.0);
    assert!(game.profiler.metrics.contains_key("FrameBudgetMs"));
    assert_eq!(report.entity_count, game.units.len());
    assert!(report.project_path.contains("backend-runtime"));
    assert_eq!(game.diagnostics.last_frame.health, FrameHealth::OverBudget);
    assert_eq!(game.diagnostics.last_frame.fixed_steps, 3);
    assert!(game.diagnostics.health_summary().contains("stability"));

    let old_runtime_json = json!({"target_fps": 75});
    let tuning = RuntimeTuning::from_value(&old_runtime_json);
    assert_eq!(tuning.target_fps, 75);
    assert_eq!(tuning.quality_preset, "balanced");
    assert_eq!(tuning.performance_class, "desktop");

    AssetTools::write_json(
        paths.settings.join("runtime_config.json"),
        &json!({
            "graphics": {
                "quality": "low"
            }
        }),
    )
    .unwrap();
    let migrated_runtime = RuntimeConfig::new(paths.settings.join("runtime_config.json")).unwrap();
    assert_eq!(migrated_runtime.data["graphics"]["quality"], "low");
    assert_eq!(
        migrated_runtime.data["graphics"]["profiles"]["medium"]["max_drawn_entities"],
        520
    );
    assert_eq!(
        migrated_runtime.data["graphics"]["view_frustum_culling"],
        true
    );
}

#[test]
fn runtime_tuning_can_optimize_for_apple_silicon_style_hardware() {
    let hardware = HardwareProfile {
        logical_cpus: 12,
        memory_mb: 36 * 1024,
        os_name: "macOS".to_string(),
        arch: "aarch64".to_string(),
        apple_silicon: true,
        performance_tier: "apple_silicon_pro".to_string(),
    };
    let tuning = RuntimeTuning::from_value(&json!({
        "performance_class": "auto",
        "worker_threads": "auto",
        "parallel_asset_scan": true
    }));
    let optimized = tuning.optimized_for_hardware(&hardware);
    assert_eq!(optimized.performance_class, "apple_silicon_pro");
    assert_eq!(optimized.worker_threads, Some(10));
    assert_eq!(optimized.recommended_worker_threads_for(&hardware), 10);
    assert!(
        optimized
            .hardware_recommendations(&hardware)
            .iter()
            .any(|note| note.contains("Apple Silicon"))
    );
}

#[test]
fn input_edges_axis_scheduler_and_render_stats_are_measurable() {
    let tmp = temp_dir("runtime-controls");
    let mut input_map = InputMap::new(tmp.join("input_map.json")).unwrap();
    input_map
        .set_binding("Move", vec!["keyboard:wasd".to_string()])
        .unwrap();

    let mut input = InputHandler::default();
    input.set_pressed("w", true);
    assert!(input.action_pressed(&input_map, "Move"));
    assert!(input.action_just_pressed(&input_map, "Move"));
    assert_eq!(input.action_axis_2d(&input_map, "Move"), (0.0, -1.0));
    input.update();
    assert!(!input.action_just_pressed(&input_map, "Move"));
    input.set_pressed("w", false);
    assert!(input.action_just_released(&input_map, "Move"));

    let mut scheduler = SystemScheduler {
        budget_ms: 0.01,
        ..Default::default()
    };
    scheduler.register(
        Box::new(TestSystem::new("PlayOnly", false, true, false)),
        10,
    );
    scheduler.register(
        Box::new(TestSystem::new("EditorProfiler", true, true, true)),
        5,
    );
    let report = scheduler.update_with_report(1.0 / 60.0, "EDITOR", None);
    assert_eq!(report.ran, 1);
    assert_eq!(report.skipped, 1);
    assert_eq!(report.samples[0].name, "EditorProfiler");
    assert!(report.samples.iter().any(|sample| sample.skipped));
    assert!(!report.warnings.is_empty());

    let mut renderer = Renderer::default();
    let visible = GameObject::new(0.0, 0.0, Some("Visible".to_string()));
    let mut hidden = GameObject::new(2.0, 0.0, Some("Hidden".to_string()));
    hidden.visible = false;
    renderer.begin_frame();
    renderer.draw_entities(&[visible, hidden]);
    let stats = renderer.frame_stats();
    assert_eq!(stats.submitted_entities, 2);
    assert_eq!(stats.visible_entities, 1);
    assert_eq!(stats.culled_entities, 1);
    assert_eq!(stats.draw_calls, 1);
    assert!((renderer.visibility_ratio() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn entity_bundles_and_pathfinding_reports_reduce_guesswork() {
    let mut entity = GameObject::new(3.0, 4.0, Some("Hero".to_string()));
    let report = entity.ensure_components(&["Transform", "Health", "Stats", "Missing093"]);
    assert_eq!(report.existing, vec!["Transform"]);
    assert!(report.added.contains(&"Health".to_string()));
    assert!(report.added.contains(&"Stats".to_string()));
    assert_eq!(report.missing, vec!["Missing093"]);
    let component_types = entity.component_types();
    assert!(component_types.contains(&"Health".to_string()));
    assert!(entity.is_runtime_active());
    entity.active = false;
    assert!(!entity.is_runtime_active());

    let mut grid = Grid::new(6, 6, 32, 4);
    grid.set_tile(2, 1, 1);
    grid.set_tile(2, 2, 1);
    grid.set_tile(2, 3, 1);
    let path_report = astar_report(&grid, (0, 0), (5, 5), true);
    assert!(path_report.found);
    assert!(path_report.raw_len >= path_report.smoothed_len);
    assert!(path_report.used_visibility_smoothing);
    assert_eq!(path_report.path.last().copied(), Some((5, 5)));
}

struct TestSystem {
    name: &'static str,
    editor: bool,
    play: bool,
    sleep: bool,
}

impl TestSystem {
    fn new(name: &'static str, editor: bool, play: bool, sleep: bool) -> Self {
        Self {
            name,
            editor,
            play,
            sleep,
        }
    }
}

impl ScheduledSystem for TestSystem {
    fn name(&self) -> &str {
        self.name
    }

    fn run_in_editor(&self) -> bool {
        self.editor
    }

    fn run_in_play(&self) -> bool {
        self.play
    }

    fn update(&mut self, _dt: f64) {
        if self.sleep {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
