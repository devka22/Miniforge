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
    #[serde(default)]
    pub base_color_texture: Option<String>,
    #[serde(default)]
    pub normal_texture: Option<String>,
    #[serde(default)]
    pub roughness_texture: Option<String>,
    #[serde(default)]
    pub metallic_texture: Option<String>,
    #[serde(default)]
    pub emissive_texture: Option<String>,
    #[serde(default)]
    pub texture_parameters: BTreeMap<String, String>,
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
    #[serde(default)]
    pub texture_slots: BTreeMap<String, String>,
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
        for (name, supports_lighting, supports_fog) in [
            ("sprite_lit_2d", true, false),
            ("water_2d", true, true),
            ("distortion_2d", false, false),
            ("fire_2d", false, true),
            ("fog_2d", false, true),
            ("outline_2d", false, false),
            ("bloom_2d", false, false),
            ("pixel_art_2d", false, false),
        ] {
            shaders.insert(
                name.to_string(),
                Shader2D {
                    name: name.to_string(),
                    source: format!("builtin://{name}"),
                    supports_lighting,
                    supports_fog,
                },
            );
        }
        let mut materials = BTreeMap::new();
        materials.insert("Default".to_string(), Material2D::default_sprite("Default"));
        materials.insert("LitSprite".to_string(), Material2D::lit_fog("LitSprite"));
        materials.insert(
            "PixelArt".to_string(),
            Material2D {
                shader: "pixel_art_2d".to_string(),
                texture_parameters: BTreeMap::from([
                    ("palette_size".to_string(), "16".to_string()),
                    ("dither".to_string(), "bayer4x4".to_string()),
                    ("filter".to_string(), "nearest".to_string()),
                ]),
                ..Material2D::default_sprite("PixelArt")
            },
        );
        materials.insert(
            "Water2D".to_string(),
            Material2D {
                shader: "water_2d".to_string(),
                lighting: true,
                fog: true,
                tint: [55, 145, 210, 190],
                texture_parameters: BTreeMap::from([
                    ("wave_strength".to_string(), "0.15".to_string()),
                    ("refraction".to_string(), "0.08".to_string()),
                ]),
                ..Material2D::default_sprite("Water2D")
            },
        );
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
            base_color_texture: read_texture_slot(value, "base_color_texture")
                .or_else(|| read_texture_slot(value, "albedo_texture"))
                .or_else(|| read_texture_slot(value, "texture")),
            normal_texture: read_texture_slot(value, "normal_texture"),
            roughness_texture: read_texture_slot(value, "roughness_texture"),
            metallic_texture: read_texture_slot(value, "metallic_texture"),
            emissive_texture: read_texture_slot(value, "emissive_texture"),
            texture_parameters: read_texture_parameters(value),
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
        if material.normal_texture.is_some() && !material.lighting {
            warnings.push("Normal texture assigned while lighting is disabled".to_string());
        }
        if material.metallic_texture.is_some() && material.shader == "sprite_default" {
            warnings.push("Metallic texture needs a lit shader to be visible".to_string());
        }
        let final_tint = if material.lighting && self.lighting.enabled {
            multiply_rgb(material.tint, self.lighting.ambient)
        } else {
            material.tint
        };
        let texture_slots = material.texture_slots();
        MaterialPreview {
            material: material.name,
            shader: shader
                .map(|shader| shader.name.clone())
                .unwrap_or_else(|| "sprite_default".to_string()),
            final_tint,
            texture_slots,
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

    pub fn assign_texture_to_material(
        &mut self,
        material_name: &str,
        slot: TextureSlot2D,
        texture_path: &str,
    ) -> MaterialPreview {
        let mut material = self
            .materials
            .get(material_name)
            .cloned()
            .unwrap_or_else(|| Material2D::default_sprite(material_name));
        material.assign_texture_slot(slot, texture_path);
        self.upsert_material(material);
        self.preview(material_name)
    }
}

impl Material2D {
    pub fn default_sprite(name: &str) -> Self {
        Self {
            name: name.to_string(),
            shader: "sprite_default".to_string(),
            tint: [255, 255, 255, 255],
            texture: None,
            base_color_texture: None,
            normal_texture: None,
            roughness_texture: None,
            metallic_texture: None,
            emissive_texture: None,
            texture_parameters: BTreeMap::new(),
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
            "base_color_texture": self.base_color_texture,
            "normal_texture": self.normal_texture,
            "roughness_texture": self.roughness_texture,
            "metallic_texture": self.metallic_texture,
            "emissive_texture": self.emissive_texture,
            "texture_parameters": self.texture_parameters,
            "lighting": self.lighting,
            "fog": self.fog,
            "roughness": self.roughness,
            "emission": self.emission,
        })
    }

    pub fn assign_texture_slot(&mut self, slot: TextureSlot2D, texture_path: &str) {
        let texture_path = texture_path.to_string();
        match slot {
            TextureSlot2D::BaseColor => {
                self.texture = Some(texture_path.clone());
                self.base_color_texture = Some(texture_path);
            }
            TextureSlot2D::Normal => self.normal_texture = Some(texture_path),
            TextureSlot2D::Roughness => self.roughness_texture = Some(texture_path),
            TextureSlot2D::Metallic => self.metallic_texture = Some(texture_path),
            TextureSlot2D::Emissive => self.emissive_texture = Some(texture_path),
            TextureSlot2D::Custom(name) => {
                self.texture_parameters.insert(name, texture_path);
            }
        }
    }

    pub fn texture_slots(&self) -> BTreeMap<String, String> {
        let mut slots = BTreeMap::new();
        push_slot(
            &mut slots,
            "base_color",
            self.base_color_texture.as_ref().or(self.texture.as_ref()),
        );
        push_slot(&mut slots, "normal", self.normal_texture.as_ref());
        push_slot(&mut slots, "roughness", self.roughness_texture.as_ref());
        push_slot(&mut slots, "metallic", self.metallic_texture.as_ref());
        push_slot(&mut slots, "emissive", self.emissive_texture.as_ref());
        slots.extend(self.texture_parameters.clone());
        slots
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureSlot2D {
    BaseColor,
    Normal,
    Roughness,
    Metallic,
    Emissive,
    Custom(String),
}

impl TextureSlot2D {
    pub fn infer_from_path(path: &str) -> Self {
        let name = path.to_lowercase();
        if contains_any(&name, &["normal", "_n.", "-n.", "nrm"]) {
            Self::Normal
        } else if contains_any(&name, &["roughness", "rough", "_r.", "-r."]) {
            Self::Roughness
        } else if contains_any(&name, &["metallic", "metalness", "metal"]) {
            Self::Metallic
        } else if contains_any(&name, &["emissive", "emission", "glow"]) {
            Self::Emissive
        } else {
            Self::BaseColor
        }
    }

    pub fn field_name(&self) -> String {
        match self {
            Self::BaseColor => "base_color_texture".to_string(),
            Self::Normal => "normal_texture".to_string(),
            Self::Roughness => "roughness_texture".to_string(),
            Self::Metallic => "metallic_texture".to_string(),
            Self::Emissive => "emissive_texture".to_string(),
            Self::Custom(name) => format!("texture_parameters.{name}"),
        }
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

fn read_texture_slot(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .map(ToString::to_string)
}

fn read_texture_parameters(value: &Value) -> BTreeMap<String, String> {
    value
        .get("texture_parameters")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|path| (key.clone(), path.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn push_slot(slots: &mut BTreeMap<String, String>, name: &str, value: Option<&String>) {
    if let Some(value) = value {
        slots.insert(name.to_string(), value.clone());
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}
