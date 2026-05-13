use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rhai::{AST, Engine, FLOAT, INT, Scope};

use crate::engine::game_api::GameAPI;
use crate::entities::game_object::GameObject;

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
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RhaiRunReport {
    pub scripts_run: usize,
    pub commands_applied: usize,
    pub spawned: Vec<u64>,
    pub destroyed: Vec<u64>,
    pub sounds: Vec<String>,
    pub scene_requests: Vec<String>,
    pub ui_updates: usize,
    pub errors: Vec<String>,
}

impl RhaiRunReport {
    pub fn merge(&mut self, other: RhaiRunReport) {
        self.scripts_run += other.scripts_run;
        self.commands_applied += other.commands_applied;
        self.spawned.extend(other.spawned);
        self.destroyed.extend(other.destroyed);
        self.sounds.extend(other.sounds);
        self.scene_requests.extend(other.scene_requests);
        self.ui_updates += other.ui_updates;
        self.errors.extend(other.errors);
    }
}

#[derive(Debug, Clone)]
enum RhaiScriptEvent {
    Start,
    Update(f64),
    KeyDown(String),
    CollisionEnter(String),
    Destroy,
}

impl RhaiScriptEvent {
    fn function_name(&self) -> &'static str {
        match self {
            Self::Start => "on_start",
            Self::Update(_) => "on_update",
            Self::KeyDown(_) => "on_key_down",
            Self::CollisionEnter(_) => "on_collision_enter",
            Self::Destroy => "on_destroy",
        }
    }
}

#[derive(Clone)]
struct CachedRhaiScript {
    source: String,
    ast: AST,
    modified: Option<SystemTime>,
}

#[derive(Debug, Default)]
struct ScriptHostState {
    current_entity_id: Option<u64>,
    commands: Vec<ScriptCommand>,
    inputs_pressed: BTreeSet<String>,
}

type SharedHostState = Arc<Mutex<ScriptHostState>>;

pub struct RhaiScriptRuntime {
    engine: Engine,
    host: SharedHostState,
    project_path: PathBuf,
    cache: BTreeMap<PathBuf, CachedRhaiScript>,
    started_entities: BTreeSet<u64>,
    destroying_entities: BTreeSet<u64>,
    changed_paths: Arc<Mutex<BTreeSet<PathBuf>>>,
    watcher: Option<RecommendedWatcher>,
    pub watcher_active: bool,
    pub reload_count: usize,
    pub last_frame_scripts: usize,
    pub last_errors: Vec<String>,
}

impl std::fmt::Debug for RhaiScriptRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RhaiScriptRuntime")
            .field("project_path", &self.project_path)
            .field("cache_len", &self.cache.len())
            .field("started_entities", &self.started_entities)
            .field("watcher_active", &self.watcher_active)
            .field("reload_count", &self.reload_count)
            .field("last_frame_scripts", &self.last_frame_scripts)
            .field("last_errors", &self.last_errors)
            .finish()
    }
}

impl Default for RhaiScriptRuntime {
    fn default() -> Self {
        Self::new(PathBuf::new())
    }
}

