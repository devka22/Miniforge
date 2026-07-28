use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::actions::AiAction;
use crate::engine::forge_ai::planner::AiPlan;
use crate::engine::luau_scripting::LuauScriptRuntime;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

impl AiValidationReport {
    pub fn ok() -> Self {
        Self {
            valid: true,
            ..Self::default()
        }
    }

    pub fn push_error(&mut self, message: impl Into<String>) {
        self.valid = false;
        self.errors.push(message.into());
    }

    pub fn merge(&mut self, other: Self) {
        self.valid &= other.valid;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.suggestions.extend(other.suggestions);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MiniForgeLuauApiDoc {
    pub classes: Vec<LuauApiClass>,
    pub globals: Vec<LuauApiSymbol>,
    pub events: Vec<LuauApiSymbol>,
    pub best_practices: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LuauApiClass {
    pub name: String,
    pub methods: Vec<LuauApiSymbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LuauApiSymbol {
    pub name: String,
    pub signature: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct AiValidator;

impl AiValidator {
    pub fn validate_plan(plan: &AiPlan) -> AiValidationReport {
        let mut report = AiValidationReport::ok();
        if plan.steps.is_empty() {
            report.push_error("plan must include visible steps");
        }
        if plan.actions.is_empty() {
            report.push_error("plan must include typed actions");
        }
        let mut action_ids = BTreeSet::new();
        for action in &plan.actions {
            if !action_ids.insert(action.action_id().to_string()) {
                report.push_error(format!("duplicate action id: {}", action.action_id()));
            }
            if let Err(error) = action.validate() {
                report.push_error(error.to_string());
            }
            if let AiAction::CreateLuauScript {
                source,
                relative_path,
                ..
            }
            | AiAction::ModifyLuauScript {
                source,
                relative_path,
                ..
            } = action
            {
                report.merge(Self::validate_luau_source(source, relative_path));
            }
        }
        report
    }

    pub fn validate_luau_source(source: &str, filename: &str) -> AiValidationReport {
        let mut report = AiValidationReport::ok();
        if let Err(error) = LuauScriptRuntime::validate_source(source, filename) {
            report.push_error(format!("Luau parse failed for {filename}: {error}"));
        }
        for forbidden in ["UnityEngine", "game:GetService", "love.", "os.execute"] {
            if source.contains(forbidden) {
                report.push_error(format!(
                    "Luau source references non-MiniForge or unsafe API: {forbidden}"
                ));
            }
        }
        if source.contains("---@export") && !source.contains("local ") {
            report.warnings.push(
                "export annotations should be attached to local variables visible in Inspector"
                    .to_string(),
            );
        }
        report
    }

    pub fn api_doc() -> MiniForgeLuauApiDoc {
        MiniForgeLuauApiDoc {
            globals: vec![
                symbol("set_tag(tag: string)", "Set current entity tag."),
                symbol(
                    "set_layer(layer: string)",
                    "Set current entity physics/render layer.",
                ),
                symbol(
                    "set_component_number(component: string, key: string, value: number)",
                    "Set numeric component property on the current entity.",
                ),
                symbol(
                    "set_component_text(component: string, key: string, value: string)",
                    "Set text component property on the current entity.",
                ),
                symbol(
                    "set_blackboard(key: string, value: any)",
                    "Store runtime AI data.",
                ),
            ],
            classes: vec![
                class(
                    "Component",
                    vec![
                        symbol(
                            "Component.get(target, component: string, key: string)",
                            "Read a component value from an entity target.",
                        ),
                        symbol(
                            "Component.set(target, component: string, key: string, value: any)",
                            "Queue a component value write.",
                        ),
                    ],
                ),
                class(
                    "Navigation2D",
                    vec![symbol(
                        "Navigation2D.set_destination(target, x: number, y: number)",
                        "Set a NavAgent destination.",
                    )],
                ),
                class(
                    "Rigidbody2D",
                    vec![
                        symbol(
                            "Rigidbody2D.set_velocity(target, x: number, y: number)",
                            "Set a 2D body velocity.",
                        ),
                        symbol(
                            "Rigidbody2D.apply_impulse(target, x: number, y: number)",
                            "Apply an impulse to a body.",
                        ),
                        symbol(
                            "Rigidbody2D.apply_force(target, x: number, y: number)",
                            "Accumulate a force for the next physics update.",
                        ),
                        symbol(
                            "Rigidbody2D.apply_torque(target, torque: number)",
                            "Accumulate angular force for the next physics update.",
                        ),
                        symbol("Rigidbody2D.wake(target)", "Wake a sleeping dynamic body."),
                        symbol(
                            "Rigidbody2D.sleep(target)",
                            "Stop and explicitly sleep a dynamic body.",
                        ),
                    ],
                ),
                class(
                    "Transform2D",
                    vec![
                        symbol(
                            "Transform2D.position(target)",
                            "Return a table with x/y position.",
                        ),
                        symbol(
                            "Transform2D.translate(target, dx: number, dy: number)",
                            "Move an entity relative to its current position.",
                        ),
                    ],
                ),
            ],
            events: vec![
                symbol("on_start()", "Called when the script starts."),
                symbol("on_update(dt: number)", "Called every runtime update."),
                symbol(
                    "on_fixed_update(dt: number)",
                    "Called during fixed simulation ticks.",
                ),
                symbol(
                    "on_collision_enter(other: string)",
                    "Called when a collision begins.",
                ),
            ],
            best_practices: vec![
                "Expose tunables with ---@export local variables.".to_string(),
                "Prefer component writes over direct storage mutation.".to_string(),
                "Keep expensive searches out of on_update; cache ids in Blackboard.".to_string(),
            ],
            limitations: vec![
                "Generated code runs in a sandboxed Luau VM with memory/time limits.".to_string(),
                "Scripts enqueue engine commands; commands apply after callbacks return."
                    .to_string(),
            ],
        }
    }
}

fn class(name: &str, methods: Vec<LuauApiSymbol>) -> LuauApiClass {
    LuauApiClass {
        name: name.to_string(),
        methods,
    }
}

fn symbol(signature: &str, description: &str) -> LuauApiSymbol {
    let name = signature
        .split(['(', ':'])
        .next()
        .unwrap_or(signature)
        .to_string();
    LuauApiSymbol {
        name,
        signature: signature.to_string(),
        description: description.to_string(),
    }
}
