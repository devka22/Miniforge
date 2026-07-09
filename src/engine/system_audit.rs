use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::plugin_manager::PluginManager;
use crate::engine::project_validator::ProjectValidator;
use crate::engine::resource_manager::{ResourceManager, ResourceReport};
use crate::engine::runtime_config::{RuntimeConfig, RuntimeTuning};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    Info,
    Improve,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemReadinessLevel {
    Ready,
    Watch,
    Weak,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemAuditFinding {
    pub system: String,
    pub severity: AuditSeverity,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemReadinessArea {
    pub system: String,
    pub level: SystemReadinessLevel,
    pub score: u8,
    #[serde(default)]
    pub strengths: Vec<String>,
    #[serde(default)]
    pub gaps: Vec<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemReadinessReport {
    pub project_path: String,
    pub total_score: u8,
    #[serde(default)]
    pub areas: BTreeMap<String, SystemReadinessArea>,
    #[serde(default)]
    pub findings: Vec<SystemAuditFinding>,
    #[serde(default)]
    pub next_pass_backlog: Vec<String>,
}

#[derive(Debug, Clone)]
struct AuditContext {
    project_path: PathBuf,
    paths: crate::engine::asset_tools::ProjectPaths,
    resources: ResourceReport,
    validator_errors: Vec<String>,
    validator_warnings: Vec<String>,
    runtime_tuning: RuntimeTuning,
}

#[derive(Debug, Clone)]
struct AreaBuilder {
    system: String,
    strengths: Vec<String>,
    gaps: Vec<String>,
    next_actions: Vec<String>,
    blockers: Vec<String>,
}

impl SystemReadinessReport {
    pub fn audit_project(project_path: impl AsRef<Path>) -> io::Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();
        let paths = AssetTools::get_project_paths(&project_path);
        let resources = ResourceManager::scan_project_resources(&project_path)
            .map(|manager| manager.report())
            .unwrap_or_default();
        let mut validator = ProjectValidator::default();
        validator.validate(&project_path);
        let runtime_tuning = read_runtime_tuning(&paths.settings);
        let ctx = AuditContext {
            project_path: project_path.clone(),
            paths,
            resources,
            validator_errors: validator.errors,
            validator_warnings: validator.warnings,
            runtime_tuning,
        };

        let mut report = Self {
            project_path: project_path.display().to_string(),
            total_score: 0,
            areas: BTreeMap::new(),
            findings: Vec::new(),
            next_pass_backlog: Vec::new(),
        };

        for area in [
            audit_project_shell(&ctx),
            audit_assets(&ctx),
            audit_scenes(&ctx),
            audit_scripting(&ctx),
            audit_ui(&ctx),
            audit_gameplay(&ctx),
            audit_physics(&ctx),
            audit_audio(&ctx),
            audit_rendering(&ctx),
            audit_input(&ctx),
            audit_packaging(&ctx),
            audit_plugins(&ctx),
            audit_runtime(&ctx),
            audit_editor(&ctx),
        ] {
            report.add_area(area);
        }
        report.finalize();
        Ok(report)
    }

    pub fn top_actions(&self, limit: usize) -> Vec<String> {
        self.next_pass_backlog.iter().take(limit).cloned().collect()
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|error| {
            json!({
                "project_path": self.project_path,
                "serialization_error": error.to_string(),
            })
        })
    }

    pub fn concise_summary(&self) -> String {
        format!(
            "Readiness {}% | {} sistemas | {} hallazgos | proxima accion: {}",
            self.total_score,
            self.areas.len(),
            self.findings.len(),
            self.next_pass_backlog
                .first()
                .cloned()
                .unwrap_or_else(|| "mantener cobertura y polish".to_string())
        )
    }

    fn add_area(&mut self, builder: AreaBuilder) {
        let system = builder.system.clone();
        let score = area_score(builder.gaps.len(), builder.blockers.len());
        let level = readiness_level(score, builder.blockers.len());
        let area = SystemReadinessArea {
            system: system.clone(),
            level,
            score,
            strengths: builder.strengths,
            gaps: builder.gaps.clone(),
            next_actions: builder.next_actions.clone(),
        };
        for blocker in builder.blockers {
            self.findings.push(SystemAuditFinding {
                system: system.clone(),
                severity: AuditSeverity::Blocker,
                message: blocker.clone(),
                next_action: builder
                    .next_actions
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "resolver bloqueo antes de exportar".to_string()),
            });
        }
        for gap in builder.gaps {
            self.findings.push(SystemAuditFinding {
                system: system.clone(),
                severity: if score < 70 {
                    AuditSeverity::Warning
                } else {
                    AuditSeverity::Improve
                },
                message: gap,
                next_action: builder
                    .next_actions
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "agendar mejora concreta".to_string()),
            });
        }
        self.areas.insert(system, area);
    }

    fn finalize(&mut self) {
        if self.areas.is_empty() {
            self.total_score = 0;
            return;
        }
        let total = self
            .areas
            .values()
            .map(|area| area.score as usize)
            .sum::<usize>();
        self.total_score = (total / self.areas.len()).min(100) as u8;
        self.findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.system.cmp(&b.system))
                .then_with(|| a.message.cmp(&b.message))
        });
        let mut backlog = Vec::new();
        for finding in &self.findings {
            let item = format!("{}: {}", finding.system, finding.next_action);
            if !backlog.contains(&item) {
                backlog.push(item);
            }
        }
        self.next_pass_backlog = backlog;
    }
}

