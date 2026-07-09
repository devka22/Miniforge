use serde_json::{Value, json};

use crate::engine::component::{component_from_data, default_component};
use crate::engine::editor_asset_connector::{EditorAssetConnector, SceneAssetBindingReport};
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct InspectorEditor;

#[derive(Debug, Clone, PartialEq)]
pub struct InspectorField {
    pub target: String,
    pub key: String,
    pub value: Value,
    pub value_type: String,
    pub editable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorQuickAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub target_component: Option<String>,
}

impl InspectorEditor {
    pub fn set_name(entity: &mut GameObject, name: &str) {
        entity.name = name.to_string();
    }

    pub fn set_position(entity: &mut GameObject, x: f64, y: f64) {
        entity.x = x;
        entity.y = y;
        entity.sync_to_components();
    }

    pub fn editable_fields(entity: &GameObject) -> Vec<InspectorField> {
        let mut fields = vec![
            field("Transform", "name", json!(entity.name), "string"),
            field("Transform", "x", json!(entity.x), "number"),
            field("Transform", "y", json!(entity.y), "number"),
            field("Transform", "rotation", json!(entity.rotation), "number"),
            field("Transform", "scale_x", json!(entity.scale_x), "number"),
            field("Transform", "scale_y", json!(entity.scale_y), "number"),
            field("Transform", "width", json!(entity.width), "number"),
            field("Transform", "height", json!(entity.height), "number"),
            field("Transform", "speed", json!(entity.speed), "number"),
            field("Transform", "radius", json!(entity.radius), "number"),
            field("Identity", "tag", json!(entity.tag), "string"),
            field("Identity", "layer", json!(entity.layer), "string"),
            field("Identity", "enabled", json!(entity.enabled), "bool"),
            field("Identity", "active", json!(entity.active), "bool"),
            field("Identity", "visible", json!(entity.visible), "bool"),
            field("Identity", "locked", json!(entity.locked), "bool"),
            field("Assets", "sprite_name", json!(entity.sprite_name), "string"),
            field("Assets", "sprite_guid", json!(entity.sprite_guid), "string"),
            field("Assets", "script", json!(entity.script), "string"),
        ];
        for component in &entity.components {
            fields.push(InspectorField {
                target: component.component_type.clone(),
                key: "enabled".to_string(),
                value: json!(component.enabled),
                value_type: "bool".to_string(),
                editable: true,
            });
            for (key, value) in &component.data {
                fields.push(InspectorField {
                    target: component.component_type.clone(),
                    key: key.clone(),
                    value: value.clone(),
                    value_type: value_type(value),
                    editable: !matches!(value, Value::Array(_) | Value::Object(_)),
                });
            }
            if component.component_type == "ScriptComponent"
                && let Some(public_variables) =
                    component.get("public_variables").and_then(Value::as_object)
            {
                for (key, value) in public_variables {
                    fields.push(InspectorField {
                        target: "ScriptVariables".to_string(),
                        key: key.clone(),
                        value: value.clone(),
                        value_type: value_type(value),
                        editable: !matches!(value, Value::Array(_) | Value::Object(_)),
                    });
                }
            }
        }
        fields
    }

    pub fn set_root_value(
        entity: &mut GameObject,
        key: &str,
        value: Value,
    ) -> Result<Value, String> {
        let previous = match key {
            "name" => json!(entity.name),
            "x" => json!(entity.x),
            "y" => json!(entity.y),
            "rotation" => json!(entity.rotation),
            "scale_x" => json!(entity.scale_x),
            "scale_y" => json!(entity.scale_y),
            "width" => json!(entity.width),
            "height" => json!(entity.height),
            "speed" => json!(entity.speed),
            "radius" => json!(entity.radius),
            "tag" => json!(entity.tag),
            "layer" => json!(entity.layer),
            "enabled" => json!(entity.enabled),
            "active" => json!(entity.active),
            "visible" => json!(entity.visible),
            "locked" => json!(entity.locked),
            "sprite_name" => json!(entity.sprite_name),
            "sprite_guid" => json!(entity.sprite_guid),
            "script" => json!(entity.script),
            _ => return Err(format!("Campo root no editable: {key}")),
        };
        let coerced = coerce_like(&previous, value)?;
        match key {
            "name" => entity.name = coerced.as_str().unwrap_or(&entity.name).to_string(),
            "x" => entity.x = coerced.as_f64().unwrap_or(entity.x),
            "y" => entity.y = coerced.as_f64().unwrap_or(entity.y),
            "rotation" => entity.rotation = coerced.as_f64().unwrap_or(entity.rotation),
            "scale_x" => entity.scale_x = coerced.as_f64().unwrap_or(entity.scale_x).max(0.01),
            "scale_y" => entity.scale_y = coerced.as_f64().unwrap_or(entity.scale_y).max(0.01),
            "width" => entity.width = coerced.as_f64().unwrap_or(entity.width).max(0.01),
            "height" => entity.height = coerced.as_f64().unwrap_or(entity.height).max(0.01),
            "speed" => entity.speed = coerced.as_f64().unwrap_or(entity.speed).max(0.0),
            "radius" => entity.radius = coerced.as_f64().unwrap_or(entity.radius).max(0.0),
            "tag" => entity.tag = coerced.as_str().unwrap_or(&entity.tag).to_string(),
            "layer" => entity.layer = coerced.as_str().unwrap_or(&entity.layer).to_string(),
            "enabled" => entity.enabled = coerced.as_bool().unwrap_or(entity.enabled),
            "active" => entity.active = coerced.as_bool().unwrap_or(entity.active),
            "visible" => entity.visible = coerced.as_bool().unwrap_or(entity.visible),
            "locked" => entity.locked = coerced.as_bool().unwrap_or(entity.locked),
            "sprite_name" => entity.sprite_name = optional_string(&coerced),
            "sprite_guid" => entity.sprite_guid = optional_string(&coerced),
            "script" => entity.script = optional_string(&coerced),
            _ => {}
        }
        entity.sync_to_components();
        Ok(previous)
    }

