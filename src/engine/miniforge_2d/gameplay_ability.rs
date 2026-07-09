use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameplayTag(pub String);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameplayTagContainer {
    #[serde(default)]
    pub tags: BTreeSet<GameplayTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttributeSet2D {
    pub name: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameplayAbility2D {
    pub name: String,
    #[serde(default)]
    pub tags: GameplayTagContainer,
    #[serde(default)]
    pub required_tags: GameplayTagContainer,
    #[serde(default)]
    pub blocked_tags: GameplayTagContainer,
    pub cooldown_seconds: f64,
    pub cost_attribute: Option<String>,
    pub cost_amount: f64,
    pub targeting: Targeting2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameplayEffect2D {
    pub name: String,
    pub duration_seconds: f64,
    #[serde(default)]
    pub granted_tags: GameplayTagContainer,
    #[serde(default)]
    pub modifiers: Vec<AttributeModifier2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttributeModifier2D {
    pub attribute: String,
    pub operation: String,
    pub magnitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Targeting2D {
    pub mode: String,
    pub radius: f64,
    #[serde(default)]
    pub required_target_tags: GameplayTagContainer,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AbilityQueue2D {
    #[serde(default)]
    pub queued: VecDeque<String>,
    pub max_len: usize,
}

impl GameplayTag {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn is_child_of(&self, parent: &GameplayTag) -> bool {
        self.0 == parent.0 || self.0.starts_with(&format!("{}.", parent.0))
    }
}

impl GameplayTagContainer {
    pub fn from_tags(tags: &[&str]) -> Self {
        Self {
            tags: tags.iter().map(|tag| GameplayTag::new(*tag)).collect(),
        }
    }

    pub fn has(&self, tag: &GameplayTag) -> bool {
        self.tags.iter().any(|candidate| candidate.is_child_of(tag))
    }

    pub fn has_all(&self, required: &GameplayTagContainer) -> bool {
        required.tags.iter().all(|tag| self.has(tag))
    }

    pub fn has_any(&self, blocked: &GameplayTagContainer) -> bool {
        blocked.tags.iter().any(|tag| self.has(tag))
    }
}

impl Default for Targeting2D {
    fn default() -> Self {
        Self {
            mode: "self".to_string(),
            radius: 0.0,
            required_target_tags: GameplayTagContainer::default(),
        }
    }
}

impl AbilityQueue2D {
    pub fn queue(&mut self, ability: impl Into<String>) {
        let max_len = self.max_len.max(1);
        if self.queued.len() >= max_len {
            self.queued.pop_front();
        }
        self.queued.push_back(ability.into());
    }

    pub fn pop_next(&mut self) -> Option<String> {
        self.queued.pop_front()
    }
}

impl GameplayAbility2D {
    pub fn can_activate(
        &self,
        owner_tags: &GameplayTagContainer,
        attributes: &AttributeSet2D,
    ) -> bool {
        owner_tags.has_all(&self.required_tags)
            && !owner_tags.has_any(&self.blocked_tags)
            && self
                .cost_attribute
                .as_ref()
                .and_then(|name| attributes.attributes.get(name))
                .is_none_or(|value| *value >= self.cost_amount)
    }
}

impl GameplayEffect2D {
    pub fn apply_to(&self, attributes: &mut AttributeSet2D) {
        for modifier in &self.modifiers {
            let value = attributes
                .attributes
                .entry(modifier.attribute.clone())
                .or_insert(0.0);
            match modifier.operation.as_str() {
                "add" => *value += modifier.magnitude,
                "multiply" => *value *= modifier.magnitude,
                "override" => *value = modifier.magnitude,
                _ => {}
            }
        }
    }
}

pub fn minimal_ability_system() -> Value {
    json!({
        "tags": ["Character.Player", "Faction.Player", "Weapon.Sword"],
        "attributes": {"Health": 100.0, "Mana": 50.0, "Stamina": 75.0},
        "abilities": [{
            "name": "Slash",
            "tags": {"tags": ["Ability.Melee"]},
            "required_tags": {"tags": ["Weapon.Sword"]},
            "blocked_tags": {"tags": ["State.Stunned"]},
            "cooldown_seconds": 0.35,
            "cost_attribute": "Stamina",
            "cost_amount": 8.0,
            "targeting": {"mode": "cone", "radius": 1.5, "required_target_tags": {"tags": ["Faction.Enemy"]}}
        }],
        "effects": [{
            "name": "Burning",
            "duration_seconds": 4.0,
            "granted_tags": {"tags": ["State.Burning"]},
            "modifiers": [{"attribute": "Health", "operation": "add", "magnitude": -3.0}]
        }]
    })
}
