use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{Value, json};

use crate::core::engine_config::EngineConfig;
use crate::engine::advanced_prefabs::AdvancedPrefabSystem;
use crate::engine::animation_graph::AnimationGraphLibrary;
use crate::engine::asset_database::AssetDatabase;
use crate::engine::asset_tools::{AssetTools, ProjectPaths};
use crate::engine::audio_mixer::AudioMixer;
use crate::engine::autosave_manager::AutosaveManager;
use crate::engine::build_placement::{BuildFootprint, BuildPlacement};
use crate::engine::build_profiles::BuildProfiles;
use crate::engine::build_settings::BuildSettings;
use crate::engine::camera::Camera;
use crate::engine::component::default_component;
use crate::engine::component_registry::ComponentRegistry;
use crate::engine::developer_console::DeveloperConsole;
use crate::engine::diagnostics::Diagnostics;
use crate::engine::editor_history::EditorHistory;
use crate::engine::editor_workspace::{EditorWorkspace, WorkspaceMode};
use crate::engine::engine_programming::ProgrammingEnvironment;
use crate::engine::event_bus::EventBus;
use crate::engine::game_clock::GameClock;
use crate::engine::input_map::InputMap;
use crate::engine::manifest_builder::ManifestBuilder;
use crate::engine::play_mode_manager::PlayModeManager;
use crate::engine::prefab_manager::PrefabManager;
use crate::engine::profiler::Profiler;
use crate::engine::project_templates::ProjectTemplates;
use crate::engine::project_validator::ProjectValidator;
use crate::engine::resource_manager::ResourceManager;
use crate::engine::runtime_config::RuntimeConfig;
use crate::engine::scene_manager::SceneManager;
use crate::engine::scene_validator::SceneValidator;
use crate::engine::spatial_index::SpatialIndex;
use crate::engine::tags_layers_manager::TagsLayersManager;
use crate::engine::tilemap_layers::TilemapLayers;
use crate::engine::upgrade_manifest::EngineUpgradeManifest;
use crate::engine::visual_scripting::VisualScriptRuntime;
use crate::engine::world::World;
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;
use crate::map::pathfinding::Point;
use crate::systems::animation_system::AnimationSystem;
use crate::systems::gameplay_system::GameplaySystem;
use crate::systems::movement_system::MovementSystem;
use crate::systems::physics_system::PhysicsSystem;
use crate::systems::rts_system::RTSSystem;

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
    pub world: World,
    pub spatial_index: SpatialIndex,
    pub units: Vec<GameObject>,
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
    pub autosave_manager: AutosaveManager,
    pub history: EditorHistory,
    pub profiler: Profiler,
    pub diagnostics: Diagnostics,
    pub audio_mixer: AudioMixer,
    pub animation_graphs: AnimationGraphLibrary,
    pub visual_script_runtime: VisualScriptRuntime,
    pub play_mode_manager: PlayModeManager,
    pub gameplay_system: GameplaySystem,
    pub rts_system: RTSSystem,
    pub physics_system: PhysicsSystem,
    pub advanced_prefabs: AdvancedPrefabSystem,
    pub upgrade_manifest: EngineUpgradeManifest,
    pub editor_workspace: EditorWorkspace,
    pub programming: ProgrammingEnvironment,
}

impl Game {
    pub fn new(runtime_mode: bool) -> io::Result<Self> {
        Self::from_project(std::env::current_dir()?, runtime_mode)
    }

    pub fn from_project(project_path: impl AsRef<Path>, runtime_mode: bool) -> io::Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();
        let project_paths = AssetTools::ensure_project_folders(&project_path)?;
        let engine_config = EngineConfig::new(&project_path)?;
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
        let world = World {
            entities: units.clone(),
        };

        let mut console = DeveloperConsole::default();
        console.log(
            format!("Project path: {}", project_path.display()),
            "ENGINE",
        );
        console.log(crate::engine::version::version_label(), "ENGINE");