    pub fn set_component_value(
        entity: &mut GameObject,
        component_type: &str,
        key: &str,
        value: Value,
    ) -> Result<Value, String> {
        let Some(component) = entity.get_component_mut(component_type) else {
            return Err(format!("Componente no existe: {component_type}"));
        };
        if key == "enabled" {
            let previous = json!(component.enabled);
            let coerced = coerce_like(&previous, value)?;
            component.enabled = coerced.as_bool().unwrap_or(component.enabled);
            return Ok(previous);
        }
        let previous = component.get(key).cloned().unwrap_or(Value::Null);
        let coerced = coerce_like(&previous, value)?;
        component.set(key, coerced);
        entity.sync_from_components();
        Ok(previous)
    }

    pub fn edit_value(
        entity: &mut GameObject,
        target: &str,
        key: &str,
        value: Value,
    ) -> Result<Value, String> {
        match target {
            "Transform" | "Identity" => Self::set_root_value(entity, key, value),
            "ScriptVariables" => Self::set_script_variable(entity, key, value),
            component_type => Self::set_component_value(entity, component_type, key, value),
        }
    }

    pub fn set_script_variable(
        entity: &mut GameObject,
        key: &str,
        value: Value,
    ) -> Result<Value, String> {
        let Some(component) = entity.get_component_mut("ScriptComponent") else {
            return Err("Componente no existe: ScriptComponent".to_string());
        };
        let mut variables = component
            .get("public_variables")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let previous = variables.get(key).cloned().unwrap_or(Value::Null);
        let coerced = coerce_like(&previous, value)?;
        variables.insert(key.to_string(), coerced);
        component.set("public_variables", Value::Object(variables));
        Ok(previous)
    }

    pub fn add_component(entity: &mut GameObject, component_type: &str) -> Result<(), String> {
        let Some(component) = default_component(component_type) else {
            return Err(format!("Componente desconocido: {component_type}"));
        };
        entity.add_component(component);
        entity.sync_from_components();
        Ok(())
    }

    pub fn remove_component(entity: &mut GameObject, component_type: &str) -> Result<(), String> {
        if matches!(
            component_type,
            "Transform" | "Selectable" | "SpriteRenderer" | "Collider2D"
        ) {
            return Err(format!("Componente core protegido: {component_type}"));
        }
        entity.remove_component(component_type);
        entity.sync_from_components();
        Ok(())
    }

    pub fn reset_transform(entity: &mut GameObject) {
        entity.x = 0.0;
        entity.y = 0.0;
        entity.rotation = 0.0;
        entity.scale_x = 1.0;
        entity.scale_y = 1.0;
        entity.width = 1.0;
        entity.height = 1.0;
        entity.radius = 0.45;
        entity.sync_to_components();
    }

    pub fn toggle_component_enabled(
        entity: &mut GameObject,
        component_type: &str,
    ) -> Result<bool, String> {
        let Some(component) = entity.get_component_mut(component_type) else {
            return Err(format!("Componente no existe: {component_type}"));
        };
        component.enabled = !component.enabled;
        Ok(component.enabled)
    }

    pub fn component_summary(entity: &GameObject) -> Vec<String> {
        entity
            .components
            .iter()
            .map(|component| {
                format!(
                    "{}:{}:{}",
                    component.component_type,
                    if component.enabled { "on" } else { "off" },
                    component.data.len()
                )
            })
            .collect()
    }

