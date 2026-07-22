use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use walkdir::WalkDir;

use crate::core::game::Game;
use crate::engine::asset_database::AssetRecord;
use crate::engine::asset_importers::SpriteSheetImporter;
use crate::engine::asset_tools::AssetTools;
use crate::engine::command_palette::CommandPalette;
use crate::engine::component::{Component, default_component};
use crate::engine::component_registry::ComponentSubMenu;
use crate::engine::developer_console::ConsoleEntry;
use crate::engine::editor_asset_connector::EditorAssetConnector;
use crate::engine::editor_command::EditorCommandKind;
use crate::engine::editor_python::{
    PythonAutomationHost, PythonEditorContext, batch_convert_sprites, batch_import_assets,
    generate_paged_sprite_atlases,
};
use crate::engine::editor_spatial_tools_2d::{AlignMode2D, EditorSpatialTools2D};
use crate::engine::editor_tool_sessions::EditorToolSessions;
use crate::engine::editor_ui::EditorFileWatcher;
use crate::engine::forge_ai::context::AiProjectContext;
use crate::engine::forge_ai::diagnostics::{AiDiagnostic, ProjectDoctor};
use crate::engine::forge_ai::executor::{AiFileChange, AiHostValidation};
use crate::engine::forge_ai::testing::{AiTestReport, AiTestStatus, AiTestSuite};
use crate::engine::input_map::{InputActionInfo, InputMap};
use crate::engine::inspector_editor::InspectorEditor;
use crate::engine::luau_scripting::{
    LuauScriptRuntime, ScriptBreakpoint, ScriptDebuggerState, ScriptWatchResult,
};
use crate::engine::miniforge_2d::content_browser::asset_from_record;
use crate::engine::miniforge_2d::paper2d::SpriteFrames2D;
use crate::engine::project_launcher::{LauncherTemplate, ProjectLauncherState};
use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
use crate::engine::project_validator::ProjectValidator;
use crate::engine::render_2d::{
    Render2DCompatibilityProfile, SpriteAtlasExportOptions2D, export_sprite_atlas_pages_from_files,
};
use crate::engine::runtime_exporter::{ExportProfile, RuntimeExportReport};
use crate::engine::safe_mode::SafeModeSettings;
use crate::engine::session_recovery::{SessionRecoveryManager, SessionUiState};
use crate::engine::sprite_editor::{SpriteColor, SpriteEditorCanvas};
use crate::engine::system_audit::{SystemReadinessLevel, SystemReadinessReport};
use crate::engine::visual_graph_serializer::VisualGraphSerializer;
use crate::entities::game_object::GameObject;
use crate::render::backend::RenderBackendConfig;
use crate::systems::rts_system::RTSSystem;

