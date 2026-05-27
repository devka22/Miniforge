use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::miniforge_2d::validation::value_kind;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailsField2D {
    pub path: String,
    pub label: String,
    pub value_type: String,
    pub editable: bool,
    pub value_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailsSection2D {
    pub title: String,
    pub fields: Vec<DetailsField2D>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailsInspector2D {
    pub target_id: Option<u64>,
    pub target_name: String,
    pub sections: Vec<DetailsSection2D>,
}

pub fn supported_field_types() -> Vec<&'static str> {
    vec![
        "int",
        "float",
        "bool",
        "string",
        "color",
        "enum",
        "vector2",
        "asset_reference",
        "entity_reference",
        "component_reference",
        "file_path",
    ]
}

pub fn supported_inspector_actions() -> Vec<&'static str> {
    vec![
        "add_component",
        "remove_component",
        "reset_component",
        "copy_component",
        "paste_component",
        "foldouts",
        "tooltips",
        "warnings",
        "undo_redo",
        "save_scene_or_prefab",
    ]
}

impl DetailsInspector2D {
    pub fn from_entity(entity: &GameObject) -> Self {
        let mut sections = vec![DetailsSection2D {
            title: "Actor".to_string(),
            fields: vec![
                field("name", "Name", "string", true, &entity.name),
                field(
                    "enabled",
                    "Enabled",
                    "bool",
                    true,
                    &entity.enabled.to_string(),
                ),
                field("tag", "Tag", "string", true, &entity.tag),
                field("layer", "Layer", "string", true, &entity.layer),
            ],
        }];
        sections.push(DetailsSection2D {
            title: "Transform".to_string(),
            fields: vec![
                number_field("x", "X", entity.x),
                number_field("y", "Y", entity.y),
                number_field("rotation", "Rotation", entity.rotation),
                number_field("scale_x", "Scale X", entity.scale_x),
                number_field("scale_y", "Scale Y", entity.scale_y),
            ],
        });
        for component in &entity.components {
            let mut fields = vec![field(
                &format!("components.{}.enabled", component.component_type),
                "Enabled",
                "bool",
                true,
                &component.enabled.to_string(),
            )];
            for (key, value) in &component.data {
                fields.push(value_field(
                    &format!("components.{}.{}", component.component_type, key),
                    key,
                    value,
                ));
            }
            sections.push(DetailsSection2D {
                title: component.component_type.clone(),
                fields,
            });
        }
        Self {
            target_id: Some(entity.id),
            target_name: entity.name.clone(),
            sections,
        }
    }

    pub fn from_asset(path: impl Into<String>, metadata: &Value) -> Self {
        let path = path.into();
        let fields = metadata
            .as_object()
            .map(|map| {
                map.iter()
                    .map(|(key, value)| value_field(key, key, value))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Self {
            target_id: None,
            target_name: path,
            sections: vec![DetailsSection2D {
                title: "Asset".to_string(),
                fields,
            }],
        }
    }

    pub fn editable_field_count(&self) -> usize {
        self.sections
            .iter()
            .flat_map(|section| &section.fields)
            .filter(|field| field.editable)
            .count()
    }
}

fn field(
    path: &str,
    label: &str,
    value_type: &str,
    editable: bool,
    value_preview: &str,
) -> DetailsField2D {
    DetailsField2D {
        path: path.to_string(),
        label: label.to_string(),
        value_type: value_type.to_string(),
        editable,
        value_preview: value_preview.to_string(),
    }
}

fn number_field(path: &str, label: &str, value: f64) -> DetailsField2D {
    field(path, label, "number", true, &format!("{value:.3}"))
}

fn value_field(path: &str, label: &str, value: &Value) -> DetailsField2D {
    let preview = match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(items) => format!("{} items", items.len()),
        Value::Object(map) => format!("{} keys", map.len()),
    };
    field(path, label, value_kind(value), true, &preview)
}
