use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};

pub const AUTHORING_CATALOG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringPresetKind2D {
    Actor,
    Gameplay,
    Physics,
    World,
    Effects,
    UserInterface,
    Strategy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringPresetMaturity2D {
    Production,
    Advanced,
    Experimental,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetParameterBinding2D {
    pub component: String,
    pub property: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetParameter2D {
    pub id: String,
    pub label: String,
    pub value_type: String,
    pub default_value: Value,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub description: String,
    pub bindings: Vec<PresetParameterBinding2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentOverride2D {
    pub component: String,
    pub property: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicsWorldProfile2D {
    pub gravity: [f64; 2],
    pub solver_iterations: usize,
    pub fixed_hz: u32,
    pub max_substeps: usize,
    pub continuous_collision: bool,
    pub sleeping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthoringPreset2D {
    pub id: String,
    pub label: String,
    pub summary: String,
    pub category: String,
    pub kind: AuthoringPresetKind2D,
    pub maturity: AuthoringPresetMaturity2D,
    pub aliases: Vec<String>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub components: Vec<String>,
    pub overrides: Vec<ComponentOverride2D>,
    pub parameters: Vec<PresetParameter2D>,
    pub requirements: Vec<String>,
    pub workflow_steps: Vec<String>,
    pub recommended_next: Vec<String>,
    pub estimated_setup_minutes: u32,
    pub physics_world: Option<PhysicsWorldProfile2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthoringCatalog2D {
    pub schema_version: u32,
    pub presets: Vec<AuthoringPreset2D>,
    pub categories: Vec<String>,
    pub kinds: BTreeMap<String, usize>,
    pub total_components_referenced: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthoringCatalogIssue2D {
    pub preset_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthoringCatalogValidation2D {
    pub valid: bool,
    pub issues: Vec<AuthoringCatalogIssue2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthoringApplicationPlan2D {
    pub preset_id: String,
    pub label: String,
    pub add_components: Vec<String>,
    pub existing_components: Vec<String>,
    pub configured_components: Vec<Component>,
    pub requirements: Vec<String>,
    pub workflow_steps: Vec<String>,
    pub recommended_next: Vec<String>,
    pub physics_world: Option<PhysicsWorldProfile2D>,
}

impl AuthoringCatalog2D {
    pub fn builtin() -> Self {
        let mut presets = builtin_presets();
        presets.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then_with(|| left.label.cmp(&right.label))
        });
        let categories = presets
            .iter()
            .map(|preset| preset.category.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut kinds = BTreeMap::new();
        for preset in &presets {
            *kinds.entry(kind_name(preset.kind).to_string()).or_insert(0) += 1;
        }
        let total_components_referenced = presets
            .iter()
            .flat_map(|preset| preset.components.iter())
            .collect::<BTreeSet<_>>()
            .len();
        Self {
            schema_version: AUTHORING_CATALOG_SCHEMA_VERSION,
            presets,
            categories,
            kinds,
            total_components_referenced,
        }
    }

    pub fn resolve(&self, id_or_alias: &str) -> Option<&AuthoringPreset2D> {
        let needle = normalize_id(id_or_alias);
        self.presets.iter().find(|preset| {
            normalize_id(&preset.id) == needle
                || preset
                    .aliases
                    .iter()
                    .any(|alias| normalize_id(alias) == needle)
        })
    }

    pub fn search(
        &self,
        query: &str,
        kind: Option<AuthoringPresetKind2D>,
        limit: usize,
    ) -> Vec<&AuthoringPreset2D> {
        let tokens = query
            .split_whitespace()
            .map(|token| token.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let mut matches = self
            .presets
            .iter()
            .filter(|preset| kind.is_none_or(|expected| preset.kind == expected))
            .filter_map(|preset| {
                let haystack = format!(
                    "{} {} {} {} {} {}",
                    preset.id,
                    preset.label,
                    preset.summary,
                    preset.category,
                    preset.tags.join(" "),
                    preset.genres.join(" ")
                )
                .to_ascii_lowercase();
                if tokens.iter().any(|token| !haystack.contains(token)) {
                    return None;
                }
                let score = tokens.iter().fold(0usize, |score, token| {
                    score
                        + usize::from(preset.id.contains(token)) * 12
                        + usize::from(preset.label.to_ascii_lowercase().contains(token)) * 8
                        + usize::from(preset.tags.iter().any(|tag| tag.contains(token))) * 4
                });
                Some((score, preset))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.label.cmp(&right.label))
        });
        matches
            .into_iter()
            .take(limit)
            .map(|(_, preset)| preset)
            .collect()
    }

    pub fn configure_component(
        &self,
        preset_id: &str,
        component: &mut Component,
        parameters: Option<&Value>,
    ) -> bool {
        let Some(preset) = self.resolve(preset_id) else {
            return false;
        };
        let component_type = component.component_type.clone();
        let mut changed = false;
        for item in preset
            .overrides
            .iter()
            .filter(|item| item.component == component_type)
        {
            if item.property == "enabled" {
                component.enabled = item.value.as_bool().unwrap_or(component.enabled);
            } else {
                component.set(&item.property, item.value.clone());
            }
            changed = true;
        }
        let parameters = parameters.and_then(Value::as_object);
        for parameter in &preset.parameters {
            let Some(value) = parameters
                .and_then(|values| values.get(&parameter.id))
                .cloned()
            else {
                continue;
            };
            let value = sanitize_parameter_value(parameter, value);
            for binding in parameter
                .bindings
                .iter()
                .filter(|binding| binding.component == component_type)
            {
                if binding.property == "enabled" {
                    component.enabled = value.as_bool().unwrap_or(component.enabled);
                } else {
                    component.set(&binding.property, value.clone());
                }
                changed = true;
            }
        }
        changed
    }

    pub fn application_plan(
        &self,
        preset_id: &str,
        existing_components: impl IntoIterator<Item = impl AsRef<str>>,
        parameters: Option<&Value>,
    ) -> Option<AuthoringApplicationPlan2D> {
        let preset = self.resolve(preset_id)?;
        let existing = existing_components
            .into_iter()
            .map(|component| component.as_ref().to_string())
            .collect::<BTreeSet<_>>();
        let mut add_components = Vec::new();
        let mut existing_components = Vec::new();
        let mut configured_components = Vec::new();
        for component_type in &preset.components {
            if existing.contains(component_type) {
                existing_components.push(component_type.clone());
                continue;
            }
            let mut component = default_component(component_type)?;
            self.configure_component(&preset.id, &mut component, parameters);
            add_components.push(component_type.clone());
            configured_components.push(component);
        }
        Some(AuthoringApplicationPlan2D {
            preset_id: preset.id.clone(),
            label: preset.label.clone(),
            add_components,
            existing_components,
            configured_components,
            requirements: preset.requirements.clone(),
            workflow_steps: preset.workflow_steps.clone(),
            recommended_next: preset.recommended_next.clone(),
            physics_world: preset.physics_world.clone(),
        })
    }

    pub fn validate(&self) -> AuthoringCatalogValidation2D {
        let mut issues = Vec::new();
        let mut ids = BTreeSet::new();
        let mut names = BTreeMap::<String, String>::new();
        for preset in &self.presets {
            if !ids.insert(preset.id.clone()) {
                issue(&mut issues, preset, "duplicate preset id");
            }
            for name in std::iter::once(&preset.id).chain(preset.aliases.iter()) {
                let normalized = normalize_id(name);
                if let Some(owner) = names.insert(normalized, preset.id.clone())
                    && owner != preset.id
                {
                    issue(
                        &mut issues,
                        preset,
                        format!("id or alias collides with {owner}"),
                    );
                }
            }
            if preset.components.is_empty() {
                issue(&mut issues, preset, "preset has no components");
            }
            for component in &preset.components {
                if default_component(component).is_none() {
                    issue(
                        &mut issues,
                        preset,
                        format!("unknown component {component}"),
                    );
                }
            }
            for item in &preset.overrides {
                if !preset
                    .components
                    .iter()
                    .any(|component| component == &item.component)
                {
                    issue(
                        &mut issues,
                        preset,
                        format!(
                            "override targets component not in preset: {}",
                            item.component
                        ),
                    );
                }
            }
            for parameter in &preset.parameters {
                for binding in &parameter.bindings {
                    if !preset
                        .components
                        .iter()
                        .any(|component| component == &binding.component)
                    {
                        issue(
                            &mut issues,
                            preset,
                            format!(
                                "parameter {} targets component not in preset: {}",
                                parameter.id, binding.component
                            ),
                        );
                    }
                }
            }
        }
        AuthoringCatalogValidation2D {
            valid: issues.is_empty(),
            issues,
        }
    }
}

fn issue(
    issues: &mut Vec<AuthoringCatalogIssue2D>,
    preset: &AuthoringPreset2D,
    message: impl Into<String>,
) {
    issues.push(AuthoringCatalogIssue2D {
        preset_id: preset.id.clone(),
        message: message.into(),
    });
}

fn sanitize_parameter_value(parameter: &PresetParameter2D, value: Value) -> Value {
    if parameter.value_type != "number" && parameter.value_type != "integer" {
        return value;
    }
    let Some(mut number) = value.as_f64() else {
        return parameter.default_value.clone();
    };
    if !number.is_finite() {
        return parameter.default_value.clone();
    }
    if let Some(minimum) = parameter.minimum {
        number = number.max(minimum);
    }
    if let Some(maximum) = parameter.maximum {
        number = number.min(maximum);
    }
    if parameter.value_type == "integer" {
        json!(number.round() as i64)
    } else {
        json!(number)
    }
}

fn normalize_id(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

fn kind_name(kind: AuthoringPresetKind2D) -> &'static str {
    match kind {
        AuthoringPresetKind2D::Actor => "actor",
        AuthoringPresetKind2D::Gameplay => "gameplay",
        AuthoringPresetKind2D::Physics => "physics",
        AuthoringPresetKind2D::World => "world",
        AuthoringPresetKind2D::Effects => "effects",
        AuthoringPresetKind2D::UserInterface => "user_interface",
        AuthoringPresetKind2D::Strategy => "strategy",
    }
}

fn builtin_presets() -> Vec<AuthoringPreset2D> {
    vec![
        preset(
            "topdown_player",
            "Top-down Player",
            "Players",
            AuthoringPresetKind2D::Actor,
            "Responsive gravity-free movement, input, camera, animation, health and persistence.",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Animator2D",
                "Health",
                "Saveable",
            ],
            &[
                ov("Pawn2D", "movement_mode", json!("topdown")),
                ov("CharacterController2D", "mode", json!("topdown")),
                ov("CharacterController2D", "jump_force", json!(0.0)),
                ov("CharacterController2D", "max_jumps", json!(0)),
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "gravity_scale", json!(0.0)),
                ov("Rigidbody2D", "freeze_rotation", json!(true)),
                ov("PlayerController2D", "cursor_visible", json!(false)),
            ],
            &["top-down", "adventure", "action"],
            &["player", "movement", "camera"],
        )
        .aliases(&["topdown"]),
        preset(
            "platformer_player",
            "Platformer Player",
            "Players",
            AuthoringPresetKind2D::Actor,
            "Character body with coyote time, jump buffering, collisions, camera and checkpoints.",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterBody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Animator2D",
                "Health",
                "Checkpoint",
            ],
            &[
                ov("Pawn2D", "movement_mode", json!("platformer")),
                ov("CharacterBody2D", "mode", json!("platformer")),
                ov("PlayerController2D", "cursor_visible", json!(false)),
            ],
            &["platformer", "metroidvania"],
            &["player", "jump", "character-body"],
        )
        .aliases(&["platformer"]),
        preset(
            "twin_stick_player",
            "Twin-stick Player",
            "Players",
            AuthoringPresetKind2D::Actor,
            "Aim-independent top-down controller with combat, abilities and camera support.",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Animator2D",
                "Health",
                "DamageDealer",
                "Ability",
                "Saveable",
            ],
            &[
                ov("Pawn2D", "movement_mode", json!("topdown")),
                ov("CharacterController2D", "mode", json!("topdown")),
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "freeze_rotation", json!(true)),
                ov("PlayerController2D", "cursor_visible", json!(true)),
            ],
            &["shooter", "arena", "action"],
            &["player", "aim", "combat"],
        ),
        preset(
            "action_rpg_hero",
            "Action RPG Hero",
            "Players",
            AuthoringPresetKind2D::Gameplay,
            "Combat, abilities, inventory, equipment, quests, status effects and persistence.",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Health",
                "Stats",
                "DamageDealer",
                "StatusEffects",
                "Inventory",
                "Equipment",
                "Ability",
                "QuestLog",
                "Saveable",
            ],
            &[
                ov("Pawn2D", "movement_mode", json!("topdown")),
                ov("CharacterController2D", "mode", json!("topdown")),
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "freeze_rotation", json!(true)),
                ov("PlayerController2D", "cursor_visible", json!(false)),
            ],
            &["action-rpg", "roguelite", "adventure"],
            &["player", "rpg", "inventory", "abilities"],
        )
        .aliases(&["action_rpg"]),
        preset(
            "roguelike_hero",
            "Roguelike Hero",
            "Players",
            AuthoringPresetKind2D::Gameplay,
            "Fast top-down hero with procedural-run inventory, combat and save checkpoints.",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Health",
                "Stats",
                "DamageDealer",
                "StatusEffects",
                "Inventory",
                "Equipment",
                "Ability",
                "Checkpoint",
                "Saveable",
            ],
            &[
                ov("CharacterController2D", "mode", json!("topdown")),
                ov("CharacterController2D", "dash_speed", json!(15.0)),
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Inventory", "capacity", json!(12)),
            ],
            &["roguelike", "roguelite", "dungeon"],
            &["player", "procedural", "combat"],
        ),
        preset(
            "stealth_agent",
            "Stealth Agent",
            "Players",
            AuthoringPresetKind2D::Gameplay,
            "Top-down agent with state machine, interaction, quiet movement and persistence.",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "CharacterController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "StateMachine",
                "Stats",
                "Health",
                "Interaction",
                "Saveable",
            ],
            &[
                ov("CharacterController2D", "mode", json!("topdown")),
                ov("CharacterController2D", "walk_speed", json!(3.5)),
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "drag", json!(0.18)),
            ],
            &["stealth", "immersive-sim", "adventure"],
            &["player", "stealth", "interaction"],
        ),
        preset(
            "space_pilot",
            "Space Pilot",
            "Players",
            AuthoringPresetKind2D::Actor,
            "Zero-gravity movement with free rotation, combat, abilities and camera tracking.",
            &[
                "Actor2D",
                "Pawn2D",
                "PlayerController2D",
                "Rigidbody2D",
                "Collider2D",
                "InputActions2D",
                "CameraFollow",
                "Health",
                "Stats",
                "DamageDealer",
                "Ability",
                "Saveable",
            ],
            &[
                ov("Pawn2D", "movement_mode", json!("topdown")),
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "freeze_rotation", json!(false)),
                ov("Rigidbody2D", "drag", json!(0.015)),
                ov("Rigidbody2D", "angular_drag", json!(0.08)),
            ],
            &["space", "shooter", "simulation"],
            &["player", "zero-gravity", "physics"],
        ),
        preset(
            "survival_actor",
            "Survival Actor",
            "Survival",
            AuthoringPresetKind2D::Gameplay,
            "Health, needs, weighted inventory, equipment, crafting, effects and save data.",
            &[
                "Health",
                "SurvivalNeeds",
                "BodyCondition2D",
                "Inventory",
                "Equipment",
                "CraftingBook",
                "StatusEffects",
                "Saveable",
            ],
            &[],
            &["survival", "sandbox", "simulation"],
            &["needs", "inventory", "crafting"],
        )
        .aliases(&["survival"]),
        preset(
            "survival_environment",
            "Survival Environment 2D",
            "Survival",
            AuthoringPresetKind2D::World,
            "Weather and exposure sample for temperature, wind, rain, shelter, air quality and infection pressure.",
            &["SurvivalEnvironment2D", "Saveable"],
            &[],
            &["survival", "horror", "simulation", "open-world"],
            &["weather", "temperature", "exposure", "survival"],
        )
        .aliases(&["survival_weather", "exposure_zone"]),
        preset(
            "survival_loadout",
            "Advanced Survival Loadout",
            "Survival",
            AuthoringPresetKind2D::Gameplay,
            "Weighted inventory, atomic multi-slot equipment, durability, protection and body condition with no custom code.",
            &["Inventory", "Equipment", "Stats", "BodyCondition2D", "Saveable"],
            &[
                ov("Inventory", "capacity", json!(32)),
                ov("Inventory", "max_weight", json!(35.0)),
            ],
            &["survival", "rpg", "action"],
            &["inventory", "equipment", "durability", "injuries", "loadout"],
        )
        .aliases(&["equipment_loadout", "advanced_inventory"]),
        preset(
            "enemy_ai",
            "Enemy AI",
            "Artificial Intelligence",
            AuthoringPresetKind2D::Actor,
            "Behavior tree, blackboard, navigation, combat, status effects and loot.",
            &[
                "Actor2D",
                "AIController2D",
                "BehaviorTree2D",
                "Blackboard",
                "NavAgent",
                "Collider2D",
                "Health",
                "Stats",
                "DamageDealer",
                "CombatTarget",
                "StatusEffects",
                "LootTable",
            ],
            &[],
            &["action", "rpg", "strategy", "survival"],
            &["enemy", "ai", "navigation", "combat"],
        )
        .aliases(&["enemy"]),
        preset(
            "boss_enemy",
            "Boss Enemy",
            "Artificial Intelligence",
            AuthoringPresetKind2D::Actor,
            "Multi-phase enemy with advanced AI, abilities, effects, loot and persistence.",
            &[
                "Actor2D",
                "AIController2D",
                "BehaviorTree2D",
                "Blackboard",
                "NavAgent",
                "Collider2D",
                "Health",
                "Stats",
                "DamageDealer",
                "CombatTarget",
                "StatusEffects",
                "StateMachine",
                "Ability",
                "LootTable",
                "ParticleEmitter",
                "Saveable",
            ],
            &[
                ov("Health", "max_health", json!(1000.0)),
                ov("Health", "health", json!(1000.0)),
                ov("Stats", "level", json!(10)),
            ],
            &["action", "rpg", "raid"],
            &["boss", "ai", "phases", "combat"],
        ),
        preset(
            "companion_ai",
            "Companion AI",
            "Artificial Intelligence",
            AuthoringPresetKind2D::Actor,
            "Friendly follower with navigation, dialogue, combat targeting and persistence.",
            &[
                "Actor2D",
                "AIController2D",
                "BehaviorTree2D",
                "Blackboard",
                "NavAgent",
                "Collider2D",
                "Health",
                "Stats",
                "CombatTarget",
                "Dialogue",
                "Saveable",
            ],
            &[],
            &["rpg", "adventure", "survival"],
            &["companion", "follower", "ai"],
        ),
        preset(
            "city_npc",
            "Living City NPC",
            "Artificial Intelligence",
            AuthoringPresetKind2D::Actor,
            "Schedule-ready civilian with navigation, dialogue, interaction and state memory.",
            &[
                "Actor2D",
                "AIController2D",
                "BehaviorTree2D",
                "Blackboard",
                "NavAgent",
                "Collider2D",
                "Interaction",
                "Dialogue",
                "StateMachine",
                "Saveable",
            ],
            &[],
            &["simulation", "open-world", "city-builder"],
            &["npc", "schedule", "dialogue"],
        ),
        preset(
            "dialogue_npc",
            "Dialogue NPC",
            "Narrative",
            AuthoringPresetKind2D::Gameplay,
            "Interaction, branching dialogue, objective marker and persistence.",
            &[
                "Actor2D",
                "Interaction",
                "Dialogue",
                "ObjectiveMarker",
                "Saveable",
            ],
            &[],
            &["rpg", "adventure", "visual-novel"],
            &["npc", "dialogue", "narrative"],
        )
        .aliases(&["npc"]),
        preset(
            "quest_giver",
            "Quest Giver",
            "Narrative",
            AuthoringPresetKind2D::Gameplay,
            "Dialogue NPC with quest log integration, objective marker and persistent state.",
            &[
                "Actor2D",
                "Interaction",
                "Dialogue",
                "QuestLog",
                "ObjectiveMarker",
                "Saveable",
            ],
            &[],
            &["rpg", "adventure", "mmo"],
            &["npc", "quest", "dialogue"],
        ),
        preset(
            "economy_vendor",
            "Economy Vendor",
            "Narrative",
            AuthoringPresetKind2D::Gameplay,
            "Interactive vendor with inventory, wallet, dialogue and persistent stock.",
            &[
                "Actor2D",
                "Interaction",
                "Dialogue",
                "Inventory",
                "EconomyWallet",
                "Saveable",
            ],
            &[],
            &["rpg", "simulation", "survival"],
            &["vendor", "economy", "inventory"],
        ),
        preset(
            "collectible",
            "Collectible",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Trigger, interaction, loot, feedback particles and persistence.",
            &[
                "Area2D",
                "Trigger2D",
                "Interaction",
                "LootTable",
                "ParticleEmitter",
                "Saveable",
            ],
            &[ov("Interaction", "prompt", json!("Pick up"))],
            &["all"],
            &["pickup", "loot", "feedback"],
        )
        .aliases(&["pickup"]),
        preset(
            "loot_container",
            "Loot Container",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Searchable weighted container with interaction and persistence.",
            &["LootContainer", "Interaction", "Saveable"],
            &[],
            &["rpg", "survival", "adventure"],
            &["loot", "container", "inventory"],
        )
        .aliases(&["lootable"]),
        preset(
            "harvestable",
            "Harvestable Resource",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Reusable gathering node with tool checks, interaction and persistence.",
            &["Harvestable", "Interaction", "Saveable"],
            &[],
            &["survival", "crafting", "simulation"],
            &["resource", "gathering", "crafting"],
        ),
        preset(
            "crafting_station",
            "Crafting Station",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Data-driven recipes, station tags, interaction and persistent power state.",
            &["CraftingStation", "Interaction", "Saveable"],
            &[],
            &["survival", "rpg", "simulation"],
            &["crafting", "recipes", "station"],
        ),
        preset(
            "inventory",
            "Inventory",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Inventory and equipment usable without custom code.",
            &["Inventory", "Equipment"],
            &[],
            &["all"],
            &["inventory", "equipment", "items"],
        ),
        preset(
            "combat_actor",
            "Combat Actor",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Health, attributes, damage and status effects.",
            &["Health", "Stats", "DamageDealer", "StatusEffects"],
            &[],
            &["action", "rpg", "strategy"],
            &["combat", "health", "damage"],
        )
        .aliases(&["combat"]),
        preset(
            "projectile",
            "Projectile",
            "Gameplay Objects",
            AuthoringPresetKind2D::Physics,
            "Continuous-collision projectile with damage, lifetime and impact feedback.",
            &[
                "Rigidbody2D",
                "Collider2D",
                "DamageDealer",
                "Lifetime",
                "ParticleEmitter",
            ],
            &[
                ov("Rigidbody2D", "continuous_collision", json!(true)),
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Collider2D", "shape", json!("circle")),
                ov("DamageDealer", "hit_once", json!(true)),
            ],
            &["action", "shooter", "rpg"],
            &["projectile", "ccd", "damage"],
        ),
        preset(
            "destructible_prop",
            "Destructible Prop",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Damageable prop with destruction effect, particles, loot and save state.",
            &[
                "Collider2D",
                "Health",
                "DamageEffect2D",
                "ParticleEmitter",
                "LootTable",
                "Saveable",
            ],
            &[],
            &["all"],
            &["destructible", "damage", "loot"],
        ),
        preset(
            "interactive_door",
            "Interactive Door",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Collision, interaction, state machine, audio and persistent open/locked state.",
            &[
                "StaticBody2D",
                "Collider2D",
                "Interaction",
                "StateMachine",
                "AudioSource2D",
                "Saveable",
            ],
            &[],
            &["all"],
            &["door", "interaction", "state"],
        ),
        preset(
            "checkpoint_spawn",
            "Checkpoint & Spawn",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Triggerable checkpoint with respawn data, particles, audio and persistence.",
            &[
                "Area2D",
                "Trigger2D",
                "Checkpoint",
                "ParticleEmitter",
                "AudioSource2D",
                "Saveable",
            ],
            &[],
            &["platformer", "action", "adventure"],
            &["checkpoint", "respawn", "save"],
        ),
        preset(
            "puzzle_actor",
            "Puzzle Actor",
            "Gameplay Objects",
            AuthoringPresetKind2D::Gameplay,
            "Interactive state machine with events, tweened feedback, audio and save state.",
            &[
                "Actor2D",
                "Interaction",
                "StateMachine",
                "EventBus2D",
                "Tween",
                "AudioSource2D",
                "Saveable",
            ],
            &[],
            &["puzzle", "adventure", "immersive-sim"],
            &["puzzle", "state", "events"],
        ),
        preset(
            "moving_platform",
            "Moving Platform",
            "Platforming",
            AuthoringPresetKind2D::Physics,
            "Kinematic platform with collision and a reusable tween path.",
            &["KinematicBody2D", "Collider2D", "Tween"],
            &[
                ov("Tween", "property_path", json!("x")),
                ov("Tween", "loop", json!(true)),
                ov("Tween", "ping_pong", json!(true)),
                ov("Tween", "active", json!(true)),
            ],
            &["platformer"],
            &["platform", "kinematic", "tween"],
        ),
        preset(
            "platformer_hazard",
            "Platformer Hazard",
            "Platforming",
            AuthoringPresetKind2D::Physics,
            "Trigger area with damage and visible particle feedback.",
            &["Area2D", "Trigger2D", "DamageDealer", "ParticleEmitter"],
            &[ov("DamageDealer", "damage", json!(25.0))],
            &["platformer", "action"],
            &["hazard", "trigger", "damage"],
        ),
        preset(
            "camera_rig",
            "Camera Rig",
            "Presentation",
            AuthoringPresetKind2D::Actor,
            "Active pixel-perfect camera follow with reusable screen shake.",
            &["Camera2D", "CameraFollow", "CameraShake"],
            &[
                ov("Camera2D", "active", json!(true)),
                ov("Camera2D", "pixel_perfect", json!(true)),
            ],
            &["all"],
            &["camera", "follow", "shake"],
        )
        .aliases(&["camera"]),
        preset(
            "render_target_camera",
            "Camera to Texture 2D",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Off-screen WGPU camera target that can be sampled by sprites for monitors, portals, minimaps and composition.",
            &["Camera2D", "RenderTexture2D"],
            &[
                ov("Camera2D", "active", json!(false)),
                ov("Camera2D", "render_target_update_mode", json!("always")),
                ov("RenderTexture2D", "format", json!("rgba8_srgb")),
                ov("RenderTexture2D", "update_mode", json!("always")),
            ],
            &["all"],
            &["camera", "render-target", "portal", "minimap", "wgpu"],
        )
        .aliases(&["camera_texture", "render_texture_camera"]),
        preset(
            "hybrid_world_2d3d",
            "Hybrid 2D + 3D World",
            "Presentation",
            AuthoringPresetKind2D::World,
            "A 3D camera, light and depth world driven by authoritative 2D gameplay coordinates.",
            &["HybridScene3D", "Camera3D", "Light3D"],
            &[
                ov("HybridScene3D", "enabled", json!(true)),
                ov("HybridScene3D", "physics_mode", json!("2d_gameplay")),
                ov("HybridScene3D", "render_2d_overlay", json!(true)),
                ov("Camera3D", "active", json!(true)),
                ov("Camera3D", "projection", json!("perspective")),
                ov("Light3D", "light_type", json!("directional")),
            ],
            &["all", "survival", "rpg", "strategy"],
            &["hybrid", "2d", "3d", "billboard", "depth"],
        )
        .aliases(&["hybrid_world", "2d3d_world", "two_point_five_d"]),
        preset(
            "hybrid_billboard_actor",
            "Hybrid Billboard Actor",
            "Presentation",
            AuthoringPresetKind2D::Actor,
            "Animated 2D actor synchronized into 3D X/Z space while retaining 2D collision and navigation.",
            &[
                "Actor2D",
                "HybridAnchor2D3D",
                "Billboard3D",
                "Animator2D",
                "Collider2D",
            ],
            &[
                ov("HybridAnchor2D3D", "sync_mode", json!("from_2d")),
                ov("Billboard3D", "face_camera", json!(true)),
                ov("Billboard3D", "lock_y_axis", json!(true)),
                ov("Billboard3D", "use_2d_animation", json!(true)),
            ],
            &["all", "survival", "rpg", "action"],
            &["hybrid", "billboard", "actor", "animation", "2d", "3d"],
        )
        .aliases(&["billboard_actor", "sprite3d_actor"]),
        preset(
            "post_process_cinematic",
            "Cinematic Post Process 2D",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Global WGPU color grade with restrained bloom and vignette, ready without shader code.",
            &["PostProcessVolume2D"],
            &[
                ov("PostProcessVolume2D", "preset", json!("cinematic")),
                ov("PostProcessVolume2D", "contrast", json!(1.08)),
                ov("PostProcessVolume2D", "bloom_intensity", json!(0.35)),
                ov("PostProcessVolume2D", "vignette_intensity", json!(0.22)),
            ],
            &["all", "adventure", "narrative"],
            &["post-process", "color-grading", "bloom", "wgpu"],
        )
        .aliases(&["post_fx", "cinematic_grade"]),
        preset(
            "post_process_horror",
            "Horror Survival Grade 2D",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Cold desaturated survival-horror grade with dense edge falloff and subtle lens separation.",
            &["PostProcessVolume2D"],
            &[
                ov("PostProcessVolume2D", "preset", json!("horror_survival")),
                ov("PostProcessVolume2D", "contrast", json!(1.18)),
                ov("PostProcessVolume2D", "saturation", json!(0.68)),
                ov("PostProcessVolume2D", "gamma", json!(0.92)),
                ov("PostProcessVolume2D", "bloom_intensity", json!(0.18)),
                ov("PostProcessVolume2D", "vignette_intensity", json!(0.52)),
                ov("PostProcessVolume2D", "chromatic_aberration", json!(0.003)),
                ov("PostProcessVolume2D", "tint", json!([215, 228, 245, 255])),
                ov("PostProcessVolume2D", "fog_density", json!(0.08)),
                ov("PostProcessVolume2D", "fog_color", json!([58, 72, 88, 255])),
            ],
            &["survival", "horror", "stealth"],
            &["post-process", "horror", "fog", "vignette", "wgpu"],
        )
        .aliases(&["horror_grade", "survival_grade"]),
        preset(
            "post_process_pixel",
            "Pixel Presentation 2D",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Crisp low-resolution presentation with controllable pixel blocks and subtle scanlines.",
            &["PostProcessVolume2D"],
            &[
                ov("PostProcessVolume2D", "preset", json!("pixel_crisp")),
                ov("PostProcessVolume2D", "saturation", json!(1.08)),
                ov("PostProcessVolume2D", "pixel_size", json!(3.0)),
                ov("PostProcessVolume2D", "scanline_intensity", json!(0.08)),
                ov("PostProcessVolume2D", "vignette_intensity", json!(0.1)),
                ov("PostProcessVolume2D", "bloom_intensity", json!(0.08)),
            ],
            &["pixel-art", "retro", "all"],
            &["post-process", "pixel", "scanlines", "wgpu"],
        )
        .aliases(&["pixel_grade", "retro_grade"]),
        preset(
            "post_process_damage",
            "Damage Impact Screen 2D",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Reusable full-screen damage feedback with red flash, lens separation and vignette pressure.",
            &["PostProcessVolume2D"],
            &[
                ov("PostProcessVolume2D", "preset", json!("damage_impact")),
                ov("PostProcessVolume2D", "damage_strength", json!(0.65)),
                ov("PostProcessVolume2D", "chromatic_aberration", json!(0.012)),
                ov("PostProcessVolume2D", "vignette_intensity", json!(0.62)),
                ov("PostProcessVolume2D", "bloom_intensity", json!(0.12)),
            ],
            &["action", "survival", "horror"],
            &["post-process", "damage", "feedback", "wgpu"],
        )
        .aliases(&["damage_screen", "hit_post_fx"]),
        preset(
            "audio_emitter",
            "Spatial Audio Emitter",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Spatial 2D audio ready for a SoundCue or AudioEvent.",
            &["AudioSource2D"],
            &[ov("AudioSource2D", "spatial_blend", json!(1.0))],
            &["all"],
            &["audio", "spatial", "sfx"],
        )
        .aliases(&["audio"]),
        preset(
            "gpu_particle_emitter",
            "GPU Particle Emitter",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Persistent WGPU compute particles with additive rendering and an automatic CPU fallback.",
            &["GpuParticles2D", "ParticleEmitter"],
            &[
                ov("GpuParticles2D", "simulation", json!("compute")),
                ov("GpuParticles2D", "fallback", json!("cpu_emitter")),
                ov("GpuParticles2D", "blend_mode", json!("additive")),
                ov("ParticleEmitter", "blend_mode", json!("additive")),
            ],
            &["all"],
            &["particles", "gpu", "compute", "effects"],
        )
        .aliases(&["gpu_particles", "compute_particles"]),
        preset(
            "weather_system",
            "Dynamic Weather",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Particles, fog, lighting, ambience and state transitions for weather.",
            &[
                "ParticleEmitter",
                "Fog2D",
                "Light2D",
                "AudioSource2D",
                "StateMachine",
            ],
            &[
                ov("AudioSource2D", "loop", json!(true)),
                ov("AudioSource2D", "bus", json!("Ambience")),
            ],
            &["survival", "simulation", "open-world"],
            &["weather", "particles", "ambience"],
        ),
        preset(
            "parallax_background",
            "Parallax Background",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Layered sprite background with parallax and material control.",
            &["SpriteRenderer", "ParallaxLayer", "Material2D"],
            &[],
            &["all"],
            &["background", "parallax", "rendering"],
        ),
        preset(
            "lit_sprite",
            "WGPU Lit Sprite",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Sprite material with tangent-space normal mapping and automatic point or directional light selection.",
            &["SpriteRenderer", "Material2D", "NormalMap2D"],
            &[
                ov("Material2D", "lighting", json!(true)),
                ov("NormalMap2D", "strength", json!(1.0)),
            ],
            &["all"],
            &["lighting", "normal-map", "sprite", "wgpu"],
        )
        .aliases(&["normal_mapped_sprite", "normal_sprite"]),
        preset(
            "lighting_rig",
            "2D Lighting Rig",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Light, shadow caster, normal map and bloom authoring stack.",
            &["Light2D", "ShadowCaster2D", "NormalMap2D", "Bloom2D"],
            &[],
            &["horror", "action", "adventure"],
            &["lighting", "shadows", "bloom"],
        ),
        preset(
            "cinematic_actor",
            "Cinematic Actor",
            "Presentation",
            AuthoringPresetKind2D::Effects,
            "Sequencer, animation, audio and camera shake for authored scenes.",
            &["Sequencer2D", "Animator2D", "AudioSource2D", "CameraShake"],
            &[],
            &["all"],
            &["cinematic", "sequencer", "animation"],
        ),
        preset(
            "hud_root",
            "HUD Root",
            "User Interface",
            AuthoringPresetKind2D::UserInterface,
            "Responsive widget canvas with runtime stat binding and event routing.",
            &["WidgetCanvas2D", "SurvivalUIBinding", "EventBus2D"],
            &[],
            &["all"],
            &["ui", "hud", "binding"],
        ),
        preset(
            "world_streamer",
            "Open-world Streamer",
            "World Building",
            AuthoringPresetKind2D::World,
            "World partition, chunk streaming, budgets, pools, spawn director and save shards.",
            &[
                "WorldPartition2D",
                "StreamingChunk2D",
                "RuntimeBudget2D",
                "ObjectPool2D",
                "SpawnDirector2D",
                "SaveShard2D",
            ],
            &[],
            &["open-world", "survival", "simulation"],
            &["streaming", "performance", "world"],
        ),
        preset(
            "procedural_spawner",
            "Procedural Spawn Director",
            "World Building",
            AuthoringPresetKind2D::World,
            "Budget-aware spawning backed by object pools and reusable spawn rules.",
            &[
                "SpawnDirector2D",
                "ObjectPool2D",
                "RuntimeBudget2D",
                "Spawner",
            ],
            &[],
            &["roguelike", "survival", "sandbox"],
            &["spawning", "pooling", "procedural"],
        ),
        preset(
            "rts_unit",
            "RTS Unit",
            "Strategy",
            AuthoringPresetKind2D::Strategy,
            "Commandable squad member with navigation, combat, vision and health.",
            &[
                "Actor2D",
                "Commandable",
                "SquadMember",
                "RtsBrain",
                "NavAgent",
                "Collider2D",
                "Stats",
                "Health",
                "DamageDealer",
                "Vision",
            ],
            &[],
            &["rts", "tactics"],
            &["rts", "unit", "squad"],
        ),
        preset(
            "rts_structure",
            "RTS Structure",
            "Strategy",
            AuthoringPresetKind2D::Strategy,
            "Buildable production structure with construction, vision and health.",
            &[
                "Actor2D",
                "Buildable",
                "ProductionQueue",
                "ConstructionSite",
                "Vision",
                "Health",
            ],
            &[],
            &["rts", "city-builder"],
            &["rts", "building", "production"],
        ),
        preset(
            "grand_strategy_province",
            "Grand-strategy Province",
            "Strategy",
            AuthoringPresetKind2D::Strategy,
            "Province economy, population, factories and persistent ownership.",
            &[
                "Province2D",
                "Market2D",
                "PopulationPops2D",
                "Factory2D",
                "Saveable",
            ],
            &[],
            &["grand-strategy", "simulation"],
            &["province", "economy", "population"],
        ),
        preset(
            "grand_strategy_nation",
            "Grand-strategy Nation",
            "Strategy",
            AuthoringPresetKind2D::Strategy,
            "Nation, diplomacy, research, armies, war goals, trade and persistence.",
            &[
                "Nation2D",
                "Diplomacy2D",
                "ResearchTree2D",
                "ArmyStack2D",
                "WarGoal2D",
                "TradeRoute2D",
                "Saveable",
            ],
            &[],
            &["grand-strategy", "simulation"],
            &["nation", "diplomacy", "research"],
        ),
        physics_preset(
            "physics_topdown_arcade",
            "Top-down Arcade Physics",
            "Zero-gravity, rotation-locked bodies with responsive damping.",
            [0.0, 0.0],
            4,
            &[
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "gravity_scale", json!(0.0)),
                ov("Rigidbody2D", "drag", json!(0.18)),
                ov("Rigidbody2D", "freeze_rotation", json!(true)),
            ],
            &["top-down", "action", "rpg"],
        ),
        physics_preset(
            "physics_platformer",
            "Platformer Physics",
            "Stable character body with floor snapping, slopes and buffered jumping.",
            [0.0, 22.0],
            6,
            &[
                ov("CharacterBody2D", "max_speed", json!(7.0)),
                ov("CharacterBody2D", "acceleration", json!(40.0)),
                ov("CharacterBody2D", "floor_snap", json!(0.08)),
                ov("CharacterBody2D", "max_slope_degrees", json!(45.0)),
            ],
            &["platformer", "metroidvania"],
        ),
        physics_preset(
            "physics_bouncy",
            "Bouncy Arcade Body",
            "Low-friction continuous body for pinball, sports and arcade interactions.",
            [0.0, 18.0],
            8,
            &[
                ov("Rigidbody2D", "bounciness", json!(0.92)),
                ov("Rigidbody2D", "friction", json!(0.03)),
                ov("Rigidbody2D", "continuous_collision", json!(true)),
                ov(
                    "Collider2D",
                    "material",
                    json!({"friction": 0.03, "bounciness": 0.92}),
                ),
            ],
            &["arcade", "sports", "pinball"],
        ),
        physics_preset(
            "physics_heavy",
            "Heavy Dynamic Body",
            "High-mass body with controlled damping and continuous collision.",
            [0.0, 18.0],
            8,
            &[
                ov("Rigidbody2D", "mass", json!(12.0)),
                ov("Rigidbody2D", "drag", json!(0.12)),
                ov("Rigidbody2D", "angular_drag", json!(0.18)),
                ov("Rigidbody2D", "continuous_collision", json!(true)),
            ],
            &["simulation", "destruction", "vehicle"],
        ),
        physics_preset(
            "physics_zero_gravity",
            "Zero-gravity Body",
            "Free-moving, freely rotating body for space and underwater games.",
            [0.0, 0.0],
            6,
            &[
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "gravity_scale", json!(0.0)),
                ov("Rigidbody2D", "drag", json!(0.015)),
                ov("Rigidbody2D", "freeze_rotation", json!(false)),
            ],
            &["space", "underwater", "simulation"],
        ),
        preset(
            "physics_sensor",
            "Physics Sensor",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "Non-solid monitored area with enter/exit events.",
            &["Area2D", "Trigger2D", "EventBus2D"],
            &[],
            &["all"],
            &["physics", "sensor", "trigger"],
        ),
        preset(
            "physics_one_way_platform",
            "One-way Platform",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "Static collision surface that permits passage from below.",
            &["StaticBody2D", "Collider2D", "OneWayPlatform2D"],
            &[
                ov("StaticBody2D", "one_way", json!(true)),
                ov("Collider2D", "collision_layer", json!("WorldStatic")),
            ],
            &["platformer"],
            &["physics", "platform", "one-way"],
        ),
        physics_preset(
            "physics_projectile_ccd",
            "Projectile CCD",
            "Fast continuous-collision body that avoids tunneling.",
            [0.0, 0.0],
            8,
            &[
                ov("Rigidbody2D", "use_gravity", json!(false)),
                ov("Rigidbody2D", "continuous_collision", json!(true)),
                ov("Rigidbody2D", "drag", json!(0.0)),
                ov("Collider2D", "shape", json!("circle")),
            ],
            &["shooter", "action", "simulation"],
        ),
        preset(
            "physics_ice_surface",
            "Ice Surface",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "Reusable nearly frictionless static surface with explicit material combining.",
            &["StaticBody2D", "Collider2D", "PhysicsMaterial2D"],
            &[
                ov("PhysicsMaterial2D", "friction", json!(0.015)),
                ov("PhysicsMaterial2D", "bounciness", json!(0.0)),
                ov("PhysicsMaterial2D", "friction_combine", json!("minimum")),
                ov("Collider2D", "collision_layer", json!("WorldStatic")),
            ],
            &["platformer", "racing", "puzzle"],
            &["physics", "material", "ice", "surface"],
        )
        .aliases(&["ice", "slippery_surface"]),
        preset(
            "physics_rubber_surface",
            "Rubber Surface",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "High-restitution surface for trampolines, pinball tables and arcade props.",
            &["StaticBody2D", "Collider2D", "PhysicsMaterial2D"],
            &[
                ov("PhysicsMaterial2D", "friction", json!(0.35)),
                ov("PhysicsMaterial2D", "bounciness", json!(0.92)),
                ov("PhysicsMaterial2D", "bounce_combine", json!("maximum")),
                ov("Collider2D", "collision_layer", json!("WorldStatic")),
            ],
            &["platformer", "sports", "pinball"],
            &["physics", "material", "rubber", "bounce"],
        )
        .aliases(&["rubber", "trampoline_surface"]),
        preset(
            "physics_distance_joint",
            "Distance Joint",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "Dynamic body constrained to a second selected entity without custom code.",
            &["Rigidbody2D", "Collider2D", "Joint2D"],
            &[
                ov("Joint2D", "joint_type", json!("distance")),
                ov("Joint2D", "rest_length", json!(2.0)),
                ov("Joint2D", "max_distance", json!(2.0)),
                ov("Joint2D", "stiffness", json!(0.9)),
                ov("Joint2D", "collide_connected", json!(false)),
            ],
            &["physics", "puzzle", "simulation"],
            &["physics", "joint", "rope", "constraint"],
        )
        .aliases(&["rope_joint", "distance_constraint"]),
        preset(
            "physics_spring_joint",
            "Spring Joint",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "Damped spring constraint for suspension, soft attachments and physical props.",
            &["Rigidbody2D", "Collider2D", "Joint2D"],
            &[
                ov("Joint2D", "joint_type", json!("spring")),
                ov("Joint2D", "rest_length", json!(2.0)),
                ov("Joint2D", "max_distance", json!(3.0)),
                ov("Joint2D", "stiffness", json!(18.0)),
                ov("Joint2D", "damping", json!(3.0)),
            ],
            &["physics", "vehicle", "simulation"],
            &["physics", "joint", "spring", "suspension"],
        )
        .aliases(&["spring", "suspension_joint"]),
        preset(
            "physics_wind_zone",
            "Directional Wind Zone",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "Bounded directional force field that accelerates matching physics layers.",
            &["ForceField2D"],
            &[
                ov("ForceField2D", "field_type", json!("directional")),
                ov("ForceField2D", "direction_x", json!(1.0)),
                ov("ForceField2D", "direction_y", json!(0.0)),
                ov("ForceField2D", "strength", json!(12.0)),
                ov("ForceField2D", "radius", json!(8.0)),
            ],
            &["platformer", "puzzle", "simulation"],
            &["physics", "force-field", "wind", "zone"],
        )
        .aliases(&["wind", "wind_zone"]),
        preset(
            "physics_radial_field",
            "Radial Force Field",
            "Physics Profiles",
            AuthoringPresetKind2D::Physics,
            "Radial attraction or repulsion zone with radius, falloff and layer filtering.",
            &["ForceField2D"],
            &[
                ov("ForceField2D", "field_type", json!("radial")),
                ov("ForceField2D", "strength", json!(-24.0)),
                ov("ForceField2D", "radius", json!(10.0)),
                ov("ForceField2D", "falloff", json!(1.0)),
            ],
            &["space", "puzzle", "simulation"],
            &["physics", "force-field", "gravity", "radial"],
        )
        .aliases(&["gravity_well", "radial_force"]),
    ]
}

