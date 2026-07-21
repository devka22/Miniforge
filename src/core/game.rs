use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{Value, json};

use crate::core::engine_config::EngineConfig;
use crate::engine::advanced_prefabs::AdvancedPrefabSystem;
use crate::engine::animation_graph::AnimationGraphLibrary;
use crate::engine::archetype_library::ArchetypeLibrary;
use crate::engine::asset_database::AssetDatabase;
use crate::engine::asset_tools::{AssetTools, ProjectPaths};
use crate::engine::audio_mixer::AudioMixer;
use crate::engine::autosave_manager::AutosaveManager;
use crate::engine::build_placement::{BuildFootprint, BuildPlacement};
use crate::engine::build_profiles::BuildProfiles;
use crate::engine::build_settings::BuildSettings;
use crate::engine::camera::Camera;
use crate::engine::component::{Component, default_component};
use crate::engine::component_registry::ComponentRegistry;
use crate::engine::content_drag::{ContentDropper, DragAssetKind, DragPayload, DropOutcome};
use crate::engine::developer_console::DeveloperConsole;
use crate::engine::diagnostics::Diagnostics;
use crate::engine::docking_panel::{EditorDockTab, EditorDockingWorkspace};
use crate::engine::editor_command::{EditorCommand, EditorCommandKind, EditorSnapshot};
use crate::engine::editor_history::EditorHistory;
use crate::engine::editor_workspace::{EditorWorkspace, WorkspaceMode};
use crate::engine::engine_backend::{EngineBackend, EngineBackendPlan};
use crate::engine::engine_programming::{
    ProgrammingEnvironment, VisualGraphQuickAction, VisualGraphView,
};
use crate::engine::entity_id::generate_entity_name;
use crate::engine::event_bus::EventBus;
use crate::engine::game_clock::GameClock;
use crate::engine::hierarchy_manager::HierarchyManager;
use crate::engine::input_map::InputMap;
use crate::engine::inspector_editor::InspectorEditor;
use crate::engine::luau_scripting::{LuauRunReport, LuauScriptRuntime};
use crate::engine::manifest_builder::ManifestBuilder;
use crate::engine::material_system::MaterialLibrary;
use crate::engine::native_library::NativeLibraryManager;
use crate::engine::play_mode_manager::PlayModeManager;
use crate::engine::prefab_manager::PrefabManager;
use crate::engine::prefab_serializer::PrefabSerializer;
use crate::engine::profiler::Profiler;
use crate::engine::project_package::{ProjectPackageManager, ProjectPackageReport};
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
use crate::engine::project_templates::ProjectTemplates;
use crate::engine::project_validator::ProjectValidator;
use crate::engine::resource_manager::ResourceManager;
use crate::engine::runtime_config::RuntimeConfig;
use crate::engine::runtime_exporter::{ExportProfile, RuntimeExportReport, RuntimeExporter};
use crate::engine::runtime_stability::RuntimeStabilityGuard;
use crate::engine::safe_mode::SafeModeSettings;
use crate::engine::scene_manager::SceneManager;
use crate::engine::scene_save_manager::SceneSaveManager;
use crate::engine::scene_validator::SceneValidator;
use crate::engine::script_debugger::ScriptDebugger;
use crate::engine::script_editor::ScriptEditor;
use crate::engine::sprite_editor::{SpriteColor, SpriteEditorCanvas};
use crate::engine::tags_layers_manager::TagsLayersManager;
use crate::engine::tile_brush::{TileBrush, TileBrushMode};
use crate::engine::tilemap_layers::TilemapLayers;
use crate::engine::ui_canvas::{
    UiCanvasElement, UiCanvasRoot, UiRect, push_canvas, ui_canvases_from_value,
};
use crate::engine::ui_runtime::UiRuntime;
use crate::engine::upgrade_manifest::EngineUpgradeManifest;
use crate::engine::visual_scripting::VisualScriptRuntime;
use crate::engine::world::RuntimeWorld;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;
use crate::map::pathfinding::Point;
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

const COMPLEX_FOUNDATION_PLAYER_COMPONENTS: &[&str] = &[
    "Health",
    "Stats",
    "Rigidbody2D",
    "CharacterController2D",
    "CameraFollow",
    "Inventory",
    "Equipment",
    "Ability",
    "QuestLog",
    "Saveable",
    "Cooldown",
    "StatusEffects",
    "Interaction",
    "CombatTarget",
    "NavAgent",
    "VisualScript",
];

const COMPLEX_FOUNDATION_SYSTEM_COMPONENTS: &[&str] = &[
    "RuntimeBudget2D",
    "WorldPartition2D",
    "ObjectPool2D",
    "SpawnDirector2D",
    "SaveShard2D",
    "EconomyWallet",
    "DontDestroyOnLoad",
];

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComplexGameFoundationReport {
    pub target_entity_id: u64,
    pub systems_entity_id: Option<u64>,
    pub created_player: bool,
    pub created_systems_entity: bool,
    pub created_ui_canvas: bool,
    pub ui_canvas_ready: bool,
    pub identity_changed: bool,
    pub configured_components: Vec<String>,
    pub added_player_components: Vec<String>,
    pub added_system_components: Vec<String>,
    pub missing_components: Vec<String>,
    pub changed: bool,
}

#[derive(Debug)]
pub struct Game {
    pub runtime_mode: bool,
    pub project_path: PathBuf,
    pub project_paths: ProjectPaths,
    pub engine_config: EngineConfig,
    pub mode: String,
    pub active_tool: String,
    pub tile_brush: i32,
    pub brush_size: usize,
    pub scene_dirty: bool,
    pub scene_dirty_reason: String,
    pub console: DeveloperConsole,
    pub runtime_world: RuntimeWorld,
    pub selected_units: Vec<u64>,
    pub grid: Grid,
    pub tilemap_layers: TilemapLayers,
    pub camera: Camera,
    pub clock: GameClock,
    pub event_bus: EventBus,
    pub resources: ResourceManager,
    pub asset_database: AssetDatabase,
    pub input_map: InputMap,
    pub build_settings: BuildSettings,
    pub build_profiles: BuildProfiles,
    pub runtime_config: RuntimeConfig,
    pub tags_layers_manager: TagsLayersManager,
    pub component_registry: ComponentRegistry,
    pub scene_manager: SceneManager,
    pub scene_validator: SceneValidator,
    pub scene_save_manager: SceneSaveManager,
    pub ui_canvases: Value,
    pub autosave_manager: AutosaveManager,
    pub history: EditorHistory,
    pub profiler: Profiler,
    pub diagnostics: Diagnostics,
    pub stability_guard: RuntimeStabilityGuard,
    pub docking_workspace: EditorDockingWorkspace,
    pub audio_mixer: AudioMixer,
    pub audio_system: AudioSystem,
    pub animation_graphs: AnimationGraphLibrary,
    pub material_library: MaterialLibrary,
    pub native_libraries: NativeLibraryManager,
    pub safe_mode: SafeModeSettings,
    pub visual_script_runtime: VisualScriptRuntime,
    pub luau_script_runtime: LuauScriptRuntime,
    pub script_debugger: ScriptDebugger,
    pub script_editor: ScriptEditor,
    pub sprite_editor: SpriteEditorCanvas,
    pub ui_runtime: UiRuntime,
    pub play_mode_manager: PlayModeManager,
    pub gameplay_system: GameplaySystem,
    pub rts_system: RTSSystem,
    pub runtime_2d_system: Runtime2DSystem,
    pub physics_system: PhysicsSystem,
    pub particle_system: ParticleSystem,
    pub narrative_system: NarrativeSystem,
    pub sprite_animation_system: SpriteAnimationSystem,
    pub advanced_prefabs: AdvancedPrefabSystem,
    pub archetypes: ArchetypeLibrary,
    pub upgrade_manifest: EngineUpgradeManifest,
    pub editor_workspace: EditorWorkspace,
    pub programming: ProgrammingEnvironment,
}

impl Deref for Game {
    type Target = RuntimeWorld;

    fn deref(&self) -> &Self::Target {
        &self.runtime_world
    }
}

impl DerefMut for Game {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime_world
    }
}

fn set_component_value_if_changed(component: &mut Component, key: &str, value: Value) -> bool {
    if component.get(key) == Some(&value) {
        return false;
    }
    component.set(key, value);
    true
}

fn push_configured(configured: &mut Vec<String>, name: &str, changed: bool) {
    if changed {
        configured.push(name.to_string());
    }
}

fn ensure_scene_node_components(entity: &mut GameObject) {
    for component_type in ["Node2D", "SceneTreeNode"] {
        if entity.get_component(component_type).is_none()
            && let Some(component) = default_component(component_type)
        {
            entity.add_component(component);
        }
    }
}

fn configure_complex_foundation_player(entity: &mut GameObject, entity_id: u64) -> Vec<String> {
    let mut configured = Vec::new();

    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
        let mut changed = false;
        changed |= set_component_value_if_changed(body, "use_gravity", json!(false));
        changed |= set_component_value_if_changed(body, "freeze_rotation", json!(true));
        changed |= set_component_value_if_changed(body, "drag", json!(0.2));
        push_configured(&mut configured, "Rigidbody2D", changed);
    }

    if let Some(controller) = entity.get_component_mut("CharacterController2D") {
        let mut changed = false;
        changed |= set_component_value_if_changed(controller, "mode", json!("topdown"));
        changed |= set_component_value_if_changed(controller, "walk_speed", json!(5.25));
        changed |= set_component_value_if_changed(controller, "run_speed", json!(7.25));
        changed |= set_component_value_if_changed(controller, "dash_speed", json!(13.0));
        changed |= set_component_value_if_changed(controller, "dash_cooldown", json!(0.5));
        changed |= set_component_value_if_changed(controller, "input_enabled", json!(true));
        push_configured(&mut configured, "CharacterController2D", changed);
    }

    if let Some(camera) = entity.get_component_mut("CameraFollow") {
        let mut changed = false;
        changed |= set_component_value_if_changed(camera, "target_id", json!(entity_id));
        changed |= set_component_value_if_changed(camera, "zoom", json!(1.1));
        changed |= set_component_value_if_changed(camera, "dead_zone", json!(0.25));
        push_configured(&mut configured, "CameraFollow", changed);
    }

    if let Some(saveable) = entity.get_component_mut("Saveable") {
        let mut changed = false;
        changed |= set_component_value_if_changed(saveable, "save_key", json!("player"));
        changed |= set_component_value_if_changed(saveable, "autosave", json!(true));
        changed |= set_component_value_if_changed(saveable, "include_components", json!(true));
        push_configured(&mut configured, "Saveable", changed);
    }

    if let Some(ability) = entity.get_component_mut("Ability") {
        let mut changed = false;
        changed |= set_component_value_if_changed(ability, "ability_id", json!("dash"));
        changed |= set_component_value_if_changed(ability, "display_name", json!("Dash"));
        changed |= set_component_value_if_changed(ability, "cooldown", json!(0.8));
        changed |= set_component_value_if_changed(ability, "target_mode", json!("self"));
        push_configured(&mut configured, "Ability", changed);
    }

    if let Some(interaction) = entity.get_component_mut("Interaction") {
        let mut changed = false;
        changed |= set_component_value_if_changed(interaction, "prompt", json!("Interact"));
        changed |= set_component_value_if_changed(interaction, "radius", json!(1.35));
        changed |= set_component_value_if_changed(interaction, "requires_tag", json!("Player"));
        push_configured(&mut configured, "Interaction", changed);
    }

    if let Some(combat_target) = entity.get_component_mut("CombatTarget") {
        let mut changed = false;
        changed |= set_component_value_if_changed(combat_target, "target_tags", json!(["Enemy"]));
        changed |= set_component_value_if_changed(combat_target, "attack_radius", json!(1.35));
        push_configured(&mut configured, "CombatTarget", changed);
    }

    if let Some(nav_agent) = entity.get_component_mut("NavAgent") {
        let mut changed = false;
        changed |= set_component_value_if_changed(nav_agent, "speed", json!(5.0));
        changed |= set_component_value_if_changed(nav_agent, "auto_repath", json!(true));
        changed |= set_component_value_if_changed(nav_agent, "path_smoothing", json!(true));
        push_configured(&mut configured, "NavAgent", changed);
    }

    if let Some(visual) = entity.get_component_mut("VisualScript") {
        let mut changed = false;
        changed |= set_component_value_if_changed(visual, "graph_name", json!("PlayerFoundation"));
        changed |= set_component_value_if_changed(
            visual,
            "enabled_events",
            json!(["start", "update", "collision"]),
        );
        push_configured(&mut configured, "VisualScript", changed);
    }

    configured
}

fn configure_complex_foundation_systems(entity: &mut GameObject) -> Vec<String> {
    let mut configured = Vec::new();

    if let Some(budget) = entity.get_component_mut("RuntimeBudget2D") {
        let mut changed = false;
        changed |= set_component_value_if_changed(budget, "target_fps", json!(60));
        changed |= set_component_value_if_changed(budget, "max_script_ms", json!(4.0));
        changed |= set_component_value_if_changed(budget, "max_physics_ms", json!(4.0));
        changed |= set_component_value_if_changed(budget, "max_ui_ms", json!(2.0));
        push_configured(&mut configured, "RuntimeBudget2D", changed);
    }

    if let Some(partition) = entity.get_component_mut("WorldPartition2D") {
        let mut changed = false;
        changed |= set_component_value_if_changed(partition, "streaming_enabled", json!(true));
        changed |= set_component_value_if_changed(partition, "load_radius_cells", json!(2));
        changed |= set_component_value_if_changed(partition, "keepalive_radius_cells", json!(3));
        push_configured(&mut configured, "WorldPartition2D", changed);
    }

    if let Some(pool) = entity.get_component_mut("ObjectPool2D") {
        let mut changed = false;
        changed |= set_component_value_if_changed(pool, "enabled", json!(true));
        push_configured(&mut configured, "ObjectPool2D", changed);
    }

    if let Some(spawn_director) = entity.get_component_mut("SpawnDirector2D") {
        let mut changed = false;
        changed |= set_component_value_if_changed(spawn_director, "enabled", json!(false));
        changed |= set_component_value_if_changed(spawn_director, "max_spawn_per_tick", json!(8));
        push_configured(&mut configured, "SpawnDirector2D", changed);
    }

    if let Some(save_shard) = entity.get_component_mut("SaveShard2D") {
        let mut changed = false;
        changed |= set_component_value_if_changed(save_shard, "autosave_dirty_shards", json!(true));
        changed |= set_component_value_if_changed(
            save_shard,
            "global_save_path",
            json!("saves/profile/global.json"),
        );
        push_configured(&mut configured, "SaveShard2D", changed);
    }

    if let Some(wallet) = entity.get_component_mut("EconomyWallet") {
        let mut changed = false;
        changed |= set_component_value_if_changed(
            wallet,
            "resources",
            json!({"Gold": 0, "Wood": 0, "Gems": 0}),
        );
        changed |= set_component_value_if_changed(wallet, "allow_negative", json!(false));
        push_configured(&mut configured, "EconomyWallet", changed);
    }

    if let Some(persistent) = entity.get_component_mut("DontDestroyOnLoad") {
        let mut changed = false;
        changed |= set_component_value_if_changed(persistent, "preserve", json!(true));
        changed |= set_component_value_if_changed(persistent, "group", json!("systems"));
        push_configured(&mut configured, "DontDestroyOnLoad", changed);
    }

    configured
}

impl Game {
    pub fn new(runtime_mode: bool) -> io::Result<Self> {
        Self::from_project(std::env::current_dir()?, runtime_mode)
    }

