//! Runtime-only composition root.
//!
//! This module deliberately does not import `core::game::Game` or any editor
//! service.  Exported players build this type, which makes the editor/runtime
//! boundary enforceable by Cargo features instead of relying on a boolean.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Component as PathComponent, Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::core::engine_config::EngineConfig;
use crate::engine::animation_graph::AnimationGraphLibrary;
use crate::engine::asset_database::AssetDatabase;
use crate::engine::asset_tools::{AssetTools, ProjectPaths};
use crate::engine::audio_mixer::AudioMixer;
use crate::engine::camera::Camera;
use crate::engine::developer_console::DeveloperConsole;
use crate::engine::diagnostics::Diagnostics;
use crate::engine::game_clock::GameClock;
use crate::engine::luau_scripting::{LuauRunReport, LuauScriptRuntime, ScriptSchedulerConfig};
use crate::engine::miniforge_2d::ui_framework::UiCanvas2D;
use crate::engine::profiler::Profiler;
use crate::engine::resource_manager::ResourceManager;
use crate::engine::runtime_config::RuntimeConfig;
use crate::engine::runtime_stability::RuntimeStabilityGuard;
use crate::engine::safe_mode::SafeModeSettings;
use crate::engine::scene_manager::SceneManager;
use crate::engine::tilemap_layers::TilemapLayers;
use crate::engine::visual_scripting::VisualScriptRuntime;
use crate::engine::world::RuntimeWorld;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;
use crate::systems::animation_system::AnimationSystem;
use crate::systems::audio_system::AudioSystem;
use crate::systems::gameplay_system::GameplaySystem;
use crate::systems::movement_system::MovementSystem;
use crate::systems::narrative_system::{NarrativeEvent, NarrativeSystem};
use crate::systems::particle_system::ParticleSystem;
use crate::systems::physics_system::{PairType, PhysicsEventPhase, PhysicsSystem};
use crate::systems::rts_system::RTSSystem;
use crate::systems::runtime_2d_system::Runtime2DSystem;
use crate::systems::sprite_animation_system::SpriteAnimationSystem;

const MAX_UI_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_UI_DOCUMENT_WIDGETS: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeUiDocument2D {
    pub entity_id: u64,
    pub asset_path: String,
    pub input_enabled: bool,
    #[serde(default = "default_ui_scale_mode")]
    pub scale_mode: String,
    pub canvas: UiCanvas2D,
}

fn default_ui_scale_mode() -> String {
    "scale_with_screen".to_string()
}

impl RuntimeUiDocument2D {
    pub fn layout_viewport(&self, screen: (f32, f32)) -> (f32, f32) {
        if self.scales_with_screen()
            && self.canvas.viewport_width.is_finite()
            && self.canvas.viewport_height.is_finite()
            && self.canvas.viewport_width > 0.0
            && self.canvas.viewport_height > 0.0
        {
            (self.canvas.viewport_width, self.canvas.viewport_height)
        } else {
            (screen.0.max(1.0), screen.1.max(1.0))
        }
    }

    pub fn layout_scale(&self, screen: (f32, f32)) -> (f32, f32) {
        let layout = self.layout_viewport(screen);
        (
            (screen.0.max(1.0) / layout.0.max(1.0)).clamp(0.01, 100.0),
            (screen.1.max(1.0) / layout.1.max(1.0)).clamp(0.01, 100.0),
        )
    }

    pub fn screen_to_layout(&self, screen: (f32, f32), pointer: (f32, f32)) -> (f32, f32) {
        let scale = self.layout_scale(screen);
        (pointer.0 / scale.0, pointer.1 / scale.1)
    }

