//! Luau gameplay runtime for MiniForge.
//!
//! The runtime deliberately exposes a small command API instead of direct Rust
//! references. Scripts enqueue commands and the engine applies them after the
//! callback returns, keeping entity storage and the Luau VM separated.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use mlua::chunk::Compiler;
use mlua::luau::{NavigateError, Require};
use mlua::{Function, Lua, Table, Value as LuaValue, VmState};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::{Value, json};

use crate::engine::camera::Camera;
use crate::engine::entity_id::generate_entity_id;
use crate::engine::game_api::GameAPI;
use crate::engine::spatial_index::{SpatialEntry, SpatialIndex};
use crate::entities::game_object::GameObject;
use crate::systems::physics_system::{
    BoxCastQuery, CircleCastQuery, PhysicsQueryFilter, PhysicsSystem, RaycastHit,
};

const SCRIPT_INTERRUPT_BUDGET: u64 = 20_000;
const LUA_MEMORY_LIMIT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptTarget {
    Id(u64),
    Name(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptCommand {
    Move {
        entity_id: u64,
        dx: f64,
        dy: f64,
    },
    SetPosition {
        entity_id: u64,
        x: f64,
        y: f64,
    },
    Spawn {
        name: String,
        x: f64,
        y: f64,
    },
    SpawnConfigured {
        name: String,
        x: f64,
        y: f64,
        data: Value,
    },
    SpawnWithId {
        entity_id: u64,
        name: String,
        x: f64,
        y: f64,
        data: Option<Value>,
    },
    Destroy {
        target: ScriptTarget,
    },
    PlaySound {
        name: String,
        bus: String,
        volume: f64,
        looped: bool,
    },
    LoadScene {
        name: String,
    },
    SetUiText {
        target: ScriptTarget,
        text: String,
    },
    SetUiProgress {
        target: ScriptTarget,
        value: f64,
        max: f64,
    },
    SetUiVisible {
        target: ScriptTarget,
        visible: bool,
    },
    SetTag {
        target: ScriptTarget,
        tag: String,
    },
    SetLayer {
        target: ScriptTarget,
        layer: String,
    },
    SetEnabled {
        target: ScriptTarget,
        enabled: bool,
    },
    SetVisible {
        target: ScriptTarget,
        visible: bool,
    },
    SetComponentNumber {
        target: ScriptTarget,
        component: String,
        key: String,
        value: f64,
    },
    SetComponentText {
        target: ScriptTarget,
        component: String,
        key: String,
        value: String,
    },
    AddComponent {
        target: ScriptTarget,
        component: String,
        data: Value,
    },
    RemoveComponent {
        target: ScriptTarget,
        component: String,
    },
    SetComponentValue {
        target: ScriptTarget,
        component: String,
        key: String,
        value: Value,
    },
    SetVelocity {
        target: ScriptTarget,
        x: f64,
        y: f64,
    },
    ApplyImpulse {
        target: ScriptTarget,
        x: f64,
        y: f64,
    },
    ApplyForce {
        target: ScriptTarget,
        x: f64,
        y: f64,
    },
    ApplyTorque {
        target: ScriptTarget,
        torque: f64,
    },
    WakeBody {
        target: ScriptTarget,
    },
    SleepBody {
        target: ScriptTarget,
    },
    SetCharacterInput {
        target: ScriptTarget,
        x: f64,
        y: f64,
        jump: bool,
        run: bool,
    },
    SetCameraFollow {
        target: ScriptTarget,
        follow_target_id: u64,
    },
    SetCameraShake {
        target: ScriptTarget,
        duration: f64,
        amplitude: f64,
    },
    SetCameraPixelPerfect {
        target: ScriptTarget,
        enabled: bool,
        pixels_per_unit: f64,
    },
    SetAnimation {
        target: ScriptTarget,
        animation: String,
    },
    SetAnimationParameter {
        target: ScriptTarget,
        key: String,
        value: Value,
    },
    SetTile {
        target: ScriptTarget,
        layer: String,
        x: usize,
        y: usize,
        tile: i64,
    },
    SetTween {
        target: ScriptTarget,
        property_path: String,
        to_value: f64,
        duration: f64,
        easing: String,
    },
    SetNavDestination {
        target: ScriptTarget,
        x: f64,
        y: f64,
    },
    ParticleBurst {
        target: ScriptTarget,
        count: i64,
    },
    SetSprite {
        target: ScriptTarget,
        sprite_path: String,
    },
    SetSpriteAnimation {
        target: ScriptTarget,
        frames_path: String,
        animation: String,
    },
    SetSpriteFlip {
        target: ScriptTarget,
        flip_x: bool,
        flip_y: bool,
    },
    AddItem {
        target: ScriptTarget,
        item_id: String,
        quantity: i64,
    },
    AddResource {
        target: ScriptTarget,
        resource_type: String,
        amount: f64,
    },
    SetBlackboard {
        target: ScriptTarget,
        key: String,
        value: Value,
    },
    AddQuest {
        target: ScriptTarget,
        quest_id: String,
        title: String,
        objectives: Value,
    },
    QuestProgress {
        target: ScriptTarget,
        quest_id: String,
        objective_id: String,
        progress: Value,
    },
    CompleteQuest {
        target: ScriptTarget,
        quest_id: String,
    },
    SaveGame {
        slot: String,
    },
    LoadGame {
        slot: String,
    },
    TriggerAbility {
        target: ScriptTarget,
        now: f64,
    },
    RechargeAbility {
        target: ScriptTarget,
        amount: i64,
    },
    EmitEvent {
        name: String,
        payload: Value,
    },
    DebugLog {
        level: String,
        message: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LuauRunReport {
    pub scripts_run: usize,
    pub commands_applied: usize,
    pub spawned: Vec<u64>,
    pub destroyed: Vec<u64>,
    pub sounds: Vec<String>,
    pub scene_requests: Vec<String>,
    pub ui_updates: usize,
    pub errors: Vec<String>,
    pub debug_messages: Vec<String>,
}

impl LuauRunReport {
    pub fn merge(&mut self, other: Self) {
        self.scripts_run += other.scripts_run;
        self.commands_applied += other.commands_applied;
        self.spawned.extend(other.spawned);
        self.destroyed.extend(other.destroyed);
        self.sounds.extend(other.sounds);
        self.scene_requests.extend(other.scene_requests);
        self.ui_updates += other.ui_updates;
        self.errors.extend(other.errors);
        self.debug_messages.extend(other.debug_messages);
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScriptTraceEntry {
    pub path: PathBuf,
    pub line: usize,
    pub function: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScriptDebugSnapshot {
    pub watcher_active: bool,
    pub reload_count: usize,
    pub active_scripts: Vec<String>,
    pub errors: Vec<String>,
    pub traces: Vec<ScriptTraceEntry>,
    #[serde(default)]
    pub cached_scripts: usize,
    #[serde(default)]
    pub persistent_contexts: usize,
    #[serde(default)]
    pub last_frame_scripts: usize,
    #[serde(default)]
    pub memory_bytes: usize,
    #[serde(default)]
    pub scheduler: ScriptSchedulerFrameStats,
    #[serde(default)]
    pub queries: ScriptQueryFrameStats,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LuauSourceDiagnostic {
    pub source: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub message: String,
    pub incomplete_input: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScriptBreakpoint {
    pub path: String,
    pub line: Option<usize>,
    pub function: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScriptPausedFrame {
    pub entity_id: u64,
    pub entity_name: String,
    pub path: String,
    pub function: String,
    pub line: Option<usize>,
    pub event: String,
    pub context: Value,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScriptWatchResult {
    pub expression: String,
    pub value: Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScriptDebuggerState {
    pub paused: bool,
    pub pause_requested: bool,
    pub step_pending: bool,
    pub frame: Option<ScriptPausedFrame>,
    pub breakpoints: Vec<ScriptBreakpoint>,
}

fn default_true() -> bool {
    true
}

impl LuauSourceDiagnostic {
    pub fn display_message(&self) -> String {
        match (self.line, self.column) {
            (Some(line), Some(column)) => {
                format!("{}:{line}:{column}: {}", self.source, self.message)
            }
            (Some(line), None) => format!("{}:{line}: {}", self.source, self.message),
            _ => format!("{}: {}", self.source, self.message),
        }
    }
}

#[derive(Debug, Clone)]
enum LuauScriptEvent {
    Create,
    Ready,
    Update(f64),
    FixedUpdate(f64),
    KeyDown(String),
    CollisionEnter(String),
    CollisionExit(String),
    Destroy,
    Custom { name: String, payload: Value },
}

impl LuauScriptEvent {
    fn function_names(&self) -> &'static [&'static str] {
        match self {
            Self::Create => &["on_create", "on_start"],
            Self::Ready => &["on_ready"],
            Self::Update(_) => &["on_update"],
            Self::FixedUpdate(_) => &["on_fixed_update"],
            Self::KeyDown(_) => &["on_key_down"],
            Self::CollisionEnter(_) => &["on_collision_enter"],
            Self::CollisionExit(_) => &["on_collision_exit"],
            Self::Destroy => &["on_destroy"],
            Self::Custom { .. } => &["on_event"],
        }
    }
}

#[derive(Clone)]
struct CachedLuauScript {
    bytecode: Vec<u8>,
    modified: Option<SystemTime>,
    handlers: BTreeSet<String>,
}

#[derive(Clone)]
struct LuauScriptContext {
    environment: Table,
    instance: Table,
    modified: Option<SystemTime>,
    method_style: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DebugCallbackKey {
    entity_id: u64,
    path: PathBuf,
    function: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptSchedulerConfig {
    pub enabled: bool,
    pub max_update_scripts_per_frame: usize,
    pub default_update_interval: f64,
    pub distant_update_interval: f64,
    pub budget_bypass_priority: i64,
    pub prioritize_by_distance: bool,
    pub open_world_auto_policy: bool,
}

impl Default for ScriptSchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_update_scripts_per_frame: usize::MAX,
            default_update_interval: 0.0,
            distant_update_interval: 0.75,
            budget_bypass_priority: 100,
            prioritize_by_distance: true,
            open_world_auto_policy: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ScriptUpdateState {
    next_update_time: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScriptSchedulerFrameStats {
    pub update_candidates: usize,
    pub update_budget_used: usize,
    pub skipped_disabled: usize,
    pub skipped_budget: usize,
    pub skipped_interval: usize,
    pub distance_throttled: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScriptQueryFrameStats {
    pub nearby_queries: usize,
    pub nearby_indexed: usize,
    pub nearby_linear_scans: usize,
    pub nearby_candidates: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ScriptSimulationClass {
    Critical,
    Police,
    Vehicle,
    Pedestrian,
    Pickup,
    Background,
    #[default]
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScriptUpdatePolicy {
    enabled: bool,
    always_update: bool,
    update_interval: f64,
    max_distance: Option<f64>,
    distant_update_interval: Option<f64>,
    priority: i64,
    simulation_class: ScriptSimulationClass,
}

impl Default for ScriptUpdatePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            always_update: false,
            update_interval: 0.0,
            max_distance: None,
            distant_update_interval: None,
            priority: 0,
            simulation_class: ScriptSimulationClass::Default,
        }
    }
}

#[derive(Debug, Default)]
struct ScriptHostState {
    current_entity_id: Option<u64>,
    commands: Vec<ScriptCommand>,
    inputs_pressed: BTreeSet<String>,
    time: ScriptTimeState,
    world_entities: Vec<GameObject>,
    world_entity_ids: BTreeMap<u64, usize>,
    world_entity_names: BTreeMap<String, usize>,
    world_entity_tags: BTreeMap<String, Vec<usize>>,
    spatial_index: SpatialIndex,
    query_stats: ScriptQueryFrameStats,
    camera: ScriptCameraSnapshot,
}

impl ScriptHostState {
    fn replace_world_entities(&mut self, entities: &[GameObject]) {
        self.world_entities = entities.to_vec();
        self.world_entity_ids.clear();
        self.world_entity_names.clear();
        self.world_entity_tags.clear();
        for (index, entity) in self.world_entities.iter().enumerate() {
            self.world_entity_ids.entry(entity.id).or_insert(index);
            self.world_entity_names
                .entry(entity.name.clone())
                .or_insert(index);
            self.world_entity_tags
                .entry(entity.tag.clone())
                .or_default()
                .push(index);
        }
        self.spatial_index.rebuild(&self.world_entities);
    }

    fn reset_query_stats(&mut self) {
        self.query_stats = ScriptQueryFrameStats::default();
    }
}

type SharedHostState = Arc<Mutex<ScriptHostState>>;

#[derive(Debug, Clone, Default)]
struct ScriptTimeState {
    delta_time: f64,
    fixed_delta_time: f64,
    total_time: f64,
    frame: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct ScriptCameraSnapshot {
    x: f64,
    y: f64,
    zoom: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    viewport: (f64, f64, f64, f64),
}

impl Default for ScriptCameraSnapshot {
    fn default() -> Self {
        Self::from_camera(&Camera::default())
    }
}

impl ScriptCameraSnapshot {
    fn from_camera(camera: &Camera) -> Self {
        Self {
            x: camera.x,
            y: camera.y,
            zoom: camera.zoom,
            min_x: camera.min_x,
            min_y: camera.min_y,
            max_x: camera.max_x,
            max_y: camera.max_y,
            viewport: camera.viewport,
        }
    }
}

pub struct LuauScriptRuntime {
    lua: Lua,
    host: SharedHostState,
    interrupt_budget: Arc<AtomicU64>,
    project_path: PathBuf,
    cache: BTreeMap<PathBuf, CachedLuauScript>,
    contexts: BTreeMap<(u64, PathBuf), LuauScriptContext>,
    created_scripts: BTreeSet<(u64, PathBuf)>,
    ready_scripts: BTreeSet<(u64, PathBuf)>,
    destroying_entities: BTreeSet<u64>,
    update_states: BTreeMap<(u64, PathBuf), ScriptUpdateState>,
    scheduler_config: ScriptSchedulerConfig,
    changed_paths: Arc<Mutex<BTreeSet<PathBuf>>>,
    watcher: Option<RecommendedWatcher>,
    pub watcher_active: bool,
    pub reload_count: usize,
    pub last_frame_scripts: usize,
    pub last_scheduler_stats: ScriptSchedulerFrameStats,
    pub last_query_stats: ScriptQueryFrameStats,
    pub last_errors: Vec<String>,
    debug_breakpoints: Vec<ScriptBreakpoint>,
    debug_paused: Option<ScriptPausedFrame>,
    debug_pause_requested: bool,
    debug_step_after_callback: bool,
    debug_skip_once: Option<DebugCallbackKey>,
}

impl std::fmt::Debug for LuauScriptRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuauScriptRuntime")
            .field("project_path", &self.project_path)
            .field("cache_len", &self.cache.len())
            .field("context_len", &self.contexts.len())
            .field("created_scripts_len", &self.created_scripts.len())
            .field("ready_scripts_len", &self.ready_scripts.len())
            .field("watcher_active", &self.watcher_active)
            .field("reload_count", &self.reload_count)
            .field("last_frame_scripts", &self.last_frame_scripts)
            .field("last_scheduler_stats", &self.last_scheduler_stats)
            .field("last_query_stats", &self.last_query_stats)
            .field("last_errors", &self.last_errors)
            .field("debug_breakpoints", &self.debug_breakpoints.len())
            .field("debug_paused", &self.debug_paused)
            .finish()
    }
}

impl Default for LuauScriptRuntime {
    fn default() -> Self {
        Self::new(PathBuf::new())
    }
}

impl LuauScriptRuntime {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        let host = Arc::new(Mutex::new(ScriptHostState::default()));
        let project_path = project_path.as_ref().to_path_buf();
        let interrupt_budget = Arc::new(AtomicU64::new(SCRIPT_INTERRUPT_BUDGET));
        let lua = build_luau_vm(host.clone(), interrupt_budget.clone(), project_path.clone())
            .expect("MiniForge Luau API must initialize");
        let mut runtime = Self {
            lua,
            host,
            interrupt_budget,
            project_path,
            cache: BTreeMap::new(),
            contexts: BTreeMap::new(),
            created_scripts: BTreeSet::new(),
            ready_scripts: BTreeSet::new(),
            destroying_entities: BTreeSet::new(),
            update_states: BTreeMap::new(),
            scheduler_config: ScriptSchedulerConfig::default(),
            changed_paths: Arc::new(Mutex::new(BTreeSet::new())),
            watcher: None,
            watcher_active: false,
            reload_count: 0,
            last_frame_scripts: 0,
            last_scheduler_stats: ScriptSchedulerFrameStats::default(),
            last_query_stats: ScriptQueryFrameStats::default(),
            last_errors: Vec::new(),
            debug_breakpoints: Vec::new(),
            debug_paused: None,
            debug_pause_requested: false,
            debug_step_after_callback: false,
            debug_skip_once: None,
        };
        let _ = runtime.watch_project();
        runtime
    }

    pub fn set_project_path(&mut self, project_path: impl AsRef<Path>) {
        self.project_path = project_path.as_ref().to_path_buf();
        self.cache.clear();
        self.contexts.clear();
        self.created_scripts.clear();
        self.ready_scripts.clear();
        self.update_states.clear();
        self.debug_paused = None;
        self.debug_skip_once = None;
        self.rebuild_vm();
        let _ = self.watch_project();
    }

    fn rebuild_vm(&mut self) {
        match build_luau_vm(
            self.host.clone(),
            self.interrupt_budget.clone(),
            self.project_path.clone(),
        ) {
            Ok(lua) => self.lua = lua,
            Err(error) => self.last_errors.push(format!("Luau VM init: {error}")),
        }
    }

    pub fn watch_project(&mut self) -> io::Result<()> {
        self.watcher = None;
        self.watcher_active = false;
        if self.project_path.as_os_str().is_empty() {
            return Ok(());
        }
        let scripts = self.project_path.join("scripts");
        fs::create_dir_all(&scripts)?;
        let changed_paths = self.changed_paths.clone();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let Ok(event) = result else { return };
            if !is_reload_event(&event.kind) {
                return;
            }
            if let Ok(mut changed) = changed_paths.lock() {
                for path in event.paths {
                    if is_luau_path(&path) {
                        changed.insert(normalize_path(path));
                    }
                }
            }
        })
        .map_err(io::Error::other)?;
        watcher
            .watch(&scripts, RecursiveMode::Recursive)
            .map_err(io::Error::other)?;
        self.watcher = Some(watcher);
        self.watcher_active = true;
        Ok(())
    }

    pub fn mark_script_changed(&mut self, path: impl AsRef<Path>) {
        if is_luau_path(path.as_ref())
            && let Ok(mut changed) = self.changed_paths.lock()
        {
            changed.insert(normalize_path(path));
        }
    }

    pub fn poll_hot_reload(&mut self) -> usize {
        let changed = {
            let Ok(mut changed) = self.changed_paths.lock() else {
                return 0;
            };
            std::mem::take(&mut *changed)
        };
        let mut reloaded = 0;
        for path in changed.into_iter().filter(|path| is_luau_path(path)) {
            let path = normalize_path(path);
            self.cache.remove(&path);
            self.contexts
                .retain(|(_, context_path), _| context_path != &path);
            self.created_scripts
                .retain(|(_, context_path)| context_path != &path);
            self.ready_scripts
                .retain(|(_, context_path)| context_path != &path);
            reloaded += 1;
        }
        self.reload_count += reloaded;
        if reloaded > 0 {
            self.cache.clear();
            self.contexts.clear();
            self.created_scripts.clear();
            self.ready_scripts.clear();
            self.update_states.clear();
            self.debug_paused = None;
            self.debug_skip_once = None;
            self.rebuild_vm();
        }
        reloaded
    }

    pub fn set_scheduler_config(&mut self, config: ScriptSchedulerConfig) {
        self.scheduler_config = config;
    }

    pub fn set_input_pressed(&mut self, key: &str, pressed: bool) {
        if let Ok(mut host) = self.host.lock() {
            if pressed {
                host.inputs_pressed.insert(key.to_string());
            } else {
                host.inputs_pressed.remove(key);
            }
        }
    }

    pub fn clear_input(&mut self) {
        if let Ok(mut host) = self.host.lock() {
            host.inputs_pressed.clear();
        }
    }

    pub fn set_camera_state(&mut self, camera: &Camera) {
        if let Ok(mut host) = self.host.lock() {
            host.camera = ScriptCameraSnapshot::from_camera(camera);
        }
    }

    pub fn input_pressed(&self, key: &str) -> bool {
        self.host
            .lock()
            .map(|host| host.inputs_pressed.contains(key))
            .unwrap_or(false)
    }

    fn advance_time(&mut self, dt: f64, fixed_dt: f64) {
        if let Ok(mut host) = self.host.lock() {
            host.time.delta_time = dt.max(0.0);
            host.time.fixed_delta_time = fixed_dt.max(0.0);
            host.time.total_time += dt.max(0.0);
            host.time.frame = host.time.frame.saturating_add(1);
        }
        self.sync_time_globals();
    }

    fn sync_time_globals(&self) {
        let Ok(host) = self.host.lock() else {
            return;
        };
        let globals = self.lua.globals();
        if let Ok(time) = globals.get::<Table>("Time") {
            let _ = time.set("delta_time", host.time.delta_time);
            let _ = time.set("fixed_delta_time", host.time.fixed_delta_time);
            let _ = time.set("time", host.time.total_time);
            let _ = time.set("frame", host.time.frame);
        }
    }

    fn host_time(&self) -> ScriptTimeState {
        self.host
            .lock()
            .map(|host| host.time.clone())
            .unwrap_or_default()
    }

    pub fn reload_all(&mut self) -> usize {
        let count = self.cache.len();
        self.cache.clear();
        self.contexts.clear();
        self.created_scripts.clear();
        self.ready_scripts.clear();
        self.update_states.clear();
        self.debug_paused = None;
        self.debug_skip_once = None;
        self.rebuild_vm();
        self.reload_count += count.max(1);
        count
    }

    pub fn active_scripts(&self, entities: &[GameObject]) -> Vec<String> {
        let mut scripts = self
            .collect_script_calls(entities)
            .into_iter()
            .flat_map(|call| call.scripts.into_iter().map(|script| script.path))
            .map(|path| {
                path.strip_prefix(&self.project_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        scripts.sort();
        scripts.dedup();
        scripts
    }

    pub fn debug_snapshot(
        &self,
        project_path: impl AsRef<Path>,
        entities: &[GameObject],
    ) -> ScriptDebugSnapshot {
        let active_scripts = self.active_scripts(entities);
        let traces = active_scripts
            .iter()
            .flat_map(|script| trace_script_lines(&project_path.as_ref().join(script)))
            .collect();
        ScriptDebugSnapshot {
            watcher_active: self.watcher_active,
            reload_count: self.reload_count,
            active_scripts,
            errors: self.last_errors.clone(),
            traces,
            cached_scripts: self.cache.len(),
            persistent_contexts: self.contexts.len(),
            last_frame_scripts: self.last_frame_scripts,
            memory_bytes: self.lua.used_memory(),
            scheduler: self.last_scheduler_stats,
            queries: self.last_query_stats,
        }
    }

    pub fn set_debug_breakpoints(&mut self, breakpoints: Vec<ScriptBreakpoint>) {
        self.debug_breakpoints = breakpoints
            .into_iter()
            .filter(|breakpoint| {
                !breakpoint.path.trim().is_empty()
                    && (breakpoint.line.is_some() || breakpoint.function.is_some())
            })
            .collect();
    }

    pub fn debugger_state(&self) -> ScriptDebuggerState {
        ScriptDebuggerState {
            paused: self.debug_paused.is_some(),
            pause_requested: self.debug_pause_requested,
            step_pending: self.debug_step_after_callback,
            frame: self.debug_paused.clone(),
            breakpoints: self.debug_breakpoints.clone(),
        }
    }

    pub fn request_debug_pause(&mut self) {
        self.debug_pause_requested = true;
    }

    pub fn resume_debugger(&mut self) -> bool {
        let Some(frame) = self.debug_paused.take() else {
            self.debug_pause_requested = false;
            return false;
        };
        self.debug_skip_once = Some(DebugCallbackKey {
            entity_id: frame.entity_id,
            path: normalize_path(if Path::new(&frame.path).is_absolute() {
                PathBuf::from(&frame.path)
            } else {
                self.project_path.join(&frame.path)
            }),
            function: frame.function,
        });
        self.debug_pause_requested = false;
        self.debug_step_after_callback = false;
        true
    }

    pub fn step_debugger(&mut self) -> bool {
        if !self.resume_debugger() {
            return false;
        }
        self.debug_step_after_callback = true;
        true
    }

    pub fn evaluate_debug_watches(&self, expressions: &[String]) -> Vec<ScriptWatchResult> {
        expressions
            .iter()
            .map(|expression| {
                let expression = expression.trim();
                if expression.is_empty()
                    || !expression
                        .split('.')
                        .all(|part| !part.is_empty() && is_luau_debug_identifier(part))
                {
                    return ScriptWatchResult {
                        expression: expression.to_string(),
                        value: Value::Null,
                        error: Some(
                            "watch expressions are limited to dotted identifiers".to_string(),
                        ),
                    };
                }
                let Some(frame) = self.debug_paused.as_ref() else {
                    return ScriptWatchResult {
                        expression: expression.to_string(),
                        value: Value::Null,
                        error: Some("runtime is not paused".to_string()),
                    };
                };
                let mut value = &frame.context;
                for part in expression.split('.') {
                    let Some(next) = value.get(part) else {
                        return ScriptWatchResult {
                            expression: expression.to_string(),
                            value: Value::Null,
                            error: Some(format!("`{part}` is not available in this frame")),
                        };
                    };
                    value = next;
                }
                ScriptWatchResult {
                    expression: expression.to_string(),
                    value: value.clone(),
                    error: None,
                }
            })
            .collect()
    }

    pub fn update_entities(
        &mut self,
        entities: &mut Vec<GameObject>,
        dt: f64,
        mode: &str,
    ) -> LuauRunReport {
        self.update_entities_with_fixed_steps(entities, dt, dt, 1, mode)
    }

    pub fn update_entities_with_fixed_steps(
        &mut self,
        entities: &mut Vec<GameObject>,
        dt: f64,
        fixed_dt: f64,
        fixed_steps: usize,
        mode: &str,
    ) -> LuauRunReport {
        self.poll_hot_reload();
        self.last_frame_scripts = 0;
        self.last_scheduler_stats = ScriptSchedulerFrameStats::default();
        self.last_query_stats = ScriptQueryFrameStats::default();
        self.last_errors.clear();
        self.reset_query_stats();
        if mode != "PLAY" {
            return LuauRunReport::default();
        }
        self.advance_time(dt, fixed_dt);
        self.retain_live_entities(entities);
        let mut report = self.run_start_events(entities);
        for _ in 0..fixed_steps {
            report.merge(self.run_event_for_all(entities, LuauScriptEvent::FixedUpdate(fixed_dt)));
        }
        report.merge(self.run_event_for_all(entities, LuauScriptEvent::Update(dt)));
        report.errors = self.last_errors.clone();
        self.sync_query_stats();
        report
    }

    pub fn run_start_events(&mut self, entities: &mut Vec<GameObject>) -> LuauRunReport {
        let calls = self.collect_script_calls(entities);
        let mut report = LuauRunReport::default();
        self.refresh_world_snapshot(entities);
        for call in calls {
            for script in &call.scripts {
                let key = (call.entity_id, script.path.clone());
                if self.ready_scripts.contains(&key) {
                    continue;
                }
                let Some(cached_script) = self.load_script(&script.path) else {
                    continue;
                };
                if !self.created_scripts.contains(&key)
                    && cached_script.supports_event(&LuauScriptEvent::Create)
                {
                    self.call_script_event(
                        entities,
                        &call,
                        script,
                        cached_script.clone(),
                        &LuauScriptEvent::Create,
                        &mut report,
                    );
                    if self.debug_paused.is_some() {
                        report.errors = self.last_errors.clone();
                        self.sync_query_stats();
                        return report;
                    }
                    self.created_scripts.insert(key.clone());
                    if self.apply_pending_commands(entities, &mut report) {
                        self.refresh_world_snapshot(entities);
                    }
                }
                if cached_script.supports_event(&LuauScriptEvent::Ready) {
                    self.call_script_event(
                        entities,
                        &call,
                        script,
                        cached_script,
                        &LuauScriptEvent::Ready,
                        &mut report,
                    );
                    if self.debug_paused.is_some() {
                        report.errors = self.last_errors.clone();
                        self.sync_query_stats();
                        return report;
                    }
                    if self.apply_pending_commands(entities, &mut report) {
                        self.refresh_world_snapshot(entities);
                    }
                }
                self.ready_scripts.insert(key);
            }
        }
        report.errors = self.last_errors.clone();
        self.sync_query_stats();
        report
    }

    pub fn run_key_down(
        &mut self,
        entities: &mut Vec<GameObject>,
        key: impl Into<String>,
    ) -> LuauRunReport {
        let key = key.into();
        self.set_input_pressed(&key, true);
        self.run_event_for_all(entities, LuauScriptEvent::KeyDown(key))
    }

    pub fn run_collision_enter(
        &mut self,
        entities: &mut Vec<GameObject>,
        entity_id: u64,
        other_name: impl Into<String>,
    ) -> LuauRunReport {
        self.run_event_for_ids(
            entities,
            &[entity_id],
            LuauScriptEvent::CollisionEnter(other_name.into()),
        )
    }

    pub fn run_collision_exit(
        &mut self,
        entities: &mut Vec<GameObject>,
        entity_id: u64,
        other_name: impl Into<String>,
    ) -> LuauRunReport {
        self.run_event_for_ids(
            entities,
            &[entity_id],
            LuauScriptEvent::CollisionExit(other_name.into()),
        )
    }

    /// Sends a structured gameplay event to every active Luau script.
    ///
    /// Scripts receive it through `on_event(name, payload)`. JSON objects and
    /// arrays are converted recursively into Luau tables.
    pub fn run_custom_event(
        &mut self,
        entities: &mut Vec<GameObject>,
        name: impl Into<String>,
        payload: Value,
    ) -> LuauRunReport {
        self.run_event_for_all(
            entities,
            LuauScriptEvent::Custom {
                name: name.into(),
                payload,
            },
        )
    }

    /// Sends a structured gameplay event to one entity and all Luau scripts
    /// attached to it.
    pub fn run_custom_event_for_entity(
        &mut self,
        entities: &mut Vec<GameObject>,
        entity_id: u64,
        name: impl Into<String>,
        payload: Value,
    ) -> LuauRunReport {
        self.run_event_for_ids(
            entities,
            &[entity_id],
            LuauScriptEvent::Custom {
                name: name.into(),
                payload,
            },
        )
    }

    pub fn run_destroy(&mut self, entities: &mut Vec<GameObject>, entity_id: u64) -> LuauRunReport {
        if self.destroying_entities.contains(&entity_id) {
            return LuauRunReport::default();
        }
        self.destroying_entities.insert(entity_id);
        let mut report = self.run_event_for_ids(entities, &[entity_id], LuauScriptEvent::Destroy);
        if GameAPI::destroy(entities, entity_id) {
            report.destroyed.push(entity_id);
            report.commands_applied += 1;
        }
        self.ready_scripts
            .retain(|(ready_entity_id, _)| *ready_entity_id != entity_id);
        self.created_scripts
            .retain(|(created_entity_id, _)| *created_entity_id != entity_id);
        self.destroying_entities.remove(&entity_id);
        report.errors = self.last_errors.clone();
        report
    }

    pub fn drain_commands(&mut self) -> Vec<ScriptCommand> {
        self.host
            .lock()
            .map(|mut host| std::mem::take(&mut host.commands))
            .unwrap_or_default()
    }

    pub fn validate_source(source: &str, name: &str) -> Result<(), String> {
        match Self::validate_source_diagnostics(source, name)
            .into_iter()
            .next()
        {
            Some(diagnostic) => Err(diagnostic.display_message()),
            None => Ok(()),
        }
    }

    pub fn validate_source_diagnostics(source: &str, name: &str) -> Vec<LuauSourceDiagnostic> {
        let bytecode = match luau_compiler().compile(source) {
            Ok(bytecode) => bytecode,
            Err(error) => return vec![luau_source_diagnostic(name, error)],
        };
        let lua = Lua::new();
        match lua.load(&bytecode).set_name(name).into_function() {
            Ok(_) => Vec::new(),
            Err(error) => vec![luau_source_diagnostic(name, error)],
        }
    }

    fn run_event_for_all(
        &mut self,
        entities: &mut Vec<GameObject>,
        event: LuauScriptEvent,
    ) -> LuauRunReport {
        let ids = self
            .collect_script_calls(entities)
            .into_iter()
            .map(|call| call.entity_id)
            .collect::<Vec<_>>();
        self.run_event_for_ids(entities, &ids, event)
    }

    fn run_event_for_ids(
        &mut self,
        entities: &mut Vec<GameObject>,
        ids: &[u64],
        event: LuauScriptEvent,
    ) -> LuauRunReport {
        if self.debug_paused.is_some() {
            return LuauRunReport::default();
        }
        let ids = ids.iter().copied().collect::<BTreeSet<_>>();
        let mut calls = self
            .collect_script_calls(entities)
            .into_iter()
            .filter(|call| ids.contains(&call.entity_id))
            .collect::<Vec<_>>();
        let focus = script_focus_point(entities).or_else(|| self.camera_focus_point());
        if matches!(event, LuauScriptEvent::Update(_)) {
            self.last_scheduler_stats.update_candidates = calls.len();
            calls.sort_by(|a, b| compare_update_calls(a, b, focus, self.scheduler_config));
        }
        let mut report = LuauRunReport::default();
        self.refresh_world_snapshot(entities);
        let mut update_budget_used = 0usize;
        'script_calls: for call in calls {
            for script in &call.scripts {
                let Some(cached_script) = self.load_script(&script.path) else {
                    continue;
                };
                if !cached_script.supports_event(&event) {
                    continue;
                }
                if matches!(event, LuauScriptEvent::Update(_))
                    && !self.should_run_update_script(
                        &call,
                        &script.path,
                        focus,
                        &mut update_budget_used,
                    )
                {
                    continue;
                }
                self.call_script_event(entities, &call, script, cached_script, &event, &mut report);
                if self.debug_paused.is_some() {
                    break 'script_calls;
                }
                if !matches!(event, LuauScriptEvent::Update(_)) {
                    self.wake_update_script(call.entity_id, &script.path);
                }
                if self.apply_pending_commands(entities, &mut report) {
                    self.refresh_world_snapshot(entities);
                }
            }
        }
        report.errors = self.last_errors.clone();
        self.sync_query_stats();
        report
    }

    fn call_script_event(
        &mut self,
        entities: &mut [GameObject],
        call: &EntityScriptCall,
        script_ref: &ScriptAttachment,
        script: CachedLuauScript,
        event: &LuauScriptEvent,
        report: &mut LuauRunReport,
    ) {
        let path = &script_ref.path;
        if let Ok(mut host) = self.host.lock() {
            host.current_entity_id = Some(call.entity_id);
        }
        self.interrupt_budget
            .store(SCRIPT_INTERRUPT_BUDGET, Ordering::Relaxed);
        let context_key = (call.entity_id, path.clone());
        let result = (|| -> mlua::Result<usize> {
            let (environment, instance, method_style) =
                self.script_context(&context_key, &script, call, path)?;
            sync_time_table(&self.lua, &environment, self.host_time())?;
            environment.set("entity_id", call.entity_id)?;
            environment.set("entity_name", call.entity_name.clone())?;
            instance.set("entity_id", call.entity_id)?;
            instance.set("entity_name", call.entity_name.clone())?;
            apply_public_variables(&self.lua, &instance, &script_ref.public_variables)?;
            if let Some(entity) = entities.iter().find(|entity| entity.id == call.entity_id) {
                let entity_proxy = entity_to_luau(&self.lua, entity)?;
                instance.set("entity", entity_proxy.clone())?;
                environment.set("entity", entity_proxy)?;
            }

            if let LuauScriptEvent::Update(dt) = event {
                pump_context_tasks(&environment, *dt)?;
            }
            let mut called = 0;
            for function_name in event.function_names() {
                if !script.handlers.contains(*function_name) {
                    continue;
                }
                if !self.handler_exists(&environment, &instance, method_style, function_name)? {
                    continue;
                }
                let callback_key = DebugCallbackKey {
                    entity_id: call.entity_id,
                    path: normalize_path(path),
                    function: (*function_name).to_string(),
                };
                let skip_breakpoint = self
                    .debug_skip_once
                    .as_ref()
                    .is_some_and(|skip| skip == &callback_key);
                if skip_breakpoint {
                    self.debug_skip_once = None;
                }
                let source_line = script_handler_line(path, function_name);
                if !skip_breakpoint
                    && (self.debug_pause_requested
                        || self.debug_breakpoints.iter().any(|breakpoint| {
                            breakpoint_matches(breakpoint, path, function_name, source_line)
                        }))
                {
                    self.debug_pause_requested = false;
                    let relative_path = path
                        .strip_prefix(&self.project_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    let entity = entities
                        .iter()
                        .find(|entity| entity.id == call.entity_id)
                        .and_then(|entity| serde_json::to_value(entity).ok())
                        .unwrap_or(Value::Null);
                    self.debug_paused = Some(ScriptPausedFrame {
                        entity_id: call.entity_id,
                        entity_name: call.entity_name.clone(),
                        path: relative_path,
                        function: (*function_name).to_string(),
                        line: source_line,
                        event: event_debug_name(event).to_string(),
                        context: json!({
                            "self": exported_variables_from_instance(&instance),
                            "entity": entity,
                            "event": event_debug_context(event),
                            "Time": {
                                "delta_time": self.host_time().delta_time,
                                "fixed_delta_time": self.host_time().fixed_delta_time,
                                "time": self.host_time().total_time,
                                "frame": self.host_time().frame,
                            }
                        }),
                    });
                    report.debug_messages.push(format!(
                        "[debugger] paused at {}:{} ({function_name})",
                        path.display(),
                        source_line.unwrap_or_default()
                    ));
                    break;
                }
                if self.call_handler(&environment, &instance, method_style, function_name, event)? {
                    called += 1;
                    if self.debug_step_after_callback {
                        self.debug_step_after_callback = false;
                        self.debug_pause_requested = true;
                    }
                }
            }
            if called > 0
                && let Some(entity) = entities
                    .iter_mut()
                    .find(|entity| entity.id == call.entity_id)
            {
                sync_entity_proxy_from_instance(&instance, entity)?;
                let variables = exported_variables_from_instance(&instance);
                sync_public_variables_to_entity(entity, &script_ref.path, variables);
            }
            Ok(called)
        })();
        if let Ok(mut host) = self.host.lock() {
            host.current_entity_id = None;
        }
        self.interrupt_budget
            .store(SCRIPT_INTERRUPT_BUDGET, Ordering::Relaxed);
        match result {
            Ok(called) => {
                self.last_frame_scripts += called;
                report.scripts_run += called;
            }
            Err(error) => self.last_errors.push(format_luau_error(path, event, error)),
        }
    }

    fn script_context(
        &mut self,
        context_key: &(u64, PathBuf),
        script: &CachedLuauScript,
        call: &EntityScriptCall,
        path: &Path,
    ) -> mlua::Result<(Table, Table, bool)> {
        if let Some(context) = self
            .contexts
            .get(context_key)
            .filter(|context| context.modified == script.modified)
        {
            return Ok((
                context.environment.clone(),
                context.instance.clone(),
                context.method_style,
            ));
        }

        let environment = self.lua.create_table()?;
        let metatable = self.lua.create_table()?;
        metatable.set("__index", self.lua.globals())?;
        environment.set_metatable(Some(metatable))?;
        install_context_luau_api(&self.lua, &environment)?;
        environment.set("entity_id", call.entity_id)?;
        environment.set("entity_name", call.entity_name.clone())?;
        let result: LuaValue = self
            .lua
            .load(&script.bytecode)
            .set_name(format!("@{}", path.display()))
            .set_environment(environment.clone())
            .eval()?;
        let (instance, method_style) = match result {
            LuaValue::Table(table) => (table, true),
            LuaValue::Nil => (environment.clone(), false),
            _ => {
                let instance = self.lua.create_table()?;
                (instance, true)
            }
        };
        self.contexts.insert(
            context_key.clone(),
            LuauScriptContext {
                environment: environment.clone(),
                instance: instance.clone(),
                modified: script.modified,
                method_style,
            },
        );
        Ok((environment, instance, method_style))
    }

    fn call_handler(
        &self,
        environment: &Table,
        instance: &Table,
        method_style: bool,
        function_name: &str,
        event: &LuauScriptEvent,
    ) -> mlua::Result<bool> {
        if method_style
            && let LuaValue::Function(function) = instance.get::<LuaValue>(function_name)?
        {
            call_event_function(&self.lua, function, Some(instance.clone()), event)?;
            return Ok(true);
        }
        if let LuaValue::Function(function) = environment.get::<LuaValue>(function_name)? {
            call_event_function(&self.lua, function, None, event)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn handler_exists(
        &self,
        environment: &Table,
        instance: &Table,
        method_style: bool,
        function_name: &str,
    ) -> mlua::Result<bool> {
        if method_style
            && matches!(
                instance.get::<LuaValue>(function_name)?,
                LuaValue::Function(_)
            )
        {
            return Ok(true);
        }
        Ok(matches!(
            environment.get::<LuaValue>(function_name)?,
            LuaValue::Function(_)
        ))
    }

    fn refresh_world_snapshot(&mut self, entities: &[GameObject]) {
        if let Ok(mut host) = self.host.lock() {
            host.replace_world_entities(entities);
        }
    }

    fn reset_query_stats(&mut self) {
        if let Ok(mut host) = self.host.lock() {
            host.reset_query_stats();
        }
    }

    fn sync_query_stats(&mut self) {
        if let Ok(host) = self.host.lock() {
            self.last_query_stats = host.query_stats;
        }
    }

    fn camera_focus_point(&self) -> Option<(f64, f64)> {
        self.host
            .lock()
            .ok()
            .map(|host| (host.camera.x, host.camera.y))
    }

    fn should_run_update_script(
        &mut self,
        call: &EntityScriptCall,
        path: &Path,
        focus: Option<(f64, f64)>,
        update_budget_used: &mut usize,
    ) -> bool {
        let policy = call.update_policy;
        if !self.scheduler_config.enabled || policy.always_update {
            *update_budget_used = update_budget_used.saturating_add(1);
            self.last_scheduler_stats.update_budget_used = self
                .last_scheduler_stats
                .update_budget_used
                .saturating_add(1);
            return policy.enabled;
        }
        if !policy.enabled {
            self.last_scheduler_stats.skipped_disabled =
                self.last_scheduler_stats.skipped_disabled.saturating_add(1);
            return false;
        }
        if *update_budget_used >= self.scheduler_config.max_update_scripts_per_frame
            && policy.priority < self.scheduler_config.budget_bypass_priority
        {
            self.last_scheduler_stats.skipped_budget =
                self.last_scheduler_stats.skipped_budget.saturating_add(1);
            return false;
        }

        let mut interval = policy
            .update_interval
            .max(self.scheduler_config.default_update_interval)
            .max(0.0);
        if let (Some((fx, fy)), Some(max_distance)) = (focus, policy.max_distance) {
            let dx = call.x - fx;
            let dy = call.y - fy;
            if dx * dx + dy * dy > max_distance * max_distance {
                self.last_scheduler_stats.distance_throttled = self
                    .last_scheduler_stats
                    .distance_throttled
                    .saturating_add(1);
                interval = interval.max(
                    policy
                        .distant_update_interval
                        .unwrap_or(self.scheduler_config.distant_update_interval),
                );
            }
        }
        if interval <= f64::EPSILON {
            *update_budget_used = update_budget_used.saturating_add(1);
            self.last_scheduler_stats.update_budget_used = self
                .last_scheduler_stats
                .update_budget_used
                .saturating_add(1);
            return true;
        }

        let now = self.host_time().total_time;
        let key = (call.entity_id, normalize_path(path));
        let state = self.update_states.entry(key).or_default();
        if now + 0.000_001 < state.next_update_time {
            self.last_scheduler_stats.skipped_interval =
                self.last_scheduler_stats.skipped_interval.saturating_add(1);
            return false;
        }
        let phase = script_schedule_phase(call.entity_id, path, interval);
        state.next_update_time = now + interval + phase;
        *update_budget_used = update_budget_used.saturating_add(1);
        self.last_scheduler_stats.update_budget_used = self
            .last_scheduler_stats
            .update_budget_used
            .saturating_add(1);
        true
    }

    fn wake_update_script(&mut self, entity_id: u64, path: &Path) {
        let key = (entity_id, normalize_path(path));
        self.update_states.insert(
            key,
            ScriptUpdateState {
                next_update_time: 0.0,
            },
        );
    }

    fn load_script(&mut self, path: &Path) -> Option<CachedLuauScript> {
        let key = normalize_path(path);
        let modified = fs::metadata(&key)
            .and_then(|metadata| metadata.modified())
            .ok();
        if let Some(cached) = self
            .cache
            .get(&key)
            .filter(|cached| cached.modified == modified)
        {
            return Some(cached.clone());
        }
        let source = fs::read_to_string(&key)
            .map_err(|error| self.last_errors.push(format!("{}: {error}", key.display())))
            .ok()?;
        let bytecode = luau_compiler()
            .compile(&source)
            .map_err(|error| self.last_errors.push(format!("{}: {error}", key.display())))
            .ok()?;
        let cached = CachedLuauScript {
            bytecode,
            modified,
            handlers: detect_script_handlers(&source),
        };
        self.cache.insert(key, cached.clone());
        Some(cached)
    }

    fn collect_script_calls(&self, entities: &[GameObject]) -> Vec<EntityScriptCall> {
        entities
            .iter()
            .filter(|entity| entity.enabled && entity.active)
            .filter_map(|entity| {
                let scripts = self.script_attachments_for_entity(entity);
                (!scripts.is_empty()).then(|| EntityScriptCall {
                    entity_id: entity.id,
                    entity_name: entity.name.clone(),
                    x: entity.x,
                    y: entity.y,
                    update_policy: script_update_policy(
                        entity,
                        self.scheduler_config.open_world_auto_policy,
                    ),
                    scripts,
                })
            })
            .collect()
    }

    fn retain_live_entities(&mut self, entities: &[GameObject]) {
        let live = entities
            .iter()
            .map(|entity| entity.id)
            .collect::<BTreeSet<_>>();
        self.ready_scripts
            .retain(|(entity_id, _)| live.contains(entity_id));
        self.created_scripts
            .retain(|(entity_id, _)| live.contains(entity_id));
        self.destroying_entities.retain(|id| live.contains(id));
        self.contexts
            .retain(|(entity_id, _), _| live.contains(entity_id));
        self.update_states
            .retain(|(entity_id, _), _| live.contains(entity_id));
    }

    fn script_attachments_for_entity(&self, entity: &GameObject) -> Vec<ScriptAttachment> {
        let mut refs = Vec::<(String, Value)>::new();
        if let Some(script) = &entity.script {
            refs.push((
                script.clone(),
                script_component_public_variables(entity, script),
            ));
        }
        if let Some(component) = entity.get_component("ScriptComponent")
            && component.enabled
            && component
                .get("runtime")
                .and_then(Value::as_str)
                .unwrap_or("luau")
                .eq_ignore_ascii_case("luau")
        {
            if let Some(path) = component.get("path").and_then(Value::as_str) {
                refs.push((path.to_string(), component_public_variables(component)));
            }
            if let Some(scripts) = component.get("scripts").and_then(Value::as_array) {
                for script in scripts {
                    if let Some(path) = script.as_str() {
                        refs.push((path.to_string(), Value::Null));
                    } else if script
                        .get("runtime")
                        .and_then(Value::as_str)
                        .unwrap_or("luau")
                        .eq_ignore_ascii_case("luau")
                        && let Some(path) = ["path", "script", "name"]
                            .iter()
                            .find_map(|key| script.get(key).and_then(Value::as_str))
                    {
                        refs.push((
                            path.to_string(),
                            script
                                .get("public_variables")
                                .cloned()
                                .unwrap_or(Value::Null),
                        ));
                    }
                }
            }
        }
        for script in &entity.scripts {
            if let Some(path) = script.as_str() {
                refs.push((path.to_string(), Value::Null));
            } else if script
                .get("runtime")
                .and_then(Value::as_str)
                .is_some_and(|runtime| runtime.eq_ignore_ascii_case("luau"))
                && let Some(path) = ["path", "script", "name"]
                    .iter()
                    .find_map(|key| script.get(key).and_then(Value::as_str))
            {
                refs.push((
                    path.to_string(),
                    script
                        .get("public_variables")
                        .cloned()
                        .unwrap_or(Value::Null),
                ));
            }
        }
        let mut seen = BTreeSet::new();
        refs.into_iter()
            .filter_map(|(value, public_variables)| {
                let path = self.resolve_script_path(&value)?;
                seen.insert(path.clone()).then_some(ScriptAttachment {
                    path,
                    public_variables,
                })
            })
            .collect()
    }

    fn resolve_script_path(&self, value: &str) -> Option<PathBuf> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        let raw = Path::new(value);
        let extension = raw
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase);
        if extension
            .as_deref()
            .is_some_and(|ext| ext != "luau" && ext != "lua")
        {
            return None;
        }
        let mut candidates = if raw.is_absolute() {
            vec![raw.to_path_buf()]
        } else {
            vec![
                self.project_path.join(raw),
                self.project_path.join("scripts").join(raw),
            ]
        };
        if extension.is_none() {
            candidates.push(
                self.project_path
                    .join("scripts")
                    .join(format!("{value}.luau")),
            );
        }
        candidates
            .iter()
            .find(|path| path.exists())
            .map(normalize_path)
            .or_else(|| {
                extension
                    .is_some()
                    .then(|| normalize_path(candidates.last().expect("candidate")))
            })
    }

    fn apply_pending_commands(
        &mut self,
        entities: &mut Vec<GameObject>,
        report: &mut LuauRunReport,
    ) -> bool {
        let before = report.commands_applied
            + report.spawned.len()
            + report.destroyed.len()
            + report.ui_updates
            + report.scene_requests.len();
        let commands = self.drain_commands();
        if commands.is_empty() {
            return false;
        }
        for command in commands {
            self.apply_command(entities, command, report);
        }
        let after = report.commands_applied
            + report.spawned.len()
            + report.destroyed.len()
            + report.ui_updates
            + report.scene_requests.len();
        after != before
    }

    fn apply_command(
        &mut self,
        entities: &mut Vec<GameObject>,
        command: ScriptCommand,
        report: &mut LuauRunReport,
    ) {
        match command {
            ScriptCommand::Move { entity_id, dx, dy } => {
                if let Some(entity) = entities.iter_mut().find(|entity| entity.id == entity_id) {
                    GameAPI::translate(entity, dx, dy);
                    report.commands_applied += 1;
                }
            }
            ScriptCommand::SetPosition { entity_id, x, y } => {
                if let Some(entity) = entities.iter_mut().find(|entity| entity.id == entity_id) {
                    GameAPI::set_position(entity, x, y);
                    report.commands_applied += 1;
                }
            }
            ScriptCommand::Spawn { name, x, y } => {
                let id = GameAPI::spawn(entities, &name, x, y);
                report.spawned.push(id);
                report.commands_applied += 1;
            }
            ScriptCommand::SpawnConfigured { name, x, y, data } => {
                let mut entity = GameObject::new(x, y, Some(name));
                configure_spawned_entity(&mut entity, data);
                let id = entity.id;
                entities.push(entity);
                report.spawned.push(id);
                report.commands_applied += 1;
            }
            ScriptCommand::SpawnWithId {
                entity_id,
                name,
                x,
                y,
                data,
            } => {
                let mut entity = GameObject::new(x, y, Some(name));
                entity.id = entity_id;
                if let Some(data) = data {
                    configure_spawned_entity(&mut entity, data);
                }
                entity.sync_to_components();
                entities.push(entity);
                report.spawned.push(entity_id);
                report.commands_applied += 1;
            }
            ScriptCommand::Destroy { target } => {
                for id in resolve_targets(entities, &target) {
                    if !self.destroying_entities.contains(&id) {
                        let destroyed = self.run_destroy(entities, id);
                        report.merge(destroyed);
                    }
                }
            }
            ScriptCommand::PlaySound {
                name,
                bus,
                volume,
                looped,
            } => {
                GameAPI::play_sound(entities, &name, &bus, volume, looped);
                report.sounds.push(name);
                report.commands_applied += 1;
            }
            ScriptCommand::LoadScene { name } => {
                report.scene_requests.push(name);
                report.commands_applied += 1;
            }
            ScriptCommand::SetUiText { target, text } => {
                let changed = match target {
                    ScriptTarget::Id(id) => GameAPI::set_ui_text_by_id(entities, id, &text),
                    ScriptTarget::Name(name) => {
                        GameAPI::set_ui_text_by_name(entities, &name, &text)
                    }
                };
                if changed {
                    report.ui_updates += 1;
                    report.commands_applied += 1;
                }
            }
            ScriptCommand::SetUiProgress { target, value, max } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::set_ui_progress(entity, value, max)
                })
            }
            ScriptCommand::SetUiVisible { target, visible } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::set_ui_visible(entity, visible)
                })
            }
            ScriptCommand::SetTag { target, tag } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::set_tag(entity, &tag);
                    true
                })
            }
            ScriptCommand::SetLayer { target, layer } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::set_layer(entity, &layer);
                    true
                })
            }
            ScriptCommand::SetEnabled { target, enabled } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::set_enabled(entity, enabled);
                    true
                })
            }
            ScriptCommand::SetVisible { target, visible } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::set_visible(entity, visible);
                    true
                })
            }
            ScriptCommand::SetComponentNumber {
                target,
                component,
                key,
                value,
            } => for_each_target(entities, &target, report, |entity| {
                GameAPI::set_component_value(entity, &component, &key, json!(value))
            }),
            ScriptCommand::SetComponentText {
                target,
                component,
                key,
                value,
            } => for_each_target(entities, &target, report, |entity| {
                GameAPI::set_component_value(entity, &component, &key, json!(value))
            }),
            ScriptCommand::AddComponent {
                target,
                component,
                data,
            } => for_each_target(entities, &target, report, |entity| {
                GameAPI::add_component(entity, &component, Some(data.clone())).is_some()
            }),
            ScriptCommand::RemoveComponent { target, component } => {
                for_each_target(entities, &target, report, |entity| {
                    let existed = entity.get_component(&component).is_some();
                    if existed {
                        GameAPI::remove_component(entity, &component);
                    }
                    existed
                })
            }
            ScriptCommand::SetComponentValue {
                target,
                component,
                key,
                value,
            } => for_each_target(entities, &target, report, |entity| {
                ensure_component(entity, &component);
                GameAPI::set_component_value(entity, &component, &key, value.clone())
            }),
            ScriptCommand::SetVelocity { target, x, y } => {
                for_each_target(entities, &target, report, |entity| {
                    ensure_component(entity, "Rigidbody2D");
                    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                        body.set_f64("velocity_x", x);
                        body.set_f64("velocity_y", y);
                        body.set("sleeping", json!(false));
                        return true;
                    }
                    false
                })
            }
            ScriptCommand::ApplyImpulse { target, x, y } => {
                for_each_target(entities, &target, report, |entity| {
                    ensure_component(entity, "Rigidbody2D");
                    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                        body.add_force(x, y, true);
                        return true;
                    }
                    false
                })
            }
            ScriptCommand::ApplyForce { target, x, y } => {
                for_each_target(entities, &target, report, |entity| {
                    ensure_component(entity, "Rigidbody2D");
                    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                        body.add_force(x, y, false);
                        return true;
                    }
                    false
                })
            }
            ScriptCommand::ApplyTorque { target, torque } => {
                for_each_target(entities, &target, report, |entity| {
                    ensure_component(entity, "Rigidbody2D");
                    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                        body.set_f64("_torque", body.get_f64("_torque", 0.0) + torque);
                        body.set("sleeping", json!(false));
                        body.set_f64("_sleep_timer", 0.0);
                        return true;
                    }
                    false
                })
            }
            ScriptCommand::WakeBody { target } => {
                for_each_target(entities, &target, report, |entity| {
                    let Some(body) = entity.get_component_mut("Rigidbody2D") else {
                        return false;
                    };
                    body.set("sleeping", json!(false));
                    body.set_f64("_sleep_timer", 0.0);
                    true
                })
            }
            ScriptCommand::SleepBody { target } => {
                for_each_target(entities, &target, report, |entity| {
                    let Some(body) = entity.get_component_mut("Rigidbody2D") else {
                        return false;
                    };
                    body.set_f64("velocity_x", 0.0);
                    body.set_f64("velocity_y", 0.0);
                    body.set_f64("angular_velocity", 0.0);
                    body.set("sleeping", json!(true));
                    true
                })
            }
            ScriptCommand::SetCharacterInput {
                target,
                x,
                y,
                jump,
                run,
            } => for_each_target(entities, &target, report, |entity| {
                ensure_component(entity, "CharacterController2D");
                if let Some(controller) = entity.get_component_mut("CharacterController2D") {
                    controller.set_f64("input_x", x.clamp(-1.0, 1.0));
                    controller.set_f64("input_y", y.clamp(-1.0, 1.0));
                    controller.set("jump_pressed", json!(jump));
                    controller.set("jump_held", json!(jump));
                    controller.set("run_pressed", json!(run));
                    return true;
                }
                false
            }),
            ScriptCommand::SetCameraFollow {
                target,
                follow_target_id,
            } => for_each_target(entities, &target, report, |entity| {
                ensure_component(entity, "Camera2D");
                ensure_component(entity, "CameraFollow");
                if let Some(camera) = entity.get_component_mut("Camera2D") {
                    camera.set("active", json!(true));
                    camera.set("follow_target", json!(follow_target_id));
                }
                if let Some(follow) = entity.get_component_mut("CameraFollow") {
                    follow.set("target_id", json!(follow_target_id));
                    return true;
                }
                false
            }),
            ScriptCommand::SetCameraShake {
                target,
                duration,
                amplitude,
            } => for_each_target(entities, &target, report, |entity| {
                ensure_component(entity, "CameraShake");
                if let Some(shake) = entity.get_component_mut("CameraShake") {
                    shake.set_f64("duration", duration.max(0.0));
                    shake.set_f64("amplitude", amplitude.max(0.0));
                    shake.camera_shake(1.0);
                    return true;
                }
                false
            }),
            ScriptCommand::SetCameraPixelPerfect {
                target,
                enabled,
                pixels_per_unit,
            } => for_each_target(entities, &target, report, |entity| {
                ensure_component(entity, "Camera2D");
                if let Some(camera) = entity.get_component_mut("Camera2D") {
                    camera.set("pixel_perfect", json!(enabled));
                    camera.set_f64("pixels_per_unit", pixels_per_unit.max(1.0));
                    return true;
                }
                false
            }),
            ScriptCommand::SetAnimation { target, animation } => {
                for_each_target(entities, &target, report, |entity| {
                    set_entity_animation(entity, &animation)
                })
            }
            ScriptCommand::SetAnimationParameter { target, key, value } => {
                for_each_target(entities, &target, report, |entity| {
                    ensure_component(entity, "Animator");
                    ensure_component(entity, "AnimationPlayer");
                    set_animation_parameter(entity, &key, value.clone())
                })
            }
            ScriptCommand::SetTile {
                target,
                layer,
                x,
                y,
                tile,
            } => for_each_target(entities, &target, report, |entity| {
                ensure_component(entity, "Tilemap2D");
                set_tilemap_cell(entity, &layer, x, y, tile)
            }),
            ScriptCommand::SetTween {
                target,
                property_path,
                to_value,
                duration,
                easing,
            } => for_each_target(entities, &target, report, |entity| {
                ensure_component(entity, "Tween");
                let from_value = entity_property_value(entity, &property_path).unwrap_or(0.0);
                if let Some(tween) = entity.get_component_mut("Tween") {
                    tween.set("property_path", json!(property_path));
                    tween.set_f64("from_value", from_value);
                    tween.set_f64("to_value", to_value);
                    tween.set_f64("duration", duration.max(0.0001));
                    tween.set_f64("elapsed", 0.0);
                    tween.set("easing", json!(easing));
                    tween.set("active", json!(true));
                    return true;
                }
                false
            }),
            ScriptCommand::SetNavDestination { target, x, y } => {
                for_each_target(entities, &target, report, |entity| {
                    ensure_component(entity, "NavAgent");
                    if let Some(agent) = entity.get_component_mut("NavAgent") {
                        agent.nav_set_destination(x, y);
                        entity.path = vec![(x, y)];
                        entity.command = "NAVIGATE".to_string();
                        entity.state = "MOVING".to_string();
                        return true;
                    }
                    false
                })
            }
            ScriptCommand::ParticleBurst { target, count } => {
                for_each_target(entities, &target, report, |entity| {
                    ensure_component(entity, "ParticleEmitter");
                    if let Some(emitter) = entity.get_component_mut("ParticleEmitter") {
                        emitter.set("burst_count", json!(count.max(0)));
                        emitter.set("burst_emitted", json!(false));
                        return true;
                    }
                    false
                })
            }
            ScriptCommand::SetSprite {
                target,
                sprite_path,
            } => for_each_target(entities, &target, report, |entity| {
                set_sprite(entity, &sprite_path)
            }),
            ScriptCommand::SetSpriteAnimation {
                target,
                frames_path,
                animation,
            } => for_each_target(entities, &target, report, |entity| {
                set_sprite_animation(entity, &frames_path, &animation)
            }),
            ScriptCommand::SetSpriteFlip {
                target,
                flip_x,
                flip_y,
            } => for_each_target(entities, &target, report, |entity| {
                set_sprite_flip(entity, flip_x, flip_y)
            }),
            ScriptCommand::AddItem {
                target,
                item_id,
                quantity,
            } => for_each_target(entities, &target, report, |entity| {
                GameAPI::add_item(entity, &item_id, quantity);
                true
            }),
            ScriptCommand::AddResource {
                target,
                resource_type,
                amount,
            } => for_each_target(entities, &target, report, |entity| {
                GameAPI::add_resource(entity, &resource_type, amount);
                true
            }),
            ScriptCommand::SetBlackboard { target, key, value } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::set_blackboard(entity, &key, value.clone())
                })
            }
            ScriptCommand::AddQuest {
                target,
                quest_id,
                title,
                objectives,
            } => for_each_target(entities, &target, report, |entity| {
                GameAPI::add_quest(entity, &quest_id, &title, objectives.clone())
            }),
            ScriptCommand::QuestProgress {
                target,
                quest_id,
                objective_id,
                progress,
            } => for_each_target(entities, &target, report, |entity| {
                GameAPI::set_quest_objective_progress(
                    entity,
                    &quest_id,
                    &objective_id,
                    progress.clone(),
                )
            }),
            ScriptCommand::CompleteQuest { target, quest_id } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::complete_quest(entity, &quest_id)
                })
            }
            ScriptCommand::SaveGame { slot } => {
                match runtime_save_path(&self.project_path, &slot) {
                    Ok(path) => match GameAPI::save_game_state(entities, &path) {
                        Ok(()) => {
                            report
                                .debug_messages
                                .push(format!("[save] slot {slot} -> {}", path.display()));
                            report.commands_applied += 1;
                        }
                        Err(error) => report
                            .errors
                            .push(format!("save slot {slot} failed: {error}")),
                    },
                    Err(error) => report.errors.push(error),
                }
            }
            ScriptCommand::LoadGame { slot } => {
                match runtime_save_path(&self.project_path, &slot) {
                    Ok(path) => match GameAPI::load_game_state_into(entities, &path) {
                        Ok(count) => {
                            report.debug_messages.push(format!(
                                "[load] slot {slot} <- {} ({count} entities)",
                                path.display()
                            ));
                            report.commands_applied += 1;
                        }
                        Err(error) => report
                            .errors
                            .push(format!("load slot {slot} failed: {error}")),
                    },
                    Err(error) => report.errors.push(error),
                }
            }
            ScriptCommand::TriggerAbility { target, now } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::trigger_ability(entity, now)
                })
            }
            ScriptCommand::RechargeAbility { target, amount } => {
                for_each_target(entities, &target, report, |entity| {
                    GameAPI::recharge_ability(entity, amount)
                })
            }
            ScriptCommand::EmitEvent { name, payload } => {
                let event_report = self.run_custom_event(entities, name, payload);
                report.merge(event_report);
                report.commands_applied += 1;
            }
            ScriptCommand::DebugLog { level, message } => {
                report.debug_messages.push(format!("[{level}] {message}"));
                report.commands_applied += 1;
            }
        }
    }
}

