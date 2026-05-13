use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Shader2D {
    pub name: String,
    pub source: String,
    pub supports_lighting: bool,
    pub supports_fog: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Material2D {
    pub name: String,
    pub shader: String,
    pub tint: [u8; 4],
    pub texture: Option<String>,
    pub lighting: bool,
    pub fog: bool,
    pub roughness: f64,
    pub emission: [u8; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lighting2DSettings {
    pub ambient: [u8; 3],
    pub fog_color: [u8; 4],
    pub fog_density: f64,
    pub enabled: bool,
}

impl Default for Lighting2DSettings {
    fn default() -> Self {
        Self {
            ambient: [190, 198, 210],
            fog_color: [80, 96, 118, 90],
            fog_density: 0.0,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaterialPreview {
    pub material: String,
    pub shader: String,
    pub final_tint: [u8; 4],
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaterialLibrary {
    pub shaders: BTreeMap<String, Shader2D>,
    pub materials: BTreeMap<String, Material2D>,
    pub lighting: Lighting2DSettings,
}

impl Default for MaterialLibrary {
    fn default() -> Self {
        Self::with_builtin_shaders()
    }
}

impl MaterialLibrary {
    pub fn with_builtin_shaders() -> Self {
        let mut shaders = BTreeMap::new();
        shaders.insert(
            "sprite_default".to_string(),
            Shader2D {
                name: "sprite_default".to_string(),
                source: "builtin://sprite_default".to_string(),
                supports_lighting: false,
                supports_fog: false,
            },
        );
        shaders.insert(
            "sprite_lit_fog".to_string(),
            Shader2D {
                name: "sprite_lit_fog".to_string(),
                source: "builtin://sprite_lit_fog".to_string(),
                supports_lighting: true,
                supports_fog: true,
            },
        );
        let mut materials = BTreeMap::new();
        materials.insert("Default".to_string(), Material2D::default_sprite("Default"));
        Self {
            shaders,
            materials,
            lighting: Lighting2DSettings::default(),
        }
    }

    pub fn upsert_material(&mut self, material: Material2D) {
        self.materials.insert(material.name.clone(), material);
    }

    pub fn material_from_value(value: &Value) -> Material2D {
        Material2D {
            name: value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Material")
                .to_string(),
            shader: value
                .get("shader")
                .and_then(Value::as_str)
                .unwrap_or("sprite_default")
                .to_string(),
            tint: parse_color4(value.get("tint"), [255, 255, 255, 255]),
            texture: value
                .get("texture")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            lighting: value
                .get("lighting")
                .or_else(|| value.get("lighting_enabled"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            fog: value
                .get("fog")
                .or_else(|| value.get("fog_enabled"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            roughness: value
                .get("roughness")
                .and_then(Value::as_f64)
                .unwrap_or(0.5),
            emission: parse_color3(value.get("emission"), [0, 0, 0]),
        }
    }

    pub fn preview(&self, material_name: &str) -> MaterialPreview {
        let material = self
            .materials
            .get(material_name)
            .cloned()
            .unwrap_or_else(|| Material2D::default_sprite(material_name));
        let shader = self.shaders.get(&material.shader);
        let mut warnings = Vec::new();
        if shader.is_none() {
            warnings.push(format!(
                "Shader {} missing; falling back to sprite_default",
                material.shader
            ));
        }
        if material.lighting && !shader.is_some_and(|shader| shader.supports_lighting) {
            warnings.push("Material lighting enabled on unlit shader".to_string());
        }
        if material.fog && !shader.is_some_and(|shader| shader.supports_fog) {
            warnings.push("Material fog enabled on shader without fog support".to_string());
        }
        let final_tint = if material.lighting && self.lighting.enabled {
            multiply_rgb(material.tint, self.lighting.ambient)
        } else {
            material.tint
        };
        MaterialPreview {
            material: material.name,
            shader: shader
                .map(|shader| shader.name.clone())
                .unwrap_or_else(|| "sprite_default".to_string()),
            final_tint,
            warnings,
        }
    }

    pub fn apply_to_entity(&self, entity: &mut GameObject, material_name: &str) -> MaterialPreview {
        let preview = self.preview(material_name);
        if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
            sprite.set(
                "tint",
                json!([
                    preview.final_tint[0],
                    preview.final_tint[1],
                    preview.final_tint[2],
                    preview.final_tint[3]
                ]),
            );
            sprite.set("material", json!(preview.material.clone()));
        }
        preview
    }
}

impl Material2D {
    pub fn default_sprite(name: &str) -> Self {
        Self {
            name: name.to_string(),
            shader: "sprite_default".to_string(),
            tint: [255, 255, 255, 255],
            texture: None,
            lighting: false,
            fog: false,
            roughness: 0.5,
            emission: [0, 0, 0],
        }
    }

    pub fn lit_fog(name: &str) -> Self {
        Self {
            shader: "sprite_lit_fog".to_string(),
            lighting: true,
            fog: true,
            ..Self::default_sprite(name)
        }
    }

    pub fn to_value(&self) -> Value {
        json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "kind": "MiniForgeMaterial2D",
            "name": self.name,
            "shader": self.shader,
            "tint": self.tint,
            "texture": self.texture,
            "lighting": self.lighting,
            "fog": self.fog,
            "roughness": self.roughness,
            "emission": self.emission,
        })
    }
}

fn parse_color4(value: Option<&Value>, fallback: [u8; 4]) -> [u8; 4] {
    let Some(items) = value.and_then(Value::as_array) else {
        return fallback;
    };
    let mut out = fallback;
    for (index, item) in items.iter().take(4).enumerate() {
        out[index] = item.as_u64().unwrap_or(out[index] as u64).min(255) as u8;
    }
    out
}

fn parse_color3(value: Option<&Value>, fallback: [u8; 3]) -> [u8; 3] {
    let Some(items) = value.and_then(Value::as_array) else {
        return fallback;
    };
    let mut out = fallback;
    for (index, item) in items.iter().take(3).enumerate() {
        out[index] = item.as_u64().unwrap_or(out[index] as u64).min(255) as u8;
    }
    out
}

fn multiply_rgb(mut tint: [u8; 4], ambient: [u8; 3]) -> [u8; 4] {
    tint[0] = ((tint[0] as u16 * ambient[0] as u16) / 255) as u8;
    tint[1] = ((tint[1] as u16 * ambient[1] as u16) / 255) as u8;
    tint[2] = ((tint[2] as u16 * ambient[2] as u16) / 255) as u8;
    tint
}