    fn scales_with_screen(&self) -> bool {
        matches!(
            self.scale_mode
                .trim()
                .to_ascii_lowercase()
                .replace(['-', ' '], "_")
                .as_str(),
            "scale_with_screen" | "scale" | "reference_resolution" | "responsive"
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeUiDocumentLoadReport {
    pub loaded: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

fn physics_pair_type_name(pair_type: PairType) -> &'static str {
    match pair_type {
        PairType::Collision => "collision",
        PairType::Trigger => "trigger",
    }
}

fn physics_event_name(pair_type: PairType, phase: PhysicsEventPhase) -> &'static str {
    match (pair_type, phase) {
        (PairType::Collision, PhysicsEventPhase::Enter) => "physics_collision_enter",
        (PairType::Collision, PhysicsEventPhase::Exit) => "physics_collision_exit",
        (PairType::Collision, PhysicsEventPhase::Stay) => "physics_collision_stay",
        (PairType::Trigger, PhysicsEventPhase::Enter) => "physics_trigger_enter",
        (PairType::Trigger, PhysicsEventPhase::Exit) => "physics_trigger_exit",
        (PairType::Trigger, PhysicsEventPhase::Stay) => "physics_trigger_stay",
    }
}

fn script_scheduler_config(data: &Value) -> ScriptSchedulerConfig {
    let Some(scheduler) = data.get("script_scheduler").and_then(Value::as_object) else {
        return ScriptSchedulerConfig::default();
    };
    let mut config = ScriptSchedulerConfig::default();
    config.enabled = scheduler
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(config.enabled);
    config.max_update_scripts_per_frame = scheduler
        .get("max_update_scripts_per_frame")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(1, 100_000) as usize)
        .unwrap_or(config.max_update_scripts_per_frame);
    config.default_update_interval = scheduler
        .get("default_update_interval")
        .and_then(Value::as_f64)
        .filter(|value| *value >= 0.0)
        .unwrap_or(config.default_update_interval);
    config.distant_update_interval = scheduler
        .get("distant_update_interval")
        .and_then(Value::as_f64)
        .filter(|value| *value >= 0.0)
        .unwrap_or(config.distant_update_interval);
    config.budget_bypass_priority = scheduler
        .get("budget_bypass_priority")
        .and_then(Value::as_i64)
        .unwrap_or(config.budget_bypass_priority);
    config.prioritize_by_distance = scheduler
        .get("prioritize_by_distance")
        .and_then(Value::as_bool)
        .unwrap_or(config.prioritize_by_distance);
    config.open_world_auto_policy = scheduler
        .get("open_world_auto_policy")
        .and_then(Value::as_bool)
        .unwrap_or(config.open_world_auto_policy);
    config
}

/// Services required by an exported game.
///
/// Editor history, docking, inspectors, project packaging, script editors and
/// sprite editors intentionally do not appear here.
#[derive(Debug)]
pub struct EngineRuntime {
    pub project_path: PathBuf,
    pub project_paths: ProjectPaths,
    pub engine_config: EngineConfig,
    pub mode: String,
    pub console: DeveloperConsole,
    pub runtime_world: RuntimeWorld,
    pub grid: Grid,
    pub tilemap_layers: TilemapLayers,
    pub camera: Camera,
    pub scene_manager: SceneManager,
    pub ui_canvases: Value,
    pub ui_documents: Vec<RuntimeUiDocument2D>,
    pub runtime_config: RuntimeConfig,
    pub resources: ResourceManager,
    pub asset_database: AssetDatabase,
    pub safe_mode: SafeModeSettings,
    pub clock: GameClock,
    pub profiler: Profiler,
    pub diagnostics: Diagnostics,
    pub stability_guard: RuntimeStabilityGuard,
    pub animation_graphs: AnimationGraphLibrary,
    pub audio_mixer: AudioMixer,
    pub audio_system: AudioSystem,
    pub visual_script_runtime: VisualScriptRuntime,
    pub luau_script_runtime: LuauScriptRuntime,
    pub gameplay_system: GameplaySystem,
    pub rts_system: RTSSystem,
    pub runtime_2d_system: Runtime2DSystem,
    pub physics_system: PhysicsSystem,
    pub particle_system: ParticleSystem,
    pub narrative_system: NarrativeSystem,
    pub sprite_animation_system: SpriteAnimationSystem,
}

impl Deref for EngineRuntime {
    type Target = RuntimeWorld;

    fn deref(&self) -> &Self::Target {
        &self.runtime_world
    }
}

impl DerefMut for EngineRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime_world
    }
}

impl EngineRuntime {
    pub fn new(project_path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_project_with_safe_mode(project_path, SafeModeSettings::default())
    }

    pub fn from_project_with_safe_mode(
        project_path: impl AsRef<Path>,
        safe_mode: SafeModeSettings,
    ) -> io::Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();
        let project_paths = AssetTools::get_project_paths(&project_path);
        fs::create_dir_all(&project_paths.logs)?;
        fs::create_dir_all(&project_paths.settings)?;
        let engine_config = EngineConfig::new(&project_path)?;
        let runtime_config =
            RuntimeConfig::new(project_paths.settings.join("runtime_config.json"))?;
        let runtime_tuning = runtime_config.optimized_tuning();
        let stability_guard = RuntimeStabilityGuard::from_runtime_config(
            &runtime_config.data,
            runtime_tuning.max_entities,
        );
        let start_scene = engine_config
            .get("start_scene")
            .and_then(Value::as_str)
            .unwrap_or("main.scene")
            .to_string();
        let mut resources = ResourceManager::new(&project_paths.assets);
        resources.scan_all().ok();
        let asset_database = AssetDatabase::new(&project_paths.assets, &project_path)?;
        let grid = Grid::new(60, 40, 32, 8);
        let tilemap_layers = TilemapLayers::new(grid.width, grid.height);
        let mut camera = Camera::default();
        camera.set_bounds(
            0.0,
            0.0,
            (grid.width * grid.tile_size) as f64,
            (grid.height * grid.tile_size) as f64,
        );
        let mut console = DeveloperConsole::with_log_file(project_paths.logs.join("miniforge.log"));
        console.log(
            format!("Runtime project: {}", project_path.display()),
            "RUNTIME",
        );
        console.log(crate::engine::version::version_label(), "RUNTIME");