impl AreaBuilder {
    fn new(system: &str) -> Self {
        Self {
            system: system.to_string(),
            strengths: Vec::new(),
            gaps: Vec::new(),
            next_actions: Vec::new(),
            blockers: Vec::new(),
        }
    }

    fn strength(mut self, value: impl Into<String>) -> Self {
        self.strengths.push(value.into());
        self
    }

    fn gap(mut self, value: impl Into<String>, next_action: impl Into<String>) -> Self {
        self.gaps.push(value.into());
        self.next_actions.push(next_action.into());
        self
    }

    fn blocker(mut self, value: impl Into<String>, next_action: impl Into<String>) -> Self {
        self.blockers.push(value.into());
        self.next_actions.push(next_action.into());
        self
    }
}

fn audit_project_shell(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Project");
    if ctx.project_path.join("project.json").exists()
        || ctx
            .project_path
            .join("project")
            .join("project.json")
            .exists()
    {
        area = area.strength("project.json presente");
    } else {
        area = area.blocker("project.json ausente", "crear metadata base del proyecto");
    }
    if ctx.validator_errors.is_empty() {
        area = area.strength("sin errores de ProjectValidator");
    } else {
        area = area.blocker(
            format!("{} errores de validacion", ctx.validator_errors.len()),
            "ejecutar auto_fix_safe y corregir errores restantes",
        );
    }
    if ctx.validator_warnings.len() > 8 {
        area = area.gap(
            format!("{} warnings de validacion", ctx.validator_warnings.len()),
            "limpiar warnings antes de una version exportable",
        );
    }
    area
}

fn audit_assets(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Assets");
    if ctx.resources.total_files > 0 {
        area = area.strength(format!("{} recursos indexados", ctx.resources.total_files));
    } else {
        area = area.gap(
            "no hay recursos indexados",
            "crear assets minimos de sprites/data/audio",
        );
    }
    if count(ctx, "image") == 0 {
        area = area.gap(
            "sin sprites/imagenes",
            "agregar sprites base o placeholders visuales",
        );
    }
    if !ctx.resources.duplicates.is_empty() {
        area = area.gap(
            format!(
                "{} recursos duplican nombre",
                ctx.resources.duplicates.len()
            ),
            "usar GUID/rutas canonicas para dependencias duplicadas",
        );
    }
    area
}