pub const EDITOR_CORE_API_VERSION: u32 = 1;
const MAX_EDITOR_SCRIPT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CONTENT_TEXT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityRow {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub entity_type: String,
    pub tag: String,
    pub layer: String,
    pub x: f64,
    pub y: f64,
    pub visible: bool,
    pub enabled: bool,
    pub locked: bool,
    pub selected: bool,
    pub component_count: usize,
    pub child_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InspectorFieldDto {
    pub entity_id: u64,
    pub target: String,
    pub key: String,
    pub display_name: String,
    pub value_json: String,
    pub value_type: String,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InspectorQuickAssetDto {
    pub relative_path: String,
    pub name: String,
    pub asset_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InspectorQuickActionDto {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub target_component: Option<String>,
    pub enabled: bool,
    pub disabled_reason: String,
    pub requires_asset: bool,
    pub asset_type: String,
    pub attached_asset_path: Option<String>,
    pub assets: Vec<InspectorQuickAssetDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InspectorQuickActionOutcomeDto {
    pub changed: bool,
    pub message: String,
    pub open_asset_path: Option<String>,
    pub open_asset_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetRow {
    pub guid: String,
    pub relative_path: String,
    pub name: String,
    pub asset_type: String,
    pub size_bytes: u64,
    pub labels: Vec<String>,
    pub dependency_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentFolderDto {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub asset_count: usize,
    pub child_folder_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentEntryDto {
    pub name: String,
    pub relative_path: String,
    pub asset_type: String,
    pub is_directory: bool,
    pub editable: bool,
    pub bytes: u64,
    pub modified_ms: u64,
    pub child_count: usize,
    pub preview_url: String,
    pub guid: String,
    pub labels: Vec<String>,
    pub include_in_build: bool,
    pub dependencies: Vec<String>,
    pub reverse_dependencies: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandDescriptor {
    pub id: String,
    pub label: String,
    pub category: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandOutcome {
    pub changed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneStateDto {
    pub scene_name: String,
    pub dirty: bool,
    pub dirty_reason: String,
    pub mode: String,
    pub selected_count: usize,
    pub entity_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrefabStudioStateDto {
    pub prefab_assets: Vec<AssetRow>,
    pub selected_entity_id: Option<u64>,
    pub selected_instance: Option<crate::engine::advanced_prefabs::PrefabInstanceReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrefabActionResultDto {
    pub changed: bool,
    pub path: Option<String>,
    pub entity_id: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessRow {
    pub system: String,
    pub level: SystemReadinessLevel,
    pub score: u8,
    pub strength_count: usize,
    pub gap_count: usize,
    pub action_count: usize,
    pub top_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeHealthDto {
    pub level: String,
    pub healthy: bool,
    pub summary: String,
    pub mode: String,
    pub guard_enabled: bool,
    pub raw_delta_ms: f64,
    pub safe_delta_ms: f64,
    pub delta_was_invalid: bool,
    pub delta_was_clamped: bool,
    pub repaired_values: usize,
    pub quarantined_entities: usize,
    pub entity_count: usize,
    pub max_entities: usize,
    pub entity_limit_exceeded_by: usize,
    pub optional_cadence_divisor: u64,
    pub stability_score: f64,
    pub fps: f64,
    pub average_frame_time_ms: f64,
    pub frame_budget_ms: f64,
    pub safe_mode_active: bool,
    pub safe_mode_reason: String,
    pub safe_mode_disabled_systems: Vec<String>,
    pub warnings: Vec<String>,
}

/// Options accepted by the native editor when a project is opened.
///
/// The legacy `open_project` entry point remains equivalent to the default
/// options. Native frontends can opt into recovery-safe startup without
/// relying on process-global environment variables.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct EditorOpenOptions {
    pub safe_mode: bool,
    pub safe_mode_reason: String,
    pub disable_asset_importers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSettingsDto {
    pub engine: Value,
    pub input: Value,
    pub tags: Vec<String>,
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectLauncherDto {
    pub workspace_root: String,
    pub project_location: String,
    pub recent_projects: Vec<String>,
    pub templates: Vec<String>,
    pub settings: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetManageOutcomeDto {
    pub action: String,
    pub source: String,
    pub destination: Option<String>,
    pub sidecars: Vec<String>,
    pub refreshed_asset_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfilerSystemDto {
    pub name: String,
    pub milliseconds: f64,
    pub frame_percent: f64,
    pub over_frame_budget: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfilerSnapshotDto {
    pub frame_time_ms: f64,
    pub frame_budget_ms: f64,
    pub fps: f64,
    pub systems_total_ms: f64,
    pub unaccounted_ms: f64,
    pub budget_usage_percent: f64,
    pub over_budget: bool,
    pub slowest_system: Option<String>,
    pub systems: Vec<ProfilerSystemDto>,
    pub metrics: BTreeMap<String, f64>,
    pub counters: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetDependencyNodeDto {
    pub path: String,
    pub guid: String,
    pub asset_type: String,
    pub size_bytes: u64,
    pub dependency_count: usize,
    pub reverse_dependency_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetDependencyEdgeDto {
    pub dependency: String,
    pub consumer: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetDependencyGraphDto {
    pub nodes: Vec<AssetDependencyNodeDto>,
    pub edges: Vec<AssetDependencyEdgeDto>,
    pub build_order: Vec<String>,
    pub cycles: Vec<Vec<String>>,
    pub unresolved_dependencies: Vec<String>,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectOperationOutcomeDto {
    pub action: String,
    pub message: String,
    pub artifact_path: Option<String>,
    pub files: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalLaunchPlanDto {
    pub kind: String,
    pub profile: String,
    pub ready: bool,
    pub executable: Option<String>,
    pub arguments: Vec<String>,
    pub working_directory: String,
    pub artifact_path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutosaveStatusDto {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub exists: bool,
    pub health: String,
    pub path: String,
    pub backup_path: String,
    pub last_error: Option<String>,
    pub recoveries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStatusDto {
    pub pending: bool,
    pub path: String,
    pub saved_unix_ms: Option<u128>,
    pub current_scene: Option<String>,
    pub scene_dirty: bool,
    pub documents: usize,
    pub dirty_buffers: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectOperationsDto {
    pub project_path: String,
    pub autosave: AutosaveStatusDto,
    pub session: SessionStatusDto,
    pub last_operation: Option<ProjectOperationOutcomeDto>,
    pub external_launch: Option<ExternalLaunchPlanDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewportSnapshot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LuauScriptRow {
    pub relative_path: String,
    pub name: String,
    pub bytes: u64,
    pub valid: bool,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LuauValidationResult {
    pub valid: bool,
    pub diagnostic: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub incomplete_input: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EditorCoreErrorKind {
    NoProjectOpen,
    InvalidArgument,
    NotFound,
    Io,
    Serde,
    CommandFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorCoreError {
    pub kind: EditorCoreErrorKind,
    pub message: String,
}

impl EditorCoreError {
    pub fn new(kind: EditorCoreErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn no_project() -> Self {
        Self::new(EditorCoreErrorKind::NoProjectOpen, "No project is open")
    }
}

impl fmt::Display for EditorCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for EditorCoreError {}

impl From<std::io::Error> for EditorCoreError {
    fn from(error: std::io::Error) -> Self {
        Self::new(EditorCoreErrorKind::Io, error.to_string())
    }
}

impl From<serde_json::Error> for EditorCoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(EditorCoreErrorKind::Serde, error.to_string())
    }
}

pub struct EditorCore {
    project_path: Option<PathBuf>,
    game: Option<Game>,
    tool_sessions: EditorToolSessions,
    command_palette: CommandPalette,
    entity_cache: Vec<EntityRow>,
    selected_cache: Vec<u64>,
    inspector_cache: BTreeMap<u64, Vec<InspectorFieldDto>>,
    asset_cache: Vec<AssetRow>,
    command_cache: Vec<CommandDescriptor>,
    readiness_cache: Vec<ReadinessRow>,
    readiness_score: u8,
    readiness_summary: String,
    last_export_report: Option<RuntimeExportReport>,
    last_python_result: Option<Value>,
    last_project_operation: Option<ProjectOperationOutcomeDto>,
    external_launch_plan: Option<ExternalLaunchPlanDto>,
    session_recovery: Option<SessionRecoveryManager>,
    file_watcher: Option<EditorFileWatcher>,
}

impl Default for EditorCore {
    fn default() -> Self {
        let command_cache = default_command_descriptors();
        let commands = command_cache
            .iter()
            .map(|command| command.label.clone())
            .collect::<Vec<_>>();
        Self {
            project_path: None,
            game: None,
            tool_sessions: EditorToolSessions::default(),
            command_palette: CommandPalette::with_commands(commands),
            entity_cache: Vec::new(),
            selected_cache: Vec::new(),
            inspector_cache: BTreeMap::new(),
            asset_cache: Vec::new(),
            command_cache,
            readiness_cache: Vec::new(),
            readiness_score: 0,
            readiness_summary: "Readiness unavailable".to_string(),
            last_export_report: None,
            last_python_result: None,
            last_project_operation: None,
            external_launch_plan: None,
            session_recovery: None,
            file_watcher: None,
        }
    }
}

impl EditorCore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_project(&mut self, path: impl AsRef<Path>) -> Result<(), EditorCoreError> {
        self.open_project_with_options(path, EditorOpenOptions::default())
    }

    pub fn open_project_with_options(
        &mut self,
        path: impl AsRef<Path>,
        options: EditorOpenOptions,
    ) -> Result<(), EditorCoreError> {
        validate_editor_open_options(&options)?;
        let project_path = path.as_ref().to_path_buf();
        let safe_mode = if options.safe_mode {
            let reason = if options.safe_mode_reason.trim().is_empty() {
                "requested by native editor".to_string()
            } else {
                options.safe_mode_reason.trim().to_string()
            };
            let mut settings = SafeModeSettings::for_recovery(reason);
            settings.disable_asset_importers = options.disable_asset_importers;
            settings
        } else {
            SafeModeSettings::default()
        };
        let mut game = Game::from_project_with_safe_mode(&project_path, false, safe_mode)?;
        if game.safe_mode.allows_asset_importers() {
            game.asset_database.scan()?;
        } else {
            game.console.warning(
                "Initial asset import scan skipped by Safe Mode",
                "SAFE_MODE",
            );
        }
        game.console.log(
            "Qt editor bridge opened this project through EditorCore",
            "EDITOR",
        );
        self.tool_sessions
            .open_project(&project_path)
            .map_err(|message| EditorCoreError::new(EditorCoreErrorKind::Io, message))?;
        self.finish_active_session();
        self.project_path = Some(project_path);
        self.game = Some(game);
        self.last_export_report = None;
        self.last_python_result = None;
        self.last_project_operation = None;
        self.external_launch_plan = None;
        self.session_recovery = Some(SessionRecoveryManager::new(
            self.project_path
                .as_ref()
                .expect("project path was just set"),
            Duration::from_secs(10),
        ));
        self.file_watcher = EditorFileWatcher::watch(
            self.project_path
                .as_ref()
                .expect("project path was just set"),
        )
        .ok();
        self.refresh_all_caches();
        Ok(())
    }

    pub fn is_project_open(&self) -> bool {
        self.game.is_some()
    }

    /// Re-synchronizes the live scene and asset views without replacing the
    /// in-memory scene or running filesystem-heavy asset/readiness audits.
    /// Those remain explicit through `assets.refresh` and `project.audit`.
    pub fn refresh(&mut self) -> Result<(), EditorCoreError> {
        self.game()?;
        self.run_periodic_maintenance();
        self.refresh_scene_cache();
        self.refresh_asset_cache();
        Ok(())
    }

    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    pub fn editor_tool_state(&self, tool: &str) -> Result<Value, EditorCoreError> {
        self.game()?;
        self.tool_sessions
            .state(tool)
            .map_err(|message| EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message))
    }

    pub fn editor_tool_action(
        &mut self,
        tool: &str,
        action: &str,
        payload_json: &str,
    ) -> Result<Value, EditorCoreError> {
        self.game()?;
        let payload = if payload_json.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(payload_json)?
        };
        let state = self
            .tool_sessions
            .action(tool, action, &payload)
            .map_err(|message| EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message))?;
        if action == "save" {
            self.refresh_asset_cache();
        }
        Ok(state)
    }

    pub fn project_settings(&self) -> Result<ProjectSettingsDto, EditorCoreError> {
        let game = self.game()?;
        Ok(ProjectSettingsDto {
            engine: game.engine_config.data.clone(),
            input: input_map_value(&game.input_map),
            tags: game.tags_layers_manager.tags.clone(),
            layers: game.tags_layers_manager.layers.clone(),
        })
    }

    pub fn save_engine_settings_json(&mut self, source: &str) -> Result<(), EditorCoreError> {
        let value: Value = serde_json::from_str(source)?;
        validate_engine_settings(&value)?;
        let game = self.game_mut()?;
        game.engine_config.data = value;
        game.engine_config.save()?;
        game.console
            .log("Project settings saved atomically", "PROJECT");
        Ok(())
    }

    pub fn save_input_map_json(&mut self, source: &str) -> Result<(), EditorCoreError> {
        let value: Value = serde_json::from_str(source)?;
        let (bindings, actions) = parse_input_map(&value)?;
        let game = self.game_mut()?;
        game.input_map.bindings = bindings;
        game.input_map.actions = actions;
        game.input_map.ensure_default_actions();
        game.input_map.save()?;
        game.console.log("Input Map saved atomically", "PROJECT");
        Ok(())
    }

    pub fn save_tags_layers_json(&mut self, source: &str) -> Result<(), EditorCoreError> {
        let value: Value = serde_json::from_str(source)?;
        let tags = parse_named_items(&value, "tags", "Untagged")?;
        let layers = parse_named_items(&value, "layers", "Default")?;
        let game = self.game_mut()?;
        game.tags_layers_manager.tags = tags;
        game.tags_layers_manager.layers = layers;
        crate::engine::asset_tools::AssetTools::write_json(
            game.tags_layers_manager.settings_path.join("tags.json"),
            &serde_json::json!({"items": game.tags_layers_manager.tags}),
        )?;
        crate::engine::asset_tools::AssetTools::write_json(
            game.tags_layers_manager.settings_path.join("layers.json"),
            &serde_json::json!({"items": game.tags_layers_manager.layers}),
        )?;
        game.console
            .log("Tags and Layers saved atomically", "PROJECT");
        Ok(())
    }

    pub fn launcher_snapshot(
        &self,
        workspace_root: impl AsRef<Path>,
    ) -> Result<ProjectLauncherDto, EditorCoreError> {
        let mut launcher = ProjectLauncherState::new(workspace_root.as_ref());
        launcher.discover_recent_projects()?;
        Ok(project_launcher_dto(&launcher))
    }

    pub fn launcher_create_project(
        &self,
        workspace_root: impl AsRef<Path>,
        location: impl AsRef<Path>,
        name: &str,
        template: &str,
    ) -> Result<PathBuf, EditorCoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 80 {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Project name must contain between 1 and 80 characters",
            ));
        }
        let selected_template = parse_launcher_template(template)?;
        let mut launcher = ProjectLauncherState::new(workspace_root.as_ref());
        launcher.project_name = name.to_string();
        launcher.project_location = location.as_ref().to_string_lossy().to_string();
        launcher.selected_template = selected_template;
        launcher.create_new_project().map_err(EditorCoreError::from)
    }

    pub fn launcher_repair_project(
        &self,
        workspace_root: impl AsRef<Path>,
        project_path: impl AsRef<Path>,
    ) -> Result<Value, EditorCoreError> {
        let mut launcher = ProjectLauncherState::new(workspace_root.as_ref());
        let notes = launcher.repair_project(project_path.as_ref())?;
        Ok(serde_json::json!({
            "project_path": project_path.as_ref().to_string_lossy(),
            "notes": notes,
            "backend_summary": launcher.backend_summary,
            "backend_actions": launcher.backend_actions,
            "status": launcher.status,
        }))
    }

    pub fn project_operations(&self) -> Result<ProjectOperationsDto, EditorCoreError> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or_else(EditorCoreError::no_project)?;
        let game = self.game()?;
        let autosave = &game.autosave_manager;
        let recoveries = autosave
            .available_recoveries()
            .into_iter()
            .map(|entry| {
                format!(
                    "{}/{}",
                    entry.domain.folder(),
                    entry
                        .path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("recovery")
                )
            })
            .collect();
        let recovery = SessionRecoveryManager::new(project_path, Duration::from_secs(10));
        let (snapshot, session_error) = match recovery.load_pending() {
            Ok(snapshot) => (snapshot, None),
            Err(error) => (None, Some(error.to_string())),
        };
        let session = SessionStatusDto {
            pending: recovery.is_pending(),
            path: recovery.path.to_string_lossy().to_string(),
            saved_unix_ms: snapshot.as_ref().map(|snapshot| snapshot.saved_unix_ms),
            current_scene: snapshot
                .as_ref()
                .map(|snapshot| snapshot.current_scene.clone()),
            scene_dirty: snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.scene_dirty),
            documents: snapshot
                .as_ref()
                .map(|snapshot| snapshot.documents.len())
                .unwrap_or_default(),
            dirty_buffers: snapshot
                .as_ref()
                .map(|snapshot| {
                    snapshot
                        .documents
                        .iter()
                        .filter(|document| document.dirty && document.buffer.is_some())
                        .count()
                })
                .unwrap_or_default(),
            last_error: session_error,
        };
        Ok(ProjectOperationsDto {
            project_path: project_path.to_string_lossy().to_string(),
            autosave: AutosaveStatusDto {
                enabled: game
                    .engine_config
                    .get("autosave")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                interval_seconds: autosave.interval.as_secs(),
                exists: autosave.autosave_exists(),
                health: autosave.health().to_string(),
                path: autosave.path.to_string_lossy().to_string(),
                backup_path: autosave.backup_path.to_string_lossy().to_string(),
                last_error: autosave.last_error.clone(),
                recoveries,
            },
            session,
            last_operation: self.last_project_operation.clone(),
            external_launch: self.external_launch_plan.clone(),
        })
    }

    /// Executes one project/recovery/build mutation. The C ABI exposes this as
    /// a status-only call; structured results are read afterwards through
    /// `project_operations`, so destructive actions are never buffer-probed.
    pub fn project_operation(
        &mut self,
        action: &str,
        payload_json: &str,
    ) -> Result<(), EditorCoreError> {
        let payload = serde_json::from_str::<Value>(payload_json)?;
        let action = action.trim().to_ascii_lowercase();
        let outcome = match action.as_str() {
            "package_export" => {
                let report = self.game_mut()?.export_project_package()?;
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: format!("Project package exported: {} files", report.files),
                    artifact_path: Some(report.archive_path.to_string_lossy().to_string()),
                    files: report.files,
                    bytes: report.bytes,
                }
            }
            "package_import" => {
                let archive =
                    validated_external_file(required_payload_string(&payload, "archive_path")?)?;
                let destination = validated_external_directory(required_payload_string(
                    &payload,
                    "destination_root",
                )?)?;
                let report = self
                    .game_mut()?
                    .import_project_package(&archive, &destination)?;
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: format!("Project package imported: {} files", report.files),
                    artifact_path: Some(report.project_path.to_string_lossy().to_string()),
                    files: report.files,
                    bytes: report.bytes,
                }
            }
            "package_distributable" => {
                let profile = parse_export_profile(
                    payload
                        .get("profile")
                        .and_then(Value::as_str)
                        .unwrap_or("release"),
                )?;
                let label = safe_package_label(
                    payload
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("game"),
                )?;
                let report = self.game_mut()?.package_distributable(profile, label)?;
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: format!("{} package ready", profile.label()),
                    artifact_path: Some(report.destination.to_string_lossy().to_string()),
                    files: report.export.copied_files,
                    bytes: 0,
                }
            }
            "autosave_configure" => {
                let enabled = payload
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| {
                        EditorCoreError::new(
                            EditorCoreErrorKind::InvalidArgument,
                            "autosave_configure requires enabled:boolean",
                        )
                    })?;
                let interval_seconds = payload
                    .get("interval_seconds")
                    .and_then(Value::as_u64)
                    .unwrap_or(60)
                    .clamp(5, 3600);
                let game = self.game_mut()?;
                game.engine_config.data["autosave"] = Value::Bool(enabled);
                game.engine_config.data["autosave_interval_seconds"] =
                    Value::from(interval_seconds);
                game.engine_config.save()?;
                game.autosave_manager.interval = Duration::from_secs(interval_seconds);
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: format!(
                        "Autosave {} every {interval_seconds}s",
                        if enabled { "enabled" } else { "disabled" }
                    ),
                    artifact_path: None,
                    files: 0,
                    bytes: 0,
                }
            }
            "autosave_now" => {
                let game = self.game_mut()?;
                game.autosave_manager.save(&mut game.runtime_world.units)?;
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: "Scene autosave checkpoint written".to_string(),
                    artifact_path: Some(game.autosave_manager.path.to_string_lossy().to_string()),
                    files: 1,
                    bytes: fs::metadata(&game.autosave_manager.path)
                        .map(|metadata| metadata.len())
                        .unwrap_or_default(),
                }
            }
            "autosave_recover" => {
                self.game_mut()?
                    .recover_from_autosave()
                    .map_err(|message| {
                        EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message)
                    })?;
                self.refresh_scene_cache();
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: "Scene restored from autosave".to_string(),
                    artifact_path: None,
                    files: 1,
                    bytes: 0,
                }
            }
            "session_checkpoint" => {
                let project_path = self
                    .project_path
                    .clone()
                    .ok_or_else(EditorCoreError::no_project)?;
                let ui = payload
                    .get("ui")
                    .cloned()
                    .map(serde_json::from_value::<SessionUiState>)
                    .transpose()?
                    .unwrap_or_default();
                let mut recovery =
                    SessionRecoveryManager::new(&project_path, Duration::from_secs(10));
                let game = self.game_mut()?;
                let report = recovery.checkpoint(
                    &game.scene_manager.current_scene,
                    game.scene_dirty,
                    &game.scene_dirty_reason,
                    &mut game.script_editor,
                    ui,
                )?;
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: format!(
                        "Session checkpoint: {} documents, {} dirty buffers",
                        report.documents, report.dirty_buffers
                    ),
                    artifact_path: Some(report.path.to_string_lossy().to_string()),
                    files: report.documents,
                    bytes: fs::metadata(&report.path)
                        .map(|metadata| metadata.len())
                        .unwrap_or_default(),
                }
            }
            "session_restore" => {
                let project_path = self
                    .project_path
                    .clone()
                    .ok_or_else(EditorCoreError::no_project)?;
                let recovery = SessionRecoveryManager::new(&project_path, Duration::from_secs(10));
                let snapshot = recovery.load_pending()?.ok_or_else(|| {
                    EditorCoreError::new(
                        EditorCoreErrorKind::NotFound,
                        "No pending editor session checkpoint",
                    )
                })?;
                let report =
                    recovery.restore_script_editor(&snapshot, &mut self.game_mut()?.script_editor);
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: format!(
                        "Session restored: {} documents, {} dirty buffers",
                        report.restored_documents, report.restored_dirty_buffers
                    ),
                    artifact_path: Some(recovery.path.to_string_lossy().to_string()),
                    files: report.restored_documents,
                    bytes: 0,
                }
            }
            "session_clear" => {
                let project_path = self
                    .project_path
                    .clone()
                    .ok_or_else(EditorCoreError::no_project)?;
                let mut recovery =
                    SessionRecoveryManager::new(&project_path, Duration::from_secs(10));
                recovery.clear()?;
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: "Session recovery checkpoint cleared".to_string(),
                    artifact_path: None,
                    files: 0,
                    bytes: 0,
                }
            }
            "prepare_wgpu_preview" => {
                let project_path = self
                    .project_path
                    .clone()
                    .ok_or_else(EditorCoreError::no_project)?;
                let executable =
                    crate::engine::packaging_manager::PackagingManager::wgpu_preview_binary();
                let ready = executable.is_some();
                let warnings = if ready {
                    Vec::new()
                } else {
                    vec![
                        "wgpu preview executable unavailable; build miniforge_wgpu_preview with the wgpu_runtime feature or set MINIFORGE_WGPU_PREVIEW"
                            .to_string(),
                    ]
                };
                let plan = ExternalLaunchPlanDto {
                    kind: "wgpu-preview".to_string(),
                    profile: "development".to_string(),
                    ready,
                    executable: executable.map(|path| path.to_string_lossy().to_string()),
                    arguments: vec![project_path.to_string_lossy().to_string()],
                    working_directory: project_path.to_string_lossy().to_string(),
                    artifact_path: project_path.to_string_lossy().to_string(),
                    warnings,
                };
                self.external_launch_plan = Some(plan.clone());
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: if plan.ready {
                        "Native wgpu project preview prepared".to_string()
                    } else {
                        "wgpu preview executable is missing".to_string()
                    },
                    artifact_path: Some(plan.artifact_path.clone()),
                    files: 0,
                    bytes: 0,
                }
            }
            "prepare_external_play" => {
                let profile = parse_export_profile(
                    payload
                        .get("profile")
                        .and_then(Value::as_str)
                        .unwrap_or("debug"),
                )?;
                let report = self.game_mut()?.export_runtime(profile)?;
                let runtime = crate::engine::packaging_manager::PackagingManager::runtime_binary();
                let plan = external_launch_plan(
                    "play",
                    profile,
                    runtime,
                    report.output_path.clone(),
                    Vec::new(),
                );
                self.last_export_report = Some(report);
                self.external_launch_plan = Some(plan.clone());
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: if plan.ready {
                        "External play build prepared".to_string()
                    } else {
                        "Play build prepared; runtime executable is missing".to_string()
                    },
                    artifact_path: Some(plan.artifact_path.clone()),
                    files: 0,
                    bytes: 0,
                }
            }
            "prepare_external_build" => {
                let profile = parse_export_profile(
                    payload
                        .get("profile")
                        .and_then(Value::as_str)
                        .unwrap_or("release"),
                )?;
                let label = safe_package_label(
                    payload
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("game"),
                )?;
                let report = self.game_mut()?.package_distributable(profile, label)?;
                let plan = external_launch_plan(
                    "build",
                    profile,
                    report.runtime_binary_copied.clone(),
                    report.destination.clone(),
                    report.warnings.clone(),
                );
                self.external_launch_plan = Some(plan.clone());
                ProjectOperationOutcomeDto {
                    action: action.clone(),
                    message: if plan.ready {
                        "External packaged build prepared".to_string()
                    } else {
                        "Package prepared without a runtime executable".to_string()
                    },
                    artifact_path: Some(plan.artifact_path.clone()),
                    files: report.export.copied_files,
                    bytes: 0,
                }
            }
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown project operation: {action}"),
                ));
            }
        };
        self.last_project_operation = Some(outcome);
        Ok(())
    }

    pub fn luau_scripts(&self) -> Result<Vec<LuauScriptRow>, EditorCoreError> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or_else(EditorCoreError::no_project)?;
        let scripts_path = project_path.join("scripts");
        if !scripts_path.exists() {
            return Ok(Vec::new());
        }

        let mut scripts = Vec::new();
        for entry in WalkDir::new(&scripts_path)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file() && is_luau_path(entry.path()))
        {
            let path = entry.path();
            let metadata = entry.metadata().map_err(|error| {
                EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string())
            })?;
            let relative_path = project_relative(project_path, path);
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("script.luau")
                .to_string();
            let diagnostic = if metadata.len() > MAX_EDITOR_SCRIPT_BYTES as u64 {
                Some(format!(
                    "Script exceeds the editor limit of {} MiB",
                    MAX_EDITOR_SCRIPT_BYTES / (1024 * 1024)
                ))
            } else {
                match fs::read_to_string(path) {
                    Ok(source) => LuauScriptRuntime::validate_source(&source, &name).err(),
                    Err(error) => Some(error.to_string()),
                }
            };
            scripts.push(LuauScriptRow {
                relative_path,
                name,
                bytes: metadata.len(),
                valid: diagnostic.is_none(),
                diagnostic,
            });
        }
        scripts.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(scripts)
    }

    pub fn read_luau_script(&self, relative_path: &str) -> Result<String, EditorCoreError> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or_else(EditorCoreError::no_project)?;
        let path = resolve_luau_script_path(project_path, relative_path, true)?;
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_EDITOR_SCRIPT_BYTES as u64 {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!(
                    "Luau script is larger than the editor limit of {} MiB",
                    MAX_EDITOR_SCRIPT_BYTES / (1024 * 1024)
                ),
            ));
        }
        fs::read_to_string(path).map_err(EditorCoreError::from)
    }

    pub fn validate_luau_source(
        &self,
        relative_path: &str,
        source: &str,
    ) -> Result<LuauValidationResult, EditorCoreError> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or_else(EditorCoreError::no_project)?;
        let path = resolve_luau_script_path(project_path, relative_path, false)?;
        validate_editor_script_size(source)?;
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("script.luau");
        let source_diagnostic = LuauScriptRuntime::validate_source_diagnostics(source, name)
            .into_iter()
            .next();
        Ok(LuauValidationResult {
            valid: source_diagnostic.is_none(),
            diagnostic: source_diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.display_message()),
            line: source_diagnostic
                .as_ref()
                .and_then(|diagnostic| diagnostic.line),
            column: source_diagnostic
                .as_ref()
                .and_then(|diagnostic| diagnostic.column),
            incomplete_input: source_diagnostic
                .as_ref()
                .is_some_and(|diagnostic| diagnostic.incomplete_input),
        })
    }

    pub fn save_luau_script(
        &mut self,
        relative_path: &str,
        source: &str,
    ) -> Result<(), EditorCoreError> {
        let validation = self.validate_luau_source(relative_path, source)?;
        if let Some(diagnostic) = validation.diagnostic {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::CommandFailed,
                format!("Luau validation failed: {diagnostic}"),
            ));
        }
        let project_path = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let path = resolve_luau_script_path(&project_path, relative_path, false)?;
        if path.exists() {
            ProjectStorage::write_atomic_with_backup(
                &path,
                source.as_bytes(),
                BackupPolicy::new(luau_backup_path(&path), DEFAULT_BACKUP_GENERATIONS),
            )
            .map_err(|error| EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string()))?;
        } else {
            ProjectStorage::write_atomic(&path, source.as_bytes()).map_err(|error| {
                EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string())
            })?;
        }

        {
            let game = self.game_mut()?;
            game.asset_database.scan()?;
            game.console
                .log(format!("Luau script saved: {relative_path}"), "LUAU");
        }
        self.refresh_asset_cache();
        Ok(())
    }

    pub fn luau_debug_state(&self) -> Result<ScriptDebuggerState, EditorCoreError> {
        Ok(self.game()?.luau_script_runtime.debugger_state())
    }

    pub fn set_luau_breakpoints(
        &mut self,
        breakpoints: Vec<ScriptBreakpoint>,
    ) -> Result<(), EditorCoreError> {
        self.game_mut()?
            .luau_script_runtime
            .set_debug_breakpoints(breakpoints);
        Ok(())
    }

    pub fn luau_debug_command(&mut self, command: &str) -> Result<bool, EditorCoreError> {
        let runtime = &mut self.game_mut()?.luau_script_runtime;
        match command.trim().to_ascii_lowercase().as_str() {
            "pause" => {
                runtime.request_debug_pause();
                Ok(true)
            }
            "resume" | "continue" => Ok(runtime.resume_debugger()),
            "step" | "step_callback" => Ok(runtime.step_debugger()),
            other => Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("unknown Luau debugger command: {other}"),
            )),
        }
    }

    pub fn evaluate_luau_watches(
        &self,
        expressions: &[String],
    ) -> Result<Vec<ScriptWatchResult>, EditorCoreError> {
        Ok(self
            .game()?
            .luau_script_runtime
            .evaluate_debug_watches(expressions))
    }

    pub fn validate_visual_graph_source(
        &self,
        relative_path: &str,
        source: &str,
    ) -> Result<Value, EditorCoreError> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or_else(EditorCoreError::no_project)?;
        resolve_visual_graph_path(project_path, relative_path, false)?;
        validate_editor_script_size(source)?;
        let value: Value = serde_json::from_str(source)?;
        let migration = VisualGraphSerializer::try_migrate(value).map_err(|error| {
            EditorCoreError::new(EditorCoreErrorKind::InvalidArgument, error.to_string())
        })?;
        Ok(json!({
            "valid": true,
            "from_version": migration.from_version,
            "to_version": migration.to_version,
            "changed": migration.changed,
            "warnings": migration.warnings,
            "node_count": migration.data.get("nodes").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "normalized": migration.data,
        }))
    }

    pub fn save_visual_graph(
        &mut self,
        relative_path: &str,
        source: &str,
    ) -> Result<(), EditorCoreError> {
        let project_path = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let path = resolve_visual_graph_path(&project_path, relative_path, false)?;
        let validation = self.validate_visual_graph_source(relative_path, source)?;
        let normalized = validation.get("normalized").cloned().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "visual graph normalization failed",
            )
        })?;
        let bytes = serde_json::to_vec_pretty(&normalized)?;
        if path.exists() {
            ProjectStorage::write_atomic_with_backup(
                &path,
                &bytes,
                BackupPolicy::new(
                    path.with_extension("mfgraph.backup"),
                    DEFAULT_BACKUP_GENERATIONS,
                ),
            )
            .map_err(|error| EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string()))?;
        } else {
            ProjectStorage::write_atomic(&path, &bytes).map_err(|error| {
                EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string())
            })?;
        }
        let game = self.game_mut()?;
        game.asset_database.scan()?;
        game.console
            .log(format!("Visual Graph saved: {relative_path}"), "BLUEPRINT");
        self.refresh_asset_cache();
        Ok(())
    }

    pub fn visual_graph_catalog(&self) -> Result<Value, EditorCoreError> {
        let programming = &self.game()?.programming;
        Ok(json!({
            "nodes": programming.node_catalog().into_iter().map(|node| {
                let output_pins = programming
                    .graph_view(&json!({"name":"Preview","nodes":[node.default_node.clone()]}))
                    .nodes
                    .into_iter()
                    .next()
                    .map(|view| view.output_pins)
                    .unwrap_or_default();
                json!({
                    "type": node.node_type,
                    "category": node.category,
                    "title": node.label,
                    "detail": node.description,
                    "defaults": node.default_node,
                    "output_pins": output_pins,
                })
            }).collect::<Vec<_>>(),
            "templates": programming.templates.iter().map(|template| json!({
                "name": template.name,
                "description": template.description,
            })).collect::<Vec<_>>(),
            "quick_actions": programming.quick_actions().into_iter().map(|action| json!({
                "label": action.label,
                "description": action.description,
                "node_types": action.node_types,
            })).collect::<Vec<_>>(),
        }))
    }

    pub fn create_visual_graph_from_template(
        &mut self,
        relative_path: &str,
        template_name: &str,
    ) -> Result<(), EditorCoreError> {
        let graph = self.game()?.programming.template_graph(template_name);
        let source = serde_json::to_string_pretty(&graph)?;
        self.save_visual_graph(relative_path, &source)
    }

    pub fn python_tools_state(&self) -> Result<Value, EditorCoreError> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or_else(EditorCoreError::no_project)?;
        let host = PythonAutomationHost::new(project_path);
        let tools = host.discover()?;
        Ok(json!({
            "interpreter": host.interpreter_version().unwrap_or_else(|error| format!("Unavailable: {error}")),
            "tools": tools,
        }))
    }

    pub fn install_python_tools(&mut self) -> Result<Value, EditorCoreError> {
        let project_path = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let installed = PythonAutomationHost::new(&project_path).install_builtin_tools()?;
        self.game_mut()?.console.log(
            format!(
                "Python automation installed/refreshed: {} files",
                installed.len()
            ),
            "PYTHON",
        );
        let report = json!({
            "installed": installed.iter().map(|path| project_relative(&project_path, path)).collect::<Vec<_>>(),
            "count": installed.len(),
        });
        self.last_python_result = Some(report.clone());
        Ok(report)
    }

    pub fn run_python_tool(
        &mut self,
        tool_id: &str,
        parameters: Value,
    ) -> Result<Value, EditorCoreError> {
        let project_path = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let (active_scene, selected_entity_ids, assets) = {
            let game = self.game()?;
            (
                Some(game.scene_manager.current_scene.clone()),
                game.selected_units.clone(),
                game.asset_database
                    .assets
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        };
        let host = PythonAutomationHost::new(&project_path);
        let manifest = host
            .discover()?
            .into_iter()
            .find(|tool| tool.id == tool_id)
            .ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Python tool not found: {tool_id}"),
                )
            })?;
        let result = host.run(
            &manifest,
            PythonEditorContext {
                project_root: project_path.to_string_lossy().to_string(),
                active_scene,
                selected_entity_ids,
                assets,
                parameters,
            },
        )?;
        let mut applied = 0usize;
        let mut refresh_assets = false;
        let mut operation_reports = Vec::new();
        for operation in &result.operations {
            let report = match operation.operation.as_str() {
                "batch_import_assets" => {
                    let destination = operation
                        .value
                        .get("destination")
                        .and_then(Value::as_str)
                        .unwrap_or("assets/imported");
                    Some(serde_json::to_value(batch_import_assets(
                        &project_path,
                        &operation.target,
                        destination,
                    )?)?)
                }
                "convert_sprites" => {
                    let destination = operation
                        .value
                        .get("destination")
                        .and_then(Value::as_str)
                        .unwrap_or("assets/sprites/converted");
                    Some(serde_json::to_value(batch_convert_sprites(
                        &project_path,
                        &operation.target,
                        destination,
                    )?)?)
                }
                "generate_atlas" => {
                    let destination = operation
                        .value
                        .get("destination")
                        .and_then(Value::as_str)
                        .unwrap_or("assets/atlases");
                    let size = operation
                        .value
                        .get("size")
                        .and_then(Value::as_u64)
                        .unwrap_or(4096)
                        .min(u32::MAX as u64) as u32;
                    let extrude = operation
                        .value
                        .get("extrude")
                        .and_then(Value::as_u64)
                        .unwrap_or(1)
                        .min(u32::MAX as u64) as u32;
                    Some(serde_json::to_value(generate_paged_sprite_atlases(
                        &project_path,
                        &operation.target,
                        destination,
                        size,
                        extrude,
                    )?)?)
                }
                _ => None,
            };
            if let Some(report) = report {
                applied += 1;
                refresh_assets = true;
                operation_reports.push(json!({
                    "operation": operation.operation,
                    "target": operation.target,
                    "report": report,
                }));
            }
        }
        {
            let game = self.game_mut()?;
            game.console.log(
                if result.message.is_empty() {
                    format!("Python tool completed: {}", manifest.label)
                } else {
                    result.message.clone()
                },
                if result.success { "PYTHON" } else { "ERROR" },
            );
            for operation in &result.operations {
                match operation.operation.as_str() {
                    "log" => {
                        game.console
                            .log(operation.value.as_str().unwrap_or_default(), "PYTHON");
                        applied += 1;
                    }
                    "select_entities" => {
                        game.selected_units = operation
                            .value
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_u64)
                            .filter(|id| {
                                game.runtime_world
                                    .units
                                    .iter()
                                    .any(|entity| entity.id == *id)
                            })
                            .collect();
                        applied += 1;
                    }
                    "set_editor_property" if operation.target == "selection" => {
                        let Some(properties) = operation.value.as_object() else {
                            continue;
                        };
                        let before = game.capture_editor_snapshot();
                        let mut changed = 0usize;
                        for id in game.selected_units.clone() {
                            let Some(entity) = game.get_entity_by_id_mut(id) else {
                                continue;
                            };
                            let old = (entity.visible, entity.locked, entity.enabled);
                            if let Some(value) = properties.get("visible").and_then(Value::as_bool)
                            {
                                entity.visible = value;
                            }
                            if let Some(value) = properties.get("locked").and_then(Value::as_bool) {
                                entity.locked = value;
                            }
                            if let Some(value) = properties.get("enabled").and_then(Value::as_bool)
                            {
                                entity.enabled = value;
                            }
                            if old != (entity.visible, entity.locked, entity.enabled) {
                                entity.sync_to_components();
                                changed += 1;
                            }
                        }
                        if changed > 0 {
                            game.sync_world();
                            game.mark_scene_dirty("Python: Set Selection Properties");
                            game.push_editor_command(
                                "Python: Set Selection Properties",
                                EditorCommandKind::SceneOperation {
                                    name: "Python automation".to_string(),
                                },
                                before,
                            );
                            applied += 1;
                            operation_reports.push(json!({
                                "operation": operation.operation,
                                "target": operation.target,
                                "changed_entities": changed,
                            }));
                        }
                    }
                    "request_reimport" | "refresh_assets" => {
                        refresh_assets = true;
                        applied += 1;
                    }
                    _ => {}
                }
            }
            if refresh_assets {
                game.asset_database.scan()?;
            }
        }
        self.refresh_all_caches();
        let report = json!({
            "tool": manifest,
            "result": result,
            "applied_operations": applied,
            "operation_reports": operation_reports,
        });
        self.last_python_result = Some(report.clone());
        Ok(report)
    }

    pub fn last_python_result(&self) -> Result<&Value, EditorCoreError> {
        self.game()?;
        self.last_python_result.as_ref().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                "No Python automation action has completed in this editor session",
            )
        })
    }

    pub fn export_runtime_profile(&mut self, profile: &str) -> Result<(), EditorCoreError> {
        let profile = parse_export_profile(profile)?;
        let report = self.game_mut()?.export_runtime(profile)?;
        self.last_export_report = Some(report);
        self.refresh_all_caches();
        Ok(())
    }

    pub fn last_export_report(&self) -> Result<&RuntimeExportReport, EditorCoreError> {
        self.game()?;
        self.last_export_report.as_ref().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                "No runtime export has completed in this editor session",
            )
        })
    }

    pub fn entity_count(&self) -> Result<usize, EditorCoreError> {
        self.game()?;
        Ok(self.entity_cache.len())
    }

    pub fn entity_at(&self, index: usize) -> Result<EntityRow, EditorCoreError> {
        self.game()?;
        self.entity_cache.get(index).cloned().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                format!("Entity index out of range: {index}"),
            )
        })
    }

    pub fn entity_row(&self, entity_id: u64) -> Result<EntityRow, EditorCoreError> {
        self.game()?;
        self.entity_cache
            .iter()
            .find(|entity| entity.id == entity_id)
            .cloned()
            .ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Entity not found: {entity_id}"),
                )
            })
    }

    pub fn selected_entities(&self) -> Result<Vec<u64>, EditorCoreError> {
        self.game()?;
        Ok(self.selected_cache.clone())
    }

    pub fn select_entity(&mut self, entity_id: u64) -> Result<(), EditorCoreError> {
        {
            let game = self.game_mut()?;
            if !game.select_entity(entity_id) {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Entity not selectable or not found: {entity_id}"),
                ));
            }
        }
        self.refresh_scene_cache();
        Ok(())
    }

    pub fn update_selection(&mut self, entity_id: u64, mode: &str) -> Result<(), EditorCoreError> {
        let game = self.game_mut()?;
        if game.get_entity_by_id(entity_id).is_none() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                format!("Entity not found: {entity_id}"),
            ));
        }
        match mode {
            "replace" => {
                if !game.select_entity(entity_id) {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        format!("Entity is not selectable: {entity_id}"),
                    ));
                }
            }
            "add" => {
                if !game.select_entity_additive(entity_id) {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        format!("Entity is not selectable: {entity_id}"),
                    ));
                }
            }
            "toggle" => {
                if game.selected_units.contains(&entity_id) {
                    game.toggle_entity_selection(entity_id);
                } else if !game.toggle_entity_selection(entity_id) {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        format!("Entity is not selectable: {entity_id}"),
                    ));
                }
            }
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown selection mode: {mode}"),
                ));
            }
        }
        self.refresh_scene_cache();
        Ok(())
    }

    pub fn clear_selection(&mut self) -> Result<(), EditorCoreError> {
        self.game_mut()?.clear_selection();
        self.refresh_scene_cache();
        Ok(())
    }

    pub fn entity_action(
        &mut self,
        entity_id: u64,
        action: &str,
        payload_json: &str,
    ) -> Result<Option<u64>, EditorCoreError> {
        let payload = if payload_json.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str::<Value>(payload_json)?
        };
        if !payload.is_object() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Entity action payload must be a JSON object",
            ));
        }
        if self.game()?.get_entity_by_id(entity_id).is_none() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                format!("Entity not found: {entity_id}"),
            ));
        }

        let result = match action {
            "rename" => {
                let name = required_payload_string(&payload, "name")?;
                if name.trim().is_empty() {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        "Entity name cannot be empty",
                    ));
                }
                self.game_mut()?
                    .edit_inspector_value(entity_id, "Transform", "name", json!(name))
                    .map_err(|message| {
                        EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message)
                    })?;
                None
            }
            "duplicate" => Some(
                self.game_mut()?
                    .duplicate_entity(entity_id)
                    .ok_or_else(|| {
                        EditorCoreError::new(
                            EditorCoreErrorKind::CommandFailed,
                            "Entity was not duplicated",
                        )
                    })?,
            ),
            "delete" => {
                if !self.game_mut()?.delete_entity(entity_id) {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Entity was not deleted",
                    ));
                }
                None
            }
            "reparent" => {
                let parent_id = required_payload_u64(&payload, "parent_id")?;
                if !self.game_mut()?.set_entity_parent(entity_id, parent_id) {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Invalid hierarchy operation (missing node or cycle)",
                    ));
                }
                None
            }
            "unparent" => {
                if !self.game_mut()?.clear_entity_parent(entity_id) {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Entity parent was not cleared",
                    ));
                }
                None
            }
            "set_visible" | "set_enabled" | "set_locked" => {
                let value = required_payload_bool(&payload, "value")?;
                let key = action.trim_start_matches("set_");
                self.game_mut()?
                    .edit_inspector_value(entity_id, "Identity", key, json!(value))
                    .map_err(|message| {
                        EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message)
                    })?;
                None
            }
            "apply_asset" => {
                let relative_path = required_payload_string(&payload, "relative_path")?;
                let record = self
                    .game()?
                    .asset_database
                    .assets
                    .get(relative_path)
                    .cloned()
                    .ok_or_else(|| {
                        EditorCoreError::new(
                            EditorCoreErrorKind::NotFound,
                            format!("Indexed asset not found: {relative_path}"),
                        )
                    })?;
                let asset = asset_from_record(&record);
                if !matches!(
                    asset.asset_type.as_str(),
                    "Sprite2D"
                        | "SpriteSheet"
                        | "AnimationBlueprint2D"
                        | "FlipbookAnimation2D"
                        | "Animation"
                        | "BlueprintGraph2D"
                        | "Script"
                        | "LuauScript"
                        | "Material"
                        | "Material2D"
                        | "Shader"
                        | "Texture"
                        | "Texture2D"
                        | "Image"
                        | "ImageTexture2D"
                ) {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        format!(
                            "Asset type cannot be assigned to an entity: {}",
                            record.asset_type
                        ),
                    ));
                }
                let game = self.game_mut()?;
                let before = game.capture_editor_snapshot();
                let entity = game.get_entity_by_id_mut(entity_id).ok_or_else(|| {
                    EditorCoreError::new(EditorCoreErrorKind::NotFound, "Entity not found")
                })?;
                let report = EditorAssetConnector::apply_content_asset(entity, &asset);
                game.sync_world();
                game.mark_scene_dirty("Apply Asset");
                game.scene_save_manager.note_entity_dirty(entity_id);
                game.push_editor_command(
                    "Apply Asset",
                    EditorCommandKind::SceneOperation {
                        name: format!("Apply {}", record.asset_type),
                    },
                    before,
                );
                game.console.log(
                    format!(
                        "Applied {} to entity #{} ({})",
                        relative_path, entity_id, report.asset_type
                    ),
                    "EDITOR",
                );
                None
            }
            "collision_vertex_move" | "collision_vertex_add" | "collision_vertex_remove" => {
                let index = if action == "collision_vertex_add" {
                    None
                } else {
                    Some(required_payload_u64(&payload, "index")? as usize)
                };
                let local = if action == "collision_vertex_remove" {
                    None
                } else {
                    Some((
                        required_payload_f64(&payload, "x")?,
                        required_payload_f64(&payload, "y")?,
                    ))
                };
                let game = self.game_mut()?;
                let before = game.capture_editor_snapshot();
                let entity = game.get_entity_by_id_mut(entity_id).ok_or_else(|| {
                    EditorCoreError::new(EditorCoreErrorKind::NotFound, "Entity not found")
                })?;
                let changed = match action {
                    "collision_vertex_move" => EditorSpatialTools2D::move_collision_vertex(
                        entity,
                        index.expect("validated index"),
                        local.expect("validated local point"),
                        None,
                    ),
                    "collision_vertex_add" => EditorSpatialTools2D::add_collision_vertex(
                        entity,
                        local.expect("validated local point"),
                    ),
                    "collision_vertex_remove" => EditorSpatialTools2D::remove_collision_vertex(
                        entity,
                        index.expect("validated index"),
                    ),
                    _ => unreachable!(),
                };
                if !changed {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        format!("Collision vertex action could not be applied: {action}"),
                    ));
                }
                game.sync_world();
                game.mark_scene_dirty("Edit Collision Polygon");
                game.scene_save_manager.note_entity_dirty(entity_id);
                game.push_editor_command(
                    "Edit Collision Polygon",
                    EditorCommandKind::SceneOperation {
                        name: action.to_string(),
                    },
                    before,
                );
                None
            }
            "reset_transform" => {
                let game = self.game_mut()?;
                let before = game.capture_editor_snapshot();
                let entity = game.get_entity_by_id_mut(entity_id).ok_or_else(|| {
                    EditorCoreError::new(EditorCoreErrorKind::NotFound, "Entity not found")
                })?;
                InspectorEditor::reset_transform(entity);
                game.sync_world();
                game.mark_scene_dirty("Reset Transform");
                game.push_editor_command(
                    "Reset Transform",
                    EditorCommandKind::SceneOperation {
                        name: "Reset Transform".to_string(),
                    },
                    before,
                );
                None
            }
            "add_component" => {
                let component_type = required_payload_string(&payload, "component_type")?;
                if !self
                    .game_mut()?
                    .add_component_to_entity(entity_id, component_type)
                {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        format!("Unknown component type: {component_type}"),
                    ));
                }
                None
            }
            "add_component_bundle" => {
                let bundle = required_payload_string(&payload, "bundle")?;
                let _ = add_component_bundle_to_entities(self.game_mut()?, &[entity_id], bundle)?;
                None
            }
            "remove_component" => {
                let component_type = required_payload_string(&payload, "component_type")?;
                self.game_mut()?
                    .remove_component_from_entity(entity_id, component_type)
                    .map_err(|message| {
                        EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message)
                    })?;
                None
            }
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown entity action: {action}"),
                ));
            }
        };
        self.refresh_scene_cache();
        Ok(result)
    }

    pub fn selected_entity_action(
        &mut self,
        action: &str,
        payload_json: &str,
    ) -> Result<usize, EditorCoreError> {
        let payload = if payload_json.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str::<Value>(payload_json)?
        };
        if !payload.is_object() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Selected entity action payload must be a JSON object",
            ));
        }
        let changed = match action {
            "duplicate" => duplicate_selected_entities(self.game_mut()?)?,
            "delete" => delete_selected_entities(self.game_mut()?)?,
            "add_component" => add_component_to_selected(
                self.game_mut()?,
                required_payload_string(&payload, "component_type")?,
            )?,
            "add_component_bundle" => {
                let selection = selected_entity_ids(self.game()?)?;
                add_component_bundle_to_entities(
                    self.game_mut()?,
                    &selection,
                    required_payload_string(&payload, "bundle")?,
                )?
            }
            "remove_component" => remove_component_from_selected(
                self.game_mut()?,
                required_payload_string(&payload, "component_type")?,
            )?,
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown selected entity action: {action}"),
                ));
            }
        };
        self.refresh_scene_cache();
        Ok(changed)
    }

    pub fn scene_state(&self) -> Result<SceneStateDto, EditorCoreError> {
        let game = self.game()?;
        Ok(SceneStateDto {
            scene_name: game.scene_manager.current_scene.clone(),
            dirty: game.scene_dirty,
            dirty_reason: game.scene_dirty_reason.clone(),
            mode: game.mode.clone(),
            selected_count: game.selected_units.len(),
            entity_count: game.runtime_world.units.len(),
        })
    }

    pub fn scene_browser_state(&self) -> Result<Value, EditorCoreError> {
        let game = self.game()?;
        Ok(json!({
            "scenes": game.scene_names()?,
            "current": game.scene_manager.current_scene,
            "loaded": game.scene_manager.loaded_scenes,
            "stack": game.scene_manager.scene_stack,
            "dirty": game.scene_dirty,
            "dirty_reason": game.scene_dirty_reason,
            "transition": game.scene_manager.transition,
        }))
    }

    pub fn scene_browser_action(
        &mut self,
        action: &str,
        payload_json: &str,
    ) -> Result<Value, EditorCoreError> {
        let payload = if payload_json.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str::<Value>(payload_json)?
        };
        let scene_name = || -> Result<&str, EditorCoreError> {
            let name = required_payload_string(&payload, "name")?.trim();
            if name.is_empty() {
                Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    "Scene name cannot be empty",
                ))
            } else {
                Ok(name)
            }
        };
        match action {
            "new" => {
                self.game_mut()?.create_empty_scene(scene_name()?)?;
            }
            "duplicate" => {
                self.game_mut()?
                    .scene_manager
                    .duplicate_current_scene(scene_name()?)?;
                self.game_mut()?.asset_database.scan()?;
            }
            "restart" => {
                self.game_mut()?.restart_scene()?;
            }
            "load" => {
                self.game_mut()?.load_scene(scene_name()?)?;
            }
            "additive" => {
                self.game_mut()?.load_scene_additive(scene_name()?)?;
            }
            "unload" => {
                self.game_mut()?.unload_scene(scene_name()?);
            }
            "push" => {
                self.game_mut()?.push_scene(scene_name()?)?;
            }
            "pop" => {
                if self.game_mut()?.pop_scene()?.is_none() {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Scene stack has no previous scene to restore",
                    ));
                }
            }
            "save" => {
                self.game_mut()?.save_scene()?;
            }
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown Scene Browser action: {action}"),
                ));
            }
        }
        self.refresh_scene_cache();
        self.refresh_asset_cache();
        self.scene_browser_state()
    }

    pub fn component_catalog(&self) -> Result<Vec<ComponentSubMenu>, EditorCoreError> {
        Ok(self.game()?.component_registry.submenu_model())
    }

    pub fn prefab_studio_state(&mut self) -> Result<PrefabStudioStateDto, EditorCoreError> {
        self.game()?;
        let prefab_assets = self
            .asset_cache
            .iter()
            .filter(|asset| asset.asset_type == "Prefab")
            .cloned()
            .collect();
        let selected_entity_id = self.selected_cache.first().copied();
        let selected_instance = self.game_mut()?.analyze_selected_prefab();
        Ok(PrefabStudioStateDto {
            prefab_assets,
            selected_entity_id,
            selected_instance,
        })
    }

    pub fn prefab_action(
        &mut self,
        action: &str,
        payload_json: &str,
    ) -> Result<PrefabActionResultDto, EditorCoreError> {
        let payload = if payload_json.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str::<Value>(payload_json)?
        };
        if !payload.is_object() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Prefab action payload must be a JSON object",
            ));
        }
        if let Some(entity_id) = payload.get("entity_id").and_then(Value::as_u64) {
            self.select_entity(entity_id)?;
        }

        let mut result = match action {
            "create_from_selected" => {
                let path = self
                    .game_mut()?
                    .save_selected_as_prefab()?
                    .ok_or_else(|| selected_entity_required("create a prefab"))?;
                PrefabActionResultDto {
                    changed: true,
                    path: Some(self.project_relative_path(&path)),
                    entity_id: self.selected_cache.first().copied(),
                    message: "Prefab created from the selected entity".to_string(),
                }
            }
            "create_variant" => {
                let path = self
                    .game_mut()?
                    .create_selected_prefab_variant()?
                    .ok_or_else(|| selected_entity_required("create a prefab variant"))?;
                PrefabActionResultDto {
                    changed: true,
                    path: Some(self.project_relative_path(&path)),
                    entity_id: self.selected_cache.first().copied(),
                    message: "Prefab variant created".to_string(),
                }
            }
            "apply_overrides" => {
                if !self.game_mut()?.apply_selected_to_prefab_source()? {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Selected entity is not a writable prefab instance",
                    ));
                }
                PrefabActionResultDto {
                    changed: true,
                    path: None,
                    entity_id: self.selected_cache.first().copied(),
                    message: "Prefab overrides applied to source".to_string(),
                }
            }
            "revert_overrides" => {
                if !self.game_mut()?.revert_selected_prefab_instance()? {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Selected entity is not a readable prefab instance",
                    ));
                }
                PrefabActionResultDto {
                    changed: true,
                    path: None,
                    entity_id: self.selected_cache.first().copied(),
                    message: "Prefab instance reverted to source".to_string(),
                }
            }
            "detach" => {
                if !self.game_mut()?.detach_selected_prefab_instance() {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Selected entity is not a prefab instance",
                    ));
                }
                PrefabActionResultDto {
                    changed: true,
                    path: None,
                    entity_id: self.selected_cache.first().copied(),
                    message: "Prefab instance detached".to_string(),
                }
            }
            "instantiate" => {
                let relative_path = required_payload_string(&payload, "relative_path")?;
                validate_prefab_relative_path(relative_path)?;
                let x = payload.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                let y = payload.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                if !x.is_finite() || !y.is_finite() {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        "Prefab instance coordinates must be finite",
                    ));
                }
                let entity_id = self
                    .game_mut()?
                    .instantiate_prefab_asset(relative_path, x, y)?
                    .ok_or_else(|| {
                        EditorCoreError::new(
                            EditorCoreErrorKind::NotFound,
                            format!("Prefab could not be loaded: {relative_path}"),
                        )
                    })?;
                PrefabActionResultDto {
                    changed: true,
                    path: Some(relative_path.to_string()),
                    entity_id: Some(entity_id),
                    message: format!("Prefab instantiated as entity #{entity_id}"),
                }
            }
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown prefab action: {action}"),
                ));
            }
        };
        self.refresh_all_caches();
        result.entity_id = result
            .entity_id
            .or_else(|| self.selected_cache.first().copied());
        Ok(result)
    }

    pub fn pick_entity(
        &mut self,
        viewport_width: u32,
        viewport_height: u32,
        x: f64,
        y: f64,
        selection_mode: &str,
    ) -> Result<Option<u64>, EditorCoreError> {
        if viewport_width == 0 || viewport_height == 0 || !x.is_finite() || !y.is_finite() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Viewport picking requires finite coordinates and non-zero dimensions",
            ));
        }
        let hit = {
            let game = self.game()?;
            let (tile, scale, offset_x, offset_y) =
                viewport_layout(game, viewport_width, viewport_height);
            game.runtime_world.units.iter().rev().find_map(|entity| {
                if !entity.visible || !entity.enabled || entity.locked {
                    return None;
                }
                let center_x = offset_x + (entity.x as f32 * tile) * scale;
                let center_y = offset_y + (entity.y as f32 * tile) * scale;
                let width =
                    (entity.width as f32 * entity.scale_x.abs() as f32 * tile * scale).max(4.0);
                let height =
                    (entity.height as f32 * entity.scale_y.abs() as f32 * tile * scale).max(4.0);
                let radians = -(entity.rotation as f32).to_radians();
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let local_x = dx * radians.cos() - dy * radians.sin();
                let local_y = dx * radians.sin() + dy * radians.cos();
                let contains = local_x.abs() <= width * 0.5 && local_y.abs() <= height * 0.5;
                contains.then_some(entity.id)
            })
        };
        if let Some(entity_id) = hit {
            self.update_selection(entity_id, selection_mode)?;
        } else if selection_mode == "replace" {
            self.clear_selection()?;
        } else if !matches!(selection_mode, "add" | "toggle") {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Unknown selection mode: {selection_mode}"),
            ));
        }
        Ok(hit)
    }

    pub fn viewport_state(
        &self,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<Value, EditorCoreError> {
        if viewport_width == 0 || viewport_height == 0 {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Viewport dimensions must be greater than zero",
            ));
        }
        let game = self.game()?;
        let (tile, scale, offset_x, offset_y) =
            viewport_layout(game, viewport_width, viewport_height);
        let pixels_per_unit = tile * scale;
        let entities = game
            .runtime_world
            .units
            .iter()
            .map(|entity| {
                let component_types = entity.component_types();
                let light = entity.get_component("Light2D");
                json!({
                    "id": entity.id,
                    "name": entity.name,
                    "center_x": offset_x + entity.x as f32 * pixels_per_unit,
                    "center_y": offset_y + entity.y as f32 * pixels_per_unit,
                    "width": (entity.width as f32 * entity.scale_x.abs() as f32 * pixels_per_unit).max(4.0),
                    "height": (entity.height as f32 * entity.scale_y.abs() as f32 * pixels_per_unit).max(4.0),
                    "world_x": entity.x,
                    "world_y": entity.y,
                    "rotation": entity.rotation,
                    "scale_x": entity.scale_x,
                    "scale_y": entity.scale_y,
                    "selected": game.selected_units.contains(&entity.id),
                    "visible": entity.visible,
                    "enabled": entity.enabled,
                    "locked": entity.locked,
                    "component_types": component_types,
                    "has_collision": entity.get_component("Collider2D").is_some()
                        || entity.get_component("Area2D").is_some(),
                    "is_trigger": entity.get_component("Trigger2D").is_some()
                        || entity.get_component("Area2D").is_some(),
                    "light_radius": light.map(|component| component.get_f64("radius", 5.0)).unwrap_or(0.0),
                    "light_angle": light.map(|component| component.get_f64("angle", 360.0)).unwrap_or(360.0),
                    "light_direction": light.map(|component| component.get_f64("direction", 0.0)).unwrap_or(0.0),
                    "collision_points": EditorSpatialTools2D::collision_points(entity),
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "width": viewport_width,
            "height": viewport_height,
            "grid_width": game.grid.width,
            "grid_height": game.grid.height,
            "pixels_per_unit": pixels_per_unit,
            "offset_x": offset_x,
            "offset_y": offset_y,
            "entities": entities,
        }))
    }

    pub fn transform_selection_json(
        &mut self,
        payload_json: &str,
    ) -> Result<usize, EditorCoreError> {
        let payload = serde_json::from_str::<Value>(payload_json)?;
        let Some(payload) = payload.as_object() else {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Selection transform payload must be a JSON object",
            ));
        };
        let finite = |key: &str, default: f64| -> Result<f64, EditorCoreError> {
            let value = payload.get(key).and_then(Value::as_f64).unwrap_or(default);
            if value.is_finite() {
                Ok(value)
            } else {
                Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Transform value must be finite: {key}"),
                ))
            }
        };
        let absolute = payload.get("mode").and_then(Value::as_str) == Some("absolute");
        let dx = finite("dx", 0.0)?;
        let dy = finite("dy", 0.0)?;
        let rotation_delta = finite("rotation_delta", 0.0)?;
        let scale_x_factor = finite("scale_x_factor", 1.0)?.clamp(0.001, 1000.0);
        let scale_y_factor = finite("scale_y_factor", 1.0)?.clamp(0.001, 1000.0);

        let game = self.game_mut()?;
        let selection = game.selected_units.clone();
        if selection.is_empty() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Select at least one entity before transforming",
            ));
        }
        let before = game.capture_editor_snapshot();
        let mut changed = Vec::new();
        for entity_id in selection {
            let Some(entity) = game.get_entity_by_id_mut(entity_id) else {
                continue;
            };
            if entity.locked {
                continue;
            }
            if absolute {
                entity.x = finite("x", entity.x)?;
                entity.y = finite("y", entity.y)?;
                entity.rotation = finite("rotation", entity.rotation)?;
                entity.scale_x = finite("scale_x", entity.scale_x)?.max(0.01);
                entity.scale_y = finite("scale_y", entity.scale_y)?.max(0.01);
            } else {
                entity.x += dx;
                entity.y += dy;
                entity.rotation += rotation_delta;
                entity.scale_x = (entity.scale_x * scale_x_factor).max(0.01);
                entity.scale_y = (entity.scale_y * scale_y_factor).max(0.01);
            }
            entity.sync_to_components();
            changed.push(entity_id);
        }
        if changed.is_empty() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::CommandFailed,
                "The current selection is locked or unavailable",
            ));
        }
        game.sync_world();
        game.mark_scene_dirty("Transform Selection");
        for entity_id in &changed {
            game.scene_save_manager.note_entity_dirty(*entity_id);
        }
        game.push_editor_command(
            "Transform Selection",
            EditorCommandKind::SceneOperation {
                name: "Transform Selection".to_string(),
            },
            before,
        );
        let count = changed.len();
        self.refresh_scene_cache();
        Ok(count)
    }

    pub fn inspector_fields(
        &self,
        entity_id: u64,
    ) -> Result<Vec<InspectorFieldDto>, EditorCoreError> {
        self.game()?;
        self.inspector_cache
            .get(&entity_id)
            .cloned()
            .ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Entity not found: {entity_id}"),
                )
            })
    }

    pub fn inspector_quick_actions(
        &self,
        entity_id: u64,
    ) -> Result<Vec<InspectorQuickActionDto>, EditorCoreError> {
        let entity = self
            .game()?
            .get_entity_by_id(entity_id)
            .cloned()
            .ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Entity not found: {entity_id}"),
                )
            })?;
        let attached_script = entity
            .script
            .clone()
            .filter(|path| !path.trim().is_empty())
            .or_else(|| component_asset_path(&entity, "ScriptComponent", "path"));
        let attached_blueprint = component_asset_path(&entity, "VisualScript", "graph_path")
            .or_else(|| {
                entity.scripts.iter().find_map(|binding| {
                    (binding.get("runtime").and_then(Value::as_str) == Some("visual_graph"))
                        .then(|| {
                            binding
                                .get("path")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                        })
                        .flatten()
                })
            });

        Ok(InspectorEditor::quick_actions(&entity)
            .into_iter()
            .map(|action| {
                let (requires_asset, asset_type, attached_asset_path) = match action.id.as_str() {
                    "assign_sprite" => (true, "Sprite", None),
                    "assign_material" => (true, "Material", None),
                    "assign_texture_slot" => (true, "Texture2D", None),
                    "attach_script" => (true, "LuauScript", None),
                    "open_script" => (false, "LuauScript", attached_script.clone()),
                    "attach_blueprint" => (true, "VisualGraph", None),
                    "open_blueprint" => (false, "VisualGraph", attached_blueprint.clone()),
                    _ => (false, "", None),
                };
                let assets = inspector_assets_for_action(&self.asset_cache, &action.id);
                let (enabled, disabled_reason) = if requires_asset && assets.is_empty() {
                    (
                        false,
                        format!("No compatible {asset_type} assets are indexed"),
                    )
                } else if matches!(action.id.as_str(), "open_script" | "open_blueprint")
                    && attached_asset_path.is_none()
                {
                    (false, "No asset is attached to this entity".to_string())
                } else {
                    (true, String::new())
                };
                InspectorQuickActionDto {
                    id: action.id,
                    label: action.label,
                    icon: action.icon,
                    target_component: action.target_component,
                    enabled,
                    disabled_reason,
                    requires_asset,
                    asset_type: asset_type.to_string(),
                    attached_asset_path,
                    assets,
                }
            })
            .collect())
    }

    pub fn execute_inspector_quick_action(
        &mut self,
        entity_id: u64,
        action_id: &str,
        asset_path: &str,
    ) -> Result<InspectorQuickActionOutcomeDto, EditorCoreError> {
        let descriptor = self
            .inspector_quick_actions(entity_id)?
            .into_iter()
            .find(|action| action.id == action_id)
            .ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown Inspector quick action: {action_id}"),
                )
            })?;
        if !descriptor.enabled {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::CommandFailed,
                descriptor.disabled_reason,
            ));
        }
        let selected_asset = if descriptor.requires_asset {
            let asset_path = asset_path.trim();
            if asset_path.is_empty() {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("{} requires a compatible asset", descriptor.label),
                ));
            }
            if !descriptor
                .assets
                .iter()
                .any(|asset| asset.relative_path == asset_path)
            {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!(
                        "Asset is not compatible with {}: {asset_path}",
                        descriptor.label
                    ),
                ));
            }
            Some(asset_path.to_string())
        } else {
            descriptor.attached_asset_path.clone()
        };

        match action_id {
            "open_script" | "open_blueprint" => Ok(InspectorQuickActionOutcomeDto {
                changed: false,
                message: format!("Open {}", descriptor.label),
                open_asset_path: selected_asset,
                open_asset_type: Some(descriptor.asset_type),
            }),
            "add_sprite_renderer" | "add_material2d" => {
                let component_type = if action_id == "add_sprite_renderer" {
                    "SpriteRenderer"
                } else {
                    "Material2D"
                };
                self.entity_action(
                    entity_id,
                    "add_component",
                    &json!({"component_type": component_type}).to_string(),
                )?;
                Ok(InspectorQuickActionOutcomeDto {
                    changed: true,
                    message: format!("Added {component_type}"),
                    open_asset_path: None,
                    open_asset_type: None,
                })
            }
            "assign_sprite"
            | "assign_material"
            | "assign_texture_slot"
            | "attach_script"
            | "attach_blueprint" => {
                let asset_path = selected_asset.expect("asset was validated above");
                self.entity_action(
                    entity_id,
                    "apply_asset",
                    &json!({"relative_path": asset_path}).to_string(),
                )?;
                Ok(InspectorQuickActionOutcomeDto {
                    changed: true,
                    message: format!("{} complete", descriptor.label),
                    open_asset_path: None,
                    open_asset_type: None,
                })
            }
            "create_prefab" => {
                let game = self.game_mut()?;
                let original_selection = game.selected_units.clone();
                let Some(position) = original_selection.iter().position(|id| *id == entity_id)
                else {
                    return Err(EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Inspector entity must remain selected to create a prefab",
                    ));
                };
                game.selected_units.swap(0, position);
                let result = game.save_selected_as_prefab();
                game.selected_units = original_selection;
                let path = result?.ok_or_else(|| {
                    EditorCoreError::new(
                        EditorCoreErrorKind::CommandFailed,
                        "Prefab was not created",
                    )
                })?;
                self.refresh_all_caches();
                Ok(InspectorQuickActionOutcomeDto {
                    changed: true,
                    message: format!("Prefab created: {}", path.display()),
                    open_asset_path: None,
                    open_asset_type: None,
                })
            }
            _ => Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Unsupported Inspector quick action: {action_id}"),
            )),
        }
    }

    pub fn edit_inspector_value_json(
        &mut self,
        entity_id: u64,
        target: &str,
        key: &str,
        value_json: &str,
    ) -> Result<String, EditorCoreError> {
        let value = serde_json::from_str::<Value>(value_json)?;
        let previous = {
            self.game_mut()?
                .edit_inspector_value(entity_id, target, key, value)
                .map_err(|message| {
                    EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message)
                })?
        };
        self.refresh_scene_cache();
        Ok(previous.to_string())
    }

    pub fn edit_selected_inspector_value_json(
        &mut self,
        target: &str,
        key: &str,
        value_json: &str,
    ) -> Result<usize, EditorCoreError> {
        let value = serde_json::from_str::<Value>(value_json)?;
        let game = self.game_mut()?;
        let selection = game.selected_units.clone();
        if selection.is_empty() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Select at least one entity before editing common properties",
            ));
        }
        for entity_id in &selection {
            let mut probe = game.get_entity_by_id(*entity_id).cloned().ok_or_else(|| {
                EditorCoreError::new(EditorCoreErrorKind::NotFound, "Selected entity is missing")
            })?;
            InspectorEditor::edit_value(&mut probe, target, key, value.clone()).map_err(
                |message| EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message),
            )?;
        }
        let before = game.capture_editor_snapshot();
        for entity_id in &selection {
            let entity = game.get_entity_by_id_mut(*entity_id).ok_or_else(|| {
                EditorCoreError::new(EditorCoreErrorKind::NotFound, "Selected entity is missing")
            })?;
            InspectorEditor::edit_value(entity, target, key, value.clone()).map_err(|message| {
                EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message)
            })?;
        }
        game.sync_world();
        game.mark_scene_dirty("Edit Common Inspector Property");
        for entity_id in &selection {
            game.scene_save_manager.note_entity_dirty(*entity_id);
        }
        game.push_editor_command(
            "Edit Common Inspector Property",
            EditorCommandKind::SceneOperation {
                name: "Edit Common Inspector Property".to_string(),
            },
            before,
        );
        let count = selection.len();
        self.refresh_scene_cache();
        Ok(count)
    }

    pub fn asset_count(&self) -> Result<usize, EditorCoreError> {
        self.game()?;
        Ok(self.asset_cache.len())
    }

    pub fn asset_at(&self, index: usize) -> Result<AssetRow, EditorCoreError> {
        self.game()?;
        self.asset_cache.get(index).cloned().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                format!("Asset index out of range: {index}"),
            )
        })
    }

    pub fn content_folders(&self) -> Result<Vec<ContentFolderDto>, EditorCoreError> {
        let project_root = self
            .project_path
            .as_deref()
            .ok_or_else(EditorCoreError::no_project)?;
        collect_content_folders(project_root)
    }

    pub fn content_entries(
        &self,
        relative_directory: &str,
    ) -> Result<Vec<ContentEntryDto>, EditorCoreError> {
        let project_root = self
            .project_path
            .as_deref()
            .ok_or_else(EditorCoreError::no_project)?;
        collect_content_entries(
            project_root,
            &self.game()?.asset_database,
            relative_directory,
        )
    }

    pub fn create_content_folder(
        &mut self,
        relative_directory: &str,
        name: &str,
    ) -> Result<String, EditorCoreError> {
        validate_content_entry_name(name, "Folder")?;
        let project_root = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let parent = resolve_content_directory(&project_root, relative_directory, false)?;
        let destination = parent.join(name.trim());
        if destination.exists() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "A file or folder with that name already exists",
            ));
        }
        fs::create_dir(&destination)?;
        let relative_path = normalized_project_relative(&project_root, &destination);
        self.refresh_content_after_mutation(format!("Content folder created: {relative_path}"))?;
        Ok(relative_path)
    }

    pub fn create_content_file(
        &mut self,
        kind: &str,
        relative_directory: &str,
        name: &str,
    ) -> Result<String, EditorCoreError> {
        validate_content_entry_name(name, "Asset")?;
        let project_root = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let template = content_file_template(kind, name);
        let normalized_kind = kind.trim().to_ascii_lowercase();
        let requested_directory = relative_directory.trim().replace('\\', "/");
        let mut target_directory = if requested_directory.is_empty() || requested_directory == "." {
            template.directory.to_string()
        } else {
            requested_directory
        };
        if template.luau
            && target_directory != "scripts"
            && !target_directory.starts_with("scripts/")
        {
            target_directory = "scripts".to_string();
        }
        if matches!(normalized_kind.as_str(), "visual_graph" | "blueprint")
            && target_directory != "scripts/visual_graphs"
            && !target_directory.starts_with("scripts/visual_graphs/")
        {
            target_directory = "scripts/visual_graphs".to_string();
        }
        let directory = resolve_content_directory(&project_root, &target_directory, true)?;
        let stem = safe_content_stem(name);
        let path = unique_content_path(&directory, &stem, template.suffix);
        let relative_path = normalized_project_relative(&project_root, &path);
        if template.luau {
            self.save_luau_script(&relative_path, &template.source)?;
        } else if template.visual_graph {
            self.save_visual_graph(&relative_path, &template.source)?;
        } else {
            ProjectStorage::write_atomic(&path, template.source.as_bytes()).map_err(|error| {
                EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string())
            })?;
            self.refresh_content_after_mutation(format!("Content asset created: {relative_path}"))?;
        }
        Ok(relative_path)
    }

    pub fn read_text_asset(&self, relative_path: &str) -> Result<String, EditorCoreError> {
        let project_root = self
            .project_path
            .as_deref()
            .ok_or_else(EditorCoreError::no_project)?;
        let path = resolve_content_file(project_root, relative_path)?;
        if !is_editable_content_path(&path) {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Asset is not an editable project text file",
            ));
        }
        if is_luau_path(&path) {
            return self.read_luau_script(relative_path);
        }
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_CONTENT_TEXT_BYTES as u64 {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Text asset exceeds the 8 MiB editor limit",
            ));
        }
        let bytes = fs::read(&path)?;
        if bytes.len() > MAX_CONTENT_TEXT_BYTES {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Text asset exceeds the 8 MiB editor limit",
            ));
        }
        String::from_utf8(bytes).map_err(|error| {
            EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Text asset is not valid UTF-8: {error}"),
            )
        })
    }

    pub fn save_text_asset(
        &mut self,
        relative_path: &str,
        source: &str,
    ) -> Result<(), EditorCoreError> {
        if source.len() > MAX_CONTENT_TEXT_BYTES {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Text asset exceeds the 8 MiB editor limit",
            ));
        }
        let project_root = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let path = resolve_content_file(&project_root, relative_path)?;
        if !is_editable_content_path(&path) {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Asset is not an editable project text file",
            ));
        }
        if is_luau_path(&path) {
            return self.save_luau_script(relative_path, source);
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mfgraph"))
        {
            return self.save_visual_graph(relative_path, source);
        }
        if content_path_requires_json(&path) {
            serde_json::from_str::<Value>(source).map_err(|error| {
                EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("JSON validation failed: {error}"),
                )
            })?;
        }
        ProjectStorage::write_atomic_with_backup(
            &path,
            source.as_bytes(),
            BackupPolicy::new(
                path.with_file_name(format!(
                    "{}.backup",
                    path.file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("asset")
                )),
                DEFAULT_BACKUP_GENERATIONS,
            ),
        )
        .map_err(|error| EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string()))?;
        self.refresh_content_after_mutation(format!("Text asset saved: {relative_path}"))
    }

    fn refresh_content_after_mutation(&mut self, message: String) -> Result<(), EditorCoreError> {
        {
            let game = self.game_mut()?;
            game.asset_database.scan()?;
            game.console.log(message, "ASSETS");
        }
        self.refresh_asset_cache();
        Ok(())
    }

    /// Applies one safe Content Browser mutation and immediately reconciles
    /// the persistent asset database. `payload_json` is operation-specific:
    /// rename `{source,new_name}`, duplicate `{source,target_folder?}`, move
    /// `{source,target_folder}`, delete `{source,confirm,force?}`, and import
    /// `{source_external,target_folder}`.
    pub fn manage_asset(
        &mut self,
        action: &str,
        payload_json: &str,
    ) -> Result<AssetManageOutcomeDto, EditorCoreError> {
        let payload = serde_json::from_str::<Value>(payload_json)?;
        let project_root = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let action = action.trim().to_ascii_lowercase();
        let mut outcome = match action.as_str() {
            "rename" => manage_asset_rename(&project_root, &payload)?,
            "duplicate" => manage_asset_duplicate(&project_root, &payload)?,
            "move" => manage_asset_move(&project_root, &payload)?,
            "delete" | "trash" => {
                let database = &self.game()?.asset_database;
                manage_asset_delete(&project_root, database, &payload)?
            }
            "import" => manage_asset_import(&project_root, &payload)?,
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Unknown asset management action: {action}"),
                ));
            }
        };

        if let Err(error) = self.game_mut()?.asset_database.scan() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::Io,
                format!("Asset operation completed, but database refresh failed: {error}"),
            ));
        }
        self.refresh_asset_cache();
        outcome.refreshed_asset_count = self.asset_cache.len();
        self.game_mut()?
            .console
            .log(format!("{}: {}", outcome.action, outcome.message), "ASSETS");
        Ok(outcome)
    }

    pub fn profiler_snapshot(&self) -> Result<ProfilerSnapshotDto, EditorCoreError> {
        let game = self.game()?;
        let profiler_frame_ms = game.profiler.frame_time.as_secs_f64() * 1000.0;
        let frame_time_ms = if profiler_frame_ms > 0.0 {
            profiler_frame_ms
        } else {
            game.diagnostics.frame_time_ms.max(0.0)
        };
        let frame_budget_ms = game.clock.frame_budget_ms().max(0.1);
        let systems_total_ms = game.profiler.systems_time_total_ms();
        let denominator = frame_time_ms.max(systems_total_ms).max(f64::EPSILON);
        let mut systems = game
            .profiler
            .systems
            .iter()
            .map(|(name, milliseconds)| ProfilerSystemDto {
                name: name.clone(),
                milliseconds: *milliseconds,
                frame_percent: *milliseconds / denominator * 100.0,
                over_frame_budget: *milliseconds > frame_budget_ms,
            })
            .collect::<Vec<_>>();
        systems.sort_by(|left, right| {
            right
                .milliseconds
                .partial_cmp(&left.milliseconds)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.name.cmp(&right.name))
        });
        let observed_frame_ms = frame_time_ms.max(systems_total_ms);
        Ok(ProfilerSnapshotDto {
            frame_time_ms: observed_frame_ms,
            frame_budget_ms,
            fps: if observed_frame_ms > 0.0 {
                1000.0 / observed_frame_ms
            } else {
                game.diagnostics.fps.max(0.0)
            },
            systems_total_ms,
            unaccounted_ms: (observed_frame_ms - systems_total_ms).max(0.0),
            budget_usage_percent: observed_frame_ms / frame_budget_ms * 100.0,
            over_budget: observed_frame_ms > frame_budget_ms,
            slowest_system: systems.first().map(|system| system.name.clone()),
            systems,
            metrics: game.profiler.metrics.clone(),
            counters: game.profiler.counters.clone(),
        })
    }

    pub fn rebuild_asset_dependencies(&mut self) -> Result<(), EditorCoreError> {
        self.game_mut()?.asset_database.rebuild_dependency_graph()?;
        self.refresh_asset_cache();
        Ok(())
    }

    pub fn asset_dependency_graph(&self) -> Result<AssetDependencyGraphDto, EditorCoreError> {
        let database = &self.game()?.asset_database;
        let report = database.dependency_report();
        let mut reverse_counts = BTreeMap::<String, usize>::new();
        let mut edges = Vec::new();
        let mut unresolved_dependencies = Vec::new();
        for (consumer, record) in &database.assets {
            for dependency in &record.dependencies {
                let resolved = database.assets.contains_key(dependency);
                if resolved {
                    *reverse_counts.entry(dependency.clone()).or_default() += 1;
                } else {
                    unresolved_dependencies.push(format!("{consumer} -> {dependency}"));
                }
                edges.push(AssetDependencyEdgeDto {
                    dependency: dependency.clone(),
                    consumer: consumer.clone(),
                    resolved,
                });
            }
        }
        edges.sort_by(|left, right| {
            left.dependency
                .cmp(&right.dependency)
                .then_with(|| left.consumer.cmp(&right.consumer))
        });
        unresolved_dependencies.sort();
        unresolved_dependencies.dedup();
        let nodes = database
            .assets
            .iter()
            .map(|(path, record)| AssetDependencyNodeDto {
                path: path.clone(),
                guid: record.guid.clone(),
                asset_type: record.asset_type.clone(),
                size_bytes: record.size_bytes,
                dependency_count: record.dependencies.len(),
                reverse_dependency_count: reverse_counts.get(path).copied().unwrap_or_default(),
            })
            .collect();
        Ok(AssetDependencyGraphDto {
            nodes,
            edges,
            build_order: report.build_order,
            cycles: report.cycles,
            unresolved_dependencies,
            edge_count: report.edge_count,
        })
    }

    pub fn command_count(&self) -> usize {
        self.command_cache.len()
    }

    pub fn command_at(&self, index: usize) -> Result<CommandDescriptor, EditorCoreError> {
        self.command_cache.get(index).cloned().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                format!("Command index out of range: {index}"),
            )
        })
    }

    pub fn readiness_score(&self) -> Result<u8, EditorCoreError> {
        self.game()?;
        Ok(self.readiness_score)
    }

    pub fn readiness_summary(&self) -> Result<String, EditorCoreError> {
        self.game()?;
        Ok(self.readiness_summary.clone())
    }

    pub fn readiness_count(&self) -> Result<usize, EditorCoreError> {
        self.game()?;
        Ok(self.readiness_cache.len())
    }

    pub fn readiness_at(&self, index: usize) -> Result<ReadinessRow, EditorCoreError> {
        self.game()?;
        self.readiness_cache.get(index).cloned().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::NotFound,
                format!("Readiness row index out of range: {index}"),
            )
        })
    }

    pub fn runtime_health(&self) -> Result<RuntimeHealthDto, EditorCoreError> {
        let game = self.game()?;
        let guard = &game.stability_guard;
        let safe_mode = game.safe_mode.report();
        let frame = &guard.last_frame;
        let entity_count = game.runtime_world.units.len();
        let entity_limit_exceeded_by = entity_count.saturating_sub(guard.config.max_entities);
        let quarantined_entities = guard.quarantined_entity_count();
        let healthy = guard.level() == crate::engine::runtime_stability::StabilityLevel::Stable
            && !frame.delta_was_invalid
            && !frame.delta_was_clamped
            && frame.repaired_values == 0
            && quarantined_entities == 0
            && entity_limit_exceeded_by == 0;
        let mut warnings = game.diagnostics.warnings.clone();
        warnings.extend(game.runtime_config.tuning().warnings());
        warnings.extend(game.safe_mode.report().warnings);
        warnings.sort();
        warnings.dedup();
        warnings.truncate(32);
        let summary = if game.diagnostics.frames == 0 {
            format!(
                "{} · runtime ready · {} entities",
                guard.level().label(),
                entity_count
            )
        } else {
            format!(
                "{} · {}",
                frame.summary(),
                game.diagnostics.health_summary()
            )
        };
        Ok(RuntimeHealthDto {
            level: guard.level().label().to_string(),
            healthy,
            summary,
            mode: game.mode.clone(),
            guard_enabled: guard.config.enabled,
            raw_delta_ms: frame.raw_delta_seconds * 1000.0,
            safe_delta_ms: frame.safe_delta_seconds * 1000.0,
            delta_was_invalid: frame.delta_was_invalid,
            delta_was_clamped: frame.delta_was_clamped,
            repaired_values: frame.repaired_values,
            quarantined_entities,
            entity_count,
            max_entities: guard.config.max_entities,
            entity_limit_exceeded_by,
            optional_cadence_divisor: guard.optional_cadence_divisor(),
            stability_score: game.diagnostics.stability_score(),
            fps: game.diagnostics.fps,
            average_frame_time_ms: game.diagnostics.average_frame_time_ms,
            frame_budget_ms: game.clock.frame_budget_ms(),
            safe_mode_active: safe_mode.active,
            safe_mode_reason: game.safe_mode.reason.clone(),
            safe_mode_disabled_systems: safe_mode.disabled_systems,
            warnings,
        })
    }

    pub fn search_commands(&mut self, query: &str) -> Vec<CommandDescriptor> {
        self.command_palette.set_query(query);
        let by_label = self
            .command_cache
            .iter()
            .cloned()
            .map(|command| (command.label.clone(), command))
            .collect::<BTreeMap<_, _>>();
        self.command_palette
            .search()
            .into_iter()
            .filter_map(|label| by_label.get(&label).cloned())
            .collect()
    }

    pub fn execute_command(&mut self, command_id: &str) -> Result<CommandOutcome, EditorCoreError> {
        let descriptor = self
            .command_cache
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
            .ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Unknown command: {command_id}"),
                )
            })?;
        if !descriptor.enabled {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::CommandFailed,
                format!("Command is not available in the current editor state: {command_id}"),
            ));
        }
        let label = descriptor.label;
        if command_id == "project.audit" {
            self.refresh_readiness_cache()?;
            let message = self.readiness_summary.clone();
            self.game_mut()?.console.log(message.clone(), "AUDIT");
            self.command_palette.record_execution(&label);
            return Ok(CommandOutcome {
                changed: true,
                message,
            });
        }
        if command_id == "forge_ai.project_doctor" {
            let diagnostics = self.forge_ai_diagnostics()?;
            let message = format!("Forge AI Doctor: {} diagnostics", diagnostics.len());
            let game = self.game_mut()?;
            game.console.log(message.clone(), "FORGE_AI");
            for diagnostic in diagnostics.iter().take(8) {
                game.console.log(
                    format!(
                        "[{:?}] {}: {}",
                        diagnostic.severity, diagnostic.code, diagnostic.message
                    ),
                    "FORGE_AI",
                );
            }
            self.command_palette.record_execution(&label);
            return Ok(CommandOutcome {
                changed: false,
                message,
            });
        }
        if command_id == "forge_ai.enemy_smoke" {
            let report = self.forge_ai_run_test("forge_ai_enemy_smoke")?;
            let message = format!(
                "Forge AI enemy smoke: {:?}, {} cases, {} failures",
                report.status,
                report.cases_run,
                report.failures.len()
            );
            let game = self.game_mut()?;
            game.console.log(message.clone(), "FORGE_AI");
            for failure in &report.failures {
                game.console.log(failure.clone(), "FORGE_AI");
            }
            self.command_palette.record_execution(&label);
            return Ok(CommandOutcome {
                changed: false,
                message,
            });
        }

        let project_path = self.project_path.clone();
        let game = self.game_mut()?;
        let outcome = match command_id {
            "project.save" => {
                game.save_project()?;
                CommandOutcome {
                    changed: true,
                    message: "Project saved".to_string(),
                }
            }
            "scene.save" => {
                game.save_scene()?;
                CommandOutcome {
                    changed: true,
                    message: "Scene saved".to_string(),
                }
            }
            "scene.audit_tree" => {
                let tree = game.runtime_world.scene_tree();
                let signal_bus = game.runtime_world.signal_bus();
                let signal_report = signal_bus.validate();
                for warning in tree.warnings.iter().take(8) {
                    game.console.log(warning.clone(), "SCENE_TREE");
                }
                for missing in signal_report.missing_targets.iter().take(8) {
                    game.console
                        .log(format!("Signal target missing: {missing}"), "SCENE_TREE");
                }
                let message = format!(
                    "SceneTree audit: {} nodes, {} roots, {} groups, {} signal connections",
                    tree.nodes.len(),
                    tree.roots.len(),
                    tree.groups.len(),
                    signal_bus.connections.len()
                );
                game.console.log(message.clone(), "SCENE_TREE");
                CommandOutcome {
                    changed: false,
                    message,
                }
            }
            "scene.pack_selected" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let root_id = game.selected_units.first().copied().ok_or_else(|| {
                    EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        "Select an entity before packing a scene branch",
                    )
                })?;
                let packed =
                    game.runtime_world
                        .pack_scene_from_root(root_id)
                        .map_err(|message| {
                            EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message)
                        })?;
                let path = write_packed_scene_asset(&project_path, &packed.root_name, &packed)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!(
                        "Packed scene branch {}",
                        project_relative(&project_path, &path)
                    ),
                }
            }
            "edit.undo" => {
                let label = game
                    .undo_editor_command()
                    .unwrap_or_else(|| "Nothing to undo".into());
                CommandOutcome {
                    changed: label != "Nothing to undo",
                    message: label,
                }
            }
            "edit.redo" => {
                let label = game
                    .redo_editor_command()
                    .unwrap_or_else(|| "Nothing to redo".into());
                CommandOutcome {
                    changed: label != "Nothing to redo",
                    message: label,
                }
            }
            "console.clear" => {
                game.console.clear();
                CommandOutcome {
                    changed: true,
                    message: "Console cleared".to_string(),
                }
            }
            "console.clear_errors" => {
                game.console.clear_errors();
                CommandOutcome {
                    changed: true,
                    message: "Console errors cleared".to_string(),
                }
            }
            "selection.align_left" => align_selection(game, AlignMode2D::Left),
            "selection.align_center_x" => align_selection(game, AlignMode2D::CenterX),
            "selection.align_right" => align_selection(game, AlignMode2D::Right),
            "selection.align_top" => align_selection(game, AlignMode2D::Top),
            "selection.align_center_y" => align_selection(game, AlignMode2D::CenterY),
            "selection.align_bottom" => align_selection(game, AlignMode2D::Bottom),
            "selection.distribute_x" => align_selection(game, AlignMode2D::DistributeX),
            "selection.distribute_y" => align_selection(game, AlignMode2D::DistributeY),
            "selection.group" => group_selection(game),
            "selection.ungroup" => ungroup_selection(game),
            "selection.toggle_layer_lock" => toggle_selection_layer(game, true),
            "selection.toggle_layer_visibility" => toggle_selection_layer(game, false),
            "selection.cycle_layer" => cycle_selection_layer(game),
            "entity.create_empty" => {
                let id = game.spawn_game_object("GameObject", 0.0, 0.0);
                CommandOutcome {
                    changed: true,
                    message: format!("Created entity #{id}"),
                }
            }
            "object.create_node2d" => {
                let id = game.spawn_scene_node("Node2D", &[], 0.0, 0.0);
                CommandOutcome {
                    changed: true,
                    message: format!("Created Node2D #{id}"),
                }
            }
            "object.create_sprite_actor" => {
                let id = game.spawn_scene_node(
                    "SpriteActor",
                    &["SpriteRenderer", "Animator2D"],
                    0.0,
                    0.0,
                );
                let _ = game.add_component_to_entity(id, "SpriteRenderer");
                let _ = game.add_component_to_entity(id, "Animator2D");
                CommandOutcome {
                    changed: true,
                    message: format!("Created SpriteActor #{id}"),
                }
            }
            "object.create_camera" => {
                let id = game.spawn_scene_node("CameraRig", &["Camera2D"], 0.0, 0.0);
                let _ = game.add_component_to_entity(id, "Camera2D");
                CommandOutcome {
                    changed: true,
                    message: format!("Created CameraRig #{id}"),
                }
            }
            "object.create_point_light2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node("PointLight2D", &["Light2D"], x, y);
                CommandOutcome {
                    changed: true,
                    message: format!("Created shadow-casting PointLight2D #{id}"),
                }
            }
            "object.create_spot_light2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node("SpotLight2D", &["Light2D"], x, y);
                if let Some(entity) = game.get_entity_by_id_mut(id)
                    && let Some(light) = entity.get_component_mut("Light2D")
                {
                    light.set("light_type", json!("spot"));
                    light.set_f64("angle", 60.0);
                    light.set_f64("direction", 0.0);
                    light.set_f64("radius", 7.0);
                }
                game.sync_world();
                CommandOutcome {
                    changed: true,
                    message: format!("Created raycast SpotLight2D #{id}"),
                }
            }
            "object.create_shadow_occluder2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node(
                    "ShadowOccluder2D",
                    &["StaticBody2D", "Collider2D", "ShadowCaster2D"],
                    x,
                    y,
                );
                CommandOutcome {
                    changed: true,
                    message: format!("Created physics and lighting occluder #{id}"),
                }
            }
            "object.create_area2d" => {
                let id = game.spawn_scene_node("Area2D", &["Area2D"], 0.0, 0.0);
                CommandOutcome {
                    changed: true,
                    message: format!("Created Area2D #{id}"),
                }
            }
            "object.create_rigidbody2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node("Rigidbody2D", &["Rigidbody2D", "Collider2D"], x, y);
                CommandOutcome {
                    changed: true,
                    message: format!("Created dynamic Rigidbody2D #{id}"),
                }
            }
            "object.create_static_body2d" => {
                let (x, y) = editor_spawn_position(game);
                let id =
                    game.spawn_scene_node("StaticBody2D", &["StaticBody2D", "Collider2D"], x, y);
                CommandOutcome {
                    changed: true,
                    message: format!("Created StaticBody2D #{id}"),
                }
            }
            "object.create_trigger_volume2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node("TriggerVolume2D", &["Area2D", "Trigger2D"], x, y);
                CommandOutcome {
                    changed: true,
                    message: format!("Created monitored TriggerVolume2D #{id}"),
                }
            }
            "object.create_one_way_platform2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node(
                    "OneWayPlatform2D",
                    &["StaticBody2D", "Collider2D", "OneWayPlatform2D"],
                    x,
                    y,
                );
                if let Some(entity) = game.get_entity_by_id_mut(id) {
                    if let Some(body) = entity.get_component_mut("StaticBody2D") {
                        body.set("one_way", json!(true));
                    }
                    if let Some(collider) = entity.get_component_mut("Collider2D") {
                        collider.set("one_way", json!(true));
                    }
                }
                game.sync_world();
                CommandOutcome {
                    changed: true,
                    message: format!("Created OneWayPlatform2D #{id}"),
                }
            }
            "object.create_character_body2d" => {
                let id = game.spawn_scene_node(
                    "CharacterBody2D",
                    &["CharacterBody2D", "Rigidbody2D", "CharacterController2D"],
                    0.0,
                    0.0,
                );
                CommandOutcome {
                    changed: true,
                    message: format!("Created CharacterBody2D #{id}"),
                }
            }
            "object.create_nav_agent2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node(
                    "NavigationAgent2D",
                    &["NavAgent", "Collider2D", "Selectable"],
                    x,
                    y,
                );
                CommandOutcome {
                    changed: true,
                    message: format!("Created obstacle-aware NavigationAgent2D #{id}"),
                }
            }
            "object.create_particle_emitter2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node("ParticleEmitter2D", &["ParticleEmitter"], x, y);
                CommandOutcome {
                    changed: true,
                    message: format!("Created ParticleEmitter2D #{id}"),
                }
            }
            "object.create_audio_emitter2d" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_scene_node("AudioEmitter2D", &["AudioSource"], x, y);
                if let Some(entity) = game.get_entity_by_id_mut(id)
                    && let Some(audio) = entity.get_component_mut("AudioSource")
                {
                    audio.set_f64("spatial_blend", 1.0);
                }
                game.sync_world();
                CommandOutcome {
                    changed: true,
                    message: format!("Created spatial AudioEmitter2D #{id}"),
                }
            }
            "object.create_ui_text" => {
                let id = game.spawn_game_object("HUD_Text", 0.0, 0.0);
                let _ = game.add_component_to_entity(id, "UIText");
                CommandOutcome {
                    changed: true,
                    message: format!("Created HUD text #{id}"),
                }
            }
            "gameplay.spawn_unit" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_unit("PlayerUnit", x, y);
                CommandOutcome {
                    changed: true,
                    message: format!("Created gameplay unit #{id}"),
                }
            }
            "gameplay.spawn_enemy" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_enemy(x + 2.0, y);
                CommandOutcome {
                    changed: true,
                    message: format!("Created enemy AI #{id}"),
                }
            }
            "gameplay.spawn_resource" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_resource(x, y + 2.0);
                CommandOutcome {
                    changed: true,
                    message: format!("Created resource node #{id}"),
                }
            }
            "rts.spawn_base" => {
                let (x, y) = editor_spawn_position(game);
                let id = game.spawn_rts_building("CommandCenter", x, y, 1);
                CommandOutcome {
                    changed: true,
                    message: format!("Created RTS command center #{id}"),
                }
            }
            "rts.queue_worker" => queue_worker_on_selection(game),
            "rts.place_barracks" => place_barracks_for_selection(game),
            "scene.starter_topdown" => create_scene_starter(game, "topdown"),
            "scene.starter_platformer" => create_scene_starter(game, "platformer"),
            "scene.starter_rts" => create_scene_starter(game, "rts"),
            "project.template_rts" => create_project_template_files(game, "RTS")?,
            "project.template_actionrpg" => create_project_template_files(game, "ActionRPG")?,
            "project.template_survival" => create_project_template_files(game, "Survival")?,
            "play.enter" => {
                game.enter_play_mode();
                CommandOutcome {
                    changed: true,
                    message: "Entered Play Mode".to_string(),
                }
            }
            "play.stop" => {
                game.exit_play_mode("Qt editor command");
                CommandOutcome {
                    changed: true,
                    message: "Stopped Play Mode".to_string(),
                }
            }
            "assets.refresh" => {
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: "Assets refreshed".to_string(),
                }
            }
            "render.write_2d_profile" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let path = write_render_2d_profile_report(&project_path)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!(
                        "Wrote render profile {}",
                        project_relative(&project_path, &path)
                    ),
                }
            }
            "sprite.new_pixel_art" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let path = create_pixel_art_sprite(&project_path, "PixelSprite", false)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!("Created sprite {}", project_relative(&project_path, &path)),
                }
            }
            "sprite.create_hero_template" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let path = create_pixel_art_sprite(&project_path, "HeroTemplate", true)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!(
                        "Created hero sprite {}",
                        project_relative(&project_path, &path)
                    ),
                }
            }
            "sprite.export_frames" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let path = create_spriteframes_asset(&project_path)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!(
                        "Created spriteframes {}",
                        project_relative(&project_path, &path)
                    ),
                }
            }
            "sprite.export_atlas_pages" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let report = export_project_sprite_atlas_pages(&project_path)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!(
                        "Exported {} atlas pages with {} sprites",
                        report.manifest.pages.len(),
                        report.manifest.regions.len()
                    ),
                }
            }
            "sprite.optimize_palette" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let path = create_palette_ramp_asset(&project_path)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!(
                        "Created palette ramp {}",
                        project_relative(&project_path, &path)
                    ),
                }
            }
            "luau.new_controller" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let path = create_luau_controller(&project_path)?;
                game.asset_database.scan()?;
                CommandOutcome {
                    changed: true,
                    message: format!(
                        "Created Luau controller {}",
                        project_relative(&project_path, &path)
                    ),
                }
            }
            "luau.validate_scripts" => {
                let project_path = project_path.ok_or_else(EditorCoreError::no_project)?;
                let (checked, errors) = validate_luau_scripts(&project_path)?;
                for error in &errors {
                    game.console.log(error.clone(), "LUAU");
                }
                CommandOutcome {
                    changed: false,
                    message: if errors.is_empty() {
                        format!("Validated {checked} Luau scripts")
                    } else {
                        format!(
                            "Validated {checked} Luau scripts with {} errors",
                            errors.len()
                        )
                    },
                }
            }
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Unknown command: {command_id}"),
                ));
            }
        };
        self.command_palette.record_execution(&label);
        if outcome.changed {
            self.refresh_all_caches();
        }
        Ok(outcome)
    }

    pub fn forge_ai_create_entity(
        &mut self,
        name: &str,
        x: f64,
        y: f64,
    ) -> Result<u64, EditorCoreError> {
        if name.trim().is_empty() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "ForgeAI entity name cannot be empty",
            ));
        }
        let id = self.game_mut()?.spawn_game_object(name, x, y);
        self.refresh_scene_cache();
        Ok(id)
    }

    pub fn forge_ai_add_component(
        &mut self,
        entity_id: u64,
        component_type: &str,
    ) -> Result<(), EditorCoreError> {
        if !self
            .game_mut()?
            .add_component_to_entity(entity_id, component_type)
        {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::CommandFailed,
                format!("Could not add component {component_type} to entity {entity_id}"),
            ));
        }
        self.refresh_scene_cache();
        Ok(())
    }

    pub fn forge_ai_set_component_property(
        &mut self,
        entity_id: u64,
        component_type: &str,
        key: &str,
        value: Value,
    ) -> Result<Value, EditorCoreError> {
        let previous = self
            .game_mut()?
            .edit_inspector_value(entity_id, component_type, key, value)
            .map_err(|message| EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message))?;
        self.refresh_scene_cache();
        Ok(previous)
    }

    pub fn forge_ai_write_project_file(
        &mut self,
        relative_path: &str,
        contents: &str,
    ) -> Result<AiFileChange, EditorCoreError> {
        validate_forge_ai_relative_path(relative_path)?;
        let project_path = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let path = project_path.join(relative_path);
        let created = !path.exists();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        self.refresh_asset_cache();
        Ok(AiFileChange {
            relative_path: relative_path.to_string(),
            created,
            bytes_written: contents.len(),
        })
    }

    pub fn forge_ai_create_prefab(
        &mut self,
        entity_id: u64,
        _prefab_name: &str,
    ) -> Result<String, EditorCoreError> {
        self.select_entity(entity_id)?;
        let path = self.game_mut()?.save_selected_as_prefab()?.ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::CommandFailed,
                format!("Could not create prefab from entity {entity_id}"),
            )
        })?;
        self.refresh_all_caches();
        Ok(path.to_string_lossy().to_string())
    }

    pub fn forge_ai_validate_project(&mut self) -> Result<AiHostValidation, EditorCoreError> {
        let project_path = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let game = self.game()?;
        let mut validator = ProjectValidator::default();
        validator.validate_with_context(
            &project_path,
            &game.runtime_world.units,
            Some(&game.asset_database),
        );
        Ok(AiHostValidation {
            errors: validator.errors,
            warnings: validator.warnings,
        })
    }

    pub fn forge_ai_run_test(&mut self, suite_id: &str) -> Result<AiTestReport, EditorCoreError> {
        self.refresh_scene_cache();
        let context = AiProjectContext::from_editor_core(self)?;
        let report = match suite_id {
            "forge_ai_enemy_smoke" => AiTestSuite::enemy_smoke().run_static(&context),
            other => AiTestReport {
                suite_id: other.to_string(),
                status: AiTestStatus::Skipped,
                cases_run: 0,
                failures: vec![format!("Unknown ForgeAI test suite: {other}")],
                replay_path: None,
            },
        };
        Ok(report)
    }

    pub fn forge_ai_diagnostics(&mut self) -> Result<Vec<AiDiagnostic>, EditorCoreError> {
        self.refresh_scene_cache();
        self.refresh_asset_cache();
        let context = AiProjectContext::from_editor_core(self)?;
        let validation = self.forge_ai_validate_project()?;
        let mut diagnostics = ProjectDoctor::analyze(&context);
        diagnostics.extend(ProjectDoctor::from_project_validation(
            &validation.errors,
            &validation.warnings,
        ));
        diagnostics.sort_by(|a, b| {
            a.severity
                .cmp(&b.severity)
                .then_with(|| a.code.cmp(&b.code))
        });
        Ok(diagnostics)
    }

    pub fn console_count(&self) -> Result<usize, EditorCoreError> {
        Ok(self.game()?.console.structured_entries.len())
    }

    pub fn console_entry_at(&self, index: usize) -> Result<ConsoleEntry, EditorCoreError> {
        self.game()?
            .console
            .structured_entries
            .get(index)
            .cloned()
            .ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::NotFound,
                    format!("Console index out of range: {index}"),
                )
            })
    }

    pub fn sprite_snapshot(&self) -> Result<ViewportSnapshot, EditorCoreError> {
        let canvas = &self.game()?.sprite_editor;
        let rgba = canvas
            .pixels
            .iter()
            .flat_map(|color| color.rgba())
            .collect::<Vec<_>>();
        Ok(ViewportSnapshot {
            width: canvas.width,
            height: canvas.height,
            rgba,
        })
    }

    pub fn sprite_can_undo(&self) -> Result<bool, EditorCoreError> {
        Ok(self.game()?.sprite_editor.can_undo())
    }

    pub fn sprite_can_redo(&self) -> Result<bool, EditorCoreError> {
        Ok(self.game()?.sprite_editor.can_redo())
    }

    pub fn sprite_new_canvas(&mut self, width: u32, height: u32) -> Result<(), EditorCoreError> {
        if width == 0 || height == 0 || width > 512 || height > 512 {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Sprite canvas dimensions must be between 1 and 512",
            ));
        }
        self.game_mut()?.new_sprite_canvas(width, height);
        Ok(())
    }

    pub fn sprite_begin_edit(&mut self) -> Result<(), EditorCoreError> {
        self.game_mut()?.sprite_editor.begin_edit();
        Ok(())
    }

    pub fn sprite_set_pixel(
        &mut self,
        x: u32,
        y: u32,
        color: SpriteColor,
    ) -> Result<bool, EditorCoreError> {
        let canvas = &mut self.game_mut()?.sprite_editor;
        if x >= canvas.width || y >= canvas.height {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Sprite pixel out of bounds: {x},{y}"),
            ));
        }
        Ok(canvas.set_pixel(x, y, color))
    }

    pub fn sprite_clear(&mut self, color: SpriteColor) -> Result<(), EditorCoreError> {
        self.game_mut()?.sprite_editor.clear(color);
        Ok(())
    }

    /// Applies one non-destructive sprite utility as a single undoable edit.
    /// This keeps native-editor actions (paint utilities and transforms) on the
    /// same history stack as regular pixel strokes.
    pub fn sprite_transform(
        &mut self,
        action: &str,
        payload_json: &str,
    ) -> Result<bool, EditorCoreError> {
        let payload = if payload_json.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str::<Value>(payload_json)?
        };
        if !matches!(
            action,
            "flip_horizontal"
                | "flip_vertical"
                | "rotate_right"
                | "crop_to_content"
                | "outline"
                | "bucket_fill"
                | "replace_color"
                | "drop_shadow"
        ) {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Unknown sprite transform: {action}"),
            ));
        }
        let effect_color = matches!(action, "outline" | "bucket_fill" | "drop_shadow")
            .then(|| sprite_color_from_payload(&payload))
            .transpose()?;
        let replace_colors = (action == "replace_color")
            .then(|| {
                let from = payload.get("from").ok_or_else(|| {
                    EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        "replace_color requires a from color",
                    )
                })?;
                let to = payload.get("to").ok_or_else(|| {
                    EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        "replace_color requires a to color",
                    )
                })?;
                Ok::<_, EditorCoreError>((
                    sprite_color_from_payload(from)?,
                    sprite_color_from_payload(to)?,
                ))
            })
            .transpose()?;
        let bucket_coordinates = (action == "bucket_fill")
            .then(|| {
                let x = payload.get("x").and_then(Value::as_u64).ok_or_else(|| {
                    EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        "bucket_fill requires an x coordinate",
                    )
                })?;
                let y = payload.get("y").and_then(Value::as_u64).ok_or_else(|| {
                    EditorCoreError::new(
                        EditorCoreErrorKind::InvalidArgument,
                        "bucket_fill requires a y coordinate",
                    )
                })?;
                Ok::<_, EditorCoreError>((x as u32, y as u32))
            })
            .transpose()?;
        let canvas = &mut self.game_mut()?.sprite_editor;
        if let Some((x, y)) = bucket_coordinates
            && (x >= canvas.width || y >= canvas.height)
        {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Sprite pixel out of bounds: {x},{y}"),
            ));
        }
        canvas.begin_edit();
        match action {
            "flip_horizontal" => canvas.flip_horizontal(),
            "flip_vertical" => canvas.flip_vertical(),
            "rotate_right" => canvas.rotate_right(),
            "crop_to_content" => {
                let padding = payload
                    .get("padding")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(64) as u32;
                let _ = canvas.crop_to_content(padding);
            }
            "outline" => {
                let thickness = payload
                    .get("thickness")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .clamp(1, 16) as u32;
                let color = effect_color.expect("outline payload was validated");
                let _ = canvas.outline_alpha_thick(thickness, color);
            }
            "bucket_fill" => {
                let (x, y) = bucket_coordinates.expect("bucket coordinates were validated");
                let color = effect_color.expect("bucket payload was validated");
                let _ = canvas.bucket_fill(x, y, color);
            }
            "replace_color" => {
                let (from, to) = replace_colors.expect("replace payload was validated");
                let _ = canvas.replace_color(from, to);
            }
            "drop_shadow" => {
                let offset_x = payload
                    .get("offset_x")
                    .and_then(Value::as_i64)
                    .unwrap_or(1)
                    .clamp(-64, 64) as i32;
                let offset_y = payload
                    .get("offset_y")
                    .and_then(Value::as_i64)
                    .unwrap_or(1)
                    .clamp(-64, 64) as i32;
                let color = effect_color.expect("shadow payload was validated");
                let _ = canvas.drop_shadow(offset_x, offset_y, color);
            }
            _ => unreachable!("sprite transform action was validated"),
        }
        Ok(canvas.commit_edit())
    }

    pub fn sprite_animation_clip(
        &self,
        frame_width: u32,
        frame_height: u32,
        fps: f32,
    ) -> Result<Value, EditorCoreError> {
        if frame_width == 0 || frame_height == 0 || !fps.is_finite() || fps <= 0.0 {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Sprite animation frame dimensions and fps must be greater than zero",
            ));
        }
        let canvas = &self.game()?.sprite_editor;
        let clip = canvas.animation_clip_draft("SpriteAnimation", frame_width, frame_height, fps);
        let timeline = clip.timeline_preview();
        Ok(json!({
            "clip": clip,
            "timeline": timeline,
        }))
    }

    pub fn sprite_commit_edit(&mut self) -> Result<bool, EditorCoreError> {
        Ok(self.game_mut()?.sprite_editor.commit_edit())
    }

    pub fn sprite_undo(&mut self) -> Result<bool, EditorCoreError> {
        Ok(self.game_mut()?.sprite_editor.undo())
    }

    pub fn sprite_redo(&mut self) -> Result<bool, EditorCoreError> {
        Ok(self.game_mut()?.sprite_editor.redo())
    }

    pub fn sprite_save_current(&mut self, fallback_name: &str) -> Result<String, EditorCoreError> {
        if fallback_name.trim().is_empty() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Sprite fallback name cannot be empty",
            ));
        }
        let path = self.game_mut()?.save_sprite_canvas_current(fallback_name)?;
        self.refresh_asset_cache();
        Ok(path.to_string_lossy().to_string())
    }

    pub fn viewport_snapshot(
        &self,
        width: u32,
        height: u32,
    ) -> Result<ViewportSnapshot, EditorCoreError> {
        if width == 0 || height == 0 {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Viewport dimensions must be greater than zero",
            ));
        }
        let game = self.game()?;
        Ok(render_snapshot(game, width, height))
    }

    fn game(&self) -> Result<&Game, EditorCoreError> {
        self.game.as_ref().ok_or_else(EditorCoreError::no_project)
    }

    fn project_relative_path(&self, path: &Path) -> String {
        self.project_path
            .as_deref()
            .map(|project_path| project_relative(project_path, path))
            .unwrap_or_else(|| path.to_string_lossy().to_string())
    }

    fn game_mut(&mut self) -> Result<&mut Game, EditorCoreError> {
        self.game.as_mut().ok_or_else(EditorCoreError::no_project)
    }

    fn refresh_all_caches(&mut self) {
        self.refresh_scene_cache();
        self.refresh_asset_cache();
        if self.project_path.is_some() {
            let _ = self.refresh_readiness_cache();
        }
    }

    /// Runs low-frequency editor safety work from the native workbench refresh
    /// pulse. Failures are retained in the owning subsystem and surfaced in
    /// the developer console instead of making hierarchy/inspector refreshes
    /// unusable.
    fn run_periodic_maintenance(&mut self) {
        let changes = self
            .file_watcher
            .as_ref()
            .map(EditorFileWatcher::drain)
            .unwrap_or_default();

        let Some(game) = self.game.as_mut() else {
            return;
        };

        if !changes.is_empty() {
            let changed_paths = changes
                .iter()
                .flat_map(|change| change.paths.iter())
                .collect::<Vec<_>>();
            let active_path = game.script_editor.document.path.clone();
            if !game.script_editor.document.dirty
                && active_path
                    .as_ref()
                    .is_some_and(|active| changed_paths.iter().any(|path| *path == active))
                && let Err(error) = game.reload_open_file()
            {
                game.console
                    .warning(format!("External script reload failed: {error}"), "WATCHER");
            }

            if changed_paths
                .iter()
                .any(|path| is_project_authoring_path(path))
            {
                match game.refresh_assets() {
                    Ok(count) => game.console.log(
                        format!("External project changes indexed: {count} assets"),
                        "WATCHER",
                    ),
                    Err(error) => game
                        .console
                        .warning(format!("External asset refresh failed: {error}"), "WATCHER"),
                }
            }
        }

        let autosave_enabled = game
            .engine_config
            .get("autosave")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if autosave_enabled && game.scene_dirty && game.autosave_manager.should_save() {
            if let Err(error) = game.autosave_manager.save(&mut game.runtime_world.units) {
                game.autosave_manager.last_save = Instant::now();
                game.console.warning(
                    format!("Automatic scene checkpoint failed: {error}"),
                    "AUTOSAVE",
                );
            } else {
                game.console
                    .log("Automatic scene checkpoint written", "AUTOSAVE");
            }
        }

        let Some(recovery) = self.session_recovery.as_mut() else {
            return;
        };
        if !recovery.should_checkpoint() {
            return;
        }
        if let Err(error) = recovery.checkpoint(
            &game.scene_manager.current_scene,
            game.scene_dirty,
            &game.scene_dirty_reason,
            &mut game.script_editor,
            SessionUiState::default(),
        ) {
            recovery.last_checkpoint = Instant::now();
            game.console.warning(
                format!("Automatic editor-session checkpoint failed: {error}"),
                "RECOVERY",
            );
        }
    }

    fn finish_active_session(&mut self) {
        if self
            .project_path
            .as_ref()
            .is_none_or(|project_path| !project_path.is_dir())
        {
            return;
        }
        let (Some(game), Some(recovery)) = (self.game.as_mut(), self.session_recovery.as_mut())
        else {
            return;
        };
        let has_unsaved_changes = game.scene_dirty || game.script_editor.has_dirty_documents();
        if has_unsaved_changes {
            if let Err(error) = recovery.checkpoint(
                &game.scene_manager.current_scene,
                game.scene_dirty,
                &game.scene_dirty_reason,
                &mut game.script_editor,
                SessionUiState::default(),
            ) {
                game.console.warning(
                    format!("Final editor-session checkpoint failed: {error}"),
                    "RECOVERY",
                );
            }
        } else if let Err(error) = recovery.clear() {
            game.console.warning(
                format!("Clean editor-session recovery cleanup failed: {error}"),
                "RECOVERY",
            );
        }
    }

    fn refresh_scene_cache(&mut self) {
        let (entity_cache, selected_cache, inspector_cache) = match self.game.as_ref() {
            Some(game) => {
                let child_counts = child_counts(&game.runtime_world.units);
                let entity_cache = game
                    .runtime_world
                    .units
                    .iter()
                    .map(|entity| entity_row(entity, &child_counts))
                    .collect();
                let inspector_cache = game
                    .runtime_world
                    .units
                    .iter()
                    .map(|entity| (entity.id, inspector_fields_for_entity(entity)))
                    .collect();
                (entity_cache, game.selected_units.clone(), inspector_cache)
            }
            None => (Vec::new(), Vec::new(), BTreeMap::new()),
        };
        self.entity_cache = entity_cache;
        self.selected_cache = selected_cache;
        self.inspector_cache = inspector_cache;
        self.refresh_command_availability();
    }

    fn refresh_command_availability(&mut self) {
        let Some(game) = self.game.as_ref() else {
            for command in &mut self.command_cache {
                command.enabled = false;
            }
            return;
        };
        let selection_count = game.selected_units.len();
        let play_mode = game.mode == "PLAY";
        let can_undo = !game.history.command_undo.is_empty();
        let can_redo = !game.history.command_redo.is_empty();
        for command in &mut self.command_cache {
            command.enabled = match command.id.as_str() {
                "edit.undo" => can_undo,
                "edit.redo" => can_redo,
                "play.enter" => !play_mode,
                "play.stop" => play_mode,
                "scene.pack_selected"
                | "selection.toggle_layer_lock"
                | "selection.toggle_layer_visibility"
                | "selection.cycle_layer"
                | "rts.queue_worker"
                | "rts.place_barracks" => selection_count >= 1,
                "selection.align_left"
                | "selection.align_center_x"
                | "selection.align_right"
                | "selection.align_top"
                | "selection.align_center_y"
                | "selection.align_bottom"
                | "selection.distribute_x"
                | "selection.distribute_y"
                | "selection.group" => selection_count >= 2,
                "selection.ungroup" => selection_count >= 1,
                _ => true,
            };
        }
    }

    fn refresh_asset_cache(&mut self) {
        self.asset_cache = self
            .game
            .as_ref()
            .map(|game| game.asset_database.assets.values().map(asset_row).collect())
            .unwrap_or_default();
    }

    fn refresh_readiness_cache(&mut self) -> Result<(), EditorCoreError> {
        let project_path = self
            .project_path
            .clone()
            .ok_or_else(EditorCoreError::no_project)?;
        let report = SystemReadinessReport::audit_project(&project_path)
            .map_err(|error| EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string()))?;
        self.readiness_score = report.total_score;
        self.readiness_summary = report.concise_summary();
        self.readiness_cache = report.areas.values().map(readiness_row).collect::<Vec<_>>();
        self.readiness_cache
            .sort_by(|a, b| a.score.cmp(&b.score).then_with(|| a.system.cmp(&b.system)));
        Ok(())
    }
}

