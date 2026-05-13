use std::io;
use std::path::{Path, PathBuf};

use crate::engine::asset_tools::AssetTools;
use crate::engine::project_templates::ProjectTemplates;
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
}

#[derive(Debug, Clone)]
pub struct EguiProjectLauncher {
    pub workspace_root: PathBuf,
    pub project_name: String,
    pub open_path: String,
    pub selected_template: LauncherTemplate,
    pub recent_projects: Vec<PathBuf>,
    pub export_profile: ExportProfile,
    pub status: String,
    pub last_action: Option<LauncherAction>,
}

impl Default for EguiProjectLauncher {
    fn default() -> Self {
        Self::new("projects")
    }
}

impl EguiProjectLauncher {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            project_name: "NewProject".to_string(),
            open_path: String::new(),
            selected_template: LauncherTemplate::TopDown,
            recent_projects: Vec::new(),
            export_profile: ExportProfile::Debug,
            status: String::new(),
            last_action: None,
        }
    }

    pub fn create_new_project(&mut self) -> io::Result<PathBuf> {
        let safe = AssetTools::safe_name(&self.project_name, "NewProject");
        let project_path = AssetTools::unique_path(&self.workspace_root, &safe);
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
        self.record_recent(path.clone());
        self.status = format!("Proyecto abierto: {}", path.display());
        self.last_action = Some(LauncherAction::OpenProject(path.clone()));
        Ok(path)
    }

    pub fn open_typed_project(&mut self) -> io::Result<PathBuf> {
        let path = if self.open_path.trim().is_empty() {
            AssetTools::default_project_path()
        } else {
            PathBuf::from(self.open_path.trim())
        };
        self.open_project(path)
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

    pub fn record_recent(&mut self, path: PathBuf) {
        self.recent_projects.retain(|existing| existing != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(8);
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