#[allow(clippy::too_many_arguments)]
fn preset(
    id: &str,
    label: &str,
    category: &str,
    kind: AuthoringPresetKind2D,
    summary: &str,
    components: &[&str],
    overrides: &[ComponentOverride2D],
    genres: &[&str],
    tags: &[&str],
) -> AuthoringPreset2D {
    let components = components
        .iter()
        .map(|component| (*component).to_string())
        .collect::<Vec<_>>();
    let parameters = automatic_parameters(&components);
    let requirements = automatic_requirements(&components);
    let workflow_steps = automatic_workflow_steps(kind, &components);
    let recommended_next = automatic_recommendations(kind, &components);
    AuthoringPreset2D {
        id: id.to_string(),
        label: label.to_string(),
        summary: summary.to_string(),
        category: category.to_string(),
        kind,
        maturity: AuthoringPresetMaturity2D::Production,
        aliases: Vec::new(),
        genres: genres.iter().map(|value| (*value).to_string()).collect(),
        tags: tags.iter().map(|value| (*value).to_string()).collect(),
        components,
        overrides: overrides.to_vec(),
        parameters,
        requirements,
        workflow_steps,
        recommended_next,
        estimated_setup_minutes: 1,
        physics_world: None,
    }
}

fn physics_preset(
    id: &str,
    label: &str,
    summary: &str,
    gravity: [f64; 2],
    solver_iterations: usize,
    overrides: &[ComponentOverride2D],
    genres: &[&str],
) -> AuthoringPreset2D {
    let mut preset = preset(
        id,
        label,
        "Physics Profiles",
        AuthoringPresetKind2D::Physics,
        summary,
        if id == "physics_platformer" {
            &["CharacterBody2D", "Collider2D", "CharacterController2D"]
        } else {
            &["Rigidbody2D", "Collider2D"]
        },
        overrides,
        genres,
        &["physics", "profile", "ready-made"],
    );
    preset.physics_world = Some(PhysicsWorldProfile2D {
        gravity,
        solver_iterations,
        fixed_hz: 60,
        max_substeps: 4,
        continuous_collision: overrides.iter().any(|item| {
            item.component == "Rigidbody2D"
                && item.property == "continuous_collision"
                && item.value == json!(true)
        }),
        sleeping: true,
    });
    preset
}

