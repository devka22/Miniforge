use serde_json::json;

use crate::engine::asset_database::AssetRecord;
use crate::engine::component::default_component;
use crate::engine::material_system::TextureSlot2D;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragAssetKind {
    Sprite,
    SpriteSheet,
    SpriteAnimation,
    Audio,
    AudioEvent,
    Material,
    Shader,
    Texture,
    Prefab,
    VisualGraph,
    Script,
    Scene,
    Tilemap,
    Font,
    ParticlePreset,
    Data,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragPayload {
    pub relative_path: String,
    pub name: String,
    pub asset_type: String,
    pub guid: String,
    pub kind: DragAssetKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragPreview2D {
    pub label: String,
    pub icon: String,
    pub accent: String,
    pub detail: String,
    pub compatible_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropOutcome {
    SpawnedEntity(u64),
    AppliedToEntity(u64),
    OpenScene(String),
    Unsupported(String),
}

impl DragPayload {
    pub fn from_asset(asset: &AssetRecord) -> Self {
        Self {
            relative_path: asset.relative_path.clone(),
            name: asset.name.clone(),
            asset_type: asset.asset_type.clone(),
            guid: asset.guid.clone(),
            kind: DragAssetKind::from_asset_type(&asset.asset_type),
        }
    }

    pub fn drop_hint(&self) -> String {
        self.kind.drop_hint(self)
    }

    pub fn can_spawn_entity(&self) -> bool {
        self.kind.can_spawn_entity()
    }

    pub fn can_apply_to_entity(&self) -> bool {
        self.kind.can_apply_to_entity()
    }

    pub fn preview(&self) -> DragPreview2D {
        DragPreview2D {
            label: self.name.clone(),
            icon: self.kind.icon().to_string(),
            accent: self.kind.accent().to_string(),
            detail: self.drop_hint(),
            compatible_targets: self.compatible_drop_targets(),
        }
    }

    pub fn compatible_drop_targets(&self) -> Vec<String> {
        self.kind.compatible_drop_targets(self)
    }

    pub fn preferred_texture_slot(&self) -> Option<TextureSlot2D> {
        (self.kind == DragAssetKind::Texture)
            .then(|| TextureSlot2D::infer_from_path(&self.relative_path))
    }
}

impl DragAssetKind {
    pub fn from_asset_type(asset_type: &str) -> Self {
        match asset_type {
            "Sprite" | "Sprite2D" => Self::Sprite,
            "SpriteSheet" | "SpriteSheet2D" => Self::SpriteSheet,
            "SpriteFrames2D" | "AnimationBlueprint2D" | "FlipbookAnimation2D" => {
                Self::SpriteAnimation
            }
            "Audio" | "Audio2D" => Self::Audio,
            "AudioEvent" => Self::AudioEvent,
            "Material" | "Material2D" => Self::Material,
            "Shader" => Self::Shader,
            "Texture" | "Texture2D" | "Image" | "ImageTexture2D" => Self::Texture,
            "Prefab" | "Prefab2D" => Self::Prefab,
            "VisualGraph" | "BlueprintGraph2D" => Self::VisualGraph,
            "LuauScript" | "Script" => Self::Script,
            "Scene" | "Scene2D" => Self::Scene,
            "Tilemap" | "Tilemap2D" | "Tileset2D" | "Atlas" => Self::Tilemap,
            "Font" => Self::Font,
            "ParticlePreset" | "Particles2D" => Self::ParticlePreset,
            "Data" | "DataAsset2D" => Self::Data,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sprite => "Sprite",
            Self::SpriteSheet => "SpriteSheet",
            Self::SpriteAnimation => "SpriteAnimation",
            Self::Audio => "Audio",
            Self::AudioEvent => "AudioEvent",
            Self::Material => "Material",
            Self::Shader => "Shader",
            Self::Texture => "Texture",
            Self::Prefab => "Prefab",
            Self::VisualGraph => "VisualGraph",
            Self::Script => "Script",
            Self::Scene => "Scene",
            Self::Tilemap => "Tilemap",
            Self::Font => "Font",
            Self::ParticlePreset => "Particles",
            Self::Data => "Data",
            Self::Other => "Asset",
        }
    }

    pub fn drop_hint(self, payload: &DragPayload) -> String {
        match self {
            Self::Prefab => format!("Instantiate prefab {}", payload.name),
            Self::Scene => format!("Open scene {}", payload.name),
            Self::Sprite | Self::SpriteSheet => format!("Spawn or assign sprite {}", payload.name),
            Self::SpriteAnimation => format!("Assign animation {}", payload.name),
            Self::Audio | Self::AudioEvent => format!("Create or assign audio {}", payload.name),
            Self::Material | Self::Shader => format!("Assign material {}", payload.name),
            Self::Texture => {
                let slot = TextureSlot2D::infer_from_path(&payload.relative_path).field_name();
                format!("Assign texture {} to {}", payload.name, slot)
            }
            Self::VisualGraph => format!("Attach visual graph {}", payload.name),
            Self::Script => format!("Attach script {}", payload.name),
            Self::Tilemap => format!("Create tilemap reference {}", payload.name),
            Self::Font => format!("Create UI/font reference {}", payload.name),
            Self::ParticlePreset => format!("Create particle emitter {}", payload.name),
            Self::Data | Self::Other => format!("Create asset reference {}", payload.name),
        }
    }

    pub fn can_spawn_entity(self) -> bool {
        !matches!(self, Self::Prefab | Self::Scene)
    }

    pub fn can_apply_to_entity(self) -> bool {
        matches!(
            self,
            Self::Sprite
                | Self::SpriteSheet
                | Self::SpriteAnimation
                | Self::Audio
                | Self::AudioEvent
                | Self::Material
                | Self::Shader
                | Self::Texture
                | Self::VisualGraph
                | Self::Script
                | Self::Tilemap
                | Self::Font
                | Self::ParticlePreset
                | Self::Data
        )
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Sprite | Self::SpriteSheet => "image",
            Self::SpriteAnimation => "film",
            Self::Texture => "layers",
            Self::Audio | Self::AudioEvent => "volume-2",
            Self::Material | Self::Shader => "swatch-book",
            Self::Prefab => "box",
            Self::VisualGraph => "git-branch",
            Self::Script => "scroll-text",
            Self::Scene => "map",
            Self::Tilemap => "grid-3x3",
            Self::Font => "type",
            Self::ParticlePreset => "sparkles",
            Self::Data | Self::Other => "file-box",
        }
    }

    pub fn accent(self) -> &'static str {
        match self {
            Self::Sprite | Self::SpriteSheet | Self::SpriteAnimation | Self::Texture => "#3b82f6",
            Self::Audio | Self::AudioEvent => "#f59e0b",
            Self::Material | Self::Shader => "#8b5cf6",
            Self::VisualGraph | Self::Script => "#10b981",
            Self::Prefab | Self::Scene => "#64748b",
            Self::Tilemap => "#22c55e",
            Self::Font => "#06b6d4",
            Self::ParticlePreset => "#ec4899",
            Self::Data | Self::Other => "#94a3b8",
        }
    }

    pub fn compatible_drop_targets(self, payload: &DragPayload) -> Vec<String> {
        match self {
            Self::Sprite | Self::SpriteSheet => vec![
                "SceneViewport.spawn_sprite".to_string(),
                "Actor.SpriteRenderer.sprite_path".to_string(),
                "Actor.Material2D.base_color_texture".to_string(),
            ],
            Self::SpriteAnimation => vec![
                "Actor.SpriteRenderer.sprite_frames".to_string(),
                "Actor.Animator2D.animation_blueprint".to_string(),
            ],
            Self::Texture => vec![
                "Actor.Material2D.texture".to_string(),
                format!(
                    "Actor.Material2D.{}",
                    TextureSlot2D::infer_from_path(&payload.relative_path).field_name()
                ),
                "ContentBrowser.MaterialEditor.texture_slot".to_string(),
            ],
            Self::Material => vec![
                "Actor.SpriteRenderer.material_path".to_string(),
                "Actor.Material2D.material_path".to_string(),
                "SceneViewport.spawn_material_reference".to_string(),
            ],
            Self::Shader => vec![
                "Actor.Material2D.shader".to_string(),
                "ContentBrowser.MaterialEditor.shader".to_string(),
            ],
            Self::Audio | Self::AudioEvent => vec![
                "Actor.AudioSource.audio_path".to_string(),
                "SceneViewport.spawn_audio_source".to_string(),
            ],
            Self::VisualGraph => vec![
                "Actor.VisualScript.graph_path".to_string(),
                "Actor.VisualGraphComponent.path".to_string(),
            ],
            Self::Script => vec!["Actor.ScriptComponent.path".to_string()],
            Self::Tilemap => vec![
                "Actor.TilemapRenderer2D.tilemap".to_string(),
                "SceneViewport.spawn_tilemap".to_string(),
            ],
            Self::Font => vec!["WidgetCanvas2D.font".to_string()],
            Self::ParticlePreset => vec![
                "Actor.ParticleEmitter.preset".to_string(),
                "SceneViewport.spawn_particle_emitter".to_string(),
            ],
            Self::Data | Self::Other => vec!["Actor.AssetIdentity2D.path".to_string()],
            Self::Prefab => vec!["SceneViewport.instantiate_prefab".to_string()],
            Self::Scene => vec!["Editor.open_scene".to_string()],
        }
    }
}

pub struct ContentDropper;

impl ContentDropper {
    pub fn spawn_from_payload(payload: &DragPayload, x: f64, y: f64) -> Option<GameObject> {
        match payload.kind {
            DragAssetKind::Sprite | DragAssetKind::SpriteSheet => {
                Some(sprite_entity(payload, x, y))
            }
            DragAssetKind::SpriteAnimation => Some(animation_entity(payload, x, y)),
            DragAssetKind::Audio | DragAssetKind::AudioEvent => Some(audio_entity(payload, x, y)),
            DragAssetKind::VisualGraph => Some(visual_graph_entity(payload, x, y)),
            DragAssetKind::Script => Some(script_entity(payload, x, y)),
            DragAssetKind::Tilemap => Some(tilemap_entity(payload, x, y)),
            DragAssetKind::Material | DragAssetKind::Shader | DragAssetKind::Texture => {
                Some(material_entity(payload, x, y))
            }
            DragAssetKind::ParticlePreset => Some(particle_entity(payload, x, y)),
            DragAssetKind::Font | DragAssetKind::Data | DragAssetKind::Other => {
                Some(asset_reference_entity(payload, x, y))
            }
            DragAssetKind::Prefab | DragAssetKind::Scene => None,
        }
    }

    pub fn apply_to_entity(entity: &mut GameObject, payload: &DragPayload) -> DropOutcome {
        match payload.kind {
            DragAssetKind::Sprite | DragAssetKind::SpriteSheet => {
                if entity.get_component("SpriteRenderer").is_none()
                    && let Some(sprite) = default_component("SpriteRenderer")
                {
                    entity.add_component(sprite);
                }
                entity.sprite_name = Some(payload.name.clone());
                entity.sprite_guid = Some(payload.guid.clone());
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("sprite_name", json!(payload.name));
                    sprite.set("sprite_guid", json!(payload.guid));
                    sprite.set("asset_path", json!(payload.relative_path));
                    sprite.set("sprite_path", json!(payload.relative_path));
                    sprite.set("source_asset", json!(payload.relative_path));
                    sprite.set("visible", json!(true));
                    if payload.kind == DragAssetKind::SpriteSheet {
                        sprite.set("sprite_sheet", json!(payload.relative_path));
                    }
                }
                entity.sync_to_components();
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::SpriteAnimation => {
                if entity.get_component("SpriteRenderer").is_none()
                    && let Some(sprite) = default_component("SpriteRenderer")
                {
                    entity.add_component(sprite);
                }
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("sprite_frames", json!(payload.relative_path));
                    sprite.set("animation", json!("default"));
                    sprite.set("active_animation", json!("default"));
                    sprite.set("use_2d_animation", json!(true));
                    sprite.set("animation_guid", json!(payload.guid));
                }
                if entity.get_component("Animator2D").is_none()
                    && let Some(animator) = default_component("Animator2D")
                {
                    entity.add_component(animator);
                }
                if let Some(animator) = entity.get_component_mut("Animator2D") {
                    animator.set("animation_blueprint", json!(payload.relative_path));
                    animator.set("playing", json!(true));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Audio | DragAssetKind::AudioEvent => {
                if entity.get_component("AudioSource").is_none()
                    && let Some(audio) = default_component("AudioSource")
                {
                    entity.add_component(audio);
                }
                if let Some(audio) = entity.get_component_mut("AudioSource") {
                    audio.set("audio_name", json!(payload.name));
                    audio.set("asset_guid", json!(payload.guid));
                    audio.set("asset_path", json!(payload.relative_path));
                    audio.set("audio_path", json!(payload.relative_path));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Material | DragAssetKind::Shader => {
                if entity.get_component("Material2D").is_none()
                    && let Some(material) = default_component("Material2D")
                {
                    entity.add_component(material);
                }
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("material", json!(payload.name));
                    sprite.set("material_guid", json!(payload.guid));
                    sprite.set("material_path", json!(payload.relative_path));
                }
                if let Some(material) = entity.get_component_mut("Material2D") {
                    material.set("material", json!(payload.name));
                    if payload.kind == DragAssetKind::Shader {
                        material.set("shader", json!(payload.relative_path));
                    } else {
                        material.set("material_path", json!(payload.relative_path));
                    }
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Texture => {
                if entity.get_component("Material2D").is_none()
                    && let Some(material) = default_component("Material2D")
                {
                    entity.add_component(material);
                }
                if entity.get_component("SpriteRenderer").is_none()
                    && let Some(sprite) = default_component("SpriteRenderer")
                {
                    entity.add_component(sprite);
                }
                let slot = TextureSlot2D::infer_from_path(&payload.relative_path);
                let field_name = slot.field_name();
                if let Some(material) = entity.get_component_mut("Material2D") {
                    material.set(field_name.clone(), json!(payload.relative_path));
                    if matches!(slot, TextureSlot2D::BaseColor) {
                        material.set("texture", json!(payload.relative_path));
                    }
                    material.set("last_texture_slot", json!(field_name));
                }
                if matches!(slot, TextureSlot2D::BaseColor)
                    && let Some(sprite) = entity.get_component_mut("SpriteRenderer")
                {
                    sprite.set("texture_path", json!(payload.relative_path));
                    sprite.set("source_asset", json!(payload.relative_path));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::VisualGraph => {
                if entity.get_component("VisualScript").is_none()
                    && let Some(visual) = default_component("VisualScript")
                {
                    entity.add_component(visual);
                }
                if let Some(visual) = entity.get_component_mut("VisualScript") {
                    visual.set("graph_name", json!(payload.name));
                    visual.set("graph_path", json!(payload.relative_path));
                }
                if entity.get_component("VisualGraphComponent").is_none()
                    && let Some(component) = default_component("VisualGraphComponent")
                {
                    entity.add_component(component);
                }
                if let Some(component) = entity.get_component_mut("VisualGraphComponent") {
                    component.set("path", json!(payload.relative_path));
                }
                if !entity.scripts.iter().any(|script| {
                    script
                        .get("path")
                        .and_then(|value| value.as_str())
                        .is_some_and(|path| path == payload.relative_path)
                }) {
                    entity.scripts.push(json!({
                        "runtime": "visual_graph",
                        "path": payload.relative_path
                    }));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Script => {
                entity.script = Some(payload.relative_path.clone());
                if entity.get_component("ScriptComponent").is_none()
                    && let Some(script) = default_component("ScriptComponent")
                {
                    entity.add_component(script);
                }
                if let Some(script) = entity.get_component_mut("ScriptComponent") {
                    script.set("runtime", json!("luau"));
                    script.set("path", json!(payload.relative_path));
                    script.set("hot_reload", json!(true));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Tilemap => {
                if entity.get_component("TilemapRenderer2D").is_none()
                    && let Some(tilemap) = default_component("TilemapRenderer2D")
                {
                    entity.add_component(tilemap);
                }
                if let Some(tilemap) = entity.get_component_mut("TilemapRenderer2D") {
                    tilemap.set("tilemap", json!(payload.relative_path));
                    tilemap.set("source_asset", json!(payload.relative_path));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::ParticlePreset => {
                if entity.get_component("ParticleEmitter").is_none()
                    && let Some(particles) = default_component("ParticleEmitter")
                {
                    entity.add_component(particles);
                }
                if let Some(particles) = entity.get_component_mut("ParticleEmitter") {
                    particles.set("preset", json!(payload.relative_path));
                    particles.set("asset_guid", json!(payload.guid));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Font | DragAssetKind::Data | DragAssetKind::Other => {
                if entity.get_component("AssetIdentity2D").is_none()
                    && let Some(identity) = default_component("AssetIdentity2D")
                {
                    entity.add_component(identity);
                }
                if let Some(identity) = entity.get_component_mut("AssetIdentity2D") {
                    identity.set("guid", json!(payload.guid));
                    identity.set("asset_type", json!(payload.asset_type));
                    identity.set("path", json!(payload.relative_path));
                    identity.set("preview", json!(payload.name));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Prefab => {
                DropOutcome::Unsupported("Prefab is instantiated by Game".to_string())
            }
            DragAssetKind::Scene => DropOutcome::OpenScene(payload.relative_path.clone()),
        }
    }
}

fn sprite_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(payload.name.clone()));
    entity.sprite_name = Some(payload.name.clone());
    entity.sprite_guid = Some(payload.guid.clone());
    entity.layer = "Sprites".to_string();
    if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
        sprite.set("sprite_name", json!(payload.name));
        sprite.set("sprite_guid", json!(payload.guid));
        sprite.set("asset_path", json!(payload.relative_path));
        sprite.set("sprite_path", json!(payload.relative_path));
        sprite.set("source_asset", json!(payload.relative_path));
        sprite.set("visible", json!(true));
        if payload.kind == DragAssetKind::SpriteSheet {
            sprite.set("sprite_sheet", json!(payload.relative_path));
        }
    }
    entity.sync_to_components();
    entity
}

fn animation_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(payload.name.clone()));
    entity.layer = "Sprites".to_string();
    ContentDropper::apply_to_entity(&mut entity, payload);
    entity.sync_to_components();
    entity
}

fn audio_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(format!("Audio_{}", payload.name)));
    entity.layer = "Audio".to_string();
    if let Some(audio) = default_component("AudioSource") {
        entity.add_component(audio);
    }
    if let Some(audio) = entity.get_component_mut("AudioSource") {
        audio.set("audio_name", json!(payload.name));
        audio.set("asset_guid", json!(payload.guid));
        audio.set("asset_path", json!(payload.relative_path));
        audio.set("audio_path", json!(payload.relative_path));
        audio.set("play_on_start", json!(false));
        audio.set("spatial", json!(true));
    }
    entity.sync_to_components();
    entity
}

fn visual_graph_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(payload.name.clone()));
    entity.layer = "Scripts".to_string();
    entity.tag = "VisualGraph".to_string();
    ContentDropper::apply_to_entity(&mut entity, payload);
    entity.sync_to_components();
    entity
}

fn script_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(payload.name.clone()));
    entity.layer = "Scripts".to_string();
    entity.tag = "Script".to_string();
    ContentDropper::apply_to_entity(&mut entity, payload);
    entity.sync_to_components();
    entity
}

fn tilemap_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(format!("Tilemap_{}", payload.name)));
    entity.layer = "Tilemaps".to_string();
    entity.tag = "Tilemap".to_string();
    ContentDropper::apply_to_entity(&mut entity, payload);
    entity.sync_to_components();
    entity
}

fn material_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = asset_reference_entity(payload, x, y);
    entity.layer = "Materials".to_string();
    entity.tag = "Material".to_string();
    ContentDropper::apply_to_entity(&mut entity, payload);
    entity.sync_to_components();
    entity
}

fn particle_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(format!("Particles_{}", payload.name)));
    entity.layer = "Effects".to_string();
    entity.tag = "Particles".to_string();
    ContentDropper::apply_to_entity(&mut entity, payload);
    entity.sync_to_components();
    entity
}

fn asset_reference_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(payload.name.clone()));
    entity.layer = "Assets".to_string();
    entity.tag = payload.kind.label().to_string();
    if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
        sprite.set("asset_path", json!(payload.relative_path));
        sprite.set("asset_guid", json!(payload.guid));
        sprite.set("visible", json!(false));
    }
    ContentDropper::apply_to_entity(&mut entity, payload);
    entity.sync_to_components();
    entity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spriteframes_drop_configures_runtime_animation() {
        let payload = DragPayload {
            relative_path: "assets/animations/hero.spriteframes".to_string(),
            name: "HeroFrames".to_string(),
            asset_type: "SpriteFrames2D".to_string(),
            guid: "frames-guid".to_string(),
            kind: DragAssetKind::from_asset_type("SpriteFrames2D"),
        };
        assert_eq!(payload.kind, DragAssetKind::SpriteAnimation);
        let mut entity = GameObject::new(0.0, 0.0, Some("Hero".to_string()));
        assert_eq!(
            ContentDropper::apply_to_entity(&mut entity, &payload),
            DropOutcome::AppliedToEntity(entity.id)
        );
        let sprite = entity.get_component("SpriteRenderer").expect("sprite");
        assert_eq!(
            sprite.get_string("sprite_frames", ""),
            payload.relative_path
        );
        assert!(sprite.get_bool("use_2d_animation", false));
        assert!(entity.get_component("Animator2D").is_some());
    }
}
