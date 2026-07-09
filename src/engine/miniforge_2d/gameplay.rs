use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::miniforge_2d::actor::{Actor2DFactory, ensure_component};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameFramework2D {
    pub game_instance: GameInstance2D,
    pub game_mode: GameMode2DConfig,
    pub game_state: GameState2DConfig,
    pub default_pawn: Pawn2DConfig,
    pub player_state: PlayerState2DConfig,
    pub player_controller: Controller2DConfig,
    pub ai_controller: Controller2DConfig,
    pub camera_manager: CameraManager2DConfig,
    pub hud: HUD2DConfig,
    pub save_game: SaveGame2DConfig,
    #[serde(default)]
    pub world_subsystems: Vec<Subsystem2DConfig>,
    #[serde(default)]
    pub game_subsystems: Vec<Subsystem2DConfig>,
    #[serde(default)]
    pub editor_subsystems: Vec<Subsystem2DConfig>,
    #[serde(default)]
    pub scene_streaming: SceneStreamingPlan2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameInstance2D {
    pub name: String,
    #[serde(default)]
    pub persistent_services: Vec<String>,
    #[serde(default)]
    pub global_state: Value,
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
pub struct GameState2DConfig {
    pub replicated: bool,
    pub phase: String,
    #[serde(default)]
    pub score_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerState2DConfig {
    pub profile_key: String,
    #[serde(default)]
    pub tracked_stats: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraManager2DConfig {
    pub default_camera: String,
    pub pixel_perfect: bool,
    pub stack: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HUD2DConfig {
    pub root_canvas: String,
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveGame2DConfig {
    pub slot_name: String,
    pub autosave_enabled: bool,
    #[serde(default)]
    pub saved_systems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subsystem2DConfig {
    pub name: String,
    pub startup_order: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneStreamingPlan2D {
    #[serde(default)]
    pub persistent_scene: String,
    #[serde(default)]
    pub additive_scenes: Vec<String>,
    #[serde(default)]
    pub preload_assets: Vec<String>,
}

impl Default for GameFramework2D {
    fn default() -> Self {
        Self {
            game_instance: GameInstance2D {
                name: "GI_Main2D".to_string(),
                persistent_services: vec![
                    "SaveService".to_string(),
                    "AudioService".to_string(),
                    "UiService".to_string(),
                    "AbilityService".to_string(),
                ],
                global_state: json!({"profile": "default", "language": "en"}),
            },
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
            game_state: GameState2DConfig {
                replicated: false,
                phase: "playing".to_string(),
                score_keys: vec!["score".to_string(), "timer".to_string()],
            },
            default_pawn: Pawn2DConfig {
                name: "BP_PlayerPawn2D".to_string(),
                movement_mode: "topdown".to_string(),
                collision_layer: "Pawn".to_string(),
                input_enabled: true,
            },
            player_state: PlayerState2DConfig {
                profile_key: "player.profile".to_string(),
                tracked_stats: vec![
                    "score".to_string(),
                    "lives".to_string(),
                    "coins".to_string(),
                    "quest_flags".to_string(),
                ],
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
            camera_manager: CameraManager2DConfig {
                default_camera: "MainCamera2D".to_string(),
                pixel_perfect: true,
                stack: vec!["WorldCamera".to_string(), "UiCamera".to_string()],
            },
            hud: HUD2DConfig {
                root_canvas: "assets/ui/hud.ui2d.json".to_string(),
                layers: vec!["HUD".to_string(), "Menu".to_string(), "Overlay".to_string()],
            },
            save_game: SaveGame2DConfig {
                slot_name: "slot_0".to_string(),
                autosave_enabled: true,
                saved_systems: vec![
                    "SceneState".to_string(),
                    "PlayerState".to_string(),
                    "Inventory".to_string(),
                    "QuestLog".to_string(),
                ],
            },
            world_subsystems: vec![
                subsystem("PhysicsService", 10),
                subsystem("RenderService", 20),
                subsystem("AiService", 30),
                subsystem("ValidationService", 90),
            ],
            game_subsystems: vec![
                subsystem("SaveService", 10),
                subsystem("AbilityService", 20),
                subsystem("QuestService", 30),
                subsystem("EconomyService", 40),
            ],
            editor_subsystems: vec![
                subsystem("AssetService", 10),
                subsystem("SceneService", 20),
                subsystem("ScriptService", 30),
                subsystem("GraphService", 40),
                subsystem("UiService", 50),
            ],
            scene_streaming: SceneStreamingPlan2D {
                persistent_scene: "saves/scenes/main.scene".to_string(),
                additive_scenes: vec!["saves/scenes/hud.scene".to_string()],
                preload_assets: vec![
                    "assets/ui/hud.ui2d.json".to_string(),
                    "scripts/visual_graphs/BP_PlayerPawn2D.mfgraph".to_string(),
                ],
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

    pub fn startup_order(&self) -> Vec<String> {
        let mut systems = self
            .world_subsystems
            .iter()
            .chain(self.game_subsystems.iter())
            .chain(self.editor_subsystems.iter())
            .filter(|system| system.enabled)
            .collect::<Vec<_>>();
        systems.sort_by_key(|system| system.startup_order);
        systems.iter().map(|system| system.name.clone()).collect()
    }

    pub fn validate_complex_game_setup(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.game_instance.persistent_services.is_empty() {
            warnings.push("GameInstance2D sin servicios persistentes".to_string());
        }
        if self.hud.root_canvas.trim().is_empty() {
            warnings.push("HUD2D sin root_canvas".to_string());
        }
        if self.save_game.autosave_enabled && self.save_game.saved_systems.is_empty() {
            warnings.push("SaveGame2D autosave activo sin sistemas guardados".to_string());
        }
        if self.scene_streaming.persistent_scene.trim().is_empty() {
            warnings.push("SceneStreaming sin persistent_scene".to_string());
        }
        warnings
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

fn subsystem(name: &str, startup_order: i32) -> Subsystem2DConfig {
    Subsystem2DConfig {
        name: name.to_string(),
        startup_order,
        enabled: true,
    }
}
