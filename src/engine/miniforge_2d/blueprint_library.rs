use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engine::miniforge_2d::blueprint::{
    BlueprintEdge2D, BlueprintGraph2D, BlueprintNode2D, BlueprintPin2D, BlueprintVariable2D,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintTemplateInfo2D {
    pub name: String,
    pub category: String,
    pub description: String,
    pub attach_to_selected: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BlueprintLibrary2D {
    pub templates: Vec<BlueprintTemplateInfo2D>,
}

impl BlueprintLibrary2D {
    pub fn default_library() -> Self {
        Self {
            templates: template_names()
                .into_iter()
                .map(|(name, category)| BlueprintTemplateInfo2D {
                    name: name.to_string(),
                    category: category.to_string(),
                    description: format!("Template {name} para MiniForge2D."),
                    attach_to_selected: true,
                })
                .collect(),
        }
    }

    pub fn search(&self, query: &str) -> Vec<&BlueprintTemplateInfo2D> {
        let query = query.to_lowercase();
        self.templates
            .iter()
            .filter(|template| {
                query.is_empty()
                    || template.name.to_lowercase().contains(&query)
                    || template.category.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn instantiate(&self, name: &str) -> Option<BlueprintGraph2D> {
        if !self.templates.iter().any(|template| template.name == name) {
            return None;
        }
        Some(template_graph(name))
    }
}

pub fn template_names() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Player TopDown", "Player"),
        ("Player Platformer", "Player"),
        ("Enemy Patrol", "AI"),
        ("Enemy Chase", "AI"),
        ("Enemy Shooter", "AI"),
        ("Pickup Item", "Gameplay"),
        ("Health System", "Gameplay"),
        ("Damage Zone", "Gameplay"),
        ("Door + Key", "Gameplay"),
        ("Dialogue Trigger", "Narrative"),
        ("Quest Giver", "Narrative"),
        ("RTS Unit", "RTS"),
        ("RTS Building", "RTS"),
        ("RTS Spawner", "RTS"),
        ("UI Health Bar", "UI"),
        ("Main Menu Button", "UI"),
        ("Pause Menu", "UI"),
        ("Save Point", "Persistence"),
    ]
}

fn template_graph(name: &str) -> BlueprintGraph2D {
    let runtime_kind = match name {
        "Enemy Patrol" => "Patrol",
        "Enemy Chase" => "RunBehaviorTree",
        "Pickup Item" => "InventoryAdd",
        "Damage Zone" => "Damage",
        "UI Health Bar" => "SetUiText",
        "Save Point" => "SaveGame",
        _ => "PrintString",
    };
    BlueprintGraph2D {
        name: name.replace([' ', '+'], "_"),
        runtime: "miniforge_visual_script_2d".to_string(),
        variables: BTreeMap::from([(
            "Enabled".to_string(),
            BlueprintVariable2D {
                value_type: "bool".to_string(),
                default_value: json!(true),
                editable: true,
            },
        )]),
        functions: BTreeMap::new(),
        nodes: vec![
            BlueprintNode2D {
                id: "begin_play".to_string(),
                kind: "EventBeginPlay".to_string(),
                title: "Begin Play".to_string(),
                x: 0.0,
                y: 0.0,
                pins: vec![pin("then", "exec", "out")],
                data: json!({}),
            },
            BlueprintNode2D {
                id: "template_action".to_string(),
                kind: runtime_kind.to_string(),
                title: name.to_string(),
                x: 260.0,
                y: 0.0,
                pins: vec![pin("exec", "exec", "in"), pin("then", "exec", "out")],
                data: json!({"message": name, "seconds": 0.25, "state": "Idle", "text": name}),
            },
        ],
        edges: vec![BlueprintEdge2D {
            from: "begin_play".to_string(),
            from_pin: "then".to_string(),
            to: "template_action".to_string(),
            to_pin: "exec".to_string(),
        }],
    }
}

fn pin(name: &str, pin_type: &str, direction: &str) -> BlueprintPin2D {
    BlueprintPin2D {
        name: name.to_string(),
        pin_type: pin_type.to_string(),
        direction: direction.to_string(),
    }
}
