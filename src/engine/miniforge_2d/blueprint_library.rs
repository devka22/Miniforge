use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engine::miniforge_2d::blueprint::{
    BlueprintClassSettings2D, BlueprintComponent2D, BlueprintEdge2D, BlueprintGraph2D,
    BlueprintNode2D, BlueprintPin2D, BlueprintVariable2D,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintTemplateInfo2D {
    pub name: String,
    pub category: String,
    pub description: String,
    pub attach_to_selected: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub complexity: u8,
    #[serde(default)]
    pub recommended_assets: Vec<String>,
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
                .map(|(name, category)| {
                    let tags = template_tags(name, category);
                    BlueprintTemplateInfo2D {
                        name: name.to_string(),
                        category: category.to_string(),
                        description: format!("Template {name} para MiniForge2D."),
                        attach_to_selected: true,
                        complexity: template_complexity(name),
                        recommended_assets: recommended_assets_for_category(category),
                        tags,
                    }
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
                    || template
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
            })
            .collect()
    }

    pub fn categories(&self) -> Vec<String> {
        self.templates
            .iter()
            .map(|template| template.category.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn templates_for_category(&self, category: &str) -> Vec<&BlueprintTemplateInfo2D> {
        self.templates
            .iter()
            .filter(|template| template.category.eq_ignore_ascii_case(category))
            .collect()
    }

    pub fn recommended_for_context(
        &self,
        asset_type: &str,
        selected_layer: &str,
    ) -> Vec<&BlueprintTemplateInfo2D> {
        let asset_type = asset_type.to_lowercase();
        let selected_layer = selected_layer.to_lowercase();
        let mut templates = self
            .templates
            .iter()
            .filter(|template| {
                template.tags.iter().any(|tag| {
                    asset_type.contains(&tag.to_lowercase())
                        || selected_layer.contains(&tag.to_lowercase())
                }) || (asset_type.contains("ui") && template.category == "UI")
                    || (selected_layer.contains("enemy") && template.category == "AI")
                    || (selected_layer.contains("player") && template.category == "Player")
            })
            .collect::<Vec<_>>();
        templates.sort_by_key(|template| template.complexity);
        templates
    }

    pub fn instantiate(&self, name: &str) -> Option<BlueprintGraph2D> {
        if !self.templates.iter().any(|template| template.name == name) {
            return None;
        }
        Some(template_graph(name))
    }
}

fn template_tags(name: &str, category: &str) -> Vec<String> {
    let mut tags = vec![category.to_lowercase()];
    tags.extend(
        name.split_whitespace()
            .map(|part| part.trim_matches('+').to_lowercase())
            .filter(|part| !part.is_empty()),
    );
    tags.sort();
    tags.dedup();
    tags
}

fn template_complexity(name: &str) -> u8 {
    match name {
        "Main Menu Flow" | "RTS Spawner" | "Door + Key" | "Settings Menu" => 3,
        "Enemy Chase" | "Enemy Shooter" | "Health System" | "Input Remap Row" => 2,
        _ => 1,
    }
}

fn recommended_assets_for_category(category: &str) -> Vec<String> {
    match category {
        "Player" => vec!["Sprite2D", "InputMap", "Camera2D"],
        "AI" => vec!["BehaviorTree2D", "Navigation2D", "Sprite2D"],
        "UI" => vec!["UiLayout2D", "Font", "WidgetStyle"],
        "Persistence" => vec!["SaveSchema2D"],
        _ => vec!["Prefab2D"],
    }
    .into_iter()
    .map(str::to_string)
    .collect()
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
        ("Main Menu Flow", "UI"),
        ("Pause Menu", "UI"),
        ("Settings Menu", "UI"),
        ("Widget Visibility Toggle", "UI"),
        ("Input Remap Row", "UI"),
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
        "Main Menu Flow" => "OpenMenu",
        "Pause Menu" => "OpenMenu",
        "Settings Menu" => "OpenMenu",
        "Widget Visibility Toggle" => "SetWidgetVisibility",
        "Input Remap Row" => "BindInputAction",
        "Save Point" => "SaveGame",
        _ => "PrintString",
    };
    BlueprintGraph2D {
        name: name.replace([' ', '+'], "_"),
        runtime: "miniforge_visual_script_2d".to_string(),
        asset_kind: "BlueprintClass".to_string(),
        parent_class: "Actor2D".to_string(),
        graph_type: "EventGraph".to_string(),
        class_settings: BlueprintClassSettings2D {
            blueprint_type: "Normal".to_string(),
            category: "Template".to_string(),
            description: format!("Template {name} para MiniForge2D."),
            tick_enabled: true,
            ..Default::default()
        },
        components: vec![BlueprintComponent2D {
            name: "Root".to_string(),
            component_type: "Transform2D".to_string(),
            editable: true,
            exposed_as_variable: true,
            ..Default::default()
        }],
        interfaces: Vec::new(),
        event_dispatchers: BTreeMap::new(),
        macros: BTreeMap::new(),
        variables: BTreeMap::from([(
            "Enabled".to_string(),
            BlueprintVariable2D {
                value_type: "bool".to_string(),
                default_value: json!(true),
                editable: true,
                category: "Template".to_string(),
                ..Default::default()
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
                data: json!({
                    "message": name,
                    "seconds": 0.25,
                    "state": "Idle",
                    "text": name,
                    "menu": name,
                    "widget": "MenuPanel",
                    "action": "Submit"
                }),
            },
        ],
        edges: vec![BlueprintEdge2D {
            from: "begin_play".to_string(),
            from_pin: "then".to_string(),
            to: "template_action".to_string(),
            to_pin: "exec".to_string(),
        }],
        comments: Vec::new(),
    }
}

fn pin(name: &str, pin_type: &str, direction: &str) -> BlueprintPin2D {
    BlueprintPin2D {
        name: name.to_string(),
        pin_type: pin_type.to_string(),
        direction: direction.to_string(),
        ..Default::default()
    }
}
