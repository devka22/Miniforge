use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::AiResult;
use crate::engine::forge_ai::context::AiProjectContext;
use crate::engine::forge_ai::diagnostics::{AiDiagnostic, ProjectDoctor};
use crate::engine::forge_ai::executor::{
    AiEditorHost, AiExecutionOptions, AiExecutionReport, AiExecutor,
};
use crate::engine::forge_ai::permissions::AiPermissionPolicy;
use crate::engine::forge_ai::planner::AiPlan;
use crate::engine::forge_ai::providers::{AiProvider, ChangeSet, LocalRuleProvider};
use crate::engine::forge_ai::validator::{AiValidationReport, AiValidator};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForgeAiRequest {
    pub instruction: String,
    pub approved: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForgeAiResponse {
    pub context_summary: String,
    pub plan: AiPlan,
    pub validation: AiValidationReport,
    pub diagnostics: Vec<AiDiagnostic>,
    pub execution: AiExecutionReport,
}

#[derive(Debug, Clone)]
pub struct ForgeAiAgent<P: AiProvider = LocalRuleProvider> {
    pub provider: P,
    pub permissions: AiPermissionPolicy,
}

impl Default for ForgeAiAgent<LocalRuleProvider> {
    fn default() -> Self {
        Self {
            provider: LocalRuleProvider::default(),
            permissions: AiPermissionPolicy::default(),
        }
    }
}

impl<P: AiProvider> ForgeAiAgent<P> {
    pub fn new(provider: P, permissions: AiPermissionPolicy) -> Self {
        Self {
            provider,
            permissions,
        }
    }

    pub fn plan(&self, request: &ForgeAiRequest, context: &AiProjectContext) -> AiResult<AiPlan> {
        self.provider.generate_plan(&request.instruction, context)
    }

    pub fn run<H: AiEditorHost>(
        &self,
        host: &mut H,
        request: ForgeAiRequest,
        context: AiProjectContext,
    ) -> AiResult<ForgeAiResponse> {
        let plan = self.plan(&request, &context)?;
        let validation = AiValidator::validate_plan(&plan);
        let diagnostics = ProjectDoctor::analyze(&context);
        let changes = ChangeSet {
            summary: plan.objective.clone(),
            files: plan
                .actions
                .iter()
                .flat_map(|action| action.affected_files())
                .collect(),
            entities: plan
                .steps
                .iter()
                .flat_map(|step| step.entities_affected.clone())
                .collect(),
        };
        let _review = self.provider.review_changes(&changes)?;
        let executor = AiExecutor::new(self.permissions.clone());
        let execution = executor.execute_plan(
            host,
            &plan,
            &AiExecutionOptions {
                approved: request.approved,
                dry_run: request.dry_run,
                continue_on_error: false,
            },
        );
        Ok(ForgeAiResponse {
            context_summary: context.summary(),
            plan,
            validation,
            diagnostics,
            execution,
        })
    }
}
