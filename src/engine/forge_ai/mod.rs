//! ForgeAI orchestration layer.
//!
//! This module keeps AI planning separate from editor/runtime internals. The
//! first implementation is a deterministic vertical slice that produces typed
//! actions and executes them through a host interface implemented by
//! `EditorCore`.

pub mod actions;
pub mod agent;
pub mod context;
pub mod diagnostics;
pub mod executor;
pub mod memory;
pub mod optimizer;
pub mod permissions;
pub mod planner;
pub mod providers;
pub mod testing;
pub mod validator;

use std::fmt;

use serde::{Deserialize, Serialize};

pub type AiResult<T> = Result<T, AiError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiErrorKind {
    Permission,
    Validation,
    Provider,
    Execution,
    Io,
    Serde,
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiError {
    pub kind: AiErrorKind,
    pub message: String,
}

impl AiError {
    pub fn new(kind: AiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(AiErrorKind::Validation, message)
    }

    pub fn execution(message: impl Into<String>) -> Self {
        Self::new(AiErrorKind::Execution, message)
    }
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AiError {}

impl From<std::io::Error> for AiError {
    fn from(error: std::io::Error) -> Self {
        Self::new(AiErrorKind::Io, error.to_string())
    }
}

impl From<serde_json::Error> for AiError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(AiErrorKind::Serde, error.to_string())
    }
}

#[cfg(feature = "editor_core")]
impl From<crate::engine::editor_core::EditorCoreError> for AiError {
    fn from(error: crate::engine::editor_core::EditorCoreError) -> Self {
        Self::new(AiErrorKind::Execution, error.to_string())
    }
}

pub use actions::{AiAction, AiActionPreview};
pub use agent::{ForgeAiAgent, ForgeAiRequest, ForgeAiResponse};
pub use context::{AiAssetContext, AiComponentContext, AiEntityContext, AiProjectContext};
pub use diagnostics::{AiDiagnostic, AiDiagnosticSeverity, ProjectDoctor};
pub use executor::{
    AiEditorHost, AiExecutionOptions, AiExecutionReport, AiExecutor, AiFileChange, AiHostValidation,
};
pub use memory::{AiDecisionRecord, AiMemoryStore, AiProjectMemory};
pub use optimizer::{AiOptimizationSuggestion, AiOptimizer};
pub use permissions::{AiPermissionLevel, AiPermissionPolicy};
pub use planner::{AiPlan, AiPlanStep, AiPlanStepStatus, AiRiskLevel};
pub use providers::{AiProvider, AiProviderId, AiReview, ChangeSet, LocalRuleProvider};
pub use testing::{AiTestCase, AiTestReport, AiTestStatus, AiTestSuite};
pub use validator::{AiValidationReport, AiValidator, MiniForgeLuauApiDoc};
