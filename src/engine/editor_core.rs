use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

use crate::core::game::Game;
use crate::engine::asset_database::AssetRecord;
use crate::engine::command_palette::CommandPalette;
use crate::engine::developer_console::ConsoleEntry;
use crate::engine::forge_ai::context::AiProjectContext;
use crate::engine::forge_ai::executor::{AiFileChange, AiHostValidation};
use crate::engine::forge_ai::testing::{AiTestReport, AiTestStatus, AiTestSuite};
use crate::engine::inspector_editor::InspectorEditor;
use crate::engine::luau_scripting::LuauScriptRuntime;
use crate::engine::project_validator::ProjectValidator;
use crate::engine::render_2d::{
    Render2DCompatibilityProfile, SpriteAtlasExportOptions2D, export_sprite_atlas_pages_from_files,
};
use crate::engine::sprite_editor::{SpriteColor, SpriteEditorCanvas};
use crate::engine::system_audit::{SystemReadinessLevel, SystemReadinessReport};
use crate::entities::game_object::GameObject;
use crate::render::backend::RenderBackendConfig;

pub const EDITOR_CORE_API_VERSION: u32 = 1;

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
pub struct ReadinessRow {
    pub system: String,
    pub level: SystemReadinessLevel,
    pub score: u8,
    pub strength_count: usize,
    pub gap_count: usize,
    pub action_count: usize,
    pub top_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewportSnapshot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
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

#[derive(Debug)]
pub struct EditorCore {
    project_path: Option<PathBuf>,
    game: Option<Game>,
    command_palette: CommandPalette,
    entity_cache: Vec<EntityRow>,
    selected_cache: Vec<u64>,
    inspector_cache: BTreeMap<u64, Vec<InspectorFieldDto>>,
    asset_cache: Vec<AssetRow>,
    command_cache: Vec<CommandDescriptor>,
    readiness_cache: Vec<ReadinessRow>,
    readiness_score: u8,
    readiness_summary: String,
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
            command_palette: CommandPalette::with_commands(commands),
            entity_cache: Vec::new(),
            selected_cache: Vec::new(),
            inspector_cache: BTreeMap::new(),
            asset_cache: Vec::new(),
            command_cache,
            readiness_cache: Vec::new(),
            readiness_score: 0,
            readiness_summary: "Readiness unavailable".to_string(),
        }
    }
}

impl EditorCore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_project(&mut self, path: impl AsRef<Path>) -> Result<(), EditorCoreError> {
        let project_path = path.as_ref().to_path_buf();
        let mut game = Game::from_project(&project_path, false)?;
        game.asset_database.scan()?;
        game.console.log(
            "Qt editor bridge opened this project through EditorCore",
            "EDITOR",
        );
        self.project_path = Some(project_path);
        self.game = Some(game);
        self.refresh_all_caches();
        Ok(())
    }

    pub fn is_project_open(&self) -> bool {
        self.game.is_some()
    }

    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
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
        let label = self
            .command_cache
            .iter()
            .find(|command| command.id == command_id)
            .map(|command| command.label.clone())
            .unwrap_or_else(|| command_id.to_string());
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
            "entity.create_empty" => {
                let id = game.spawn_game_object("GameObject", 0.0, 0.0);
                CommandOutcome {
                    changed: true,
                    message: format!("Created entity #{id}"),
                }
            }
            "object.create_sprite_actor" => {
                let id = game.spawn_game_object("SpriteActor", 0.0, 0.0);
                let _ = game.add_component_to_entity(id, "SpriteRenderer");
                let _ = game.add_component_to_entity(id, "Animator2D");
                CommandOutcome {
                    changed: true,
                    message: format!("Created SpriteActor #{id}"),
                }
            }
            "object.create_camera" => {
                let id = game.spawn_game_object("CameraRig", 0.0, 0.0);
                let _ = game.add_component_to_entity(id, "Camera2D");
                CommandOutcome {
                    changed: true,
                    message: format!("Created CameraRig #{id}"),
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

fn default_command_descriptors() -> Vec<CommandDescriptor> {
    vec![
        command(
            "project.save",
            "Save Project",
            "Project",
            Some("Cmd/Ctrl+S"),
        ),
        command("scene.save", "Save Scene", "Scene", None),
        command("edit.undo", "Undo", "Edit", Some("Cmd/Ctrl+Z")),
        command("edit.redo", "Redo", "Edit", Some("Cmd/Ctrl+Shift+Z")),
        command("entity.create_empty", "Create Empty Entity", "Entity", None),
        command(
            "object.create_sprite_actor",
            "Create Sprite Actor",
            "Objects",
            None,
        ),
        command("object.create_camera", "Create Camera Rig", "Objects", None),
        command("object.create_ui_text", "Create HUD Text", "Objects", None),
        command("project.audit", "Run Project Audit", "Project", None),
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
        enabled: true,
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
    path.strip_prefix(project_path)
        .unwrap_or(path)
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

    let tile = game.grid.tile_size.max(1) as f32;
    let world_w = (game.grid.width.max(1) as f32 * tile).max(1.0);
    let world_h = (game.grid.height.max(1) as f32 * tile).max(1.0);
    let scale = (width as f32 / world_w).min(height as f32 / world_h);
    let offset_x = (width as f32 - world_w * scale) * 0.5;
    let offset_y = (height as f32 - world_h * scale) * 0.5;

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
        let w = (entity.width as f32 * tile * scale).max(4.0);
        let h = (entity.height as f32 * tile * scale).max(4.0);
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
        draw_rect_center(&mut rgba, (width, height), (x, y), (w, h), color);
    }

    ViewportSnapshot {
        width,
        height,
        rgba,
    }
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
    fn command_search_reuses_palette_fuzzy_matching() {
        let mut core = EditorCore::new();
        let results = core.search_commands("svpr");
        assert_eq!(
            results.first().map(|command| command.id.as_str()),
            Some("project.save")
        );
    }

    fn temp_project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("miniforge_editor_core_{name}_{stamp}"))
    }
}
