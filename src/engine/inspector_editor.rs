use serde_json::{Value, json};

use crate::engine::component::{component_from_data, default_component};
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
            field("Identity", "tag", json!(entity.tag), "string"),
            field("Identity", "layer", json!(entity.layer), "string"),
            field("Identity", "enabled", json!(entity.enabled), "bool"),
            field("Identity", "visible", json!(entity.visible), "bool"),
        ];
        for component in &entity.components {
            for (key, value) in &component.data {
                fields.push(InspectorField {
                    target: component.component_type.clone(),
                    key: key.clone(),
                    value: value.clone(),
                    value_type: value_type(value),
                    editable: !matches!(value, Value::Array(_) | Value::Object(_)),
                });
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
            "tag" => json!(entity.tag),
            "layer" => json!(entity.layer),
            "enabled" => json!(entity.enabled),
            "visible" => json!(entity.visible),
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
            "tag" => entity.tag = coerced.as_str().unwrap_or(&entity.tag).to_string(),
            "layer" => entity.layer = coerced.as_str().unwrap_or(&entity.layer).to_string(),
            "enabled" => entity.enabled = coerced.as_bool().unwrap_or(entity.enabled),
            "visible" => entity.visible = coerced.as_bool().unwrap_or(entity.visible),
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
            component_type => Self::set_component_value(entity, component_type, key, value),
        }
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