impl Drop for EditorCore {
    fn drop(&mut self) {
        self.finish_active_session();
    }
}

fn validate_editor_open_options(options: &EditorOpenOptions) -> Result<(), EditorCoreError> {
    if options.safe_mode_reason.len() > MF_VALUE_CAPACITY_EQUIVALENT
        || options
            .safe_mode_reason
            .chars()
            .any(|character| character.is_control() && !character.is_whitespace())
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Safe Mode reason must be valid text up to 512 bytes",
        ));
    }
    if !options.safe_mode && options.disable_asset_importers {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "disable_asset_importers requires safe_mode=true",
        ));
    }
    Ok(())
}

const MF_VALUE_CAPACITY_EQUIVALENT: usize = 512;

fn is_project_authoring_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.contains("/assets/")
        || normalized.contains("/scripts/")
        || normalized.contains("/saves/scenes/")
        || normalized.ends_with("/engine_config.json")
        || normalized.contains("/settings/")
}

fn align_selection(game: &mut Game, mode: AlignMode2D) -> CommandOutcome {
    let before = game.capture_editor_snapshot();
    let changed = EditorSpatialTools2D::align(
        &mut game.runtime_world.units,
        &game.selected_units.clone(),
        mode,
    );
    if changed.is_empty() {
        return CommandOutcome {
            changed: false,
            message: "Select at least two unlocked entities to align".to_string(),
        };
    }
    game.sync_world();
    game.mark_scene_dirty("Align Selection");
    game.push_editor_command(
        format!("Align {mode:?}"),
        EditorCommandKind::SceneOperation {
            name: format!("Align {mode:?}"),
        },
        before,
    );
    CommandOutcome {
        changed: true,
        message: format!("Aligned {} entities", changed.len()),
    }
}

