use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::asset_tools::AssetTools;
use crate::engine::editor_ui::{EditorIcon, install_phosphor_fonts};
use crate::engine::engine_backend::{EngineBackend, EngineBackendPlan};
use crate::engine::manifest_builder::ManifestBuilder;
use crate::engine::project_templates::ProjectTemplates;
use crate::engine::project_validator::ProjectValidator;
use crate::engine::runtime_exporter::{ExportProfile, RuntimeExportReport, RuntimeExporter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ProjectLauncher;

impl ProjectLauncher {
    pub fn run(&self) -> PathBuf {
        AssetTools::default_project_path()
    }

    pub fn egui(workspace_root: impl AsRef<Path>) -> EguiProjectLauncher {
        EguiProjectLauncher::new(workspace_root)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherTemplate {
    Empty,
    TopDown,
    Platformer,
    Rts,
}

impl LauncherTemplate {
    pub fn all() -> [Self; 4] {
        [Self::Empty, Self::TopDown, Self::Platformer, Self::Rts]
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::TopDown => "TopDown",
            Self::Platformer => "Platformer",
            Self::Rts => "RTS",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::TopDown => "TopDown",
            Self::Platformer => "Platformer",
            Self::Rts => "RTS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherAction {
    NewProject(PathBuf),
    OpenProject(PathBuf),
    ExportGame(PathBuf),
    RepairProject(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherPatchNote {
    pub version: String,
    pub title: String,
    pub date: String,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EguiProjectLauncher {
    pub workspace_root: PathBuf,
    pub project_name: String,
    pub project_location: String,
    pub open_path: String,
    pub selected_template: LauncherTemplate,
    pub recent_projects: Vec<PathBuf>,
    pub patch_notes: Vec<LauncherPatchNote>,
    pub selected_patch_note: usize,
    pub export_profile: ExportProfile,
    pub settings: LauncherSettings,
    pub backend_plan: Option<EngineBackendPlan>,
    pub backend_summary: String,
    pub backend_actions: Vec<String>,
    pub last_repair_notes: Vec<String>,
    pub status: String,
    pub last_action: Option<LauncherAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherSettings {
    #[serde(default = "default_true")]
    pub safe_mode: bool,
    #[serde(default = "default_true")]
    pub validate_on_open: bool,
    #[serde(default = "default_true")]
    pub remember_recent: bool,
    #[serde(default = "default_true")]
    pub analyze_before_export: bool,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            safe_mode: true,
            validate_on_open: true,
            remember_recent: true,
            analyze_before_export: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LauncherDiskState {
    #[serde(default)]
    recent_projects: Vec<PathBuf>,
    #[serde(default)]
    settings: LauncherSettings,
    project_location: Option<String>,
    open_path: Option<String>,
    selected_template: Option<LauncherTemplate>,
    export_profile: Option<ExportProfile>,
}

impl Default for EguiProjectLauncher {
    fn default() -> Self {
        Self::new("projects")
    }
}

impl EguiProjectLauncher {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let mut launcher = Self {
            project_location: workspace_root.display().to_string(),
            workspace_root,
            project_name: "NewProject".to_string(),
            open_path: String::new(),
            selected_template: LauncherTemplate::TopDown,
            recent_projects: Vec::new(),
            patch_notes: default_patch_notes(),
            selected_patch_note: 0,
            export_profile: ExportProfile::Debug,
            settings: LauncherSettings::default(),
            backend_plan: None,
            backend_summary: String::new(),
            backend_actions: Vec::new(),
            last_repair_notes: Vec::new(),
            status: String::new(),
            last_action: None,
        };
        let _ = launcher.load_saved_state();
        launcher
    }

    pub fn create_new_project(&mut self) -> io::Result<PathBuf> {
        let safe = AssetTools::safe_name(&self.project_name, "NewProject");
        let root = self.create_location_root();
        fs::create_dir_all(&root)?;
        let project_path = AssetTools::unique_path(&root, &safe);
        AssetTools::ensure_project_folders(&project_path)?;
        ProjectTemplates::create(&project_path, self.selected_template.key())?;
        self.record_recent(project_path.clone());
        let _ = self.refresh_project_status(&project_path);
        self.status = format!("Proyecto creado: {}", project_path.display());
        self.last_action = Some(LauncherAction::NewProject(project_path.clone()));
        Ok(project_path)
    }

    pub fn open_project(&mut self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let path = path.as_ref().to_path_buf();
        AssetTools::ensure_project_folders(&path)?;
        let mut validation_status = None;
        if self.settings.validate_on_open {
            let mut validator = ProjectValidator::default();
            validator.validate(&path);
            if !validator.errors.is_empty() {
                validation_status = Some(format!(
                    "Proyecto abierto con errores: {}",
                    validator.errors.join("; ")
                ));
            }
        }
        let backend_status = self.refresh_project_status(&path).ok();
        if self.settings.remember_recent {
            self.record_recent(path.clone());
        }
        self.status = validation_status.unwrap_or_else(|| {
            backend_status.unwrap_or_else(|| format!("Proyecto abierto: {}", path.display()))
        });
        self.last_action = Some(LauncherAction::OpenProject(path.clone()));
        Ok(path)
    }

    pub fn open_typed_project(&mut self) -> io::Result<PathBuf> {
        self.open_project(self.typed_or_default_path())
    }

    pub fn typed_or_default_path(&self) -> PathBuf {
        if self.open_path.trim().is_empty() {
            AssetTools::default_project_path()
        } else {
            expand_user_path(self.open_path.trim())
        }
    }

    pub fn export_game(
        &mut self,
        project_path: impl AsRef<Path>,
    ) -> io::Result<RuntimeExportReport> {
        let project_path = project_path.as_ref();
        if self.settings.analyze_before_export {
            let _ = self.refresh_project_status(project_path);
        }
        let report = RuntimeExporter::export_with_profile(
            project_path,
            project_path.join("builds"),
            self.export_profile,
        )?;
        self.status = format!("Export listo: {}", report.output_path.display());
        self.last_action = Some(LauncherAction::ExportGame(report.output_path.clone()));
        Ok(report)
    }

    pub fn repair_project(&mut self, path: impl AsRef<Path>) -> io::Result<Vec<String>> {
        let path = path.as_ref();
        AssetTools::ensure_project_folders(path)?;
        let manifest = ManifestBuilder::build_manifest(path)?;
        let mut validator = ProjectValidator::default();
        validator.validate(path);
        let mut notes = vec![format!(
            "Manifest reconstruido con {} scripts",
            manifest
                .get("scripts")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        )];
        notes.extend(
            validator
                .warnings
                .iter()
                .map(|warning| format!("Warning: {warning}")),
        );
        notes.extend(
            validator
                .errors
                .iter()
                .map(|error| format!("Error: {error}")),
        );
        if let Ok(summary) = self.refresh_project_status(path) {
            notes.push(summary);
        }
        for action in self.backend_actions.iter().take(5) {
            notes.push(format!("Next: {action}"));
        }
        self.last_repair_notes = notes.clone();
        self.status = format!("Repair listo: {} notas", notes.len());
        self.last_action = Some(LauncherAction::RepairProject(path.to_path_buf()));
        Ok(notes)
    }

    pub fn record_recent(&mut self, path: PathBuf) {
        self.recent_projects.retain(|existing| existing != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(8);
        let _ = self.save_state();
    }

    pub fn create_location_root(&self) -> PathBuf {
        let trimmed = self.project_location.trim();
        if trimmed.is_empty() {
            self.workspace_root.clone()
        } else {
            expand_user_path(trimmed)
        }
    }

    pub fn discover_recent_projects(&mut self) -> io::Result<usize> {
        let mut discovered = Vec::new();
        for root in [self.workspace_root.clone(), self.create_location_root()] {
            if !root.exists() {
                continue;
            }
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let path = entry.path();
                if path.join("project.json").exists() {
                    discovered.push(path);
                }
            }
        }
        discovered.sort();
        discovered.dedup();
        for path in discovered {
            self.record_recent(path);
        }
        Ok(self.recent_projects.len())
    }

    pub fn refresh_typed_project_status(&mut self) -> io::Result<String> {
        self.refresh_project_status(self.typed_or_default_path())
    }

    pub fn refresh_project_status(&mut self, path: impl AsRef<Path>) -> io::Result<String> {
        let path = path.as_ref();
        let plan = EngineBackend::plan_project(path)?;
        let top_action = plan
            .system_audit
            .top_actions(1)
            .first()
            .cloned()
            .unwrap_or_else(|| "mantener cobertura y polish".to_string());
        let summary = format!(
            "{} | readiness {}% | editor={} runtime={} export={} | {} recursos | next: {}",
            plan.project_name,
            plan.system_audit.total_score,
            ready_label(plan.editor_ready),
            ready_label(plan.runtime_ready),
            ready_label(plan.export_ready),
            plan.resources.total_files,
            top_action
        );
        self.backend_actions = plan.system_audit.top_actions(8);
        self.backend_summary = summary.clone();
        self.backend_plan = Some(plan);
        self.status = summary.clone();
        Ok(summary)
    }

    pub fn launcher_state_path(&self) -> PathBuf {
        self.workspace_root.join(".miniforge_launcher.json")
    }

    pub fn load_saved_state(&mut self) -> io::Result<bool> {
        let path = self.launcher_state_path();
        if !path.exists() {
            return Ok(false);
        }
        let data = AssetTools::read_json(&path)?;
        let state: LauncherDiskState = serde_json::from_value(data).map_err(io::Error::other)?;
        self.recent_projects = state
            .recent_projects
            .into_iter()
            .filter(|path| path.join("project.json").exists())
            .take(8)
            .collect();
        self.settings = state.settings;
        if let Some(project_location) = state.project_location {
            self.project_location = project_location;
        }
        if let Some(open_path) = state.open_path {
            self.open_path = open_path;
        }
        if let Some(selected_template) = state.selected_template {
            self.selected_template = selected_template;
        }
        if let Some(export_profile) = state.export_profile {
            self.export_profile = export_profile;
        }
        Ok(true)
    }

    pub fn save_state(&self) -> io::Result<()> {
        let state = LauncherDiskState {
            recent_projects: self.recent_projects.clone(),
            settings: self.settings.clone(),
            project_location: Some(self.project_location.clone()),
            open_path: Some(self.open_path.clone()),
            selected_template: Some(self.selected_template),
            export_profile: Some(self.export_profile),
        };
        let data = serde_json::to_value(state).map_err(io::Error::other)?;
        AssetTools::write_json(self.launcher_state_path(), &data)
    }

    pub fn active_patch_note(&self) -> Option<&LauncherPatchNote> {
        self.patch_notes
            .get(self.selected_patch_note)
            .or_else(|| self.patch_notes.first())
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<LauncherAction> {
        install_phosphor_fonts(ctx);
        let mut action = None;
        egui::Window::new(EditorIcon::Scene.label("MiniForge Launcher"))
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Proyecto");
                    ui.text_edit_singleline(&mut self.project_name);
                });
                ui.horizontal(|ui| {
                    ui.label("Ubicacion");
                    ui.text_edit_singleline(&mut self.project_location);
                });
                ui.horizontal(|ui| {
                    ui.label("Template");
                    for template in LauncherTemplate::all() {
                        ui.selectable_value(
                            &mut self.selected_template,
                            template,
                            template.label(),
                        );
                    }
                });
                if ui
                    .button(EditorIcon::NewEntity.label("Nuevo proyecto"))
                    .clicked()
                {
                    match self.create_new_project() {
                        Ok(path) => action = Some(LauncherAction::NewProject(path)),
                        Err(error) => self.status = error.to_string(),
                    }
                }
                if ui
                    .button(EditorIcon::Search.label("Buscar proyectos locales"))
                    .clicked()
                {
                    match self.discover_recent_projects() {
                        Ok(count) => self.status = format!("{count} proyectos encontrados"),
                        Err(error) => self.status = error.to_string(),
                    }
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Abrir");
                    ui.text_edit_singleline(&mut self.open_path);
                });
                if ui
                    .button(EditorIcon::Open.label("Abrir proyecto"))
                    .clicked()
                {
                    match self.open_typed_project() {
                        Ok(path) => action = Some(LauncherAction::OpenProject(path)),
                        Err(error) => self.status = error.to_string(),
                    }
                }

                ui.separator();
                ui.label(EditorIcon::Warning.label("Notas del parche"));
                let notes = self.patch_notes.clone();
                for (index, note) in notes.iter().enumerate() {
                    if ui
                        .selectable_label(
                            self.selected_patch_note == index,
                            format!("{} {}", note.version, note.title),
                        )
                        .clicked()
                    {
                        self.selected_patch_note = index;
                    }
                }
                if let Some(note) = self.active_patch_note() {
                    ui.label(format!("{} - {}", note.date, note.title));
                    for highlight in note.highlights.iter().take(6) {
                        ui.label(format!("• {highlight}"));
                    }
                }

                ui.separator();
                ui.label(EditorIcon::Folder.label("Proyectos recientes"));
                let recent_projects = self.recent_projects.clone();
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::auto())
                    .column(egui_extras::Column::remainder())
                    .body(|mut body| {
                        for path in recent_projects {
                            body.row(24.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(EditorIcon::Folder.glyph());
                                });
                                row.col(|ui| {
                                    if ui
                                        .selectable_label(false, path.display().to_string())
                                        .clicked()
                                    {
                                        match self.open_project(&path) {
                                            Ok(path) => {
                                                action = Some(LauncherAction::OpenProject(path))
                                            }
                                            Err(error) => self.status = error.to_string(),
                                        }
                                    }
                                });
                            });
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Export");
                    ui.selectable_value(&mut self.export_profile, ExportProfile::Debug, "Debug");
                    ui.selectable_value(
                        &mut self.export_profile,
                        ExportProfile::Release,
                        "Release",
                    );
                });
                if ui
                    .button(EditorIcon::Save.label("Exportar juego"))
                    .clicked()
                {
                    let path = self
                        .recent_projects
                        .first()
                        .cloned()
                        .unwrap_or_else(AssetTools::default_project_path);
                    match self.export_game(&path) {
                        Ok(report) => action = Some(LauncherAction::ExportGame(report.output_path)),
                        Err(error) => self.status = error.to_string(),
                    }
                }

                if ui
                    .button(EditorIcon::Settings.label("Repair project"))
                    .clicked()
                {
                    let path = self
                        .recent_projects
                        .first()
                        .cloned()
                        .unwrap_or_else(AssetTools::default_project_path);
                    match self.repair_project(&path) {
                        Ok(_) => action = Some(LauncherAction::RepairProject(path)),
                        Err(error) => self.status = error.to_string(),
                    }
                }

                ui.separator();
                ui.checkbox(&mut self.settings.safe_mode, "Safe mode");
                ui.checkbox(&mut self.settings.validate_on_open, "Validate on open");
                ui.checkbox(&mut self.settings.remember_recent, "Remember recent");
                ui.checkbox(
                    &mut self.settings.analyze_before_export,
                    "Analyze before export",
                );

                ui.separator();
                if ui
                    .button(EditorIcon::Validate.label("Analizar backend"))
                    .clicked()
                {
                    match self.refresh_typed_project_status() {
                        Ok(summary) => self.status = summary,
                        Err(error) => self.status = error.to_string(),
                    }
                }
                if let Some(plan) = &self.backend_plan {
                    ui.label(format!(
                        "Readiness {}% | Editor {} | Runtime {} | Export {}",
                        plan.system_audit.total_score,
                        ready_label(plan.editor_ready),
                        ready_label(plan.runtime_ready),
                        ready_label(plan.export_ready),
                    ));
                    for action in self.backend_actions.iter().take(4) {
                        ui.label(format!("Next: {action}"));
                    }
                }

                if !self.status.is_empty() {
                    ui.separator();
                    ui.label(&self.status);
                }
            });
        if action.is_some() {
            self.last_action = action.clone();
        }
        action
    }
}

fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

fn ready_label(value: bool) -> &'static str {
    if value { "OK" } else { "WATCH" }
}