fn audit_scenes(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Scenes");
    if count(ctx, "scene") > 0 {
        area = area.strength(format!("{} escenas encontradas", count(ctx, "scene")));
    } else {
        area = area.blocker("no hay escenas", "crear una escena inicial en saves/scenes");
    }
    let start_scene = read_json(ctx.project_path.join("engine_config.json")).and_then(|value| {
        value
            .get("start_scene")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    if let Some(scene) = start_scene {
        let normalized = crate::engine::scene_manager::normalize_scene_name(&scene);
        if ctx.paths.scenes.join(normalized).exists() {
            area = area.strength("start_scene resuelve a una escena real");
        } else {
            area = area.gap(
                format!("start_scene no encontrado: {scene}"),
                "actualizar engine_config.json o crear la escena",
            );
        }
    } else {
        area = area.gap(
            "engine_config sin start_scene",
            "definir start_scene para runtime/export",
        );
    }
    area
}

fn audit_scripting(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Scripting");
    let scripts = count(ctx, "script");
    let graphs = count(ctx, "visual_graph");
    if scripts + graphs > 0 {
        area = area.strength(format!("{scripts} scripts Luau y {graphs} visual graphs"));
    } else {
        area = area.gap(
            "sin logica runtime",
            "crear un script Luau o visual graph de arranque",
        );
    }
    if ctx
        .validator_warnings
        .iter()
        .any(|warning| warning.contains("Graph con runtime desconocido"))
    {
        area = area.gap(
            "hay graphs con runtime desconocido",
            "migrar graphs a rust_visual_graph",
        );
    }
    area
}

fn audit_ui(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("UI");
    if count(ctx, "ui_document") > 0 || project_contains(ctx, "ui_canvases") {
        area = area.strength("UI document/canvas detectado");
    } else {
        area = area.gap(
            "sin UI document/canvas",
            "crear HUDScreen o assets/ui/*.mfui",
        );
    }
    if project_contains(ctx, "ScreenManager") || project_contains(ctx, "HUDScreen") {
        area = area.strength("pantallas UI modernas detectadas");
    } else {
        area = area.gap(
            "ScreenManager aun no esta conectado en contenido",
            "migrar HUD/menu a UIScreen + ScreenManager",
        );
    }
    area
}

fn audit_gameplay(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Gameplay");
    if count(ctx, "prefab") > 0 {
        area = area.strength(format!("{} prefabs jugables", count(ctx, "prefab")));
    } else {
        area = area.gap(
            "sin prefabs",
            "crear prefabs de player/enemy/pickups para iterar mas rapido",
        );
    }
    if project_contains_any(
        ctx,
        &["Health", "Stats", "Inventory", "Ability", "QuestLog"],
    ) {
        area = area.strength("componentes gameplay encontrados");
    } else {
        area = area.gap(
            "no se detectan componentes gameplay comunes",
            "usar archetypes o templates para player/enemy/items",
        );
    }
    area
}

fn audit_physics(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Physics");
    if project_contains_any(
        ctx,
        &["Collider2D", "Rigidbody2D", "Trigger2D", "TilemapCollider"],
    ) {
        area = area.strength("componentes fisicos detectados");
    } else {
        area = area.gap(
            "sin fisica/colisiones en contenido",
            "agregar Collider2D/Rigidbody2D a entidades jugables",
        );
    }
    if ctx.paths.settings.join("physics2d.json").exists()
        || ctx.paths.settings.join("project_settings2d.json").exists()
    {
        area = area.strength("settings de fisica disponibles");
    } else {
        area = area.gap(
            "sin settings explicitos de fisica",
            "crear settings/physics2d.json o validar ProjectSettings2D",
        );
    }
    area
}

fn audit_audio(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Audio");
    if count(ctx, "audio") > 0 {
        area = area.strength(format!("{} assets de audio", count(ctx, "audio")));
    } else {
        area = area.gap(
            "sin audio",
            "agregar SFX/music placeholders y AudioSource2D",
        );
    }
    if project_contains_any(ctx, &["AudioSource", "AudioSource2D", "sound"]) {
        area = area.strength("referencias de audio en contenido");
    }
    area
}

fn audit_rendering(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Rendering");
    if count(ctx, "image") > 0
        || project_contains_any(ctx, &["SpriteRenderer", "TilemapRenderer2D"])
    {
        area = area.strength("contenido 2D renderizable detectado");
    } else {
        area = area.gap(
            "sin contenido renderizable",
            "agregar SpriteRenderer o TilemapRenderer2D a una escena",
        );
    }
    if project_contains_any(ctx, &["RenderGraph2D", "TextureAtlas2D", "SpriteBatcher"]) {
        area = area.strength("pipeline/render graph avanzado aparece en contenido");
    } else {
        area = area.gap(
            "sin configuracion render avanzada en proyecto",
            "preparar atlas/batching/render graph para la pasada fuerte",
        );
    }
    area
}

fn audit_input(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Input");
    let input_path = ctx.paths.settings.join("input_map.json");
    if input_path.exists() {
        area = area.strength("input_map.json presente");
        let text = fs::read_to_string(input_path).unwrap_or_default();
        for action in ["Move", "Interact", "Pause"] {
            if !text.contains(action) {
                area = area.gap(
                    format!("accion input ausente: {action}"),
                    "completar input_map con acciones base",
                );
            }
        }
    } else {
        area = area.gap(
            "sin input_map.json",
            "crear settings/input_map.json desde InputMap::default",
        );
    }
    area
}

fn audit_packaging(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Packaging");
    if ctx.paths.settings.join("build_settings.json").exists()
        || ctx.project_path.join("build_settings.json").exists()
    {
        area = area.strength("build settings presentes");
    } else {
        area = area.gap(
            "sin build_settings.json",
            "guardar BuildSettings antes de empaquetar",
        );
    }
    if ctx.project_path.join("runtime_manifest.json").exists()
        || ctx.project_path.join("manifest.json").exists()
    {
        area = area.strength("manifest de runtime/proyecto presente");
    } else {
        area = area.gap(
            "sin manifest de build/proyecto",
            "ejecutar build_manifest/export_runtime",
        );
    }
    area
}

fn audit_plugins(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Plugins");
    match PluginManager::new(&ctx.project_path).load_plan() {
        Ok(plan) => {
            if plan.load_order.is_empty() {
                let explicit_vanilla = read_json(ctx.project_path.join("project.json"))
                    .and_then(|project| project.get("plugin_policy").cloned())
                    .and_then(|value| value.as_str().map(ToString::to_string))
                    .is_some_and(|policy| policy == "vanilla");
                if explicit_vanilla {
                    area = area.strength("politica vanilla explicita; no requiere plugins");
                } else {
                    area = area.gap(
                        "sin plugins activos",
                        "decidir si el juego necesita plugins o dejarlo documentado como vanilla",
                    );
                }
            } else {
                area = area.strength(format!("{} plugins activos", plan.load_order.len()));
            }
            if !plan.blocked_plugins.is_empty() {
                area = area.gap(
                    format!("{} plugins bloqueados", plan.blocked_plugins.len()),
                    "resolver dependencias de plugins bloqueados",
                );
            }
        }
        Err(error) => {
            area = area.gap(
                format!("no se pudo leer plan de plugins: {error}"),
                "validar plugin.json y dependencias",
            );
        }
    }
    area
}

fn audit_runtime(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Runtime");
    if ctx.runtime_tuning.complex_game_ready() {
        area = area.strength("runtime_tuning listo para juegos complejos");
    } else {
        for warning in ctx.runtime_tuning.warnings() {
            area = area.gap(warning, "ajustar settings/runtime_config.json");
        }
    }
    if ctx.runtime_tuning.max_frame_steps < 2 {
        area = area.gap(
            "max_frame_steps demasiado bajo para frames irregulares",
            "subir max_frame_steps para estabilidad del fixed timestep",
        );
    }
    area
}

fn audit_editor(ctx: &AuditContext) -> AreaBuilder {
    let mut area = AreaBuilder::new("Editor");
    if ctx.paths.settings.join("editor_layout.json").exists() {
        area = area.strength("layout del editor persistido");
    } else {
        area = area.gap(
            "sin editor_layout.json",
            "guardar layout por defecto para equipos y sesiones largas",
        );
    }
    if ctx
        .project_path
        .join("project")
        .join("asset_metadata.json")
        .exists()
        || ctx
            .project_path
            .join("project")
            .join("asset_database.json")
            .exists()
    {
        area = area.strength("metadata de assets disponible");
    } else {
        area = area.gap(
            "sin metadata de assets",
            "reconstruir asset database desde Content Browser",
        );
    }
    area
}

fn area_score(gaps: usize, blockers: usize) -> u8 {
    let penalty = gaps.saturating_mul(12) + blockers.saturating_mul(30);
    100usize
        .saturating_sub(penalty)
        .max(if blockers > 0 { 0 } else { 35 }) as u8
}

fn readiness_level(score: u8, blockers: usize) -> SystemReadinessLevel {
    if blockers > 0 {
        SystemReadinessLevel::Blocked
    } else if score >= 85 {
        SystemReadinessLevel::Ready
    } else if score >= 70 {
        SystemReadinessLevel::Watch
    } else {
        SystemReadinessLevel::Weak
    }
}

fn count(ctx: &AuditContext, kind: &str) -> usize {
    ctx.resources.counts.get(kind).copied().unwrap_or(0)
}

fn read_runtime_tuning(settings: &Path) -> RuntimeTuning {
    let data =
        read_json(settings.join("runtime_config.json")).unwrap_or_else(RuntimeConfig::default_data);
    RuntimeTuning::from_value(&data)
}

fn read_json(path: impl AsRef<Path>) -> Option<Value> {
    AssetTools::read_json(path).ok()
}

fn project_contains(ctx: &AuditContext, needle: &str) -> bool {
    project_contains_any(ctx, &[needle])
}

fn project_contains_any(ctx: &AuditContext, needles: &[&str]) -> bool {
    let roots = [
        ctx.paths.scenes.as_path(),
        ctx.paths.prefabs.as_path(),
        ctx.paths.assets.as_path(),
        ctx.paths.scripts.as_path(),
        ctx.paths.settings.as_path(),
    ];
    roots.iter().any(|root| {
        text_files(root)
            .iter()
            .any(|path| file_contains(path, needles))
    })
}

fn text_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(text_files(&path));
        } else if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("json" | "scene" | "prefab" | "luau" | "mfgraph" | "mfui" | "ui2d")
        ) {
            files.push(path);
        }
    }
    files
}

fn file_contains(path: &Path, needles: &[&str]) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    needles.iter().any(|needle| text.contains(needle))
}