fn group_selection(game: &mut Game) -> CommandOutcome {
    if game.selected_units.len() < 2 {
        return CommandOutcome {
            changed: false,
            message: "Select at least two entities to group".to_string(),
        };
    }
    let before = game.capture_editor_snapshot();
    let group = format!("group_{}", game.selected_units[0]);
    for id in game.selected_units.clone() {
        if let Some(entity) = game.get_entity_by_id_mut(id) {
            entity.editor_group = Some(group.clone());
        }
    }
    game.mark_scene_dirty("Group Selection");
    game.push_editor_command(
        "Group Selection",
        EditorCommandKind::SceneOperation {
            name: group.clone(),
        },
        before,
    );
    CommandOutcome {
        changed: true,
        message: format!("Created {group}"),
    }
}

fn ungroup_selection(game: &mut Game) -> CommandOutcome {
    let before = game.capture_editor_snapshot();
    let mut changed = 0usize;
    for id in game.selected_units.clone() {
        if let Some(entity) = game.get_entity_by_id_mut(id)
            && entity.editor_group.take().is_some()
        {
            changed += 1;
        }
    }
    if changed == 0 {
        return CommandOutcome {
            changed: false,
            message: "Selection is not grouped".to_string(),
        };
    }
    game.mark_scene_dirty("Ungroup Selection");
    game.push_editor_command(
        "Ungroup Selection",
        EditorCommandKind::SceneOperation {
            name: "Ungroup Selection".to_string(),
        },
        before,
    );
    CommandOutcome {
        changed: true,
        message: format!("Ungrouped {changed} entities"),
    }
}

fn toggle_selection_layer(game: &mut Game, lock: bool) -> CommandOutcome {
    let Some(layer) = game
        .selected_units
        .first()
        .and_then(|id| game.get_entity_by_id(*id))
        .map(|entity| entity.layer.clone())
    else {
        return CommandOutcome {
            changed: false,
            message: "Select an entity to operate on its layer".to_string(),
        };
    };
    let before = game.capture_editor_snapshot();
    let next = if lock {
        game.runtime_world
            .units
            .iter()
            .filter(|entity| entity.layer == layer)
            .any(|entity| !entity.locked)
    } else {
        game.runtime_world
            .units
            .iter()
            .filter(|entity| entity.layer == layer)
            .any(|entity| !entity.visible)
    };
    for entity in game
        .runtime_world
        .units
        .iter_mut()
        .filter(|entity| entity.layer == layer)
    {
        if lock {
            entity.locked = next;
        } else {
            entity.visible = next;
        }
        entity.sync_to_components();
    }
    if (lock && next) || (!lock && !next) {
        game.clear_selection();
    }
    game.sync_world();
    game.mark_scene_dirty("Edit Layer State");
    game.push_editor_command(
        "Edit Layer State",
        EditorCommandKind::SceneOperation {
            name: format!("Layer {layer}"),
        },
        before,
    );
    CommandOutcome {
        changed: true,
        message: format!("Updated layer {layer}"),
    }
}

