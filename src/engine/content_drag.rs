use serde_json::json;

use crate::engine::asset_database::AssetRecord;
use crate::engine::component::default_component;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragAssetKind {
    Sprite,
    Audio,
    Material,
    Prefab,
    VisualGraph,
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
pub enum DropOutcome {
    SpawnedEntity(u64),
    AppliedToEntity(u64),
    Unsupported(String),
}

impl DragPayload {
    pub fn from_asset(asset: &AssetRecord) -> Self {
        Self {
            relative_path: asset.relative_path.clone(),
            name: asset.name.clone(),
            asset_type: asset.asset_type.clone(),
            guid: asset.guid.clone(),
            kind: match asset.asset_type.as_str() {
                "Sprite" => DragAssetKind::Sprite,
                "Audio" => DragAssetKind::Audio,
                "Material" | "Shader" => DragAssetKind::Material,
                "Prefab" => DragAssetKind::Prefab,
                "VisualGraph" => DragAssetKind::VisualGraph,
                _ => DragAssetKind::Other,
            },
        }
    }
}

pub struct ContentDropper;

impl ContentDropper {
    pub fn spawn_from_payload(payload: &DragPayload, x: f64, y: f64) -> Option<GameObject> {
        match payload.kind {
            DragAssetKind::Sprite => Some(sprite_entity(payload, x, y)),
            DragAssetKind::Audio => Some(audio_entity(payload, x, y)),
            DragAssetKind::Material | DragAssetKind::VisualGraph | DragAssetKind::Other => {
                Some(marker_entity(payload, x, y))
            }
            DragAssetKind::Prefab => None,
        }
    }

    pub fn apply_to_entity(entity: &mut GameObject, payload: &DragPayload) -> DropOutcome {
        match payload.kind {
            DragAssetKind::Sprite => {
                entity.sprite_name = Some(payload.name.clone());
                entity.sprite_guid = Some(payload.guid.clone());
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("sprite_name", json!(payload.name));
                    sprite.set("sprite_guid", json!(payload.guid));
                    sprite.set("asset_path", json!(payload.relative_path));
                }
                entity.sync_to_components();
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Audio => {
                if entity.get_component("AudioSource").is_none()
                    && let Some(audio) = default_component("AudioSource")
                {
                    entity.add_component(audio);
                }
                if let Some(audio) = entity.get_component_mut("AudioSource") {
                    audio.set("audio_name", json!(payload.name));
                    audio.set("asset_guid", json!(payload.guid));
                    audio.set("asset_path", json!(payload.relative_path));
                }
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Material => {
                if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
                    sprite.set("material", json!(payload.name));
                    sprite.set("material_guid", json!(payload.guid));
                    sprite.set("material_path", json!(payload.relative_path));
                    return DropOutcome::AppliedToEntity(entity.id);
                }
                DropOutcome::Unsupported("Entity has no SpriteRenderer".to_string())
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
                DropOutcome::AppliedToEntity(entity.id)
            }
            DragAssetKind::Prefab => {
                DropOutcome::Unsupported("Prefab is instantiated by Game".to_string())
            }
            DragAssetKind::Other => DropOutcome::Unsupported(format!(
                "{} cannot be dropped directly",
                payload.asset_type
            )),
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
        sprite.set("visible", json!(true));
    }
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
        audio.set("play_on_start", json!(false));
        audio.set("spatial", json!(true));
    }
    entity.sync_to_components();
    entity
}

fn marker_entity(payload: &DragPayload, x: f64, y: f64) -> GameObject {
    let mut entity = GameObject::new(x, y, Some(payload.name.clone()));
    entity.layer = "Assets".to_string();
    entity.tag = payload.asset_type.clone();
    if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
        sprite.set("asset_path", json!(payload.relative_path));
        sprite.set("asset_guid", json!(payload.guid));
    }
    entity.sync_to_components();
    entity
}