        let script_scheduler_config = script_scheduler_config(&runtime_config.data);
        let mut luau_script_runtime = LuauScriptRuntime::new(&project_path);
        luau_script_runtime.set_scheduler_config(script_scheduler_config);

        let mut runtime = Self {
            project_path: project_path.clone(),
            project_paths,
            engine_config,
            mode: "PLAY".to_string(),
            console,
            runtime_world: RuntimeWorld::default(),
            grid,
            tilemap_layers,
            camera,
            scene_manager: SceneManager::new_with_start_scene(&project_path, &start_scene),
            ui_canvases: json!([]),
            ui_documents: Vec::new(),
            runtime_config,
            resources,
            asset_database,
            safe_mode,
            clock: GameClock::from_tuning(&runtime_tuning),
            profiler: Profiler::new(),
            diagnostics: Diagnostics::default(),
            stability_guard,
            animation_graphs: AnimationGraphLibrary::new(),
            audio_mixer: AudioMixer::new(),
            audio_system: AudioSystem::default(),
            visual_script_runtime: VisualScriptRuntime::default(),
            luau_script_runtime,
            gameplay_system: GameplaySystem::default(),
            rts_system: RTSSystem::default(),
            runtime_2d_system: Runtime2DSystem::default(),
            physics_system: PhysicsSystem::new(),
            particle_system: ParticleSystem::default(),
            narrative_system: NarrativeSystem::default(),
            sprite_animation_system: SpriteAnimationSystem::new(&project_path),
        };

