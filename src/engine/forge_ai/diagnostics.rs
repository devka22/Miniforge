use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::context::AiProjectContext;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AiDiagnosticSeverity {
    Critical,
    Error,
    Warning,
    Suggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiDiagnostic {
    pub severity: AiDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub evidence: String,
    pub proposed_fix: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectDoctor;

impl ProjectDoctor {
    pub fn analyze(context: &AiProjectContext) -> Vec<AiDiagnostic> {
        let mut diagnostics = Vec::new();
        if context.entities.is_empty() {
            diagnostics.push(diag(
                AiDiagnosticSeverity::Warning,
                "empty_scene",
                "Active scene has no entities.",
                "Context entity list is empty.",
                "Create a PlayerStart, camera and at least one gameplay actor.",
            ));
        }
        diagnostics.extend(duplicate_entity_names(context));
        diagnostics.extend(invisible_active_entities(context));
        diagnostics.extend(component_pressure(context));
        diagnostics.extend(asset_health(context));
        diagnostics.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.code.cmp(&b.code)));
        diagnostics
    }

    pub fn from_project_validation(errors: &[String], warnings: &[String]) -> Vec<AiDiagnostic> {
        errors
            .iter()
            .map(|error| {
                diag(
                    AiDiagnosticSeverity::Error,
                    "project_validator_error",
                    error,
                    "ProjectValidator reported an error.",
                    "Open the referenced asset, apply migration/fix, then re-run validation.",
                )
            })
            .chain(warnings.iter().map(|warning| {
                diag(
                    AiDiagnosticSeverity::Warning,
                    "project_validator_warning",
                    warning,
                    "ProjectValidator reported a warning.",
                    "Review project settings or asset references.",
                )
            }))
            .collect()
    }
}

fn duplicate_entity_names(context: &AiProjectContext) -> Vec<AiDiagnostic> {
    let mut names = BTreeMap::<String, usize>::new();
    for entity in &context.entities {
        *names.entry(entity.name.clone()).or_default() += 1;
    }
    names
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, count)| {
            diag(
                AiDiagnosticSeverity::Warning,
                "duplicate_entity_name",
                format!("Entity name '{name}' appears {count} times."),
                "Name collision can confuse script and ForgeAI target resolution.",
                "Rename duplicates or use explicit entity ids in actions.",
            )
        })
        .collect()
}

fn invisible_active_entities(context: &AiProjectContext) -> Vec<AiDiagnostic> {
    context
        .entities
        .iter()
        .filter(|entity| entity.enabled && !entity.visible)
        .map(|entity| {
            diag(
                AiDiagnosticSeverity::Suggestion,
                "active_invisible_entity",
                format!("{} is active but invisible.", entity.name),
                format!("entity_id={} layer={}", entity.id, entity.layer),
                "Disable it when not needed or mark it as a non-render gameplay helper.",
            )
        })
        .collect()
}

fn component_pressure(context: &AiProjectContext) -> Vec<AiDiagnostic> {
    let mut diagnostics = Vec::new();
    let collider_count = context.physics_summary.collider_count;
    if collider_count > 1000 {
        diagnostics.push(diag(
            AiDiagnosticSeverity::Warning,
            "too_many_colliders",
            format!("Scene has {collider_count} Collider2D components."),
            "High collider counts can create broadphase and solver pressure.",
            "Use tilemap collision chunks, static bodies and layers to reduce pair checks.",
        ));
    }
    let lights = context.component_count("Light2D") + context.component_count("Light3D");
    if lights > 128 {
        diagnostics.push(diag(
            AiDiagnosticSeverity::Warning,
            "too_many_lights",
            format!("Scene has {lights} lights."),
            "Many dynamic lights can increase draw calls and shadow work.",
            "Batch static lighting, lower shadow casters, and cap dynamic lights per view.",
        ));
    }
    if context.physics_summary.rigidbody_count > 0 && context.physics_summary.collider_count == 0 {
        diagnostics.push(diag(
            AiDiagnosticSeverity::Error,
            "rigidbodies_without_colliders",
            "Rigidbody2D components exist without Collider2D components.",
            format!(
                "{} rigid bodies, {} colliders",
                context.physics_summary.rigidbody_count, context.physics_summary.collider_count
            ),
            "Add Collider2D to physical actors or remove unused Rigidbody2D components.",
        ));
    }
    diagnostics
}

fn asset_health(context: &AiProjectContext) -> Vec<AiDiagnostic> {
    let mut diagnostics = Vec::new();
    if context.prefabs.is_empty() && context.entities.len() > 10 {
        diagnostics.push(diag(
            AiDiagnosticSeverity::Suggestion,
            "missing_prefabs",
            "Project has many scene entities but no indexed prefabs.",
            format!("{} entities, 0 prefabs in context", context.entities.len()),
            "Convert repeated actors into prefabs to improve reuse and AI-safe edits.",
        ));
    }
    let mut paths = BTreeSet::new();
    for asset in &context.assets {
        if !paths.insert(asset.relative_path.clone()) {
            diagnostics.push(diag(
                AiDiagnosticSeverity::Warning,
                "duplicate_asset_path",
                format!("Duplicate asset path {}", asset.relative_path),
                "Asset index contains repeated relative path.",
                "Refresh asset database and regenerate GUIDs if needed.",
            ));
        }
    }
    diagnostics
}

fn diag(
    severity: AiDiagnosticSeverity,
    code: &str,
    message: impl Into<String>,
    evidence: impl Into<String>,
    proposed_fix: impl Into<String>,
) -> AiDiagnostic {
    AiDiagnostic {
        severity,
        code: code.to_string(),
        message: message.into(),
        evidence: evidence.into(),
        proposed_fix: proposed_fix.into(),
    }
}
