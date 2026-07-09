use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engine::forge_ai::actions::AiAction;
use crate::engine::forge_ai::context::AiProjectContext;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiPlanStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiPlanStep {
    pub id: String,
    pub description: String,
    pub status: AiPlanStepStatus,
    pub risk: AiRiskLevel,
    pub files_affected: Vec<String>,
    pub entities_affected: Vec<String>,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiPlan {
    pub id: String,
    pub objective: String,
    pub provider: String,
    pub estimated_cost: String,
    pub steps: Vec<AiPlanStep>,
    pub actions: Vec<AiAction>,
}

#[derive(Debug, Clone, Default)]
pub struct AiPlanner;

impl AiPlanner {
    pub fn plan(request: &str, context: &AiProjectContext, provider: &str) -> AiPlan {
        if looks_like_enemy_request(request) {
            return enemy_2d_vertical_slice(request, context, provider);
        }
        generic_project_plan(request, context, provider)
    }
}

fn enemy_2d_vertical_slice(request: &str, _context: &AiProjectContext, provider: &str) -> AiPlan {
    let script_path = "scripts/enemy_controller.luau".to_string();
    let enemy_name = "Enemy2D".to_string();
    let components = vec![
        "Actor2D",
        "SpriteRenderer",
        "Rigidbody2D",
        "Collider2D",
        "Health",
        "AIController",
        "NavAgent",
        "StateMachine",
        "CombatTarget",
        "ScriptComponent",
        "FlipbookAnimation2D",
        "Light2D",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();

    let mut actions = Vec::new();
    actions.push(AiAction::CreateEntity {
        action_id: "create_enemy_entity".to_string(),
        name: enemy_name.clone(),
        x: 2.0,
        y: 0.0,
        components,
        tags: vec!["Enemy".to_string()],
    });
    actions.extend([
        set_prop(
            "set_enemy_tag",
            &enemy_name,
            "Identity",
            "tag",
            json!("Enemy"),
        ),
        set_prop(
            "enemy_health_max",
            &enemy_name,
            "Health",
            "max_health",
            json!(100.0),
        ),
        set_prop(
            "enemy_health_now",
            &enemy_name,
            "Health",
            "health",
            json!(100.0),
        ),
        set_prop(
            "enemy_body_no_gravity",
            &enemy_name,
            "Rigidbody2D",
            "use_gravity",
            json!(false),
        ),
        set_prop(
            "enemy_body_freeze_rotation",
            &enemy_name,
            "Rigidbody2D",
            "freeze_rotation",
            json!(true),
        ),
        set_prop(
            "enemy_body_continuous",
            &enemy_name,
            "Rigidbody2D",
            "continuous_collision",
            json!(true),
        ),
        set_prop(
            "enemy_ai_behavior",
            &enemy_name,
            "AIController",
            "behavior",
            json!("patrol_chase_attack"),
        ),
        set_prop(
            "enemy_ai_detection",
            &enemy_name,
            "AIController",
            "detection_radius",
            json!(7.5),
        ),
        set_prop(
            "enemy_ai_attack",
            &enemy_name,
            "AIController",
            "attack_radius",
            json!(1.1),
        ),
        set_prop(
            "enemy_nav_speed",
            &enemy_name,
            "NavAgent",
            "speed",
            json!(3.75),
        ),
        set_prop(
            "enemy_state",
            &enemy_name,
            "StateMachine",
            "current_state",
            json!("Patrol"),
        ),
        set_prop(
            "enemy_light_radius",
            &enemy_name,
            "Light2D",
            "radius",
            json!(3.0),
        ),
        set_prop(
            "enemy_script_path",
            &enemy_name,
            "ScriptComponent",
            "path",
            json!(script_path),
        ),
        set_prop(
            "enemy_asset_script",
            &enemy_name,
            "Assets",
            "script",
            json!("enemy_controller.luau"),
        ),
    ]);
    actions.push(AiAction::CreateLuauScript {
        action_id: "create_enemy_luau".to_string(),
        relative_path: script_path,
        source: enemy_controller_luau(),
        attach_to_entity_name: Some(enemy_name.clone()),
    });
    actions.push(AiAction::CreatePrefab {
        action_id: "create_enemy_prefab".to_string(),
        entity_id: None,
        entity_name: Some(enemy_name.clone()),
        prefab_name: "Enemy2D.prefab".to_string(),
    });
    actions.push(AiAction::ValidateProject {
        action_id: "validate_enemy_slice".to_string(),
    });
    actions.push(AiAction::RunTests {
        action_id: "test_enemy_slice".to_string(),
        suites: vec!["forge_ai_enemy_smoke".to_string()],
    });

    AiPlan {
        id: plan_id(request),
        objective: request.to_string(),
        provider: provider.to_string(),
        estimated_cost: "local deterministic plan; no provider tokens".to_string(),
        steps: vec![
            step(
                "plan_enemy_entity",
                "Create Enemy2D actor with transform, sprite, collider, rigidbody, health, AI and navigation components.",
                AiRiskLevel::Low,
                vec![],
                vec![enemy_name.clone()],
                true,
            ),
            step(
                "configure_enemy_components",
                "Configure patrol, chase, attack, physics, light and inspector-exposed component properties.",
                AiRiskLevel::Medium,
                vec![],
                vec![enemy_name.clone()],
                true,
            ),
            step(
                "create_enemy_script",
                "Generate Luau controller with exported tuning variables and MiniForge API calls.",
                AiRiskLevel::Medium,
                vec!["scripts/enemy_controller.luau".to_string()],
                vec![enemy_name.clone()],
                true,
            ),
            step(
                "create_enemy_prefab",
                "Save configured enemy as a prefab through the existing prefab pipeline.",
                AiRiskLevel::Low,
                vec!["assets/prefabs/Enemy2D.prefab".to_string()],
                vec![enemy_name],
                true,
            ),
            step(
                "validate_and_test",
                "Validate project and run ForgeAI enemy smoke test.",
                AiRiskLevel::Low,
                vec![],
                vec![],
                false,
            ),
        ],
        actions,
    }
}

fn generic_project_plan(request: &str, context: &AiProjectContext, provider: &str) -> AiPlan {
    let actions = vec![
        AiAction::ValidateProject {
            action_id: "validate_project".to_string(),
        },
        AiAction::AnalyzePerformance {
            action_id: "analyze_performance".to_string(),
        },
    ];
    AiPlan {
        id: plan_id(request),
        objective: request.to_string(),
        provider: provider.to_string(),
        estimated_cost: "local deterministic plan; no provider tokens".to_string(),
        steps: vec![
            step(
                "inspect_context",
                &format!("Inspect project context: {}", context.summary()),
                AiRiskLevel::Low,
                vec![],
                vec![],
                false,
            ),
            step(
                "doctor_and_profile",
                "Run Project Doctor and performance analysis before generating edits.",
                AiRiskLevel::Low,
                vec![],
                vec![],
                false,
            ),
        ],
        actions,
    }
}

fn set_prop(
    action_id: &str,
    entity_name: &str,
    component_type: &str,
    key: &str,
    value: serde_json::Value,
) -> AiAction {
    AiAction::SetComponentProperty {
        action_id: action_id.to_string(),
        entity_id: None,
        entity_name: Some(entity_name.to_string()),
        component_type: component_type.to_string(),
        key: key.to_string(),
        value,
    }
}

fn step(
    id: &str,
    description: &str,
    risk: AiRiskLevel,
    files_affected: Vec<String>,
    entities_affected: Vec<String>,
    reversible: bool,
) -> AiPlanStep {
    AiPlanStep {
        id: id.to_string(),
        description: description.to_string(),
        status: AiPlanStepStatus::Pending,
        risk,
        files_affected,
        entities_affected,
        reversible,
    }
}

fn looks_like_enemy_request(request: &str) -> bool {
    let lower = request.to_lowercase();
    lower.contains("enemy")
        || lower.contains("enemigo")
        || (lower.contains("patrulla") && lower.contains("ataque"))
}

fn plan_id(request: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.hash(&mut hasher);
    format!("forgeai-plan-{:x}", hasher.finish())
}

fn enemy_controller_luau() -> String {
    r#"---@export
local max_health: number = 100

---@export
local patrol_speed: number = 3.75

---@export
local chase_speed: number = 5.25

---@export
local attack_damage: number = 12

---@export
local detection_radius: number = 7.5

---@export
local attack_radius: number = 1.1

---@export
local patrol_a_x: number = -2

---@export
local patrol_b_x: number = 2

local patrol_target_x: number = patrol_b_x
local patrol_target_y: number = 0
local think_timer: number = 0
local state: string = "Patrol"

function on_start()
    set_tag("Enemy")
    set_component_number("Health", "max_health", max_health)
    set_component_number("Health", "health", max_health)
    set_component_number("AIController", "detection_radius", detection_radius)
    set_component_number("AIController", "attack_radius", attack_radius)
    set_component_text("AIController", "state", state)
    set_component_number("NavAgent", "speed", patrol_speed)
    set_blackboard("forge_ai_controller", "Enemy patrol/chase/attack")
end

function on_update(dt: number)
    think_timer += dt
    if think_timer < 0.12 then
        return
    end
    think_timer = 0

    -- Runtime AI system owns target acquisition; this script keeps inspector
    -- state, speed and patrol destination coherent for generated enemies.
    local target_id = Component.get(nil, "AIController", "target_id")
    if target_id ~= nil then
        state = "Chase"
        set_component_number("NavAgent", "speed", chase_speed)
    else
        state = "Patrol"
        set_component_number("NavAgent", "speed", patrol_speed)
        Navigation2D.set_destination(nil, patrol_target_x, patrol_target_y)
        if math.abs((Transform2D.position(nil).x or 0) - patrol_target_x) < 0.25 then
            if patrol_target_x == patrol_b_x then
                patrol_target_x = patrol_a_x
            else
                patrol_target_x = patrol_b_x
            end
        end
    end

    set_component_text("AIController", "state", state)
    set_blackboard("state", state)
end
"#
    .to_string()
}