        match runtime.scene_manager.load_current_scene_data() {
            Ok(scene) => runtime.apply_scene_data(&scene),
            Err(error) => runtime.console.error(
                format!("No se pudo cargar la escena inicial: {error}"),
                "SCENE",
            ),
        }
        Ok(runtime)
    }

    fn apply_scene_data(&mut self, data: &Value) {
        let scene_name = self.scene_manager.current_scene.clone();
        let entities = data
            .get("entities")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|value| {
                        let mut entity = GameObject::from_data(value, true);
                        entity.scene_name = Some(scene_name.clone());
                        entity.sync_from_components();
                        entity
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.runtime_world.replace_entities(entities);
        self.apply_scene_environment(data);
        self.reload_ui_documents();
    }

    fn apply_scene_environment(&mut self, data: &Value) {
        if let Some(tile_data) = data
            .get("tilemap_layers")
            .or_else(|| data.get("tiles"))
            .filter(|value| value.is_object())
        {
            self.tilemap_layers.deserialize(tile_data);
        }
        let grid_data = data.get("grid").and_then(Value::as_object);
        let configured_width = grid_data
            .and_then(|grid| grid.get("width"))
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(0);
        let configured_height = grid_data
            .and_then(|grid| grid.get("height"))
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(0);
        let width = if self.tilemap_layers.width > 0 {
            self.tilemap_layers.width
        } else {
            configured_width.max(1)
        };
        let height = if self.tilemap_layers.height > 0 {
            self.tilemap_layers.height
        } else {
            configured_height.max(1)
        };
        let tile_size = grid_data
            .and_then(|grid| grid.get("tile_size"))
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(self.grid.tile_size.max(1));
        let chunk_size = grid_data
            .and_then(|grid| grid.get("chunk_size"))
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(self.grid.chunk_size.max(1));
        self.grid = Grid::new(width, height, tile_size, chunk_size);
        if let Some(collision) = self.tilemap_layers.layer("Collision") {
            for y in 0..height {
                for x in 0..width {
                    self.grid
                        .set_tile(x, y, i32::from(collision.get(x, y) != 0));
                }
            }
        }
        self.camera.set_bounds(
            0.0,
            0.0,
            (width * tile_size) as f64,
            (height * tile_size) as f64,
        );
        if let Some(camera) = data.get("camera").and_then(Value::as_object) {
            self.camera.x = camera
                .get("x")
                .and_then(Value::as_f64)
                .unwrap_or(self.camera.x);
            self.camera.y = camera
                .get("y")
                .and_then(Value::as_f64)
                .unwrap_or(self.camera.y);
            self.camera.set_zoom(
                camera
                    .get("zoom")
                    .and_then(Value::as_f64)
                    .unwrap_or(self.camera.zoom),
            );
            self.camera.clamp_to_bounds();
        }
        self.ui_canvases = data
            .get("ui_canvases")
            .cloned()
            .unwrap_or_else(|| json!([]));
    }

    /// Reloads the retained-mode `.ui2d.json` documents referenced by visible
    /// `WidgetCanvas2D` components.
    ///
    /// Runtime documents are restricted to files inside the project, bounded
    /// to four MiB, validated for unique widget ids and capped at 10k widgets.
    /// A broken document is isolated and reported without preventing the game
    /// from loading.
    pub fn reload_ui_documents(&mut self) -> RuntimeUiDocumentLoadReport {
        let references = self
            .runtime_world
            .units
            .iter()
            .filter_map(|entity| {
                let component = entity.get_component("WidgetCanvas2D")?;
                let asset_path = component.get_string("canvas", "");
                (entity.enabled
                    && entity.visible
                    && component.enabled
                    && component.get_bool("visible", true)
                    && !asset_path.trim().is_empty())
                .then_some((
                    entity.id,
                    asset_path,
                    component.get_bool("input_enabled", true),
                    component.get_string("scale_mode", "scale_with_screen"),
                ))
            })
            .collect::<Vec<_>>();
        let mut cache = BTreeMap::<String, UiCanvas2D>::new();
        let mut documents = Vec::new();
        let mut report = RuntimeUiDocumentLoadReport::default();
        for (entity_id, asset_path, input_enabled, scale_mode) in references {
            let loaded = if let Some(canvas) = cache.get(&asset_path) {
                Ok(canvas.clone())
            } else {
                load_ui_canvas_asset(&self.project_path, &asset_path).inspect(|canvas| {
                    cache.insert(asset_path.clone(), canvas.clone());
                })
            };
            match loaded {
                Ok(canvas) => {
                    documents.push(RuntimeUiDocument2D {
                        entity_id,
                        asset_path,
                        input_enabled,
                        scale_mode,
                        canvas,
                    });
                    report.loaded += 1;
                }
                Err(error) => {
                    report.skipped += 1;
                    if report.errors.len() < 64 {
                        report
                            .errors
                            .push(format!("entity {entity_id}: {asset_path}: {error}"));
                    }
                }
            }
        }
        self.ui_documents = documents;
        for error in report.errors.iter().take(6) {
            self.console.warning(error.clone(), "UI");
        }
        if report.loaded > 0 {
            self.console.log(
                format!(
                    "{} UI document(s) retained loaded; {} skipped",
                    report.loaded, report.skipped
                ),
                "UI",
            );
        }
        report
    }

    pub fn run_headless_once(&mut self, dt: f64) {
        self.console.advance_frame();
        self.profiler.begin_frame();
        let mut marker = Instant::now();
        let safe_dt = self.stability_guard.begin_frame(dt);
        let clock_advance = self.clock.advance(safe_dt);
        let simulation_dt = clock_advance.scaled_dt;
        self.stability_guard
            .sanitize_entities(&mut self.runtime_world.units);
        self.record_system("StabilityPreflight", &mut marker);
        self.sprite_animation_system.update_entities(
            &mut self.runtime_world.units,
            simulation_dt,
            &self.mode,
        );
        self.record_system("SpriteFrames2D", &mut marker);
        AnimationSystem.update_entities(
            &mut self.runtime_world.units,
            &self.animation_graphs,
            simulation_dt,
            &self.mode,
        );
        self.record_system("Animation", &mut marker);
        if let Some(particle_dt) = self.stability_guard.optional_system_delta(simulation_dt) {
            self.particle_system.update_entities(
                &self.runtime_world.units,
                particle_dt,
                &self.mode,
            );
        }
        self.record_system("Particles", &mut marker);
        self.audio_system.update_entities(
            &mut self.runtime_world.units,
            &self.audio_mixer,
            &self.mode,
        );
        self.record_system("Audio", &mut marker);
        if self.safe_mode.allows_graphs() {
            self.visual_script_runtime.update_entities(
                &mut self.runtime_world.units,
                simulation_dt,
                &self.mode,
            );
        }
        self.record_system("VisualGraph", &mut marker);
        if self.safe_mode.allows_scripts() {
            self.luau_script_runtime.set_camera_state(&self.camera);
            let report = self.luau_script_runtime.update_entities_with_fixed_steps(
                &mut self.runtime_world.units,
                simulation_dt,
                self.clock.fixed_delta,
                clock_advance.fixed_steps,
                &self.mode,
            );
            self.handle_luau_report(report);
        }
        self.record_system("Luau", &mut marker);
        self.runtime_2d_system.update_entities(
            &mut self.runtime_world.units,
            simulation_dt,
            &self.mode,
        );
        self.record_system("Runtime2D", &mut marker);
        self.gameplay_system.update_entities_with_grid(
            &mut self.runtime_world.units,
            &self.grid,
            simulation_dt,
            &self.mode,
        );
        self.record_system("Gameplay", &mut marker);
        self.rts_system
            .update_entities(&mut self.runtime_world.units, simulation_dt, &self.mode);
        self.record_system("RTS", &mut marker);
        MovementSystem.update_entities(&mut self.runtime_world.units, simulation_dt);
        self.record_system("Movement", &mut marker);
        self.physics_system.update_entities_mut(
            &mut self.runtime_world.units,
            simulation_dt,
            &self.mode,
        );
        self.record_system("Physics", &mut marker);
        self.runtime_2d_system.resolve_tilemap_collisions(
            &mut self.runtime_world.units,
            &self.grid,
            &self.mode,
        );
        self.runtime_2d_system.advance_camera_shakes(
            &mut self.runtime_world.units,
            simulation_dt,
            &self.mode,
        );
        self.runtime_2d_system.update_camera(
            &mut self.camera,
            &self.runtime_world.units,
            simulation_dt,
            &self.mode,
            self.grid.tile_size.max(1) as f64,
        );
        self.record_system("Runtime2DLate", &mut marker);
        self.dispatch_collision_scripts();
        self.record_system("LuauCollision", &mut marker);
        self.stability_guard
            .sanitize_entities(&mut self.runtime_world.units);
        self.record_system("StabilityPostflight", &mut marker);
        self.runtime_world.mark_changed();
        self.runtime_world.rebuild_index();
        self.record_system("WorldSync", &mut marker);

        self.stability_guard.observe_frame(
            clock_advance,
            self.profiler.systems_time_total_ms(),
            self.runtime_world.units.len(),
        );
        self.diagnostics
            .update_with_budget(safe_dt, clock_advance.target_frame_delta * 1000.0);
        self.diagnostics.record_frame_runtime(
            clock_advance,
            self.clock.fixed_delta,
            self.runtime_world.units.len(),
            self.profiler.systems_time_total_ms(),
            self.profiler.slowest_system(),
        );
        for warning in self.stability_guard.take_events() {
            self.diagnostics.push_warning(warning.clone());
            self.console.log(warning, "STABILITY");
        }
        self.profiler
            .set_counter("Entities", self.runtime_world.units.len());
        self.profiler
            .set_counter("LuauScripts", self.luau_script_runtime.last_frame_scripts);
        self.profiler.set_counter(
            "LuauUpdateCandidates",
            self.luau_script_runtime
                .last_scheduler_stats
                .update_candidates,
        );
        self.profiler.set_counter(
            "LuauUpdateBudgetUsed",
            self.luau_script_runtime
                .last_scheduler_stats
                .update_budget_used,
        );
        self.profiler.set_counter(
            "LuauSkippedBudget",
            self.luau_script_runtime.last_scheduler_stats.skipped_budget,
        );
        self.profiler.set_counter(
            "LuauSkippedInterval",
            self.luau_script_runtime
                .last_scheduler_stats
                .skipped_interval,
        );
        self.profiler.set_counter(
            "LuauDistanceThrottled",
            self.luau_script_runtime
                .last_scheduler_stats
                .distance_throttled,
        );
        self.profiler.set_counter(
            "LuauNearbyQueries",
            self.luau_script_runtime.last_query_stats.nearby_queries,
        );
        self.profiler.set_counter(
            "LuauNearbyIndexed",
            self.luau_script_runtime.last_query_stats.nearby_indexed,
        );
        self.profiler.set_counter(
            "LuauNearbyLinearScans",
            self.luau_script_runtime
                .last_query_stats
                .nearby_linear_scans,
        );
        self.profiler.set_counter(
            "LuauNearbyCandidates",
            self.luau_script_runtime.last_query_stats.nearby_candidates,
        );
        self.profiler
            .set_counter("FixedTicks", clock_advance.fixed_steps);
        self.profiler
            .set_counter("FrameOverBudget", usize::from(clock_advance.over_budget));
        self.profiler.set_counter(
            "FixedStepSaturated",
            usize::from(clock_advance.saturated_fixed_steps),
        );
        self.profiler
            .set_metric("FrameBudgetMs", clock_advance.target_frame_delta * 1000.0);
        self.profiler
            .set_metric("FrameDtMs", clock_advance.scaled_dt * 1000.0);
        self.profiler
            .set_metric("FixedStepMs", self.clock.fixed_delta * 1000.0);
        self.profiler
            .set_metric("InterpolationAlpha", clock_advance.interpolation_alpha);
        self.profiler
            .set_metric("DroppedTimeMs", clock_advance.dropped_time * 1000.0);
        self.profiler.set_counter(
            "StabilityLevel",
            match self.stability_guard.level() {
                crate::engine::runtime_stability::StabilityLevel::Stable => 0,
                crate::engine::runtime_stability::StabilityLevel::Guarded => 1,
                crate::engine::runtime_stability::StabilityLevel::Recovery => 2,
            },
        );
        self.profiler.set_counter(
            "StabilityRepairs",
            self.stability_guard.last_frame.repaired_values,
        );
        self.profiler.set_counter(
            "StabilityQuarantined",
            self.stability_guard.quarantined_entity_count(),
        );
        self.profiler.set_counter(
            "StabilityOptionalCadence",
            self.stability_guard.last_frame.optional_cadence_divisor as usize,
        );
        self.profiler.set_counter(
            "StabilityEntityOverflow",
            self.stability_guard.last_frame.entity_limit_exceeded_by,
        );
        self.profiler.set_metric(
            "RawFrameDtMs",
            self.stability_guard.last_frame.raw_delta_seconds * 1000.0,
        );
        self.profiler.set_metric(
            "SafeFrameDtMs",
            self.stability_guard.last_frame.safe_delta_seconds * 1000.0,
        );
        self.profiler
            .set_counter("SpatialCells", self.runtime_world.spatial_index.cells.len());
        self.profiler.end_frame();
    }

    fn record_system(&mut self, name: &str, marker: &mut Instant) {
        self.profiler
            .record_system(name, marker.elapsed().as_secs_f64() * 1000.0);
        *marker = Instant::now();
    }

    fn dispatch_collision_scripts(&mut self) {
        if !self.safe_mode.allows_scripts() {
            return;
        }
        let mut report = LuauRunReport::default();
        for event in self
            .physics_system
            .events
            .clone()
            .into_iter()
            .filter(|event| {
                matches!(
                    event.phase,
                    PhysicsEventPhase::Enter | PhysicsEventPhase::Exit
                )
            })
        {
            match event.phase {
                PhysicsEventPhase::Enter => {
                    report.merge(self.luau_script_runtime.run_collision_enter(
                        &mut self.runtime_world.units,
                        event.first_id,
                        event.second_name.clone(),
                    ));
                    report.merge(self.luau_script_runtime.run_custom_event_for_entity(
                        &mut self.runtime_world.units,
                        event.first_id,
                        physics_event_name(event.pair_type, event.phase),
                        json!({
                            "self_id": event.first_id,
                            "other_id": event.second_id,
                            "other_name": event.second_name,
                            "pair_type": physics_pair_type_name(event.pair_type),
                            "phase": "enter",
                            "normal": {"x": event.normal.0, "y": event.normal.1},
                            "depth": event.depth,
                        }),
                    ));
                    report.merge(self.luau_script_runtime.run_collision_enter(
                        &mut self.runtime_world.units,
                        event.second_id,
                        event.first_name.clone(),
                    ));
                    report.merge(self.luau_script_runtime.run_custom_event_for_entity(
                        &mut self.runtime_world.units,
                        event.second_id,
                        physics_event_name(event.pair_type, event.phase),
                        json!({
                            "self_id": event.second_id,
                            "other_id": event.first_id,
                            "other_name": event.first_name,
                            "pair_type": physics_pair_type_name(event.pair_type),
                            "phase": "enter",
                            "normal": {"x": -event.normal.0, "y": -event.normal.1},
                            "depth": event.depth,
                        }),
                    ));
                }
                PhysicsEventPhase::Exit => {
                    report.merge(self.luau_script_runtime.run_collision_exit(
                        &mut self.runtime_world.units,
                        event.first_id,
                        event.second_name.clone(),
                    ));
                    report.merge(self.luau_script_runtime.run_custom_event_for_entity(
                        &mut self.runtime_world.units,
                        event.first_id,
                        physics_event_name(event.pair_type, event.phase),
                        json!({
                            "self_id": event.first_id,
                            "other_id": event.second_id,
                            "other_name": event.second_name,
                            "pair_type": physics_pair_type_name(event.pair_type),
                            "phase": "exit",
                            "normal": {"x": event.normal.0, "y": event.normal.1},
                            "depth": event.depth,
                        }),
                    ));
                    report.merge(self.luau_script_runtime.run_collision_exit(
                        &mut self.runtime_world.units,
                        event.second_id,
                        event.first_name.clone(),
                    ));
                    report.merge(self.luau_script_runtime.run_custom_event_for_entity(
                        &mut self.runtime_world.units,
                        event.second_id,
                        physics_event_name(event.pair_type, event.phase),
                        json!({
                            "self_id": event.second_id,
                            "other_id": event.first_id,
                            "other_name": event.first_name,
                            "pair_type": physics_pair_type_name(event.pair_type),
                            "phase": "exit",
                            "normal": {"x": -event.normal.0, "y": -event.normal.1},
                            "depth": event.depth,
                        }),
                    ));
                }
                PhysicsEventPhase::Stay => {}
            }
        }
        self.handle_luau_report(report);
    }

    fn handle_luau_report(&mut self, report: LuauRunReport) {
        for error in report.errors.iter().take(6) {
            self.console.log(error.clone(), "SCRIPT");
        }
        for message in report.debug_messages.iter().take(8) {
            self.console.log(message.clone(), "SCRIPT");
        }
        if let Some(scene) = report.scene_requests.first()
            && let Err(error) = self.load_scene(scene)
        {
            self.console
                .error(format!("load_scene({scene}) falló: {error}"), "SCRIPT");
        }
        if !report.spawned.is_empty() || !report.destroyed.is_empty() || report.ui_updates > 0 {
            self.runtime_world.mark_changed();
            self.runtime_world.rebuild_index();
        }
    }

    pub fn load_scene(&mut self, name: &str) -> io::Result<usize> {
        let data = self.scene_manager.load_scene_data(name)?;
        let entities = self
            .scene_manager
            .load_scene(name, &self.runtime_world.units)?;
        self.runtime_world.replace_entities(entities);
        self.apply_scene_environment(&data);
        self.reload_ui_documents();
        Ok(self.runtime_world.units.len())
    }

    pub fn get_entity_by_id(&self, entity_id: u64) -> Option<&GameObject> {
        self.runtime_world.entity(entity_id)
    }

    pub fn get_entity_by_id_mut(&mut self, entity_id: u64) -> Option<&mut GameObject> {
        self.runtime_world.entity_mut(entity_id)
    }

    pub fn dispatch_script_key_down(&mut self, key: &str) {
        if self.safe_mode.allows_scripts() {
            let report = self
                .luau_script_runtime
                .run_key_down(&mut self.runtime_world.units, key);
            self.handle_luau_report(report);
        }
    }

    pub fn interact(&mut self) -> Option<NarrativeEvent> {
        self.narrative_system
            .interact(&mut self.runtime_world.units)
    }

    pub fn choose_dialogue(&mut self, choice_index: usize) -> Option<NarrativeEvent> {
        self.narrative_system
            .choose(&mut self.runtime_world.units, choice_index)
    }

    pub fn set_script_input_pressed(&mut self, key: &str, pressed: bool) {
        if self.safe_mode.allows_scripts() {
            self.luau_script_runtime.set_input_pressed(key, pressed);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_character_input_for_tag(
        &mut self,
        tag: &str,
        movement: (f64, f64),
        jump_pressed: bool,
        jump_held: bool,
        run_pressed: bool,
        dash_pressed: bool,
    ) -> usize {
        let mut updated = 0;
        for entity in &mut self.runtime_world.units {
            if entity.tag != tag {
                continue;
            }
            let Some(controller) = entity.get_component_mut("CharacterController2D") else {
                continue;
            };
            controller.set_f64("input_x", movement.0.clamp(-1.0, 1.0));
            controller.set_f64("input_y", movement.1.clamp(-1.0, 1.0));
            controller.set("jump_pressed", json!(jump_pressed));
            controller.set("jump_held", json!(jump_held));
            controller.set("run_pressed", json!(run_pressed));
            controller.set("dash_pressed", json!(dash_pressed));
            updated += 1;
        }
        updated
    }
}

fn load_ui_canvas_asset(project_path: &Path, asset_path: &str) -> Result<UiCanvas2D, String> {
    let relative = Path::new(asset_path.trim());
    if relative.as_os_str().is_empty()
        || asset_path.contains('\\')
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                PathComponent::ParentDir | PathComponent::RootDir | PathComponent::Prefix(_)
            )
        })
    {
        return Err("path must be a project-relative asset without traversal".to_string());
    }
    let project_root = project_path
        .canonicalize()
        .map_err(|error| format!("project root unavailable: {error}"))?;
    let candidate = project_path.join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("asset unavailable: {error}"))?;
    if !canonical.starts_with(&project_root) {
        return Err("asset resolves outside the project".to_string());
    }
    let metadata = fs::metadata(&canonical).map_err(|error| format!("metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("asset is not a file".to_string());
    }
    if metadata.len() > MAX_UI_DOCUMENT_BYTES {
        return Err(format!(
            "document exceeds {} byte limit",
            MAX_UI_DOCUMENT_BYTES
        ));
    }
    let bytes = fs::read(&canonical).map_err(|error| format!("read failed: {error}"))?;
    if bytes.len() as u64 > MAX_UI_DOCUMENT_BYTES {
        return Err(format!(
            "document exceeds {} byte limit",
            MAX_UI_DOCUMENT_BYTES
        ));
    }
    let canvas: UiCanvas2D =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid JSON: {error}"))?;
    if !canvas.viewport_width.is_finite()
        || !canvas.viewport_height.is_finite()
        || canvas.viewport_width <= 0.0
        || canvas.viewport_height <= 0.0
    {
        return Err("canvas viewport must be finite and positive".to_string());
    }
    if !canvas.validate_widget_ids() {
        return Err("canvas contains duplicate widget ids".to_string());
    }
    let widget_count = canvas.flatten_widgets().len();
    if widget_count > MAX_UI_DOCUMENT_WIDGETS {
        return Err(format!(
            "canvas contains {widget_count} widgets; limit is {MAX_UI_DOCUMENT_WIDGETS}"
        ));
    }
    Ok(canvas)
}

