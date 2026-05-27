use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity2D {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationIssue2D {
    pub severity: ValidationSeverity2D,
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationReport2D {
    pub issues: Vec<ValidationIssue2D>,
}

impl ValidationReport2D {
    pub fn error(
        &mut self,
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(ValidationIssue2D {
            severity: ValidationSeverity2D::Error,
            code: code.into(),
            path: path.into(),
            message: message.into(),
        });
    }

    pub fn warning(
        &mut self,
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(ValidationIssue2D {
            severity: ValidationSeverity2D::Warning,
            code: code.into(),
            path: path.into(),
            message: message.into(),
        });
    }

    pub fn merge(&mut self, other: ValidationReport2D) {
        self.issues.extend(other.issues);
    }

    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity2D::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity2D::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity2D::Warning)
            .count()
    }
}

pub fn require_keys(value: &Value, path: &str, keys: &[&str], report: &mut ValidationReport2D) {
    for key in keys {
        if value.get(*key).is_none() {
            report.error(
                "missing_key",
                format!("{path}.{key}"),
                format!("Falta la clave requerida `{key}`."),
            );
        }
    }
}

pub fn collect_project_references(value: &Value) -> Vec<String> {
    let mut references = Vec::new();
    collect_project_references_inner(value, &mut references);
    references.sort();
    references.dedup();
    references
}

pub fn validate_references(
    value: &Value,
    known_assets: &BTreeSet<String>,
    known_scripts: &BTreeSet<String>,
) -> ValidationReport2D {
    let mut report = ValidationReport2D::default();
    for reference in collect_project_references(value) {
        if reference.starts_with("scripts/") {
            if !known_scripts.contains(&reference) {
                report.warning(
                    "missing_script",
                    reference.clone(),
                    format!("Script referenciado no existe: {reference}"),
                );
            }
        } else if !known_assets.contains(&reference) {
            report.warning(
                "missing_asset",
                reference.clone(),
                format!("Asset referenciado no existe: {reference}"),
            );
        }
    }
    report
}

pub fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn collect_project_references_inner(value: &Value, references: &mut Vec<String>) {
    match value {
        Value::String(text)
            if is_project_reference(text)
                || text.ends_with(".sprite.json")
                || text.ends_with(".tileset.json")
                || text.ends_with(".tilemap.json")
                || text.ends_with(".anim2d.json")
                || text.ends_with(".ui2d.json")
                || text.ends_with(".seq2d.json")
                || text.ends_with(".bt2d.json")
                || text.ends_with(".mfgraph") =>
        {
            references.push(text.clone());
        }
        Value::Array(items) => {
            for item in items {
                collect_project_references_inner(item, references);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_project_references_inner(value, references);
            }
        }
        _ => {}
    }
}

fn is_project_reference(text: &str) -> bool {
    (text.starts_with("assets/")
        || text.starts_with("scripts/")
        || text.starts_with("saves/")
        || text.starts_with("settings/")
        || text.starts_with("project/"))
        && text.contains('.')
}