impl AuthoringPreset2D {
    fn aliases(mut self, aliases: &[&str]) -> Self {
        self.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        self
    }
}

fn ov(component: &str, property: &str, value: Value) -> ComponentOverride2D {
    ComponentOverride2D {
        component: component.to_string(),
        property: property.to_string(),
        value,
    }
}

fn automatic_parameters(components: &[String]) -> Vec<PresetParameter2D> {
    let mut parameters = Vec::new();
    if has(components, "CharacterController2D") {
        parameters.push(number_parameter(
            "movement_speed",
            "Movement Speed",
            5.0,
            0.1,
            100.0,
            "Default walk speed applied to the controller.",
            &[("CharacterController2D", "walk_speed")],
        ));
    }
    if has(components, "CharacterBody2D") {
        parameters.push(number_parameter(
            "maximum_speed",
            "Maximum Speed",
            7.0,
            0.1,
            100.0,
            "Maximum character-body movement speed.",
            &[("CharacterBody2D", "max_speed")],
        ));
    }
    if has(components, "Health") {
        parameters.push(number_parameter(
            "maximum_health",
            "Maximum Health",
            100.0,
            1.0,
            1_000_000.0,
            "Initial and maximum health.",
            &[("Health", "max_health"), ("Health", "health")],
        ));
    }
    if has(components, "Rigidbody2D") {
        parameters.push(number_parameter(
            "mass",
            "Mass",
            1.0,
            0.001,
            1_000_000.0,
            "Dynamic-body mass.",
            &[("Rigidbody2D", "mass")],
        ));
    }
    if has(components, "PhysicsMaterial2D") {
        parameters.push(number_parameter(
            "surface_friction",
            "Surface Friction",
            0.25,
            0.0,
            4.0,
            "Friction used by the contact solver.",
            &[("PhysicsMaterial2D", "friction")],
        ));
        parameters.push(number_parameter(
            "surface_bounce",
            "Surface Bounce",
            0.0,
            0.0,
            1.0,
            "Restitution used by the contact solver.",
            &[("PhysicsMaterial2D", "bounciness")],
        ));
    }
    if has(components, "Joint2D") {
        parameters.push(number_parameter(
            "joint_length",
            "Joint Length",
            2.0,
            0.0,
            100_000.0,
            "Rest and maximum distance for the generated joint.",
            &[("Joint2D", "rest_length"), ("Joint2D", "max_distance")],
        ));
        parameters.push(number_parameter(
            "joint_stiffness",
            "Joint Stiffness",
            0.9,
            0.0,
            100_000.0,
            "Constraint or spring correction strength.",
            &[("Joint2D", "stiffness")],
        ));
    }
    if has(components, "ForceField2D") {
        parameters.push(number_parameter(
            "field_strength",
            "Field Strength",
            10.0,
            -1_000_000.0,
            1_000_000.0,
            "Signed force applied inside the field.",
            &[("ForceField2D", "strength")],
        ));
        parameters.push(number_parameter(
            "field_radius",
            "Field Radius",
            8.0,
            0.0,
            1_000_000.0,
            "World-space influence radius; zero means unbounded.",
            &[("ForceField2D", "radius")],
        ));
    }
    if has(components, "DamageDealer") {
        parameters.push(number_parameter(
            "damage",
            "Damage",
            10.0,
            0.0,
            1_000_000.0,
            "Base damage per successful hit.",
            &[("DamageDealer", "damage")],
        ));
    }
    if has(components, "Inventory") {
        parameters.push(integer_parameter(
            "inventory_capacity",
            "Inventory Slots",
            24,
            1,
            10_000,
            "Number of inventory slots.",
            &[("Inventory", "capacity")],
        ));
        parameters.push(number_parameter(
            "inventory_weight_limit",
            "Weight Limit",
            35.0,
            0.0,
            1_000_000.0,
            "Maximum carried weight; zero disables the weight limit.",
            &[("Inventory", "max_weight")],
        ));
    }
    if has(components, "SurvivalEnvironment2D") {
        parameters.push(number_parameter(
            "ambient_temperature",
            "Ambient Temperature °C",
            20.0,
            -100.0,
            100.0,
            "Ambient air temperature used by the thermal model.",
            &[("SurvivalEnvironment2D", "ambient_temperature_c")],
        ));
        parameters.push(number_parameter(
            "wind_speed",
            "Wind Speed",
            0.0,
            0.0,
            250.0,
            "Wind exposure used for chill and drying.",
            &[("SurvivalEnvironment2D", "wind_speed")],
        ));
        parameters.push(number_parameter(
            "precipitation",
            "Precipitation",
            0.0,
            0.0,
            1.0,
            "Normalized rain or snow intensity.",
            &[("SurvivalEnvironment2D", "precipitation")],
        ));
        parameters.push(number_parameter(
            "shelter",
            "Shelter",
            0.0,
            0.0,
            1.0,
            "Protection from wind and precipitation.",
            &[("SurvivalEnvironment2D", "shelter")],
        ));
        parameters.push(number_parameter(
            "pathogen_exposure",
            "Pathogen Exposure",
            0.0,
            0.0,
            1.0,
            "Environmental infection pressure applied to exposed actors.",
            &[("SurvivalEnvironment2D", "pathogen_exposure")],
        ));
    }
    if has(components, "GpuParticles2D") {
        parameters.push(integer_parameter(
            "particle_capacity",
            "Particle Capacity",
            8_192,
            1,
            1_000_000,
            "Persistent particle slots reserved for this compute emitter.",
            &[
                ("GpuParticles2D", "max_particles"),
                ("ParticleEmitter", "max_particles"),
            ],
        ));
        parameters.push(number_parameter(
            "emission_rate",
            "Emission Rate",
            128.0,
            0.0,
            1_000_000.0,
            "Particles emitted per second by compute and fallback paths.",
            &[
                ("GpuParticles2D", "emission_rate"),
                ("ParticleEmitter", "rate"),
            ],
        ));
        parameters.push(number_parameter(
            "particle_lifetime",
            "Particle Lifetime",
            1.25,
            0.01,
            3_600.0,
            "Lifetime in seconds for compute and fallback particles.",
            &[
                ("GpuParticles2D", "lifetime"),
                ("ParticleEmitter", "lifetime"),
            ],
        ));
    }
    if has(components, "NormalMap2D") {
        parameters.push(number_parameter(
            "normal_strength",
            "Normal Strength",
            1.0,
            0.0,
            1.0,
            "Tangent-space normal influence used by the WGPU sprite shader.",
            &[("NormalMap2D", "strength")],
        ));
    }
    if has(components, "RenderTexture2D") {
        parameters.push(integer_parameter(
            "render_target_width",
            "Target Width",
            512,
            1,
            i64::from(crate::render::backend::MAX_RENDER_TARGET_SIZE_2D),
            "Off-screen target width in pixels.",
            &[("RenderTexture2D", "width")],
        ));
        parameters.push(integer_parameter(
            "render_target_height",
            "Target Height",
            512,
            1,
            i64::from(crate::render::backend::MAX_RENDER_TARGET_SIZE_2D),
            "Off-screen target height in pixels.",
            &[("RenderTexture2D", "height")],
        ));
        if has(components, "Camera2D") {
            parameters.push(boolean_parameter(
                "render_target_include_ui",
                "Include UI",
                false,
                "Render legacy UI, scene canvases and retained UI text into the camera texture.",
                &[("Camera2D", "render_target_include_ui")],
            ));
        }
    }
    if has(components, "HybridScene3D") {
        parameters.push(number_parameter(
            "hybrid_world_scale",
            "3D World Scale",
            1.0,
            0.001,
            10_000.0,
            "Conversion scale from 2D gameplay units to 3D X/Z coordinates.",
            &[("HybridScene3D", "world_scale")],
        ));
        parameters.push(number_parameter(
            "hybrid_camera_pitch",
            "Camera Pitch",
            58.0,
            -89.0,
            89.0,
            "Presentation camera pitch in degrees.",
            &[("HybridScene3D", "camera_pitch_degrees")],
        ));
        parameters.push(number_parameter(
            "hybrid_camera_yaw",
            "Camera Yaw",
            35.0,
            -360.0,
            360.0,
            "Presentation camera yaw in degrees.",
            &[("HybridScene3D", "camera_yaw_degrees")],
        ));
        parameters.push(boolean_parameter(
            "hybrid_2d_overlay",
            "Render 2D Overlay",
            true,
            "Composite 2D world-space and UI layers after the 3D depth pass.",
            &[("HybridScene3D", "render_2d_overlay")],
        ));
    }
    if has(components, "HybridAnchor2D3D") {
        parameters.push(number_parameter(
            "hybrid_elevation",
            "3D Elevation",
            0.0,
            -100_000.0,
            100_000.0,
            "Vertical elevation above the synchronized 2D ground point.",
            &[("HybridAnchor2D3D", "elevation")],
        ));
    }
    if has(components, "Billboard3D") {
        parameters.push(number_parameter(
            "billboard_width",
            "Billboard Width",
            1.0,
            0.001,
            100_000.0,
            "World-space billboard width.",
            &[("Billboard3D", "width")],
        ));
        parameters.push(number_parameter(
            "billboard_height",
            "Billboard Height",
            1.0,
            0.001,
            100_000.0,
            "World-space billboard height.",
            &[("Billboard3D", "height")],
        ));
    }
    if has(components, "PostProcessVolume2D") {
        parameters.push(number_parameter(
            "post_exposure",
            "Exposure",
            1.0,
            0.0,
            8.0,
            "Scene exposure multiplier applied by the WGPU composite pass.",
            &[("PostProcessVolume2D", "exposure")],
        ));
        parameters.push(number_parameter(
            "post_contrast",
            "Contrast",
            1.05,
            0.0,
            4.0,
            "Contrast around middle gray.",
            &[("PostProcessVolume2D", "contrast")],
        ));
        parameters.push(number_parameter(
            "post_saturation",
            "Saturation",
            1.0,
            0.0,
            4.0,
            "Global color saturation.",
            &[("PostProcessVolume2D", "saturation")],
        ));
        parameters.push(number_parameter(
            "post_bloom",
            "Bloom Intensity",
            0.35,
            0.0,
            4.0,
            "Threshold bloom contribution from bright pixels.",
            &[("PostProcessVolume2D", "bloom_intensity")],
        ));
        parameters.push(number_parameter(
            "post_vignette",
            "Vignette Intensity",
            0.22,
            0.0,
            1.0,
            "Screen-edge darkening strength.",
            &[("PostProcessVolume2D", "vignette_intensity")],
        ));
        parameters.push(number_parameter(
            "post_chromatic_aberration",
            "Chromatic Aberration",
            0.0,
            0.0,
            0.05,
            "Normalized RGB lens separation in screen space.",
            &[("PostProcessVolume2D", "chromatic_aberration")],
        ));
        parameters.push(integer_parameter(
            "post_pixel_size",
            "Pixel Size",
            1,
            1,
            256,
            "Screen-space pixel block size; one disables pixelation.",
            &[("PostProcessVolume2D", "pixel_size")],
        ));
        parameters.push(number_parameter(
            "post_fog_density",
            "Fog Density",
            0.0,
            0.0,
            1.0,
            "Animated full-screen fog blend.",
            &[("PostProcessVolume2D", "fog_density")],
        ));
        parameters.push(number_parameter(
            "post_weight",
            "Volume Weight",
            1.0,
            0.0,
            1.0,
            "Blend weight used when multiple global volumes are active.",
            &[("PostProcessVolume2D", "weight")],
        ));
        parameters.push(boolean_parameter(
            "post_global",
            "Global Volume",
            true,
            "Apply the volume to the entire frame.",
            &[("PostProcessVolume2D", "global")],
        ));
    }
    parameters
}