#[cfg(test)]
mod ui_document_tests {
    use super::*;

    fn temporary_project(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("miniforge-{name}-{unique}"));
        fs::create_dir_all(root.join("assets/ui")).expect("UI directory");
        root
    }

    #[test]
    fn ui_asset_loader_accepts_project_assets_and_rejects_traversal() {
        let root = temporary_project("ui-loader");
        fs::write(
            root.join("assets/ui/hud.ui2d.json"),
            br#"{
                "name":"HUD",
                "viewport_width":320.0,
                "viewport_height":180.0,
                "widgets":[],
                "theme":{"name":"Test","styles":{}},
                "animations":[]
            }"#,
        )
        .expect("write UI document");

        let canvas =
            load_ui_canvas_asset(&root, "assets/ui/hud.ui2d.json").expect("project UI document");
        assert_eq!(canvas.name, "HUD");
        let document = RuntimeUiDocument2D {
            entity_id: 7,
            asset_path: "assets/ui/hud.ui2d.json".to_string(),
            input_enabled: true,
            scale_mode: "scale_with_screen".to_string(),
            canvas,
        };
        assert_eq!(document.layout_viewport((640.0, 360.0)), (320.0, 180.0));
        assert_eq!(document.layout_scale((640.0, 360.0)), (2.0, 2.0));
        assert_eq!(
            document.screen_to_layout((640.0, 360.0), (200.0, 100.0)),
            (100.0, 50.0)
        );
        let error = load_ui_canvas_asset(&root, "../outside.ui2d.json").unwrap_err();
        assert!(error.contains("without traversal"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn ui_asset_loader_rejects_duplicate_widget_ids() {
        let root = temporary_project("ui-duplicates");
        fs::write(
            root.join("assets/ui/hud.ui2d.json"),
            br#"{
                "name":"HUD",
                "viewport_width":320.0,
                "viewport_height":180.0,
                "widgets":[
                    {"id":"same","widget_type":"Panel","rect":{"x":0.0,"y":0.0,"width":10.0,"height":10.0},"anchors":{"min_x":0.0,"min_y":0.0,"max_x":0.0,"max_y":0.0}},
                    {"id":"same","widget_type":"Panel","rect":{"x":20.0,"y":0.0,"width":10.0,"height":10.0},"anchors":{"min_x":0.0,"min_y":0.0,"max_x":0.0,"max_y":0.0}}
                ],
                "theme":{"name":"Test","styles":{}},
                "animations":[]
            }"#,
        )
        .expect("write UI document");

        let error =
            load_ui_canvas_asset(&root, "assets/ui/hud.ui2d.json").expect_err("duplicates fail");
        assert!(error.contains("duplicate widget ids"));
        fs::remove_dir_all(root).ok();
    }
}
