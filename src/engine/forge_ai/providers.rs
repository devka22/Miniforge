use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::context::AiProjectContext;
use crate::engine::forge_ai::planner::{AiPlan, AiPlanner};
use crate::engine::forge_ai::{AiError, AiErrorKind, AiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiProviderId {
    OpenAI,
    Anthropic,
    LocalCompatible,
    LocalRules,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeSet {
    pub summary: String,
    pub files: Vec<String>,
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiReview {
    pub approved: bool,
    pub notes: Vec<String>,
    pub risks: Vec<String>,
}

pub trait AiProvider {
    fn provider_id(&self) -> AiProviderId;
    fn model_name(&self) -> &str;
    fn generate_plan(&self, request: &str, context: &AiProjectContext) -> AiResult<AiPlan>;
    fn review_changes(&self, changes: &ChangeSet) -> AiResult<AiReview>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalRuleProvider {
    pub model: String,
}

impl Default for LocalRuleProvider {
    fn default() -> Self {
        Self {
            model: "forgeai-local-rules-v1".to_string(),
        }
    }
}

impl AiProvider for LocalRuleProvider {
    fn provider_id(&self) -> AiProviderId {
        AiProviderId::LocalRules
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn generate_plan(&self, request: &str, context: &AiProjectContext) -> AiResult<AiPlan> {
        if request.trim().is_empty() {
            return Err(AiError::new(
                AiErrorKind::Provider,
                "ForgeAI request cannot be empty",
            ));
        }
        Ok(AiPlanner::plan(request, context, self.model_name()))
    }

    fn review_changes(&self, changes: &ChangeSet) -> AiResult<AiReview> {
        Ok(AiReview {
            approved: true,
            notes: vec![format!(
                "Local review prepared for {} files and {} entities",
                changes.files.len(),
                changes.entities.len()
            )],
            risks: if changes.files.is_empty() {
                Vec::new()
            } else {
                vec!["Review generated diffs before applying file writes".to_string()]
            },
        })
    }
}
