use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::actions::AiAction;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AiPermissionLevel {
    ReadOnly,
    Suggest,
    #[default]
    EditWithApproval,
    AutonomousSandbox,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiPermissionPolicy {
    pub level: AiPermissionLevel,
    pub allow_file_writes: bool,
    pub allow_scene_mutation: bool,
    pub allow_test_execution: bool,
}

impl Default for AiPermissionPolicy {
    fn default() -> Self {
        Self {
            level: AiPermissionLevel::EditWithApproval,
            allow_file_writes: true,
            allow_scene_mutation: true,
            allow_test_execution: true,
        }
    }
}

impl AiPermissionPolicy {
    pub fn read_only() -> Self {
        Self {
            level: AiPermissionLevel::ReadOnly,
            allow_file_writes: false,
            allow_scene_mutation: false,
            allow_test_execution: false,
        }
    }

    pub fn can_preview(&self, _action: &AiAction) -> bool {
        true
    }

    pub fn can_execute(&self, action: &AiAction, approved: bool) -> bool {
        match self.level {
            AiPermissionLevel::ReadOnly | AiPermissionLevel::Suggest => false,
            AiPermissionLevel::EditWithApproval => approved && self.permits_action_surface(action),
            AiPermissionLevel::AutonomousSandbox => self.permits_action_surface(action),
        }
    }

    fn permits_action_surface(&self, action: &AiAction) -> bool {
        match action {
            AiAction::CreateLuauScript { .. }
            | AiAction::ModifyLuauScript { .. }
            | AiAction::CreateVisualGraph { .. }
            | AiAction::ModifyVisualGraph { .. }
            | AiAction::ImportAsset { .. } => self.allow_file_writes,
            AiAction::RunProject { .. } | AiAction::RunTests { .. } => self.allow_test_execution,
            AiAction::AnalyzePerformance { .. } | AiAction::ValidateProject { .. } => true,
            _ => self.allow_scene_mutation,
        }
    }

    pub fn requires_explicit_confirmation(action: &AiAction) -> bool {
        matches!(
            action,
            AiAction::DeleteEntity { .. } | AiAction::ModifyScene { .. }
        )
    }
}
