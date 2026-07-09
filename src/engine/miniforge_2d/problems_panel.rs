use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::engine::miniforge_2d::validation::{
    ValidationIssue2D, ValidationReport2D, ValidationSeverity2D,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProblemKind2D {
    LuauScript,
    VisualGraph,
    UnknownNode,
    BrokenLink,
    MissingAsset,
    DuplicateGuid,
    InvalidPrefab,
    InvalidScene,
    MissingComponent,
    BrokenReference,
    UnusedAsset,
    MissingStartScene,
    ExportBlocked,
    InvalidConfig,
    MissingPlugin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProblemItem2D {
    pub kind: ProblemKind2D,
    pub severity: ValidationSeverity2D,
    pub file: String,
    pub target: String,
    pub message: String,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProblemsPanel2D {
    pub problems: Vec<ProblemItem2D>,
}

impl ProblemsPanel2D {
    pub fn from_report(report: &ValidationReport2D) -> Self {
        Self {
            problems: report.issues.iter().map(problem_from_issue).collect(),
        }
    }

    pub fn filter(
        &self,
        kind: Option<ProblemKind2D>,
        severity: Option<ValidationSeverity2D>,
    ) -> Vec<&ProblemItem2D> {
        self.problems
            .iter()
            .filter(|problem| kind.as_ref().is_none_or(|kind| &problem.kind == kind))
            .filter(|problem| severity.is_none_or(|severity| problem.severity == severity))
            .collect()
    }

    pub fn group_by_file(&self) -> BTreeMap<String, Vec<&ProblemItem2D>> {
        let mut grouped = BTreeMap::new();
        for problem in &self.problems {
            grouped
                .entry(problem.file.clone())
                .or_insert_with(Vec::new)
                .push(problem);
        }
        grouped
    }

    pub fn group_by_severity(&self) -> BTreeMap<ValidationSeverity2D, Vec<&ProblemItem2D>> {
        let mut grouped = BTreeMap::new();
        for problem in &self.problems {
            grouped
                .entry(problem.severity)
                .or_insert_with(Vec::new)
                .push(problem);
        }
        grouped
    }

    pub fn toolbar_counts(&self) -> (usize, usize) {
        (
            self.problems
                .iter()
                .filter(|problem| problem.severity == ValidationSeverity2D::Error)
                .count(),
            self.problems
                .iter()
                .filter(|problem| problem.severity == ValidationSeverity2D::Warning)
                .count(),
        )
    }

    pub fn safe_fixes(&self) -> Vec<&ProblemItem2D> {
        self.problems
            .iter()
            .filter(|problem| problem.fix.is_some())
            .collect()
    }
}

fn problem_from_issue(issue: &ValidationIssue2D) -> ProblemItem2D {
    let kind = match issue.code.as_str() {
        "missing_script" => ProblemKind2D::LuauScript,
        "invalid_node_kind" => ProblemKind2D::UnknownNode,
        "edge_missing_from" | "edge_missing_to" | "missing_child" => ProblemKind2D::BrokenLink,
        "missing_asset" | "package_missing_asset" => ProblemKind2D::MissingAsset,
        "duplicate_guid" => ProblemKind2D::DuplicateGuid,
        "missing_default_state" | "transition_from_missing" | "transition_to_missing" => {
            ProblemKind2D::VisualGraph
        }
        "missing_root" => ProblemKind2D::VisualGraph,
        "package_start_scene" => ProblemKind2D::MissingStartScene,
        "package_game_name" => ProblemKind2D::InvalidConfig,
        _ => ProblemKind2D::BrokenReference,
    };
    ProblemItem2D {
        kind,
        severity: issue.severity,
        file: issue.path.clone(),
        target: issue.code.clone(),
        message: issue.message.clone(),
        fix: safe_fix_for(issue),
    }
}

fn safe_fix_for(issue: &ValidationIssue2D) -> Option<String> {
    match issue.code.as_str() {
        "missing_key" => Some("add_default_key".to_string()),
        "layout_theme" => Some("reset_dark_theme".to_string()),
        _ => None,
    }
}
