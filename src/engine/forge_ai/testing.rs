use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::context::AiProjectContext;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiTestStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiTestCase {
    pub id: String,
    pub description: String,
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiTestSuite {
    pub id: String,
    pub cases: Vec<AiTestCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiTestReport {
    pub suite_id: String,
    pub status: AiTestStatus,
    pub cases_run: usize,
    pub failures: Vec<String>,
    pub replay_path: Option<String>,
}

impl AiTestSuite {
    pub fn enemy_smoke() -> Self {
        Self {
            id: "forge_ai_enemy_smoke".to_string(),
            cases: vec![
                case(
                    "enemy_has_health",
                    "Enemy has Health component.",
                    "Health exists",
                ),
                case(
                    "enemy_has_ai",
                    "Enemy has AI and NavAgent.",
                    "AIController and NavAgent exist",
                ),
                case(
                    "enemy_has_script",
                    "Enemy controller Luau is attached.",
                    "Assets.script or ScriptComponent.path points at enemy_controller.luau",
                ),
            ],
        }
    }

    pub fn run_static(&self, context: &AiProjectContext) -> AiTestReport {
        let mut failures = Vec::new();
        if self.id == "forge_ai_enemy_smoke" {
            let enemy = context.entities.iter().find(|entity| {
                entity.name == "Enemy2D" || entity.tags.iter().any(|tag| tag == "Enemy")
            });
            match enemy {
                Some(entity) => {
                    for component in ["Health", "AIController", "NavAgent"] {
                        if !entity
                            .components
                            .iter()
                            .any(|candidate| candidate.component_type == component)
                        {
                            failures.push(format!("Enemy missing {component}"));
                        }
                    }
                }
                None => failures.push("Enemy2D entity not found".to_string()),
            }
        }
        AiTestReport {
            suite_id: self.id.clone(),
            status: if failures.is_empty() {
                AiTestStatus::Passed
            } else {
                AiTestStatus::Failed
            },
            cases_run: self.cases.len(),
            failures,
            replay_path: None,
        }
    }
}

fn case(id: &str, description: &str, expected: &str) -> AiTestCase {
    AiTestCase {
        id: id.to_string(),
        description: description.to_string(),
        expected: expected.to_string(),
    }
}
