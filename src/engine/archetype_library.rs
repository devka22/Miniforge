use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityArchetype {
    pub key: String,
    pub display_name: String,
    pub entity_type: String,
    pub tag: String,
    pub layer: String,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub speed: f64,
    pub components: Vec<Value>,
    #[serde(default)]
    pub scripts: Vec<Value>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ArchetypeLibrary {
    pub archetypes: BTreeMap<String, EntityArchetype>,
}

impl ArchetypeLibrary {
    pub fn with_defaults() -> Self {
        let mut library = Self::default();
        for archetype in default_archetypes() {
            library.register(archetype);
        }
        library
    }

    pub fn register(&mut self, archetype: EntityArchetype) {
        self.archetypes.insert(archetype.key.clone(), archetype);
    }

    pub fn get(&self, key: &str) -> Option<&EntityArchetype> {
        self.archetypes.get(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.archetypes.keys().cloned().collect()
    }

    pub fn by_tag(&self, tag: &str) -> Vec<&EntityArchetype> {
        self.archetypes
            .values()
            .filter(|archetype| archetype.tag == tag)
            .collect()
    }

    pub fn instantiate(
        &self,
        key: &str,
        x: f64,
        y: f64,
        team_id: Option<i64>,
    ) -> Option<GameObject> {
        let archetype = self.get(key)?;
        Some(archetype.instantiate(x, y, team_id))
    }
}

impl EntityArchetype {
    pub fn instantiate(&self, x: f64, y: f64, team_id: Option<i64>) -> GameObject {
        let mut entity = if self.entity_type == "Unit" {
            GameObject::new_unit(x, y, Some(self.display_name.clone()))
        } else {
            GameObject::new(x, y, Some(self.display_name.clone()))
        };
        entity.entity_type = self.entity_type.clone();
        entity.tag = self.tag.clone();
        entity.layer = self.layer.clone();
        entity.width = self.width.max(0.05);
        entity.height = self.height.max(0.05);
        entity.radius = self.radius.max(0.01);
        entity.speed = self.speed.max(0.0);
        entity.scripts = self.scripts.clone();

        for spec in &self.components {
            if let Some(component) = component_from_spec(spec, team_id) {
                entity.add_component(component);
            }
        }
        if let Some(team_id) = team_id {
            ensure_team(&mut entity, team_id);
        }
        entity.sync_to_components();
        entity
    }
}

fn component_from_spec(spec: &Value, team_id: Option<i64>) -> Option<Component> {
    let component_type = spec.as_str().map(ToString::to_string).or_else(|| {
        spec.get("component_type")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    })?;
    let mut component = default_component(&component_type)?;
    if spec.is_object() {
        component.merge_data(spec);
    }
    if component_type == "Team"
        && let Some(team_id) = team_id
    {
        component.set("team_id", json!(team_id));
        component.set(
            "team_name",
            json!(match team_id {
                1 => "Player",
                2 => "Enemy",
                _ => "Neutral",
            }),
        );
        component.set(
            "color",
            json!(match team_id {
                1 => [80, 160, 255],
                2 => [255, 95, 95],
                _ => [160, 160, 160],
            }),
        );
    }
    Some(component)
}

fn ensure_team(entity: &mut GameObject, team_id: i64) {
    let mut team = default_component("Team").expect("Team");
    team.set("team_id", json!(team_id));
    team.set(
        "team_name",
        json!(match team_id {
            1 => "Player",
            2 => "Enemy",
            _ => "Neutral",
        }),
    );
    team.set(
        "color",
        json!(match team_id {
            1 => [80, 160, 255],
            2 => [255, 95, 95],
            _ => [160, 160, 160],
        }),
    );
    entity.add_component(team);
    entity.tag = match team_id {
        1 => "Player",
        2 => "Enemy",
        _ => "Neutral",
    }
    .to_string();
}

fn archetype(
    key: &str,
    display_name: &str,
    entity_type: &str,
    tag: &str,
    layer: &str,
    components: Vec<Value>,
) -> EntityArchetype {
    EntityArchetype {
        key: key.to_string(),
        display_name: display_name.to_string(),
        entity_type: entity_type.to_string(),
        tag: tag.to_string(),
        layer: layer.to_string(),
        width: if layer == "Buildings" { 2.4 } else { 1.0 },
        height: if layer == "Buildings" { 2.0 } else { 1.0 },
        radius: if layer == "Buildings" { 1.2 } else { 0.45 },
        speed: if entity_type == "Unit" { 4.5 } else { 0.0 },
        components,
        scripts: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

fn default_archetypes() -> Vec<EntityArchetype> {
    vec![
        archetype(
            "topdown_hero",
            "Hero",
            "Unit",
            "Player",
            "Units",
            vec![
                json!("Health"),
                json!("Stats"),
                json!("Inventory"),
                json!("Cooldown"),
                json!("NavAgent"),
                json!("Rigidbody2D"),
                json!("CharacterController2D"),
                json!("CameraFollow"),
                json!("Ability"),
                json!("QuestLog"),
                json!("Saveable"),
                json!("Light2D"),
            ],
        ),
        archetype(
            "platformer_player",
            "PlatformerPlayer",
            "Unit",
            "Player",
            "Units",
            vec![
                json!("Health"),
                json!("Stats"),
                json!("Rigidbody2D"),
                json!("CharacterController2D"),
                json!("CameraFollow"),
                json!("Checkpoint"),
                json!("Saveable"),
            ],
        ),
        archetype(
            "rts_worker",
            "Worker",
            "Unit",
            "Player",
            "Units",
            vec![
                json!("Team"),
                json!("Health"),
                json!("Stats"),
                json!("Inventory"),
                json!("Worker"),
                json!("Commandable"),
                json!("Vision"),
                json!("NavAgent"),
                json!("SquadMember"),
                json!("Blackboard"),
            ],
        ),
        archetype(
            "rts_soldier",
            "Soldier",
            "Unit",
            "Player",
            "Units",
            vec![
                json!("Team"),
                json!("Health"),
                json!({"component_type": "Stats", "attack": 16.0, "defense": 2.0}),
                json!("Commandable"),
                json!("Vision"),
                json!("NavAgent"),
                json!("DamageDealer"),
                json!("CombatTarget"),
                json!("ThreatSource"),
                json!("SquadMember"),
                json!("Blackboard"),
            ],
        ),
        archetype(
            "rts_command_center",
            "CommandCenter",
            "Building",
            "Player",
            "Buildings",
            vec![
                json!("Team"),
                json!({"component_type": "Health", "max_health": 900.0, "health": 900.0}),
                json!({"component_type": "EconomyWallet", "resources": {"Gold": 500.0, "Wood": 250.0, "Supply": 0.0}}),
                json!("ProductionQueue"),
                json!("ProductionRecipeBook"),
                json!("Buildable"),
                json!("Commandable"),
                json!("Vision"),
                json!("RtsBrain"),
            ],
        ),
        archetype(
            "rts_barracks",
            "Barracks",
            "Building",
            "Player",
            "Buildings",
            vec![
                json!("Team"),
                json!({"component_type": "Health", "max_health": 600.0, "health": 600.0}),
                json!("ProductionQueue"),
                json!("ProductionRecipeBook"),
                json!("Buildable"),
                json!("Commandable"),
                json!("Vision"),
            ],
        ),
        archetype(
            "gold_node",
            "GoldNode",
            "GameObject",
            "Resource",
            "Resources",
            vec![json!("ResourceNode"), json!("ObjectiveMarker")],
        ),
    ]
}