impl RhaiScriptRuntime {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        let host = Arc::new(Mutex::new(ScriptHostState::default()));
        let engine = build_engine(host.clone());
        let mut runtime = Self {
            engine,
            host,
            project_path: project_path.as_ref().to_path_buf(),
            cache: BTreeMap::new(),
            started_entities: BTreeSet::new(),
            destroying_entities: BTreeSet::new(),
            changed_paths: Arc::new(Mutex::new(BTreeSet::new())),
            watcher: None,
            watcher_active: false,
            reload_count: 0,
            last_frame_scripts: 0,
            last_errors: Vec::new(),
        };
        let _ = runtime.watch_project();
        runtime
    }

    pub fn set_project_path(&mut self, project_path: impl AsRef<Path>) {
        self.project_path = project_path.as_ref().to_path_buf();
        self.cache.clear();
        self.started_entities.clear();
        let _ = self.watch_project();
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
            let Ok(event) = result else {
                return;
            };
            if !is_reload_event(&event.kind) {
                return;
            }
            if let Ok(mut changed) = changed_paths.lock() {
                for path in event.paths {
                    if path.extension().and_then(|value| value.to_str()) == Some("rhai") {
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
        if let Ok(mut changed) = self.changed_paths.lock() {
            changed.insert(normalize_path(path.as_ref()));
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
        for path in changed {
            if path.extension().and_then(|value| value.to_str()) != Some("rhai") {
                continue;
            }
            self.cache.remove(&normalize_path(&path));
            reloaded += 1;
        }
        self.reload_count += reloaded;
        reloaded
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

    pub fn input_pressed(&self, key: &str) -> bool {
        self.host
            .lock()
            .map(|host| host.inputs_pressed.contains(key))
            .unwrap_or(false)
    }

    pub fn update_entities(
        &mut self,
        entities: &mut Vec<GameObject>,
        dt: f64,
        mode: &str,
    ) -> RhaiRunReport {
        self.poll_hot_reload();
        self.last_frame_scripts = 0;
        self.last_errors.clear();
        if mode != "PLAY" {
            return RhaiRunReport::default();
        }
        self.retain_live_entities(entities);

        let mut report = self.run_start_events(entities);
        report.merge(self.run_event_for_all(entities, RhaiScriptEvent::Update(dt)));
        report.errors = self.last_errors.clone();
        report
    }

    pub fn run_start_events(&mut self, entities: &mut Vec<GameObject>) -> RhaiRunReport {
        let calls = self
            .collect_script_calls(entities)
            .into_iter()
            .filter(|call| !self.started_entities.contains(&call.entity_id))
            .collect::<Vec<_>>();
        let mut report = RhaiRunReport::default();
        for call in calls {
            for script_path in &call.script_paths {
                self.call_script_event(&call, script_path, &RhaiScriptEvent::Start, &mut report);
                self.apply_pending_commands(entities, &mut report);
            }
            self.started_entities.insert(call.entity_id);
        }
        report.errors = self.last_errors.clone();
        report
    }

    pub fn run_key_down(
        &mut self,
        entities: &mut Vec<GameObject>,
        key: impl Into<String>,
    ) -> RhaiRunReport {
        let key = key.into();
        self.set_input_pressed(&key, true);
        let mut report = self.run_event_for_all(entities, RhaiScriptEvent::KeyDown(key));
        report.errors = self.last_errors.clone();
        report
    }

    pub fn run_collision_enter(
        &mut self,
        entities: &mut Vec<GameObject>,
        entity_id: u64,
        other_name: impl Into<String>,
    ) -> RhaiRunReport {
        let Some(call) = self
            .collect_script_calls(entities)
            .into_iter()
            .find(|call| call.entity_id == entity_id)
        else {
            return RhaiRunReport::default();
        };
        let event = RhaiScriptEvent::CollisionEnter(other_name.into());
        let mut report = RhaiRunReport::default();
        for script_path in &call.script_paths {
            self.call_script_event(&call, script_path, &event, &mut report);
            self.apply_pending_commands(entities, &mut report);
        }
        report.errors = self.last_errors.clone();
        report
    }

    pub fn run_destroy(&mut self, entities: &mut Vec<GameObject>, entity_id: u64) -> RhaiRunReport {
        if self.destroying_entities.contains(&entity_id) {
            return RhaiRunReport::default();
        }
        self.destroying_entities.insert(entity_id);
        let mut report = self.run_event_for_ids(entities, &[entity_id], RhaiScriptEvent::Destroy);
        if GameAPI::destroy(entities, entity_id) {
            report.destroyed.push(entity_id);
            report.commands_applied += 1;
        }
        self.started_entities.remove(&entity_id);
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

    fn run_event_for_all(
        &mut self,
        entities: &mut Vec<GameObject>,
        event: RhaiScriptEvent,
    ) -> RhaiRunReport {
        let calls = self.collect_script_calls(entities);
        let mut report = RhaiRunReport::default();
        for call in calls {
            for script_path in &call.script_paths {
                self.call_script_event(&call, script_path, &event, &mut report);
                self.apply_pending_commands(entities, &mut report);
            }
        }
        report.errors = self.last_errors.clone();
        report
    }

    fn run_event_for_ids(
        &mut self,
        entities: &mut Vec<GameObject>,
        ids: &[u64],
        event: RhaiScriptEvent,
    ) -> RhaiRunReport {
        let ids = ids.iter().copied().collect::<BTreeSet<_>>();
        let calls = self
            .collect_script_calls(entities)
            .into_iter()
            .filter(|call| ids.contains(&call.entity_id))
            .collect::<Vec<_>>();
        let mut report = RhaiRunReport::default();
        for call in calls {
            for script_path in &call.script_paths {
                self.call_script_event(&call, script_path, &event, &mut report);
                self.apply_pending_commands(entities, &mut report);
            }
        }
        report.errors = self.last_errors.clone();
        report
    }

    fn call_script_event(
        &mut self,
        call: &EntityScriptCall,
        script_path: &Path,
        event: &RhaiScriptEvent,
        report: &mut RhaiRunReport,
    ) {
        let Some((source, ast)) = self.load_script(script_path) else {
            return;
        };
        let function = event.function_name();
        if !source_has_function(&source, function) {
            return;
        }
        if let Ok(mut host) = self.host.lock() {
            host.current_entity_id = Some(call.entity_id);
        }

        let mut scope = Scope::new();
        scope.push_constant("entity_id", call.entity_id as INT);
        scope.push_constant("entity_name", call.entity_name.clone());

        let result = match event {
            RhaiScriptEvent::Start | RhaiScriptEvent::Destroy => {
                self.engine.call_fn::<()>(&mut scope, &ast, function, ())
            }
            RhaiScriptEvent::Update(dt) => {
                self.engine
                    .call_fn::<()>(&mut scope, &ast, function, (*dt as FLOAT,))
            }
            RhaiScriptEvent::KeyDown(key) | RhaiScriptEvent::CollisionEnter(key) => self
                .engine
                .call_fn::<()>(&mut scope, &ast, function, (key.clone(),)),
        };

        if let Ok(mut host) = self.host.lock() {
            host.current_entity_id = None;
        }

        match result {
            Ok(()) => {
                self.last_frame_scripts += 1;
                report.scripts_run += 1;
            }
            Err(error) => {
                self.last_errors
                    .push(format!("{}::{function}: {error}", script_path.display()));
            }
        }
    }

    fn load_script(&mut self, path: &Path) -> Option<(String, AST)> {
        let key = normalize_path(path);
        let modified = fs::metadata(&key).and_then(|meta| meta.modified()).ok();
        if let Some(cached) = self.cache.get(&key)
            && cached.modified == modified
        {
            return Some((cached.source.clone(), cached.ast.clone()));
        }

        let source = match fs::read_to_string(&key) {
            Ok(source) => source,
            Err(error) => {
                self.last_errors.push(format!("{}: {error}", key.display()));
                return None;
            }
        };
        let compile_source = rewrite_reserved_gameplay_api(&source);
        let ast = match self.engine.compile(&compile_source) {
            Ok(ast) => ast,
            Err(error) => {
                self.last_errors.push(format!("{}: {error}", key.display()));
                return None;
            }
        };
        self.cache.insert(
            key,
            CachedRhaiScript {
                source: source.clone(),
                ast: ast.clone(),
                modified,
            },
        );
        Some((source, ast))
    }

    fn collect_script_calls(&self, entities: &[GameObject]) -> Vec<EntityScriptCall> {
        entities
            .iter()
            .filter(|entity| entity.enabled && entity.active)
            .filter_map(|entity| {
                let script_paths = self.script_paths_for_entity(entity);
                if script_paths.is_empty() {
                    None
                } else {
                    Some(EntityScriptCall {
                        entity_id: entity.id,
                        entity_name: entity.name.clone(),
                        script_paths,
                    })
                }
            })
            .collect()
    }

    fn retain_live_entities(&mut self, entities: &[GameObject]) {
        let live = entities
            .iter()
            .map(|entity| entity.id)
            .collect::<BTreeSet<_>>();
        self.started_entities
            .retain(|entity_id| live.contains(entity_id));
        self.destroying_entities
            .retain(|entity_id| live.contains(entity_id));
    }

    fn script_paths_for_entity(&self, entity: &GameObject) -> Vec<PathBuf> {
        let mut refs = Vec::new();
        if let Some(script) = &entity.script {
            refs.push(script.clone());
        }
        for script in &entity.scripts {
            if let Some(path) = script.as_str() {
                refs.push(path.to_string());
            } else if script
                .get("runtime")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|runtime| runtime.eq_ignore_ascii_case("rhai"))
            {
                for key in ["path", "script", "name"] {
                    if let Some(value) = script.get(key).and_then(serde_json::Value::as_str) {
                        refs.push(value.to_string());
                        break;
                    }
                }
            }
        }

        refs.into_iter()
            .filter_map(|value| self.resolve_script_path(&value))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn resolve_script_path(&self, value: &str) -> Option<PathBuf> {
        let value = value.trim();
        if value.is_empty() || value.ends_with(".mfgraph") {
            return None;
        }
        let raw = Path::new(value);
        let extension = raw.extension().and_then(|value| value.to_str());
        if extension.is_some_and(|ext| ext != "rhai") {
            return None;
        }

        let mut candidates = Vec::new();
        if raw.is_absolute() {
            candidates.push(raw.to_path_buf());
        } else {
            candidates.push(self.project_path.join(raw));
            candidates.push(self.project_path.join("scripts").join(raw));
            if extension.is_none() {
                candidates.push(
                    self.project_path
                        .join("scripts")
                        .join(format!("{value}.rhai")),
                );
            }
        }
        candidates
            .iter()
            .find(|path| path.exists())
            .map(|path| normalize_path(path))
            .or_else(|| {
                if extension == Some("rhai") {
                    candidates.last().map(normalize_path)
                } else {
                    None
                }
            })
    }

    fn apply_pending_commands(
        &mut self,
        entities: &mut Vec<GameObject>,
        report: &mut RhaiRunReport,
    ) {
        let commands = self.drain_commands();
        for command in commands {
            self.apply_command(entities, command, report);
        }
    }

    fn apply_command(
        &mut self,
        entities: &mut Vec<GameObject>,
        command: ScriptCommand,
        report: &mut RhaiRunReport,
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
            ScriptCommand::Destroy { target } => {
                let ids = resolve_targets(entities, &target);
                for id in ids {
                    if self.destroying_entities.contains(&id) {
                        continue;
                    }
                    let destroy_report = self.run_destroy(entities, id);
                    report.merge(destroy_report);
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
                let updated = match target {
                    ScriptTarget::Id(id) => GameAPI::set_ui_text_by_id(entities, id, &text),
                    ScriptTarget::Name(name) => {
                        GameAPI::set_ui_text_by_name(entities, &name, &text)
                    }
                };
                if updated {
                    report.ui_updates += 1;
                    report.commands_applied += 1;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct EntityScriptCall {
    entity_id: u64,
    entity_name: String,
    script_paths: Vec<PathBuf>,
}

fn build_engine(host: SharedHostState) -> Engine {
    let mut engine = Engine::new();

    let shared = host.clone();
    engine.register_fn("move", move |dx: FLOAT, dy: FLOAT| {
        push_self_command(&shared, |entity_id| ScriptCommand::Move {
            entity_id,
            dx,
            dy,
        });
    });
    let shared = host.clone();
    engine.register_fn("move", move |dx: INT, dy: INT| {
        push_self_command(&shared, |entity_id| ScriptCommand::Move {
            entity_id,
            dx: dx as f64,
            dy: dy as f64,
        });
    });

    let shared = host.clone();
    engine.register_fn("set_position", move |x: FLOAT, y: FLOAT| {
        push_self_command(&shared, |entity_id| ScriptCommand::SetPosition {
            entity_id,
            x,
            y,
        });
    });
    let shared = host.clone();
    engine.register_fn("set_position", move |x: INT, y: INT| {
        push_self_command(&shared, |entity_id| ScriptCommand::SetPosition {
            entity_id,
            x: x as f64,
            y: y as f64,
        });
    });

    let shared = host.clone();
    engine.register_fn("spawn", move |name: String, x: FLOAT, y: FLOAT| -> INT {
        push_command(&shared, ScriptCommand::Spawn { name, x, y });
        0
    });
    let shared = host.clone();
    engine.register_fn(
        "spawn_entity",
        move |name: String, x: FLOAT, y: FLOAT| -> INT {
            push_command(&shared, ScriptCommand::Spawn { name, x, y });
            0
        },
    );
    let shared = host.clone();
    engine.register_fn("spawn", move |name: String, x: INT, y: INT| -> INT {
        push_command(
            &shared,
            ScriptCommand::Spawn {
                name,
                x: x as f64,
                y: y as f64,
            },
        );
        0
    });
    let shared = host.clone();
    engine.register_fn("spawn_entity", move |name: String, x: INT, y: INT| -> INT {
        push_command(
            &shared,
            ScriptCommand::Spawn {
                name,
                x: x as f64,
                y: y as f64,
            },
        );
        0
    });

    let shared = host.clone();
    engine.register_fn("destroy", move || {
        push_self_command(&shared, |entity_id| ScriptCommand::Destroy {
            target: ScriptTarget::Id(entity_id),
        });
    });
    let shared = host.clone();
    engine.register_fn("destroy", move |entity_id: INT| {
        if entity_id >= 0 {
            push_command(
                &shared,
                ScriptCommand::Destroy {
                    target: ScriptTarget::Id(entity_id as u64),
                },
            );
        }
    });
    let shared = host.clone();
    engine.register_fn("destroy", move |name: String| {
        push_command(
            &shared,
            ScriptCommand::Destroy {
                target: ScriptTarget::Name(name),
            },
        );
    });

    let shared = host.clone();
    engine.register_fn("play_sound", move |name: String| {
        push_command(
            &shared,
            ScriptCommand::PlaySound {
                name,
                bus: "SFX".to_string(),
                volume: 1.0,
                looped: false,
            },
        );
    });
    let shared = host.clone();
    engine.register_fn("play_sound", move |name: String, bus: String| {
        push_command(
            &shared,
            ScriptCommand::PlaySound {
                name,
                bus,
                volume: 1.0,
                looped: false,
            },
        );
    });

    let shared = host.clone();
    engine.register_fn("load_scene", move |name: String| {
        push_command(&shared, ScriptCommand::LoadScene { name });
    });

    let shared = host.clone();
    engine.register_fn("input_pressed", move |key: String| -> bool {
        shared
            .lock()
            .map(|host| host.inputs_pressed.contains(&key))
            .unwrap_or(false)
    });

    let shared = host.clone();
    engine.register_fn("set_ui_text", move |target: String, text: String| {
        push_command(
            &shared,
            ScriptCommand::SetUiText {
                target: ScriptTarget::Name(target),
                text,
            },
        );
    });
    let shared = host.clone();
    engine.register_fn("ui_text", move |text: String| {
        push_self_command(&shared, |entity_id| ScriptCommand::SetUiText {
            target: ScriptTarget::Id(entity_id),
            text: text.clone(),
        });
    });

    engine
}

fn push_self_command(host: &SharedHostState, command: impl FnOnce(u64) -> ScriptCommand) {
    let Ok(mut host) = host.lock() else {
        return;
    };
    if let Some(entity_id) = host.current_entity_id {
        host.commands.push(command(entity_id));
    }
}

fn push_command(host: &SharedHostState, command: ScriptCommand) {
    if let Ok(mut host) = host.lock() {
        host.commands.push(command);
    }
}

fn resolve_targets(entities: &[GameObject], target: &ScriptTarget) -> Vec<u64> {
    match target {
        ScriptTarget::Id(id) => entities
            .iter()
            .find(|entity| entity.id == *id)
            .map(|entity| vec![entity.id])
            .unwrap_or_default(),
        ScriptTarget::Name(name) => entities
            .iter()
            .filter(|entity| entity.name == *name)
            .map(|entity| entity.id)
            .collect(),
    }
}

fn source_has_function(source: &str, function: &str) -> bool {
    source.contains(&format!("fn {function}("))
        || source.contains(&format!("fn {function} ("))
        || source.contains(&format!("private fn {function}("))
        || source.contains(&format!("public fn {function}("))
}

fn is_reload_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

fn rewrite_reserved_gameplay_api(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    let mut in_string: Option<u8> = None;
    let mut escaped = false;
    let mut in_line_comment = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_line_comment {
            output.push(byte as char);
            index += 1;
            if byte == b'\n' {
                in_line_comment = false;
            }
            continue;
        }
        if let Some(quote) = in_string {
            output.push(byte as char);
            index += 1;
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == quote {
                in_string = None;
            }
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            output.push('/');
            output.push('/');
            index += 2;
            in_line_comment = true;
            continue;
        }
        if byte == b'"' || byte == b'\'' || byte == b'`' {
            in_string = Some(byte);
            output.push(byte as char);
            index += 1;
            continue;
        }

        if is_spawn_call(bytes, index) {
            output.push_str("spawn_entity");
            index += "spawn".len();
        } else {
            output.push(byte as char);
            index += 1;
        }
    }

    output
}

fn is_spawn_call(bytes: &[u8], index: usize) -> bool {
    let keyword = b"spawn";
    if !bytes[index..].starts_with(keyword) {
        return false;
    }
    if index > 0 && is_ident_byte(bytes[index - 1]) {
        return false;
    }
    let mut after = index + keyword.len();
    if bytes.get(after).is_some_and(|byte| is_ident_byte(*byte)) {
        return false;
    }
    while bytes
        .get(after)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        after += 1;
    }
    bytes.get(after) == Some(&b'(')
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().components().collect()
}