fn default_true() -> bool {
    true
}

fn default_patch_notes() -> Vec<LauncherPatchNote> {
    vec![
        LauncherPatchNote {
            version: crate::engine::version::ENGINE_VERSION.to_string(),
            title: "2D Workflow Foundations".to_string(),
            date: "2026-07-06".to_string(),
            highlights: vec![
                "Launcher finalizado para uso diario: arranque por defecto, flag --launcher y overlay desde File > Open Launcher."
                    .to_string(),
                "Top bar del editor mas minimalista, con acciones secundarias movidas a menus y Command Palette."
                    .to_string(),
                "System audit 0.9.3.4: Project, Assets, Scenes, Scripting, UI, Gameplay, Physics, Audio, Rendering, Input, Packaging, Plugins, Runtime y Editor con score y backlog."
                    .to_string(),
                "Profiler conectado al frontend con salud de frame, fixed timestep, tiempo descartado, sistema mas lento y acciones de rendimiento."
                    .to_string(),
                "Scheduler, runtime runner, input, render y pathfinding ahora emiten reportes utiles para debug y proxima optimizacion."
                    .to_string(),
                "Packaging standalone con runtime autodetectado y launchers run_game para jugar sin codigo fuente."
                    .to_string(),
                "UI ScreenManager con pantallas estandar de juego y comandos runtime mas fiables."
                    .to_string(),
                "Blueprints y visual graphs conservan el flujo 0.9.2, ahora con auditoria y manifest 0.9.3.4 para saber que falta antes de exportar."
                    .to_string(),
                "Manifests y exports incluyen readiness_score, acciones de siguiente pasada y matriz de capacidades 0.9.3.4."
                    .to_string(),
            ],
        },
        LauncherPatchNote {
            version: "0.9.1.1".to_string(),
            title: "Interface Overhaul Patch".to_string(),
            date: "2026-05-25".to_string(),
            highlights: vec![
                "Interfaz redisenada con superficies unificadas, sombras suaves, gradientes y acentos modernos."
                    .to_string(),
                "Launcher oscuro estilo mac con panel principal de vidrio, fondo profundo y notas de parche mas limpias."
                    .to_string(),
                "Hierarchy, Inspector, Browser, Graph editor y Code editor usan paneles conectados al estado real del motor."
                    .to_string(),
            ],
        },
        LauncherPatchNote {
            version: "0.9.2".to_string(),
            title: "Game Creation API Update".to_string(),
            date: "2026-05-25".to_string(),
            highlights: vec![
                "Blueprints 0.9.2 con nodos de inventario, economia, quests, habilidades y produccion RTS."
                    .to_string(),
                "GameAPI ampliada para sistemas complejos: transferencias, costs, wallets, recetas, gathering y quest progress."
                    .to_string(),
                "Paleta de comandos con busqueda difusa y acciones directas para crear/adjuntar graphs jugables."
                    .to_string(),
                "Templates nuevos: InventoryEconomyLoop, QuestAbilityLoop y RTSProductionEconomy."
                    .to_string(),
            ],
        },
        LauncherPatchNote {
            version: "0.9.1".to_string(),
            title: "Creation Workflow Update".to_string(),
            date: "2026-05-25".to_string(),
            highlights: vec![
                "Ventanas flotantes movibles para scripts Luau, blueprints y Play Window."
                    .to_string(),
                "Jerarquia con menu contextual para borrar, mover y parentar entidades.".to_string(),
                "Escenas, sprites, prefabs, consola e import/export conectados al UI.".to_string(),
                "Conexiones de blueprints con pines exec/true/false/A/B y validacion mejorada."
                    .to_string(),
            ],
        },
        LauncherPatchNote {
            version: "0.8.0".to_string(),
            title: "Developer Stability Update".to_string(),
            date: "2026-05-16".to_string(),
            highlights: vec![
                "Flujo completo de editor, Play Mode y export runtime estabilizado.".to_string(),
                "Content Browser con busqueda, preview, dependencias y drag/drop.".to_string(),
                "Visual graphs editables como nodos conectables.".to_string(),
                "Validacion reforzada para proyecto, escenas, prefabs, Luau y graphs.".to_string(),
            ],
        },
    ]
}