    pub fn quick_actions(entity: &GameObject) -> Vec<InspectorQuickAction> {
        let mut actions = Vec::new();
        push_action(
            &mut actions,
            if entity.get_component("SpriteRenderer").is_some() {
                "assign_sprite"
            } else {
                "add_sprite_renderer"
            },
            if entity.get_component("SpriteRenderer").is_some() {
                "Assign Sprite"
            } else {
                "Add Sprite Renderer"
            },
            "image",
            Some("SpriteRenderer"),
        );
        push_action(
            &mut actions,
            if entity.get_component("Material2D").is_some() {
                "assign_material"
            } else {
                "add_material2d"
            },
            if entity.get_component("Material2D").is_some() {
                "Assign Material"
            } else {
                "Add Material"
            },
            "swatch-book",
            Some("Material2D"),
        );
        push_action(
            &mut actions,
            "assign_texture_slot",
            "Assign Texture Slot",
            "layers",
            Some("Material2D"),
        );
        push_action(
            &mut actions,
            if entity.get_component("VisualScript").is_some() {
                "open_blueprint"
            } else {
                "attach_blueprint"
            },
            if entity.get_component("VisualScript").is_some() {
                "Open Blueprint"
            } else {
                "Attach Blueprint"
            },
            "git-branch",
            Some("VisualScript"),
        );
        push_action(
            &mut actions,
            if entity.get_component("ScriptComponent").is_some() {
                "open_script"
            } else {
                "attach_script"
            },
            if entity.get_component("ScriptComponent").is_some() {
                "Open Script"
            } else {
                "Attach Script"
            },
            "scroll-text",
            Some("ScriptComponent"),
        );
        push_action(&mut actions, "create_prefab", "Create Prefab", "box", None);
        actions
    }

    pub fn assign_sprite_asset(
        entity: &mut GameObject,
        sprite_path: &str,
        sprite_guid: Option<&str>,
    ) -> SceneAssetBindingReport {
        EditorAssetConnector::assign_sprite(entity, sprite_path, sprite_guid)
    }

    pub fn assign_material_asset(
        entity: &mut GameObject,
        material_path: &str,
        material_guid: Option<&str>,
    ) -> SceneAssetBindingReport {
        EditorAssetConnector::assign_material(entity, material_path, material_guid)
    }

    pub fn assign_texture_asset(
        entity: &mut GameObject,
        texture_path: &str,
    ) -> SceneAssetBindingReport {
        EditorAssetConnector::assign_texture(entity, texture_path)
    }

    pub fn attach_script_asset(
        entity: &mut GameObject,
        script_path: &str,
    ) -> SceneAssetBindingReport {
        EditorAssetConnector::assign_script(entity, script_path)
    }

    pub fn fields_for_target(entity: &GameObject, target: &str) -> Vec<InspectorField> {
        Self::editable_fields(entity)
            .into_iter()
            .filter(|field| field.target == target)
            .collect()
    }

    pub fn component_from_json(data: Value) -> Result<crate::engine::component::Component, String> {
        component_from_data(&data).ok_or_else(|| "JSON de componente invalido".to_string())
    }
}

fn field(target: &str, key: &str, value: Value, value_type: &str) -> InspectorField {
    InspectorField {
        target: target.to_string(),
        key: key.to_string(),
        value,
        value_type: value_type.to_string(),
        editable: true,
    }
}

fn push_action(
    actions: &mut Vec<InspectorQuickAction>,
    id: &str,
    label: &str,
    icon: &str,
    target_component: Option<&str>,
) {
    actions.push(InspectorQuickAction {
        id: id.to_string(),
        label: label.to_string(),
        icon: icon.to_string(),
        target_component: target_component.map(ToString::to_string),
    });
}

fn value_type(value: &Value) -> String {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_string()
}

fn coerce_like(previous: &Value, value: Value) -> Result<Value, String> {
    match previous {
        Value::Null => Ok(value),
        Value::Bool(_) => value
            .as_bool()
            .map(|value| json!(value))
            .or_else(|| {
                value
                    .as_str()
                    .and_then(|text| text.parse::<bool>().ok())
                    .map(|v| json!(v))
            })
            .ok_or_else(|| "Se esperaba bool".to_string()),
        Value::Number(_) => value
            .as_f64()
            .map(|value| json!(value))
            .or_else(|| {
                value
                    .as_str()
                    .and_then(|text| text.parse::<f64>().ok())
                    .map(|v| json!(v))
            })
            .ok_or_else(|| "Se esperaba number".to_string()),
        Value::String(_) => Ok(json!(
            value
                .as_str()
                .map_or_else(|| value.to_string(), ToString::to_string)
        )),
        Value::Array(_) => {
            if value.is_array() {
                Ok(value)
            } else {
                Err("Se esperaba array".to_string())
            }
        }
        Value::Object(_) => {
            if value.is_object() {
                Ok(value)
            } else {
                Err("Se esperaba object".to_string())
            }
        }
    }
}

fn optional_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) if text.trim().is_empty() || text.trim() == "null" => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}