fn integer_parameter(
    id: &str,
    label: &str,
    default_value: i64,
    minimum: i64,
    maximum: i64,
    description: &str,
    bindings: &[(&str, &str)],
) -> PresetParameter2D {
    PresetParameter2D {
        id: id.to_string(),
        label: label.to_string(),
        value_type: "integer".to_string(),
        default_value: json!(default_value),
        minimum: Some(minimum as f64),
        maximum: Some(maximum as f64),
        description: description.to_string(),
        bindings: bindings
            .iter()
            .map(|(component, property)| PresetParameterBinding2D {
                component: (*component).to_string(),
                property: (*property).to_string(),
            })
            .collect(),
    }
}

fn number_parameter(
    id: &str,
    label: &str,
    default_value: f64,
    minimum: f64,
    maximum: f64,
    description: &str,
    bindings: &[(&str, &str)],
) -> PresetParameter2D {
    PresetParameter2D {
        id: id.to_string(),
        label: label.to_string(),
        value_type: "number".to_string(),
        default_value: json!(default_value),
        minimum: Some(minimum),
        maximum: Some(maximum),
        description: description.to_string(),
        bindings: bindings
            .iter()
            .map(|(component, property)| PresetParameterBinding2D {
                component: (*component).to_string(),
                property: (*property).to_string(),
            })
            .collect(),
    }
}

