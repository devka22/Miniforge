use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engine::component::default_component;
use crate::engine::material_system::TextureSlot2D;
use crate::engine::miniforge_2d::content_browser::ContentAsset2D;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneAssetBindingReport {
    pub entity_id: u64,
    pub entity_name: String,
    pub asset_path: String,
    pub asset_type: String,
    #[serde(default)]
    pub created_components: Vec<String>,
    #[serde(default)]
    pub updated_fields: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EditorAssetConnector;

impl EditorAssetConnector {
    pub fn apply_content_asset(
        entity: &mut GameObject,
        asset: &ContentAsset2D,
    ) -> SceneAssetBindingReport {
        match asset.asset_type.as_str() {
            "Sprite2D" | "SpriteSheet" => {
                Self::assign_sprite(entity, &asset.path, Some(asset.guid.as_str()))
            }
            "AnimationBlueprint2D" | "FlipbookAnimation2D" | "Animation" => {
                Self::assign_animation(entity, &asset.path)
            }
            "BlueprintGraph2D" => Self::attach_visual_graph(entity, &asset.path),
            "Script" | "LuauScript" => Self::assign_script(entity, &asset.path),
            "Material" | "Material2D" => {
                Self::assign_material(entity, &asset.path, Some(asset.guid.as_str()))
            }
            "Shader" => Self::assign_shader(entity, &asset.path),
            "Texture" | "Texture2D" | "Image" | "ImageTexture2D" => {
                Self::assign_texture(entity, &asset.path)
            }
            _ => unsupported(entity, &asset.path, &asset.asset_type),
        }
    }

    pub fn assign_sprite(
        entity: &mut GameObject,
        sprite_path: &str,
        sprite_guid: Option<&str>,
    ) -> SceneAssetBindingReport {
        let mut report = base_report(entity, sprite_path, "Sprite2D");
        if entity.get_component("SpriteRenderer").is_none()
            && let Some(component) = default_component("SpriteRenderer")
        {
            entity.add_component(component);
            report.created_components.push("SpriteRenderer".to_string());
        }
        let sprite_name = asset_stem(sprite_path);
        entity.sprite_name = Some(sprite_name.clone());
        entity.sprite_guid = sprite_guid.map(ToString::to_string);
        if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
            sprite.set("sprite_name", json!(sprite_name));
            sprite.set("sprite_path", json!(sprite_path));
            sprite.set("sprite_guid", json!(sprite_guid));
            sprite.set("visible", json!(true));
            sprite.set("source_asset", json!(sprite_path));
            report.updated_fields.extend(
                [
                    "SpriteRenderer.sprite_name",
                    "SpriteRenderer.sprite_path",
                    "SpriteRenderer.sprite_guid",
                    "SpriteRenderer.visible",
                ]
                .into_iter()
                .map(ToString::to_string),
            );
        } else {
            report
                .warnings
                .push("No se pudo crear SpriteRenderer.".to_string());
        }
        entity.sync_to_components();
        report
    }

    pub fn assign_animation(
        entity: &mut GameObject,
        animation_path: &str,
    ) -> SceneAssetBindingReport {
        let mut report = base_report(entity, animation_path, "Animation");
        for component_type in ["Animator", "Animator2D"] {
            if entity.get_component(component_type).is_none()
                && let Some(component) = default_component(component_type)
            {
                entity.add_component(component);
                report.created_components.push(component_type.to_string());
            }
        }
        let clip_name = asset_stem(animation_path);
        if let Some(animator) = entity.get_component_mut("Animator") {
            animator.set("controller", json!(clip_name));
            animator.set("current_state", json!("Idle"));
            animator.set("preview", json!(true));
            animator.set("apply_sprite", json!(true));
            report.updated_fields.extend(
                [
                    "Animator.controller",
                    "Animator.current_state",
                    "Animator.preview",
                ]
                .into_iter()
                .map(ToString::to_string),
            );
        }
        if let Some(animator2d) = entity.get_component_mut("Animator2D") {
            animator2d.set("animation_blueprint", json!(animation_path));
            animator2d.set("current_state", json!("Idle"));
            animator2d.set("preview", json!(true));
            report.updated_fields.extend(
                [
                    "Animator2D.animation_blueprint",
                    "Animator2D.current_state",
                    "Animator2D.preview",
                ]
                .into_iter()
                .map(ToString::to_string),
            );
        }
        report
    }

    pub fn attach_visual_graph(
        entity: &mut GameObject,
        graph_path: &str,
    ) -> SceneAssetBindingReport {
        let mut report = base_report(entity, graph_path, "BlueprintGraph2D");
        if entity.get_component("VisualScript").is_none()
            && let Some(component) = default_component("VisualScript")
        {
            entity.add_component(component);
            report.created_components.push("VisualScript".to_string());
        }
        if let Some(visual_script) = entity.get_component_mut("VisualScript") {
            visual_script.set("graph_name", json!(asset_stem(graph_path)));
            visual_script.set("graph_path", json!(graph_path));
            visual_script.set("run_in_editor", json!(false));
            report.updated_fields.extend(
                [
                    "VisualScript.graph_name",
                    "VisualScript.graph_path",
                    "VisualScript.run_in_editor",
                ]
                .into_iter()
                .map(ToString::to_string),
            );
        }
        if !entity.scripts.iter().any(|script| {
            script
                .get("path")
                .and_then(|value| value.as_str())
                .is_some_and(|path| path == graph_path)
        }) {
            entity
                .scripts
                .push(json!({"runtime": "visual_graph", "path": graph_path}));
            report.updated_fields.push("scripts".to_string());
        }
        report
    }

    pub fn assign_script(entity: &mut GameObject, script_path: &str) -> SceneAssetBindingReport {
        let mut report = base_report(entity, script_path, "Script");
        entity.script = Some(script_path.to_string());
        if entity.get_component("ScriptComponent").is_none()
            && let Some(component) = default_component("ScriptComponent")
        {
            entity.add_component(component);
            report
                .created_components
                .push("ScriptComponent".to_string());
        }
        if let Some(script) = entity.get_component_mut("ScriptComponent") {
            script.set("runtime", json!("luau"));
            script.set("path", json!(script_path));
            script.set("hot_reload", json!(true));
            report.updated_fields.extend(
                [
                    "script",
                    "ScriptComponent.runtime",
                    "ScriptComponent.path",
                    "ScriptComponent.hot_reload",
                ]
                .into_iter()
                .map(ToString::to_string),
            );
        }
        report
    }

    pub fn assign_material(
        entity: &mut GameObject,
        material_path: &str,
        material_guid: Option<&str>,
    ) -> SceneAssetBindingReport {
        let mut report = base_report(entity, material_path, "Material2D");
        ensure_component(entity, "Material2D", &mut report);
        ensure_component(entity, "SpriteRenderer", &mut report);
        let material_name = asset_stem(material_path);
        if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
            sprite.set("material", json!(material_name.clone()));
            sprite.set("material_path", json!(material_path));
            sprite.set("material_guid", json!(material_guid));
            report.updated_fields.extend(
                [
                    "SpriteRenderer.material",
                    "SpriteRenderer.material_path",
                    "SpriteRenderer.material_guid",
                ]
                .into_iter()
                .map(ToString::to_string),
            );
        }
        if let Some(material) = entity.get_component_mut("Material2D") {
            material.set("material", json!(material_name));
            material.set("material_path", json!(material_path));
            material.set("material_guid", json!(material_guid));
            report.updated_fields.extend(
                [
                    "Material2D.material",
                    "Material2D.material_path",
                    "Material2D.material_guid",
                ]
                .into_iter()
                .map(ToString::to_string),
            );
        }
        report
    }

    pub fn assign_shader(entity: &mut GameObject, shader_path: &str) -> SceneAssetBindingReport {
        let mut report = base_report(entity, shader_path, "Shader");
        ensure_component(entity, "Material2D", &mut report);
        if let Some(material) = entity.get_component_mut("Material2D") {
            material.set("shader", json!(shader_path));
            material.set("shader_path", json!(shader_path));
            report.updated_fields.extend(
                ["Material2D.shader", "Material2D.shader_path"]
                    .into_iter()
                    .map(ToString::to_string),
            );
        }
        report
    }

    pub fn assign_texture(entity: &mut GameObject, texture_path: &str) -> SceneAssetBindingReport {
        let slot = TextureSlot2D::infer_from_path(texture_path);
        let mut report = base_report(entity, texture_path, "Texture2D");
        ensure_component(entity, "Material2D", &mut report);
        ensure_component(entity, "SpriteRenderer", &mut report);
        let field_name = slot.field_name();
        if let Some(material) = entity.get_component_mut("Material2D") {
            material.set(field_name.clone(), json!(texture_path));
            if matches!(slot, TextureSlot2D::BaseColor) {
                material.set("texture", json!(texture_path));
            }
            material.set("last_texture_slot", json!(field_name.clone()));
            report
                .updated_fields
                .push(format!("Material2D.{field_name}"));
            if matches!(slot, TextureSlot2D::BaseColor) {
                report.updated_fields.push("Material2D.texture".to_string());
            }
            report
                .updated_fields
                .push("Material2D.last_texture_slot".to_string());
        }
        if matches!(slot, TextureSlot2D::BaseColor)
            && let Some(sprite) = entity.get_component_mut("SpriteRenderer")
        {
            sprite.set("texture_path", json!(texture_path));
            sprite.set("source_asset", json!(texture_path));
            report
                .updated_fields
                .push("SpriteRenderer.texture_path".to_string());
            report
                .updated_fields
                .push("SpriteRenderer.source_asset".to_string());
        }
        report
    }
}

fn base_report(entity: &GameObject, asset_path: &str, asset_type: &str) -> SceneAssetBindingReport {
    SceneAssetBindingReport {
        entity_id: entity.id,
        entity_name: entity.name.clone(),
        asset_path: asset_path.to_string(),
        asset_type: asset_type.to_string(),
        created_components: Vec::new(),
        updated_fields: Vec::new(),
        warnings: Vec::new(),
    }
}

fn unsupported(entity: &GameObject, asset_path: &str, asset_type: &str) -> SceneAssetBindingReport {
    let mut report = base_report(entity, asset_path, asset_type);
    report.warnings.push(format!(
        "Asset type '{asset_type}' aun no tiene binding directo a escena."
    ));
    report
}

fn ensure_component(
    entity: &mut GameObject,
    component_type: &str,
    report: &mut SceneAssetBindingReport,
) {
    if entity.get_component(component_type).is_none()
        && let Some(component) = default_component(component_type)
    {
        entity.add_component(component);
        report.created_components.push(component_type.to_string());
    }
}

fn asset_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("asset")
        .to_string()
}
