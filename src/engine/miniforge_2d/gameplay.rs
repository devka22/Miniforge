use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::miniforge_2d::actor::{Actor2DFactory, ensure_component};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameFramework2D {
    pub game_mode: GameMode2DConfig,
    pub default_pawn: Pawn2DConfig,
    pub player_controller: Controller2DConfig,
    pub ai_controller: Controller2DConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameMode2DConfig {
    pub name: String,
    pub start_scene: String,
    pub default_pawn: String,
    pub player_controller: String,
    pub hud_canvas: Option<String>,
    pub spawn_policy: String,
    #[serde(default)]
    pub rules: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pawn2DConfig {
    pub name: String,
    pub movement_mode: String,
    pub collision_layer: String,
    pub input_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Controller2DConfig {
    pub name: String,
    pub controller_type: String,
    pub possessed_pawn: Option<u64>,
    pub input_context: Option<String>,
    pub behavior_tree: Option<String>,
}

impl Default for GameFramework2D {
    fn default() -> Self {
        Self {
            game_mode: GameMode2DConfig {
                name: "GM_Main2D".to_string(),
                start_scene: "saves/scenes/main.scene".to_string(),
                default_pawn: "BP_PlayerPawn2D".to_string(),
                player_controller: "BP_PlayerController2D".to_string(),
                hud_canvas: Some("assets/ui/hud.ui2d.json".to_string()),
                spawn_policy: "spawn_at_player_start".to_string(),
                rules: json!({
                    "pause_on_focus_lost": true,
                    "fixed_timestep": 1.0 / 60.0
                }),
            },
            default_pawn: Pawn2DConfig {
                name: "BP_PlayerPawn2D".to_string(),
                movement_mode: "topdown".to_string(),
                collision_layer: "Pawn".to_string(),
                input_enabled: true,
            },
            player_controller: Controller2DConfig {
                name: "BP_PlayerController2D".to_string(),
                controller_type: "player".to_string(),
                possessed_pawn: None,
                input_context: Some("settings/input_map.json".to_string()),
                behavior_tree: None,
            },
            ai_controller: Controller2DConfig {
                name: "BP_AIController2D".to_string(),
                controller_type: "ai".to_string(),
                possessed_pawn: None,
                input_context: None,
                behavior_tree: Some("assets/ai/basic_enemy.bt2d.json".to_string()),
            },
        }
    }
}

impl GameFramework2D {
    pub fn spawn_minimal_entities(&self) -> Vec<GameObject> {
        let game_mode = Actor2DFactory::game_mode(
            self.game_mode.name.clone(),
            self.game_mode.default_pawn.clone(),
        );
        let pawn = Actor2DFactory::pawn(self.default_pawn.name.clone(), 4.0, 4.0);
        let mut player_controller =
            Actor2DFactory::player_controller(self.player_controller.name.clone());
        let ai_controller = Actor2DFactory::ai_controller(self.ai_controller.name.clone());
        if let Some(controller) = player_controller.get_component_mut("PlayerController2D") {
            controller.set("possessed_pawn", json!(pawn.id));
            controller.set("input_context", json!(self.player_controller.input_context));
        }
        vec![game_mode, pawn, player_controller, ai_controller]
    }

    pub fn apply_to_entities(&self, entities: &mut [GameObject]) {
        for entity in entities {
            match entity.entity_type.as_str() {
                "Pawn2D" | "Unit" if entity.tag == "Player" => {
                    ensure_component(entity, "Pawn2D");
                    ensure_component(entity, "CharacterController2D");
                    if let Some(controller) = entity.get_component_mut("CharacterController2D") {
                        controller.set("mode", json!(self.default_pawn.movement_mode));
                        controller.set("input_enabled", json!(self.default_pawn.input_enabled));
                    }
                    if let Some(collider) = entity.get_component_mut("Collider2D") {
                        collider.set("collision_layer", json!(self.default_pawn.collision_layer));
                    }
                }
                "PlayerController2D" => {
                    ensure_component(entity, "PlayerController2D");
                }
                "AIController2D" => {
                    ensure_component(entity, "AIController2D");
                    ensure_component(entity, "Blackboard");
                    ensure_component(entity, "BehaviorTree2D");
                }
                _ => {}
            }
        }
    }
}