fn cycle_selection_layer(game: &mut Game) -> CommandOutcome {
    if game.selected_units.is_empty() || game.tags_layers_manager.layers.is_empty() {
        return CommandOutcome {
            changed: false,
            message: "Selection or layer catalog is empty".to_string(),
        };
    }
    let before = game.capture_editor_snapshot();
    let layers = game.tags_layers_manager.layers.clone();
    for id in game.selected_units.clone() {
        if let Some(entity) = game.get_entity_by_id_mut(id) {
            let index = layers
                .iter()
                .position(|layer| layer == &entity.layer)
                .unwrap_or(0);
            entity.layer = layers[(index + 1) % layers.len()].clone();
            entity.sync_to_components();
        }
    }
    game.sync_world();
    game.mark_scene_dirty("Cycle Selection Layer");
    game.push_editor_command(
        "Cycle Selection Layer",
        EditorCommandKind::SceneOperation {
            name: "Cycle Selection Layer".to_string(),
        },
        before,
    );
    CommandOutcome {
        changed: true,
        message: "Moved selection to the next layer".to_string(),
    }
}

fn editor_spawn_position(game: &Game) -> (f64, f64) {
    let tile_size = game.grid.tile_size.max(1) as f64;
    (
        game.camera.x / tile_size + 5.0,
        game.camera.y / tile_size + 4.0,
    )
}

fn queue_worker_on_selection(game: &mut Game) -> CommandOutcome {
    let Some(entity_id) = game.selected_units.first().copied() else {
        return CommandOutcome {
            changed: false,
            message: "Select a building with ProductionQueue first".to_string(),
        };
    };
    let before = game.capture_editor_snapshot();
    let queued = game.get_entity_by_id_mut(entity_id).is_some_and(|entity| {
        RTSSystem::enqueue_production(entity, "Worker", "Worker", 3.0, json!({"Gold": 50.0}))
    });
    if !queued {
        return CommandOutcome {
            changed: false,
            message: "The selected entity cannot queue a Worker; check its queue and resources"
                .to_string(),
        };
    }
    game.sync_world();
    game.mark_scene_dirty("Queue Worker");
    game.push_editor_command(
        "Queue Worker",
        EditorCommandKind::SceneOperation {
            name: "Queue Worker".to_string(),
        },
        before,
    );
    CommandOutcome {
        changed: true,
        message: format!("Queued Worker on building #{entity_id}"),
    }
}

fn place_barracks_for_selection(game: &mut Game) -> CommandOutcome {
    let builder_ids = game.selected_units.clone();
    let (x, y) = editor_spawn_position(game);
    let Some(entity_id) = game.try_place_rts_building(
        "Barracks",
        (x.round() as i32, y.round() as i32),
        1,
        builder_ids,
    ) else {
        return CommandOutcome {
            changed: false,
            message: "No valid nearby grid cell can fit a Barracks".to_string(),
        };
    };
    CommandOutcome {
        changed: true,
        message: format!("Placed Barracks construction site #{entity_id}"),
    }
}

fn create_scene_starter(game: &mut Game, starter: &str) -> CommandOutcome {
    let before = game.capture_editor_snapshot();
    let undo_len = game.history.command_undo.len();
    let (label, entity_count) = match starter {
        "topdown" => ("TopDown", game.create_topdown_starter()),
        "platformer" => ("Platformer", game.create_platformer_starter()),
        "rts" => {
            game.create_rts_skirmish();
            ("RTS Skirmish", game.runtime_world.units.len())
        }
        _ => unreachable!("scene starter id is internal"),
    };

    // Starter builders create through the regular authoring API. Collapse the
    // internal entity history into one predictable user-facing undo step.
    game.history.command_undo.truncate(undo_len);
    game.history.command_redo.clear();
    game.push_editor_command(
        format!("Create {label} Starter"),
        EditorCommandKind::SceneOperation {
            name: format!("{label} Starter"),
        },
        before,
    );
    CommandOutcome {
        changed: true,
        message: format!("Created {label} starter with {entity_count} entities"),
    }
}

fn create_project_template_files(
    game: &mut Game,
    template: &str,
) -> Result<CommandOutcome, EditorCoreError> {
    let created = game.create_project_template(template)?;
    Ok(CommandOutcome {
        changed: created > 0,
        message: format!("Created {created} files for the {template} project template"),
    })
}

fn sprite_color_from_payload(payload: &Value) -> Result<SpriteColor, EditorCoreError> {
    let color = payload.get("color").unwrap_or(payload);
    let channel = |name: &str, index: usize, default: u8| -> Result<u8, EditorCoreError> {
        let value = color
            .get(name)
            .and_then(Value::as_u64)
            .or_else(|| color.as_array()?.get(index)?.as_u64())
            .unwrap_or(default as u64);
        u8::try_from(value).map_err(|_| {
            EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Sprite color channel {name} must be between 0 and 255"),
            )
        })
    };
    Ok(SpriteColor {
        r: channel("r", 0, 0)?,
        g: channel("g", 1, 0)?,
        b: channel("b", 2, 0)?,
        a: channel("a", 3, 255)?,
    })
}

fn input_map_value(input: &InputMap) -> Value {
    let actions = input
        .actions
        .iter()
        .map(|(name, action)| {
            (
                name.clone(),
                serde_json::json!({
                    "display_name": action.display_name,
                    "category": action.category,
                    "devices": action.devices,
                    "description": action.description,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    serde_json::json!({"bindings": input.bindings, "actions": actions})
}

fn validate_engine_settings(value: &Value) -> Result<(), EditorCoreError> {
    let Some(settings) = value.as_object() else {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Project settings must be a JSON object",
        ));
    };
    if settings
        .get("start_scene")
        .is_some_and(|value| value.as_str().is_none_or(str::is_empty))
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "start_scene must be a non-empty string",
        ));
    }
    for key in ["rendering", "runtime", "editor", "logs"] {
        if settings.get(key).is_some_and(|value| !value.is_object()) {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("{key} must be a JSON object"),
            ));
        }
    }
    Ok(())
}

fn parse_input_map(
    value: &Value,
) -> Result<
    (
        BTreeMap<String, Vec<String>>,
        BTreeMap<String, InputActionInfo>,
    ),
    EditorCoreError,
> {
    let object = value.as_object().ok_or_else(|| {
        EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Input Map must be a JSON object",
        )
    })?;
    let binding_values = object
        .get("bindings")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                "Input Map requires a bindings object",
            )
        })?;
    if binding_values.len() > 256 {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Input Map cannot contain more than 256 actions",
        ));
    }

    let mut bindings = BTreeMap::new();
    for (name, values) in binding_values {
        validate_setting_name(name, "input action")?;
        let values = values.as_array().ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Bindings for {name} must be an array"),
            )
        })?;
        let mut unique = BTreeMap::new();
        for value in values {
            let binding = value.as_str().map(str::trim).ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Binding for {name} must be a string"),
                )
            })?;
            if binding.is_empty() || binding.len() > 128 {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Binding for {name} must contain between 1 and 128 characters"),
                ));
            }
            unique.insert(binding.to_string(), ());
        }
        bindings.insert(name.clone(), unique.into_keys().collect());
    }

    let mut actions = BTreeMap::new();
    if let Some(action_values) = object.get("actions").and_then(Value::as_object) {
        for (name, value) in action_values {
            validate_setting_name(name, "input action")?;
            let metadata = value.as_object().ok_or_else(|| {
                EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("Action metadata for {name} must be an object"),
                )
            })?;
            let devices = metadata
                .get("devices")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| vec!["keyboard".to_string()]);
            actions.insert(
                name.clone(),
                InputActionInfo {
                    name: name.clone(),
                    display_name: metadata
                        .get("display_name")
                        .and_then(Value::as_str)
                        .unwrap_or(name)
                        .to_string(),
                    category: metadata
                        .get("category")
                        .and_then(Value::as_str)
                        .unwrap_or("Gameplay")
                        .to_string(),
                    devices,
                    description: metadata
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                },
            );
        }
    }
    for name in bindings.keys() {
        actions
            .entry(name.clone())
            .or_insert_with(|| InputActionInfo {
                name: name.clone(),
                display_name: name.clone(),
                category: "Gameplay".to_string(),
                devices: vec!["keyboard".to_string()],
                description: String::new(),
            });
    }
    Ok((bindings, actions))
}

fn parse_named_items(
    value: &Value,
    key: &str,
    required: &str,
) -> Result<Vec<String>, EditorCoreError> {
    let values = value.get(key).and_then(Value::as_array).ok_or_else(|| {
        EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("{key} must be an array"),
        )
    })?;
    if values.len() > 256 {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("{key} cannot contain more than 256 items"),
        ));
    }
    let mut unique = BTreeMap::new();
    for value in values {
        let name = value.as_str().map(str::trim).ok_or_else(|| {
            EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Every {key} item must be a string"),
            )
        })?;
        validate_setting_name(name, key)?;
        unique.insert(name.to_string(), ());
    }
    unique.remove(required);
    let mut items = vec![required.to_string()];
    items.extend(unique.into_keys());
    Ok(items)
}

fn validate_setting_name(value: &str, kind: &str) -> Result<(), EditorCoreError> {
    if value.is_empty()
        || value.len() > 64
        || value.chars().any(char::is_control)
        || value.contains(['/', '\\'])
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Invalid {kind} name: {value}"),
        ));
    }
    Ok(())
}

fn parse_launcher_template(value: &str) -> Result<LauncherTemplate, EditorCoreError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "empty" => Ok(LauncherTemplate::Empty),
        "topdown" | "top_down" | "top-down" => Ok(LauncherTemplate::TopDown),
        "platformer" => Ok(LauncherTemplate::Platformer),
        "rts" => Ok(LauncherTemplate::Rts),
        _ => Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Unknown project template: {value}"),
        )),
    }
}

fn project_launcher_dto(launcher: &ProjectLauncherState) -> ProjectLauncherDto {
    ProjectLauncherDto {
        workspace_root: launcher.workspace_root.to_string_lossy().to_string(),
        project_location: launcher.project_location.clone(),
        recent_projects: launcher
            .recent_projects
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        templates: LauncherTemplate::all()
            .into_iter()
            .map(|template| template.key().to_string())
            .collect(),
        settings: serde_json::to_value(&launcher.settings).unwrap_or(Value::Null),
        status: launcher.status.clone(),
    }
}

fn validated_external_file(value: &str) -> Result<PathBuf, EditorCoreError> {
    let path = PathBuf::from(value);
    if value.contains('\0') || !path.is_absolute() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "External file path must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "External file must be a regular non-symlink file",
        ));
    }
    Ok(path.canonicalize()?)
}

fn validated_external_directory(value: &str) -> Result<PathBuf, EditorCoreError> {
    let path = PathBuf::from(value);
    if value.contains('\0') || !path.is_absolute() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "External destination must be an absolute directory",
        ));
    }
    fs::create_dir_all(&path)?;
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "External destination must be a regular non-symlink directory",
        ));
    }
    Ok(path.canonicalize()?)
}

fn safe_package_label(value: &str) -> Result<&str, EditorCoreError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || value.chars().any(char::is_control)
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Invalid package label: {value}"),
        ));
    }
    Ok(value)
}

fn external_launch_plan(
    kind: &str,
    profile: ExportProfile,
    runtime: Option<PathBuf>,
    artifact_path: PathBuf,
    mut warnings: Vec<String>,
) -> ExternalLaunchPlanDto {
    let runtime = runtime.filter(|path| path.is_file());
    if runtime.is_none() {
        warnings.push(
            "Runtime executable unavailable; build miniforge_runtime or set MINIFORGE_RUNTIME"
                .to_string(),
        );
    }
    ExternalLaunchPlanDto {
        kind: kind.to_string(),
        profile: profile.label().to_string(),
        ready: runtime.is_some(),
        executable: runtime.map(|path| path.to_string_lossy().to_string()),
        arguments: vec![
            "--build".to_string(),
            artifact_path.to_string_lossy().to_string(),
        ],
        working_directory: artifact_path.to_string_lossy().to_string(),
        artifact_path: artifact_path.to_string_lossy().to_string(),
        warnings,
    }
}

fn default_command_descriptors() -> Vec<CommandDescriptor> {
    vec![
        command(
            "project.save",
            "Save Project",
            "Project",
            Some("Cmd/Ctrl+Shift+S"),
        ),
        command("scene.save", "Save Scene", "Scene", Some("Cmd/Ctrl+S")),
        command("scene.audit_tree", "Audit Scene Tree", "Scene", None),
        command(
            "scene.pack_selected",
            "Pack Selected Scene Branch",
            "Scene",
            None,
        ),
        command("edit.undo", "Undo", "Edit", Some("Cmd/Ctrl+Z")),
        command("edit.redo", "Redo", "Edit", Some("Cmd/Ctrl+Shift+Z")),
        command("console.clear", "Clear Console", "Console", None),
        command(
            "console.clear_errors",
            "Clear Console Errors",
            "Console",
            None,
        ),
        command("selection.align_left", "Align Left", "Selection", None),
        command(
            "selection.align_center_x",
            "Align Center X",
            "Selection",
            None,
        ),
        command("selection.align_right", "Align Right", "Selection", None),
        command("selection.align_top", "Align Top", "Selection", None),
        command(
            "selection.align_center_y",
            "Align Center Y",
            "Selection",
            None,
        ),
        command("selection.align_bottom", "Align Bottom", "Selection", None),
        command(
            "selection.distribute_x",
            "Distribute Horizontally",
            "Selection",
            None,
        ),
        command(
            "selection.distribute_y",
            "Distribute Vertically",
            "Selection",
            None,
        ),
        command(
            "selection.group",
            "Group Selection",
            "Selection",
            Some("Cmd/Ctrl+Shift+G"),
        ),
        command(
            "selection.ungroup",
            "Ungroup Selection",
            "Selection",
            Some("Cmd/Ctrl+Shift+U"),
        ),
        command(
            "selection.toggle_layer_lock",
            "Toggle Layer Lock",
            "Selection",
            None,
        ),
        command(
            "selection.toggle_layer_visibility",
            "Toggle Layer Visibility",
            "Selection",
            None,
        ),
        command(
            "selection.cycle_layer",
            "Move Selection to Next Layer",
            "Selection",
            None,
        ),
        command("entity.create_empty", "Create Empty Entity", "Entity", None),
        command("object.create_node2d", "Create Node2D", "Objects", None),
        command(
            "object.create_sprite_actor",
            "Create Sprite Actor",
            "Objects",
            None,
        ),
        command("object.create_camera", "Create Camera Rig", "Objects", None),
        command(
            "object.create_point_light2d",
            "Create Point Light 2D",
            "Lighting",
            None,
        ),
        command(
            "object.create_spot_light2d",
            "Create Spot Light 2D",
            "Lighting",
            None,
        ),
        command(
            "object.create_shadow_occluder2d",
            "Create Shadow Occluder 2D",
            "Lighting",
            None,
        ),
        command("object.create_area2d", "Create Area2D", "Objects", None),
        command(
            "object.create_rigidbody2d",
            "Create Rigidbody2D",
            "Physics",
            None,
        ),
        command(
            "object.create_static_body2d",
            "Create StaticBody2D",
            "Physics",
            None,
        ),
        command(
            "object.create_trigger_volume2d",
            "Create Trigger Volume 2D",
            "Physics",
            None,
        ),
        command(
            "object.create_one_way_platform2d",
            "Create One-Way Platform 2D",
            "Physics",
            None,
        ),
        command(
            "object.create_character_body2d",
            "Create CharacterBody2D",
            "Objects",
            None,
        ),
        command(
            "object.create_nav_agent2d",
            "Create Navigation Agent 2D",
            "Navigation",
            None,
        ),
        command(
            "object.create_particle_emitter2d",
            "Create Particle Emitter 2D",
            "Effects",
            None,
        ),
        command(
            "object.create_audio_emitter2d",
            "Create Audio Emitter 2D",
            "Audio",
            None,
        ),
        command("object.create_ui_text", "Create HUD Text", "Objects", None),
        command(
            "gameplay.spawn_unit",
            "Create Gameplay Unit",
            "Gameplay",
            None,
        ),
        command("gameplay.spawn_enemy", "Create Enemy AI", "Gameplay", None),
        command(
            "gameplay.spawn_resource",
            "Create Resource Node",
            "Gameplay",
            None,
        ),
        command("rts.spawn_base", "Create RTS Command Center", "RTS", None),
        command(
            "rts.queue_worker",
            "Queue Worker on Selected Building",
            "RTS",
            None,
        ),
        command(
            "rts.place_barracks",
            "Place Barracks Construction Site",
            "RTS",
            None,
        ),
        command(
            "scene.starter_topdown",
            "Create TopDown Starter Scene",
            "Scene Templates",
            None,
        ),
        command(
            "scene.starter_platformer",
            "Create Platformer Starter Scene",
            "Scene Templates",
            None,
        ),
        command(
            "scene.starter_rts",
            "Create RTS Skirmish Scene",
            "Scene Templates",
            None,
        ),
        command(
            "project.template_rts",
            "Create RTS Project Files",
            "Project Templates",
            None,
        ),
        command(
            "project.template_actionrpg",
            "Create Action RPG Project Files",
            "Project Templates",
            None,
        ),
        command(
            "project.template_survival",
            "Create Survival Project Files",
            "Project Templates",
            None,
        ),
        command("project.audit", "Run Project Audit", "Project", None),
        command(
            "forge_ai.project_doctor",
            "Run Forge AI Project Doctor",
            "Forge AI",
            None,
        ),
        command(
            "forge_ai.enemy_smoke",
            "Run Forge AI Enemy Smoke Test",
            "Forge AI",
            None,
        ),
        command("play.enter", "Enter Play Mode", "Play", Some("F5")),
        command("play.stop", "Stop Play Mode", "Play", Some("Shift+F5")),
        command("assets.refresh", "Refresh Assets", "Assets", None),
        command(
            "render.write_2d_profile",
            "Write 2D Render Backend Profile",
            "Rendering",
            None,
        ),
        command(
            "sprite.new_pixel_art",
            "New Pixel Art Sprite",
            "Sprite",
            None,
        ),
        command(
            "sprite.create_hero_template",
            "Create Hero Sprite Template",
            "Sprite",
            None,
        ),
        command(
            "sprite.export_frames",
            "Create SpriteFrames Asset",
            "Sprite",
            None,
        ),
        command(
            "sprite.export_atlas_pages",
            "Export Sprite Atlas Pages",
            "Sprite",
            None,
        ),
        command(
            "sprite.optimize_palette",
            "Create Palette Ramp",
            "Sprite",
            None,
        ),
        command(
            "luau.new_controller",
            "New Luau 2D Controller",
            "Luau",
            None,
        ),
        command(
            "luau.validate_scripts",
            "Validate Luau Scripts",
            "Luau",
            None,
        ),
    ]
}

fn command(id: &str, label: &str, category: &str, shortcut: Option<&str>) -> CommandDescriptor {
    CommandDescriptor {
        id: id.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        shortcut: shortcut.map(str::to_string),
        enabled: false,
    }
}

fn create_pixel_art_sprite(
    project_path: &Path,
    stem: &str,
    hero_template: bool,
) -> Result<PathBuf, EditorCoreError> {
    let path = unique_project_file(project_path, "assets/sprites", stem, "png")?;
    let primary = SpriteColor {
        r: 92,
        g: 198,
        b: 128,
        a: 255,
    };
    let accent = SpriteColor {
        r: 244,
        g: 205,
        b: 92,
        a: 255,
    };
    let mut canvas = if hero_template {
        SpriteEditorCanvas::create_pixel_art_character(48, 48, primary, accent)
    } else {
        let mut canvas = SpriteEditorCanvas::new(32, 32);
        canvas.fill_circle(16, 15, 9, primary);
        canvas.fill_circle(13, 12, 2, SpriteColor::WHITE);
        canvas.draw_rect_outline(8, 8, 17, 17, accent);
        canvas.outline_alpha_thick(
            1,
            SpriteColor {
                r: 20,
                g: 24,
                b: 30,
                a: 255,
            },
        );
        canvas.drop_shadow(
            2,
            2,
            SpriteColor {
                r: 0,
                g: 0,
                b: 0,
                a: 96,
            },
        );
        canvas
    };
    canvas.save_png(&path)?;
    Ok(path)
}

fn create_spriteframes_asset(project_path: &Path) -> Result<PathBuf, EditorCoreError> {
    let path = unique_project_file(
        project_path,
        "assets/animations",
        "SpriteFrames",
        "spriteframes",
    )?;
    let canvas = SpriteEditorCanvas::new(64, 16);
    let draft = canvas.animation_clip_draft("idle", 16, 16, 8.0);
    let json = serde_json::to_string_pretty(&draft)?;
    fs::write(&path, json)?;
    Ok(path)
}

fn export_project_sprite_atlas_pages(
    project_path: &Path,
) -> Result<crate::engine::render_2d::SpriteAtlasExportReport2D, EditorCoreError> {
    let sources = collect_project_sprite_sources(project_path)?;
    if sources.is_empty() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::NotFound,
            "No sprite images found under assets/sprites",
        ));
    }
    export_sprite_atlas_pages_from_files(
        &sources,
        project_path.join("assets/atlases"),
        SpriteAtlasExportOptions2D {
            atlas_name: "ProjectSprites".to_string(),
            width: 2048,
            height: 2048,
            extrude: 1,
            trim_transparent: true,
            power_of_two_pages: true,
            output_prefix: "ProjectSprites".to_string(),
            source_root: Some(project_path.to_path_buf()),
        },
    )
    .map_err(|error| EditorCoreError::new(EditorCoreErrorKind::CommandFailed, error.to_string()))
}

fn write_render_2d_profile_report(project_path: &Path) -> Result<PathBuf, EditorCoreError> {
    let output = project_path.join("project/reports/render_2d_backend_profile.json");
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let macroquad =
        Render2DCompatibilityProfile::from_config(&RenderBackendConfig::default(), 50_000, 1024);
    let opengl = Render2DCompatibilityProfile::from_config(
        &RenderBackendConfig {
            backend: "opengl".to_string(),
            gpu_particles: true,
            ..RenderBackendConfig::default()
        },
        30_000,
        1024,
    );
    let metal = Render2DCompatibilityProfile::from_config(
        &RenderBackendConfig {
            backend: "wgpu".to_string(),
            experimental_wgpu: true,
            gpu_particles: true,
            tilemap_chunk_batching: true,
            ..RenderBackendConfig::default()
        },
        120_000,
        2048,
    );
    let report = serde_json::json!({
        "version": 1,
        "goal": "large_2d_world_first",
        "profiles": {
            "macroquad_stable": macroquad,
            "opengl_compatibility": opengl,
            "metal_experimental": metal,
        }
    });
    fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    Ok(output)
}

fn write_packed_scene_asset(
    project_path: &Path,
    root_name: &str,
    packed: &crate::engine::packed_scene::PackedScene2D,
) -> Result<PathBuf, EditorCoreError> {
    let stem = sanitize_asset_stem(root_name);
    let path = unique_project_file(project_path, "assets/packed_scenes", &stem, "mpscene.json")?;
    fs::write(&path, serde_json::to_vec_pretty(packed)?)?;
    Ok(path)
}

fn sanitize_asset_stem(name: &str) -> String {
    let stem = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        "PackedScene".to_string()
    } else {
        stem
    }
}

fn collect_project_sprite_sources(
    project_path: &Path,
) -> Result<Vec<(String, PathBuf)>, EditorCoreError> {
    let sprites_root = project_path.join("assets/sprites");
    if !sprites_root.exists() {
        return Ok(Vec::new());
    }
    let mut sources = Vec::new();
    for entry in WalkDir::new(&sprites_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !entry.file_type().is_file() || !is_sprite_image_path(path) {
            continue;
        }
        let relative = path.strip_prefix(&sprites_root).unwrap_or(path);
        let name = relative
            .with_extension("")
            .to_string_lossy()
            .replace(['/', '\\', ' '], "_");
        sources.push((name, path.to_path_buf()));
    }
    sources.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    Ok(sources)
}

fn create_palette_ramp_asset(project_path: &Path) -> Result<PathBuf, EditorCoreError> {
    let path = unique_project_file(project_path, "assets/palettes", "SpriteRamp", "json")?;
    let ramp = SpriteEditorCanvas::palette_ramp(
        SpriteColor {
            r: 94,
            g: 198,
            b: 132,
            a: 255,
        },
        8,
    );
    let json = serde_json::to_string_pretty(&ramp)?;
    fs::write(&path, json)?;
    Ok(path)
}

fn create_luau_controller(project_path: &Path) -> Result<PathBuf, EditorCoreError> {
    let path = unique_project_file(project_path, "scripts", "PlayerController", "luau")?;
    fs::write(
        &path,
        r#"local speed = 180.0

function on_start()
    set_sprite("assets/sprites/player.sprite.json")
end

function on_update(dt: number)
    local x = Input.axis("A", "D")
    local y = Input.axis("W", "S")
    move(x * speed * dt, y * speed * dt)

    if x < 0 then
        face_left()
    elseif x > 0 then
        face_right()
    end
end
"#,
    )?;
    Ok(path)
}

fn validate_luau_scripts(project_path: &Path) -> Result<(usize, Vec<String>), EditorCoreError> {
    let scripts_path = project_path.join("scripts");
    if !scripts_path.exists() {
        return Ok((0, Vec::new()));
    }
    let mut checked = 0usize;
    let mut errors = Vec::new();
    for entry in WalkDir::new(&scripts_path)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() || !is_luau_path(entry.path()) {
            continue;
        }
        checked += 1;
        let source = fs::read_to_string(entry.path())?;
        let name = entry
            .path()
            .strip_prefix(project_path)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();
        if let Err(error) = LuauScriptRuntime::validate_source(&source, &name) {
            errors.push(error);
        }
    }
    Ok((checked, errors))
}

fn unique_project_file(
    project_path: &Path,
    relative_dir: &str,
    stem: &str,
    extension: &str,
) -> Result<PathBuf, EditorCoreError> {
    let directory = project_path.join(relative_dir);
    fs::create_dir_all(&directory)?;
    for index in 0..10_000 {
        let name = if index == 0 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}_{index}.{extension}")
        };
        let path = directory.join(name);
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(EditorCoreError::new(
        EditorCoreErrorKind::Io,
        format!("Could not allocate unique file name under {relative_dir}"),
    ))
}

fn project_relative(project_path: &Path, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(project_path) {
        return relative.to_string_lossy().to_string();
    }
    let canonical_project = project_path.canonicalize().ok();
    let canonical_path = path.canonicalize().ok();
    canonical_path
        .as_deref()
        .unwrap_or(path)
        .strip_prefix(canonical_project.as_deref().unwrap_or(project_path))
        .unwrap_or(canonical_path.as_deref().unwrap_or(path))
        .to_string_lossy()
        .to_string()
}

fn is_luau_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("luau") || extension.eq_ignore_ascii_case("lua")
        })
}

#[derive(Debug)]
struct ContentFileTemplate {
    directory: &'static str,
    suffix: &'static str,
    source: String,
    luau: bool,
    visual_graph: bool,
}

#[derive(Debug)]
struct VisibleContentChild {
    path: PathBuf,
    name: String,
    is_directory: bool,
    bytes: u64,
    modified_ms: u64,
}

fn collect_content_folders(project_root: &Path) -> Result<Vec<ContentFolderDto>, EditorCoreError> {
    let root = resolve_content_directory(project_root, "", false)?;
    let project_name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Project")
        .to_string();
    let mut pending = vec![(root, String::new(), project_name, 0usize)];
    let mut rows = Vec::new();
    const MAX_FOLDER_ROWS: usize = 10_000;
    const MAX_FOLDER_DEPTH: usize = 48;
    while let Some((path, relative_path, name, depth)) = pending.pop() {
        if rows.len() >= MAX_FOLDER_ROWS {
            break;
        }
        let children = visible_content_children(&path)?;
        let asset_count = children.iter().filter(|child| !child.is_directory).count();
        let child_folders = children
            .iter()
            .filter(|child| child.is_directory && depth < MAX_FOLDER_DEPTH)
            .collect::<Vec<_>>();
        rows.push(ContentFolderDto {
            path: relative_path,
            name: if name.is_empty() {
                "Project".to_string()
            } else {
                name
            },
            depth,
            asset_count,
            child_folder_count: child_folders.len(),
        });
        for child in child_folders.into_iter().rev() {
            pending.push((
                child.path.clone(),
                normalized_project_relative(project_root, &child.path),
                child.name.clone(),
                depth + 1,
            ));
        }
    }
    Ok(rows)
}

fn collect_content_entries(
    project_root: &Path,
    database: &crate::engine::asset_database::AssetDatabase,
    relative_directory: &str,
) -> Result<Vec<ContentEntryDto>, EditorCoreError> {
    let directory = resolve_content_directory(project_root, relative_directory, false)?;
    let mut reverse_dependencies = BTreeMap::<String, Vec<String>>::new();
    for (consumer, record) in &database.assets {
        for dependency in &record.dependencies {
            reverse_dependencies
                .entry(dependency.clone())
                .or_default()
                .push(consumer.clone());
        }
    }
    for consumers in reverse_dependencies.values_mut() {
        consumers.sort();
        consumers.dedup();
    }

    visible_content_children(&directory)?
        .into_iter()
        .map(|child| {
            let relative_path = normalized_project_relative(project_root, &child.path);
            let record = database.assets.get(&relative_path);
            let asset_type = if child.is_directory {
                "Folder".to_string()
            } else {
                let inferred = content_asset_type(&child.path);
                if matches!(inferred.as_str(), "File" | "Data" | "Texture" | "Audio") {
                    record
                        .map(|record| record.asset_type.clone())
                        .unwrap_or(inferred)
                } else {
                    inferred
                }
            };
            let dependencies = record
                .map(|record| record.dependencies.clone())
                .unwrap_or_default();
            let mut warnings = record
                .map(|record| record.compatibility.clone())
                .unwrap_or_default();
            warnings.extend(
                dependencies
                    .iter()
                    .filter(|dependency| !database.assets.contains_key(*dependency))
                    .map(|dependency| format!("Missing dependency: {dependency}")),
            );
            warnings.sort();
            warnings.dedup();
            let child_count = if child.is_directory {
                visible_content_children(&child.path)?.len()
            } else {
                0
            };
            let include_in_build = record
                .and_then(|record| record.import_settings.get("include_in_build"))
                .and_then(Value::as_bool)
                .unwrap_or(!child.is_directory);
            Ok(ContentEntryDto {
                name: child.name,
                relative_path: relative_path.clone(),
                asset_type,
                is_directory: child.is_directory,
                editable: !child.is_directory && is_editable_content_path(&child.path),
                bytes: child.bytes,
                modified_ms: child.modified_ms,
                child_count,
                preview_url: if !child.is_directory && is_previewable_image(&child.path) {
                    content_file_url(&child.path)
                } else {
                    String::new()
                },
                guid: record.map(|record| record.guid.clone()).unwrap_or_default(),
                labels: record
                    .map(|record| record.labels.clone())
                    .unwrap_or_default(),
                include_in_build,
                dependencies,
                reverse_dependencies: reverse_dependencies
                    .get(&relative_path)
                    .cloned()
                    .unwrap_or_default(),
                warnings,
            })
        })
        .collect()
}

fn visible_content_children(directory: &Path) -> Result<Vec<VisibleContentChild>, EditorCoreError> {
    let mut rows = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            continue;
        }
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
            .unwrap_or_default();
        rows.push(VisibleContentChild {
            path,
            name,
            is_directory: metadata.is_dir(),
            bytes: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified_ms,
        });
    }
    rows.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(rows)
}

fn validate_content_entry_name(name: &str, kind: &str) -> Result<(), EditorCoreError> {
    let name = name.trim();
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.contains(['/', '\\', '\0'])
        || name.chars().any(char::is_control)
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("{kind} name cannot be empty or contain path separators/control characters"),
        ));
    }
    Ok(())
}

fn resolve_content_directory(
    project_root: &Path,
    relative: &str,
    create: bool,
) -> Result<PathBuf, EditorCoreError> {
    let path = resolve_content_candidate(project_root, relative, true)?;
    if create && !path.exists() {
        fs::create_dir_all(&path)?;
    }
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        EditorCoreError::new(
            if error.kind() == std::io::ErrorKind::NotFound {
                EditorCoreErrorKind::NotFound
            } else {
                EditorCoreErrorKind::Io
            },
            format!("Content directory does not exist: {relative}"),
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Content directory is invalid: {relative}"),
        ));
    }
    Ok(path)
}

