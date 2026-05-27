use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RtsTools2D {
    pub unit_selection: bool,
    pub box_selection: bool,
    pub command_system: bool,
    pub move_command: bool,
    pub attack_move: bool,
    pub patrol: bool,
    pub hold_position: bool,
    pub resource_gather: bool,
    pub building_placement: bool,
    pub production_queue: bool,
    pub squad_movement: bool,
    pub flow_fields: bool,
    pub influence_map: bool,
    pub threat_map: bool,
    pub fog_of_war: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RtsDemoScene2D {
    pub scene_name: String,
    pub entities: Vec<Value>,
    pub ui: Value,
}

impl Default for RtsTools2D {
    fn default() -> Self {
        Self {
            unit_selection: true,
            box_selection: true,
            command_system: true,
            move_command: true,
            attack_move: true,
            patrol: true,
            hold_position: true,
            resource_gather: true,
            building_placement: true,
            production_queue: true,
            squad_movement: true,
            flow_fields: true,
            influence_map: true,
            threat_map: true,
            fog_of_war: true,
        }
    }
}

impl RtsTools2D {
    pub fn feature_names(&self) -> Vec<&'static str> {
        vec![
            "Unit Selection",
            "Box Selection",
            "Command System",
            "Move Command",
            "Attack Move",
            "Patrol",
            "Hold Position",
            "Resource Gather",
            "Building Placement",
            "Production Queue",
            "Squad Movement",
            "Flow Fields",
            "Influence Map",
            "Threat Map",
            "Fog of War",
        ]
    }

    pub fn minimal_demo_scene(&self) -> RtsDemoScene2D {
        RtsDemoScene2D {
            scene_name: "RTS_Minimal_Demo".to_string(),
            entities: vec![
                json!({
                    "type": "Pawn2D",
                    "name": "Worker",
                    "tag": "Unit",
                    "components": [
                        {"component_type": "RTSMovement"},
                        {"component_type": "Commandable", "can_move": true, "can_gather": true},
                        {"component_type": "Worker"}
                    ]
                }),
                json!({
                    "type": "Actor2D",
                    "name": "CommandCenter",
                    "tag": "Building",
                    "components": [
                        {"component_type": "ProductionQueue"},
                        {"component_type": "ProductionRecipeBook"},
                        {"component_type": "EconomyWallet", "resources": {"Gold": 150}}
                    ]
                }),
                json!({
                    "type": "Actor2D",
                    "name": "GoldNode",
                    "tag": "Resource",
                    "components": [{"component_type": "ResourceNode", "resource_type": "Gold"}]
                }),
            ],
            ui: json!({"canvas": "assets/ui/rts_hud.mfui"}),
        }
    }
}