#[derive(Debug, Clone)]
struct EntityScriptCall {
    entity_id: u64,
    entity_name: String,
    x: f64,
    y: f64,
    update_policy: ScriptUpdatePolicy,
    scripts: Vec<ScriptAttachment>,
}

#[derive(Debug, Clone)]
struct ScriptAttachment {
    path: PathBuf,
    public_variables: Value,
}

#[derive(Debug, Clone)]
struct ProjectRequire {
    root: PathBuf,
    current: PathBuf,
    resolved: Option<PathBuf>,
}

impl CachedLuauScript {
    fn supports_event(&self, event: &LuauScriptEvent) -> bool {
        event
            .function_names()
            .iter()
            .any(|handler| self.handlers.contains(*handler))
    }
}

fn detect_script_handlers(source: &str) -> BTreeSet<String> {
    [
        "on_create",
        "on_start",
        "on_ready",
        "on_update",
        "on_fixed_update",
        "on_key_down",
        "on_collision_enter",
        "on_collision_exit",
        "on_destroy",
        "on_event",
    ]
    .into_iter()
    .filter(|handler| source.contains(handler))
    .map(ToString::to_string)
    .collect()
}

fn script_focus_point(entities: &[GameObject]) -> Option<(f64, f64)> {
    entities
        .iter()
        .find(|entity| entity.enabled && entity.active && entity.tag == "Player")
        .or_else(|| {
            entities
                .iter()
                .find(|entity| entity.enabled && entity.active && entity.name == "Player")
        })
        .map(|entity| (entity.x, entity.y))
}