fn resolve_content_file(project_root: &Path, relative: &str) -> Result<PathBuf, EditorCoreError> {
    if relative.trim().is_empty() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Content file path cannot be empty",
        ));
    }
    let path = resolve_content_candidate(project_root, relative, false)?;
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        EditorCoreError::new(
            if error.kind() == std::io::ErrorKind::NotFound {
                EditorCoreErrorKind::NotFound
            } else {
                EditorCoreErrorKind::Io
            },
            format!("Content file does not exist: {relative}"),
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Content path is not a regular project file: {relative}"),
        ));
    }
    Ok(path)
}

fn resolve_content_candidate(
    project_root: &Path,
    relative: &str,
    allow_empty: bool,
) -> Result<PathBuf, EditorCoreError> {
    let relative = relative.trim().replace('\\', "/");
    if relative.contains('\0') || (!allow_empty && relative.is_empty()) {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Content path must be project-relative",
        ));
    }
    let relative_path = Path::new(&relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Content path must stay inside the project: {relative}"),
        ));
    }
    let canonical_root = project_root.canonicalize()?;
    let path = project_root.join(relative_path);
    let mut current = project_root.to_path_buf();
    for component in relative_path.components() {
        let std::path::Component::Normal(part) = component else {
            unreachable!("relative path components were validated")
        };
        current.push(part);
        if current.exists() && fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::InvalidArgument,
                format!("Content path cannot traverse a symbolic link: {relative}"),
            ));
        }
    }
    let mut ancestor = path.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or_else(|| {
            EditorCoreError::new(EditorCoreErrorKind::InvalidArgument, "Invalid content path")
        })?;
    }
    if !ancestor.canonicalize()?.starts_with(&canonical_root) {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Content path escapes the project: {relative}"),
        ));
    }
    if path.exists() && !path.canonicalize()?.starts_with(&canonical_root) {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Content path escapes the project: {relative}"),
        ));
    }
    Ok(path)
}

fn normalized_project_relative(project_root: &Path, path: &Path) -> String {
    project_relative(project_root, path).replace('\\', "/")
}

fn is_editable_content_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "luau"
                    | "lua"
                    | "scene"
                    | "prefab"
                    | "mfgraph"
                    | "mfui"
                    | "mftilemap"
                    | "json"
                    | "toml"
                    | "ron"
                    | "yaml"
                    | "yml"
                    | "wgsl"
                    | "glsl"
                    | "txt"
                    | "md"
            )
        })
}

fn content_path_requires_json(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "json" | "scene" | "prefab" | "mfgraph" | "mfui" | "mftilemap"
    )
}

fn is_previewable_image(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp"
            )
        })
}

fn content_file_url(path: &Path) -> String {
    let encoded = path
        .to_string_lossy()
        .replace('\\', "/")
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23")
        .replace('?', "%3F");
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

fn content_asset_type(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let kind = if name.ends_with(".ui.prefab") || extension == "mfui" {
        "UI"
    } else if name.ends_with(".tilemap2d.json")
        || name.ends_with(".tilemap.json")
        || extension == "mftilemap"
    {
        "Tilemap"
    } else if name.ends_with(".sound.json") {
        "SoundCue"
    } else if name.ends_with(".audio.json") {
        "AudioEvent"
    } else if name.ends_with(".sprite.json") {
        "Sprite"
    } else if name.ends_with(".spritesheet.json") {
        "SpriteSheet"
    } else if matches!(extension.as_str(), "luau" | "lua") {
        "LuauScript"
    } else if extension == "scene" {
        "Scene"
    } else if extension == "prefab" {
        "Prefab"
    } else if extension == "mfgraph" {
        "VisualGraph"
    } else if name.ends_with(".material.json") {
        "Material"
    } else if name.ends_with(".shader.json") || matches!(extension.as_str(), "wgsl" | "glsl") {
        "Shader"
    } else if name.ends_with(".particles.json") {
        "ParticlePreset"
    } else if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp") {
        "Texture"
    } else if matches!(extension.as_str(), "wav" | "ogg" | "mp3" | "flac") {
        "Audio"
    } else if matches!(extension.as_str(), "json" | "toml" | "ron" | "yaml" | "yml") {
        "Data"
    } else {
        "File"
    };
    kind.to_string()
}

fn safe_content_stem(value: &str) -> String {
    let mut output = String::new();
    for character in value.trim().chars() {
        if character.is_alphanumeric() || matches!(character, '_' | '-') {
            output.push(character);
        } else if character.is_whitespace() || character == '.' {
            output.push('_');
        }
        if output.chars().count() >= 96 {
            break;
        }
    }
    while output.contains("__") {
        output = output.replace("__", "_");
    }
    if output.is_empty() {
        "NewAsset".to_string()
    } else {
        output
    }
}

fn unique_content_path(directory: &Path, stem: &str, suffix: &str) -> PathBuf {
    let direct = directory.join(format!("{stem}{suffix}"));
    if !direct.exists() {
        return direct;
    }
    for index in 2.. {
        let candidate = directory.join(format!("{stem}_{index}{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn json_content_template(value: Value) -> String {
    let mut source = serde_json::to_string_pretty(&value).expect("JSON Value serialization");
    source.push('\n');
    source
}

fn content_file_template(kind: &str, raw_name: &str) -> ContentFileTemplate {
    let kind = kind.trim().to_ascii_lowercase();
    let name = safe_content_stem(raw_name);
    let version = crate::engine::version::ENGINE_VERSION;
    match kind.as_str() {
        "luau" | "script" => ContentFileTemplate {
            directory: "scripts",
            suffix: ".luau",
            source: format!(
                "--!strict\n\nlocal {name} = {{}}\n\nfunction {name}:on_ready()\n    -- Initialize this entity.\nend\n\nfunction {name}:on_update(dt: number)\n    -- Frame-rate independent gameplay logic.\nend\n\nreturn {name}\n"
            ),
            luau: true,
            visual_graph: false,
        },
        "scene" => ContentFileTemplate {
            directory: "saves/scenes",
            suffix: ".scene",
            source: json_content_template(AssetTools::template_scene(&name)),
            luau: false,
            visual_graph: false,
        },
        "visual_graph" | "blueprint" => ContentFileTemplate {
            directory: "scripts/visual_graphs",
            suffix: ".mfgraph",
            source: json_content_template(json!({
                "format": "miniforge.visual-graph",
                "schema_version": 1,
                "engine_version": version,
                "version": version,
                "kind": "MiniForgeVisualGraph",
                "runtime": "rust_visual_graph",
                "name": name,
                "variables": {},
                "editor": {},
                "nodes": [
                    {"id":"start","type":"EventStart","x":80,"y":100,"next":"log"},
                    {"id":"log","type":"Log","x":340,"y":100,"message":"Blueprint ready","next":null}
                ]
            })),
            luau: false,
            visual_graph: true,
        },
        "prefab" => ContentFileTemplate {
            directory: "assets/prefabs",
            suffix: ".prefab",
            source: json_content_template(AssetTools::template_prefab(&name)),
            luau: false,
            visual_graph: false,
        },
        "material" => ContentFileTemplate {
            directory: "assets/data",
            suffix: ".material.json",
            source: json_content_template(AssetTools::template_material(&name)),
            luau: false,
            visual_graph: false,
        },
        "shader" => ContentFileTemplate {
            directory: "assets/data",
            suffix: ".shader.json",
            source: json_content_template(AssetTools::template_shader(&name)),
            luau: false,
            visual_graph: false,
        },
        "ui" | "ui_canvas" => ContentFileTemplate {
            directory: "assets/ui",
            suffix: ".mfui",
            source: json_content_template(json!({
                "name": name,
                "viewport_width": 1280.0,
                "viewport_height": 720.0,
                "widgets": [{
                    "id":"RootCanvas", "widget_type":"Canvas",
                    "rect":{"x":0.0,"y":0.0,"width":0.0,"height":0.0},
                    "anchors":{"min_x":0.0,"min_y":0.0,"max_x":1.0,"max_y":1.0},
                    "children":[], "callbacks":[], "properties":{}, "style":{},
                    "bindings":[], "navigation":{}
                }],
                "theme":{"name":"Default","styles":{}},
                "animations":[]
            })),
            luau: false,
            visual_graph: false,
        },
        "tilemap" | "tilemap2d" => {
            let empty_tiles = vec![vec![0u32; 16]; 12];
            let layers = ["Ground", "Decoration", "Collision", "Overlay"]
                .into_iter()
                .map(|layer| {
                    json!({
                        "name": layer, "visible": true, "locked": false, "tiles": empty_tiles
                    })
                })
                .collect::<Vec<_>>();
            ContentFileTemplate {
                directory: "assets/tilemaps",
                suffix: ".tilemap2d.json",
                source: json_content_template(json!({
                    "tilemap":{"width":16,"height":12,"active_layer":0,"layers":layers},
                    "active_layer":0, "active_brush":"Pencil",
                    "palette":{"name":"DefaultTiles","selected":1,"tiles":{"Grass":1,"Dirt":2,"Stone":3,"Water":4},"tags":{}},
                    "terrain_sets":[], "rule_tiles":[], "stamps":[], "object_brushes":[],
                    "placed_objects":[], "last_strokes":[],
                    "selection":{"layer":0,"cells":[]}, "clipboard":null,
                    "random_seed":12648430u32
                })),
                luau: false,
                visual_graph: false,
            }
        }
        "sound_cue" | "soundcue" => ContentFileTemplate {
            directory: "assets/audio",
            suffix: ".sound.json",
            source: json_content_template(AssetTools::template_sound_cue(&name, "")),
            luau: false,
            visual_graph: false,
        },
        "config" | "resource" => ContentFileTemplate {
            directory: "settings",
            suffix: ".json",
            source: json_content_template(json!({"version":1,"name":name,"enabled":true})),
            luau: false,
            visual_graph: false,
        },
        _ => ContentFileTemplate {
            directory: "assets/data",
            suffix: ".json",
            source: "{\n}\n".to_string(),
            luau: false,
            visual_graph: false,
        },
    }
}

fn resolve_luau_script_path(
    project_path: &Path,
    relative_path: &str,
    must_exist: bool,
) -> Result<PathBuf, EditorCoreError> {
    let relative = Path::new(relative_path);
    if relative_path.trim().is_empty() || relative.is_absolute() || relative_path.contains('\0') {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Luau path must be a non-empty project-relative path",
        ));
    }
    let mut components = relative.components();
    if !matches!(
        components.next(),
        Some(std::path::Component::Normal(part)) if part.to_str() == Some("scripts")
    ) || components.any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Luau path must stay inside scripts/: {relative_path}"),
        ));
    }
    if !is_luau_path(relative) {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Luau script must use .luau or .lua: {relative_path}"),
        ));
    }
    let path = project_path.join(relative);
    if must_exist && !path.is_file() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::NotFound,
            format!("Luau script not found: {relative_path}"),
        ));
    }
    Ok(path)
}

fn resolve_visual_graph_path(
    project_path: &Path,
    relative_path: &str,
    must_exist: bool,
) -> Result<PathBuf, EditorCoreError> {
    let relative = Path::new(relative_path);
    if relative_path.trim().is_empty() || relative.is_absolute() || relative_path.contains('\0') {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Visual Graph path must be project-relative",
        ));
    }
    let expected_root = Path::new("scripts").join("visual_graphs");
    if !relative.starts_with(&expected_root)
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
        || !relative
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mfgraph"))
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Visual Graph path must stay inside scripts/visual_graphs/: {relative_path}"),
        ));
    }
    let path = project_path.join(relative);
    if must_exist && !path.is_file() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::NotFound,
            format!("Visual Graph not found: {relative_path}"),
        ));
    }
    Ok(path)
}

fn validate_editor_script_size(source: &str) -> Result<(), EditorCoreError> {
    if source.len() > MAX_EDITOR_SCRIPT_BYTES {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!(
                "Luau source exceeds the editor limit of {} MiB",
                MAX_EDITOR_SCRIPT_BYTES / (1024 * 1024)
            ),
        ));
    }
    Ok(())
}

fn manage_asset_rename(
    project_root: &Path,
    payload: &Value,
) -> Result<AssetManageOutcomeDto, EditorCoreError> {
    let source_relative = required_payload_string(payload, "source")?;
    let source = resolve_managed_asset_file(project_root, source_relative)?;
    let new_name = required_payload_string(payload, "new_name")?.trim();
    validate_asset_filename(new_name)?;
    let destination = source.with_file_name(renamed_asset_filename(&source, new_name)?);
    if destination.exists() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!(
                "Asset destination already exists: {}",
                destination.display()
            ),
        ));
    }
    let sidecars = move_asset_bundle(&source, &destination)?;
    Ok(asset_manage_outcome(
        "rename",
        project_root,
        source_relative,
        Some(&destination),
        &sidecars,
    ))
}

fn manage_asset_duplicate(
    project_root: &Path,
    payload: &Value,
) -> Result<AssetManageOutcomeDto, EditorCoreError> {
    let source_relative = required_payload_string(payload, "source")?;
    let source = resolve_managed_asset_file(project_root, source_relative)?;
    let target_folder = payload
        .get("target_folder")
        .and_then(Value::as_str)
        .map(|relative| resolve_managed_asset_folder(project_root, relative, true))
        .transpose()?
        .unwrap_or_else(|| source.parent().unwrap_or(project_root).to_path_buf());
    let filename = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_asset_path(source_relative))?;
    let destination = unique_managed_asset_path(&target_folder, filename);
    fs::copy(&source, &destination)?;
    let sidecars = copy_asset_sidecars(&source, &destination)?;
    Ok(asset_manage_outcome(
        "duplicate",
        project_root,
        source_relative,
        Some(&destination),
        &sidecars,
    ))
}

fn manage_asset_move(
    project_root: &Path,
    payload: &Value,
) -> Result<AssetManageOutcomeDto, EditorCoreError> {
    let source_relative = required_payload_string(payload, "source")?;
    let source = resolve_managed_asset_file(project_root, source_relative)?;
    let target_folder = resolve_managed_asset_folder(
        project_root,
        required_payload_string(payload, "target_folder")?,
        true,
    )?;
    let destination = target_folder.join(
        source
            .file_name()
            .ok_or_else(|| invalid_asset_path(source_relative))?,
    );
    if destination.exists() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!(
                "Asset destination already exists: {}",
                destination.display()
            ),
        ));
    }
    let sidecars = move_asset_bundle(&source, &destination)?;
    Ok(asset_manage_outcome(
        "move",
        project_root,
        source_relative,
        Some(&destination),
        &sidecars,
    ))
}

fn manage_asset_delete(
    project_root: &Path,
    database: &crate::engine::asset_database::AssetDatabase,
    payload: &Value,
) -> Result<AssetManageOutcomeDto, EditorCoreError> {
    let source_relative = required_payload_string(payload, "source")?;
    if !required_payload_bool(payload, "confirm")? {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Asset deletion requires confirm=true",
        ));
    }
    let force = payload
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !force {
        let consumers = database
            .assets
            .values()
            .filter(|asset| {
                if is_adjacent_sidecar_relative(source_relative, &asset.relative_path) {
                    return false;
                }
                asset
                    .dependencies
                    .iter()
                    .any(|dependency| dependency == source_relative)
            })
            .map(|asset| asset.relative_path.clone())
            .collect::<Vec<_>>();
        if !consumers.is_empty() {
            return Err(EditorCoreError::new(
                EditorCoreErrorKind::CommandFailed,
                format!(
                    "Asset is referenced by {}; use force=true only after reviewing dependencies",
                    consumers.join(", ")
                ),
            ));
        }
    }
    let source = resolve_managed_asset_file(project_root, source_relative)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let trash_root = project_root.join(".miniforge/trash");
    fs::create_dir_all(&trash_root)?;
    let trash_folder = AssetTools::unique_path(&trash_root, &stamp.to_string());
    fs::create_dir_all(&trash_folder)?;
    let destination = trash_folder.join(
        source
            .file_name()
            .ok_or_else(|| invalid_asset_path(source_relative))?,
    );
    let sidecars = move_asset_bundle(&source, &destination)?;
    Ok(asset_manage_outcome(
        "trash",
        project_root,
        source_relative,
        Some(&destination),
        &sidecars,
    ))
}

fn manage_asset_import(
    project_root: &Path,
    payload: &Value,
) -> Result<AssetManageOutcomeDto, EditorCoreError> {
    let source_external = PathBuf::from(required_payload_string(payload, "source_external")?);
    if !source_external.is_absolute()
        || !source_external.is_file()
        || fs::symlink_metadata(&source_external)?
            .file_type()
            .is_symlink()
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Imported asset must be an existing absolute, non-symlink file",
        ));
    }
    const MAX_IMPORT_BYTES: u64 = 512 * 1024 * 1024;
    if fs::metadata(&source_external)?.len() > MAX_IMPORT_BYTES {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            "Imported asset exceeds the 512 MiB editor limit",
        ));
    }
    let target_folder = resolve_managed_asset_folder(
        project_root,
        payload
            .get("target_folder")
            .and_then(Value::as_str)
            .unwrap_or("assets"),
        true,
    )?;
    let filename = source_external
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_asset_path(&source_external.to_string_lossy()))?;
    let destination = unique_managed_asset_path(&target_folder, filename);
    fs::copy(&source_external, &destination)?;
    let mut sidecars = copy_asset_sidecars(&source_external, &destination)?;
    let import_sidecar = destination.with_file_name(format!(
        "{}.import.json",
        destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("asset")
    ));
    if !import_sidecar.exists() {
        let size_bytes = fs::metadata(&destination)?.len();
        if let Err(error) = AssetTools::write_json(
            &import_sidecar,
            &json!({
                "format": "miniforge.asset-import",
                "schema_version": 1,
                "source": source_external.to_string_lossy(),
                "size_bytes": size_bytes,
            }),
        ) {
            for sidecar in &sidecars {
                let _ = fs::remove_file(sidecar);
            }
            let _ = fs::remove_file(&destination);
            return Err(error.into());
        }
        sidecars.push(import_sidecar);
    }
    if SpriteSheetImporter::supports_image(&destination) {
        if let Ok(generated) = create_imported_sprite_bundle(project_root, &destination) {
            sidecars.extend(generated);
        }
    }
    sidecars.sort();
    sidecars.dedup();
    Ok(asset_manage_outcome(
        "import",
        project_root,
        &source_external.to_string_lossy(),
        Some(&destination),
        &sidecars,
    ))
}

fn create_imported_sprite_bundle(
    project_root: &Path,
    image_path: &Path,
) -> Result<Vec<PathBuf>, EditorCoreError> {
    let grid = SpriteSheetImporter::infer_grid(image_path)?;
    let metadata =
        SpriteSheetImporter::build_metadata(image_path, grid.cell_width, grid.cell_height, 0, 0)?;
    let mut generated = Vec::new();
    let result = (|| -> Result<(), EditorCoreError> {
        let sheet_path = SpriteSheetImporter::write_sidecar(image_path, &metadata)?;
        generated.push(sheet_path.clone());

        let sprite_name = image_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Sprite")
            .to_string();
        let source_ref = normalized_project_relative(project_root, image_path);
        let sprite_manifest =
            AssetTools::create_sprite_import(project_root, &sprite_name, &source_ref)?;
        generated.push(sprite_manifest.clone());

        let frame_count = metadata.slices.len().max(1);
        let frames_manifest = if frame_count > 1 {
            let animation_folder = project_root.join("assets/animations");
            fs::create_dir_all(&animation_folder)?;
            let path =
                AssetTools::unique_path(&animation_folder, &format!("{sprite_name}.spriteframes"));
            let frames = SpriteFrames2D::grid_slice(
                sprite_name.clone(),
                source_ref,
                grid.columns,
                grid.rows,
                grid.cell_width,
                grid.cell_height,
                8.0,
            );
            ProjectStorage::write_json_atomic(&path, &frames).map_err(|error| {
                EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string())
            })?;
            generated.push(path.clone());
            Some(path)
        } else {
            None
        };

        let mut sprite = AssetTools::read_json(&sprite_manifest)?;
        sprite["atlas"] = json!(normalized_project_relative(project_root, &sheet_path));
        if let Some(frames_path) = frames_manifest {
            sprite["animations"] = json!([{
                "name": "default",
                "asset": normalized_project_relative(project_root, &frames_path),
                "fps": 8.0,
                "frames": frame_count,
            }]);
        }
        ProjectStorage::write_json_atomic(&sprite_manifest, &sprite)
            .map_err(|error| EditorCoreError::new(EditorCoreErrorKind::Io, error.to_string()))?;
        Ok(())
    })();
    if let Err(error) = result {
        for path in &generated {
            let _ = fs::remove_file(path);
        }
        return Err(error);
    }
    Ok(generated)
}

fn resolve_managed_asset_file(
    project_root: &Path,
    relative: &str,
) -> Result<PathBuf, EditorCoreError> {
    let path = resolve_managed_asset_path(project_root, relative)?;
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Managed asset must be a regular file: {relative}"),
        ));
    }
    let canonical_root = project_root.canonicalize()?;
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(&canonical_root) {
        return Err(invalid_asset_path(relative));
    }
    Ok(canonical)
}

fn resolve_managed_asset_folder(
    project_root: &Path,
    relative: &str,
    create: bool,
) -> Result<PathBuf, EditorCoreError> {
    let path = resolve_managed_asset_path(project_root, relative)?;
    if create && !path.exists() {
        let mut existing_ancestor = path.parent();
        while existing_ancestor.is_some_and(|ancestor| !ancestor.exists()) {
            existing_ancestor = existing_ancestor.and_then(Path::parent);
        }
        let ancestor = existing_ancestor.ok_or_else(|| invalid_asset_path(relative))?;
        let ancestor_metadata = fs::symlink_metadata(ancestor)?;
        let canonical_root = project_root.canonicalize()?;
        if ancestor_metadata.file_type().is_symlink()
            || !ancestor.canonicalize()?.starts_with(&canonical_root)
        {
            return Err(invalid_asset_path(relative));
        }
        fs::create_dir_all(&path)?;
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Managed asset folder is invalid: {relative}"),
        ));
    }
    let canonical_root = project_root.canonicalize()?;
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(&canonical_root) {
        return Err(invalid_asset_path(relative));
    }
    Ok(canonical)
}

fn resolve_managed_asset_path(
    project_root: &Path,
    relative: &str,
) -> Result<PathBuf, EditorCoreError> {
    let relative_path = Path::new(relative);
    if relative.trim().is_empty() || relative_path.is_absolute() || relative.contains('\0') {
        return Err(invalid_asset_path(relative));
    }
    let mut components = relative_path.components();
    let allowed_root = matches!(
        components.next(),
        Some(std::path::Component::Normal(root))
            if matches!(
                root.to_str(),
                Some("assets" | "scripts" | "scenes" | "saves" | "settings" | "components" | "systems" | "plugins" | "templates")
            )
    );
    if !allowed_root
        || components.any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(invalid_asset_path(relative));
    }
    Ok(project_root.join(relative_path))
}

fn validate_asset_filename(name: &str) -> Result<(), EditorCoreError> {
    let path = Path::new(name);
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('\0')
        || path.file_name().and_then(|value| value.to_str()) != Some(name)
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Invalid asset filename: {name}"),
        ));
    }
    Ok(())
}

fn renamed_asset_filename(source: &Path, new_name: &str) -> Result<String, EditorCoreError> {
    let source_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_asset_path(&source.to_string_lossy()))?;
    let suffix = managed_asset_suffix(source_name);
    if suffix.is_empty()
        || new_name
            .to_ascii_lowercase()
            .ends_with(&suffix.to_ascii_lowercase())
    {
        return Ok(new_name.to_string());
    }
    if Path::new(new_name).extension().is_some() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Rename cannot change the asset type; expected suffix {suffix}"),
        ));
    }
    Ok(format!("{new_name}{suffix}"))
}

fn managed_asset_suffix(filename: &str) -> String {
    let filename_lower = filename.to_ascii_lowercase();
    [
        ".spritesheet.json",
        ".tilemap2d.json",
        ".sprite.json",
        ".sound.json",
        ".audio.json",
        ".material.json",
        ".shader.json",
        ".particles.json",
        ".ui.prefab",
        ".import.json",
    ]
    .into_iter()
    .find(|suffix| filename_lower.ends_with(suffix))
    .map(ToString::to_string)
    .or_else(|| {
        Path::new(filename)
            .extension()
            .and_then(|value| value.to_str())
            .map(|extension| format!(".{extension}"))
    })
    .unwrap_or_default()
}

fn unique_managed_asset_path(folder: &Path, filename: &str) -> PathBuf {
    let direct = folder.join(filename);
    if !direct.exists() {
        return direct;
    }
    let suffix = managed_asset_suffix(filename);
    let stem = filename.strip_suffix(&suffix).unwrap_or(filename);
    for index in 1.. {
        let candidate = folder.join(format!("{stem}_{index}{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn adjacent_asset_sidecars(source: &Path) -> Vec<PathBuf> {
    let Some(filename) = source.file_name().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    [
        format!("{filename}.import.json"),
        format!("{filename}.meta"),
        format!("{stem}.spritesheet.json"),
    ]
    .into_iter()
    .filter_map(|name| {
        let candidate = source.with_file_name(name);
        candidate.is_file().then_some(candidate)
    })
    .collect()
}

fn sidecar_destination(source: &Path, destination: &Path, sidecar: &Path) -> PathBuf {
    let source_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let destination_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let sidecar_name = sidecar
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let source_stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(source_name);
    let destination_stem = destination
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(destination_name);
    if sidecar_name == format!("{source_stem}.spritesheet.json") {
        return destination.with_file_name(format!("{destination_stem}.spritesheet.json"));
    }
    let suffix = sidecar_name.strip_prefix(source_name).unwrap_or(".meta");
    destination.with_file_name(format!("{destination_name}{suffix}"))
}

fn is_adjacent_sidecar_relative(source: &str, candidate: &str) -> bool {
    if candidate == format!("{source}.import.json") || candidate == format!("{source}.meta") {
        return true;
    }
    let source = Path::new(source);
    let candidate = Path::new(candidate);
    let source_stem = source.file_stem().and_then(|value| value.to_str());
    let candidate_name = candidate.file_name().and_then(|value| value.to_str());
    source.parent() == candidate.parent()
        && source_stem
            .zip(candidate_name)
            .is_some_and(|(stem, name)| name == format!("{stem}.spritesheet.json"))
}

fn copy_asset_sidecars(source: &Path, destination: &Path) -> Result<Vec<PathBuf>, EditorCoreError> {
    let mut copied = Vec::new();
    for sidecar in adjacent_asset_sidecars(source) {
        let target = sidecar_destination(source, destination, &sidecar);
        if let Err(error) = fs::copy(&sidecar, &target) {
            for created in &copied {
                let _ = fs::remove_file(created);
            }
            let _ = fs::remove_file(destination);
            return Err(error.into());
        }
        copied.push(target);
    }
    Ok(copied)
}

fn move_asset_bundle(source: &Path, destination: &Path) -> Result<Vec<PathBuf>, EditorCoreError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let sidecar_pairs = adjacent_asset_sidecars(source)
        .into_iter()
        .map(|sidecar| {
            let target = sidecar_destination(source, destination, &sidecar);
            (sidecar, target)
        })
        .collect::<Vec<_>>();
    fs::rename(source, destination)?;
    let mut moved = Vec::<(PathBuf, PathBuf)>::new();
    for (sidecar, target) in sidecar_pairs {
        if let Err(error) = fs::rename(&sidecar, &target) {
            for (original, moved_target) in moved.iter().rev() {
                let _ = fs::rename(moved_target, original);
            }
            let _ = fs::rename(destination, source);
            return Err(error.into());
        }
        moved.push((sidecar, target));
    }
    Ok(moved.into_iter().map(|(_, target)| target).collect())
}

fn asset_manage_outcome(
    action: &str,
    project_root: &Path,
    source: &str,
    destination: Option<&Path>,
    sidecars: &[PathBuf],
) -> AssetManageOutcomeDto {
    let destination_relative = destination.map(|path| project_relative(project_root, path));
    AssetManageOutcomeDto {
        action: action.to_string(),
        source: source.to_string(),
        destination: destination_relative.clone(),
        sidecars: sidecars
            .iter()
            .map(|path| project_relative(project_root, path))
            .collect(),
        refreshed_asset_count: 0,
        message: destination_relative
            .map(|path| format!("Asset {action} completed: {path}"))
            .unwrap_or_else(|| format!("Asset {action} completed")),
    }
}

fn invalid_asset_path(path: &str) -> EditorCoreError {
    EditorCoreError::new(
        EditorCoreErrorKind::InvalidArgument,
        format!("Asset path must stay inside a managed project folder: {path}"),
    )
}

fn required_payload_string<'a>(payload: &'a Value, key: &str) -> Result<&'a str, EditorCoreError> {
    payload.get(key).and_then(Value::as_str).ok_or_else(|| {
        EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Entity action payload requires string field '{key}'"),
        )
    })
}

fn required_payload_u64(payload: &Value, key: &str) -> Result<u64, EditorCoreError> {
    payload.get(key).and_then(Value::as_u64).ok_or_else(|| {
        EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Entity action payload requires unsigned integer field '{key}'"),
        )
    })
}

fn required_payload_bool(payload: &Value, key: &str) -> Result<bool, EditorCoreError> {
    payload.get(key).and_then(Value::as_bool).ok_or_else(|| {
        EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Entity action payload requires boolean field '{key}'"),
        )
    })
}

fn required_payload_f64(payload: &Value, key: &str) -> Result<f64, EditorCoreError> {
    let value = payload.get(key).and_then(Value::as_f64).ok_or_else(|| {
        EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Entity action payload requires numeric field '{key}'"),
        )
    })?;
    if !value.is_finite() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Entity action field must be finite: {key}"),
        ));
    }
    Ok(value)
}