    pub fn from_project(project_path: impl AsRef<Path>, runtime_mode: bool) -> io::Result<Self> {
        Self::from_project_with_safe_mode(project_path, runtime_mode, SafeModeSettings::default())
    }

    pub fn from_project_with_safe_mode(
        project_path: impl AsRef<Path>,
        runtime_mode: bool,
        safe_mode: SafeModeSettings,
    ) -> io::Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();
        let project_paths = AssetTools::ensure_project_folders(&project_path)?;
        let engine_config = EngineConfig::new(&project_path)?;
        let autosave_interval_seconds = engine_config
            .get("autosave_interval_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(60)
            .clamp(5, 3600);
        let config_warnings = engine_config.status.warnings.clone();
        let mut resources = ResourceManager::new(&project_paths.assets);
        resources.scan_all().ok();
        let asset_database = AssetDatabase::new(&project_paths.assets, &project_path)?;
        let input_map = InputMap::new(project_paths.settings.join("input_map.json"))?;
        let build_settings =
            BuildSettings::new(project_paths.settings.join("build_settings.json"))?;
        let build_profiles =
            BuildProfiles::new(project_paths.settings.join("build_profiles.json"))?;
        let runtime_config =
            RuntimeConfig::new(project_paths.settings.join("runtime_config.json"))?;
        let runtime_tuning = runtime_config.optimized_tuning();
        let stability_guard = RuntimeStabilityGuard::from_runtime_config(
            &runtime_config.data,
            runtime_tuning.max_entities,
        );
        let tags_layers_manager = TagsLayersManager::new(&project_paths.settings)?;
        let grid = Grid::new(60, 40, 32, 8);
        let tilemap_layers = TilemapLayers::new(grid.width, grid.height);
        let mut camera = Camera::default();
        camera.set_bounds(
            0.0,
            0.0,
            (grid.width * grid.tile_size) as f64,
            (grid.height * grid.tile_size) as f64,
        );
        let mut units = vec![
            GameObject::new_unit(2.0, 2.0, None),
            GameObject::new_unit(4.0, 4.0, None),
            GameObject::new_unit(6.0, 6.0, None),
        ];
        for unit in &mut units {
            unit.sync_to_components();
        }
        let runtime_world = RuntimeWorld::new(units);

        let mut console = DeveloperConsole::with_log_file(project_paths.logs.join("miniforge.log"));
        console.log(
            format!("Project path: {}", project_path.display()),
            "ENGINE",
        );
        console.log(crate::engine::version::version_label(), "ENGINE");
        for warning in config_warnings {
            console.log(warning, "WARNING");
        }

        let mut history = EditorHistory::default();
        history.take_snapshot("Initial Scene", &runtime_world.units);
        let start_scene = engine_config
            .get("start_scene")
            .and_then(Value::as_str)
            .unwrap_or("main.scene")
            .to_string();

        let mut game = Self {
            runtime_mode,
            project_path: project_path.clone(),
            project_paths,
            engine_config,
            mode: if runtime_mode { "PLAY" } else { "EDITOR" }.to_string(),
            active_tool: "Select".to_string(),
            tile_brush: 0,
            brush_size: 1,
            scene_dirty: false,
            scene_dirty_reason: String::new(),
            console,
            runtime_world,
            selected_units: Vec::new(),
            grid,
            tilemap_layers,
            camera,
            clock: GameClock::from_tuning(&runtime_tuning),
            event_bus: EventBus::default(),
            resources,
            asset_database,
            input_map,
            build_settings,
            build_profiles,
            runtime_config,
            tags_layers_manager,
            component_registry: ComponentRegistry::new(),
            scene_manager: SceneManager::new_with_start_scene(&project_path, &start_scene),
            scene_validator: SceneValidator::default(),
            scene_save_manager: SceneSaveManager::new(),
            ui_canvases: json!([]),
            autosave_manager: AutosaveManager::new(&project_path, autosave_interval_seconds),
            history,
            profiler: Profiler::new(),
            diagnostics: Diagnostics::default(),
            stability_guard,
            docking_workspace: EditorDockingWorkspace::new(),
            audio_mixer: AudioMixer::new(),
            audio_system: AudioSystem::default(),
            animation_graphs: AnimationGraphLibrary::new(),
            material_library: MaterialLibrary::default(),
            native_libraries: NativeLibraryManager::new(&project_path),
            safe_mode,
            visual_script_runtime: VisualScriptRuntime::default(),
            luau_script_runtime: LuauScriptRuntime::new(&project_path),
            script_debugger: ScriptDebugger::default(),
            script_editor: ScriptEditor::default(),
            sprite_editor: SpriteEditorCanvas::default(),
            ui_runtime: UiRuntime::default(),
            play_mode_manager: PlayModeManager::default(),
            gameplay_system: GameplaySystem::default(),
            rts_system: RTSSystem::default(),
            runtime_2d_system: Runtime2DSystem::default(),
            physics_system: PhysicsSystem::new(),
            particle_system: ParticleSystem::default(),
            narrative_system: NarrativeSystem::default(),
            sprite_animation_system: SpriteAnimationSystem::new(&project_path),
            advanced_prefabs: AdvancedPrefabSystem::default(),
            archetypes: ArchetypeLibrary::with_defaults(),
            upgrade_manifest: EngineUpgradeManifest::new(),
            editor_workspace: EditorWorkspace::default(),
            programming: ProgrammingEnvironment::new(),
        };

        if game.safe_mode.allows_plugins() {
            match game.native_libraries.load_enabled() {
                Ok(count) if count > 0 => game.console.log(
                    format!("{count} biblioteca(s) nativa(s) cargadas"),
                    "NATIVE",
                ),
                Err(errors) => {
                    for error in errors.into_iter().take(8) {
                        game.console.log(error, "NATIVE");
                    }
                }
                _ => {}
            }
        } else {
            game.console.warning(
                format!(
                    "Plugins nativos desactivados por Safe Mode: {}",
                    game.safe_mode.reason
                ),
                "SAFE_MODE",
            );
        }

        match game.scene_manager.load_current_scene_data() {
            Ok(scene_data) => game.apply_scene_data(&scene_data),
            Err(error) => game.console.log(
                format!("No se pudo cargar la escena inicial; usando escena vacia: {error}"),
                "ERROR",
            ),
        }
        game.scene_save_manager
            .bootstrap_from_scene(&mut game.runtime_world.units, &game.tilemap_layers);
        game.sync_world();
        game.history
            .take_snapshot("Loaded Scene", &game.runtime_world.units);

        let mut open_validator = ProjectValidator::default();
        let _ = open_validator.validate_with_context(
            &game.project_path,
            &game.runtime_world.units,
            Some(&game.asset_database),
        );
        for err in open_validator.errors.iter().take(10) {
            game.console.log(err.clone(), "ERROR");
        }
        for warn in open_validator.warnings.iter().take(10) {
            game.console.log(warn.clone(), "WARNING");
        }
        if game.autosave_manager.autosave_exists() {
            game.console.log(
                "Existe autosave en saves/autosave/autosave.scene (recuperación disponible).",
                "PROJECT",
            );
        }

        Ok(game)
    }

    fn apply_scene_data(&mut self, data: &Value) {
        if let Some(entities) = data.get("entities").and_then(Value::as_array) {
            let units = entities
                .iter()
                .map(|entity| {
                    let mut entity = GameObject::from_data(entity, true);
                    entity.scene_name = Some(self.scene_manager.current_scene.clone());
                    entity
                })
                .collect();
            self.runtime_world.replace_entities(units);
            for entity in &mut self.runtime_world.units {
                entity.sync_from_components();
            }
            self.clear_selection();
            self.sync_world();
            self.console.log(
                format!(
                    "Escena cargada: {} entidades",
                    self.runtime_world.units.len()
                ),
                "SCENE",
            );
        }
        self.apply_scene_environment(data);
    }