fn script_schedule_phase(entity_id: u64, path: &Path, interval: f64) -> f64 {
    if interval <= f64::EPSILON {
        return 0.0;
    }
    let mut hash = entity_id.wrapping_mul(1_099_511_628_211);
    for byte in path.to_string_lossy().bytes() {
        hash = hash.wrapping_mul(16_777_619) ^ u64::from(byte);
    }
    let unit = (hash % 997) as f64 / 997.0;
    unit * interval.min(0.5) * 0.35
}

fn compare_update_calls(
    a: &EntityScriptCall,
    b: &EntityScriptCall,
    focus: Option<(f64, f64)>,
    config: ScriptSchedulerConfig,
) -> std::cmp::Ordering {
    let priority_order = b
        .update_policy
        .priority
        .cmp(&a.update_policy.priority)
        .then_with(|| {
            script_class_rank(b.update_policy.simulation_class)
                .cmp(&script_class_rank(a.update_policy.simulation_class))
        });
    if priority_order != std::cmp::Ordering::Equal {
        return priority_order;
    }
    if config.prioritize_by_distance
        && let Some(focus) = focus
    {
        return distance_sq(a, focus)
            .partial_cmp(&distance_sq(b, focus))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.entity_id.cmp(&b.entity_id));
    }
    a.entity_id.cmp(&b.entity_id)
}

