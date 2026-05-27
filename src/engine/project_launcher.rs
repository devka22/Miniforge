use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::asset_tools::AssetTools;
use crate::engine::manifest_builder::ManifestBuilder;
use crate::engine::project_templates::ProjectTemplates;
use crate::engine::project_validator::ProjectValidator;
use crate::engine::runtime_exporter::{ExportProfile, RuntimeExportReport, RuntimeExporter};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub status: String,
    pub last_action: Option<LauncherAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherSettings {
    pub safe_mode: bool,
    pub validate_on_open: bool,
    pub remember_recent: bool,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            safe_mode: true,
            validate_on_open: true,
            remember_recent: true,
        }
    }
}

impl Default for EguiProjectLauncher {
    fn default() -> Self {
        Self::new("projects")
    }
}

impl EguiProjectLauncher {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        Self {
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
            status: String::new(),
            last_action: None,
        }
    }

    pub fn create_new_project(&mut self) -> io::Result<PathBuf> {
        let safe = AssetTools::safe_name(&self.project_name, "NewProject");
        let root = self.create_location_root();
        fs::create_dir_all(&root)?;
        let project_path = AssetTools::unique_path(&root, &safe);
        AssetTools::ensure_project_folders(&project_path)?;
        ProjectTemplates::create(&project_path, self.selected_template.key())?;
        self.record_recent(project_path.clone());
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
        if self.settings.remember_recent {
            self.record_recent(path.clone());
        }
        self.status =
            validation_status.unwrap_or_else(|| format!("Proyecto abierto: {}", path.display()));
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
        self.status = format!("Repair listo: {} notas", notes.len());
        self.last_action = Some(LauncherAction::RepairProject(path.to_path_buf()));
        Ok(notes)
    }

    pub fn record_recent(&mut self, path: PathBuf) {
        self.recent_projects.retain(|existing| existing != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(8);
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

    pub fn active_patch_note(&self) -> Option<&LauncherPatchNote> {
        self.patch_notes
            .get(self.selected_patch_note)
            .or_else(|| self.patch_notes.first())
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<LauncherAction> {
        let mut action = None;
        egui::Window::new("MiniForge Launcher")
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
                if ui.button("Nuevo proyecto").clicked() {
                    match self.create_new_project() {
                        Ok(path) => action = Some(LauncherAction::NewProject(path)),
                        Err(error) => self.status = error.to_string(),
                    }
                }
                if ui.button("Buscar proyectos locales").clicked() {
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
                if ui.button("Abrir proyecto").clicked() {
                    match self.open_typed_project() {
                        Ok(path) => action = Some(LauncherAction::OpenProject(path)),
                        Err(error) => self.status = error.to_string(),
                    }
                }

                ui.separator();
                ui.label("Notas del parche");
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
                ui.label("Recientes");
                for path in self.recent_projects.clone() {
                    if ui.button(path.display().to_string()).clicked() {
                        match self.open_project(&path) {
                            Ok(path) => action = Some(LauncherAction::OpenProject(path)),
                            Err(error) => self.status = error.to_string(),
                        }
                    }
                }

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
                if ui.button("Exportar juego").clicked() {
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

                if ui.button("Repair project").clicked() {
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

fn default_patch_notes() -> Vec<LauncherPatchNote> {
    vec![
        LauncherPatchNote {
            version: crate::engine::version::ENGINE_VERSION.to_string(),
            title: "Interface Overhaul Patch".to_string(),
            date: "2026-05-25".to_string(),
            highlights: vec![
                "Interfaz redisenada con superficies unificadas, sombras suaves, gradientes y acentos modernos."
                    .to_string(),
                "Launcher oscuro estilo mac con panel principal de vidrio, fondo profundo y notas de parche mas limpias."
                    .to_string(),
                "Top bar, menus, status bar, paleta de comandos y ventanas flotantes redibujadas con el mismo lenguaje visual."
                    .to_string(),
                "Hierarchy, Inspector, Browser, Graph editor y Code editor ahora usan paneles conectados al estado real del motor."
                    .to_string(),
                "Blueprint editor y ventanas flotantes heredan el nuevo estilo visual sin cambiar su runtime."
                    .to_string(),
                "Botones, campos de busqueda, filas y nodos visuales se ven mas avanzados sin cambiar la logica de gameplay."
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
                "Ventanas flotantes movibles para scripts Rhai, blueprints y Play Window."
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
                "Validacion reforzada para proyecto, escenas, prefabs, Rhai y graphs.".to_string(),
            ],
        },
    ]
}