    fn apply_scene_environment(&mut self, data: &Value) {
        let tile_data = data
            .get("tilemap_layers")
            .or_else(|| data.get("tiles"))
            .filter(|value| value.is_object());
        if let Some(tile_data) = tile_data {
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

        if !self.runtime_mode {
            self.mode = data
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("EDITOR")
                .to_string();
        }
        self.active_tool = data
            .get("active_tool")
            .and_then(Value::as_str)
            .unwrap_or("Select")
            .to_string();
        self.tile_brush = data
            .get("tile_brush")
            .and_then(Value::as_i64)
            .unwrap_or(self.tile_brush as i64) as i32;
        self.brush_size = data
            .get("brush_size")
            .and_then(Value::as_u64)
            .unwrap_or(self.brush_size as u64) as usize;

        self.ui_canvases = data
            .get("ui_canvases")
            .cloned()
            .unwrap_or_else(|| json!([]));
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
        self.profiler.record_system(
            "StabilityPreflight",
            marker.elapsed().as_secs_f64() * 1000.0,
        );
        marker = Instant::now();
        self.sprite_animation_system.update_entities(
            &mut self.runtime_world.units,
            simulation_dt,
            &self.mode,
        );
        self.profiler
            .record_system("SpriteFrames2D", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        AnimationSystem.update_entities(
            &mut self.runtime_world.units,
            &self.animation_graphs,
            simulation_dt,
            &self.mode,
        );
        self.profiler
            .record_system("Animation", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        if let Some(particle_dt) = self.stability_guard.optional_system_delta(simulation_dt) {
            self.particle_system.update_entities(
                &self.runtime_world.units,
                particle_dt,
                &self.mode,
            );
        }
        self.profiler
            .record_system("Particles", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.audio_system.update_entities(
            &mut self.runtime_world.units,
            &self.audio_mixer,
            &self.mode,
        );
        self.profiler
            .record_system("Audio", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        if self.safe_mode.allows_graphs() {
            self.visual_script_runtime.update_entities(
                &mut self.runtime_world.units,
                simulation_dt,
                &self.mode,
            );
            for error in self.visual_script_runtime.last_errors.iter().take(4) {
                self.console.log(error.clone(), "SCRIPT");
            }
        }
        self.profiler
            .record_system("VisualGraph", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        if self.safe_mode.allows_scripts() {
            self.luau_script_runtime.set_camera_state(&self.camera);
            let luau_report = self.luau_script_runtime.update_entities_with_fixed_steps(
                &mut self.runtime_world.units,
                simulation_dt,
                self.clock.fixed_delta,
                clock_advance.fixed_steps,
                &self.mode,
            );
            self.handle_luau_report(luau_report);
        }
        self.profiler
            .record_system("Luau", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.runtime_2d_system.update_entities(
            &mut self.runtime_world.units,
            simulation_dt,
            &self.mode,
        );
        self.profiler
            .record_system("Runtime2D", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.gameplay_system.update_entities_with_grid(
            &mut self.runtime_world.units,
            &self.grid,
            simulation_dt,
            &self.mode,
        );
        self.profiler
            .record_system("Gameplay", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.rts_system
            .update_entities(&mut self.runtime_world.units, simulation_dt, &self.mode);
        self.profiler
            .record_system("RTS", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        MovementSystem.update_entities(&mut self.runtime_world.units, simulation_dt);
        self.profiler
            .record_system("Movement", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.physics_system.update_entities_mut(
            &mut self.runtime_world.units,
            simulation_dt,
            &self.mode,
        );
        self.profiler
            .record_system("Physics", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
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
        self.profiler
            .record_system("Runtime2DLate", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        if self.safe_mode.allows_scripts() {
            let collision_events = self.physics_system.events.clone();
            let mut collision_report = LuauRunReport::default();
            for event in collision_events.iter().filter(|event| {
                matches!(
                    event.phase,
                    PhysicsEventPhase::Enter | PhysicsEventPhase::Exit
                )
            }) {
                match event.phase {
                    PhysicsEventPhase::Enter => {
                        collision_report.merge(self.luau_script_runtime.run_collision_enter(
                            &mut self.runtime_world.units,
                            event.first_id,
                            event.second_name.clone(),
                        ));
                        collision_report.merge(
                            self.luau_script_runtime.run_custom_event_for_entity(
                                &mut self.runtime_world.units,
                                event.first_id,
                                physics_event_name(event.pair_type, event.phase),
                                json!({
                                    "self_id": event.first_id,
                                    "other_id": event.second_id,
                                    "other_name": event.second_name.clone(),
                                    "pair_type": physics_pair_type_name(event.pair_type),
                                    "phase": "enter",
                                    "normal": {"x": event.normal.0, "y": event.normal.1},
                                    "depth": event.depth,
                                }),
                            ),
                        );
                        collision_report.merge(self.luau_script_runtime.run_collision_enter(
                            &mut self.runtime_world.units,
                            event.second_id,
                            event.first_name.clone(),
                        ));
                        collision_report.merge(
                            self.luau_script_runtime.run_custom_event_for_entity(
                                &mut self.runtime_world.units,
                                event.second_id,
                                physics_event_name(event.pair_type, event.phase),
                                json!({
                                    "self_id": event.second_id,
                                    "other_id": event.first_id,
                                    "other_name": event.first_name.clone(),
                                    "pair_type": physics_pair_type_name(event.pair_type),
                                    "phase": "enter",
                                    "normal": {"x": -event.normal.0, "y": -event.normal.1},
                                    "depth": event.depth,
                                }),
                            ),
                        );
                    }
                    PhysicsEventPhase::Exit => {
                        collision_report.merge(self.luau_script_runtime.run_collision_exit(
                            &mut self.runtime_world.units,
                            event.first_id,
                            event.second_name.clone(),
                        ));
                        collision_report.merge(
                            self.luau_script_runtime.run_custom_event_for_entity(
                                &mut self.runtime_world.units,
                                event.first_id,
                                physics_event_name(event.pair_type, event.phase),
                                json!({
                                    "self_id": event.first_id,
                                    "other_id": event.second_id,
                                    "other_name": event.second_name.clone(),
                                    "pair_type": physics_pair_type_name(event.pair_type),
                                    "phase": "exit",
                                    "normal": {"x": event.normal.0, "y": event.normal.1},
                                    "depth": event.depth,
                                }),
                            ),
                        );
                        collision_report.merge(self.luau_script_runtime.run_collision_exit(
                            &mut self.runtime_world.units,
                            event.second_id,
                            event.first_name.clone(),
                        ));
                        collision_report.merge(
                            self.luau_script_runtime.run_custom_event_for_entity(
                                &mut self.runtime_world.units,
                                event.second_id,
                                physics_event_name(event.pair_type, event.phase),
                                json!({
                                    "self_id": event.second_id,
                                    "other_id": event.first_id,
                                    "other_name": event.first_name.clone(),
                                    "pair_type": physics_pair_type_name(event.pair_type),
                                    "phase": "exit",
                                    "normal": {"x": -event.normal.0, "y": -event.normal.1},
                                    "depth": event.depth,
                                }),
                            ),
                        );
                    }
                    PhysicsEventPhase::Stay => {}
                }
            }
            self.handle_luau_report(collision_report);
        }
        self.profiler
            .record_system("LuauCollision", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.stability_guard
            .sanitize_entities(&mut self.runtime_world.units);
        self.profiler.record_system(
            "StabilityPostflight",
            marker.elapsed().as_secs_f64() * 1000.0,
        );
        marker = Instant::now();
        self.runtime_world.mark_changed();
        self.runtime_world.rebuild_index();
        self.profiler
            .record_system("WorldSync", marker.elapsed().as_secs_f64() * 1000.0);
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
        self.profiler.set_counter(
            "ActiveEntities",
            self.runtime_world
                .units
                .iter()
                .filter(|entity| entity.enabled && entity.visible)
                .count(),
        );
        self.profiler
            .set_counter("VisualGraphs", self.visual_script_runtime.last_frame_graphs);
        self.profiler
            .set_counter("VisualNodes", self.visual_script_runtime.executed_nodes);
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
            .set_counter("LuauReloads", self.luau_script_runtime.reload_count);
        self.profiler
            .set_counter("LuauErrors", self.luau_script_runtime.last_errors.len());
        self.profiler.set_counter(
            "Particles",
            self.particle_system
                .stats
                .get("particles")
                .copied()
                .unwrap_or(0),
        );
        self.profiler.set_counter(
            "Runtime2DControllers",
            self.runtime_2d_system
                .stats
                .get("character_controllers")
                .copied()
                .unwrap_or(0),
        );
        self.profiler.set_counter(
            "Runtime2DRespawns",
            self.gameplay_system
                .stats
                .get("respawned")
                .copied()
                .unwrap_or(0)
                + self
                    .runtime_2d_system
                    .stats
                    .get("fall_respawns")
                    .copied()
                    .unwrap_or(0),
        );
        self.script_debugger.refresh(
            &self.luau_script_runtime,
            &self.project_path,
            &self.runtime_world.units,
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
        self.play_mode_manager.tick_frame();
        self.profiler
            .set_counter("PlayFrames", self.play_mode_manager.frame_count);
        self.profiler.end_frame();
    }

    fn handle_luau_report(&mut self, report: LuauRunReport) {
        for error in report.errors.iter().take(6) {
            self.console.log(error.clone(), "SCRIPT");
        }
        for message in report.debug_messages.iter().take(8) {
            self.console.log(message.clone(), "SCRIPT");
        }
        for scene in report.scene_requests.iter().take(1) {
            if let Err(error) = self.load_scene(scene) {
                self.console
                    .log(format!("Luau load_scene({scene}) falló: {error}"), "SCRIPT");
            }
        }
        if !report.spawned.is_empty() || !report.destroyed.is_empty() || report.ui_updates > 0 {
            self.sync_world();
        }
    }

    pub fn dispatch_script_key_down(&mut self, key: &str) {
        if !self.safe_mode.allows_scripts() {
            return;
        }
        let report = self
            .luau_script_runtime
            .run_key_down(&mut self.runtime_world.units, key);
        self.handle_luau_report(report);
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
        if !self.safe_mode.allows_scripts() {
            return;
        }
        self.luau_script_runtime.set_input_pressed(key, pressed);
    }

    pub fn set_character_input(
        &mut self,
        entity_id: u64,
        movement: (f64, f64),
        jump_pressed: bool,
        jump_held: bool,
        run_pressed: bool,
        dash_pressed: bool,
    ) -> bool {
        let Some(entity) = self.get_entity_by_id_mut(entity_id) else {
            return false;
        };
        let Some(controller) = entity.get_component_mut("CharacterController2D") else {
            return false;
        };
        controller.set_f64("input_x", movement.0.clamp(-1.0, 1.0));
        controller.set_f64("input_y", movement.1.clamp(-1.0, 1.0));
        controller.set("jump_pressed", json!(jump_pressed));
        controller.set("jump_held", json!(jump_held));
        controller.set("run_pressed", json!(run_pressed));
        controller.set("dash_pressed", json!(dash_pressed));
        true
    }

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

    pub fn project_join(&self, parts: &[&str]) -> PathBuf {
        let mut path = self.project_path.clone();
        for part in parts {
            path.push(part);
        }
        path
    }

    pub fn mark_scene_dirty(&mut self, reason: &str) {
        self.scene_dirty = true;
        self.scene_dirty_reason = reason.to_string();
    }

    pub fn mark_scene_clean(&mut self) {
        self.scene_dirty = false;
        self.scene_dirty_reason.clear();
    }

    pub fn capture_editor_snapshot(&self) -> EditorSnapshot {
        EditorSnapshot::capture(
            &self.runtime_world.units,
            &self.tilemap_layers,
            &self.grid,
            &self.camera,
            &self.ui_canvases,
        )
    }

    pub fn push_editor_command(
        &mut self,
        label: impl Into<String>,
        kind: EditorCommandKind,
        before: EditorSnapshot,
    ) {
        if self.mode == "PLAY" {
            return;
        }
        let after = self.capture_editor_snapshot();
        self.history
            .push_command(EditorCommand::new(label, kind, before, after));
    }

    pub fn undo_editor_command(&mut self) -> Option<String> {
        let label = self.history.undo_command(
            &mut self.runtime_world.units,
            &mut self.tilemap_layers,
            &mut self.grid,
            &mut self.camera,
            &mut self.ui_canvases,
        )?;
        self.clear_selection();
        self.sync_world();
        self.mark_scene_dirty("Undo");
        Some(label)
    }

    pub fn redo_editor_command(&mut self) -> Option<String> {
        let label = self.history.redo_command(
            &mut self.runtime_world.units,
            &mut self.tilemap_layers,
            &mut self.grid,
            &mut self.camera,
            &mut self.ui_canvases,
        )?;
        self.clear_selection();
        self.sync_world();
        self.mark_scene_dirty("Redo");
        Some(label)
    }

    pub fn enter_play_mode(&mut self) {
        if self.mode == "PLAY" {
            return;
        }
        for entity in &mut self.runtime_world.units {
            entity.sync_from_components();
            if let Some(ai) = entity.get_component_mut("AIController") {
                ai.set_f64("think_timer", 0.0);
            }
            if let Some(nav) = entity.get_component_mut("NavAgent") {
                nav.set_f64("repath_timer", 9999.0);
            }
        }
        self.play_mode_manager
            .enter_play_mode(&self.runtime_world.units, &mut self.mode);
        self.sync_world();
        let dirty = if self.scene_dirty {
            format!("dirty ({})", self.scene_dirty_reason)
        } else {
            "clean".to_string()
        };
        self.console.log(
            format!(
                "Play Mode ON: snapshot de {} entidades (live). F11 pausa simulación; F5 vuelve al editor y restaura escena. Estado: {dirty}.",
                self.runtime_world.units.len()
            ),
            "ENGINE",
        );
    }

    pub fn exit_play_mode(&mut self, reason: &str) {
        if self.mode != "PLAY" {
            return;
        }
        self.play_mode_manager.exit_play_mode(
            &mut self.runtime_world.units,
            &mut self.mode,
            reason,
        );
        let frames = self.play_mode_manager.last_session_frames;
        self.clear_selection();
        self.sync_world();
        self.console.log(
            format!(
                "Play Mode OFF ({reason}): {frames} frames simulados; escena restaurada al snapshot de {} entidades.",
                self.play_mode_manager.last_session_entity_count
            ),
            "ENGINE",
        );
    }

    pub fn toggle_play_mode(&mut self) {
        if self.mode == "PLAY" {
            self.exit_play_mode("toggle");
        } else {
            self.enter_play_mode();
        }
    }

    pub fn get_entity_by_id(&self, entity_id: u64) -> Option<&GameObject> {
        self.runtime_world.entity(entity_id)
    }

    pub fn get_entity_by_id_mut(&mut self, entity_id: u64) -> Option<&mut GameObject> {
        self.runtime_world.entity_mut(entity_id)
    }

    pub fn clear_selection(&mut self) {
        for entity in &mut self.runtime_world.units {
            entity.selected = false;
        }
        self.selected_units.clear();
    }

    pub fn select_entity(&mut self, entity_id: u64) -> bool {
        self.clear_selection();
        self.select_entity_additive(entity_id)
    }

    pub fn select_entity_additive(&mut self, entity_id: u64) -> bool {
        if self.selected_units.contains(&entity_id) {
            return true;
        }
        let Some(entity) = self.get_entity_by_id_mut(entity_id) else {
            return false;
        };
        if entity.locked || !entity.visible || !entity.enabled {
            return false;
        }
        entity.set_selected(true);
        self.selected_units.push(entity_id);
        true
    }

    pub fn toggle_entity_selection(&mut self, entity_id: u64) -> bool {
        if self.selected_units.contains(&entity_id) {
            self.selected_units.retain(|id| *id != entity_id);
            if let Some(entity) = self.get_entity_by_id_mut(entity_id) {
                entity.set_selected(false);
            }
            return false;
        }
        self.select_entity_additive(entity_id)
    }

    pub fn select_editor_group(&mut self, group_id: &str, additive: bool) -> usize {
        if !additive {
            self.clear_selection();
        }
        let ids = self
            .runtime_world
            .units
            .iter()
            .filter(|entity| {
                entity.editor_group.as_deref() == Some(group_id)
                    && !entity.locked
                    && entity.visible
                    && entity.enabled
            })
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        for id in ids {
            self.select_entity_additive(id);
        }
        self.selected_units.len()
    }

    pub fn set_entity_parent(&mut self, child_id: u64, parent_id: u64) -> bool {
        if child_id == parent_id {
            return false;
        }
        // Reject cycles before mutating the hierarchy. The editor exposes
        // reparenting through drag-and-drop, so invalid drops must be harmless
        // instead of producing a scene that can no longer be saved.
        let mut ancestor = Some(parent_id);
        let mut visited = std::collections::BTreeSet::new();
        while let Some(ancestor_id) = ancestor {
            if ancestor_id == child_id || !visited.insert(ancestor_id) {
                return false;
            }
            ancestor = self
                .get_entity_by_id(ancestor_id)
                .and_then(|entity| entity.parent_id);
        }
        let Some(parent) = self.get_entity_by_id(parent_id).cloned() else {
            return false;
        };
        let Some(child) = self.get_entity_by_id_mut(child_id) else {
            return false;
        };
        HierarchyManager::set_parent(child, &parent);
        HierarchyManager::sync_child_world_transforms(&mut self.runtime_world.units);
        self.sync_world();
        self.mark_scene_dirty("Set Parent");
        self.console.log(
            format!("Hierarchy: #{child_id} ahora cuelga de #{parent_id}"),
            "EDITOR",
        );
        true
    }

    pub fn clear_entity_parent(&mut self, child_id: u64) -> bool {
        let Some(child) = self.get_entity_by_id_mut(child_id) else {
            return false;
        };
        HierarchyManager::clear_parent(child);
        self.sync_world();
        self.mark_scene_dirty("Clear Parent");
        self.console.log(
            format!("Hierarchy: #{child_id} ya no tiene parent"),
            "EDITOR",
        );
        true
    }

    pub fn move_entity_in_hierarchy(&mut self, entity_id: u64, delta: isize) -> bool {
        let Some(index) = self
            .runtime_world
            .units
            .iter()
            .position(|entity| entity.id == entity_id)
        else {
            return false;
        };
        let next = (index as isize + delta)
            .clamp(0, self.runtime_world.units.len().saturating_sub(1) as isize)
            as usize;
        if index == next {
            return false;
        }
        let entity = self.runtime_world.units.remove(index);
        self.runtime_world.units.insert(next, entity);
        self.mark_scene_dirty("Move Hierarchy Row");
        self.console
            .log(format!("Hierarchy: entity #{entity_id} movida"), "EDITOR");
        true
    }

    pub fn spawn_game_object(&mut self, name: &str, x: f64, y: f64) -> u64 {
        let before = self.capture_editor_snapshot();
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        ensure_scene_node_components(&mut entity);
        entity.width = 1.0;
        entity.height = 1.0;
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn GameObject");
        self.push_editor_command(
            "Create GameObject",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn spawn_scene_node(
        &mut self,
        name: &str,
        component_types: &[&str],
        x: f64,
        y: f64,
    ) -> u64 {
        let before = self.capture_editor_snapshot();
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        ensure_scene_node_components(&mut entity);
        for component_type in component_types {
            if let Some(component) = default_component(component_type) {
                entity.add_component(component);
            }
        }
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Scene Node");
        self.push_editor_command(
            "Create Scene Node",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn spawn_archetype(
        &mut self,
        archetype_key: &str,
        x: f64,
        y: f64,
        team_id: Option<i64>,
    ) -> Option<u64> {
        let before = self.capture_editor_snapshot();
        let entity = self.archetypes.instantiate(archetype_key, x, y, team_id)?;
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Archetype");
        self.console.log(
            format!("Arquetipo {archetype_key} creado como entity #{id}"),
            "GAMEPLAY",
        );
        self.push_editor_command(
            "Create Archetype",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        Some(id)
    }

    pub fn spawn_sprite_entity(&mut self, name: &str, sprite_name: &str, x: f64, y: f64) -> u64 {
        let before = self.capture_editor_snapshot();
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.sprite_name = Some(sprite_name.to_string());
        if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
            sprite.set("sprite_name", json!(sprite_name));
            sprite.set("visible", json!(true));
        }
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Sprite Entity");
        self.push_editor_command(
            "Create Sprite Entity",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn spawn_unit(&mut self, name: &str, x: f64, y: f64) -> u64 {
        let before = self.capture_editor_snapshot();
        let mut entity = GameObject::new_unit(x, y, Some(name.to_string()));
        entity.tag = "Player".to_string();
        entity.add_component(default_component("Health").expect("Health"));
        entity.add_component(default_component("Stats").expect("Stats"));
        entity.add_component(default_component("Inventory").expect("Inventory"));
        entity.add_component(default_component("Cooldown").expect("Cooldown"));
        entity.add_component(default_component("NavAgent").expect("NavAgent"));
        entity.add_component(default_component("Worker").expect("Worker"));
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Unit");
        self.push_editor_command(
            "Create Unit",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn spawn_enemy(&mut self, x: f64, y: f64) -> u64 {
        let before = self.capture_editor_snapshot();
        let mut entity = GameObject::new_unit(x, y, Some("Enemy".to_string()));
        entity.tag = "Enemy".to_string();
        entity.add_component(default_component("Health").expect("Health"));
        entity.add_component(default_component("Stats").expect("Stats"));
        let mut ai = default_component("AIController").expect("AIController");
        ai.set("behavior", json!("attack"));
        ai.set("target_tags", json!(["Player"]));
        entity.add_component(ai);
        let mut damage = default_component("DamageDealer").expect("DamageDealer");
        damage.set("target_tags", json!(["Player"]));
        entity.add_component(damage);
        let mut threat = default_component("ThreatSource").expect("ThreatSource");
        threat.set_f64("strength", 10.0);
        threat.set_f64("radius", 6.0);
        threat.set_f64("avoidance_weight", 28.0);
        entity.add_component(threat);
        let mut influence = default_component("InfluenceSource").expect("InfluenceSource");
        influence.set("team_id", json!(2));
        influence.set_f64("strength", -12.0);
        influence.set_f64("falloff", 2.0);
        influence.set("label", json!("EnemyControl"));
        entity.add_component(influence);
        entity.add_component(default_component("CombatTarget").expect("CombatTarget"));
        entity.add_component(default_component("LootTable").expect("LootTable"));
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Enemy");
        self.push_editor_command(
            "Create Enemy",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn spawn_resource(&mut self, x: f64, y: f64) -> u64 {
        let before = self.capture_editor_snapshot();
        let mut entity = GameObject::new(x, y, Some("GoldNode".to_string()));
        entity.tag = "Resource".to_string();
        entity.width = 1.3;
        entity.height = 1.3;
        entity.add_component(default_component("ResourceNode").expect("ResourceNode"));
        entity.add_component(default_component("ObjectiveMarker").expect("ObjectiveMarker"));
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Resource");
        self.push_editor_command(
            "Create Resource",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn spawn_rts_controller(&mut self, team_id: i64) -> u64 {
        let mut entity = GameObject::new(0.0, 0.0, Some(format!("RTSController_Team{team_id}")));
        entity.visible = false;
        entity.locked = true;
        entity.layer = "EditorOnly".to_string();
        entity.add_component(default_component("RTSController").expect("RTSController"));
        entity.add_component(default_component("FogOfWar").expect("FogOfWar"));
        entity.add_component(default_component("EconomyWallet").expect("EconomyWallet"));
        Self::apply_team(&mut entity, team_id);
        if let Some(controller) = entity.get_component_mut("RTSController") {
            controller.set("team_id", json!(team_id));
        }
        if let Some(fog) = entity.get_component_mut("FogOfWar") {
            fog.set("team_id", json!(team_id));
            fog.set("map_width", json!(self.grid.width));
            fog.set("map_height", json!(self.grid.height));
            fog.set_f64("tile_size", 1.0);
        }
        if let Some(wallet) = entity.get_component_mut("EconomyWallet") {
            wallet.set(
                "resources",
                json!({"Gold": 500.0, "Wood": 250.0, "Supply": 0.0}),
            );
        }
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.sync_world();
        self.mark_scene_dirty("Spawn RTS Controller");
        id
    }

    pub fn spawn_rts_worker(&mut self, name: &str, x: f64, y: f64, team_id: i64) -> u64 {
        let id = self
            .spawn_archetype("rts_worker", x, y, Some(team_id))
            .unwrap_or_else(|| self.spawn_unit(name, x, y));
        if let Some(entity) = self.get_entity_by_id_mut(id) {
            entity.name = name.to_string();
            entity.layer = "Units".to_string();
            entity.add_component(default_component("Commandable").expect("Commandable"));
            entity.add_component(default_component("Vision").expect("Vision"));
            Self::apply_team(entity, team_id);
            if let Some(commandable) = entity.get_component_mut("Commandable") {
                commandable.set("can_gather", json!(true));
                commandable.set("can_build", json!(true));
            }
        }
        self.sync_world();
        self.mark_scene_dirty("Spawn RTS Worker");
        id
    }

    pub fn spawn_rts_building(&mut self, name: &str, x: f64, y: f64, team_id: i64) -> u64 {
        let before = self.capture_editor_snapshot();
        let archetype_key = match name {
            "CommandCenter" | "EnemyBase" => "rts_command_center",
            "Barracks" => "rts_barracks",
            _ => "",
        };
        if !archetype_key.is_empty()
            && let Some(id) = self.spawn_archetype(archetype_key, x, y, Some(team_id))
        {
            if let Some(entity) = self.get_entity_by_id_mut(id) {
                entity.name = name.to_string();
                if let Some(commandable) = entity.get_component_mut("Commandable") {
                    commandable.set("can_move", json!(false));
                    commandable.set("can_produce", json!(true));
                }
                if let Some(queue) = entity.get_component_mut("ProductionQueue") {
                    queue.set_f64("rally_x", x + 3.0);
                    queue.set_f64("rally_y", y);
                }
                if team_id != 1 && entity.get_component("ThreatSource").is_none() {
                    let mut threat = default_component("ThreatSource").expect("ThreatSource");
                    threat.set_f64("strength", 6.0);
                    threat.set_f64("radius", 5.0);
                    entity.add_component(threat);
                }
                entity.sync_to_components();
            }
            self.sync_world();
            self.mark_scene_dirty("Spawn RTS Building");
            return id;
        }

        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.tag = if team_id == 1 { "Player" } else { "Enemy" }.to_string();
        entity.layer = "Buildings".to_string();
        entity.width = 2.4;
        entity.height = 2.0;
        entity.radius = 1.2;
        entity.add_component(default_component("Health").expect("Health"));
        entity.add_component(default_component("EconomyWallet").expect("EconomyWallet"));
        entity.add_component(default_component("ProductionQueue").expect("ProductionQueue"));
        entity.add_component(default_component("Buildable").expect("Buildable"));
        entity.add_component(default_component("Commandable").expect("Commandable"));
        entity.add_component(default_component("Vision").expect("Vision"));
        Self::apply_team(&mut entity, team_id);
        if let Some(wallet) = entity.get_component_mut("EconomyWallet") {
            wallet.set(
                "resources",
                json!({"Gold": 500.0, "Wood": 250.0, "Supply": 0.0}),
            );
        }
        if let Some(queue) = entity.get_component_mut("ProductionQueue") {
            queue.set_f64("rally_x", x + 3.0);
            queue.set_f64("rally_y", y);
        }
        if let Some(commandable) = entity.get_component_mut("Commandable") {
            commandable.set("can_move", json!(false));
            commandable.set("can_produce", json!(true));
        }
        if team_id != 1 {
            let mut threat = default_component("ThreatSource").expect("ThreatSource");
            threat.set_f64("strength", 6.0);
            threat.set_f64("radius", 5.0);
            entity.add_component(threat);
        }
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn RTS Building");
        self.push_editor_command(
            "Create RTS Building",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn spawn_construction_site(
        &mut self,
        name: &str,
        x: f64,
        y: f64,
        team_id: i64,
        build_time: f64,
        builder_ids: Vec<u64>,
    ) -> u64 {
        let before = self.capture_editor_snapshot();
        let mut entity = GameObject::new(x, y, Some(format!("{name}_Site")));
        entity.tag = "Building".to_string();
        entity.layer = "Buildings".to_string();
        entity.width = 2.4;
        entity.height = 2.0;
        entity.add_component(default_component("ConstructionSite").expect("ConstructionSite"));
        Self::apply_team(&mut entity, team_id);
        if let Some(site) = entity.get_component_mut("ConstructionSite") {
            site.set("target_name", json!(name));
            site.set_f64("build_time", build_time.max(0.01));
            site.set("builder_ids", json!(builder_ids));
        }
        entity.sync_to_components();
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Construction Site");
        self.push_editor_command(
            "Create Construction Site",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        id
    }

    pub fn try_place_rts_building(
        &mut self,
        name: &str,
        desired_cell: (i32, i32),
        team_id: i64,
        builder_ids: Vec<u64>,
    ) -> Option<u64> {
        let footprint = BuildFootprint {
            width: 2,
            height: 2,
            clearance: 1,
        };
        let placement = BuildPlacement::find_nearest_valid(
            &self.grid,
            &self.runtime_world.units,
            desired_cell,
            &footprint,
            8,
            Some(team_id),
        )?;
        if !placement.valid {
            return None;
        }
        BuildPlacement::reserve_on_grid(&mut self.grid, placement.cell, &footprint, 1);
        let id = self.spawn_construction_site(
            name,
            placement.cell.0 as f64,
            placement.cell.1 as f64,
            team_id,
            8.0,
            builder_ids,
        );
        self.console.log(
            format!(
                "Construccion {name} colocada en {},{}",
                placement.cell.0, placement.cell.1
            ),
            "RTS",
        );
        Some(id)
    }

    pub fn create_rts_skirmish(&mut self) {
        self.runtime_world.units.clear();
        self.clear_selection();
        self.grid = Grid::new(48, 32, 32, 8);
        self.tilemap_layers = TilemapLayers::new(self.grid.width, self.grid.height);
        self.tilemap_layers.active_layer = 0;
        self.tilemap_layers
            .fill_active(0, 0, self.grid.width, self.grid.height, 1);
        self.tilemap_layers.active_layer = 1;
        for x in 0..self.grid.width {
            self.tilemap_layers.set_tile(x, 0, 4);
            self.tilemap_layers.set_tile(x, self.grid.height - 1, 4);
            self.grid.set_tile(x, 0, 1);
            self.grid.set_tile(x, self.grid.height - 1, 1);
        }
        for y in 0..self.grid.height {
            self.tilemap_layers.set_tile(0, y, 4);
            self.tilemap_layers.set_tile(self.grid.width - 1, y, 4);
            self.grid.set_tile(0, y, 1);
            self.grid.set_tile(self.grid.width - 1, y, 1);
        }
        for (x, y) in (14..34).zip((10..30).cycle()).take(20) {
            if x % 3 == 0 && y < self.grid.height - 2 {
                self.tilemap_layers.set_tile(x, y, 3);
                self.grid.set_tile(x, y, 1);
            }
        }
        self.tilemap_layers.active_layer = 0;
        self.spawn_rts_controller(1);
        self.spawn_rts_controller(2);
        let base_id = self.spawn_rts_building("CommandCenter", 8.0, 8.0, 1);
        self.spawn_rts_worker("Worker_A", 10.0, 8.0, 1);
        self.spawn_rts_worker("Worker_B", 10.0, 9.0, 1);
        self.spawn_rts_worker("Scout_A", 9.0, 10.0, 1);
        self.spawn_archetype("rts_soldier", 11.0, 11.0, Some(1));
        self.spawn_archetype("rts_soldier", 12.0, 11.0, Some(1));
        self.spawn_resource(13.0, 8.0);
        self.spawn_resource(15.0, 10.0);
        self.spawn_resource(20.0, 20.0);
        self.spawn_rts_building("EnemyBase", 30.0, 22.0, 2);
        self.spawn_enemy(27.0, 21.0);
        self.spawn_enemy(25.0, 23.0);
        self.spawn_enemy(32.0, 19.0);
        if let Some(base) = self.get_entity_by_id_mut(base_id) {
            RTSSystem::enqueue_production(
                base,
                "Worker",
                "QueuedWorker",
                3.0,
                json!({"Gold": 50.0}),
            );
        }
        self.camera.x = 120.0;
        self.camera.y = 100.0;
        self.camera.set_zoom(1.1);
        self.sync_world();
        self.mark_scene_dirty("Create RTS Skirmish");
        self.console.log(
            "Skirmish RTS creado: base, workers, recursos, amenazas, obstaculos y fog.",
            "RTS",
        );
    }

    pub fn team_id_of(&self, entity_id: u64) -> i64 {
        self.get_entity_by_id(entity_id)
            .map(Self::entity_team_id)
            .unwrap_or(1)
    }

    pub fn rts_threat_sources_for_team(&self, team_id: i64) -> Vec<(Point, u32)> {
        let mut threats = Vec::new();
        for entity in &self.runtime_world.units {
            if !entity.visible || Self::entity_team_id(entity) == team_id {
                continue;
            }
            let Some(threat) = entity.get_component("ThreatSource") else {
                if entity.get_component("DamageDealer").is_some() {
                    threats.push(((entity.x.round() as i32, entity.y.round() as i32), 6));
                }
                continue;
            };
            if !threat.get_bool("enabled", true) {
                continue;
            }
            let affects = threat
                .get("affects_teams")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if !affects.is_empty()
                && !affects
                    .iter()
                    .any(|value| value.as_i64().is_some_and(|id| id == team_id))
            {
                continue;
            }
            let center = (entity.x.round() as i32, entity.y.round() as i32);
            let radius = threat.get_f64("radius", 4.0).round().max(0.0) as i32;
            let strength = threat.get_f64("strength", 8.0).max(1.0) as u32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let point = (center.0 + dx, center.1 + dy);
                    if !self.grid.is_walkable(point.0, point.1) {
                        continue;
                    }
                    let distance = dx.abs() + dy.abs();
                    if distance > radius {
                        continue;
                    }
                    let value = strength.saturating_sub(distance as u32).max(1);
                    threats.push((point, value));
                }
            }
        }
        threats
    }

    pub fn duplicate_entity(&mut self, entity_id: u64) -> Option<u64> {
        let before = self.capture_editor_snapshot();
        let mut source = self.get_entity_by_id(entity_id)?.clone();
        source.sync_to_components();
        let data = source.serialize();
        let mut clone = GameObject::from_data(&data, false);
        clone.name = format!("{}_Copy", source.name);
        clone.x += 1.0;
        clone.y += 1.0;
        clone.path.clear();
        clone.selected = false;
        clone.sync_to_components();
        let id = clone.id;
        self.runtime_world.units.push(clone);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Duplicate Entity");
        self.push_editor_command(
            "Duplicate Entity",
            EditorCommandKind::DuplicateEntity {
                source_id: entity_id,
                clone_id: id,
            },
            before,
        );
        Some(id)
    }

    pub fn delete_entity(&mut self, entity_id: u64) -> bool {
        let snapshot_before = self.capture_editor_snapshot();
        let parent_id = self
            .runtime_world
            .units
            .iter()
            .find(|entity| entity.id == entity_id)
            .and_then(|entity| entity.parent_id);
        let len_before = self.runtime_world.units.len();
        self.runtime_world
            .units
            .retain(|entity| entity.id != entity_id);
        let deleted = self.runtime_world.units.len() != len_before;
        if deleted {
            // Preserve the deleted node's branch by promoting direct children
            // to its parent (or to scene roots). This keeps parent references
            // valid and makes deletion safe for both the Qt hierarchy and API
            // callers.
            for child in self
                .runtime_world
                .units
                .iter_mut()
                .filter(|entity| entity.parent_id == Some(entity_id))
            {
                child.parent_id = parent_id;
            }
            self.selected_units
                .retain(|selected| *selected != entity_id);
            self.sync_world();
            self.mark_scene_dirty("Delete Entity");
            self.push_editor_command(
                "Delete Entity",
                EditorCommandKind::DeleteEntity { entity_id },
                snapshot_before,
            );
        }
        deleted
    }

    pub fn add_component_to_entity(&mut self, entity_id: u64, component_type: &str) -> bool {
        let before = self.capture_editor_snapshot();
        let Some(component) = default_component(component_type) else {
            return false;
        };
        let Some(entity) = self.get_entity_by_id_mut(entity_id) else {
            return false;
        };
        let already_had = entity.get_component(component_type).is_some();
        entity.add_component(component);
        entity.sync_from_components();
        if !already_had {
            self.sync_world();
            self.mark_scene_dirty("Add Component");
            self.push_editor_command(
                "Add Component",
                EditorCommandKind::AddComponent {
                    entity_id,
                    component_type: component_type.to_string(),
                },
                before,
            );
        }
        true
    }

    pub fn prepare_complex_game_foundations(&mut self) -> ComplexGameFoundationReport {
        let before = self.capture_editor_snapshot();
        let mut report = ComplexGameFoundationReport::default();

        let target_id = self
            .selected_units
            .iter()
            .copied()
            .find(|id| self.get_entity_by_id(*id).is_some())
            .or_else(|| {
                self.runtime_world
                    .units
                    .iter()
                    .find(|entity| entity.tag == "Player")
                    .map(|entity| entity.id)
            })
            .unwrap_or_else(|| {
                let mut player = GameObject::new(4.0, 4.0, Some("Player".to_string()));
                player.entity_type = "Player".to_string();
                player.tag = "Player".to_string();
                player.layer = "Units".to_string();
                player.width = 0.85;
                player.height = 1.2;
                player.sync_to_components();
                let id = player.id;
                self.runtime_world.units.push(player);
                report.created_player = true;
                id
            });
        report.target_entity_id = target_id;

        if let Some(player) = self.get_entity_by_id_mut(target_id) {
            if player.tag != "Player" {
                player.tag = "Player".to_string();
                report.identity_changed = true;
            }
            if player.layer != "Units" {
                player.layer = "Units".to_string();
                report.identity_changed = true;
            }
            if player.width < 0.25 {
                player.width = 0.85;
                report.identity_changed = true;
            }
            if player.height < 0.25 {
                player.height = 1.2;
                report.identity_changed = true;
            }

            let bundle = player.ensure_components(COMPLEX_FOUNDATION_PLAYER_COMPONENTS);
            report.added_player_components = bundle.added;
            report.missing_components.extend(bundle.missing);
            report.configured_components.extend(
                configure_complex_foundation_player(player, target_id)
                    .into_iter()
                    .map(|component| format!("Player.{component}")),
            );
            player.sync_to_components();
        }

        let systems_id = self
            .runtime_world
            .units
            .iter()
            .find(|entity| entity.name == "GameSystems" || entity.tag == "System")
            .map(|entity| entity.id)
            .unwrap_or_else(|| {
                let mut systems = GameObject::new(0.0, 0.0, Some("GameSystems".to_string()));
                systems.entity_type = "System".to_string();
                systems.tag = "System".to_string();
                systems.layer = "EditorOnly".to_string();
                systems.visible = false;
                systems.locked = true;
                systems.sync_to_components();
                let id = systems.id;
                self.runtime_world.units.push(systems);
                report.created_systems_entity = true;
                id
            });
        report.systems_entity_id = Some(systems_id);

        if let Some(systems) = self.get_entity_by_id_mut(systems_id) {
            if systems.name != "GameSystems" {
                systems.name = "GameSystems".to_string();
                report.identity_changed = true;
            }
            if systems.tag != "System" {
                systems.tag = "System".to_string();
                report.identity_changed = true;
            }
            if systems.layer != "EditorOnly" {
                systems.layer = "EditorOnly".to_string();
                report.identity_changed = true;
            }
            if systems.visible {
                systems.visible = false;
                report.identity_changed = true;
            }
            if !systems.locked {
                systems.locked = true;
                report.identity_changed = true;
            }

            let bundle = systems.ensure_components(COMPLEX_FOUNDATION_SYSTEM_COMPONENTS);
            report.added_system_components = bundle.added;
            report.missing_components.extend(bundle.missing);
            report.configured_components.extend(
                configure_complex_foundation_systems(systems)
                    .into_iter()
                    .map(|component| format!("GameSystems.{component}")),
            );
            systems.sync_to_components();
        }

        let had_canvas = !ui_canvases_from_value(&self.ui_canvases).is_empty();
        self.ensure_default_ui_canvas_scene_data();
        report.ui_canvas_ready = !ui_canvases_from_value(&self.ui_canvases).is_empty();
        report.created_ui_canvas = !had_canvas && report.ui_canvas_ready;

        self.select_entity(target_id);
        self.sync_world();

        report.changed = report.created_player
            || report.created_systems_entity
            || report.created_ui_canvas
            || report.identity_changed
            || !report.added_player_components.is_empty()
            || !report.added_system_components.is_empty()
            || !report.configured_components.is_empty();

        if report.changed {
            self.mark_scene_dirty("Prepare Complex Foundations");
            self.console.log(
                format!(
                    "Complex foundations listas: player #{target_id}, systems #{systems_id}, +{} player comps, +{} system comps",
                    report.added_player_components.len(),
                    report.added_system_components.len()
                ),
                "EDITOR",
            );
            self.push_editor_command(
                "Prepare Complex Foundations",
                EditorCommandKind::SceneOperation {
                    name: "Prepare Complex Foundations".to_string(),
                },
                before,
            );
        } else {
            self.console
                .log("Complex foundations ya estaban listas.", "EDITOR");
        }

        report
    }

    pub fn remove_component_from_entity(
        &mut self,
        entity_id: u64,
        component_type: &str,
    ) -> Result<(), String> {
        let before = self.capture_editor_snapshot();
        let Some(entity) = self.get_entity_by_id_mut(entity_id) else {
            return Err(format!("Entity no existe: {entity_id}"));
        };
        InspectorEditor::remove_component(entity, component_type)?;
        self.sync_world();
        self.mark_scene_dirty("Remove Component");
        self.push_editor_command(
            "Remove Component",
            EditorCommandKind::RemoveComponent {
                entity_id,
                component_type: component_type.to_string(),
            },
            before,
        );
        Ok(())
    }

    pub fn edit_inspector_value(
        &mut self,
        entity_id: u64,
        target: &str,
        key: &str,
        value: Value,
    ) -> Result<Value, String> {
        let before = self.capture_editor_snapshot();
        let requested = value.clone();
        let Some(entity) = self.get_entity_by_id_mut(entity_id) else {
            return Err(format!("Entity no existe: {entity_id}"));
        };
        let previous = InspectorEditor::edit_value(entity, target, key, value)?;
        self.sync_world();
        self.mark_scene_dirty("Edit Inspector");
        self.scene_save_manager.note_entity_dirty(entity_id);
        self.push_editor_command(
            "Edit Inspector",
            EditorCommandKind::EditInspector {
                entity_id,
                target: target.to_string(),
                field: key.to_string(),
                before: previous.clone(),
                after: requested,
            },
            before,
        );
        Ok(previous)
    }

    pub fn paint_tile(&mut self, x: usize, y: usize, value: i32) -> bool {
        self.paint_tile_brush(TileBrushMode::Pencil, (x, y), (x, y), value)
    }

    pub fn paint_tile_brush(
        &mut self,
        mode: TileBrushMode,
        start: (usize, usize),
        end: (usize, usize),
        value: i32,
    ) -> bool {
        let before = self.capture_editor_snapshot();
        let stroke = TileBrush::apply(&mut self.tilemap_layers, mode, start, end, value);
        if stroke.changes.is_empty() {
            return false;
        }
        self.mark_scene_dirty("Paint Tilemap");
        self.scene_save_manager.note_tilemap_dirty();
        self.push_editor_command(
            "Paint Tilemap",
            EditorCommandKind::PaintTilemap {
                layer: stroke.layer,
                cells: stroke
                    .changes
                    .iter()
                    .map(|change| (change.x, change.y, change.before, change.after))
                    .collect(),
            },
            before,
        );
        true
    }

    pub fn drop_asset_to_scene(
        &mut self,
        payload: &DragPayload,
        x: f64,
        y: f64,
    ) -> io::Result<DropOutcome> {
        let target = self.selected_units.first().copied();
        self.drop_asset_to_target(payload, x, y, target)
    }

    /// Applies an asset only when the pointer is over `target_entity_id`;
    /// dropping on empty viewport space always creates a new scene entity.
    pub fn drop_asset_to_target(
        &mut self,
        payload: &DragPayload,
        x: f64,
        y: f64,
        target_entity_id: Option<u64>,
    ) -> io::Result<DropOutcome> {
        if payload.kind == DragAssetKind::Scene {
            let scene_name = payload
                .relative_path
                .rsplit('/')
                .next()
                .unwrap_or(payload.relative_path.as_str())
                .to_string();
            self.load_scene(&scene_name)?;
            return Ok(DropOutcome::OpenScene(scene_name));
        }

        let before = self.capture_editor_snapshot();
        if payload.kind == DragAssetKind::Prefab {
            let manager = PrefabManager::new(&self.project_path);
            let path = self.project_path.join(&payload.relative_path);
            let Some(id) = manager.instantiate_prefab(&mut self.runtime_world.units, path, x, y)?
            else {
                return Ok(DropOutcome::Unsupported(format!(
                    "No se pudo instanciar prefab {}",
                    payload.name
                )));
            };
            self.select_entity(id);
            self.sync_world();
            self.mark_scene_dirty("Drop Prefab");
            self.push_editor_command(
                "Drop Prefab",
                EditorCommandKind::CreateEntity { entity_id: id },
                before,
            );
            return Ok(DropOutcome::SpawnedEntity(id));
        }

        if payload.can_apply_to_entity()
            && let Some(id) = target_entity_id
            && let Some(entity) = self.get_entity_by_id_mut(id)
        {
            let outcome = ContentDropper::apply_to_entity(entity, payload);
            self.sync_world();
            self.mark_scene_dirty("Drop Asset On Entity");
            self.push_editor_command(
                "Drop Asset On Entity",
                EditorCommandKind::EditInspector {
                    entity_id: id,
                    target: payload.asset_type.clone(),
                    field: "asset".to_string(),
                    before: Value::Null,
                    after: json!(payload.relative_path),
                },
                before,
            );
            return Ok(outcome);
        }

        let Some(entity) = ContentDropper::spawn_from_payload(payload, x, y) else {
            return Ok(DropOutcome::Unsupported(format!(
                "{} no tiene drop directo",
                payload.asset_type
            )));
        };
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Drop Asset");
        self.push_editor_command(
            "Drop Asset",
            EditorCommandKind::CreateEntity { entity_id: id },
            before,
        );
        Ok(DropOutcome::SpawnedEntity(id))
    }

    pub fn cycle_tilemap_layer(&mut self) -> String {
        let name = self.tilemap_layers.cycle_layer();
        self.console
            .log(format!("Capa tilemap activa: {name}"), "TILEMAP");
        name
    }

    pub fn refresh_assets(&mut self) -> io::Result<usize> {
        self.resources.scan_all()?;
        if self.safe_mode.allows_asset_importers() {
            self.asset_database.scan()?;
        } else {
            self.console
                .warning("Reimportación de assets omitida por Safe Mode", "SAFE_MODE");
        }
        Ok(self.asset_database.assets.len())
    }

    pub fn export_project_package(&mut self) -> io::Result<ProjectPackageReport> {
        let name = self
            .project_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("MiniForgeProject");
        let output = self
            .project_paths
            .builds
            .join(format!("{name}_project.mfpkg.zip"));
        let report = ProjectPackageManager::export_project(&self.project_path, output)?;
        self.console.log(
            format!(
                "Proyecto exportado: {} ({} archivos)",
                report.archive_path.display(),
                report.files
            ),
            "PROJECT",
        );
        Ok(report)
    }

    pub fn import_project_package(
        &mut self,
        archive_path: impl AsRef<Path>,
        destination_root: impl AsRef<Path>,
    ) -> io::Result<ProjectPackageReport> {
        let report = ProjectPackageManager::import_project(archive_path, destination_root)?;
        self.console.log(
            format!(
                "Proyecto importado: {} ({} archivos)",
                report.project_path.display(),
                report.files
            ),
            "PROJECT",
        );
        Ok(report)
    }

    pub fn validate_project(&mut self) -> bool {
        let mut validator = ProjectValidator::default();
        let valid = validator.validate_with_context(
            &self.project_path,
            &self.runtime_world.units,
            Some(&self.asset_database),
        );
        for warning in &validator.warnings {
            self.console.log(warning, "WARNING");
        }
        for error in &validator.errors {
            self.console.log(error, "ERROR");
        }
        if valid {
            self.console
                .log("Proyecto validado sin errores", "VALIDATOR");
        }
        valid
    }

    pub fn build_manifest(&mut self) -> io::Result<Value> {
        let manifest = ManifestBuilder::build_manifest(&self.project_path)?;
        let total = manifest
            .as_object()
            .map(|map| {
                ["assets", "scripts", "components", "systems", "scenes"]
                    .iter()
                    .filter_map(|key| map.get(*key).and_then(Value::as_array))
                    .map(Vec::len)
                    .sum::<usize>()
            })
            .unwrap_or(0);
        self.console
            .log(format!("Manifest generado con {total} entradas"), "BUILD");
        Ok(manifest)
    }

    pub fn backend_plan(&mut self) -> io::Result<EngineBackendPlan> {
        let plan = EngineBackend::plan_project(&self.project_path)?;
        self.console.log(
            format!(
                "Backend plan: {} servicios, {} plugins, {} recursos, export_ready={}",
                plan.service_startup_order.len(),
                plan.plugins.load_order.len(),
                plan.resources.total_files,
                plan.export_ready
            ),
            "BACKEND",
        );
        for recommendation in plan.recommendations.iter().take(8) {
            self.console.log(recommendation, "BACKEND");
        }
        self.console
            .log(plan.system_audit.concise_summary(), "BACKEND");
        Ok(plan)
    }

    pub fn export_runtime(&mut self, profile: ExportProfile) -> io::Result<RuntimeExportReport> {
        self.save_project()?;
        let output_root = self.project_path.join("build");
        let report =
            RuntimeExporter::export_with_profile(&self.project_path, output_root, profile)?;
        self.console.log(
            format!(
                "Build {} listo: {} archivos, {} assets usados, {} faltantes",
                profile.label(),
                report.copied_files,
                report.used_assets.len(),
                report.missing_assets.len()
            ),
            "BUILD",
        );
        for missing in report.missing_assets.iter().take(8) {
            self.console
                .log(format!("Asset faltante en build: {missing}"), "WARNING");
        }
        self.console.log(
            format!("Readiness export: {}%", report.readiness_score),
            "BUILD",
        );
        for action in report.readiness_actions.iter().take(5) {
            self.console.log(format!("Next pass: {action}"), "BUILD");
        }
        Ok(report)
    }

    pub fn package_distributable(
        &mut self,
        profile: ExportProfile,
        label: &str,
    ) -> io::Result<crate::engine::packaging_manager::PackageReport> {
        self.save_project()?;
        let dest =
            self.project_path
                .join("packages")
                .join(format!("{}_{}", label, profile.label()));
        let report = crate::engine::packaging_manager::PackagingManager::package_project(
            &self.project_path,
            &dest,
            profile,
        )?;
        for w in report.warnings.iter().take(20) {
            self.console.log(w.clone(), "WARNING");
        }
        if let Some(runtime) = report.runtime_binary_copied.as_ref() {
            self.console
                .log(format!("Runtime incluido: {}", runtime.display()), "BUILD");
        }
        for launcher in &report.launcher_scripts {
            self.console.log(
                format!("Launcher generado: {}", launcher.display()),
                "BUILD",
            );
        }
        self.console.log(
            format!(
                "Paquete {} creado en {}",
                profile.label(),
                report.destination.display()
            ),
            "BUILD",
        );
        Ok(report)
    }

    pub fn recover_from_autosave(&mut self) -> Result<(), String> {
        let entities = self
            .autosave_manager
            .recover_entities()
            .map_err(|e| e.to_string())?;
        self.runtime_world.replace_entities(entities);
        self.sync_world();
        self.mark_scene_dirty("Recovered from autosave");
        self.console.log(
            "Escena restaurada desde autosave (saves/autosave)",
            "PROJECT",
        );
        Ok(())
    }

    pub fn create_project_template(&mut self, template_name: &str) -> io::Result<usize> {
        let created = ProjectTemplates::create(&self.project_path, template_name)?;
        self.refresh_assets().ok();
        self.console.log(
            format!("Template {template_name} creó {} archivos", created.len()),
            "PROJECT",
        );
        Ok(created.len())
    }

    pub fn set_workspace_mode(&mut self, mode: WorkspaceMode) {
        self.editor_workspace.apply_mode(mode);
        self.console.log(
            format!(
                "Workspace activo: {}",
                self.editor_workspace.active_mode.label()
            ),
            "EDITOR",
        );
    }

    pub fn cycle_workspace_mode(&mut self) -> WorkspaceMode {
        let mode = self.editor_workspace.cycle_mode();
        self.console
            .log(format!("Workspace activo: {}", mode.label()), "EDITOR");
        mode
    }

    pub fn create_program_asset(&mut self, template_name: &str) -> io::Result<PathBuf> {
        let path = self
            .programming
            .create_graph_asset(&self.project_path, template_name, None)?;
        self.script_editor.open(path.clone())?;
        self.docking_workspace
            .open_tab(EditorDockTab::BlueprintEditor);
        self.docking_workspace
            .set_floating_visibility("blueprint_editor", true);
        self.refresh_assets().ok();
        self.console
            .log(format!("Graph Rust creado: {}", path.display()), "SCRIPT");
        Ok(path)
    }

    pub fn create_luau_script_asset(&mut self, name: &str) -> io::Result<PathBuf> {
        let path = AssetTools::create_luau_script(&self.project_path, name)?;
        self.script_editor.open(path.clone())?;
        self.docking_workspace.open_tab(EditorDockTab::ScriptEditor);
        self.docking_workspace
            .set_floating_visibility("script_editor", true);
        self.refresh_assets().ok();
        self.console.log(
            format!("Script Luau creado y abierto: {}", path.display()),
            "SCRIPT",
        );
        Ok(path)
    }

    pub fn open_project_file(&mut self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let opened = self
            .script_editor
            .open_project_file(&self.project_path, path.as_ref())?;
        if opened.extension().and_then(|value| value.to_str()) == Some("mfgraph") {
            self.docking_workspace
                .open_tab(EditorDockTab::BlueprintEditor);
            self.docking_workspace
                .set_floating_visibility("blueprint_editor", true);
        } else {
            self.docking_workspace.open_tab(EditorDockTab::ScriptEditor);
            self.docking_workspace
                .set_floating_visibility("script_editor", true);
        }
        self.console
            .log(format!("Archivo abierto: {}", opened.display()), "EDITOR");
        self.event_bus.emit(
            if opened.extension().and_then(|value| value.to_str()) == Some("mfgraph") {
                "GraphOpened"
            } else {
                "ScriptOpened"
            },
            serde_json::json!({"path": opened.to_string_lossy()}),
        );
        Ok(opened)
    }

    pub fn edit_open_file(&mut self, text: impl Into<String>) {
        self.script_editor.set_text(text);
        self.console
            .log("Archivo abierto marcado como modificado", "EDITOR");
    }

    pub fn save_open_file(&mut self) -> io::Result<bool> {
        let Some(path) = self.script_editor.document.path.clone() else {
            self.console
                .log("No hay archivo abierto para guardar", "WARNING");
            return Ok(false);
        };
        self.script_editor.save()?;
        let valid = self.script_editor.validate();
        if let Some(error) = &self.script_editor.document.syntax_error {
            self.console
                .log(format!("Archivo guardado con error: {error}"), "ERROR");
        } else {
            self.console
                .log(format!("Archivo guardado: {}", path.display()), "EDITOR");
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("luau") || ext.eq_ignore_ascii_case("lua"))
        {
            self.luau_script_runtime.mark_script_changed(&path);
        }
        self.event_bus.emit(
            if path.extension().and_then(|value| value.to_str()) == Some("mfgraph") {
                "GraphSaved"
            } else {
                "ScriptSaved"
            },
            serde_json::json!({"path": path.to_string_lossy(), "valid": valid}),
        );
        self.refresh_assets().ok();
        Ok(valid)
    }

    pub fn reload_open_file(&mut self) -> io::Result<()> {
        let Some(path) = self.script_editor.document.path.clone() else {
            return Ok(());
        };
        self.script_editor.open(path.clone())?;
        self.console
            .log(format!("Archivo recargado: {}", path.display()), "EDITOR");
        Ok(())
    }

    pub fn current_visual_graph_view(&self) -> Option<VisualGraphView> {
        let path = self.script_editor.document.path.as_ref()?;
        if path.extension().and_then(|value| value.to_str()) != Some("mfgraph") {
            return None;
        }
        let graph: Value = serde_json::from_str(&self.script_editor.text()).ok()?;
        Some(self.programming.graph_view(&graph))
    }

    pub fn connect_open_graph_nodes(&mut self, from: &str, to: &str) -> io::Result<bool> {
        self.connect_open_graph_nodes_on_pin(from, to, "exec")
    }

    pub fn connect_open_graph_nodes_on_pin(
        &mut self,
        from: &str,
        to: &str,
        pin: &str,
    ) -> io::Result<bool> {
        let mut graph = self.open_graph_json()?;
        let changed = ProgrammingEnvironment::connect_graph_nodes_on_pin(&mut graph, from, to, pin);
        if changed {
            self.replace_open_graph_json(&graph)?;
            self.console
                .log(format!("Graph conectado: {from}.{pin} -> {to}"), "SCRIPT");
        }
        Ok(changed)
    }

    pub fn move_open_graph_node(&mut self, node_id: &str, x: f64, y: f64) -> io::Result<bool> {
        let mut graph = self.open_graph_json()?;
        let changed = ProgrammingEnvironment::move_graph_node(&mut graph, node_id, x, y);
        if changed {
            self.replace_open_graph_json(&graph)?;
        }
        Ok(changed)
    }

    pub fn add_node_to_open_graph(&mut self, node_type: &str) -> io::Result<Option<String>> {
        let mut graph = self.open_graph_json()?;
        let id = ProgrammingEnvironment::add_graph_node(&mut graph, node_type);
        if id.is_some() {
            self.replace_open_graph_json(&graph)?;
            self.console
                .log(format!("Nodo {node_type} agregado al graph"), "SCRIPT");
        }
        Ok(id)
    }

    pub fn add_quick_action_to_open_graph(
        &mut self,
        action: &VisualGraphQuickAction,
    ) -> io::Result<Vec<String>> {
        let mut graph = self.open_graph_json()?;
        let ids = ProgrammingEnvironment::add_quick_action_to_graph(&mut graph, action);
        if !ids.is_empty() {
            self.replace_open_graph_json(&graph)?;
            self.console.log(
                format!("Quick action agregada al graph: {}", action.label),
                "SCRIPT",
            );
        }
        Ok(ids)
    }

    fn open_graph_json(&self) -> io::Result<Value> {
        let Some(path) = self.script_editor.document.path.as_ref() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No hay graph visual abierto",
            ));
        };
        if path.extension().and_then(|value| value.to_str()) != Some("mfgraph") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "El archivo abierto no es .mfgraph",
            ));
        }
        serde_json::from_str(&self.script_editor.text()).map_err(io::Error::other)
    }

    fn replace_open_graph_json(&mut self, graph: &Value) -> io::Result<()> {
        let text = serde_json::to_string_pretty(graph).map_err(io::Error::other)?;
        self.script_editor.set_text(text);
        self.script_editor.validate();
        Ok(())
    }

    pub fn create_sprite_import_asset(
        &mut self,
        name: &str,
        source_path: &str,
    ) -> io::Result<PathBuf> {
        let path = AssetTools::create_sprite_import(&self.project_path, name, source_path)?;
        self.refresh_assets().ok();
        self.console.log(
            format!("Sprite import creado: {}", path.display()),
            "ASSETS",
        );
        Ok(path)
    }

    pub fn new_sprite_canvas(&mut self, width: u32, height: u32) {
        self.sprite_editor = SpriteEditorCanvas::new(width, height);
        self.console.log(
            format!("Sprite canvas nuevo: {}x{}", width, height),
            "SPRITE",
        );
    }

    pub fn load_sprite_canvas(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        self.sprite_editor = SpriteEditorCanvas::load_png(path.as_ref())?;
        self.console.log(
            format!("Sprite abierto: {}", path.as_ref().display()),
            "SPRITE",
        );
        Ok(())
    }

    pub fn save_sprite_canvas(&mut self, name: &str) -> io::Result<PathBuf> {
        let mut filename = AssetTools::safe_name(name, "Sprite");
        if !filename.ends_with(".png") {
            filename.push_str(".png");
        }
        let path = AssetTools::unique_path(&self.project_paths.sprites, &filename);
        self.sprite_editor.save_png(&path)?;
        self.refresh_assets().ok();
        self.console
            .log(format!("Sprite guardado: {}", path.display()), "SPRITE");
        Ok(path)
    }

    pub fn save_sprite_canvas_current(&mut self, fallback_name: &str) -> io::Result<PathBuf> {
        let path = if let Some(path) = self.sprite_editor.last_path.clone() {
            path
        } else {
            let mut filename = AssetTools::safe_name(fallback_name, "Sprite");
            if !filename.ends_with(".png") {
                filename.push_str(".png");
            }
            AssetTools::unique_path(&self.project_paths.sprites, &filename)
        };
        self.sprite_editor.save_png(&path)?;
        self.refresh_assets().ok();
        self.console
            .log(format!("Sprite guardado: {}", path.display()), "SPRITE");
        Ok(path)
    }

    pub fn paint_sprite_pixel(&mut self, x: u32, y: u32, color: SpriteColor) -> bool {
        let changed = self.sprite_editor.set_pixel(x, y, color);
        if changed {
            self.console
                .debug(format!("Pixel sprite {x},{y} actualizado"), "SPRITE");
        }
        changed
    }

    pub fn create_sound_cue_asset(&mut self, name: &str, source_path: &str) -> io::Result<PathBuf> {
        let path = AssetTools::create_sound_cue(&self.project_path, name, source_path)?;
        self.refresh_assets().ok();
        self.console
            .log(format!("Sound cue creado: {}", path.display()), "ASSETS");
        Ok(path)
    }

    pub fn create_material_asset(&mut self, name: &str) -> io::Result<PathBuf> {
        let path = AssetTools::create_material(&self.project_path, name)?;
        self.refresh_assets().ok();
        self.console
            .log(format!("Material creado: {}", path.display()), "ASSETS");
        Ok(path)
    }

    pub fn add_audio_to_selected(&mut self, audio_name: &str, play_on_start: bool) -> bool {
        let Some(id) = self.selected_units.first().copied() else {
            return false;
        };
        let Some(entity) = self.get_entity_by_id_mut(id) else {
            return false;
        };
        if entity.get_component("AudioSource").is_none() {
            entity.add_component(default_component("AudioSource").expect("AudioSource"));
        }
        if let Some(audio) = entity.get_component_mut("AudioSource") {
            audio.set("audio_name", json!(audio_name));
            audio.set("play_on_start", json!(play_on_start));
        }
        self.mark_scene_dirty("Add AudioSource");
        true
    }

    pub fn attach_program_template_to_selected(&mut self, template_name: &str) -> Option<String> {
        let id = self.selected_units.first().copied()?;
        let index = self
            .runtime_world
            .units
            .iter()
            .position(|entity| entity.id == id)?;
        let graph_name = {
            let programming = &mut self.programming;
            let entity = &mut self.runtime_world.units[index];
            programming.attach_template_to_entity(entity, template_name)
        };
        self.mark_scene_dirty("Attach Visual Graph");
        self.console.log(
            format!("Graph {graph_name} conectado a entity #{id}"),
            "SCRIPT",
        );
        Some(graph_name)
    }

    pub fn save_selected_as_prefab(&mut self) -> io::Result<Option<PathBuf>> {
        let Some(id) = self.selected_units.first().copied() else {
            return Ok(None);
        };
        let before = self.capture_editor_snapshot();
        let Some(index) = self
            .runtime_world
            .units
            .iter()
            .position(|entity| entity.id == id)
        else {
            return Ok(None);
        };
        let dependencies = self
            .runtime_world
            .units
            .get(index)
            .and_then(|entity| entity.sprite_guid.clone())
            .into_iter()
            .collect::<Vec<_>>();
        let project_path = self.project_path.clone();
        let path = {
            let system = &mut self.advanced_prefabs;
            let entity = &mut self.runtime_world.units[index];
            system.create_prefab_from_entity(project_path, entity, false, dependencies)?
        };
        self.refresh_assets().ok();
        self.mark_scene_dirty("Save Prefab");
        self.push_editor_command(
            "Create Prefab",
            EditorCommandKind::SceneOperation {
                name: "Create Prefab".to_string(),
            },
            before,
        );
        self.console
            .log(format!("Prefab guardado: {}", path.display()), "PREFAB");
        Ok(Some(path))
    }

    pub fn create_selected_prefab_variant(&mut self) -> io::Result<Option<PathBuf>> {
        let Some(id) = self.selected_units.first().copied() else {
            return Ok(None);
        };
        let Some(index) = self
            .runtime_world
            .units
            .iter()
            .position(|entity| entity.id == id)
        else {
            return Ok(None);
        };
        let project_path = self.project_path.clone();
        let path = {
            let system = &mut self.advanced_prefabs;
            let entity = &mut self.runtime_world.units[index];
            system.create_variant_from_entity(project_path, entity)?
        };
        self.refresh_assets().ok();
        self.console
            .log(format!("Variant creado: {}", path.display()), "PREFAB");
        Ok(Some(path))
    }

    pub fn apply_selected_to_prefab_source(&mut self) -> io::Result<bool> {
        let Some(id) = self.selected_units.first().copied() else {
            return Ok(false);
        };
        let Some(index) = self
            .runtime_world
            .units
            .iter()
            .position(|entity| entity.id == id)
        else {
            return Ok(false);
        };
        let Some(source) = self.runtime_world.units[index].prefab_source.clone() else {
            return Ok(false);
        };
        let path = PathBuf::from(&source);
        if !path.exists() {
            return Ok(false);
        }
        let entity = &mut self.runtime_world.units[index];
        entity.sync_to_components();
        let data = PrefabSerializer::stamp(json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "kind": "MiniForgeAdvancedPrefab",
            "prefab_name": entity.name,
            "guid": entity.prefab_guid,
            "variant": false,
            "entity": entity.serialize(),
            "metadata": {
                "component_count": entity.components.len(),
                "script_count": entity.scripts.len(),
                "source": "apply_instance",
            }
        }))
        .map_err(io::Error::from)?;
        ProjectStorage::write_json_atomic_with_backup(
            &path,
            &data,
            BackupPolicy::new(
                path.with_extension("prefab.bak"),
                DEFAULT_BACKUP_GENERATIONS,
            ),
        )
        .map_err(io::Error::from)?;
        self.refresh_assets().ok();
        self.mark_scene_dirty("Apply Prefab");
        self.console
            .log(format!("Prefab aplicado: {}", path.display()), "PREFAB");
        Ok(true)
    }

    pub fn revert_selected_prefab_instance(&mut self) -> io::Result<bool> {
        let Some(id) = self.selected_units.first().copied() else {
            return Ok(false);
        };
        let before = self.capture_editor_snapshot();
        let Some(index) = self
            .runtime_world
            .units
            .iter()
            .position(|entity| entity.id == id)
        else {
            return Ok(false);
        };
        let Some(source) = self.runtime_world.units[index].prefab_source.clone() else {
            return Ok(false);
        };
        let manager = PrefabManager::new(&self.project_path);
        let Some(mut loaded) = manager.load_prefab(&source)? else {
            return Ok(false);
        };
        loaded.id = id;
        loaded.x = self.runtime_world.units[index].x;
        loaded.y = self.runtime_world.units[index].y;
        loaded.prefab_source = Some(source);
        loaded.prefab_guid = self.runtime_world.units[index].prefab_guid.clone();
        loaded.is_prefab_instance = true;
        loaded.sync_to_components();
        self.runtime_world.units[index] = loaded;
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Revert Prefab");
        self.push_editor_command(
            "Revert Prefab",
            EditorCommandKind::SceneOperation {
                name: "Revert Prefab".to_string(),
            },
            before,
        );
        self.console
            .log(format!("Prefab revertido #{id}"), "PREFAB");
        Ok(true)
    }

    pub fn detach_selected_prefab_instance(&mut self) -> bool {
        let Some(id) = self.selected_units.first().copied() else {
            return false;
        };
        let before = self.capture_editor_snapshot();
        let Some(entity) = self.get_entity_by_id_mut(id) else {
            return false;
        };
        entity.is_prefab_instance = false;
        entity.prefab_source = None;
        entity.prefab_guid = None;
        self.mark_scene_dirty("Detach Prefab");
        self.push_editor_command(
            "Detach Prefab",
            EditorCommandKind::SceneOperation {
                name: "Detach Prefab".to_string(),
            },
            before,
        );
        self.console
            .log(format!("Prefab desconectado de entity #{id}"), "PREFAB");
        true
    }

    pub fn instantiate_first_prefab(&mut self, x: f64, y: f64) -> io::Result<Option<u64>> {
        let before = self.capture_editor_snapshot();
        let Some(prefab) = self
            .asset_database
            .assets
            .values()
            .find(|asset| asset.asset_type == "Prefab")
            .cloned()
        else {
            return Ok(None);
        };
        let manager = PrefabManager::new(&self.project_path);
        let path = self.project_path.join(&prefab.relative_path);
        let id = manager.instantiate_prefab(&mut self.runtime_world.units, path, x, y)?;
        if let Some(id) = id {
            self.select_entity(id);
            self.sync_world();
            self.mark_scene_dirty("Instantiate Prefab");
            self.push_editor_command(
                "Instantiate Prefab",
                EditorCommandKind::CreateEntity { entity_id: id },
                before,
            );
            self.console.log(
                format!("Prefab instanciado: {} #{id}", prefab.name),
                "PREFAB",
            );
        }
        Ok(id)
    }

    pub fn instantiate_prefab_asset(
        &mut self,
        relative_path: &str,
        x: f64,
        y: f64,
    ) -> io::Result<Option<u64>> {
        let before = self.capture_editor_snapshot();
        let manager = PrefabManager::new(&self.project_path);
        let path = self.project_path.join(relative_path);
        let id = manager.instantiate_prefab(&mut self.runtime_world.units, &path, x, y)?;
        if let Some(id) = id {
            self.select_entity(id);
            self.sync_world();
            self.mark_scene_dirty("Instantiate Prefab");
            self.push_editor_command(
                "Instantiate Prefab",
                EditorCommandKind::CreateEntity { entity_id: id },
                before,
            );
            self.console.log(
                format!("Prefab instanciado desde {relative_path}: #{id}"),
                "PREFAB",
            );
        }
        Ok(id)
    }

    pub fn analyze_selected_prefab(
        &mut self,
    ) -> Option<crate::engine::advanced_prefabs::PrefabInstanceReport> {
        let id = self.selected_units.first().copied()?;
        let entity = self.get_entity_by_id(id)?.clone();
        let source_data = entity
            .prefab_source
            .as_deref()
            .and_then(|path| AssetTools::read_json(path).ok());
        Some(
            self.advanced_prefabs
                .analyze_instance(&entity, source_data.as_ref()),
        )
    }

    pub fn visual_graph_asset_count(&self) -> usize {
        self.asset_database
            .assets
            .values()
            .filter(|asset| asset.asset_type == "VisualGraph")
            .count()
    }

    pub fn scene_summary(&self) -> String {
        let prefab_instances = self
            .runtime_world
            .units
            .iter()
            .filter(|entity| entity.is_prefab_instance)
            .count();
        let visual_graphs = self
            .runtime_world
            .units
            .iter()
            .filter(|entity| entity.get_component("VisualScript").is_some())
            .count();
        format!(
            "{} entities | {} prefab instances | {} visual graph components | {} assets | {} graph assets",
            self.runtime_world.units.len(),
            prefab_instances,
            visual_graphs,
            self.asset_database.assets.len(),
            self.visual_graph_asset_count()
        )
    }

    pub fn sync_world(&mut self) {
        self.runtime_world.mark_changed();
        self.runtime_world.rebuild_index();
    }

    pub fn create_topdown_starter(&mut self) -> usize {
        self.runtime_world.units.clear();
        self.selected_units.clear();
        self.grid = Grid::new(60, 40, 32, 8);
        self.tilemap_layers = TilemapLayers::new(self.grid.width, self.grid.height);
        self.tilemap_layers.active_layer = 0;
        self.tilemap_layers
            .fill_active(0, 0, self.grid.width, self.grid.height, 1);
        self.tilemap_layers.active_layer = 1;
        for x in (5..self.grid.width).step_by(7) {
            for y in (4..self.grid.height).step_by(6) {
                self.tilemap_layers.set_tile(x, y, 2);
            }
        }
        self.tilemap_layers.active_layer = 2;
        for x in 0..self.grid.width {
            self.tilemap_layers.set_tile(x, 0, 4);
            self.tilemap_layers.set_tile(x, self.grid.height - 1, 4);
            self.grid.set_tile(x, 0, 1);
            self.grid.set_tile(x, self.grid.height - 1, 1);
        }
        for y in 0..self.grid.height {
            self.tilemap_layers.set_tile(0, y, 4);
            self.tilemap_layers.set_tile(self.grid.width - 1, y, 4);
            self.grid.set_tile(0, y, 1);
            self.grid.set_tile(self.grid.width - 1, y, 1);
        }
        self.tilemap_layers.active_layer = 0;

        let player_id = self.spawn_unit("Hero", 8.0, 8.0);
        if let Some(player) = self.get_entity_by_id_mut(player_id) {
            player.layer = "Units".to_string();
            player.add_component(default_component("Rigidbody2D").expect("Rigidbody2D"));
            player.add_component(
                default_component("CharacterController2D").expect("CharacterController2D"),
            );
            player.add_component(default_component("CameraFollow").expect("CameraFollow"));
            player.add_component(default_component("Equipment").expect("Equipment"));
            player.add_component(default_component("Ability").expect("Ability"));
            player.add_component(default_component("QuestLog").expect("QuestLog"));
            player.add_component(default_component("Saveable").expect("Saveable"));
            player.add_component(default_component("Checkpoint").expect("Checkpoint"));
            player.add_component(default_component("Light2D").expect("Light2D"));
            if let Some(body) = player.get_component_mut("Rigidbody2D") {
                body.set("use_gravity", json!(false));
                body.set_f64("drag", 0.2);
            }
            if let Some(controller) = player.get_component_mut("CharacterController2D") {
                controller.set("mode", json!("topdown"));
                controller.set_f64("dash_speed", 14.0);
                controller.set_f64("dash_cooldown", 0.55);
            }
            if let Some(checkpoint) = player.get_component_mut("Checkpoint") {
                checkpoint.set("active", json!(true));
                checkpoint.set_f64("respawn_x", 8.0);
                checkpoint.set_f64("respawn_y", 8.0);
            }
            if let Some(camera) = player.get_component_mut("CameraFollow") {
                camera.set("target_id", json!(player_id));
                camera.set_f64("zoom", 1.15);
            }
            if let Some(ability) = player.get_component_mut("Ability") {
                ability.set("ability_id", json!("dash"));
                ability.set("display_name", json!("Dash"));
                ability.set_f64("cooldown", 0.8);
            }
            player.sync_to_components();
        }

        for (x, y) in [(15.0, 8.0), (19.0, 13.0), (24.0, 10.0)] {
            self.spawn_enemy(x, y);
        }
        for (x, y) in [(11.0, 12.0), (28.0, 12.0), (22.0, 18.0)] {
            self.spawn_resource(x, y);
        }

        let npc_id = self.spawn_game_object("QuestNPC", 12.0, 7.0);
        if let Some(npc) = self.get_entity_by_id_mut(npc_id) {
            npc.tag = "Neutral".to_string();
            npc.add_component(default_component("Dialogue").expect("Dialogue"));
            npc.add_component(default_component("Interaction").expect("Interaction"));
            npc.add_component(default_component("ObjectiveMarker").expect("ObjectiveMarker"));
            if let Some(dialogue) = npc.get_component_mut("Dialogue") {
                dialogue.set("speaker", json!("Guide"));
                dialogue.set(
                    "lines",
                    json!(["Find the gold nodes.", "Defeat the patrol."]),
                );
            }
        }

        for index in 0..3 {
            let pickup_id = self.spawn_game_object("HealthPickup", 10.0 + index as f64 * 3.0, 15.0);
            if let Some(pickup_index) = self
                .runtime_world
                .units
                .iter()
                .position(|entity| entity.id == pickup_id)
            {
                let pickup = &mut self.runtime_world.units[pickup_index];
                pickup.tag = "Neutral".to_string();
                pickup.add_component(default_component("Interaction").expect("Interaction"));
                pickup
                    .add_component(default_component("ObjectiveMarker").expect("ObjectiveMarker"));
                self.programming
                    .attach_template_to_entity(pickup, "HealthPickup");
            }
        }

        self.create_ui_label("Quest: collect resources and survive", 24.0, 24.0);
        self.create_ui_progress_bar("HP", 24.0, 58.0);
        self.camera.x = 96.0;
        self.camera.y = 96.0;
        self.camera.set_zoom(1.1);
        self.set_workspace_mode(WorkspaceMode::WorldBuilding);
        self.select_entity(player_id);
        self.sync_world();
        self.mark_scene_dirty("Create TopDown Starter");
        self.console.log(
            "Starter TopDown creado: player, enemigos, pickups, NPC, UI y tilemap.",
            "PROJECT",
        );
        self.runtime_world.units.len()
    }

    pub fn create_platformer_starter(&mut self) -> usize {
        self.runtime_world.units.clear();
        self.selected_units.clear();
        self.grid = Grid::new(80, 28, 32, 8);
        self.tilemap_layers = TilemapLayers::new(self.grid.width, self.grid.height);
        self.tilemap_layers.active_layer = 0;
        self.tilemap_layers
            .fill_active(0, 0, self.grid.width, self.grid.height, 1);
        self.tilemap_layers.active_layer = 2;
        let floor_y = self.grid.height - 4;
        for x in 0..self.grid.width {
            for y in floor_y..self.grid.height {
                self.tilemap_layers.set_tile(x, y, 4);
                self.grid.set_tile(x, y, 1);
            }
        }
        for (start_x, y, width) in [(8, 18, 8), (22, 15, 7), (36, 12, 9), (54, 17, 10)] {
            for x in start_x..start_x + width {
                self.tilemap_layers.set_tile(x, y, 4);
                self.grid.set_tile(x, y, 1);
            }
        }
        self.tilemap_layers.active_layer = 1;
        for x in (3..self.grid.width).step_by(5) {
            self.tilemap_layers.set_tile(x, floor_y - 1, 3);
        }
        self.tilemap_layers.active_layer = 0;

        let player_id = self.spawn_game_object("PlatformerPlayer", 5.0, floor_y as f64 - 2.0);
        if let Some(player) = self.get_entity_by_id_mut(player_id) {
            player.tag = "Player".to_string();
            player.layer = "Units".to_string();
            player.width = 0.85;
            player.height = 1.4;
            player.add_component(default_component("Health").expect("Health"));
            player.add_component(default_component("Stats").expect("Stats"));
            player.add_component(default_component("Rigidbody2D").expect("Rigidbody2D"));
            player.add_component(
                default_component("CharacterController2D").expect("CharacterController2D"),
            );
            player.add_component(default_component("CameraFollow").expect("CameraFollow"));
            player.add_component(default_component("Checkpoint").expect("Checkpoint"));
            player.add_component(default_component("Saveable").expect("Saveable"));
            player.add_component(default_component("Light2D").expect("Light2D"));
            if let Some(body) = player.get_component_mut("Rigidbody2D") {
                body.set("use_gravity", json!(true));
                body.set_f64("gravity_scale", 1.0);
                body.set("freeze_rotation", json!(true));
            }
            if let Some(controller) = player.get_component_mut("CharacterController2D") {
                controller.set("mode", json!("platformer"));
                controller.set_f64("walk_speed", 5.5);
                controller.set_f64("jump_force", 10.0);
                controller.set("max_jumps", json!(2));
                controller.set_f64("dash_speed", 12.0);
                controller.set_f64("fall_death_y", floor_y as f64 + 8.0);
            }
            if let Some(checkpoint) = player.get_component_mut("Checkpoint") {
                checkpoint.set("active", json!(true));
                checkpoint.set_f64("respawn_x", 5.0);
                checkpoint.set_f64("respawn_y", floor_y as f64 - 2.0);
            }
            if let Some(camera) = player.get_component_mut("CameraFollow") {
                camera.set("target_id", json!(player_id));
                camera.set_f64("offset_y", -1.2);
                camera.set_f64("zoom", 1.0);
            }
            player.sync_to_components();
        }

        let collider_id = self.spawn_game_object("TilemapCollision", 0.0, 0.0);
        if let Some(collider) = self.get_entity_by_id_mut(collider_id) {
            collider.visible = false;
            collider.locked = true;
            collider.layer = "EditorOnly".to_string();
            collider.add_component(default_component("TilemapCollider").expect("TilemapCollider"));
        }

        for (x, y) in [(18.0, floor_y as f64 - 1.0), (35.0, 11.0), (52.0, 16.0)] {
            let enemy_id = self.spawn_enemy(x, y);
            if let Some(enemy) = self.get_entity_by_id_mut(enemy_id)
                && let Some(ai) = enemy.get_component_mut("AIController")
            {
                ai.set("behavior", json!("patrol"));
                ai.set_f64("patrol_radius", 3.0);
            }
        }
        for (index, x) in [10.0, 13.0, 26.0, 39.0, 42.0, 58.0].iter().enumerate() {
            let coin_id =
                self.spawn_game_object(&format!("Coin_{index}"), *x, floor_y as f64 - 2.0);
            if let Some(coin) = self.get_entity_by_id_mut(coin_id) {
                coin.tag = "Neutral".to_string();
                coin.add_component(default_component("Interaction").expect("Interaction"));
                coin.add_component(default_component("ObjectiveMarker").expect("ObjectiveMarker"));
                coin.add_component(default_component("Lifetime").expect("Lifetime"));
                if let Some(lifetime) = coin.get_component_mut("Lifetime") {
                    lifetime.set_f64("duration", -1.0);
                }
            }
        }

        let checkpoint_id = self.spawn_game_object("Checkpoint_A", 44.0, 10.0);
        if let Some(checkpoint) = self.get_entity_by_id_mut(checkpoint_id) {
            checkpoint.add_component(default_component("Checkpoint").expect("Checkpoint"));
            checkpoint.add_component(default_component("Interaction").expect("Interaction"));
            checkpoint.add_component(default_component("Light2D").expect("Light2D"));
            if let Some(component) = checkpoint.get_component_mut("Checkpoint") {
                component.set("checkpoint_id", json!("checkpoint_a"));
                component.set_f64("respawn_x", 44.0);
                component.set_f64("respawn_y", 10.0);
            }
        }

        self.create_ui_label("Platformer: reach the checkpoint", 24.0, 24.0);
        self.create_ui_progress_bar("HP", 24.0, 58.0);
        self.camera.x = 0.0;
        self.camera.y = 320.0;
        self.camera.set_zoom(1.0);
        self.set_workspace_mode(WorkspaceMode::WorldBuilding);
        self.select_entity(player_id);
        self.sync_world();
        self.mark_scene_dirty("Create Platformer Starter");
        self.console.log(
            "Starter Platformer creado: controller 2D, plataformas, monedas, checkpoint y enemigos.",
            "PROJECT",
        );
        self.runtime_world.units.len()
    }

    pub fn create_ui_label(&mut self, text: &str, x: f64, y: f64) -> u64 {
        let mut entity = GameObject::new(0.0, 0.0, Some("UI_Label".to_string()));
        let mut ui = default_component("UIElement").expect("UIElement");
        ui.set("element_type", json!("Label"));
        ui.set("text", json!(text));
        ui.set_f64("x", x);
        ui.set_f64("y", y);
        ui.set_f64("width", 220.0);
        ui.set_f64("height", 32.0);
        ui.set("text_align", json!("left"));
        entity.add_component(ui);
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.sync_world();
        self.mark_scene_dirty("Create UI Label");
        id
    }

    pub fn create_ui_button(&mut self, text: &str, x: f64, y: f64) -> u64 {
        let mut entity = GameObject::new(0.0, 0.0, Some("UI_Button".to_string()));
        let mut ui = default_component("UIElement").expect("UIElement");
        ui.set("element_type", json!("Button"));
        ui.set("text", json!(text));
        ui.set_f64("x", x);
        ui.set_f64("y", y);
        ui.set_f64("width", 180.0);
        ui.set_f64("height", 42.0);
        ui.set("interactable", json!(true));
        entity.add_component(ui);
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.sync_world();
        self.mark_scene_dirty("Create UI Button");
        id
    }

    pub fn create_ui_progress_bar(&mut self, text: &str, x: f64, y: f64) -> u64 {
        let mut entity = GameObject::new(0.0, 0.0, Some("UI_ProgressBar".to_string()));
        let mut ui = default_component("UIElement").expect("UIElement");
        ui.set("element_type", json!("ProgressBar"));
        ui.set("text", json!(text));
        ui.set_f64("x", x);
        ui.set_f64("y", y);
        ui.set_f64("width", 240.0);
        ui.set_f64("height", 24.0);
        ui.set_f64("progress", 0.75);
        ui.set_f64("max_progress", 1.0);
        entity.add_component(ui);
        let id = entity.id;
        self.runtime_world.units.push(entity);
        self.sync_world();
        self.mark_scene_dirty("Create UI ProgressBar");
        id
    }

    pub fn ensure_default_ui_canvas_scene_data(&mut self) {
        if self
            .ui_canvases
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(true)
        {
            push_canvas(&mut self.ui_canvases, UiCanvasRoot::default_hud());
            self.mark_scene_dirty("UI Canvas default");
        }
    }

    pub fn add_ui_canvas_scene_label(&mut self, text: &str) {
        self.ensure_default_ui_canvas_scene_data();
        let mut roots = ui_canvases_from_value(&self.ui_canvases);
        if let Some(first) = roots.get_mut(0) {
            first.elements.push(UiCanvasElement::Label {
                id: generate_entity_name("ui_lbl"),
                text: text.to_string(),
                rect: UiRect::default(),
                font_size: 16.0,
            });
        }
        self.ui_canvases = serde_json::to_value(&roots).unwrap_or_else(|_| json!([]));
        self.mark_scene_dirty("UI Canvas scene label");
    }

    pub fn add_visual_script_template(
        &mut self,
        entity_id: u64,
        template_name: &str,
    ) -> Option<String> {
        let entity = self.get_entity_by_id_mut(entity_id)?;
        if entity.get_component("VisualScript").is_none() {
            entity.add_component(default_component("VisualScript").expect("VisualScript"));
        }
        let visual = entity.get_component_mut("VisualScript")?;
        let graph_name = match template_name {
            "Button Click" => {
                visual.set("graph_name", json!("ButtonClick"));
                visual.set(
                    "nodes",
                    json!([
                        {"id": "click", "type": "EventClick", "next": "log"},
                        {"id": "log", "type": "Log", "message": "Button clicked", "next": null}
                    ]),
                );
                "ButtonClick"
            }
            "Damage Self" => {
                visual.set("graph_name", json!("DamageSelf"));
                visual.set(
                    "nodes",
                    json!([
                        {"id": "start", "type": "EventStart", "next": "damage"},
                        {"id": "damage", "type": "Damage", "amount": 5, "next": null}
                    ]),
                );
                "DamageSelf"
            }
            _ => {
                visual.set("graph_name", json!("LogAndMove"));
                visual.set(
                    "nodes",
                    json!([
                        {"id": "start", "type": "EventStart", "next": "log"},
                        {"id": "log", "type": "Log", "message": "Visual script running", "next": "move"},
                        {"id": "move", "type": "Move", "x": 1, "y": 0, "next": null}
                    ]),
                );
                "LogAndMove"
            }
        };
        self.mark_scene_dirty("Visual Script Template");
        Some(graph_name.to_string())
    }

    pub fn apply_component_preset(&mut self, entity_id: u64, preset: &str) -> bool {
        let Some(entity) = self.get_entity_by_id_mut(entity_id) else {
            return false;
        };
        match preset {
            "Playable Unit" => {
                entity.add_component(default_component("Health").expect("Health"));
                entity.add_component(default_component("Stats").expect("Stats"));
            }
            "TopDown Player" => {
                entity.tag = "Player".to_string();
                entity.add_component(default_component("Rigidbody2D").expect("Rigidbody2D"));
                entity.add_component(default_component("Inventory").expect("Inventory"));
                entity.add_component(default_component("Cooldown").expect("Cooldown"));
            }
            "Enemy AI" => {
                entity.tag = "Enemy".to_string();
                let mut ai = default_component("AIController").expect("AIController");
                ai.set("behavior", json!("attack"));
                entity.add_component(ai);
                entity.add_component(default_component("DamageDealer").expect("DamageDealer"));
                entity.add_component(default_component("Health").expect("Health"));
            }
            "Quest NPC" => {
                entity.add_component(default_component("Dialogue").expect("Dialogue"));
                entity.add_component(default_component("Interaction").expect("Interaction"));
            }
            "RTS Worker" => {
                entity.tag = "Player".to_string();
                entity.layer = "Units".to_string();
                entity.add_component(default_component("Worker").expect("Worker"));
                entity.add_component(default_component("NavAgent").expect("NavAgent"));
                entity.add_component(default_component("Commandable").expect("Commandable"));
                entity.add_component(default_component("Vision").expect("Vision"));
                Self::apply_team(entity, 1);
            }
            "RTS Building" => {
                entity.tag = "Player".to_string();
                entity.layer = "Buildings".to_string();
                entity.add_component(default_component("Buildable").expect("Buildable"));
                entity
                    .add_component(default_component("ProductionQueue").expect("ProductionQueue"));
                entity.add_component(default_component("EconomyWallet").expect("EconomyWallet"));
                entity.add_component(default_component("Commandable").expect("Commandable"));
                entity.add_component(default_component("Vision").expect("Vision"));
                Self::apply_team(entity, 1);
            }
            _ => return false,
        }
        self.mark_scene_dirty("Apply Component Preset");
        true
    }

    fn apply_team(entity: &mut GameObject, team_id: i64) {
        if entity.get_component("Team").is_none() {
            entity.add_component(default_component("Team").expect("Team"));
        }
        if let Some(team) = entity.get_component_mut("Team") {
            team.set("team_id", json!(team_id));
            team.set(
                "team_name",
                json!(match team_id {
                    1 => "Player",
                    2 => "Enemy",
                    _ => "Neutral",
                }),
            );
            team.set(
                "color",
                json!(match team_id {
                    1 => [80, 160, 255],
                    2 => [255, 95, 95],
                    _ => [160, 160, 160],
                }),
            );
        }
    }

    fn entity_team_id(entity: &GameObject) -> i64 {
        entity
            .get_component("Team")
            .map(|team| team.get_i64("team_id", 0))
            .unwrap_or_else(|| match entity.tag.as_str() {
                "Player" => 1,
                "Enemy" => 2,
                _ => 0,
            })
    }

    pub fn save_scene(&mut self) -> io::Result<()> {
        let world_report = self.runtime_world.validate();
        if !world_report.is_valid() {
            for duplicate_id in world_report.duplicate_ids.iter().take(8) {
                self.console.error(
                    format!("RuntimeWorld contiene ID duplicado: {duplicate_id}"),
                    "WORLD",
                );
            }
            for (child_id, parent_id) in world_report.dangling_parent_ids.iter().take(8) {
                self.console.error(
                    format!(
                        "RuntimeWorld contiene parent roto: entity {child_id} -> parent {parent_id}"
                    ),
                    "WORLD",
                );
            }
            for cycle in world_report.hierarchy_cycles.iter().take(8) {
                self.console.error(
                    format!("RuntimeWorld contiene ciclo de parenting: {cycle:?}"),
                    "WORLD",
                );
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "El mundo runtime contiene identidades o jerarquia invalidas.",
            ));
        }
        if !self
            .scene_validator
            .validate_entities(&self.runtime_world.units)
        {
            for error in self.scene_validator.errors.iter().take(8) {
                self.console.log(error.clone(), "ERROR");
            }
            for warning in self.scene_validator.warnings.iter().take(8) {
                self.console.log(warning.clone(), "WARNING");
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "La escena tiene errores de validacion; revisa la consola.",
            ));
        }
        for warning in self.scene_validator.warnings.iter().take(8) {
            self.console.log(warning.clone(), "WARNING");
        }
        self.scene_save_manager.save_scene(
            &self.scene_manager,
            &mut self.runtime_world.units,
            &self.tilemap_layers,
            &self.camera,
            &self.mode,
            &self.active_tool,
            self.tile_brush,
            self.brush_size,
            &self.grid,
            &self.ui_canvases,
        )?;
        self.mark_scene_clean();
        Ok(())
    }

    pub fn create_empty_scene(&mut self, name: &str) -> io::Result<PathBuf> {
        let path = self.scene_manager.create_new_scene(name)?;
        self.runtime_world.units.clear();
        self.selected_units.clear();
        self.tilemap_layers = TilemapLayers::new(self.grid.width, self.grid.height);
        self.ui_canvases = json!([]);
        self.scene_save_manager
            .bootstrap_from_scene(&mut self.runtime_world.units, &self.tilemap_layers);
        self.sync_world();
        self.mark_scene_dirty("New Empty Scene");
        self.save_scene()?;
        self.console.log(
            format!(
                "Escena nueva abierta vacia: {}",
                self.scene_manager.current_scene
            ),
            "SCENE",
        );
        Ok(path)
    }

    pub fn load_scene(&mut self, name: &str) -> io::Result<usize> {
        let data = self.scene_manager.load_scene_data(name)?;
        let entities = self
            .scene_manager
            .load_scene(name, &self.runtime_world.units)?;
        self.runtime_world.replace_entities(entities);
        self.apply_scene_environment(&data);
        self.clear_selection();
        self.sync_world();
        self.scene_save_manager
            .bootstrap_from_scene(&mut self.runtime_world.units, &self.tilemap_layers);
        self.mark_scene_clean();
        self.console.log(
            format!(
                "Escena cargada: {} ({} entidades)",
                self.scene_manager.current_scene,
                self.runtime_world.units.len()
            ),
            "SCENE",
        );
        Ok(self.runtime_world.units.len())
    }

    pub fn scene_names(&self) -> io::Result<Vec<String>> {
        self.scene_manager.list_scenes()
    }

    pub fn load_next_scene(&mut self) -> io::Result<Option<String>> {
        let Some(scene_name) = self.scene_manager.next_scene()? else {
            return Ok(None);
        };
        self.load_scene(&scene_name)?;
        Ok(Some(scene_name))
    }

    pub fn load_scene_additive(&mut self, name: &str) -> io::Result<usize> {
        let added = self
            .scene_manager
            .load_scene_additive(name, &mut self.runtime_world.units)?;
        self.sync_world();
        self.mark_scene_dirty("Load Additive Scene");
        self.console.log(
            format!(
                "Escena aditiva cargada: {} (+{} entidades)",
                self.scene_manager.current_scene, added
            ),
            "SCENE",
        );
        Ok(added)
    }

    pub fn unload_scene(&mut self, name: &str) -> usize {
        let removed = self
            .scene_manager
            .unload_scene(name, &mut self.runtime_world.units);
        self.clear_selection();
        self.sync_world();
        self.mark_scene_dirty("Unload Scene");
        self.console.log(
            format!("Escena descargada: {name} (-{removed} entidades)"),
            "SCENE",
        );
        removed
    }

    pub fn restart_scene(&mut self) -> io::Result<usize> {
        let data = self
            .scene_manager
            .load_scene_data(&self.scene_manager.current_scene)?;
        let entities = self
            .scene_manager
            .restart_scene(&self.runtime_world.units)?;
        self.runtime_world.replace_entities(entities);
        self.apply_scene_environment(&data);
        self.clear_selection();
        self.sync_world();
        self.mark_scene_clean();
        self.console.log(
            format!("Escena reiniciada: {}", self.scene_manager.current_scene),
            "SCENE",
        );
        Ok(self.runtime_world.units.len())
    }

    pub fn push_scene(&mut self, name: &str) -> io::Result<usize> {
        let data = self.scene_manager.load_scene_data(name)?;
        let entities = self
            .scene_manager
            .push_scene(name, &self.runtime_world.units)?;
        self.runtime_world.replace_entities(entities);
        self.apply_scene_environment(&data);
        self.clear_selection();
        self.sync_world();
        self.mark_scene_clean();
        self.console.log(
            format!("Scene stack push: {}", self.scene_manager.current_scene),
            "SCENE",
        );
        Ok(self.runtime_world.units.len())
    }

    pub fn pop_scene(&mut self) -> io::Result<Option<usize>> {
        let Some(entities) = self.scene_manager.pop_scene(&self.runtime_world.units)? else {
            return Ok(None);
        };
        let data = self
            .scene_manager
            .load_scene_data(&self.scene_manager.current_scene)?;
        self.runtime_world.replace_entities(entities);
        self.apply_scene_environment(&data);
        self.clear_selection();
        self.sync_world();
        self.mark_scene_clean();
        self.console.log(
            format!("Scene stack pop: {}", self.scene_manager.current_scene),
            "SCENE",
        );
        Ok(Some(self.runtime_world.units.len()))
    }

    pub fn transition_to_scene(
        &mut self,
        name: &str,
        kind: &str,
        duration: f64,
    ) -> io::Result<usize> {
        let data = self.scene_manager.load_scene_data(name)?;
        let entities = self.scene_manager.transition_to_scene(
            name,
            kind,
            duration,
            &self.runtime_world.units,
        )?;
        self.runtime_world.replace_entities(entities);
        self.apply_scene_environment(&data);
        self.clear_selection();
        self.sync_world();
        self.mark_scene_clean();
        self.console.log(
            format!(
                "Transicion {kind}: {} -> {} ({duration:.2}s)",
                self.scene_manager
                    .transition
                    .as_ref()
                    .map(|transition| transition.from_scene.as_str())
                    .unwrap_or(""),
                self.scene_manager.current_scene
            ),
            "SCENE",
        );
        Ok(self.runtime_world.units.len())
    }

    pub fn save_project(&mut self) -> io::Result<()> {
        self.save_scene()?;
        self.engine_config.save()?;
        self.asset_database.scan()?;
        let manifest = self.build_manifest()?;
        let state = json!({
            "engine_version": crate::engine::version::ENGINE_VERSION,
            "project_path": self.project_path,
            "current_scene": self.scene_manager.current_scene,
            "mode": self.mode,
            "active_tool": self.active_tool,
            "workspace": self.editor_workspace.active_mode.label(),
            "asset_count": self.asset_database.assets.len(),
            "entity_count": self.runtime_world.units.len(),
            "scene_dirty": self.scene_dirty,
            "last_dirty_reason": self.scene_dirty_reason,
            "manifest": {
                "asset_entries": manifest.get("assets").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
                "scene_entries": manifest.get("scenes").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            }
        });
        AssetTools::write_json(
            self.project_path.join("project").join("project_state.json"),
            &state,
        )?;
        self.console.log(
            "Proyecto guardado: escena, assets, manifest y estado.",
            "PROJECT",
        );
        Ok(())
    }
}