fn distance_sq(call: &EntityScriptCall, focus: (f64, f64)) -> f64 {
    let dx = call.x - focus.0;
    let dy = call.y - focus.1;
    dx * dx + dy * dy
}

fn script_class_rank(class: ScriptSimulationClass) -> i32 {
    match class {
        ScriptSimulationClass::Critical => 6,
        ScriptSimulationClass::Police => 5,
        ScriptSimulationClass::Vehicle => 4,
        ScriptSimulationClass::Pickup => 3,
        ScriptSimulationClass::Pedestrian => 2,
        ScriptSimulationClass::Background => 1,
        ScriptSimulationClass::Default => 0,
    }
}

fn script_update_policy(entity: &GameObject, open_world_auto_policy: bool) -> ScriptUpdatePolicy {
    let mut policy = ScriptUpdatePolicy::default();
    let mut explicit_timing = false;
    if let Some(component) = entity.get_component("ScriptSchedule") {
        policy.enabled = component.get_bool("enabled", policy.enabled);
        policy.always_update = component.get_bool("always_update", policy.always_update);
        policy.update_interval = component
            .get_f64("update_interval", policy.update_interval)
            .max(0.0);
        explicit_timing = true;
        let max_distance = component.get_f64("max_distance", 0.0);
        if max_distance > 0.0 {
            policy.max_distance = Some(max_distance);
        }
        let distant_update_interval = component.get_f64("distant_update_interval", 0.0);
        if distant_update_interval > 0.0 {
            policy.distant_update_interval = Some(distant_update_interval);
        }
        policy.priority = component.get_i64("priority", policy.priority);
    }
    if let Some(component) = entity.get_component("ScriptComponent") {
        policy.always_update = component.get_bool("always_update", policy.always_update);
        policy.update_interval = component
            .get_f64("update_interval", policy.update_interval)
            .max(0.0);
        explicit_timing = true;
        let max_distance = component.get_f64("max_update_distance", 0.0);
        if max_distance > 0.0 {
            policy.max_distance = Some(max_distance);
        }
        policy.priority = component.get_i64("priority", policy.priority);
    }
    if let Some(values) = entity
        .get_component("Blackboard")
        .and_then(|blackboard| blackboard.get("values"))
        .and_then(Value::as_object)
    {
        policy.always_update = values
            .get("script_always_update")
            .and_then(Value::as_bool)
            .unwrap_or(policy.always_update);
        policy.update_interval = values
            .get("script_update_interval")
            .and_then(Value::as_f64)
            .unwrap_or(policy.update_interval)
            .max(0.0);
        explicit_timing |= values.get("script_update_interval").is_some();
        if let Some(max_distance) = values.get("script_max_distance").and_then(Value::as_f64)
            && max_distance > 0.0
        {
            policy.max_distance = Some(max_distance);
        }
        if let Some(distant_interval) = values
            .get("script_distant_update_interval")
            .and_then(Value::as_f64)
            && distant_interval > 0.0
        {
            policy.distant_update_interval = Some(distant_interval);
        }
        policy.priority = values
            .get("script_priority")
            .and_then(Value::as_i64)
            .unwrap_or(policy.priority);
    }
    if open_world_auto_policy {
        apply_open_world_policy(entity, &mut policy, explicit_timing);
    }
    policy
}

fn apply_open_world_policy(
    entity: &GameObject,
    policy: &mut ScriptUpdatePolicy,
    explicit_timing: bool,
) {
    let class = classify_script_entity(entity);
    policy.simulation_class = class;
    match class {
        ScriptSimulationClass::Critical => {
            policy.priority = policy.priority.max(180);
            policy.always_update = true;
            policy.max_distance = None;
            if !explicit_timing {
                policy.update_interval = 0.0;
            }
        }
        ScriptSimulationClass::Police => {
            policy.priority = policy.priority.max(90);
            if policy.max_distance.is_none() {
                policy.max_distance = Some(80.0);
            }
            if policy.distant_update_interval.is_none() {
                policy.distant_update_interval = Some(0.55);
            }
            if !explicit_timing {
                policy.update_interval = 0.08;
            }
        }
        ScriptSimulationClass::Vehicle => {
            policy.priority = policy.priority.max(60);
            if policy.max_distance.is_none() {
                policy.max_distance = Some(72.0);
            }
            if policy.distant_update_interval.is_none() {
                policy.distant_update_interval = Some(0.9);
            }
            if !explicit_timing {
                policy.update_interval = 0.12;
            }
        }
        ScriptSimulationClass::Pickup => {
            policy.priority = policy.priority.max(30);
            if policy.max_distance.is_none() {
                policy.max_distance = Some(24.0);
            }
            if policy.distant_update_interval.is_none() {
                policy.distant_update_interval = Some(1.0);
            }
            if !explicit_timing {
                policy.update_interval = 0.18;
            }
        }
        ScriptSimulationClass::Pedestrian => {
            policy.priority = policy.priority.max(20);
            if policy.max_distance.is_none() {
                policy.max_distance = Some(36.0);
            }
            if policy.distant_update_interval.is_none() {
                policy.distant_update_interval = Some(1.25);
            }
            if !explicit_timing {
                policy.update_interval = 0.3;
            }
        }
        ScriptSimulationClass::Background => {
            policy.priority = policy.priority.max(10);
            if !explicit_timing {
                policy.update_interval = 0.5;
            }
        }
        ScriptSimulationClass::Default => {}
    }
}

fn classify_script_entity(entity: &GameObject) -> ScriptSimulationClass {
    let name = entity.name.as_str();
    let tag = entity.tag.as_str();
    if tag == "Player" || matches!(name, "Player" | "CityDirector" | "GameDirector") {
        return ScriptSimulationClass::Critical;
    }
    if tag == "Police"
        || name.starts_with("Police")
        || name.starts_with("PatrolOfficer")
        || name.starts_with("Roadblock_")
    {
        return ScriptSimulationClass::Police;
    }
    if tag == "Vehicle"
        || name.starts_with("TrafficCar")
        || name.starts_with("Vehicle")
        || name.ends_with("Car")
        || entity.get_component("Vehicle2D").is_some()
    {
        return ScriptSimulationClass::Vehicle;
    }
    if tag == "Collectible" || tag == "Pickup" || name.starts_with("Pickup") {
        return ScriptSimulationClass::Pickup;
    }
    if tag == "NPC" || name.starts_with("NPC_") || name.starts_with("Pedestrian") {
        return ScriptSimulationClass::Pedestrian;
    }
    if entity.get_component("UIElement").is_some() || !entity.visible {
        return ScriptSimulationClass::Background;
    }
    ScriptSimulationClass::Default
}

impl ProjectRequire {
    fn new(root: impl Into<PathBuf>) -> Self {
        let root = normalize_path(root.into());
        Self {
            current: root.clone(),
            root,
            resolved: None,
        }
    }

    fn normalize_logical(path: &Path) -> PathBuf {
        let mut components = VecDeque::new();
        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => components.push_back(component),
                Component::CurDir => {}
                Component::ParentDir => {
                    if matches!(components.back(), Some(Component::Normal(_))) {
                        components.pop_back();
                    } else {
                        components.push_back(component);
                    }
                }
                Component::Normal(_) => components.push_back(component),
            }
        }
        components.into_iter().collect()
    }

    fn normalize_chunk_name(chunk_name: &str) -> &str {
        if let Some((path, line)) = chunk_name.rsplit_once(':')
            && line.parse::<u32>().is_ok()
        {
            return path;
        }
        chunk_name
    }

    fn path_in_root(&self, path: impl AsRef<Path>) -> Result<PathBuf, NavigateError> {
        let path = path.as_ref();
        let normalized = if path.is_absolute() {
            Self::normalize_logical(path)
        } else {
            Self::normalize_logical(&self.root.join(path))
        };
        if normalized.starts_with(&self.root) {
            Ok(normalized)
        } else {
            Err(NavigateError::NotFound)
        }
    }

    fn set_current(&mut self, path: PathBuf) -> Result<(), NavigateError> {
        self.current = self.path_in_root(path)?;
        self.resolved = resolve_project_module(&self.current)?;
        Ok(())
    }
}

impl Require for ProjectRequire {
    fn is_require_allowed(&self, chunk_name: &str) -> bool {
        chunk_name.starts_with('@')
    }

    fn reset(&mut self, chunk_name: &str) -> Result<(), NavigateError> {
        let Some(chunk_name) = chunk_name.strip_prefix('@') else {
            return Err(NavigateError::NotFound);
        };
        let chunk_name = Self::normalize_chunk_name(chunk_name);
        self.set_current(PathBuf::from(chunk_name))
    }

    fn jump_to_alias(&mut self, path: &str) -> Result<(), NavigateError> {
        self.set_current(PathBuf::from(path))
    }

    fn to_parent(&mut self) -> Result<(), NavigateError> {
        let mut parent = self.current.clone();
        if !parent.pop() {
            return Err(NavigateError::NotFound);
        }
        self.set_current(parent)
    }

    fn to_child(&mut self, name: &str) -> Result<(), NavigateError> {
        let child = Path::new(name);
        if child
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(NavigateError::NotFound);
        }
        self.set_current(self.current.join(child))
    }

    fn has_module(&self) -> bool {
        self.resolved.as_deref().is_some_and(Path::is_file)
    }

    fn cache_key(&self) -> String {
        self.resolved
            .as_deref()
            .unwrap_or(&self.current)
            .display()
            .to_string()
    }

    fn has_config(&self) -> bool {
        false
    }

    fn config(&self) -> io::Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn loader(&self, lua: &Lua) -> mlua::Result<Function> {
        let Some(path) = self.resolved.as_deref() else {
            return Err(mlua::Error::RuntimeError(
                "Luau module not resolved".to_string(),
            ));
        };
        let source = fs::read_to_string(path).map_err(mlua::Error::external)?;
        lua.load(&source)
            .set_name(format!("@{}", path.display()))
            .set_compiler(luau_compiler())
            .into_function()
    }
}

fn resolve_project_module(path: &Path) -> Result<Option<PathBuf>, NavigateError> {
    let mut found = None;
    let mut seen = BTreeSet::new();
    let candidates = [
        path.to_path_buf(),
        path.with_extension("luau"),
        path.with_extension("lua"),
        path.join("init.luau"),
        path.join("init.lua"),
    ];
    for candidate in candidates {
        if seen.insert(candidate.clone())
            && candidate.is_file()
            && found.replace(candidate).is_some()
        {
            return Err(NavigateError::Ambiguous);
        }
    }
    if found.is_some() {
        return Ok(found);
    }
    if path.is_dir() {
        Ok(None)
    } else {
        Err(NavigateError::NotFound)
    }
}