fn component_asset_path(entity: &GameObject, component_type: &str, key: &str) -> Option<String> {
    entity
        .get_component(component_type)
        .and_then(|component| component.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
}

fn inspector_assets_for_action(
    assets: &[AssetRow],
    action_id: &str,
) -> Vec<InspectorQuickAssetDto> {
    let compatible = |asset_type: &str| match action_id {
        "assign_sprite" => matches!(asset_type, "Sprite" | "SpriteSheet"),
        "assign_material" => matches!(asset_type, "Material" | "Material2D"),
        "assign_texture_slot" => matches!(asset_type, "Texture" | "Texture2D"),
        "attach_script" => matches!(asset_type, "Script" | "LuauScript"),
        "attach_blueprint" => matches!(asset_type, "VisualGraph" | "BlueprintGraph2D"),
        _ => false,
    };
    assets
        .iter()
        .filter(|asset| compatible(&asset.asset_type))
        .map(|asset| InspectorQuickAssetDto {
            relative_path: asset.relative_path.clone(),
            name: asset.name.clone(),
            asset_type: asset.asset_type.clone(),
        })
        .collect()
}

fn selected_entity_ids(game: &Game) -> Result<Vec<u64>, EditorCoreError> {
    let mut seen = BTreeSet::new();
    let selection = game
        .selected_units
        .iter()
        .copied()
        .filter(|id| seen.insert(*id) && game.get_entity_by_id(*id).is_some())
        .collect::<Vec<_>>();
    if selection.is_empty() {
        Err(EditorCoreError::new(
            EditorCoreErrorKind::CommandFailed,
            "Select at least one entity before running a batch action",
        ))
    } else {
        Ok(selection)
    }
}

fn duplicate_selected_entities(game: &mut Game) -> Result<usize, EditorCoreError> {
    let selection = selected_entity_ids(game)?;
    let before = game.capture_editor_snapshot();
    let mut clones = Vec::with_capacity(selection.len());
    let mut id_map = BTreeMap::new();
    for source_id in &selection {
        let mut source = game.get_entity_by_id(*source_id).cloned().ok_or_else(|| {
            EditorCoreError::new(EditorCoreErrorKind::NotFound, "Selected entity is missing")
        })?;
        source.sync_to_components();
        let data = GameObject::serialize(&mut source);
        let mut clone = GameObject::from_data(&data, false);
        clone.name = format!("{}_Copy", source.name);
        clone.x += 1.0;
        clone.y += 1.0;
        clone.path.clear();
        clone.set_selected(false);
        id_map.insert(*source_id, clone.id);
        clones.push((*source_id, clone));
    }
    for (source_id, clone) in &mut clones {
        let source_parent = game
            .get_entity_by_id(*source_id)
            .and_then(|entity| entity.parent_id);
        clone.parent_id =
            source_parent.and_then(|parent| id_map.get(&parent).copied().or(Some(parent)));
        clone.sync_to_components();
    }
    game.clear_selection();
    let clone_ids = clones
        .iter()
        .map(|(_, entity)| entity.id)
        .collect::<Vec<_>>();
    for (_, mut clone) in clones {
        clone.set_selected(true);
        game.runtime_world.units.push(clone);
    }
    game.selected_units = clone_ids;
    game.sync_world();
    game.mark_scene_dirty("Duplicate Selection");
    game.push_editor_command(
        "Duplicate Selection",
        EditorCommandKind::SceneOperation {
            name: "Duplicate Selection".to_string(),
        },
        before,
    );
    Ok(selection.len())
}

fn delete_selected_entities(game: &mut Game) -> Result<usize, EditorCoreError> {
    let selection = selected_entity_ids(game)?;
    let selected = selection.iter().copied().collect::<BTreeSet<_>>();
    let parents = game
        .runtime_world
        .units
        .iter()
        .map(|entity| (entity.id, entity.parent_id))
        .collect::<BTreeMap<_, _>>();
    let before = game.capture_editor_snapshot();
    game.clear_selection();
    game.runtime_world
        .units
        .retain(|entity| !selected.contains(&entity.id));
    for entity in &mut game.runtime_world.units {
        let mut parent = entity.parent_id;
        let mut visited = BTreeSet::new();
        while parent.is_some_and(|id| selected.contains(&id)) {
            let parent_id = parent.expect("checked above");
            if !visited.insert(parent_id) {
                parent = None;
                break;
            }
            parent = parents.get(&parent_id).copied().flatten();
        }
        entity.parent_id = parent;
        entity.set_selected(false);
    }
    game.sync_world();
    game.mark_scene_dirty("Delete Selection");
    game.push_editor_command(
        "Delete Selection",
        EditorCommandKind::SceneOperation {
            name: "Delete Selection".to_string(),
        },
        before,
    );
    Ok(selection.len())
}

fn add_component_to_selected(
    game: &mut Game,
    component_type: &str,
) -> Result<usize, EditorCoreError> {
    let selection = selected_entity_ids(game)?;
    if default_component(component_type).is_none() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Unknown component type: {component_type}"),
        ));
    }
    let targets = selection
        .iter()
        .copied()
        .filter(|id| {
            game.get_entity_by_id(*id)
                .is_some_and(|entity| entity.get_component(component_type).is_none())
        })
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::CommandFailed,
            format!("Every selected entity already has {component_type}"),
        ));
    }
    let before = game.capture_editor_snapshot();
    for entity_id in &targets {
        let entity = game.get_entity_by_id_mut(*entity_id).ok_or_else(|| {
            EditorCoreError::new(EditorCoreErrorKind::NotFound, "Selected entity is missing")
        })?;
        entity.add_component(default_component(component_type).expect("validated component type"));
        entity.sync_from_components();
    }
    game.sync_world();
    game.mark_scene_dirty("Add Component to Selection");
    for entity_id in &targets {
        game.scene_save_manager.note_entity_dirty(*entity_id);
    }
    game.push_editor_command(
        "Add Component to Selection",
        EditorCommandKind::SceneOperation {
            name: format!("Add {component_type} to Selection"),
        },
        before,
    );
    Ok(targets.len())
}

fn component_bundle_types(bundle: &str) -> Option<(&'static str, &'static [&'static str])> {
    match bundle.trim().to_ascii_lowercase().as_str() {
        "topdown_player" | "topdown" => Some((
            "Top-down Player",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Animator2D",
                "Health",
                "Saveable",
            ],
        )),
        "platformer_player" | "platformer" => Some((
            "Platformer Player",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterBody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Animator2D",
                "Health",
                "Checkpoint",
            ],
        )),
        "action_rpg_hero" | "action_rpg" => Some((
            "Action RPG Hero",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Health",
                "Stats",
                "DamageDealer",
                "StatusEffects",
                "Inventory",
                "Equipment",
                "Ability",
                "QuestLog",
                "Saveable",
            ],
        )),
        "enemy_ai" | "enemy" => Some((
            "Enemy AI",
            &[
                "Actor2D",
                "AIController2D",
                "BehaviorTree2D",
                "Blackboard",
                "NavAgent",
                "Collider2D",
                "Health",
                "Stats",
                "DamageDealer",
                "CombatTarget",
                "StatusEffects",
                "LootTable",
            ],
        )),
        "dialogue_npc" | "npc" => Some((
            "Dialogue NPC",
            &[
                "Actor2D",
                "Interaction",
                "Dialogue",
                "ObjectiveMarker",
                "Saveable",
            ],
        )),
        "collectible" | "pickup" => Some((
            "Collectible",
            &[
                "Area2D",
                "Trigger2D",
                "Interaction",
                "LootTable",
                "ParticleEmitter",
                "Saveable",
            ],
        )),
        "camera_rig" | "camera" => {
            Some(("Camera Rig", &["Camera2D", "CameraFollow", "CameraShake"]))
        }
        "audio_emitter" | "audio" => Some(("Audio Emitter", &["AudioSource2D"])),
        "survival_actor" | "survival" => Some((
            "Survival Actor",
            &[
                "Health",
                "SurvivalNeeds",
                "Inventory",
                "Equipment",
                "CraftingBook",
                "StatusEffects",
                "Saveable",
            ],
        )),
        "inventory" => Some(("Inventory", &["Inventory", "Equipment"])),
        "combat_actor" | "combat" => Some((
            "Combat Actor",
            &["Health", "Stats", "DamageDealer", "StatusEffects"],
        )),
        "loot_container" | "lootable" => Some((
            "Loot Container",
            &["LootContainer", "Interaction", "Saveable"],
        )),
        "harvestable" => Some(("Harvestable", &["Harvestable", "Interaction", "Saveable"])),
        "crafting_station" => Some((
            "Crafting Station",
            &["CraftingStation", "Interaction", "Saveable"],
        )),
        _ => None,
    }
}

fn configure_bundle_component(bundle: &str, component: &mut Component) {
    let bundle = bundle.trim().to_ascii_lowercase();
    match (bundle.as_str(), component.component_type.as_str()) {
        ("topdown_player" | "topdown" | "action_rpg_hero" | "action_rpg", "Pawn2D") => {
            component.set("movement_mode", json!("topdown"));
        }
        ("platformer_player" | "platformer", "Pawn2D") => {
            component.set("movement_mode", json!("platformer"));
        }
        (
            "topdown_player" | "topdown" | "action_rpg_hero" | "action_rpg",
            "CharacterController2D",
        ) => {
            component.set("mode", json!("topdown"));
            component.set("jump_force", json!(0.0));
            component.set("max_jumps", json!(0));
        }
        ("topdown_player" | "topdown" | "action_rpg_hero" | "action_rpg", "Rigidbody2D") => {
            component.set("use_gravity", json!(false));
            component.set("gravity_scale", json!(0.0));
            component.set("freeze_rotation", json!(true));
        }
        (
            "topdown_player" | "topdown" | "platformer_player" | "platformer" | "action_rpg_hero"
            | "action_rpg",
            "PlayerController2D",
        ) => {
            component.set("cursor_visible", json!(false));
        }
        ("camera_rig" | "camera", "Camera2D") => {
            component.set("active", json!(true));
            component.set("pixel_perfect", json!(true));
        }
        ("audio_emitter" | "audio", "AudioSource2D") => {
            component.set("spatial_blend", json!(1.0));
        }
        ("collectible" | "pickup", "Interaction") => {
            component.set("prompt", json!("Pick up"));
        }
        _ => {}
    }
}

fn add_component_bundle_to_entities(
    game: &mut Game,
    entity_ids: &[u64],
    bundle: &str,
) -> Result<usize, EditorCoreError> {
    let (label, component_types) = component_bundle_types(bundle).ok_or_else(|| {
        EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Unknown component bundle: {bundle}"),
        )
    })?;
    let targets = entity_ids
        .iter()
        .copied()
        .filter(|id| game.get_entity_by_id(*id).is_some())
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::NotFound,
            "No target entities exist for the component bundle",
        ));
    }

    let before = game.capture_editor_snapshot();
    let mut added = 0usize;
    for entity_id in &targets {
        let entity = game.get_entity_by_id_mut(*entity_id).ok_or_else(|| {
            EditorCoreError::new(EditorCoreErrorKind::NotFound, "Selected entity is missing")
        })?;
        for component_type in component_types {
            if entity.get_component(component_type).is_none() {
                let mut component =
                    default_component(component_type).expect("bundle component must be registered");
                configure_bundle_component(bundle, &mut component);
                entity.add_component(component);
                added += 1;
            }
        }
        entity.sync_from_components();
    }
    if added == 0 {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::CommandFailed,
            format!("Selected entities already contain the {label} systems"),
        ));
    }

    game.sync_world();
    game.mark_scene_dirty("Add Component Bundle");
    for entity_id in &targets {
        game.scene_save_manager.note_entity_dirty(*entity_id);
    }
    game.push_editor_command(
        format!("Add {label} Systems"),
        EditorCommandKind::SceneOperation {
            name: format!("Add {label} component bundle"),
        },
        before,
    );
    Ok(added)
}

fn remove_component_from_selected(
    game: &mut Game,
    component_type: &str,
) -> Result<usize, EditorCoreError> {
    let selection = selected_entity_ids(game)?;
    let targets = selection
        .iter()
        .copied()
        .filter(|id| {
            game.get_entity_by_id(*id)
                .is_some_and(|entity| entity.get_component(component_type).is_some())
        })
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::CommandFailed,
            format!("No selected entity has {component_type}"),
        ));
    }
    for entity_id in &targets {
        let mut probe = game.get_entity_by_id(*entity_id).cloned().ok_or_else(|| {
            EditorCoreError::new(EditorCoreErrorKind::NotFound, "Selected entity is missing")
        })?;
        InspectorEditor::remove_component(&mut probe, component_type)
            .map_err(|message| EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message))?;
    }
    let before = game.capture_editor_snapshot();
    for entity_id in &targets {
        let entity = game.get_entity_by_id_mut(*entity_id).ok_or_else(|| {
            EditorCoreError::new(EditorCoreErrorKind::NotFound, "Selected entity is missing")
        })?;
        InspectorEditor::remove_component(entity, component_type)
            .map_err(|message| EditorCoreError::new(EditorCoreErrorKind::CommandFailed, message))?;
    }
    game.sync_world();
    game.mark_scene_dirty("Remove Component from Selection");
    for entity_id in &targets {
        game.scene_save_manager.note_entity_dirty(*entity_id);
    }
    game.push_editor_command(
        "Remove Component from Selection",
        EditorCommandKind::SceneOperation {
            name: format!("Remove {component_type} from Selection"),
        },
        before,
    );
    Ok(targets.len())
}

fn selected_entity_required(operation: &str) -> EditorCoreError {
    EditorCoreError::new(
        EditorCoreErrorKind::CommandFailed,
        format!("Select an entity before attempting to {operation}"),
    )
}

fn validate_prefab_relative_path(relative_path: &str) -> Result<(), EditorCoreError> {
    let path = Path::new(relative_path);
    if relative_path.trim().is_empty()
        || path.is_absolute()
        || relative_path.contains('\0')
        || !relative_path.to_ascii_lowercase().ends_with(".prefab")
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Invalid project-relative prefab path: {relative_path}"),
        ));
    }
    let mut components = path.components();
    if !matches!(
        components.next(),
        Some(std::path::Component::Normal(part)) if part == "assets"
    ) || components.any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Prefab path must stay inside assets/: {relative_path}"),
        ));
    }
    Ok(())
}

fn luau_backup_path(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("script.luau");
    path.with_file_name(format!("{filename}.bak"))
}

fn parse_export_profile(value: &str) -> Result<ExportProfile, EditorCoreError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "debug" | "development" => Ok(ExportProfile::Debug),
        "release" => Ok(ExportProfile::Release),
        "shipping" => Ok(ExportProfile::Shipping),
        "web_future" | "web-future" => Ok(ExportProfile::WebFuture),
        "macos_app_future" | "macos-app-future" => Ok(ExportProfile::MacosAppFuture),
        other => Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Unknown runtime export profile: {other}"),
        )),
    }
}

fn is_sprite_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp"
            )
        })
}

fn validate_forge_ai_relative_path(path: &str) -> Result<(), EditorCoreError> {
    let path_ref = Path::new(path);
    if path.trim().is_empty() || path_ref.is_absolute() || path.contains('\0') {
        return Err(EditorCoreError::new(
            EditorCoreErrorKind::InvalidArgument,
            format!("Invalid ForgeAI project-relative path: {path}"),
        ));
    }
    for component in path_ref.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => {
                return Err(EditorCoreError::new(
                    EditorCoreErrorKind::InvalidArgument,
                    format!("ForgeAI path escapes the project: {path}"),
                ));
            }
        }
    }
    Ok(())
}

fn child_counts(entities: &[GameObject]) -> BTreeMap<u64, usize> {
    let mut counts = BTreeMap::new();
    for entity in entities {
        if let Some(parent_id) = entity.parent_id {
            *counts.entry(parent_id).or_insert(0) += 1;
        }
    }
    counts
}

fn entity_row(entity: &GameObject, child_counts: &BTreeMap<u64, usize>) -> EntityRow {
    EntityRow {
        id: entity.id,
        parent_id: entity.parent_id,
        name: entity.name.clone(),
        entity_type: entity.entity_type.clone(),
        tag: entity.tag.clone(),
        layer: entity.layer.clone(),
        x: entity.x,
        y: entity.y,
        visible: entity.visible,
        enabled: entity.enabled,
        locked: entity.locked,
        selected: entity.selected,
        component_count: entity.components.len(),
        child_count: child_counts.get(&entity.id).copied().unwrap_or_default(),
    }
}

fn inspector_fields_for_entity(entity: &GameObject) -> Vec<InspectorFieldDto> {
    InspectorEditor::editable_fields(entity)
        .into_iter()
        .map(|field| InspectorFieldDto {
            entity_id: entity.id,
            display_name: title_case(&field.key),
            value_json: field.value.to_string(),
            target: field.target,
            key: field.key,
            value_type: field.value_type,
            editable: field.editable,
        })
        .collect()
}

fn asset_row(record: &AssetRecord) -> AssetRow {
    AssetRow {
        guid: record.guid.clone(),
        relative_path: record.relative_path.clone(),
        name: record.name.clone(),
        asset_type: record.asset_type.clone(),
        size_bytes: record.size_bytes,
        labels: record.labels.clone(),
        dependency_count: record.dependencies.len(),
    }
}

fn readiness_row(area: &crate::engine::system_audit::SystemReadinessArea) -> ReadinessRow {
    ReadinessRow {
        system: area.system.clone(),
        level: area.level,
        score: area.score,
        strength_count: area.strengths.len(),
        gap_count: area.gaps.len(),
        action_count: area.next_actions.len(),
        top_action: area
            .next_actions
            .first()
            .cloned()
            .unwrap_or_else(|| "Mantener cobertura y polish".to_string()),
    }
}

fn title_case(key: &str) -> String {
    key.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_snapshot(game: &Game, width: u32, height: u32) -> ViewportSnapshot {
    let mut rgba = vec![0; width as usize * height as usize * 4];
    fill(&mut rgba, [22, 25, 32, 255]);

    let (tile, scale, offset_x, offset_y) = viewport_layout(game, width, height);

    draw_grid(
        &mut rgba,
        (width, height),
        (game.grid.width, game.grid.height),
        tile * scale,
        (offset_x, offset_y),
    );

    for entity in &game.runtime_world.units {
        if !entity.visible {
            continue;
        }
        let x = offset_x + (entity.x as f32 * tile) * scale;
        let y = offset_y + (entity.y as f32 * tile) * scale;
        let w = (entity.width as f32 * entity.scale_x.abs() as f32 * tile * scale).max(4.0);
        let h = (entity.height as f32 * entity.scale_y.abs() as f32 * tile * scale).max(4.0);
        let color = if entity.selected {
            [94, 170, 255, 255]
        } else if entity.locked {
            [134, 142, 156, 255]
        } else if entity.tag == "Enemy" {
            [232, 92, 88, 255]
        } else if entity.tag == "Player" {
            [76, 201, 137, 255]
        } else {
            [218, 183, 92, 255]
        };
        draw_rotated_rect_center(
            &mut rgba,
            (width, height),
            (x, y),
            (w, h),
            entity.rotation as f32,
            color,
        );
    }

    ViewportSnapshot {
        width,
        height,
        rgba,
    }
}

fn viewport_layout(game: &Game, width: u32, height: u32) -> (f32, f32, f32, f32) {
    let tile = game.grid.tile_size.max(1) as f32;
    let world_w = (game.grid.width.max(1) as f32 * tile).max(1.0);
    let world_h = (game.grid.height.max(1) as f32 * tile).max(1.0);
    let scale = (width as f32 / world_w).min(height as f32 / world_h);
    let offset_x = (width as f32 - world_w * scale) * 0.5;
    let offset_y = (height as f32 - world_h * scale) * 0.5;
    (tile, scale, offset_x, offset_y)
}

fn fill(rgba: &mut [u8], color: [u8; 4]) {
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
}

fn draw_grid(
    rgba: &mut [u8],
    viewport: (u32, u32),
    cells: (usize, usize),
    cell_px: f32,
    offset: (f32, f32),
) {
    if cell_px < 4.0 {
        return;
    }
    let (width, height) = viewport;
    let (cells_x, cells_y) = cells;
    let (offset_x, offset_y) = offset;
    let color = [44, 50, 64, 255];
    for x in 0..=cells_x {
        let px = (offset_x + x as f32 * cell_px).round() as i32;
        draw_line_vertical(rgba, width, height, px, color);
    }
    for y in 0..=cells_y {
        let py = (offset_y + y as f32 * cell_px).round() as i32;
        draw_line_horizontal(rgba, width, height, py, color);
    }
}

fn draw_line_vertical(rgba: &mut [u8], width: u32, height: u32, x: i32, color: [u8; 4]) {
    if x < 0 || x >= width as i32 {
        return;
    }
    for y in 0..height as i32 {
        put_pixel(rgba, width, x, y, color);
    }
}

fn draw_line_horizontal(rgba: &mut [u8], width: u32, height: u32, y: i32, color: [u8; 4]) {
    if y < 0 || y >= height as i32 {
        return;
    }
    for x in 0..width as i32 {
        put_pixel(rgba, width, x, y, color);
    }
}

fn draw_rect_center(
    rgba: &mut [u8],
    viewport: (u32, u32),
    center: (f32, f32),
    size: (f32, f32),
    color: [u8; 4],
) {
    let (width, height) = viewport;
    let (x, y) = center;
    let (w, h) = size;
    let left = (x - w * 0.5).round() as i32;
    let top = (y - h * 0.5).round() as i32;
    let right = (x + w * 0.5).round() as i32;
    let bottom = (y + h * 0.5).round() as i32;
    for py in top.max(0)..bottom.min(height as i32) {
        for px in left.max(0)..right.min(width as i32) {
            put_pixel(rgba, width, px, py, color);
        }
    }
}

fn draw_rotated_rect_center(
    rgba: &mut [u8],
    viewport: (u32, u32),
    center: (f32, f32),
    size: (f32, f32),
    rotation_degrees: f32,
    color: [u8; 4],
) {
    if rotation_degrees.abs() < f32::EPSILON {
        draw_rect_center(rgba, viewport, center, size, color);
        return;
    }
    let (width, height) = viewport;
    let (center_x, center_y) = center;
    let (rect_width, rect_height) = size;
    let radians = -rotation_degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let radius = (rect_width.hypot(rect_height) * 0.5).ceil() as i32;
    let min_x = (center_x as i32 - radius).max(0);
    let max_x = (center_x as i32 + radius).min(width as i32 - 1);
    let min_y = (center_y as i32 - radius).max(0);
    let max_y = (center_y as i32 + radius).min(height as i32 - 1);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let local_x = dx * cos - dy * sin;
            let local_y = dx * sin + dy * cos;
            if local_x.abs() <= rect_width * 0.5 && local_y.abs() <= rect_height * 0.5 {
                put_pixel(rgba, width, x, y, color);
            }
        }
    }
}