fn boolean_parameter(
    id: &str,
    label: &str,
    default_value: bool,
    description: &str,
    bindings: &[(&str, &str)],
) -> PresetParameter2D {
    PresetParameter2D {
        id: id.to_string(),
        label: label.to_string(),
        value_type: "bool".to_string(),
        default_value: json!(default_value),
        minimum: None,
        maximum: None,
        description: description.to_string(),
        bindings: bindings
            .iter()
            .map(|(component, property)| PresetParameterBinding2D {
                component: (*component).to_string(),
                property: (*property).to_string(),
            })
            .collect(),
    }
}

fn automatic_requirements(components: &[String]) -> Vec<String> {
    let mut requirements = Vec::new();
    if has(components, "InputActions2D") || has(components, "PlayerController2D") {
        requirements.push("Configure actions in Project Settings > Input Map".to_string());
    }
    if has(components, "NavAgent") {
        requirements.push("Bake or provide a collision/navigation grid".to_string());
    }
    if has(components, "BehaviorTree2D") {
        requirements.push("Assign a BehaviorTree2D asset or use the built-in starter".to_string());
    }
    if has(components, "WidgetCanvas2D") {
        requirements.push("Assign a UI canvas asset in the UI Designer".to_string());
    }
    if has(components, "AudioSource2D") {
        requirements.push("Assign a SoundCue, AudioEvent or audio asset".to_string());
    }
    if has(components, "Joint2D") {
        requirements.push(
            "Select two entities and use Physics > Connect Selection to assign the joint target"
                .to_string(),
        );
    }
    if has(components, "GpuParticles2D") {
        requirements.push(
            "Enable WGPU and GPU particles in Project Settings for compute simulation; the CPU emitter remains available automatically"
                .to_string(),
        );
    }
    if has(components, "NormalMap2D") {
        requirements.push(
            "Assign a tangent-space normal texture to NormalMap2D; color and normal textures share the standard asset workflow"
                .to_string(),
        );
    }
    if has(components, "RenderTexture2D") {
        requirements.push(
            "Use the WGPU renderer for GPU render-target allocation and sampling; compatibility backends report the unsupported operation explicitly"
                .to_string(),
        );
    }
    if has(components, "PostProcessVolume2D") {
        requirements.push(
            "Use the WGPU renderer; compatibility backends reject screen-space post processing explicitly"
                .to_string(),
        );
    }
    requirements
}