fn build_luau_vm(
    host: SharedHostState,
    interrupt_budget: Arc<AtomicU64>,
    project_path: PathBuf,
) -> mlua::Result<Lua> {
    let lua = Lua::new();
    let _ = lua.set_memory_limit(LUA_MEMORY_LIMIT_BYTES);
    lua.set_interrupt(move |_| {
        let remaining = interrupt_budget.fetch_sub(1, Ordering::Relaxed);
        if remaining == 0 {
            return Err(mlua::Error::RuntimeError(
                "script execution budget exceeded".to_string(),
            ));
        }
        Ok(VmState::Continue)
    });
    let globals = lua.globals();
    let api = lua.create_table()?;
    let require = lua.create_require_function(ProjectRequire::new(project_path.join("scripts")))?;
    globals.set("require", require)?;

    register_command(&lua, &globals, &api, "move", host.clone(), |host, args| {
        let (dx, dy): (f64, f64) = args;
        push_self_command(host, |entity_id| ScriptCommand::Move { entity_id, dx, dy });
    })?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_position",
        host.clone(),
        |host, args| {
            let (x, y): (f64, f64) = args;
            push_self_command(host, |entity_id| ScriptCommand::SetPosition {
                entity_id,
                x,
                y,
            });
        },
    )?;
    register_command(&lua, &globals, &api, "spawn", host.clone(), |host, args| {
        let (name, x, y): (String, f64, f64) = args;
        push_command(host, ScriptCommand::Spawn { name, x, y });
    })?;
    register_command(
        &lua,
        &globals,
        &api,
        "load_scene",
        host.clone(),
        |host, name: String| push_command(host, ScriptCommand::LoadScene { name }),
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "play_sound",
        host.clone(),
        |host, args: (String, Option<String>, Option<f64>, Option<bool>)| {
            push_command(
                host,
                ScriptCommand::PlaySound {
                    name: args.0,
                    bus: args.1.unwrap_or_else(|| "SFX".to_string()),
                    volume: args.2.unwrap_or(1.0),
                    looped: args.3.unwrap_or(false),
                },
            );
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "ui_text",
        host.clone(),
        |host, text: String| {
            push_self_command(host, |id| ScriptCommand::SetUiText {
                target: ScriptTarget::Id(id),
                text,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_ui_text",
        host.clone(),
        |host, (target, text): (String, String)| {
            push_command(
                host,
                ScriptCommand::SetUiText {
                    target: ScriptTarget::Name(target),
                    text,
                },
            )
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_ui_progress",
        host.clone(),
        |host, (value, max): (f64, f64)| {
            push_self_command(host, |id| ScriptCommand::SetUiProgress {
                target: ScriptTarget::Id(id),
                value,
                max,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_ui_progress_for",
        host.clone(),
        |host, (target, value, max): (String, f64, f64)| {
            push_command(
                host,
                ScriptCommand::SetUiProgress {
                    target: ScriptTarget::Name(target),
                    value,
                    max,
                },
            )
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_ui_visible",
        host.clone(),
        |host, visible: bool| {
            push_self_command(host, |id| ScriptCommand::SetUiVisible {
                target: ScriptTarget::Id(id),
                visible,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_ui_visible_for",
        host.clone(),
        |host, (target, visible): (String, bool)| {
            push_command(
                host,
                ScriptCommand::SetUiVisible {
                    target: ScriptTarget::Name(target),
                    visible,
                },
            )
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_tag",
        host.clone(),
        |host, tag: String| {
            push_self_command(host, |id| ScriptCommand::SetTag {
                target: ScriptTarget::Id(id),
                tag,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_layer",
        host.clone(),
        |host, layer: String| {
            push_self_command(host, |id| ScriptCommand::SetLayer {
                target: ScriptTarget::Id(id),
                layer,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_enabled",
        host.clone(),
        |host, enabled: bool| {
            push_self_command(host, |id| ScriptCommand::SetEnabled {
                target: ScriptTarget::Id(id),
                enabled,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_visible",
        host.clone(),
        |host, visible: bool| {
            push_self_command(host, |id| ScriptCommand::SetVisible {
                target: ScriptTarget::Id(id),
                visible,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_component_number",
        host.clone(),
        |host, (component, key, value): (String, String, f64)| {
            push_self_command(host, |id| ScriptCommand::SetComponentNumber {
                target: ScriptTarget::Id(id),
                component,
                key,
                value,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_component_number_for",
        host.clone(),
        |host, (target, component, key, value): (String, String, String, f64)| {
            push_command(
                host,
                ScriptCommand::SetComponentNumber {
                    target: ScriptTarget::Name(target),
                    component,
                    key,
                    value,
                },
            )
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_component_text",
        host.clone(),
        |host, (component, key, value): (String, String, String)| {
            push_self_command(host, |id| ScriptCommand::SetComponentText {
                target: ScriptTarget::Id(id),
                component,
                key,
                value,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_sprite",
        host.clone(),
        |host, sprite_path: String| {
            push_self_command(host, |id| ScriptCommand::SetSprite {
                target: ScriptTarget::Id(id),
                sprite_path,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "play_sprite_animation",
        host.clone(),
        |host, (frames_path, animation): (String, String)| {
            push_self_command(host, |id| ScriptCommand::SetSpriteAnimation {
                target: ScriptTarget::Id(id),
                frames_path,
                animation,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "set_sprite_flip",
        host.clone(),
        |host, (flip_x, flip_y): (bool, bool)| {
            push_self_command(host, |id| ScriptCommand::SetSpriteFlip {
                target: ScriptTarget::Id(id),
                flip_x,
                flip_y,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "face_left",
        host.clone(),
        |host, (): ()| {
            push_self_command(host, |id| ScriptCommand::SetSpriteFlip {
                target: ScriptTarget::Id(id),
                flip_x: true,
                flip_y: false,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "face_right",
        host.clone(),
        |host, (): ()| {
            push_self_command(host, |id| ScriptCommand::SetSpriteFlip {
                target: ScriptTarget::Id(id),
                flip_x: false,
                flip_y: false,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "add_item",
        host.clone(),
        |host, (item_id, quantity): (String, i64)| {
            push_self_command(host, |id| ScriptCommand::AddItem {
                target: ScriptTarget::Id(id),
                item_id,
                quantity,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "add_resource",
        host.clone(),
        |host, (resource_type, amount): (String, f64)| {
            push_self_command(host, |id| ScriptCommand::AddResource {
                target: ScriptTarget::Id(id),
                resource_type,
                amount,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "add_quest",
        host.clone(),
        |host,
         (quest_id, title, objective_id, objective_text, target): (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
        )| {
            let objectives = match (objective_id, objective_text) {
                (Some(id), Some(text)) => json!([{
                    "id": id,
                    "text": text,
                    "progress": 0,
                    "target": target.unwrap_or(1).max(1),
                }]),
                _ => json!([]),
            };
            push_self_command(host, |id| ScriptCommand::AddQuest {
                target: ScriptTarget::Id(id),
                quest_id,
                title,
                objectives,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "quest_progress",
        host.clone(),
        |host, (quest_id, objective_id, progress): (String, String, LuaValue)| {
            let progress = luau_value_to_json(progress);
            push_self_command(host, |id| ScriptCommand::QuestProgress {
                target: ScriptTarget::Id(id),
                quest_id,
                objective_id,
                progress,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "complete_quest",
        host.clone(),
        |host, quest_id: String| {
            push_self_command(host, |id| ScriptCommand::CompleteQuest {
                target: ScriptTarget::Id(id),
                quest_id,
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "trigger_ability",
        host.clone(),
        |host, now: Option<f64>| {
            push_self_command(host, |id| ScriptCommand::TriggerAbility {
                target: ScriptTarget::Id(id),
                now: now.unwrap_or_default(),
            })
        },
    )?;
    register_command(
        &lua,
        &globals,
        &api,
        "recharge_ability",
        host.clone(),
        |host, amount: i64| {
            push_self_command(host, |id| ScriptCommand::RechargeAbility {
                target: ScriptTarget::Id(id),
                amount,
            })
        },
    )?;

    let shared = host.clone();
    let input_pressed = lua.create_function(move |_, key: String| {
        Ok(shared
            .lock()
            .map(|state| state.inputs_pressed.contains(&key))
            .unwrap_or(false))
    })?;
    globals.set("input_pressed", input_pressed.clone())?;
    api.set("input_pressed", input_pressed)?;

    let shared = host.clone();
    let destroy = lua.create_function(move |_, target: Option<LuaValue>| {
        let command = match target {
            None | Some(LuaValue::Nil) => shared
                .lock()
                .ok()
                .and_then(|state| state.current_entity_id)
                .map(|id| ScriptCommand::Destroy {
                    target: ScriptTarget::Id(id),
                }),
            Some(LuaValue::Integer(id)) if id >= 0 => Some(ScriptCommand::Destroy {
                target: ScriptTarget::Id(id as u64),
            }),
            Some(LuaValue::Number(id)) if id >= 0.0 => Some(ScriptCommand::Destroy {
                target: ScriptTarget::Id(id as u64),
            }),
            Some(LuaValue::String(name)) => Some(ScriptCommand::Destroy {
                target: ScriptTarget::Name(name.to_str()?.to_string()),
            }),
            _ => None,
        };
        if let Some(command) = command {
            push_command(&shared, command);
        }
        Ok(())
    })?;
    globals.set("destroy", destroy.clone())?;
    api.set("destroy", destroy)?;

    let shared = host.clone();
    let set_blackboard = lua.create_function(move |_, (key, value): (String, LuaValue)| {
        let value = luau_value_to_json(value);
        push_self_command(&shared, |id| ScriptCommand::SetBlackboard {
            target: ScriptTarget::Id(id),
            key,
            value,
        });
        Ok(())
    })?;
    globals.set("set_blackboard", set_blackboard.clone())?;
    api.set("set_blackboard", set_blackboard)?;

    register_safe_luau_api(&lua, &globals, &api, host.clone(), project_path)?;

    globals.set("miniforge", api)?;
    lua.sandbox(true)?;
    drop(globals);
    Ok(lua)
}

fn register_safe_luau_api(
    lua: &Lua,
    globals: &Table,
    api: &Table,
    host: SharedHostState,
    project_path: PathBuf,
) -> mlua::Result<()> {
    let vector2 = lua.create_table()?;
    vector2.set(
        "new",
        lua.create_function(|lua, (x, y): (Option<f64>, Option<f64>)| {
            vector2_table(lua, x.unwrap_or_default(), y.unwrap_or_default())
        })?,
    )?;
    vector2.set(
        "length",
        lua.create_function(|_, value: Table| {
            let x = value.get::<f64>("x").unwrap_or_default();
            let y = value.get::<f64>("y").unwrap_or_default();
            Ok((x * x + y * y).sqrt())
        })?,
    )?;
    vector2.set(
        "normalized",
        lua.create_function(|lua, value: Table| {
            let x = value.get::<f64>("x").unwrap_or_default();
            let y = value.get::<f64>("y").unwrap_or_default();
            let len = (x * x + y * y).sqrt();
            if len <= f64::EPSILON {
                vector2_table(lua, 0.0, 0.0)
            } else {
                vector2_table(lua, x / len, y / len)
            }
        })?,
    )?;
    vector2.set(
        "add",
        lua.create_function(|lua, (a, b): (LuaValue, LuaValue)| {
            let a = luau_vec2(a)?.unwrap_or_default();
            let b = luau_vec2(b)?.unwrap_or_default();
            vector2_table(lua, a.0 + b.0, a.1 + b.1)
        })?,
    )?;
    vector2.set(
        "sub",
        lua.create_function(|lua, (a, b): (LuaValue, LuaValue)| {
            let a = luau_vec2(a)?.unwrap_or_default();
            let b = luau_vec2(b)?.unwrap_or_default();
            vector2_table(lua, a.0 - b.0, a.1 - b.1)
        })?,
    )?;
    vector2.set(
        "scale",
        lua.create_function(|lua, (value, factor): (LuaValue, f64)| {
            let value = luau_vec2(value)?.unwrap_or_default();
            vector2_table(lua, value.0 * factor, value.1 * factor)
        })?,
    )?;
    vector2.set(
        "dot",
        lua.create_function(|_, (a, b): (LuaValue, LuaValue)| {
            let a = luau_vec2(a)?.unwrap_or_default();
            let b = luau_vec2(b)?.unwrap_or_default();
            Ok(a.0 * b.0 + a.1 * b.1)
        })?,
    )?;
    vector2.set(
        "distance",
        lua.create_function(|_, (a, b): (LuaValue, LuaValue)| {
            let a = luau_vec2(a)?.unwrap_or_default();
            let b = luau_vec2(b)?.unwrap_or_default();
            Ok((b.0 - a.0).hypot(b.1 - a.1))
        })?,
    )?;
    vector2.set(
        "lerp",
        lua.create_function(|lua, (a, b, alpha): (LuaValue, LuaValue, f64)| {
            let a = luau_vec2(a)?.unwrap_or_default();
            let b = luau_vec2(b)?.unwrap_or_default();
            vector2_table(lua, a.0 + (b.0 - a.0) * alpha, a.1 + (b.1 - a.1) * alpha)
        })?,
    )?;
    vector2.set(
        "move_towards",
        lua.create_function(
            |lua, (current, target, max_delta): (LuaValue, LuaValue, f64)| {
                let current = luau_vec2(current)?.unwrap_or_default();
                let target = luau_vec2(target)?.unwrap_or_default();
                let delta = (target.0 - current.0, target.1 - current.1);
                let distance = delta.0.hypot(delta.1);
                let max_delta = max_delta.max(0.0);
                if distance <= max_delta || distance <= f64::EPSILON {
                    vector2_table(lua, target.0, target.1)
                } else {
                    let scale = max_delta / distance;
                    vector2_table(
                        lua,
                        current.0 + delta.0 * scale,
                        current.1 + delta.1 * scale,
                    )
                }
            },
        )?,
    )?;
    globals.set("Vector2", vector2.clone())?;
    api.set("Vector2", vector2)?;

    let input = lua.create_table()?;
    let shared = host.clone();
    let is_pressed = lua.create_function(move |_, key: String| {
        Ok(shared
            .lock()
            .map(|state| state.inputs_pressed.contains(&key))
            .unwrap_or(false))
    })?;
    input.set("is_pressed", is_pressed.clone())?;
    input.set("pressed", is_pressed)?;
    let shared = host.clone();
    input.set(
        "get_axis",
        lua.create_function(move |_, (negative, positive): (String, String)| {
            let (neg, pos) = shared
                .lock()
                .map(|state| {
                    (
                        state.inputs_pressed.contains(&negative),
                        state.inputs_pressed.contains(&positive),
                    )
                })
                .unwrap_or((false, false));
            Ok(match (neg, pos) {
                (true, false) => -1.0,
                (false, true) => 1.0,
                _ => 0.0,
            })
        })?,
    )?;
    let shared = host.clone();
    input.set(
        "action_pressed",
        lua.create_function(move |_, action: String| Ok(input_action_pressed(&shared, &action)))?,
    )?;
    let shared = host.clone();
    input.set(
        "axis",
        lua.create_function(move |_, (negative, positive): (String, String)| {
            Ok(
                match (
                    input_action_pressed(&shared, &negative),
                    input_action_pressed(&shared, &positive),
                ) {
                    (true, false) => -1.0,
                    (false, true) => 1.0,
                    _ => 0.0,
                },
            )
        })?,
    )?;
    globals.set("Input", input.clone())?;
    api.set("Input", input)?;

    let time = lua.create_table()?;
    time.set("delta_time", 0.0)?;
    time.set("fixed_delta_time", 0.0)?;
    time.set("time", 0.0)?;
    time.set("frame", 0_u64)?;
    globals.set("Time", time.clone())?;
    api.set("Time", time)?;

    let layers = lua.create_table()?;
    for (name, value) in [
        ("DEFAULT", "Default"),
        ("PLAYER", "Player"),
        ("PAWN", "Pawn"),
        ("ENEMY", "Enemy"),
        ("WORLD", "WorldStatic"),
        ("WORLD_STATIC", "WorldStatic"),
        ("TRIGGER", "Trigger"),
        ("ONE_WAY_PLATFORM", "OneWayPlatform"),
    ] {
        layers.set(name, value)?;
    }
    globals.set("Layers", layers.clone())?;
    api.set("Layers", layers)?;

    let physics = lua.create_table()?;
    let shared = host.clone();
    physics.set(
        "raycast",
        lua.create_function(
            move |lua, (origin, target, options): (LuaValue, LuaValue, Option<Table>)| {
                let origin = luau_vec2(origin)?.unwrap_or_default();
                let target = luau_vec2(target)?.unwrap_or(origin);
                let direction = (target.0 - origin.0, target.1 - origin.1);
                let max_distance = direction.0.hypot(direction.1);
                let include_triggers =
                    luau_options_bool(options.as_ref(), "include_triggers", false)
                        || luau_options_bool(options.as_ref(), "triggers", false);
                let layers = luau_layers_from_options(options.as_ref())?;
                let entities = shared
                    .lock()
                    .map(|host| host.world_entities.clone())
                    .unwrap_or_default();
                PhysicsSystem::new()
                    .raycast_filtered(
                        &entities,
                        origin,
                        direction,
                        max_distance,
                        include_triggers,
                        non_empty_layers(&layers),
                    )
                    .map(|hit| raycast_hit_to_luau(lua, &hit))
                    .transpose()
            },
        )?,
    )?;
    let shared = host.clone();
    physics.set(
        "shape_cast",
        lua.create_function(
            move |lua, (origin, target, options): (LuaValue, LuaValue, Option<Table>)| {
                let origin = luau_vec2(origin)?.unwrap_or_default();
                let target = luau_vec2(target)?.unwrap_or(origin);
                let direction = (target.0 - origin.0, target.1 - origin.1);
                let max_distance = direction.0.hypot(direction.1);
                let include_triggers =
                    luau_options_bool(options.as_ref(), "include_triggers", true);
                let layers = luau_layers_from_options(options.as_ref())?;
                let shape = luau_options_string(options.as_ref(), "shape", "box");
                let entities = shared
                    .lock()
                    .map(|host| host.world_entities.clone())
                    .unwrap_or_default();
                let hit = if shape == "circle" {
                    PhysicsSystem::new().circle_cast_filtered(
                        &entities,
                        CircleCastQuery {
                            origin,
                            radius: luau_options_number(options.as_ref(), "radius", 0.5),
                            direction,
                            max_distance,
                            filter: PhysicsQueryFilter {
                                include_triggers,
                                layers: non_empty_layers(&layers),
                            },
                        },
                    )
                } else {
                    PhysicsSystem::new().box_cast_filtered(
                        &entities,
                        BoxCastQuery {
                            origin,
                            half_extents: luau_options_vec2(
                                options.as_ref(),
                                "half_extents",
                                (0.5, 0.5),
                            ),
                            direction,
                            max_distance,
                            filter: PhysicsQueryFilter {
                                include_triggers,
                                layers: non_empty_layers(&layers),
                            },
                        },
                    )
                };
                hit.map(|hit| raycast_hit_to_luau(lua, &hit)).transpose()
            },
        )?,
    )?;
    let shared = host.clone();
    physics.set(
        "overlap_area",
        lua.create_function(
            move |lua, (center, half_extents, options): (LuaValue, LuaValue, Option<Table>)| {
                let center = luau_vec2(center)?.unwrap_or_default();
                let half_extents = luau_vec2(half_extents)?.unwrap_or((0.5, 0.5));
                let include_triggers =
                    luau_options_bool(options.as_ref(), "include_triggers", true);
                let layers = luau_layers_from_options(options.as_ref())?;
                let entities = shared
                    .lock()
                    .map(|host| host.world_entities.clone())
                    .unwrap_or_default();
                let hits = PhysicsSystem::new().overlap_area_filtered(
                    &entities,
                    center,
                    half_extents,
                    include_triggers,
                    non_empty_layers(&layers),
                );
                raycast_hits_to_luau(lua, &hits)
            },
        )?,
    )?;
    globals.set("Physics2D", physics.clone())?;
    api.set("Physics2D", physics)?;

    let components = lua.create_table()?;
    let shared = host.clone();
    components.set(
        "add",
        lua.create_function(
            move |_, (target, component, data): (LuaValue, String, Option<LuaValue>)| {
                if let Some(target) = target_from_luau_or_current(&shared, target)? {
                    push_command(
                        &shared,
                        ScriptCommand::AddComponent {
                            target,
                            component,
                            data: data.map(luau_value_to_json).unwrap_or_else(|| json!({})),
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    let shared = host.clone();
    components.set(
        "remove",
        lua.create_function(move |_, (target, component): (LuaValue, String)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(
                    &shared,
                    ScriptCommand::RemoveComponent { target, component },
                );
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    components.set(
        "set",
        lua.create_function(
            move |_, (target, component, key, value): (LuaValue, String, String, LuaValue)| {
                if let Some(target) = target_from_luau_or_current(&shared, target)? {
                    push_command(
                        &shared,
                        ScriptCommand::SetComponentValue {
                            target,
                            component,
                            key,
                            value: luau_value_to_json(value),
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    let shared = host.clone();
    components.set(
        "get",
        lua.create_function(
            move |lua,
                  (target, component, key, default): (
                LuaValue,
                String,
                String,
                Option<LuaValue>,
            )| {
                let target = target_from_luau_or_current(&shared, target)?;
                let value = shared
                    .lock()
                    .ok()
                    .and_then(|host| {
                        target.and_then(|target| find_snapshot_entity(&host, &target).cloned())
                    })
                    .and_then(|entity| {
                        entity
                            .get_component(&component)
                            .and_then(|component| component.get(&key).cloned())
                    });
                match value {
                    Some(value) => json_to_luau(lua, &value),
                    None => Ok(default.unwrap_or(LuaValue::Nil)),
                }
            },
        )?,
    )?;
    let shared = host.clone();
    components.set(
        "has",
        lua.create_function(move |_, (target, component): (LuaValue, String)| {
            let target = target_from_luau_or_current(&shared, target)?;
            Ok(shared.lock().ok().is_some_and(|host| {
                target
                    .and_then(|target| find_snapshot_entity(&host, &target))
                    .is_some_and(|entity| entity.get_component(&component).is_some())
            }))
        })?,
    )?;
    globals.set("Component", components.clone())?;
    api.set("Component", components)?;

    let rigidbody = lua.create_table()?;
    let shared = host.clone();
    rigidbody.set(
        "set_velocity",
        lua.create_function(move |_, (target, x, y): (LuaValue, f64, f64)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::SetVelocity { target, x, y });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    rigidbody.set(
        "apply_impulse",
        lua.create_function(move |_, (target, x, y): (LuaValue, f64, f64)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::ApplyImpulse { target, x, y });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    rigidbody.set(
        "apply_force",
        lua.create_function(move |_, (target, x, y): (LuaValue, f64, f64)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::ApplyForce { target, x, y });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    rigidbody.set(
        "apply_torque",
        lua.create_function(move |_, (target, torque): (LuaValue, f64)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::ApplyTorque { target, torque });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    rigidbody.set(
        "wake",
        lua.create_function(move |_, target: LuaValue| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::WakeBody { target });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    rigidbody.set(
        "sleep",
        lua.create_function(move |_, target: LuaValue| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::SleepBody { target });
            }
            Ok(())
        })?,
    )?;
    globals.set("Rigidbody2D", rigidbody.clone())?;
    api.set("Rigidbody2D", rigidbody)?;

    let character_body = lua.create_table()?;
    let shared = host.clone();
    character_body.set(
        "move",
        lua.create_function(
            move |_, (target, x, y, jump, run): (LuaValue, f64, f64, Option<bool>, Option<bool>)| {
                if let Some(target) = target_from_luau_or_current(&shared, target)? {
                    push_command(
                        &shared,
                        ScriptCommand::SetCharacterInput {
                            target,
                            x,
                            y,
                            jump: jump.unwrap_or(false),
                            run: run.unwrap_or(false),
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    globals.set("CharacterBody2D", character_body.clone())?;
    api.set("CharacterBody2D", character_body)?;

    let camera = lua.create_table()?;
    let shared = host.clone();
    let main_camera = lua.create_function(move |lua, (): ()| camera_handle(lua, shared.clone()))?;
    camera.set("main", main_camera.clone())?;
    camera.set("current", main_camera)?;
    globals.set("Camera", camera.clone())?;
    api.set("Camera", camera)?;

    let animation = lua.create_table()?;
    let shared = host.clone();
    animation.set(
        "play",
        lua.create_function(move |_, (first, second): (LuaValue, Option<String>)| {
            let (target, animation) = animation_target_and_name(&shared, first, second)?;
            if let Some(target) = target {
                push_command(&shared, ScriptCommand::SetAnimation { target, animation });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    animation.set(
        "set_parameter",
        lua.create_function(
            move |_, (target, key, value): (LuaValue, String, LuaValue)| {
                if let Some(target) = target_from_luau_or_current(&shared, target)? {
                    push_command(
                        &shared,
                        ScriptCommand::SetAnimationParameter {
                            target,
                            key,
                            value: luau_value_to_json(value),
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    globals.set("AnimationPlayer", animation.clone())?;
    globals.set("AnimatedSprite", animation.clone())?;
    api.set("AnimationPlayer", animation.clone())?;
    api.set("AnimatedSprite", animation)?;

    let tilemap = lua.create_table()?;
    let shared = host.clone();
    tilemap.set(
        "get_tile",
        lua.create_function(
            move |_, (target, layer, x, y): (LuaValue, String, usize, usize)| {
                let target = target_from_luau_or_current(&shared, target)?;
                let entity = shared.lock().ok().and_then(|host| {
                    target.and_then(|target| find_snapshot_entity(&host, &target).cloned())
                });
                Ok(entity.and_then(|entity| tilemap_cell(&entity, &layer, x, y)))
            },
        )?,
    )?;
    let shared = host.clone();
    tilemap.set(
        "set_tile",
        lua.create_function(
            move |_, (target, layer, x, y, tile): (LuaValue, String, usize, usize, i64)| {
                if let Some(target) = target_from_luau_or_current(&shared, target)? {
                    push_command(
                        &shared,
                        ScriptCommand::SetTile {
                            target,
                            layer,
                            x,
                            y,
                            tile,
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    globals.set("Tilemap", tilemap.clone())?;
    api.set("Tilemap", tilemap)?;

    let tween = lua.create_table()?;
    let shared = host.clone();
    tween.set(
        "to",
        lua.create_function(
            move |_,
                  (target, property_path, to_value, duration, options): (
                LuaValue,
                String,
                f64,
                f64,
                Option<Table>,
            )| {
                if let Some(target) = target_from_luau_or_current(&shared, target)? {
                    push_command(
                        &shared,
                        ScriptCommand::SetTween {
                            target,
                            property_path,
                            to_value,
                            duration,
                            easing: luau_options_string(options.as_ref(), "easing", "linear"),
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    globals.set("Tween", tween.clone())?;
    api.set("Tween", tween)?;

    let navigation = lua.create_table()?;
    let shared = host.clone();
    navigation.set(
        "set_destination",
        lua.create_function(move |_, (target, x, y): (LuaValue, f64, f64)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::SetNavDestination { target, x, y });
            }
            Ok(())
        })?,
    )?;
    globals.set("Navigation2D", navigation.clone())?;
    api.set("Navigation2D", navigation)?;

    let audio = lua.create_table()?;
    let shared = host.clone();
    audio.set(
        "play",
        lua.create_function(move |_, (name, options): (String, Option<Table>)| {
            push_command(
                &shared,
                ScriptCommand::PlaySound {
                    name,
                    bus: luau_options_string(options.as_ref(), "bus", "SFX"),
                    volume: luau_options_number(options.as_ref(), "volume", 1.0),
                    looped: luau_options_bool(options.as_ref(), "loop", false),
                },
            );
            Ok(())
        })?,
    )?;
    globals.set("Audio2D", audio.clone())?;
    api.set("Audio2D", audio)?;

    let particles = lua.create_table()?;
    let shared = host.clone();
    particles.set(
        "burst",
        lua.create_function(move |_, (target, count): (LuaValue, i64)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::ParticleBurst { target, count });
            }
            Ok(())
        })?,
    )?;
    globals.set("Particles2D", particles.clone())?;
    api.set("Particles2D", particles)?;

    let spawner = lua.create_table()?;
    let shared = host.clone();
    spawner.set(
        "spawn",
        lua.create_function(move |_, (name, x, y): (String, f64, f64)| {
            let entity_id = generate_entity_id();
            push_command(
                &shared,
                ScriptCommand::SpawnWithId {
                    entity_id,
                    name,
                    x,
                    y,
                    data: None,
                },
            );
            Ok(entity_id)
        })?,
    )?;
    globals.set("Spawner", spawner.clone())?;
    api.set("Spawner", spawner)?;

    let entity = lua.create_table()?;
    let shared = host.clone();
    entity.set(
        "current",
        lua.create_function(move |lua, (): ()| {
            let table = lua.create_table()?;
            if let Some(id) = shared.lock().ok().and_then(|state| state.current_entity_id) {
                table.set("id", id)?;
            }
            Ok(table)
        })?,
    )?;
    let shared = host.clone();
    entity.set(
        "spawn",
        lua.create_function(
            move |_, (name, x, y, data): (String, f64, f64, Option<LuaValue>)| {
                let entity_id = generate_entity_id();
                push_command(
                    &shared,
                    ScriptCommand::SpawnWithId {
                        entity_id,
                        name,
                        x,
                        y,
                        data: data.map(luau_value_to_json),
                    },
                );
                Ok(entity_id)
            },
        )?,
    )?;
    let shared = host.clone();
    entity.set(
        "find",
        lua.create_function(move |lua, target: LuaValue| {
            let Some(target) = target_to_script_target(&shared, Some(target))? else {
                return Ok(LuaValue::Nil);
            };
            let entity = shared
                .lock()
                .ok()
                .and_then(|host| find_snapshot_entity(&host, &target).cloned());
            match entity {
                Some(entity) => entity_to_luau(lua, &entity).map(LuaValue::Table),
                None => Ok(LuaValue::Nil),
            }
        })?,
    )?;
    let shared = host.clone();
    entity.set(
        "exists",
        lua.create_function(move |_, target: LuaValue| {
            let Some(target) = target_to_script_target(&shared, Some(target))? else {
                return Ok(false);
            };
            Ok(shared
                .lock()
                .ok()
                .is_some_and(|host| find_snapshot_entity(&host, &target).is_some()))
        })?,
    )?;
    let shared = host.clone();
    entity.set(
        "count_with_tag",
        lua.create_function(move |_, tag: String| {
            Ok(shared
                .lock()
                .map(|host| {
                    host.world_entity_tags
                        .get(&tag)
                        .into_iter()
                        .flat_map(|indices| indices.iter())
                        .filter_map(|index| host.world_entities.get(*index))
                        .filter(|entity| entity.enabled)
                        .count()
                })
                .unwrap_or_default())
        })?,
    )?;
    let shared = host.clone();
    entity.set(
        "nearby",
        lua.create_function(
            move |lua, (origin, radius, options): (LuaValue, f64, Option<Table>)| {
                let current_target = target_from_luau_or_current(&shared, origin.clone())?;
                let origin = luau_point_or_entity_position(&shared, origin)?
                    .or_else(|| {
                        shared.lock().ok().and_then(|host| {
                            current_target
                                .and_then(|target| find_snapshot_entity(&host, &target))
                                .map(|entity| (entity.x, entity.y))
                        })
                    })
                    .unwrap_or_default();
                let radius = radius.max(0.0);
                let include_disabled =
                    luau_options_bool(options.as_ref(), "include_disabled", false);
                let tags = luau_tags_from_options(options.as_ref())?;
                let layers = luau_layers_from_options(options.as_ref())?;
                let mut matches = shared
                    .lock()
                    .map(|mut host| {
                        nearby_snapshot_entities(
                            &mut host,
                            origin,
                            radius,
                            include_disabled,
                            &tags,
                            &layers,
                        )
                    })
                    .unwrap_or_default();
                matches.sort_by(|a, b| a.0.total_cmp(&b.0));
                let table = lua.create_table()?;
                for (index, (distance, entity)) in matches.iter().enumerate() {
                    table.set(
                        index + 1,
                        entity_to_luau_with_distance(lua, entity, *distance)?,
                    )?;
                }
                Ok(table)
            },
        )?,
    )?;
    let shared = host.clone();
    entity.set(
        "nearest",
        lua.create_function(
            move |lua, (origin, radius, options): (LuaValue, f64, Option<Table>)| {
                let origin_target = target_from_luau_or_current(&shared, origin.clone())?;
                let origin_point = luau_point_or_entity_position(&shared, origin)?
                    .or_else(|| {
                        shared.lock().ok().and_then(|host| {
                            origin_target
                                .as_ref()
                                .and_then(|target| find_snapshot_entity(&host, target))
                                .map(|entity| (entity.x, entity.y))
                        })
                    })
                    .unwrap_or_default();
                let exclude_origin = luau_options_bool(options.as_ref(), "exclude_origin", true);
                let origin_id = if exclude_origin {
                    shared.lock().ok().and_then(|host| {
                        origin_target
                            .as_ref()
                            .and_then(|target| find_snapshot_entity(&host, target))
                            .map(|entity| entity.id)
                    })
                } else {
                    None
                };
                let include_disabled =
                    luau_options_bool(options.as_ref(), "include_disabled", false);
                let tags = luau_tags_from_options(options.as_ref())?;
                let layers = luau_layers_from_options(options.as_ref())?;
                let nearest = shared
                    .lock()
                    .map(|mut host| {
                        nearby_snapshot_entities(
                            &mut host,
                            origin_point,
                            radius.max(0.0),
                            include_disabled,
                            &tags,
                            &layers,
                        )
                        .into_iter()
                        .filter(|(_, entity)| Some(entity.id) != origin_id)
                        .min_by(|a, b| a.0.total_cmp(&b.0))
                    })
                    .unwrap_or_default();
                match nearest {
                    Some((distance, entity)) => {
                        entity_to_luau_with_distance(lua, &entity, distance).map(LuaValue::Table)
                    }
                    None => Ok(LuaValue::Nil),
                }
            },
        )?,
    )?;
    let shared = host.clone();
    entity.set(
        "all_with_tag",
        lua.create_function(move |lua, tag: String| {
            let entities = shared
                .lock()
                .map(|host| {
                    host.world_entity_tags
                        .get(&tag)
                        .into_iter()
                        .flat_map(|indices| indices.iter())
                        .filter_map(|index| host.world_entities.get(*index))
                        .filter(|entity| entity.enabled)
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let table = lua.create_table()?;
            for (index, entity) in (1..).zip(entities.iter()) {
                table.set(index, entity_to_luau(lua, entity)?)?;
            }
            Ok(table)
        })?,
    )?;
    let shared = host.clone();
    entity.set(
        "set_visible",
        lua.create_function(move |_, (target, visible): (LuaValue, bool)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::SetVisible { target, visible });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    entity.set(
        "set_enabled",
        lua.create_function(move |_, (target, enabled): (LuaValue, bool)| {
            if let Some(target) = target_from_luau_or_current(&shared, target)? {
                push_command(&shared, ScriptCommand::SetEnabled { target, enabled });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    entity.set(
        "destroy",
        lua.create_function(move |_, target: Option<LuaValue>| {
            if let Some(target) = target_to_script_target(&shared, target)? {
                push_command(&shared, ScriptCommand::Destroy { target });
            }
            Ok(())
        })?,
    )?;
    globals.set("Entity", entity.clone())?;
    api.set("Entity", entity)?;

    let transform = lua.create_table()?;
    let shared = host.clone();
    transform.set(
        "set_position",
        lua.create_function(move |_, (entity, x, y): (LuaValue, f64, f64)| {
            if let Some(entity_id) = entity_id_from_luau(entity) {
                push_command(&shared, ScriptCommand::SetPosition { entity_id, x, y });
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    transform.set(
        "translate",
        lua.create_function(move |_, (entity, dx, dy): (LuaValue, f64, f64)| {
            if let Some(entity_id) = entity_id_from_luau(entity) {
                push_command(&shared, ScriptCommand::Move { entity_id, dx, dy });
            }
            Ok(())
        })?,
    )?;
    globals.set("Transform2D", transform.clone())?;
    api.set("Transform2D", transform)?;

    let scene = lua.create_table()?;
    let shared = host.clone();
    scene.set(
        "load",
        lua.create_function(move |_, name: String| {
            push_command(&shared, ScriptCommand::LoadScene { name });
            Ok(())
        })?,
    )?;
    scene.set(
        "current",
        lua.create_function(|_, (): ()| Ok(LuaValue::Nil))?,
    )?;
    globals.set("Scene", scene.clone())?;
    api.set("Scene", scene)?;

    let game = lua.create_table()?;
    let shared = host.clone();
    game.set(
        "save_slot",
        lua.create_function(move |_, slot: String| {
            push_command(&shared, ScriptCommand::SaveGame { slot });
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    game.set(
        "load_slot",
        lua.create_function(move |_, slot: String| {
            push_command(&shared, ScriptCommand::LoadGame { slot });
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    game.set(
        "autosave",
        lua.create_function(move |_, (): ()| {
            push_command(
                &shared,
                ScriptCommand::SaveGame {
                    slot: "autosave".to_string(),
                },
            );
            Ok(())
        })?,
    )?;
    globals.set("Game", game.clone())?;
    api.set("Game", game)?;

    let events = lua.create_table()?;
    let shared = host.clone();
    events.set(
        "emit",
        lua.create_function(move |_, (name, payload): (String, Option<LuaValue>)| {
            push_command(
                &shared,
                ScriptCommand::EmitEvent {
                    name,
                    payload: payload.map(luau_value_to_json).unwrap_or(Value::Null),
                },
            );
            Ok(())
        })?,
    )?;
    globals.set("Events", events.clone())?;
    api.set("Events", events)?;

    let assets = lua.create_table()?;
    let assets_root = project_path.clone();
    assets.set(
        "exists",
        lua.create_function(move |_, path: String| Ok(project_file_exists(&assets_root, &path)))?,
    )?;
    assets.set("path", lua.create_function(|_, path: String| Ok(path))?)?;
    globals.set("Assets", assets.clone())?;
    api.set("Assets", assets)?;

    let debug = lua.create_table()?;
    for (name, level) in [("log", "info"), ("warn", "warning"), ("error", "error")] {
        let shared = host.clone();
        debug.set(
            name,
            lua.create_function(move |_, message: LuaValue| {
                push_command(
                    &shared,
                    ScriptCommand::DebugLog {
                        level: level.to_string(),
                        message: luau_debug_string(message),
                    },
                );
                Ok(())
            })?,
        )?;
    }
    let shared = host.clone();
    globals.set(
        "print",
        lua.create_function(move |_, message: LuaValue| {
            push_command(
                &shared,
                ScriptCommand::DebugLog {
                    level: "info".to_string(),
                    message: luau_debug_string(message),
                },
            );
            Ok(())
        })?,
    )?;
    globals.set("Debug", debug.clone())?;
    api.set("Debug", debug)?;

    Ok(())
}

fn register_command<A, F>(
    lua: &Lua,
    globals: &Table,
    api: &Table,
    name: &str,
    host: SharedHostState,
    command: F,
) -> mlua::Result<()>
where
    A: mlua::FromLuaMulti,
    F: Fn(&SharedHostState, A) + Send + 'static,
{
    let function = lua.create_function(move |_, args: A| {
        command(&host, args);
        Ok(())
    })?;
    globals.set(name, function.clone())?;
    api.set(name, function)
}

fn push_self_command(host: &SharedHostState, command: impl FnOnce(u64) -> ScriptCommand) {
    let Ok(mut host) = host.lock() else { return };
    if let Some(id) = host.current_entity_id {
        host.commands.push(command(id));
    }
}

fn push_command(host: &SharedHostState, command: ScriptCommand) {
    if let Ok(mut host) = host.lock() {
        host.commands.push(command);
    }
}

fn install_context_luau_api(lua: &Lua, environment: &Table) -> mlua::Result<()> {
    lua.load(
        r#"
local __mf_tasks = {}
Task = Task or {}

function Task.delay(seconds, callback)
    assert(type(callback) == "function", "Task.delay expects a function")
    local task = {
        remaining = math.max(0, tonumber(seconds) or 0),
        callback = callback,
        cancelled = false,
    }
    table.insert(__mf_tasks, task)
    return task
end

function Task.defer(callback)
    return Task.delay(0, callback)
end

function Task.cancel(task)
    if task then task.cancelled = true end
end

function __miniforge_task_tick(dt)
    local next_tasks = {}
    for _, task in ipairs(__mf_tasks) do
        if not task.cancelled then
            task.remaining = task.remaining - (dt or 0)
            if task.remaining <= 0 then
                task.cancelled = true
                task.callback()
            else
                table.insert(next_tasks, task)
            end
        end
    end
    __mf_tasks = next_tasks
end
"#,
    )
    .set_name("@miniforge_task_bootstrap")
    .set_environment(environment.clone())
    .exec()
}

fn sync_time_table(lua: &Lua, environment: &Table, time: ScriptTimeState) -> mlua::Result<()> {
    let table = lua.create_table()?;
    table.set("delta_time", time.delta_time)?;
    table.set("fixed_delta_time", time.fixed_delta_time)?;
    table.set("time", time.total_time)?;
    table.set("frame", time.frame)?;
    environment.set("Time", table)
}

fn pump_context_tasks(environment: &Table, dt: f64) -> mlua::Result<()> {
    if let LuaValue::Function(function) = environment.get::<LuaValue>("__miniforge_task_tick")? {
        function.call::<()>(dt)?;
    }
    Ok(())
}

fn vector2_table(lua: &Lua, x: f64, y: f64) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("x", x)?;
    table.set("y", y)?;
    Ok(table)
}

fn entity_id_from_luau(value: LuaValue) -> Option<u64> {
    match value {
        LuaValue::Integer(id) if id >= 0 => Some(id as u64),
        LuaValue::Number(id) if id >= 0.0 => Some(id as u64),
        LuaValue::Table(table) => table
            .get::<Option<u64>>("id")
            .ok()
            .flatten()
            .or_else(|| table.get::<Option<u64>>("entity_id").ok().flatten()),
        _ => None,
    }
}

fn target_to_script_target(
    host: &SharedHostState,
    target: Option<LuaValue>,
) -> mlua::Result<Option<ScriptTarget>> {
    Ok(match target {
        None | Some(LuaValue::Nil) => host
            .lock()
            .ok()
            .and_then(|state| state.current_entity_id)
            .map(ScriptTarget::Id),
        Some(LuaValue::Integer(id)) if id >= 0 => Some(ScriptTarget::Id(id as u64)),
        Some(LuaValue::Number(id)) if id >= 0.0 => Some(ScriptTarget::Id(id as u64)),
        Some(LuaValue::String(name)) => Some(ScriptTarget::Name(name.to_str()?.to_string())),
        Some(LuaValue::Table(table)) => table
            .get::<Option<u64>>("id")?
            .map(ScriptTarget::Id)
            .or_else(|| {
                table
                    .get::<Option<String>>("name")
                    .ok()
                    .flatten()
                    .map(ScriptTarget::Name)
            }),
        _ => None,
    })
}

fn target_from_luau_or_current(
    host: &SharedHostState,
    target: LuaValue,
) -> mlua::Result<Option<ScriptTarget>> {
    if matches!(target, LuaValue::Nil) {
        target_to_script_target(host, None)
    } else {
        target_to_script_target(host, Some(target))
    }
}

fn current_script_target(host: &SharedHostState) -> Option<ScriptTarget> {
    host.lock()
        .ok()
        .and_then(|state| state.current_entity_id)
        .map(ScriptTarget::Id)
}

fn input_action_pressed(host: &SharedHostState, action: &str) -> bool {
    let aliases = match action {
        "move_left" | "left" => &["move_left", "left", "A", "Left"][..],
        "move_right" | "right" => &["move_right", "right", "D", "Right"][..],
        "move_up" | "up" => &["move_up", "up", "W", "Up"][..],
        "move_down" | "down" => &["move_down", "down", "S", "Down"][..],
        "jump" => &["jump", "Space"][..],
        "fire" => &["fire", "MouseLeft", "Ctrl"][..],
        "interact" | "use" => &["interact", "use", "E", "e", "Enter"][..],
        "run" | "sprint" => &["run", "sprint", "Shift", "LeftShift", "RightShift"][..],
        "pause" | "menu" => &["pause", "menu", "Escape", "Esc", "Start"][..],
        "enter_vehicle" | "vehicle" => &["enter_vehicle", "vehicle", "F", "f"][..],
        "brake" => &["brake", "Space"][..],
        _ => {
            return host
                .lock()
                .is_ok_and(|state| state.inputs_pressed.contains(action));
        }
    };
    host.lock().is_ok_and(|state| {
        aliases
            .iter()
            .any(|alias| state.inputs_pressed.contains(*alias))
    })
}

fn luau_vec2(value: LuaValue) -> mlua::Result<Option<(f64, f64)>> {
    match value {
        LuaValue::Nil => Ok(None),
        LuaValue::Table(table) => {
            let x = table
                .get::<Option<f64>>("x")?
                .or_else(|| table.get::<Option<f64>>(1).ok().flatten());
            let y = table
                .get::<Option<f64>>("y")?
                .or_else(|| table.get::<Option<f64>>(2).ok().flatten());
            if x.is_none() && y.is_none() {
                return Ok(None);
            }
            Ok(Some((x.unwrap_or_default(), y.unwrap_or_default())))
        }
        _ => Ok(None),
    }
}

fn luau_point_or_entity_position(
    host: &SharedHostState,
    value: LuaValue,
) -> mlua::Result<Option<(f64, f64)>> {
    if let Some(point) = luau_vec2(value.clone())? {
        return Ok(Some(point));
    }
    let LuaValue::Table(table) = value else {
        return Ok(None);
    };
    if let Ok(LuaValue::Table(transform)) = table.get::<LuaValue>("transform")
        && let Ok(position) = transform.get::<LuaValue>("position")
        && let Some(point) = luau_vec2(position)?
    {
        return Ok(Some(point));
    }
    if let Ok(position) = table.get::<LuaValue>("position")
        && let Some(point) = luau_vec2(position)?
    {
        return Ok(Some(point));
    }
    let target = target_from_luau_or_current(host, LuaValue::Table(table))?;
    Ok(host.lock().ok().and_then(|host| {
        target
            .and_then(|target| find_snapshot_entity(&host, &target))
            .map(|entity| (entity.x, entity.y))
    }))
}

fn luau_options_bool(options: Option<&Table>, key: &str, default: bool) -> bool {
    options
        .and_then(|table| table.get::<Option<bool>>(key).ok().flatten())
        .unwrap_or(default)
}

fn luau_options_number(options: Option<&Table>, key: &str, default: f64) -> f64 {
    options
        .and_then(|table| table.get::<Option<f64>>(key).ok().flatten())
        .unwrap_or(default)
}

fn luau_options_string(options: Option<&Table>, key: &str, default: &str) -> String {
    options
        .and_then(|table| table.get::<Option<String>>(key).ok().flatten())
        .unwrap_or_else(|| default.to_string())
}

fn luau_options_vec2(options: Option<&Table>, key: &str, default: (f64, f64)) -> (f64, f64) {
    options
        .and_then(|table| table.get::<LuaValue>(key).ok())
        .and_then(|value| luau_vec2(value).ok().flatten())
        .unwrap_or(default)
}

fn luau_layers_from_options(options: Option<&Table>) -> mlua::Result<Vec<String>> {
    let Some(options) = options else {
        return Ok(Vec::new());
    };
    for key in ["mask", "layers", "layer"] {
        let value = options.get::<LuaValue>(key).unwrap_or(LuaValue::Nil);
        let layers = luau_layer_value(value)?;
        if !layers.is_empty() {
            return Ok(layers);
        }
    }
    Ok(Vec::new())
}

fn luau_tags_from_options(options: Option<&Table>) -> mlua::Result<Vec<String>> {
    let Some(options) = options else {
        return Ok(Vec::new());
    };
    for key in ["tag", "tags"] {
        let value = options.get::<LuaValue>(key).unwrap_or(LuaValue::Nil);
        let tags = luau_layer_value(value)?;
        if !tags.is_empty() {
            return Ok(tags);
        }
    }
    Ok(Vec::new())
}

fn luau_layer_value(value: LuaValue) -> mlua::Result<Vec<String>> {
    Ok(match value {
        LuaValue::Nil => Vec::new(),
        LuaValue::String(value) => vec![value.to_string_lossy()],
        LuaValue::Table(table) => {
            let mut layers = Vec::new();
            for pair in table.sequence_values::<LuaValue>() {
                match pair? {
                    LuaValue::String(value) => layers.push(value.to_string_lossy()),
                    LuaValue::Integer(value) => layers.push(value.to_string()),
                    LuaValue::Number(value) => layers.push(value.to_string()),
                    _ => {}
                }
            }
            layers
        }
        LuaValue::Integer(value) => vec![value.to_string()],
        LuaValue::Number(value) => vec![value.to_string()],
        _ => Vec::new(),
    })
}

fn non_empty_layers(layers: &[String]) -> Option<&[String]> {
    (!layers.is_empty()).then_some(layers)
}

fn raycast_hit_to_luau(lua: &Lua, hit: &RaycastHit) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("entity_id", hit.entity_id)?;
    table.set("id", hit.entity_id)?;
    table.set("entity_name", hit.entity_name.clone())?;
    table.set("name", hit.entity_name.clone())?;
    table.set("point", vector2_table(lua, hit.point.0, hit.point.1)?)?;
    table.set("normal", vector2_table(lua, hit.normal.0, hit.normal.1)?)?;
    table.set("distance", hit.distance)?;
    table.set("layer", hit.layer.clone())?;
    table.set("is_trigger", hit.is_trigger)?;
    Ok(table)
}

fn raycast_hits_to_luau(lua: &Lua, hits: &[RaycastHit]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, hit) in hits.iter().enumerate() {
        table.set(index + 1, raycast_hit_to_luau(lua, hit)?)?;
    }
    Ok(table)
}

fn camera_handle(lua: &Lua, host: SharedHostState) -> mlua::Result<Table> {
    let handle = lua.create_table()?;
    let shared = host.clone();
    handle.set(
        "follow",
        lua.create_function(move |_, (_self, target): (LuaValue, LuaValue)| {
            let follow_target_id = entity_id_from_luau(target)
                .or_else(|| shared.lock().ok().and_then(|state| state.current_entity_id));
            if let (Some(target), Some(follow_target_id)) =
                (current_script_target(&shared), follow_target_id)
            {
                push_command(
                    &shared,
                    ScriptCommand::SetCameraFollow {
                        target,
                        follow_target_id,
                    },
                );
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    handle.set(
        "shake",
        lua.create_function(
            move |_, (_self, duration, amplitude): (LuaValue, f64, f64)| {
                if let Some(target) = current_script_target(&shared) {
                    push_command(
                        &shared,
                        ScriptCommand::SetCameraShake {
                            target,
                            duration,
                            amplitude,
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    let shared = host.clone();
    handle.set(
        "set_zoom",
        lua.create_function(move |_, (_self, zoom): (LuaValue, f64)| {
            if let Some(target) = current_script_target(&shared) {
                push_command(
                    &shared,
                    ScriptCommand::SetComponentValue {
                        target: target.clone(),
                        component: "CameraFollow".to_string(),
                        key: "zoom".to_string(),
                        value: json!(zoom),
                    },
                );
                push_command(
                    &shared,
                    ScriptCommand::SetComponentValue {
                        target,
                        component: "Camera2D".to_string(),
                        key: "zoom".to_string(),
                        value: json!(zoom),
                    },
                );
            }
            Ok(())
        })?,
    )?;
    let shared = host.clone();
    handle.set(
        "set_limits",
        lua.create_function(
            move |_, (_self, min_x, min_y, max_x, max_y): (LuaValue, f64, f64, f64, f64)| {
                if let Some(target) = current_script_target(&shared) {
                    push_command(
                        &shared,
                        ScriptCommand::SetComponentValue {
                            target,
                            component: "Camera2D".to_string(),
                            key: "limits".to_string(),
                            value: json!({
                                "enabled": true,
                                "min_x": min_x,
                                "min_y": min_y,
                                "max_x": max_x,
                                "max_y": max_y,
                            }),
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    let shared = host.clone();
    handle.set(
        "pixel_perfect",
        lua.create_function(
            move |_, (_self, enabled, pixels_per_unit): (LuaValue, bool, Option<f64>)| {
                if let Some(target) = current_script_target(&shared) {
                    push_command(
                        &shared,
                        ScriptCommand::SetCameraPixelPerfect {
                            target,
                            enabled,
                            pixels_per_unit: pixels_per_unit.unwrap_or(16.0),
                        },
                    );
                }
                Ok(())
            },
        )?,
    )?;
    let shared = host.clone();
    handle.set(
        "world_to_screen",
        lua.create_function(move |lua, (_self, world): (LuaValue, LuaValue)| {
            let world = luau_vec2(world)?.unwrap_or_default();
            let camera = shared.lock().map(|host| host.camera).unwrap_or_default();
            vector2_table(
                lua,
                camera.viewport.0 + (world.0 - camera.x) * camera.zoom,
                camera.viewport.1 + (world.1 - camera.y) * camera.zoom,
            )
        })?,
    )?;
    let shared = host;
    handle.set(
        "screen_to_world",
        lua.create_function(move |lua, (_self, screen): (LuaValue, LuaValue)| {
            let screen = luau_vec2(screen)?.unwrap_or_default();
            let camera = shared.lock().map(|host| host.camera).unwrap_or_default();
            vector2_table(
                lua,
                camera.x + (screen.0 - camera.viewport.0) / camera.zoom.max(0.0001),
                camera.y + (screen.1 - camera.viewport.1) / camera.zoom.max(0.0001),
            )
        })?,
    )?;
    Ok(handle)
}

fn animation_target_and_name(
    host: &SharedHostState,
    first: LuaValue,
    second: Option<String>,
) -> mlua::Result<(Option<ScriptTarget>, String)> {
    if let Some(animation) = second {
        return Ok((target_from_luau_or_current(host, first)?, animation));
    }
    match first {
        LuaValue::String(animation) => {
            Ok((current_script_target(host), animation.to_string_lossy()))
        }
        other => Ok((
            target_from_luau_or_current(host, other)?,
            "default".to_string(),
        )),
    }
}

fn find_snapshot_entity<'a>(
    host: &'a ScriptHostState,
    target: &ScriptTarget,
) -> Option<&'a GameObject> {
    match target {
        ScriptTarget::Id(id) => host
            .world_entity_ids
            .get(id)
            .and_then(|index| host.world_entities.get(*index)),
        ScriptTarget::Name(name) => host
            .world_entity_names
            .get(name)
            .and_then(|index| host.world_entities.get(*index)),
    }
}

fn nearby_snapshot_entities(
    host: &mut ScriptHostState,
    origin: (f64, f64),
    radius: f64,
    include_disabled: bool,
    tags: &[String],
    layers: &[String],
) -> Vec<(f64, GameObject)> {
    host.query_stats.nearby_queries = host.query_stats.nearby_queries.saturating_add(1);
    let hits = if include_disabled {
        host.query_stats.nearby_linear_scans =
            host.query_stats.nearby_linear_scans.saturating_add(1);
        nearby_snapshot_entities_linear(host, origin, radius, include_disabled, tags, layers)
    } else {
        host.query_stats.nearby_indexed = host.query_stats.nearby_indexed.saturating_add(1);
        nearby_snapshot_entities_indexed(host, origin, radius, tags, layers)
    };
    host.query_stats.nearby_candidates = host
        .query_stats
        .nearby_candidates
        .saturating_add(hits.len());
    hits
}

fn nearby_snapshot_entities_indexed(
    host: &ScriptHostState,
    origin: (f64, f64),
    radius: f64,
    tags: &[String],
    layers: &[String],
) -> Vec<(f64, GameObject)> {
    let tag_filters = spatial_filter_values(tags);
    let layer_filters = spatial_filter_values(layers);
    let mut seen = BTreeSet::new();
    let mut hits = Vec::new();
    for tag in &tag_filters {
        for layer in &layer_filters {
            for entry in host.spatial_index.query_radius(
                origin.0,
                origin.1,
                radius,
                tag.as_deref(),
                layer.as_deref(),
            ) {
                if !seen.insert(entry.entity_id) {
                    continue;
                }
                if let Some((distance, entity)) =
                    spatial_entry_to_nearby_snapshot(host, &entry, origin, radius, tags, layers)
                {
                    hits.push((distance, entity));
                }
            }
        }
    }
    hits
}

fn spatial_entry_to_nearby_snapshot(
    host: &ScriptHostState,
    entry: &SpatialEntry,
    origin: (f64, f64),
    radius: f64,
    tags: &[String],
    layers: &[String],
) -> Option<(f64, GameObject)> {
    let entity = host
        .world_entity_ids
        .get(&entry.entity_id)
        .and_then(|index| host.world_entities.get(*index))?;
    if !entity.enabled {
        return None;
    }
    if !tags.is_empty() && !tags.iter().any(|tag| tag == &entity.tag) {
        return None;
    }
    if !layers.is_empty() && !layers.iter().any(|layer| layer == &entity.layer) {
        return None;
    }
    let dx = entity.x - origin.0;
    let dy = entity.y - origin.1;
    let distance = (dx * dx + dy * dy).sqrt();
    (distance <= radius).then(|| (distance, entity.clone()))
}

fn spatial_filter_values(values: &[String]) -> Vec<Option<&str>> {
    if values.is_empty() {
        vec![None]
    } else {
        values.iter().map(|value| Some(value.as_str())).collect()
    }
}

fn nearby_snapshot_entities_linear(
    host: &ScriptHostState,
    origin: (f64, f64),
    radius: f64,
    include_disabled: bool,
    tags: &[String],
    layers: &[String],
) -> Vec<(f64, GameObject)> {
    host.world_entities
        .iter()
        .filter(|entity| include_disabled || entity.enabled)
        .filter(|entity| tags.is_empty() || tags.iter().any(|tag| tag == &entity.tag))
        .filter(|entity| layers.is_empty() || layers.iter().any(|layer| layer == &entity.layer))
        .filter_map(|entity| {
            let dx = entity.x - origin.0;
            let dy = entity.y - origin.1;
            let distance = (dx * dx + dy * dy).sqrt();
            (distance <= radius).then(|| (distance, entity.clone()))
        })
        .collect()
}

fn tilemap_cell(entity: &GameObject, layer: &str, x: usize, y: usize) -> Option<i64> {
    let tilemap = entity.get_component("Tilemap2D")?;
    let width = tilemap.get_usize("width", 0);
    let height = tilemap.get_usize("height", 0);
    if width == 0 || height == 0 || x >= width || y >= height {
        return None;
    }
    let index = y * width + x;
    tilemap
        .get("layers")
        .and_then(Value::as_array)?
        .iter()
        .find(|entry| entry.get("name").and_then(Value::as_str) == Some(layer))
        .and_then(|entry| entry.get("tiles"))
        .and_then(Value::as_array)
        .and_then(|tiles| tiles.get(index))
        .and_then(Value::as_i64)
        .or(Some(0))
}

fn luau_debug_string(value: LuaValue) -> String {
    match value {
        LuaValue::Nil => "nil".to_string(),
        LuaValue::Boolean(value) => value.to_string(),
        LuaValue::Integer(value) => value.to_string(),
        LuaValue::Number(value) => value.to_string(),
        LuaValue::String(value) => value.to_string_lossy(),
        LuaValue::Vector(_) => "<vector>".to_string(),
        LuaValue::Table(_) => "<table>".to_string(),
        LuaValue::Function(_) => "<function>".to_string(),
        LuaValue::Thread(_) => "<thread>".to_string(),
        LuaValue::UserData(_) | LuaValue::LightUserData(_) => "<userdata>".to_string(),
        LuaValue::Buffer(_) => "<buffer>".to_string(),
        LuaValue::Error(error) => error.to_string(),
        LuaValue::Other(_) => "<value>".to_string(),
    }
}

fn project_file_exists(project_path: &Path, value: &str) -> bool {
    let raw = Path::new(value.trim());
    if raw.is_absolute()
        || raw
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return false;
    }
    let candidate = normalize_path(project_path.join(raw));
    candidate.starts_with(normalize_path(project_path)) && candidate.is_file()
}

fn runtime_save_path(project_path: &Path, slot: &str) -> Result<PathBuf, String> {
    let slot = sanitize_save_slot(slot);
    if slot.is_empty() {
        return Err("save slot must contain at least one safe character".to_string());
    }
    let save_dir = project_path.join("saves").join("profile");
    fs::create_dir_all(&save_dir).map_err(|error| {
        format!(
            "could not create save directory {}: {error}",
            save_dir.display()
        )
    })?;
    let candidate = normalize_path(save_dir.join(format!("{slot}.json")));
    let root = normalize_path(project_path);
    if !candidate.starts_with(root) {
        return Err("save slot resolved outside project".to_string());
    }
    Ok(candidate)
}

fn sanitize_save_slot(slot: &str) -> String {
    slot.chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                Some(character)
            } else if character.is_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .take(64)
        .collect()
}

fn resolve_targets(entities: &[GameObject], target: &ScriptTarget) -> Vec<u64> {
    match target {
        ScriptTarget::Id(id) => entities
            .iter()
            .any(|entity| entity.id == *id)
            .then_some(*id)
            .into_iter()
            .collect(),
        ScriptTarget::Name(name) => entities
            .iter()
            .filter(|entity| entity.name == *name)
            .map(|entity| entity.id)
            .collect(),
    }
}

fn for_each_target(
    report_entities: &mut [GameObject],
    target: &ScriptTarget,
    report: &mut LuauRunReport,
    mut action: impl FnMut(&mut GameObject) -> bool,
) {
    for id in resolve_targets(report_entities, target) {
        if let Some(entity) = report_entities.iter_mut().find(|entity| entity.id == id)
            && action(entity)
        {
            report.commands_applied += 1;
        }
    }
}

fn ensure_component(entity: &mut GameObject, component_type: &str) -> bool {
    if entity.get_component(component_type).is_some() {
        return true;
    }
    GameAPI::add_component(entity, component_type, None).is_some()
}

fn configure_spawned_entity(entity: &mut GameObject, data: Value) {
    let Some(map) = data.as_object() else {
        return;
    };
    if let Some(tag) = map.get("tag").and_then(Value::as_str) {
        entity.tag = tag.to_string();
    }
    if let Some(layer) = map.get("layer").and_then(Value::as_str) {
        entity.layer = layer.to_string();
    }
    if let Some(script) = map.get("script").and_then(Value::as_str) {
        entity.script = Some(script.to_string());
    }
    if let Some(width) = map.get("width").and_then(Value::as_f64) {
        entity.width = width.max(0.01);
    }
    if let Some(height) = map.get("height").and_then(Value::as_f64) {
        entity.height = height.max(0.01);
    }
    if let Some(components) = map.get("components").and_then(Value::as_array) {
        for component in components {
            if let Some(component_type) = component
                .get("component_type")
                .or_else(|| component.get("type"))
                .and_then(Value::as_str)
            {
                let _ = GameAPI::add_component(entity, component_type, Some(component.clone()));
            }
        }
    }
    entity.sync_to_components();
}

fn ensure_sprite(entity: &mut GameObject) {
    if entity.get_component("SpriteRenderer").is_none() {
        let _ = GameAPI::add_component(entity, "SpriteRenderer", None);
    }
}

fn set_sprite(entity: &mut GameObject, path: &str) -> bool {
    ensure_sprite(entity);
    let name = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_string();
    entity.sprite_name = Some(name.clone());
    let Some(sprite) = entity.get_component_mut("SpriteRenderer") else {
        return false;
    };
    sprite.set("sprite_name", json!(name));
    sprite.set("sprite_path", json!(path));
    sprite.set("source_asset", json!(path));
    sprite.set("visible", json!(true));
    true
}

fn set_sprite_animation(entity: &mut GameObject, frames: &str, animation: &str) -> bool {
    ensure_sprite(entity);
    let Some(sprite) = entity.get_component_mut("SpriteRenderer") else {
        return false;
    };
    sprite.set("sprite_frames", json!(frames));
    sprite.set("animation", json!(animation));
    sprite.set("active_animation", json!(animation));
    sprite.set("use_2d_animation", json!(true));
    true
}

fn set_sprite_flip(entity: &mut GameObject, flip_x: bool, flip_y: bool) -> bool {
    ensure_sprite(entity);
    let Some(sprite) = entity.get_component_mut("SpriteRenderer") else {
        return false;
    };
    sprite.set("flip_x", json!(flip_x));
    sprite.set("flip_y", json!(flip_y));
    true
}

fn set_entity_animation(entity: &mut GameObject, animation: &str) -> bool {
    let mut changed = false;
    ensure_sprite(entity);
    if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
        sprite.set("animation", json!(animation));
        sprite.set("active_animation", json!(animation));
        changed = true;
    }
    if ensure_component(entity, "Animator")
        && let Some(animator) = entity.get_component_mut("Animator")
    {
        animator.set("current_state", json!(animation));
        animator.set_f64("normalized_time", 0.0);
        animator.set("paused", json!(false));
        changed = true;
    }
    if ensure_component(entity, "AnimationPlayer")
        && let Some(player) = entity.get_component_mut("AnimationPlayer")
    {
        player.set("current", json!(animation));
        player.set("playing", json!(true));
        changed = true;
    }
    if ensure_component(entity, "AnimatedSprite")
        && let Some(animated) = entity.get_component_mut("AnimatedSprite")
    {
        animated.set("animation", json!(animation));
        animated.set("playing", json!(true));
        animated.set("frame", json!(0));
        changed = true;
    }
    changed
}

fn set_animation_parameter(entity: &mut GameObject, key: &str, value: Value) -> bool {
    let mut changed = false;
    for component_name in ["Animator", "AnimationPlayer", "Animator2D"] {
        let Some(component) = entity.get_component_mut(component_name) else {
            continue;
        };
        let mut parameters = component
            .get("parameters")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        parameters.insert(key.to_string(), value.clone());
        component.set("parameters", Value::Object(parameters));
        changed = true;
    }
    if let Some(blackboard) = entity.get_component_mut("Blackboard") {
        blackboard.blackboard_set(key, value);
        changed = true;
    }
    changed
}

fn set_tilemap_cell(entity: &mut GameObject, layer: &str, x: usize, y: usize, tile: i64) -> bool {
    let Some(tilemap) = entity.get_component_mut("Tilemap2D") else {
        return false;
    };
    let width = tilemap.get_usize("width", 0);
    let height = tilemap.get_usize("height", 0);
    if width == 0 || height == 0 || x >= width || y >= height {
        return false;
    }
    let index = y * width + x;
    let mut layers = tilemap
        .get("layers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut changed = false;
    for entry in &mut layers {
        if entry.get("name").and_then(Value::as_str) != Some(layer) {
            continue;
        }
        if let Some(map) = entry.as_object_mut() {
            let mut tiles = map
                .get("tiles")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if tiles.len() < width * height {
                tiles.resize(width * height, json!(0));
            }
            tiles[index] = json!(tile);
            map.insert("tiles".to_string(), Value::Array(tiles));
            changed = true;
        }
    }
    if !changed {
        let mut tiles = vec![json!(0); width * height];
        tiles[index] = json!(tile);
        layers.push(json!({
            "name": layer,
            "visible": true,
            "collision": false,
            "navigation": false,
            "tiles": tiles,
        }));
        changed = true;
    }
    tilemap.set("layers", Value::Array(layers));
    let mut dirty = tilemap
        .get("dirty_chunks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let chunk_w = tilemap.get_usize("chunk_width", 16).max(1);
    let chunk_h = tilemap.get_usize("chunk_height", 16).max(1);
    dirty.push(json!({"x": x / chunk_w, "y": y / chunk_h}));
    tilemap.set("dirty_chunks", Value::Array(dirty));
    changed
}

fn entity_property_value(entity: &GameObject, property_path: &str) -> Option<f64> {
    match property_path {
        "x" | "position.x" | "transform.position.x" => Some(entity.x),
        "y" | "position.y" | "transform.position.y" => Some(entity.y),
        "rotation" | "transform.rotation" => Some(entity.rotation),
        "scale_x" | "transform.scale.x" => Some(entity.scale_x),
        "scale_y" | "transform.scale.y" => Some(entity.scale_y),
        "width" | "size.x" | "transform.size.x" => Some(entity.width),
        "height" | "size.y" | "transform.size.y" => Some(entity.height),
        _ => {
            let (component, key) = property_path.split_once('.')?;
            entity
                .get_component(component)
                .and_then(|component| component.get(key))
                .and_then(Value::as_f64)
        }
    }
}

fn call_event_function(
    lua: &Lua,
    function: Function,
    instance: Option<Table>,
    event: &LuauScriptEvent,
) -> mlua::Result<()> {
    match (instance, event) {
        (
            Some(instance),
            LuauScriptEvent::Create | LuauScriptEvent::Ready | LuauScriptEvent::Destroy,
        ) => function.call::<()>((instance,)),
        (None, LuauScriptEvent::Create | LuauScriptEvent::Ready | LuauScriptEvent::Destroy) => {
            function.call::<()>(())
        }
        (Some(instance), LuauScriptEvent::Update(dt) | LuauScriptEvent::FixedUpdate(dt)) => {
            function.call::<()>((instance, *dt))
        }
        (None, LuauScriptEvent::Update(dt) | LuauScriptEvent::FixedUpdate(dt)) => {
            function.call::<()>(*dt)
        }
        (
            Some(instance),
            LuauScriptEvent::KeyDown(value)
            | LuauScriptEvent::CollisionEnter(value)
            | LuauScriptEvent::CollisionExit(value),
        ) => function.call::<()>((instance, value.clone())),
        (
            None,
            LuauScriptEvent::KeyDown(value)
            | LuauScriptEvent::CollisionEnter(value)
            | LuauScriptEvent::CollisionExit(value),
        ) => function.call::<()>(value.clone()),
        (Some(instance), LuauScriptEvent::Custom { name, payload }) => {
            function.call::<()>((instance, name.clone(), json_to_luau(lua, payload)?))
        }
        (None, LuauScriptEvent::Custom { name, payload }) => {
            function.call::<()>((name.clone(), json_to_luau(lua, payload)?))
        }
    }
}

fn entity_to_luau(lua: &Lua, entity: &GameObject) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("id", entity.id)?;
    table.set("name", entity.name.clone())?;
    table.set("tag", entity.tag.clone())?;
    table.set("layer", entity.layer.clone())?;
    table.set("enabled", entity.enabled)?;
    table.set("visible", entity.visible)?;
    table.set("transform", transform_to_luau(lua, entity)?)?;
    Ok(table)
}

fn entity_to_luau_with_distance(
    lua: &Lua,
    entity: &GameObject,
    distance: f64,
) -> mlua::Result<Table> {
    let table = entity_to_luau(lua, entity)?;
    table.set("distance", distance)?;
    Ok(table)
}

fn transform_to_luau(lua: &Lua, entity: &GameObject) -> mlua::Result<Table> {
    let transform = lua.create_table()?;
    transform.set("position", vector2_table(lua, entity.x, entity.y)?)?;
    transform.set("rotation", entity.rotation)?;
    transform.set("scale", vector2_table(lua, entity.scale_x, entity.scale_y)?)?;
    transform.set("size", vector2_table(lua, entity.width, entity.height)?)?;
    Ok(transform)
}

fn sync_entity_proxy_from_instance(instance: &Table, entity: &mut GameObject) -> mlua::Result<()> {
    let LuaValue::Table(proxy) = instance.get::<LuaValue>("entity")? else {
        return Ok(());
    };
    if let Ok(tag) = proxy.get::<String>("tag") {
        entity.tag = tag;
    }
    if let Ok(layer) = proxy.get::<String>("layer") {
        entity.layer = layer;
    }
    if let Ok(enabled) = proxy.get::<bool>("enabled") {
        entity.enabled = enabled;
        entity.active = enabled;
    }
    if let Ok(visible) = proxy.get::<bool>("visible") {
        entity.visible = visible;
    }
    if let LuaValue::Table(transform) = proxy.get::<LuaValue>("transform")? {
        if let LuaValue::Table(position) = transform.get::<LuaValue>("position")? {
            let x = position.get::<f64>("x").unwrap_or(entity.x);
            let y = position.get::<f64>("y").unwrap_or(entity.y);
            GameAPI::set_position(entity, x, y);
        }
        if let Ok(rotation) = transform.get::<f64>("rotation") {
            GameAPI::set_rotation(entity, rotation);
        }
        if let LuaValue::Table(scale) = transform.get::<LuaValue>("scale")? {
            let x = scale.get::<f64>("x").unwrap_or(entity.scale_x);
            let y = scale.get::<f64>("y").unwrap_or(entity.scale_y);
            GameAPI::set_scale(entity, x.max(0.01), y.max(0.01));
        }
        if let LuaValue::Table(size) = transform.get::<LuaValue>("size")? {
            let w = size.get::<f64>("x").unwrap_or(entity.width);
            let h = size.get::<f64>("y").unwrap_or(entity.height);
            GameAPI::set_size(entity, w.max(0.01), h.max(0.01));
        }
    }
    entity.sync_to_components();
    Ok(())
}

fn apply_public_variables(lua: &Lua, instance: &Table, variables: &Value) -> mlua::Result<()> {
    let Some(values) = variables.as_object() else {
        return Ok(());
    };
    for (key, value) in values {
        if key.starts_with('_') {
            continue;
        }
        instance.set(key.as_str(), json_to_luau(lua, value)?)?;
    }
    Ok(())
}

fn exported_variables_from_instance(instance: &Table) -> Value {
    let mut values = serde_json::Map::new();
    if let Ok(LuaValue::Table(exports)) = instance.get::<LuaValue>("exports") {
        merge_export_table(&mut values, exports);
    }
    for pair in instance.pairs::<LuaValue, LuaValue>() {
        let Ok((LuaValue::String(key), value)) = pair else {
            continue;
        };
        let key = key.to_string_lossy();
        if key.starts_with('_') || matches!(key.as_str(), "entity" | "exports") {
            continue;
        }
        if matches!(
            value,
            LuaValue::Function(_)
                | LuaValue::Thread(_)
                | LuaValue::UserData(_)
                | LuaValue::LightUserData(_)
        ) {
            continue;
        }
        let value = luau_value_to_json(value);
        if !matches!(value, Value::String(ref text) if text == "<unsupported-luau-value>") {
            values.insert(key, value);
        }
    }
    Value::Object(values)
}

fn merge_export_table(values: &mut serde_json::Map<String, Value>, table: Table) {
    for pair in table.pairs::<LuaValue, LuaValue>() {
        let Ok((LuaValue::String(key), value)) = pair else {
            continue;
        };
        let key = key.to_string_lossy();
        if !key.starts_with('_') {
            values.insert(key, luau_value_to_json(value));
        }
    }
}

fn sync_public_variables_to_entity(entity: &mut GameObject, script_path: &Path, variables: Value) {
    if variables.as_object().is_none_or(serde_json::Map::is_empty) {
        return;
    }
    let script_path = normalize_path(script_path);
    if let Some(component) = entity.get_component_mut("ScriptComponent") {
        let component_path_matches = component
            .get("path")
            .and_then(Value::as_str)
            .is_none_or(|path| script_path_matches(path, &script_path));
        if component_path_matches {
            component.set("public_variables", variables.clone());
        }
        if let Some(Value::Array(scripts)) = component.data.get_mut("scripts") {
            for script in scripts {
                if script_reference_matches(script, &script_path)
                    && let Some(map) = script.as_object_mut()
                {
                    map.insert("public_variables".to_string(), variables.clone());
                }
            }
        }
    }
    for script in &mut entity.scripts {
        if script_reference_matches(script, &script_path)
            && let Some(map) = script.as_object_mut()
        {
            map.insert("public_variables".to_string(), variables.clone());
        }
    }
}

fn component_public_variables(component: &crate::engine::component::Component) -> Value {
    component
        .get("public_variables")
        .cloned()
        .unwrap_or(Value::Null)
}

fn script_component_public_variables(entity: &GameObject, script: &str) -> Value {
    entity
        .get_component("ScriptComponent")
        .filter(|component| {
            component
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| path == script)
        })
        .map(component_public_variables)
        .unwrap_or(Value::Null)
}

fn script_reference_matches(value: &Value, script_path: &Path) -> bool {
    let Some(path) = value.as_str().or_else(|| {
        ["path", "script", "name"]
            .iter()
            .find_map(|key| value.get(key).and_then(Value::as_str))
    }) else {
        return false;
    };
    script_path_matches(path, script_path)
}

fn script_path_matches(reference: &str, script_path: &Path) -> bool {
    let raw = Path::new(reference);
    normalize_path(raw) == script_path
        || script_path.ends_with(raw)
        || raw
            .file_name()
            .is_some_and(|name| Some(name) == script_path.file_name())
}

fn format_luau_error(path: &Path, event: &LuauScriptEvent, error: mlua::Error) -> String {
    let event_name = event.function_names().first().copied().unwrap_or("script");
    let message = error.to_string();
    if message.contains(&path.to_string_lossy().to_string()) {
        format!("{event_name}: {message}")
    } else {
        format!("{}::{event_name}: {message}", path.display())
    }
}

pub fn parse_luau_source_location(message: &str) -> (Option<usize>, Option<usize>) {
    let bytes = message.as_bytes();
    let mut starts = vec![0];
    starts.extend(
        bytes
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (*byte == b':').then_some(index + 1)),
    );
    for start in starts {
        let line_end = bytes[start..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map(|offset| start + offset)
            .unwrap_or(bytes.len());
        if line_end == start || bytes.get(line_end) != Some(&b':') {
            continue;
        }
        let Ok(line) = message[start..line_end].parse::<usize>() else {
            continue;
        };
        if line == 0 {
            continue;
        }
        let column_start = line_end + 1;
        let column_end = bytes[column_start..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map(|offset| column_start + offset)
            .unwrap_or(bytes.len());
        let column = if column_end > column_start && bytes.get(column_end) == Some(&b':') {
            message[column_start..column_end]
                .parse::<usize>()
                .ok()
                .filter(|column| *column > 0)
        } else {
            None
        };
        return (Some(line), column);
    }

    let line = number_after_marker(message, "line ");
    let column = number_after_marker(message, "column ");
    (line, column)
}

fn number_after_marker(message: &str, marker: &str) -> Option<usize> {
    let start = message.find(marker)? + marker.len();
    let digits = message[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn luau_source_diagnostic(name: &str, error: mlua::Error) -> LuauSourceDiagnostic {
    let (raw_message, incomplete_input) = match error {
        mlua::Error::SyntaxError {
            message,
            incomplete_input,
        } => (message, incomplete_input),
        error => (error.to_string(), false),
    };
    let message = raw_message
        .strip_prefix("syntax error: ")
        .unwrap_or(&raw_message)
        .trim();
    let (line, column) = parse_luau_source_location(message);
    let message = strip_leading_luau_location(message).to_string();
    LuauSourceDiagnostic {
        source: name.to_string(),
        line,
        column,
        message,
        incomplete_input,
    }
}

fn strip_leading_luau_location(message: &str) -> &str {
    let message = message.strip_prefix(':').unwrap_or(message);
    let bytes = message.as_bytes();
    let mut cursor = bytes
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .unwrap_or(bytes.len());
    if cursor == 0 || bytes.get(cursor) != Some(&b':') {
        return message;
    }
    cursor += 1;
    let column_end = bytes[cursor..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .map(|offset| cursor + offset)
        .unwrap_or(bytes.len());
    if column_end > cursor && bytes.get(column_end) == Some(&b':') {
        cursor = column_end + 1;
    }
    message[cursor..].trim_start()
}

fn luau_compiler() -> Compiler {
    Compiler::new().set_debug_level(2).set_optimization_level(1)
}

fn luau_value_to_json(value: LuaValue) -> Value {
    luau_value_to_json_at_depth(value, 0)
}

fn luau_value_to_json_at_depth(value: LuaValue, depth: usize) -> Value {
    if depth >= 32 {
        return json!("<maximum-luau-table-depth>");
    }
    match value {
        LuaValue::Nil => Value::Null,
        LuaValue::Boolean(value) => json!(value),
        LuaValue::Integer(value) => json!(value),
        LuaValue::Number(value) => json!(value),
        LuaValue::Vector(value) => json!([value.x(), value.y(), value.z()]),
        LuaValue::String(value) => json!(value.to_string_lossy()),
        LuaValue::Table(table) => {
            let mut entries = Vec::new();
            for pair in table.pairs::<LuaValue, LuaValue>() {
                let Ok((key, value)) = pair else { continue };
                entries.push((key, luau_value_to_json_at_depth(value, depth + 1)));
            }
            let is_array = !entries.is_empty()
                && entries
                    .iter()
                    .all(|(key, _)| matches!(key, LuaValue::Integer(index) if *index > 0));
            if is_array {
                let mut indexed = entries
                    .into_iter()
                    .filter_map(|(key, value)| match key {
                        LuaValue::Integer(index) => Some((index, value)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                indexed.sort_by_key(|(index, _)| *index);
                if indexed
                    .iter()
                    .enumerate()
                    .all(|(offset, (index, _))| *index == offset as i64 + 1)
                {
                    Value::Array(indexed.into_iter().map(|(_, value)| value).collect())
                } else {
                    Value::Object(
                        indexed
                            .into_iter()
                            .map(|(index, value)| (index.to_string(), value))
                            .collect(),
                    )
                }
            } else {
                Value::Object(
                    entries
                        .into_iter()
                        .filter_map(|(key, value)| {
                            let key = match key {
                                LuaValue::String(key) => key.to_str().ok()?.to_string(),
                                LuaValue::Integer(key) => key.to_string(),
                                LuaValue::Number(key) => key.to_string(),
                                _ => return None,
                            };
                            Some((key, value))
                        })
                        .collect(),
                )
            }
        }
        LuaValue::Buffer(_) => json!("<buffer>"),
        _ => json!("<unsupported-luau-value>"),
    }
}

fn json_to_luau(lua: &Lua, value: &Value) -> mlua::Result<LuaValue> {
    Ok(match value {
        Value::Null => LuaValue::Nil,
        Value::Bool(value) => LuaValue::Boolean(*value),
        Value::Number(value) => value
            .as_i64()
            .map(LuaValue::Integer)
            .unwrap_or_else(|| LuaValue::Number(value.as_f64().unwrap_or_default())),
        Value::String(value) => LuaValue::String(lua.create_string(value)?),
        Value::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.set(index + 1, json_to_luau(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
        Value::Object(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                table.set(key.as_str(), json_to_luau(lua, value)?)?;
            }
            LuaValue::Table(table)
        }
    })
}

fn is_luau_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("luau") || ext.eq_ignore_ascii_case("lua"))
}

fn is_luau_debug_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn is_reload_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref()
        .canonicalize()
        .unwrap_or_else(|_| path.as_ref().to_path_buf())
}

fn trace_script_lines(path: &Path) -> Vec<ScriptTraceEntry> {
    let Ok(source) = fs::read_to_string(path) else {
        return Vec::new();
    };
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim_start();
            let function = line
                .strip_prefix("function ")
                .or_else(|| line.strip_prefix("local function "))
                .and_then(|function| function.split(['(', ' ']).next())
                .or_else(|| {
                    line.split_once("= function")
                        .map(|(function, _)| function.trim())
                })?
                .trim();
            (!function.is_empty()).then(|| ScriptTraceEntry {
                path: path.to_path_buf(),
                line: index + 1,
                function: function.to_string(),
            })
        })
        .collect()
}

fn script_handler_line(path: &Path, function_name: &str) -> Option<usize> {
    trace_script_lines(path)
        .into_iter()
        .find(|entry| {
            entry.function == function_name
                || entry.function.ends_with(&format!(":{function_name}"))
                || entry.function.ends_with(&format!(".{function_name}"))
        })
        .map(|entry| entry.line)
}

fn breakpoint_matches(
    breakpoint: &ScriptBreakpoint,
    path: &Path,
    function_name: &str,
    source_line: Option<usize>,
) -> bool {
    if !breakpoint.enabled {
        return false;
    }
    let breakpoint_path = breakpoint.path.replace('\\', "/");
    let script_path = path.to_string_lossy().replace('\\', "/");
    if script_path != breakpoint_path && !script_path.ends_with(&breakpoint_path) {
        return false;
    }
    if let Some(expected_function) = breakpoint.function.as_deref()
        && expected_function != function_name
        && !expected_function.ends_with(&format!(":{function_name}"))
        && !expected_function.ends_with(&format!(".{function_name}"))
    {
        return false;
    }
    if let Some(expected_line) = breakpoint.line
        && source_line != Some(expected_line)
    {
        return false;
    }
    true
}

fn event_debug_name(event: &LuauScriptEvent) -> &'static str {
    match event {
        LuauScriptEvent::Create => "create",
        LuauScriptEvent::Ready => "ready",
        LuauScriptEvent::Update(_) => "update",
        LuauScriptEvent::FixedUpdate(_) => "fixed_update",
        LuauScriptEvent::KeyDown(_) => "key_down",
        LuauScriptEvent::CollisionEnter(_) => "collision_enter",
        LuauScriptEvent::CollisionExit(_) => "collision_exit",
        LuauScriptEvent::Destroy => "destroy",
        LuauScriptEvent::Custom { .. } => "custom",
    }
}

fn event_debug_context(event: &LuauScriptEvent) -> Value {
    match event {
        LuauScriptEvent::Update(dt) | LuauScriptEvent::FixedUpdate(dt) => {
            json!({"name": event_debug_name(event), "dt": dt})
        }
        LuauScriptEvent::KeyDown(key) => json!({"name": "key_down", "key": key}),
        LuauScriptEvent::CollisionEnter(other) => {
            json!({"name": "collision_enter", "other": other})
        }
        LuauScriptEvent::CollisionExit(other) => {
            json!({"name": "collision_exit", "other": other})
        }
        LuauScriptEvent::Custom { name, payload } => {
            json!({"name": name, "payload": payload})
        }
        _ => json!({"name": event_debug_name(event)}),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{
        EntityScriptCall, ScriptAttachment, ScriptHostState, ScriptSchedulerConfig,
        ScriptSimulationClass, ScriptTarget, compare_update_calls, find_snapshot_entity,
        nearby_snapshot_entities, script_update_policy,
    };
    use crate::engine::component::default_component;
    use crate::entities::game_object::GameObject;

    fn call(entity_id: u64, x: f64, priority: i64) -> EntityScriptCall {
        EntityScriptCall {
            entity_id,
            entity_name: format!("Entity{entity_id}"),
            x,
            y: 0.0,
            update_policy: super::ScriptUpdatePolicy {
                priority,
                ..Default::default()
            },
            scripts: vec![ScriptAttachment {
                path: PathBuf::from("scripts/test.luau"),
                public_variables: json!(null),
            }],
        }
    }

    #[test]
    fn open_world_policy_assigns_gta_like_defaults() {
        let mut player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
        player.tag = "Player".to_string();
        let player_policy = script_update_policy(&player, true);
        assert!(player_policy.always_update);
        assert_eq!(player_policy.priority, 180);
        assert_eq!(
            player_policy.simulation_class,
            ScriptSimulationClass::Critical
        );

        let mut pedestrian = GameObject::new(8.0, 0.0, Some("NPC_01".to_string()));
        pedestrian.tag = "NPC".to_string();
        let pedestrian_policy = script_update_policy(&pedestrian, true);
        assert_eq!(
            pedestrian_policy.simulation_class,
            ScriptSimulationClass::Pedestrian
        );
        assert_eq!(pedestrian_policy.priority, 20);
        assert_eq!(pedestrian_policy.update_interval, 0.3);
        assert_eq!(pedestrian_policy.max_distance, Some(36.0));

        let mut police = GameObject::new(32.0, 0.0, Some("PoliceCar_01".to_string()));
        police.tag = "Police".to_string();
        let police_policy = script_update_policy(&police, true);
        assert_eq!(
            police_policy.simulation_class,
            ScriptSimulationClass::Police
        );
        assert_eq!(police_policy.priority, 90);
        assert_eq!(police_policy.max_distance, Some(80.0));
    }

    #[test]
    fn explicit_script_schedule_keeps_timing_but_gets_class_priority_floor() {
        let mut pedestrian = GameObject::new(8.0, 0.0, Some("NPC_02".to_string()));
        pedestrian.tag = "NPC".to_string();
        let mut schedule = default_component("ScriptSchedule").unwrap();
        schedule.set_f64("update_interval", 0.05);
        schedule.set("priority", json!(7));
        pedestrian.add_component(schedule);

        let policy = script_update_policy(&pedestrian, true);

        assert_eq!(policy.update_interval, 0.05);
        assert_eq!(policy.priority, 20);
        assert_eq!(policy.simulation_class, ScriptSimulationClass::Pedestrian);
    }

    #[test]
    fn update_sort_keeps_priority_then_prefers_nearby_scripts() {
        let config = ScriptSchedulerConfig {
            prioritize_by_distance: true,
            ..Default::default()
        };
        let near = call(1, 2.0, 10);
        let far = call(2, 80.0, 10);
        assert_eq!(
            compare_update_calls(&near, &far, Some((0.0, 0.0)), config),
            std::cmp::Ordering::Less
        );

        let high_priority_far = call(3, 200.0, 100);
        assert_eq!(
            compare_update_calls(&near, &high_priority_far, Some((0.0, 0.0)), config),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn script_snapshot_indexes_id_name_and_tag_lookups() {
        let player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
        let mut contact = GameObject::new(4.0, 0.0, Some("MissionContact".to_string()));
        contact.tag = "Contact".to_string();
        let contact_id = contact.id;

        let mut host = ScriptHostState::default();
        host.replace_world_entities(&[player, contact]);

        assert_eq!(
            find_snapshot_entity(&host, &ScriptTarget::Id(contact_id))
                .map(|entity| entity.name.as_str()),
            Some("MissionContact")
        );
        assert_eq!(
            find_snapshot_entity(&host, &ScriptTarget::Name("MissionContact".to_string()))
                .map(|entity| entity.id),
            Some(contact_id)
        );
        assert_eq!(host.world_entity_tags["Contact"].len(), 1);
    }

    #[test]
    fn nearby_snapshot_entities_uses_spatial_index_for_open_world_queries() {
        let player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
        let mut police = GameObject::new(3.0, 0.0, Some("PoliceCar_01".to_string()));
        police.tag = "Police".to_string();
        let mut pedestrian = GameObject::new(18.0, 0.0, Some("NPC_01".to_string()));
        pedestrian.tag = "NPC".to_string();

        let mut host = ScriptHostState::default();
        host.replace_world_entities(&[player, police, pedestrian]);

        let hits = nearby_snapshot_entities(
            &mut host,
            (0.0, 0.0),
            5.0,
            false,
            &["Police".to_string()],
            &[],
        );

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1.name, "PoliceCar_01");
        assert_eq!(host.query_stats.nearby_queries, 1);
        assert_eq!(host.query_stats.nearby_indexed, 1);
        assert_eq!(host.query_stats.nearby_linear_scans, 0);
        assert_eq!(host.query_stats.nearby_candidates, 1);
    }

    #[test]
    fn nearby_snapshot_entities_keeps_disabled_fallback_compatible() {
        let player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
        let mut pickup = GameObject::new(1.0, 0.0, Some("PickupCash".to_string()));
        pickup.tag = "Collectible".to_string();
        pickup.enabled = false;

        let mut host = ScriptHostState::default();
        host.replace_world_entities(&[player, pickup]);

        let hits = nearby_snapshot_entities(
            &mut host,
            (0.0, 0.0),
            2.0,
            true,
            &["Collectible".to_string()],
            &[],
        );

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1.name, "PickupCash");
        assert_eq!(host.query_stats.nearby_indexed, 0);
        assert_eq!(host.query_stats.nearby_linear_scans, 1);
    }

    #[test]
    fn source_validation_returns_editor_ready_locations() {
        let diagnostics = super::LuauScriptRuntime::validate_source_diagnostics(
            "local ok = true\nfunction on_update(\nend",
            "Broken.luau",
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].source, "Broken.luau");
        assert!(diagnostics[0].line.is_some());
        assert!(!diagnostics[0].message.is_empty());
        assert_eq!(
            super::parse_luau_source_location("scripts/Player.luau:17:4: bad value"),
            (Some(17), Some(4))
        );
    }
}