        let mut history = EditorHistory::default();
        history.take_snapshot("Initial Scene", &units);

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
            world,
            spatial_index: SpatialIndex::default(),
            units,
            selected_units: Vec::new(),
            grid,
            tilemap_layers,
            camera,
            clock: GameClock::default(),
            event_bus: EventBus::default(),
            resources,
            asset_database,
            input_map,
            build_settings,
            build_profiles,
            runtime_config,
            tags_layers_manager,
            component_registry: ComponentRegistry::new(),
            scene_manager: SceneManager::new(&project_path),
            scene_validator: SceneValidator::default(),
            autosave_manager: AutosaveManager::new(&project_path, 60),
            history,
            profiler: Profiler::new(),
            diagnostics: Diagnostics::default(),
            audio_mixer: AudioMixer::new(),
            animation_graphs: AnimationGraphLibrary::new(),
            visual_script_runtime: VisualScriptRuntime::default(),
            play_mode_manager: PlayModeManager::default(),
            gameplay_system: GameplaySystem::default(),
            rts_system: RTSSystem::default(),
            physics_system: PhysicsSystem::new(),
            advanced_prefabs: AdvancedPrefabSystem::default(),
            upgrade_manifest: EngineUpgradeManifest::new(),
            editor_workspace: EditorWorkspace::default(),
            programming: ProgrammingEnvironment::new(),
        };

        if let Ok(scene_data) = game.scene_manager.load_current_scene_data() {
            game.apply_scene_data(&scene_data);
        }
        game.sync_world();
        game.history.take_snapshot("Loaded Scene", &game.units);

        Ok(game)
    }

    fn apply_scene_data(&mut self, data: &Value) {
        if let Some(entities) = data.get("entities").and_then(Value::as_array) {
            self.units = entities
                .iter()
                .map(|entity| GameObject::from_data(entity, true))
                .collect();
            for entity in &mut self.units {
                entity.sync_from_components();
            }
            self.clear_selection();
            self.sync_world();
            self.console.log(
                format!("Escena cargada: {} entidades", self.units.len()),
                "SCENE",
            );
        }

        let tile_data = data
            .get("tilemap_layers")
            .or_else(|| data.get("tiles"))
            .filter(|value| value.is_object());
        if let Some(tile_data) = tile_data {
            self.tilemap_layers.deserialize(tile_data);
        }

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
    }

    pub fn run_headless_once(&mut self, dt: f64) {
        self.profiler.begin_frame();
        let clock_advance = self.clock.advance(dt);
        let mut marker = Instant::now();
        MovementSystem.update_entities(&mut self.units, dt);
        self.profiler
            .record_system("Movement", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        AnimationSystem.update_entities(&mut self.units, &self.animation_graphs, dt, &self.mode);
        self.profiler
            .record_system("Animation", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.visual_script_runtime
            .update_entities(&mut self.units, dt, &self.mode);
        self.profiler
            .record_system("VisualGraph", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.gameplay_system
            .update_entities(&mut self.units, dt, &self.mode);
        self.profiler
            .record_system("Gameplay", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.rts_system
            .update_entities(&mut self.units, dt, &self.mode);
        self.profiler
            .record_system("RTS", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.physics_system
            .update_entities_mut(&mut self.units, dt, &self.mode);
        self.profiler
            .record_system("Physics", marker.elapsed().as_secs_f64() * 1000.0);
        marker = Instant::now();
        self.world.entities = self.units.clone();
        self.spatial_index.rebuild(&self.units);
        self.profiler
            .record_system("WorldSync", marker.elapsed().as_secs_f64() * 1000.0);
        self.diagnostics.update(dt);
        self.profiler.set_counter("Entities", self.units.len());
        self.profiler.set_counter(
            "ActiveEntities",
            self.units
                .iter()
                .filter(|entity| entity.enabled && entity.visible)
                .count(),
        );
        self.profiler
            .set_counter("VisualGraphs", self.visual_script_runtime.last_frame_graphs);
        self.profiler
            .set_counter("VisualNodes", self.visual_script_runtime.executed_nodes);
        self.profiler
            .set_counter("FixedTicks", clock_advance.fixed_steps);
        self.profiler
            .set_counter("SpatialCells", self.spatial_index.cells.len());
        self.profiler.end_frame();
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

    pub fn get_entity_by_id(&self, entity_id: u64) -> Option<&GameObject> {
        self.units.iter().find(|entity| entity.id == entity_id)
    }

    pub fn get_entity_by_id_mut(&mut self, entity_id: u64) -> Option<&mut GameObject> {
        self.units.iter_mut().find(|entity| entity.id == entity_id)
    }

    pub fn clear_selection(&mut self) {
        for entity in &mut self.units {
            entity.selected = false;
        }
        self.selected_units.clear();
    }

    pub fn select_entity(&mut self, entity_id: u64) -> bool {
        self.clear_selection();
        let Some(entity) = self.get_entity_by_id_mut(entity_id) else {
            return false;
        };
        entity.set_selected(true);
        self.selected_units.push(entity_id);
        true
    }

    pub fn spawn_game_object(&mut self, name: &str, x: f64, y: f64) -> u64 {
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.width = 1.0;
        entity.height = 1.0;
        entity.sync_to_components();
        let id = entity.id;
        self.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn GameObject");
        id
    }

    pub fn spawn_unit(&mut self, name: &str, x: f64, y: f64) -> u64 {
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
        self.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Unit");
        id
    }

    pub fn spawn_enemy(&mut self, x: f64, y: f64) -> u64 {
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
        self.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Enemy");
        id
    }

    pub fn spawn_resource(&mut self, x: f64, y: f64) -> u64 {
        let mut entity = GameObject::new(x, y, Some("GoldNode".to_string()));
        entity.tag = "Resource".to_string();
        entity.width = 1.3;
        entity.height = 1.3;
        entity.add_component(default_component("ResourceNode").expect("ResourceNode"));
        entity.add_component(default_component("ObjectiveMarker").expect("ObjectiveMarker"));
        entity.sync_to_components();
        let id = entity.id;
        self.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Resource");
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
        self.units.push(entity);
        self.sync_world();
        self.mark_scene_dirty("Spawn RTS Controller");
        id
    }

    pub fn spawn_rts_worker(&mut self, name: &str, x: f64, y: f64, team_id: i64) -> u64 {
        let id = self.spawn_unit(name, x, y);
        if let Some(entity) = self.get_entity_by_id_mut(id) {
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
        self.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn RTS Building");
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
        self.units.push(entity);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Spawn Construction Site");
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
            &self.units,
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
        self.units.clear();
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
        for entity in &self.units {
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
        self.units.push(clone);
        self.select_entity(id);
        self.sync_world();
        self.mark_scene_dirty("Duplicate Entity");
        Some(id)
    }

    pub fn delete_entity(&mut self, entity_id: u64) -> bool {
        let before = self.units.len();
        self.units.retain(|entity| entity.id != entity_id);
        let deleted = self.units.len() != before;
        if deleted {
            self.selected_units
                .retain(|selected| *selected != entity_id);
            self.sync_world();
            self.mark_scene_dirty("Delete Entity");
        }
        deleted
    }

    pub fn add_component_to_entity(&mut self, entity_id: u64, component_type: &str) -> bool {
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
        }
        true
    }

    pub fn paint_tile(&mut self, x: usize, y: usize, value: i32) -> bool {
        if x >= self.tilemap_layers.width || y >= self.tilemap_layers.height {
            return false;
        }
        self.tilemap_layers.set_tile(x, y, value);
        self.mark_scene_dirty("Paint Tilemap");
        true
    }

    pub fn cycle_tilemap_layer(&mut self) -> String {
        let name = self.tilemap_layers.cycle_layer();
        self.console
            .log(format!("Capa tilemap activa: {name}"), "TILEMAP");
        name
    }

    pub fn refresh_assets(&mut self) -> io::Result<usize> {
        self.resources.scan_all()?;
        self.asset_database.scan()?;
        Ok(self.asset_database.assets.len())
    }

    pub fn validate_project(&mut self) -> bool {
        let mut validator = ProjectValidator::default();
        let valid = validator.validate_with_context(
            &self.project_path,
            &self.units,
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
        self.refresh_assets().ok();
        self.console
            .log(format!("Graph Rust creado: {}", path.display()), "SCRIPT");
        Ok(path)
    }

    pub fn attach_program_template_to_selected(&mut self, template_name: &str) -> Option<String> {
        let id = self.selected_units.first().copied()?;
        let index = self.units.iter().position(|entity| entity.id == id)?;
        let graph_name = {
            let programming = &mut self.programming;
            let entity = &mut self.units[index];
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
        let Some(index) = self.units.iter().position(|entity| entity.id == id) else {
            return Ok(None);
        };
        let dependencies = self
            .units
            .get(index)
            .and_then(|entity| entity.sprite_guid.clone())
            .into_iter()
            .collect::<Vec<_>>();
        let project_path = self.project_path.clone();
        let path = {
            let system = &mut self.advanced_prefabs;
            let entity = &mut self.units[index];
            system.create_prefab_from_entity(project_path, entity, false, dependencies)?
        };
        self.refresh_assets().ok();
        self.mark_scene_dirty("Save Prefab");
        self.console
            .log(format!("Prefab guardado: {}", path.display()), "PREFAB");
        Ok(Some(path))
    }

    pub fn create_selected_prefab_variant(&mut self) -> io::Result<Option<PathBuf>> {
        let Some(id) = self.selected_units.first().copied() else {
            return Ok(None);
        };
        let Some(index) = self.units.iter().position(|entity| entity.id == id) else {
            return Ok(None);
        };
        let project_path = self.project_path.clone();
        let path = {
            let system = &mut self.advanced_prefabs;
            let entity = &mut self.units[index];
            system.create_variant_from_entity(project_path, entity)?
        };
        self.refresh_assets().ok();
        self.console
            .log(format!("Variant creado: {}", path.display()), "PREFAB");
        Ok(Some(path))
    }

    pub fn instantiate_first_prefab(&mut self, x: f64, y: f64) -> io::Result<Option<u64>> {
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
        let id = manager.instantiate_prefab(&mut self.units, path, x, y)?;
        if let Some(id) = id {
            self.select_entity(id);
            self.sync_world();
            self.mark_scene_dirty("Instantiate Prefab");
            self.console.log(
                format!("Prefab instanciado: {} #{id}", prefab.name),
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

    pub fn legacy_python_asset_count(&self) -> usize {
        self.asset_database
            .assets
            .values()
            .filter(|asset| asset.asset_type == "LegacyScript")
            .count()
    }

    pub fn scene_summary(&self) -> String {
        let prefab_instances = self
            .units
            .iter()
            .filter(|entity| entity.is_prefab_instance)
            .count();
        let visual_graphs = self
            .units
            .iter()
            .filter(|entity| entity.get_component("VisualScript").is_some())
            .count();
        format!(
            "{} entities | {} prefab instances | {} visual graphs | {} assets | {} legacy py",
            self.units.len(),
            prefab_instances,
            visual_graphs,
            self.asset_database.assets.len(),
            self.legacy_python_asset_count()
        )
    }

    pub fn sync_world(&mut self) {
        self.world.entities = self.units.clone();
        self.spatial_index.rebuild(&self.units);
    }

    pub fn create_topdown_starter(&mut self) -> usize {
        self.units.clear();
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
            player.add_component(default_component("Light2D").expect("Light2D"));
            if let Some(body) = player.get_component_mut("Rigidbody2D") {
                body.set("use_gravity", json!(false));
                body.set_f64("drag", 0.2);
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
            if let Some(pickup_index) = self.units.iter().position(|entity| entity.id == pickup_id)
            {
                let pickup = &mut self.units[pickup_index];
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
        self.units.len()
    }

    pub fn create_platformer_starter(&mut self) -> usize {
        self.units.clear();
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
                controller.set_f64("walk_speed", 5.5);
                controller.set_f64("jump_force", 10.0);
                controller.set("max_jumps", json!(2));
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
        self.units.len()
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
        self.units.push(entity);
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
        self.units.push(entity);
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
        self.units.push(entity);
        self.sync_world();
        self.mark_scene_dirty("Create UI ProgressBar");
        id
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
        if self.scene_validator.validate_entities(&self.units) {
            self.scene_manager.save_current_scene_with_editor_state(
                &mut self.units,
                &self.tilemap_layers,
                &self.camera,
                &self.mode,
                &self.active_tool,
                self.tile_brush,
                self.brush_size,
                &self.grid,
            )?;
            self.mark_scene_clean();
        }
        Ok(())
    }
}