fn automatic_workflow_steps(kind: AuthoringPresetKind2D, components: &[String]) -> Vec<String> {
    let mut steps = vec![
        "Apply the preset to the current selection".to_string(),
        "Review generated component values in the Inspector".to_string(),
    ];
    if has(components, "Collider2D") {
        steps.push("Fit the collider with the Scene collision tools".to_string());
    }
    if has(components, "InputActions2D") {
        steps.push("Verify input bindings in Project Settings".to_string());
    }
    if kind == AuthoringPresetKind2D::Physics {
        steps.push("Run Physics Debug Draw and inspect contacts".to_string());
    }
    if has(components, "Joint2D") {
        steps.push("Connect the owner and target from the two-entity selection".to_string());
    }
    if has(components, "ForceField2D") {
        steps.push("Resize the force-field radius directly in the Scene view".to_string());
    }
    if has(components, "GpuParticles2D") {
        steps.push(
            "Use Render Diagnostics to verify compute dispatches, capacity and spawned particles"
                .to_string(),
        );
    }
    if has(components, "NormalMap2D") {
        steps.push(
            "Assign the normal texture, then tune strength and Flip Y directly in the Inspector"
                .to_string(),
        );
        steps.push(
            "Add a Point Light 2D or Directional Light 2D and inspect the live normal response"
                .to_string(),
        );
    }
    if has(components, "RenderTexture2D") {
        steps.push(
            "Choose target resolution, update mode and whether the camera should include UI"
                .to_string(),
        );
        steps.push(
            "Use the persistent render-target:// binding from a SpriteRenderer or Material2D texture slot"
                .to_string(),
        );
    }
    if has(components, "PostProcessVolume2D") {
        steps.push(
            "Tune grade, bloom, lens, pixel, damage and fog groups directly in the Inspector"
                .to_string(),
        );
        steps.push(
            "Use Render Diagnostics to verify the composite pass and active effect count"
                .to_string(),
        );
    }
    steps.push("Play the scene and inspect Runtime Health".to_string());
    steps
}