fn put_pixel(rgba: &mut [u8], width: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 {
        return;
    }
    let index = (y as usize * width as usize + x as usize) * 4;
    if index + 4 <= rgba.len() {
        rgba[index..index + 4].copy_from_slice(&color);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn editor_core_opens_project_and_exposes_real_entities() {
        let root = temp_project("entities");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        assert!(core.is_project_open());
        assert!(core.entity_count().unwrap() >= 1);
        let entity = core.entity_at(0).unwrap();
        assert!(!entity.name.is_empty());
        assert!(!core.inspector_fields(entity.id).unwrap().is_empty());

        let snapshot = core.viewport_snapshot(64, 48).unwrap();
        assert_eq!(snapshot.rgba.len(), 64 * 48 * 4);
        let health = core.runtime_health().unwrap();
        assert_eq!(health.level, "stable");
        assert!(health.healthy);
        assert_eq!(health.entity_count, core.entity_count().unwrap());
        assert!(health.max_entities >= health.entity_count);
        assert!(health.frame_budget_ms > 0.0);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn editor_core_executes_commands_through_rust_services() {
        let root = temp_project("commands");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        let before = core.entity_count().unwrap();
        let outcome = core.execute_command("entity.create_empty").unwrap();
        assert!(outcome.changed);
        assert_eq!(core.entity_count().unwrap(), before + 1);

        let undo = core.execute_command("edit.undo").unwrap();
        assert!(undo.changed);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_actor_palette_creates_functional_undoable_system_actors() {
        let root = temp_project("native_actor_palette");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        let cases: &[(&str, &[&str])] = &[
            ("object.create_point_light2d", &["Light2D"]),
            ("object.create_spot_light2d", &["Light2D"]),
            (
                "object.create_shadow_occluder2d",
                &["StaticBody2D", "Collider2D", "ShadowCaster2D"],
            ),
            ("object.create_rigidbody2d", &["Rigidbody2D", "Collider2D"]),
            (
                "object.create_static_body2d",
                &["StaticBody2D", "Collider2D"],
            ),
            ("object.create_trigger_volume2d", &["Area2D", "Trigger2D"]),
            (
                "object.create_one_way_platform2d",
                &["StaticBody2D", "Collider2D", "OneWayPlatform2D"],
            ),
            (
                "object.create_nav_agent2d",
                &["NavAgent", "Collider2D", "Selectable"],
            ),
            ("object.create_particle_emitter2d", &["ParticleEmitter"]),
            ("object.create_audio_emitter2d", &["AudioSource"]),
        ];

        for (command_id, expected_components) in cases {
            let before = core.entity_count().unwrap();
            assert!(core.execute_command(command_id).unwrap().changed);
            assert_eq!(core.entity_count().unwrap(), before + 1, "{command_id}");
            let entity_id = core.game().unwrap().selected_units[0];
            let entity = core.game().unwrap().get_entity_by_id(entity_id).unwrap();
            for component_type in *expected_components {
                assert!(
                    entity.get_component(component_type).is_some(),
                    "{command_id} must add {component_type}"
                );
            }

            match *command_id {
                "object.create_spot_light2d" => {
                    let light = entity.get_component("Light2D").unwrap();
                    assert_eq!(light.get_string("light_type", ""), "spot");
                    assert_eq!(light.get_f64("angle", 0.0), 60.0);
                }
                "object.create_one_way_platform2d" => {
                    assert!(
                        entity
                            .get_component("Collider2D")
                            .unwrap()
                            .get_bool("one_way", false)
                    );
                }
                "object.create_audio_emitter2d" => {
                    assert_eq!(
                        entity
                            .get_component("AudioSource")
                            .unwrap()
                            .get_f64("spatial_blend", 0.0),
                        1.0
                    );
                }
                _ => {}
            }

            assert!(core.execute_command("edit.undo").unwrap().changed);
            assert_eq!(core.entity_count().unwrap(), before, "undo {command_id}");
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn advanced_gameplay_rts_starters_and_project_templates_are_native_commands() {
        let root = temp_project("advanced_authoring_commands");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        let initial_entities = core.entity_count().unwrap();
        for command_id in [
            "gameplay.spawn_unit",
            "gameplay.spawn_enemy",
            "gameplay.spawn_resource",
            "rts.spawn_base",
        ] {
            assert!(
                core.execute_command(command_id).unwrap().changed,
                "{command_id} must execute through EditorCore"
            );
        }
        assert_eq!(core.entity_count().unwrap(), initial_entities + 4);
        assert!(core.execute_command("rts.queue_worker").unwrap().changed);
        assert!(core.execute_command("rts.place_barracks").unwrap().changed);

        let before_starter = core.entity_count().unwrap();
        assert!(
            core.execute_command("scene.starter_topdown")
                .unwrap()
                .changed
        );
        assert!(core.entity_count().unwrap() > before_starter);
        assert!(core.execute_command("edit.undo").unwrap().changed);
        assert_eq!(core.entity_count().unwrap(), before_starter);

        assert!(
            core.execute_command("project.template_actionrpg")
                .unwrap()
                .changed
        );
        assert!(root.join("saves/scenes/ActionRPG_Level.scene").is_file());
        assert!(root.join("assets/prefabs/Player.prefab").is_file());
        assert!(
            root.join("scripts/visual_graphs/PlayerCombat.mfgraph")
                .is_file()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn qt_hierarchy_actions_preserve_a_valid_scene_tree() {
        let root = temp_project("hierarchy_actions");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        let parent_id = core.entity_at(0).unwrap().id;
        let child_id = core
            .entity_action(parent_id, "duplicate", "{}")
            .unwrap()
            .expect("duplicate returns its stable id");
        core.entity_action(parent_id, "rename", r#"{"name":"RootNode"}"#)
            .unwrap();
        core.entity_action(
            child_id,
            "reparent",
            &format!(r#"{{"parent_id":{parent_id}}}"#),
        )
        .unwrap();

        assert!(
            core.entity_action(
                parent_id,
                "reparent",
                &format!(r#"{{"parent_id":{child_id}}}"#),
            )
            .is_err(),
            "reparenting a root below its descendant must reject the cycle"
        );
        core.entity_action(parent_id, "delete", "{}").unwrap();
        assert_eq!(core.entity_row(child_id).unwrap().parent_id, None);
        assert!(core.game().unwrap().runtime_world.validate().is_valid());

        let state = core.scene_state().unwrap();
        assert!(state.dirty);
        assert_eq!(state.entity_count, core.entity_count().unwrap());
        assert!(!core.component_catalog().unwrap().is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn viewport_batch_transform_and_common_inspector_edit_are_single_undo_steps() {
        let root = temp_project("viewport_batch_edit");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        let first = core.entity_at(0).unwrap().id;
        let second = core
            .entity_action(first, "duplicate", "{}")
            .unwrap()
            .unwrap();
        core.update_selection(first, "replace").unwrap();
        core.update_selection(second, "add").unwrap();
        let first_before = core.entity_row(first).unwrap();
        let second_before = core.entity_row(second).unwrap();

        assert_eq!(
            core.transform_selection_json(
                r#"{"mode":"delta","dx":2.5,"dy":-1.0,"rotation_delta":15.0,"scale_x_factor":1.5,"scale_y_factor":1.5}"#,
            )
            .unwrap(),
            2
        );
        assert_eq!(core.entity_row(first).unwrap().x, first_before.x + 2.5);
        assert_eq!(core.entity_row(second).unwrap().y, second_before.y - 1.0);
        assert!(
            core.viewport_state(640, 360).unwrap()["pixels_per_unit"]
                .as_f64()
                .unwrap()
                > 0.0
        );
        assert!(core.execute_command("edit.undo").unwrap().changed);
        assert_eq!(core.entity_row(first).unwrap().x, first_before.x);
        assert_eq!(core.entity_row(second).unwrap().y, second_before.y);

        core.update_selection(first, "replace").unwrap();
        core.update_selection(second, "add").unwrap();
        assert_eq!(
            core.edit_selected_inspector_value_json("Identity", "tag", r#""BatchEdited""#)
                .unwrap(),
            2
        );
        let common_tags = [first, second]
            .into_iter()
            .map(|id| {
                core.inspector_fields(id)
                    .unwrap()
                    .into_iter()
                    .find(|field| field.target == "Identity" && field.key == "tag")
                    .unwrap()
                    .value_json
            })
            .collect::<Vec<_>>();
        assert_eq!(common_tags, vec![r#""BatchEdited""#; 2]);
        assert!(core.execute_command("edit.undo").unwrap().changed);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selection_arrange_group_and_layer_commands_are_undoable() {
        let root = temp_project("selection_arrange");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        let first = core.entity_at(0).unwrap().id;
        let second = core
            .entity_action(first, "duplicate", "{}")
            .unwrap()
            .unwrap();
        core.edit_inspector_value_json(second, "Transform", "x", "5.0")
            .unwrap();
        core.update_selection(first, "replace").unwrap();
        core.update_selection(second, "add").unwrap();

        assert!(
            core.execute_command("selection.align_left")
                .unwrap()
                .changed
        );
        let first_left = {
            let entity = core.game().unwrap().get_entity_by_id(first).unwrap();
            entity.x - entity.width * entity.scale_x.abs() * 0.5
        };
        let second_left = {
            let entity = core.game().unwrap().get_entity_by_id(second).unwrap();
            entity.x - entity.width * entity.scale_x.abs() * 0.5
        };
        assert!((first_left - second_left).abs() < f64::EPSILON);
        assert!(core.execute_command("edit.undo").unwrap().changed);

        core.update_selection(first, "replace").unwrap();
        core.update_selection(second, "add").unwrap();
        assert!(core.execute_command("selection.group").unwrap().changed);
        let group = core
            .game()
            .unwrap()
            .get_entity_by_id(first)
            .unwrap()
            .editor_group
            .clone();
        assert!(group.is_some());
        assert_eq!(
            core.game()
                .unwrap()
                .get_entity_by_id(second)
                .unwrap()
                .editor_group,
            group
        );
        assert!(core.execute_command("selection.ungroup").unwrap().changed);
        assert!(
            core.game()
                .unwrap()
                .get_entity_by_id(first)
                .unwrap()
                .editor_group
                .is_none()
        );

        let original_layer = core.entity_row(first).unwrap().layer;
        assert!(
            core.execute_command("selection.cycle_layer")
                .unwrap()
                .changed
        );
        assert_ne!(core.entity_row(first).unwrap().layer, original_layer);
        assert!(core.execute_command("edit.undo").unwrap().changed);
        assert_eq!(core.entity_row(first).unwrap().layer, original_layer);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn command_descriptors_track_selection_history_and_play_state() {
        let root = temp_project("command_availability");
        let mut core = EditorCore::new();
        assert!(core.command_cache.iter().all(|command| !command.enabled));
        core.open_project(&root).unwrap();

        let enabled = |core: &EditorCore, id: &str| {
            core.command_cache
                .iter()
                .find(|command| command.id == id)
                .unwrap()
                .enabled
        };
        assert!(enabled(&core, "play.enter"));
        assert!(!enabled(&core, "play.stop"));
        assert!(!enabled(&core, "edit.undo"));
        assert!(!enabled(&core, "selection.group"));
        assert!(core.execute_command("selection.group").is_err());

        let first = core.entity_at(0).unwrap().id;
        let second = core
            .entity_action(first, "duplicate", "{}")
            .unwrap()
            .unwrap();
        core.update_selection(first, "add").unwrap();
        assert!(enabled(&core, "selection.group"));
        assert!(enabled(&core, "edit.undo"));
        assert!(core.execute_command("selection.group").unwrap().changed);

        core.execute_command("play.enter").unwrap();
        assert!(!enabled(&core, "play.enter"));
        assert!(enabled(&core, "play.stop"));
        assert!(core.entity_row(second).is_ok());

        let scene_save = core
            .command_cache
            .iter()
            .find(|command| command.id == "scene.save")
            .unwrap();
        assert_eq!(scene_save.shortcut.as_deref(), Some("Cmd/Ctrl+S"));

        drop(core);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scene_browser_drives_create_duplicate_additive_and_stack_lifecycle() {
        let root = temp_project("scene_browser");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        assert!(
            core.scene_browser_action("pop", "{}").is_err(),
            "Pop must not report success when there is no previous scene"
        );

        let created = core
            .scene_browser_action("new", r#"{"name":"LevelA"}"#)
            .unwrap();
        assert_eq!(created["current"], "LevelA.scene");
        let duplicated = core
            .scene_browser_action("duplicate", r#"{"name":"LevelB"}"#)
            .unwrap();
        assert_eq!(duplicated["current"], "LevelB.scene");
        assert!(duplicated["scenes"].as_array().unwrap().len() >= 2);

        core.scene_browser_action("load", r#"{"name":"LevelA"}"#)
            .unwrap();
        let additive = core
            .scene_browser_action("additive", r#"{"name":"LevelB"}"#)
            .unwrap();
        assert_eq!(additive["loaded"].as_array().unwrap().len(), 2);
        let pushed = core
            .scene_browser_action("push", r#"{"name":"LevelB"}"#)
            .unwrap();
        assert!(pushed["stack"].as_array().unwrap().len() >= 2);
        let popped = core.scene_browser_action("pop", "{}").unwrap();
        assert_eq!(popped["current"], "LevelA.scene");
        core.scene_browser_action("restart", "{}").unwrap();

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn advanced_sprite_transforms_share_history_and_build_animation_timeline() {
        let root = temp_project("advanced_sprite");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        core.sprite_new_canvas(4, 3).unwrap();
        core.sprite_begin_edit().unwrap();
        assert!(
            core.sprite_set_pixel(
                1,
                1,
                SpriteColor {
                    r: 255,
                    g: 80,
                    b: 40,
                    a: 255,
                },
            )
            .unwrap()
        );
        assert!(core.sprite_commit_edit().unwrap());
        let painted = core.sprite_snapshot().unwrap();

        assert!(core.sprite_transform("flip_horizontal", "{}").unwrap());
        assert!(core.sprite_undo().unwrap());
        assert_eq!(core.sprite_snapshot().unwrap().rgba, painted.rgba);
        assert!(
            core.sprite_transform(
                "outline",
                r#"{"thickness":1,"color":{"r":0,"g":0,"b":0,"a":255}}"#,
            )
            .unwrap()
        );
        assert!(
            core.sprite_transform("crop_to_content", r#"{"padding":0}"#)
                .unwrap()
        );
        let cropped = core.sprite_snapshot().unwrap();
        assert_eq!((cropped.width, cropped.height), (3, 3));

        let _ = core.sprite_transform("rotate_right", "{}").unwrap();
        assert_eq!(core.sprite_snapshot().unwrap().width, 3);
        let timeline = core.sprite_animation_clip(1, 1, 12.0).unwrap();
        assert_eq!(timeline["timeline"]["frame_count"], 9);
        assert!(
            timeline["timeline"]["warnings"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(core.sprite_transform("unknown", "{}").is_err());

        core.sprite_new_canvas(4, 4).unwrap();
        assert!(
            core.sprite_transform(
                "bucket_fill",
                r#"{"x":0,"y":0,"color":{"r":20,"g":40,"b":60,"a":255}}"#,
            )
            .unwrap()
        );
        assert_eq!(
            core.sprite_snapshot().unwrap().rgba[0..4],
            [20, 40, 60, 255]
        );
        assert!(
            core.sprite_transform(
                "replace_color",
                r#"{"from":{"r":20,"g":40,"b":60,"a":255},"to":{"r":200,"g":180,"b":160,"a":255}}"#,
            )
            .unwrap()
        );
        assert_eq!(
            core.sprite_snapshot().unwrap().rgba[0..4],
            [200, 180, 160, 255]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prefab_studio_round_trips_instances_variants_and_overrides() {
        let root = temp_project("prefab_studio");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        let source_id = core.entity_at(0).unwrap().id;
        core.select_entity(source_id).unwrap();

        let created = core.prefab_action("create_from_selected", "{}").unwrap();
        let relative_path = created.path.expect("prefab path");
        assert!(root.join(&relative_path).is_file());
        assert!(
            core.prefab_studio_state()
                .unwrap()
                .selected_instance
                .unwrap()
                .can_apply
        );

        core.entity_action(source_id, "rename", r#"{"name":"EditedInstance"}"#)
            .unwrap();
        assert!(core.prefab_action("apply_overrides", "{}").unwrap().changed);
        assert!(core.prefab_action("create_variant", "{}").unwrap().changed);
        assert!(
            core.prefab_action("revert_overrides", "{}")
                .unwrap()
                .changed
        );
        assert!(core.prefab_action("detach", "{}").unwrap().changed);
        assert!(core.execute_command("edit.undo").unwrap().changed);
        core.select_entity(source_id).unwrap();
        assert!(
            core.prefab_studio_state()
                .unwrap()
                .selected_instance
                .unwrap()
                .can_apply,
            "undo restores the prefab instance snapshot"
        );

        let instantiated = core
            .prefab_action(
                "instantiate",
                &format!(r#"{{"relative_path":"{relative_path}","x":4,"y":5}}"#),
            )
            .unwrap();
        assert!(instantiated.entity_id.is_some());
        assert!(core.game().unwrap().runtime_world.validate().is_valid());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn editor_core_creates_sprite_luau_and_object_workflow_assets() {
        let root = temp_project("workflow_assets");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        assert!(
            core.execute_command("sprite.new_pixel_art")
                .unwrap()
                .changed
        );
        assert!(
            core.execute_command("sprite.export_frames")
                .unwrap()
                .changed
        );
        assert!(
            core.execute_command("sprite.export_atlas_pages")
                .unwrap()
                .changed
        );
        assert!(
            core.execute_command("render.write_2d_profile")
                .unwrap()
                .changed
        );
        assert!(core.execute_command("luau.new_controller").unwrap().changed);
        assert!(
            core.execute_command("object.create_sprite_actor")
                .unwrap()
                .changed
        );
        assert!(root.join("assets/sprites/PixelSprite.png").exists());
        assert!(
            root.join("assets/animations/SpriteFrames.spriteframes")
                .exists()
        );
        assert!(
            root.join("assets/atlases/ProjectSprites.spriteatlas.json")
                .exists()
        );
        assert!(
            root.join("project/reports/render_2d_backend_profile.json")
                .exists()
        );
        assert!(root.join("scripts/PlayerController.luau").exists());
        assert!(
            core.command_count()
                > default_command_descriptors()
                    .iter()
                    .filter(|command| command.category == "Project")
                    .count()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn luau_document_bridge_lists_validates_reads_and_saves_atomically() {
        let root = temp_project("luau_documents");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        let path = "scripts/LiveController.luau";
        let source = "local speed = 120\nfunction on_update(dt)\n    move(speed * dt, 0)\nend\n";
        assert!(core.validate_luau_source(path, source).unwrap().valid);
        core.save_luau_script(path, source).unwrap();
        assert_eq!(core.read_luau_script(path).unwrap(), source);

        let scripts = core.luau_scripts().unwrap();
        let script = scripts
            .iter()
            .find(|script| script.relative_path == path)
            .expect("saved script should be listed");
        assert!(script.valid);
        assert_eq!(script.bytes, source.len() as u64);

        let invalid = "function on_update(\n";
        let invalid_result = core.validate_luau_source(path, invalid).unwrap();
        assert!(!invalid_result.valid);
        assert!(invalid_result.line.is_some());
        assert!(!invalid_result.diagnostic.unwrap_or_default().is_empty());
        assert!(core.save_luau_script(path, invalid).is_err());
        assert_eq!(core.read_luau_script(path).unwrap(), source);
        assert!(
            core.read_luau_script("../outside.luau").is_err(),
            "script paths must not escape scripts/"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn visual_graph_bridge_migrates_saves_and_confines_documents() {
        let root = temp_project("visual_graph_documents");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        let path = "scripts/visual_graphs/Patrol.mfgraph";
        let legacy = r#"{
            "version": "0.9.3",
            "kind": "MiniForgeVisualGraph",
            "name": "Patrol",
            "nodes": [{"id":"start","type":"event.ready","next":"move"}]
        }"#;
        let validation = core.validate_visual_graph_source(path, legacy).unwrap();
        assert_eq!(validation["valid"], true);
        assert_eq!(validation["changed"], true);
        assert_eq!(validation["node_count"], 1);

        core.save_visual_graph(path, legacy).unwrap();
        let saved: Value = serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap();
        assert_eq!(saved["format"], "miniforge.visual-graph");
        assert_eq!(saved["schema_version"], 1);
        assert_eq!(saved["name"], "Patrol");

        assert!(
            core.validate_visual_graph_source("scripts/Patrol.mfgraph", legacy)
                .is_err(),
            "graphs must stay inside scripts/visual_graphs/"
        );
        assert!(
            core.save_visual_graph("scripts/visual_graphs/../escape.mfgraph", legacy)
                .is_err(),
            "graph paths must reject traversal components"
        );
        assert!(
            core.validate_visual_graph_source(path, r#"{"schema_version":99}"#)
                .is_err(),
            "future schema versions must be rejected"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn python_automation_applies_selection_properties_as_one_undo_step() {
        let root = temp_project("python_selection_properties");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        core.install_python_tools().unwrap();

        let entity_id = core.entity_at(0).unwrap().id;
        core.select_entity(entity_id).unwrap();
        core.entity_action(entity_id, "set_visible", r#"{"value":false}"#)
            .unwrap();
        assert!(
            !core
                .game()
                .unwrap()
                .get_entity_by_id(entity_id)
                .unwrap()
                .visible
        );

        let report = core.run_python_tool("bulk_properties", json!({})).unwrap();
        assert_eq!(report["applied_operations"], 2);
        assert_eq!(report["operation_reports"][0]["changed_entities"], 1);
        let entity = core.game().unwrap().get_entity_by_id(entity_id).unwrap();
        assert!(entity.visible);
        assert!(entity.enabled);
        assert!(!entity.locked);

        assert!(core.execute_command("edit.undo").unwrap().changed);
        assert!(
            !core
                .game()
                .unwrap()
                .get_entity_by_id(entity_id)
                .unwrap()
                .visible
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_export_bridge_keeps_a_structured_session_report() {
        let root = temp_project("runtime_export");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        core.export_runtime_profile("debug").unwrap();
        let report = core.last_export_report().unwrap();
        assert_eq!(report.profile, ExportProfile::Debug);
        assert!(report.output_path.starts_with(root.join("build")));
        assert!(report.manifest_path.exists());
        assert!(report.copied_files > 0);

        assert!(core.export_runtime_profile("unknown-profile").is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn command_search_reuses_palette_fuzzy_matching() {
        let mut core = EditorCore::new();
        let results = core.search_commands("svpr");
        assert_eq!(
            results.first().map(|command| command.id.as_str()),
            Some("project.save")
        );
    }

    #[test]
    fn native_project_settings_round_trip_engine_input_tags_and_layers() {
        let root = temp_project("native_settings");
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        let mut settings = core.project_settings().unwrap();
        settings.engine["autosave"] = Value::Bool(false);
        core.save_engine_settings_json(&settings.engine.to_string())
            .unwrap();

        settings.input["bindings"]["Dash"] = serde_json::json!(["shift", "gamepad:east"]);
        settings.input["actions"]["Dash"] = serde_json::json!({
            "display_name": "Dash",
            "category": "Gameplay",
            "devices": ["keyboard", "gamepad"],
            "description": "Short movement burst"
        });
        core.save_input_map_json(&settings.input.to_string())
            .unwrap();
        core.save_tags_layers_json(
            &serde_json::json!({
                "tags": ["Untagged", "Boss", "Interactable", "Boss"],
                "layers": ["Default", "Gameplay", "Navigation"]
            })
            .to_string(),
        )
        .unwrap();
        assert!(root.join("engine_config.json.bak").exists());
        assert!(root.join("settings/input_map.json").exists());
        assert!(root.join("settings/tags.json").exists());
        assert!(root.join("settings/layers.json").exists());

        let mut reopened = EditorCore::new();
        reopened.open_project(&root).unwrap();
        let persisted = reopened.project_settings().unwrap();
        assert_eq!(persisted.engine["autosave"], Value::Bool(false));
        assert_eq!(persisted.input["bindings"]["Dash"][0], "gamepad:east");
        assert!(persisted.tags.contains(&"Boss".to_string()));
        assert_eq!(
            persisted.tags.iter().filter(|tag| *tag == "Boss").count(),
            1
        );
        assert!(persisted.layers.contains(&"Navigation".to_string()));
        assert!(
            reopened
                .save_tags_layers_json(r#"{"tags":["../bad"],"layers":["Default"]}"#)
                .is_err()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn project_launcher_creates_discovers_and_repairs_templates() {
        let workspace = temp_project("native_launcher");
        fs::create_dir_all(&workspace).unwrap();
        let core = EditorCore::new();
        let project = core
            .launcher_create_project(&workspace, &workspace, "QtProject", "TopDown")
            .unwrap();
        assert!(project.join("project.json").exists());

        let snapshot = core.launcher_snapshot(&workspace).unwrap();
        assert!(snapshot.templates.contains(&"TopDown".to_string()));
        assert!(
            snapshot
                .recent_projects
                .iter()
                .any(|recent| Path::new(recent) == project)
        );
        let repair = core.launcher_repair_project(&workspace, &project).unwrap();
        assert!(
            repair["notes"]
                .as_array()
                .is_some_and(|notes| !notes.is_empty())
        );
        assert!(
            core.launcher_create_project(&workspace, &workspace, "Bad", "unknown")
                .is_err()
        );

        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn project_operations_cover_packages_autosave_session_and_external_launch() {
        let root = temp_project("project_operations");
        let imports = temp_project("project_operations_imports");
        fs::create_dir_all(&imports).unwrap();
        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();

        core.project_operation(
            "autosave_configure",
            r#"{"enabled":true,"interval_seconds":15}"#,
        )
        .unwrap();
        let configured = core.project_operations().unwrap();
        assert!(configured.autosave.enabled);
        assert_eq!(configured.autosave.interval_seconds, 15);

        let entity_id = core.entity_at(0).unwrap().id;
        let saved_x = core.entity_row(entity_id).unwrap().x;
        core.project_operation("autosave_now", "{}").unwrap();
        assert!(core.project_operations().unwrap().autosave.exists);
        core.game_mut()
            .unwrap()
            .get_entity_by_id_mut(entity_id)
            .unwrap()
            .x = saved_x + 100.0;
        core.project_operation("autosave_recover", "{}").unwrap();
        assert_eq!(core.entity_row(entity_id).unwrap().x, saved_x);

        core.project_operation("session_checkpoint", "{}").unwrap();
        assert!(core.project_operations().unwrap().session.pending);
        core.project_operation("session_restore", "{}").unwrap();
        core.project_operation("session_clear", "{}").unwrap();
        assert!(!core.project_operations().unwrap().session.pending);

        core.project_operation("package_export", "{}").unwrap();
        let package = core.project_operations().unwrap().last_operation.unwrap();
        let archive = PathBuf::from(package.artifact_path.unwrap());
        assert!(archive.is_file());
        assert!(package.files > 0);
        core.project_operation(
            "package_import",
            &json!({"archive_path": archive, "destination_root": imports}).to_string(),
        )
        .unwrap();
        let imported = PathBuf::from(
            core.project_operations()
                .unwrap()
                .last_operation
                .unwrap()
                .artifact_path
                .unwrap(),
        );
        assert!(imported.join("project.json").is_file());
        assert!(!imported.join(".miniforge").exists());
        assert!(!imported.join("saves/autosave").exists());

        core.project_operation("prepare_wgpu_preview", "{}")
            .unwrap();
        let preview = core.project_operations().unwrap().external_launch.unwrap();
        assert_eq!(preview.kind, "wgpu-preview");
        assert_eq!(preview.profile, "development");
        assert_eq!(preview.arguments, vec![root.to_string_lossy().to_string()]);
        assert_eq!(preview.ready, preview.executable.is_some());

        core.project_operation("prepare_external_play", r#"{"profile":"debug"}"#)
            .unwrap();
        let play = core.project_operations().unwrap().external_launch.unwrap();
        assert_eq!(play.kind, "play");
        assert_eq!(play.arguments[0], "--build");
        assert!(Path::new(&play.artifact_path).is_dir());

        core.project_operation(
            "prepare_external_build",
            r#"{"profile":"release","label":"qa_game"}"#,
        )
        .unwrap();
        let build = core.project_operations().unwrap().external_launch.unwrap();
        assert_eq!(build.kind, "build");
        assert_eq!(build.profile, "release");
        assert!(
            Path::new(&build.artifact_path)
                .join("runtime_manifest.json")
                .is_file()
        );
        assert!(
            core.project_operation("package_import", r#"{"archive_path":"../bad"}"#)
                .is_err()
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(imports);
    }

    #[test]
    fn native_open_options_enable_observable_safe_mode() {
        let root = temp_project("safe_open_options");
        {
            let mut core = EditorCore::new();
            core.open_project_with_options(
                &root,
                EditorOpenOptions {
                    safe_mode: true,
                    safe_mode_reason: "Qt CLI recovery".to_string(),
                    disable_asset_importers: true,
                },
            )
            .unwrap();

            let health = core.runtime_health().unwrap();
            assert!(health.safe_mode_active);
            assert_eq!(health.safe_mode_reason, "Qt CLI recovery");
            assert!(
                health
                    .safe_mode_disabled_systems
                    .contains(&"scripts".to_string())
            );
            assert!(
                health
                    .safe_mode_disabled_systems
                    .contains(&"asset_importers".to_string())
            );
            assert!(
                core.open_project_with_options(
                    &root,
                    EditorOpenOptions {
                        safe_mode: false,
                        safe_mode_reason: String::new(),
                        disable_asset_importers: true,
                    },
                )
                .is_err()
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_pulse_runs_due_autosave_and_session_recovery() {
        let root = temp_project("periodic_editor_safety");
        {
            let mut core = EditorCore::new();
            core.open_project(&root).unwrap();
            core.game_mut().unwrap().autosave_manager.interval = Duration::ZERO;
            core.session_recovery.as_mut().unwrap().interval = Duration::ZERO;

            let entity_id = core.entity_at(0).unwrap().id;
            core.edit_inspector_value_json(entity_id, "Transform", "x", "42")
                .unwrap();
            core.refresh().unwrap();

            let operations = core.project_operations().unwrap();
            assert!(operations.autosave.exists);
            assert!(operations.session.pending);
            assert!(operations.session.scene_dirty);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn asset_management_round_trips_import_sidecars_move_duplicate_and_trash() {
        let root = temp_project("asset_management");
        let external_root = temp_project("asset_management_external");
        fs::create_dir_all(&external_root).unwrap();
        let external = external_root.join("hero.png");
        fs::write(&external, b"fake-png-content").unwrap();

        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        let imported = core
            .manage_asset(
                "import",
                &json!({
                    "source_external": external,
                    "target_folder": "assets/sprites"
                })
                .to_string(),
            )
            .unwrap();
        let imported_path = imported.destination.unwrap();
        assert!(root.join(&imported_path).is_file());
        assert_eq!(imported.sidecars.len(), 1);
        assert!(root.join(&imported.sidecars[0]).is_file());
        assert!(
            core.asset_cache
                .iter()
                .any(|asset| asset.relative_path == imported_path),
            "the operation refreshes the live Content Browser cache"
        );

        let renamed = core
            .manage_asset(
                "rename",
                &json!({"source": imported_path, "new_name": "HeroRenamed"}).to_string(),
            )
            .unwrap();
        let renamed_path = renamed.destination.unwrap();
        assert_eq!(renamed_path, "assets/sprites/HeroRenamed.png");
        assert!(
            root.join("assets/sprites/HeroRenamed.png.import.json")
                .is_file()
        );
        fs::write(
            root.join("assets/sprites/HeroRenamed.spritesheet.json"),
            br#"{"slices":[]}"#,
        )
        .unwrap();
        let renamed_guid = core
            .asset_cache
            .iter()
            .find(|asset| asset.relative_path == renamed_path)
            .unwrap()
            .guid
            .clone();

        let duplicated = core
            .manage_asset(
                "duplicate",
                &json!({"source": renamed_path, "target_folder": "assets/audio"}).to_string(),
            )
            .unwrap();
        let duplicate_path = duplicated.destination.unwrap();
        assert!(root.join(&duplicate_path).is_file());
        assert_eq!(duplicated.sidecars.len(), 2);
        assert_eq!(
            core.asset_cache
                .iter()
                .find(|asset| asset.relative_path == renamed_path)
                .unwrap()
                .guid,
            renamed_guid,
            "duplicating must not move the original persistent identity"
        );

        let moved = core
            .manage_asset(
                "move",
                &json!({"source": duplicate_path, "target_folder": "assets/data"}).to_string(),
            )
            .unwrap();
        let moved_path = moved.destination.unwrap();
        assert!(root.join(&moved_path).is_file());
        fs::write(
            root.join(format!("{moved_path}.import.json")),
            json!({"asset": moved_path}).to_string(),
        )
        .unwrap();
        core.rebuild_asset_dependencies().unwrap();
        assert!(
            core.asset_dependency_graph()
                .unwrap()
                .edges
                .iter()
                .any(|edge| edge.dependency == moved_path),
            "the graph sees a sidecar reference, while trash still treats it as part of the bundle"
        );
        assert!(
            core.manage_asset("delete", &json!({"source": moved_path}).to_string())
                .is_err(),
            "trash requires explicit confirmation"
        );
        let trashed = core
            .manage_asset(
                "delete",
                &json!({"source": moved_path, "confirm": true}).to_string(),
            )
            .unwrap();
        assert!(!root.join(&moved_path).exists());
        assert!(root.join(trashed.destination.unwrap()).is_file());
        assert!(
            core.manage_asset(
                "rename",
                r#"{"source":"../outside.png","new_name":"bad.png"}"#
            )
            .is_err()
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(external_root);
    }

    #[test]
    fn asset_rename_preserves_compound_extensions() {
        assert_eq!(
            renamed_asset_filename(Path::new("assets/hero.sprite.json"), "Boss").unwrap(),
            "Boss.sprite.json"
        );
        assert!(renamed_asset_filename(Path::new("assets/hero.sprite.json"), "Boss.png").is_err());
        assert_eq!(
            renamed_asset_filename(Path::new("assets/world.tilemap2d.json"), "Dungeon").unwrap(),
            "Dungeon.tilemap2d.json"
        );
        assert_eq!(
            renamed_asset_filename(Path::new("assets/cue.sound.json"), "Victory").unwrap(),
            "Victory.sound.json"
        );
        let folder = temp_project("compound_duplicate");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("hero.sprite.json"), b"{}").unwrap();
        assert_eq!(
            unique_managed_asset_path(&folder, "hero.sprite.json"),
            folder.join("hero_1.sprite.json")
        );
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn profiler_and_dependency_graph_expose_sorted_real_engine_data() {
        let root = temp_project("profiler_dependencies");
        AssetTools::ensure_project_folders(&root).unwrap();
        let data = root.join("assets/data");
        fs::write(
            data.join("a.json"),
            br#"{"dependency":"assets/data/b.json"}"#,
        )
        .unwrap();
        fs::write(
            data.join("b.json"),
            br#"{"dependency":"assets/data/a.json"}"#,
        )
        .unwrap();

        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        {
            let game = core.game_mut().unwrap();
            game.profiler.frame_time = std::time::Duration::from_millis(20);
            game.profiler.record_system("Physics", 11.0);
            game.profiler.record_system("Render", 6.0);
            game.profiler.set_metric("spatial_cells", 24.0);
            game.profiler.set_counter("entities", 32);
        }
        let profiler = core.profiler_snapshot().unwrap();
        assert_eq!(profiler.slowest_system.as_deref(), Some("Physics"));
        assert_eq!(profiler.systems[0].name, "Physics");
        assert_eq!(profiler.metrics["spatial_cells"], 24.0);
        assert_eq!(profiler.counters["entities"], 32);
        assert!(profiler.over_budget);
        assert!(profiler.budget_usage_percent > 100.0);

        core.rebuild_asset_dependencies().unwrap();
        let graph = core.asset_dependency_graph().unwrap();
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.path == "assets/data/a.json")
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.dependency == "assets/data/b.json"
                && edge.consumer == "assets/data/a.json"
                && edge.resolved
        }));
        assert!(graph.cycles.iter().any(|cycle| {
            cycle.contains(&"assets/data/a.json".to_string())
                && cycle.contains(&"assets/data/b.json".to_string())
        }));
        assert_eq!(
            graph.edge_count,
            graph.edges.iter().filter(|edge| edge.resolved).count()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ready_made_system_bundles_only_use_registered_components_and_apply_genre_defaults() {
        for bundle in [
            "topdown_player",
            "platformer_player",
            "action_rpg_hero",
            "enemy_ai",
            "dialogue_npc",
            "collectible",
            "camera_rig",
            "audio_emitter",
            "survival_actor",
            "inventory",
            "combat_actor",
            "loot_container",
            "harvestable",
            "crafting_station",
        ] {
            let (_, component_types) = component_bundle_types(bundle).expect("known bundle");
            assert!(!component_types.is_empty(), "{bundle}");
            for component_type in component_types {
                assert!(
                    default_component(component_type).is_some(),
                    "{bundle} references unregistered {component_type}"
                );
            }
        }

        let mut topdown_body = default_component("Rigidbody2D").unwrap();
        configure_bundle_component("topdown_player", &mut topdown_body);
        assert!(!topdown_body.get_bool("use_gravity", true));
        assert!(topdown_body.get_bool("freeze_rotation", false));

        let mut platformer_pawn = default_component("Pawn2D").unwrap();
        configure_bundle_component("platformer_player", &mut platformer_pawn);
        assert_eq!(
            platformer_pawn.get_string("movement_mode", ""),
            "platformer"
        );

        let mut camera = default_component("Camera2D").unwrap();
        configure_bundle_component("camera_rig", &mut camera);
        assert!(camera.get_bool("active", false));
        assert!(camera.get_bool("pixel_perfect", false));

        let mut audio = default_component("AudioSource2D").unwrap();
        configure_bundle_component("audio_emitter", &mut audio);
        assert_eq!(audio.get_f64("spatial_blend", 0.0), 1.0);
    }

    #[test]
    fn inspector_quick_actions_and_selection_batches_are_real_and_atomic() {
        let root = temp_project("inspector_quick_batch");
        AssetTools::ensure_project_folders(&root).unwrap();
        fs::write(
            root.join("scripts/InspectorAction.luau"),
            "return { update = function(self, dt) end }\n",
        )
        .unwrap();

        let mut core = EditorCore::new();
        core.open_project(&root).unwrap();
        let original_id = core.entity_at(0).unwrap().id;
        core.select_entity(original_id).unwrap();
        {
            let game = core.game_mut().unwrap();
            let entity = game.get_entity_by_id_mut(original_id).unwrap();
            entity.script = None;
            entity.remove_component("ScriptComponent");
            entity.sync_from_components();
            game.sync_world();
        }
        core.refresh_scene_cache();

        let actions = core.inspector_quick_actions(original_id).unwrap();
        let attach_script = actions
            .iter()
            .find(|action| action.id == "attach_script")
            .expect("attach script action");
        assert!(attach_script.enabled);
        assert!(
            attach_script
                .assets
                .iter()
                .any(|asset| asset.relative_path == "scripts/InspectorAction.luau")
        );
        core.execute_inspector_quick_action(
            original_id,
            "attach_script",
            "scripts/InspectorAction.luau",
        )
        .unwrap();
        assert_eq!(
            component_asset_path(
                core.game().unwrap().get_entity_by_id(original_id).unwrap(),
                "ScriptComponent",
                "path"
            )
            .as_deref(),
            Some("scripts/InspectorAction.luau")
        );
        let open_script = core
            .inspector_quick_actions(original_id)
            .unwrap()
            .into_iter()
            .find(|action| action.id == "open_script")
            .expect("open script action");
        assert!(open_script.enabled);
        assert_eq!(
            core.execute_inspector_quick_action(original_id, "open_script", "")
                .unwrap()
                .open_asset_path
                .as_deref(),
            Some("scripts/InspectorAction.luau")
        );
        assert!(core.execute_command("edit.undo").unwrap().changed);
        assert!(
            core.game()
                .unwrap()
                .get_entity_by_id(original_id)
                .unwrap()
                .get_component("ScriptComponent")
                .is_none()
        );

        let duplicated_id = core
            .entity_action(original_id, "duplicate", "{}")
            .unwrap()
            .unwrap();
        core.update_selection(original_id, "replace").unwrap();
        core.update_selection(duplicated_id, "add").unwrap();
        assert_eq!(
            core.selected_entity_action("add_component", r#"{"component_type":"Material2D"}"#)
                .unwrap(),
            2
        );
        for entity_id in [original_id, duplicated_id] {
            assert!(
                core.game()
                    .unwrap()
                    .get_entity_by_id(entity_id)
                    .unwrap()
                    .get_component("Material2D")
                    .is_some()
            );
        }
        assert!(core.execute_command("edit.undo").unwrap().changed);
        for entity_id in [original_id, duplicated_id] {
            assert!(
                core.game()
                    .unwrap()
                    .get_entity_by_id(entity_id)
                    .unwrap()
                    .get_component("Material2D")
                    .is_none()
            );
        }
        assert!(core.execute_command("edit.redo").unwrap().changed);
        core.update_selection(original_id, "replace").unwrap();
        core.update_selection(duplicated_id, "add").unwrap();
        assert!(
            core.selected_entity_action("add_component_bundle", r#"{"bundle":"survival_actor"}"#,)
                .unwrap()
                > 0
        );
        for entity_id in [original_id, duplicated_id] {
            let entity = core.game().unwrap().get_entity_by_id(entity_id).unwrap();
            for component in ["Health", "SurvivalNeeds", "Inventory", "CraftingBook"] {
                assert!(entity.get_component(component).is_some(), "{component}");
            }
        }
        assert!(core.execute_command("edit.undo").unwrap().changed);
        core.update_selection(original_id, "replace").unwrap();
        core.update_selection(duplicated_id, "add").unwrap();
        assert_eq!(
            core.selected_entity_action("remove_component", r#"{"component_type":"Material2D"}"#)
                .unwrap(),
            2
        );
        assert!(core.execute_command("edit.undo").unwrap().changed);
        for entity_id in [original_id, duplicated_id] {
            assert!(
                core.game()
                    .unwrap()
                    .get_entity_by_id(entity_id)
                    .unwrap()
                    .get_component("Material2D")
                    .is_some()
            );
        }

        core.update_selection(original_id, "replace").unwrap();
        core.update_selection(duplicated_id, "add").unwrap();
        let count_before_batch_duplicate = core.entity_count().unwrap();
        assert_eq!(core.selected_entity_action("duplicate", "{}").unwrap(), 2);
        assert_eq!(
            core.entity_count().unwrap(),
            count_before_batch_duplicate + 2
        );
        assert!(core.execute_command("edit.undo").unwrap().changed);
        assert_eq!(core.entity_count().unwrap(), count_before_batch_duplicate);

        core.update_selection(original_id, "replace").unwrap();
        core.update_selection(duplicated_id, "add").unwrap();
        assert_eq!(core.selected_entity_action("delete", "{}").unwrap(), 2);
        assert_eq!(
            core.entity_count().unwrap(),
            count_before_batch_duplicate - 2
        );
        assert!(core.execute_command("edit.undo").unwrap().changed);
        assert_eq!(core.entity_count().unwrap(), count_before_batch_duplicate);

        core.entity_action(
            original_id,
            "collision_vertex_add",
            r#"{"x":0.25,"y":-0.5}"#,
        )
        .unwrap();
        assert_eq!(
            EditorSpatialTools2D::collision_points(
                core.game().unwrap().get_entity_by_id(original_id).unwrap()
            )
            .last(),
            Some(&(0.25, -0.5))
        );
        assert!(core.execute_command("edit.undo").unwrap().changed);

        let _ = fs::remove_dir_all(root);
    }

    fn temp_project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("miniforge_editor_core_{name}_{stamp}"))
    }
}
