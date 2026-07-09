use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::forge_ai::actions::{AiAction, AiActionPreview};
use crate::engine::forge_ai::diagnostics::{AiDiagnostic, ProjectDoctor};
#[cfg(feature = "editor")]
use crate::engine::forge_ai::optimizer::AiOptimizer;
use crate::engine::forge_ai::permissions::AiPermissionPolicy;
use crate::engine::forge_ai::planner::AiPlan;
use crate::engine::forge_ai::testing::AiTestReport;
use crate::engine::forge_ai::validator::AiValidator;
use crate::engine::forge_ai::{AiError, AiErrorKind, AiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiExecutionOptions {
    pub approved: bool,
    pub dry_run: bool,
    pub continue_on_error: bool,
}

impl Default for AiExecutionOptions {
    fn default() -> Self {
        Self {
            approved: false,
            dry_run: true,
            continue_on_error: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiHostValidation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiFileChange {
    pub relative_path: String,
    pub created: bool,
    pub bytes_written: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AiExecutionReport {
    pub success: bool,
    pub dry_run: bool,
    pub previews: Vec<AiActionPreview>,
    pub changed_entities: Vec<u64>,
    pub files_written: Vec<AiFileChange>,
    pub prefabs_created: Vec<String>,
    pub validation: Option<AiHostValidation>,
    pub tests: Vec<AiTestReport>,
    pub diagnostics: Vec<AiDiagnostic>,
    pub optimization_notes: Vec<String>,
    pub errors: Vec<String>,
}

pub trait AiEditorHost {
    fn find_entity_id(&self, name: &str) -> AiResult<Option<u64>>;
    fn create_entity(&mut self, name: &str, x: f64, y: f64) -> AiResult<u64>;
    fn add_component(&mut self, entity_id: u64, component_type: &str) -> AiResult<()>;
    fn set_component_property(
        &mut self,
        entity_id: u64,
        component_type: &str,
        key: &str,
        value: Value,
    ) -> AiResult<Value>;
    fn write_project_file(&mut self, relative_path: &str, contents: &str)
    -> AiResult<AiFileChange>;
    fn create_prefab(&mut self, entity_id: u64, prefab_name: &str) -> AiResult<String>;
    fn validate_project(&mut self) -> AiResult<AiHostValidation>;
    fn run_ai_test(&mut self, suite_id: &str) -> AiResult<AiTestReport>;
    fn analyze_performance(&mut self) -> AiResult<Vec<String>>;
}

#[derive(Debug, Clone, Default)]
pub struct AiExecutor {
    pub permissions: AiPermissionPolicy,
}

impl AiExecutor {
    pub fn new(permissions: AiPermissionPolicy) -> Self {
        Self { permissions }
    }

    pub fn preview(plan: &AiPlan) -> Vec<AiActionPreview> {
        plan.actions.iter().map(AiAction::preview).collect()
    }

    pub fn execute_plan<H: AiEditorHost>(
        &self,
        host: &mut H,
        plan: &AiPlan,
        options: &AiExecutionOptions,
    ) -> AiExecutionReport {
        let mut report = AiExecutionReport {
            success: true,
            dry_run: options.dry_run || !options.approved,
            previews: Self::preview(plan),
            ..AiExecutionReport::default()
        };

        let validation = AiValidator::validate_plan(plan);
        if !validation.valid {
            report.success = false;
            report.errors.extend(validation.errors);
            return report;
        }

        let mut resolved_entities = BTreeMap::<String, u64>::new();
        for action in &plan.actions {
            if let Err(error) = action.validate() {
                self.record_error(&mut report, error, options);
                if !options.continue_on_error {
                    break;
                }
            }
            if !self.permissions.can_preview(action) {
                self.record_error(
                    &mut report,
                    AiError::new(AiErrorKind::Permission, "action cannot be previewed"),
                    options,
                );
                continue;
            }
            if report.dry_run {
                continue;
            }
            if !self.permissions.can_execute(action, options.approved) {
                self.record_error(
                    &mut report,
                    AiError::new(
                        AiErrorKind::Permission,
                        format!("permission denied for {}", action.action_type()),
                    ),
                    options,
                );
                if !options.continue_on_error {
                    break;
                }
                continue;
            }
            if let Err(error) =
                self.execute_action(host, action, &mut resolved_entities, &mut report)
            {
                self.record_error(&mut report, error, options);
                if !options.continue_on_error {
                    break;
                }
            }
        }
        report.success = report.errors.is_empty();
        report
    }

    fn execute_action<H: AiEditorHost>(
        &self,
        host: &mut H,
        action: &AiAction,
        resolved_entities: &mut BTreeMap<String, u64>,
        report: &mut AiExecutionReport,
    ) -> AiResult<()> {
        match action {
            AiAction::CreateEntity {
                name,
                x,
                y,
                components,
                tags,
                ..
            } => {
                let id = host.create_entity(name, *x, *y)?;
                resolved_entities.insert(name.clone(), id);
                report.changed_entities.push(id);
                for component in components {
                    host.add_component(id, component)?;
                }
                if let Some(tag) = tags.first() {
                    host.set_component_property(id, "Identity", "tag", Value::String(tag.clone()))?;
                }
            }
            AiAction::AddComponent {
                entity_id,
                entity_name,
                component_type,
                properties,
                ..
            } => {
                let id =
                    resolve_entity(host, resolved_entities, *entity_id, entity_name.as_deref())?;
                host.add_component(id, component_type)?;
                for (key, value) in properties {
                    host.set_component_property(id, component_type, key, value.clone())?;
                }
                report.changed_entities.push(id);
            }
            AiAction::SetComponentProperty {
                entity_id,
                entity_name,
                component_type,
                key,
                value,
                ..
            } => {
                let id =
                    resolve_entity(host, resolved_entities, *entity_id, entity_name.as_deref())?;
                host.set_component_property(id, component_type, key, value.clone())?;
                report.changed_entities.push(id);
            }
            AiAction::CreateLuauScript {
                relative_path,
                source,
                ..
            }
            | AiAction::ModifyLuauScript {
                relative_path,
                source,
                ..
            } => {
                let luau = AiValidator::validate_luau_source(source, relative_path);
                if !luau.valid {
                    return Err(AiError::validation(luau.errors.join("; ")));
                }
                let change = host.write_project_file(relative_path, source)?;
                report.files_written.push(change);
                if let AiAction::CreateLuauScript {
                    attach_to_entity_name: Some(entity_name),
                    ..
                } = action
                {
                    let id = resolve_entity(host, resolved_entities, None, Some(entity_name))?;
                    let _ = host.add_component(id, "ScriptComponent");
                    host.set_component_property(
                        id,
                        "ScriptComponent",
                        "path",
                        Value::String(relative_path.clone()),
                    )?;
                    if let Some(file_name) = relative_path.rsplit('/').next() {
                        host.set_component_property(
                            id,
                            "Assets",
                            "script",
                            Value::String(file_name.to_string()),
                        )?;
                    }
                    report.changed_entities.push(id);
                }
            }
            AiAction::CreatePrefab {
                entity_id,
                entity_name,
                prefab_name,
                ..
            } => {
                let id =
                    resolve_entity(host, resolved_entities, *entity_id, entity_name.as_deref())?;
                let path = host.create_prefab(id, prefab_name)?;
                report.prefabs_created.push(path);
            }
            AiAction::ValidateProject { .. } => {
                let validation = host.validate_project()?;
                report
                    .diagnostics
                    .extend(ProjectDoctor::from_project_validation(
                        &validation.errors,
                        &validation.warnings,
                    ));
                report.validation = Some(validation);
            }
            AiAction::RunTests { suites, .. } => {
                for suite in suites {
                    report.tests.push(host.run_ai_test(suite)?);
                }
            }
            AiAction::AnalyzePerformance { .. } => {
                report
                    .optimization_notes
                    .extend(host.analyze_performance()?);
            }
            unsupported => {
                return Err(AiError::execution(format!(
                    "{} is planned but not executable in this vertical slice",
                    unsupported.action_type()
                )));
            }
        }
        report.changed_entities.sort_unstable();
        report.changed_entities.dedup();
        Ok(())
    }

    fn record_error(
        &self,
        report: &mut AiExecutionReport,
        error: AiError,
        _options: &AiExecutionOptions,
    ) {
        report.success = false;
        report.errors.push(error.to_string());
    }
}

fn resolve_entity<H: AiEditorHost>(
    host: &H,
    resolved: &BTreeMap<String, u64>,
    entity_id: Option<u64>,
    entity_name: Option<&str>,
) -> AiResult<u64> {
    if let Some(id) = entity_id {
        return Ok(id);
    }
    if let Some(name) = entity_name {
        if let Some(id) = resolved.get(name).copied() {
            return Ok(id);
        }
        if let Some(id) = host.find_entity_id(name)? {
            return Ok(id);
        }
    }
    Err(AiError::new(
        AiErrorKind::NotFound,
        "could not resolve entity target",
    ))
}

#[cfg(feature = "editor")]
impl AiEditorHost for crate::engine::editor_core::EditorCore {
    fn find_entity_id(&self, name: &str) -> AiResult<Option<u64>> {
        let count = self.entity_count()?;
        for index in 0..count {
            let row = self.entity_at(index)?;
            if row.name == name {
                return Ok(Some(row.id));
            }
        }
        Ok(None)
    }

    fn create_entity(&mut self, name: &str, x: f64, y: f64) -> AiResult<u64> {
        self.forge_ai_create_entity(name, x, y).map_err(Into::into)
    }

    fn add_component(&mut self, entity_id: u64, component_type: &str) -> AiResult<()> {
        self.forge_ai_add_component(entity_id, component_type)
            .map_err(Into::into)
    }

    fn set_component_property(
        &mut self,
        entity_id: u64,
        component_type: &str,
        key: &str,
        value: Value,
    ) -> AiResult<Value> {
        self.forge_ai_set_component_property(entity_id, component_type, key, value)
            .map_err(Into::into)
    }

    fn write_project_file(
        &mut self,
        relative_path: &str,
        contents: &str,
    ) -> AiResult<AiFileChange> {
        self.forge_ai_write_project_file(relative_path, contents)
            .map_err(Into::into)
    }

    fn create_prefab(&mut self, entity_id: u64, prefab_name: &str) -> AiResult<String> {
        self.forge_ai_create_prefab(entity_id, prefab_name)
            .map_err(Into::into)
    }

    fn validate_project(&mut self) -> AiResult<AiHostValidation> {
        self.forge_ai_validate_project().map_err(Into::into)
    }

    fn run_ai_test(&mut self, suite_id: &str) -> AiResult<AiTestReport> {
        self.forge_ai_run_test(suite_id).map_err(Into::into)
    }

    fn analyze_performance(&mut self) -> AiResult<Vec<String>> {
        let context = crate::engine::forge_ai::context::AiProjectContext::from_editor_core(self)?;
        let notes = AiOptimizer::analyze(&context)
            .into_iter()
            .map(|suggestion| {
                format!(
                    "[{}] {}: {} ({})",
                    suggestion.severity, suggestion.system, suggestion.message, suggestion.evidence
                )
            })
            .collect();
        Ok(notes)
    }
}