fn automatic_recommendations(kind: AuthoringPresetKind2D, components: &[String]) -> Vec<String> {
    let mut recommendations = Vec::new();
    if has(components, "Health") {
        recommendations.push("hud_root".to_string());
    }
    if has(components, "DamageDealer") {
        recommendations.push("destructible_prop".to_string());
    }
    if has(components, "WorldPartition2D") {
        recommendations.push("procedural_spawner".to_string());
    }
    if kind == AuthoringPresetKind2D::Physics {
        recommendations.push("camera_rig".to_string());
    }
    if has(components, "PostProcessVolume2D") {
        recommendations.push("lighting_rig".to_string());
    }
    recommendations.sort();
    recommendations.dedup();
    recommendations
}

fn has(components: &[String], expected: &str) -> bool {
    components.iter().any(|component| component == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_is_large_valid_and_cross_genre() {
        let catalog = AuthoringCatalog2D::builtin();
        let validation = catalog.validate();
        assert!(validation.valid, "{:?}", validation.issues);
        assert!(catalog.presets.len() >= 40);
        assert!(catalog.total_components_referenced >= 60);
        for expected in [
            "topdown_player",
            "platformer_player",
            "survival_actor",
            "world_streamer",
            "rts_unit",
            "grand_strategy_nation",
            "physics_projectile_ccd",
            "physics_ice_surface",
            "physics_distance_joint",
            "physics_wind_zone",
            "gpu_particle_emitter",
            "lit_sprite",
            "render_target_camera",
            "post_process_cinematic",
            "post_process_horror",
            "post_process_pixel",
            "post_process_damage",
            "survival_environment",
            "survival_loadout",
            "hybrid_world_2d3d",
            "hybrid_billboard_actor",
        ] {
            assert!(catalog.resolve(expected).is_some(), "{expected}");
        }
    }

    #[test]
    fn application_plan_applies_defaults_parameters_and_aliases() {
        let catalog = AuthoringCatalog2D::builtin();
        let parameters = json!({"movement_speed": 9.5, "maximum_health": 175.0});
        let plan = catalog
            .application_plan("topdown", ["Transform", "Health"], Some(&parameters))
            .unwrap();
        assert_eq!(plan.preset_id, "topdown_player");
        assert!(plan.existing_components.iter().any(|item| item == "Health"));
        let controller = plan
            .configured_components
            .iter()
            .find(|component| component.component_type == "CharacterController2D")
            .unwrap();
        assert_eq!(controller.get_f64("walk_speed", 0.0), 9.5);
        let rigidbody = plan
            .configured_components
            .iter()
            .find(|component| component.component_type == "Rigidbody2D")
            .unwrap();
        assert!(!rigidbody.get_bool("use_gravity", true));
    }

    #[test]
    fn render_target_preset_exposes_scriptless_ui_capture() {
        let catalog = AuthoringCatalog2D::builtin();
        let parameters = json!({
            "render_target_width": 640,
            "render_target_height": 360,
            "render_target_include_ui": true,
        });
        let plan = catalog
            .application_plan("render_target_camera", ["Transform"], Some(&parameters))
            .unwrap();
        let camera = plan
            .configured_components
            .iter()
            .find(|component| component.component_type == "Camera2D")
            .unwrap();
        let texture = plan
            .configured_components
            .iter()
            .find(|component| component.component_type == "RenderTexture2D")
            .unwrap();
        assert!(camera.get_bool("render_target_include_ui", false));
        assert_eq!(texture.get_usize("width", 0), 640);
        assert_eq!(texture.get_usize("height", 0), 360);
    }

    #[test]
    fn post_process_presets_expose_tunable_scriptless_wgpu_effects() {
        let catalog = AuthoringCatalog2D::builtin();
        let plan = catalog
            .application_plan(
                "horror_grade",
                std::iter::empty::<&str>(),
                Some(&json!({
                    "post_bloom": 0.42,
                    "post_vignette": 0.7,
                    "post_fog_density": 0.2,
                    "post_global": true,
                })),
            )
            .unwrap();
        let volume = plan
            .configured_components
            .iter()
            .find(|component| component.component_type == "PostProcessVolume2D")
            .unwrap();
        assert_eq!(volume.get_string("preset", ""), "horror_survival");
        assert_eq!(volume.get_f64("bloom_intensity", 0.0), 0.42);
        assert_eq!(volume.get_f64("vignette_intensity", 0.0), 0.7);
        assert_eq!(volume.get_f64("fog_density", 0.0), 0.2);
        assert!(volume.get_bool("global", false));
        assert!(
            catalog
                .resolve("post_process_pixel")
                .unwrap()
                .requirements
                .iter()
                .any(|requirement| requirement.contains("WGPU"))
        );
    }

    #[test]
    fn hybrid_presets_enable_the_world_and_expose_2d_3d_sync_controls() {
        let catalog = AuthoringCatalog2D::builtin();
        let world = catalog
            .application_plan(
                "hybrid_world",
                std::iter::empty::<&str>(),
                Some(&json!({
                    "hybrid_world_scale": 2.5,
                    "hybrid_camera_pitch": 62.0,
                    "hybrid_camera_yaw": -25.0,
                })),
            )
            .unwrap();
        let settings = world
            .configured_components
            .iter()
            .find(|component| component.component_type == "HybridScene3D")
            .unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.get_f64("world_scale", 0.0), 2.5);
        assert_eq!(settings.get_f64("camera_pitch_degrees", 0.0), 62.0);
        assert_eq!(settings.get_f64("camera_yaw_degrees", 0.0), -25.0);

        let actor = catalog
            .application_plan(
                "sprite3d_actor",
                std::iter::empty::<&str>(),
                Some(&json!({
                    "hybrid_elevation": 1.25,
                    "billboard_width": 0.8,
                    "billboard_height": 1.6,
                })),
            )
            .unwrap();
        let anchor = actor
            .configured_components
            .iter()
            .find(|component| component.component_type == "HybridAnchor2D3D")
            .unwrap();
        let billboard = actor
            .configured_components
            .iter()
            .find(|component| component.component_type == "Billboard3D")
            .unwrap();
        assert_eq!(anchor.get_f64("elevation", 0.0), 1.25);
        assert_eq!(billboard.get_f64("width", 0.0), 0.8);
        assert_eq!(billboard.get_f64("height", 0.0), 1.6);
    }

    #[test]
    fn search_combines_genres_tags_and_kinds() {
        let catalog = AuthoringCatalog2D::builtin();
        let results = catalog.search("platform physics", Some(AuthoringPresetKind2D::Physics), 10);
        assert!(
            results
                .iter()
                .any(|preset| preset.id == "physics_platformer")
        );
        assert!(
            results
                .iter()
                .all(|preset| preset.kind == AuthoringPresetKind2D::Physics)
        );
    }

    #[test]
    fn advanced_physics_presets_create_material_joint_and_force_field_components() {
        let catalog = AuthoringCatalog2D::builtin();
        let ice = catalog
            .application_plan(
                "ice",
                std::iter::empty::<&str>(),
                Some(&json!({"surface_friction": 0.04})),
            )
            .unwrap();
        let material = ice
            .configured_components
            .iter()
            .find(|component| component.component_type == "PhysicsMaterial2D")
            .unwrap();
        assert_eq!(material.get_f64("friction", 1.0), 0.04);

        let joint = catalog
            .application_plan(
                "rope_joint",
                std::iter::empty::<&str>(),
                Some(&json!({"joint_length": 6.5})),
            )
            .unwrap();
        let constraint = joint
            .configured_components
            .iter()
            .find(|component| component.component_type == "Joint2D")
            .unwrap();
        assert_eq!(constraint.get_f64("rest_length", 0.0), 6.5);
        assert_eq!(constraint.get_f64("max_distance", 0.0), 6.5);
        assert!(
            joint
                .requirements
                .iter()
                .any(|requirement| requirement.contains("Select two entities"))
        );

        let wind = catalog
            .application_plan(
                "wind",
                std::iter::empty::<&str>(),
                Some(&json!({"field_strength": 42.0, "field_radius": 15.0})),
            )
            .unwrap();
        let field = wind
            .configured_components
            .iter()
            .find(|component| component.component_type == "ForceField2D")
            .unwrap();
        assert_eq!(field.get_f64("strength", 0.0), 42.0);
        assert_eq!(field.get_f64("radius", 0.0), 15.0);
    }
}
