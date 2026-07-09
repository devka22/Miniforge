use std::collections::BTreeMap;

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
    #[serde(default)]
    pub expanded: bool,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailsInspector2D {
    pub target_id: Option<u64>,
    pub target_name: String,
    #[serde(default)]
    pub target_kind: String,
    pub sections: Vec<DetailsSection2D>,
    #[serde(default)]
    pub asset_picker_hints: Vec<AssetPickerHint2D>,
    #[serde(default)]
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPickerHint2D {
    pub field_path: String,
    pub asset_type: String,
    pub action: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneInspectorSummary2D {
    pub scene_name: String,
    pub entity_count: usize,
    pub visible_count: usize,
    pub hidden_count: usize,
    pub locked_count: usize,
    pub prefab_instance_count: usize,
    pub script_count: usize,
    pub component_count: usize,
    pub by_layer: BTreeMap<String, usize>,
    pub by_tag: BTreeMap<String, usize>,
    pub by_component: BTreeMap<String, usize>,
}

pub fn supported_field_types() -> Vec<&'static str> {
    vec![
        "int",
        "float",
        "number",
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
        "search_fields",
        "pin_section",
        "copy_path",
        "reset_transform",
        "open_asset_picker",
        "inspect_scene",
        "bulk_edit_selection",
        "show_component_health",
        "copy_json_path",
        "expand_complex_values",
        "asset_drop_targets",
        "quick_asset_assignments",
        "material_texture_slots",
        "open_blueprint_from_actor",
        "swap_sprite_from_inspector",
    ]
}

impl DetailsInspector2D {
    pub fn from_entity(entity: &GameObject) -> Self {
        let mut sections = vec![section(
            "Actor",
            "box",
            vec![
                field("name", "Name", "string", true, &entity.name),
                field(
                    "enabled",
                    "Enabled",
                    "bool",
                    true,
                    &entity.enabled.to_string(),
                ),
                field(
                    "visible",
                    "Visible",
                    "bool",
                    true,
                    &entity.visible.to_string(),
                ),
                field("tag", "Tag", "string", true, &entity.tag),
                field("layer", "Layer", "string", true, &entity.layer),
                field(
                    "components",
                    "Components",
                    "int",
                    false,
                    &entity.components.len().to_string(),
                ),
            ],
        )];
        sections.push(section(
            "Transform",
            "move",
            vec![
                number_field("x", "X", entity.x),
                number_field("y", "Y", entity.y),
                number_field("rotation", "Rotation", entity.rotation),
                number_field("scale_x", "Scale X", entity.scale_x),
                number_field("scale_y", "Scale Y", entity.scale_y),
                number_field("width", "Width", entity.width),
                number_field("height", "Height", entity.height),
            ],
        ));
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
            sections.push(section(&component.component_type, "component", fields));
        }
        Self {
            target_id: Some(entity.id),
            target_name: entity.name.clone(),
            target_kind: "entity".to_string(),
            sections,
            asset_picker_hints: asset_picker_hints_for_entity(entity),
            recommended_actions: recommended_actions_for_entity(entity),
        }
    }

    pub fn from_scene(scene_name: impl Into<String>, entities: &[GameObject]) -> Self {
        let summary = scene_summary(scene_name.into(), entities);
        let mut layer_fields = summary
            .by_layer
            .iter()
            .map(|(layer, count)| {
                field(
                    &format!("layers.{layer}"),
                    layer,
                    "int",
                    false,
                    &count.to_string(),
                )
            })
            .collect::<Vec<_>>();
        if layer_fields.is_empty() {
            layer_fields.push(field("layers.empty", "No layers", "string", false, "0"));
        }

        let mut tag_fields = summary
            .by_tag
            .iter()
            .map(|(tag, count)| {
                field(
                    &format!("tags.{tag}"),
                    tag,
                    "int",
                    false,
                    &count.to_string(),
                )
            })
            .collect::<Vec<_>>();
        if tag_fields.is_empty() {
            tag_fields.push(field("tags.empty", "No tags", "string", false, "0"));
        }

        let hot_components = summary
            .by_component
            .iter()
            .rev()
            .take(12)
            .map(|(component, count)| {
                field(
                    &format!("components.{component}"),
                    component,
                    "int",
                    false,
                    &count.to_string(),
                )
            })
            .collect::<Vec<_>>();

        Self {
            target_id: None,
            target_name: summary.scene_name.clone(),
            target_kind: "scene".to_string(),
            sections: vec![
                section(
                    "Scene",
                    "scene",
                    vec![
                        field("scene.name", "Name", "string", false, &summary.scene_name),
                        field(
                            "scene.entities",
                            "Entities",
                            "int",
                            false,
                            &summary.entity_count.to_string(),
                        ),
                        field(
                            "scene.visible",
                            "Visible",
                            "int",
                            false,
                            &summary.visible_count.to_string(),
                        ),
                        field(
                            "scene.hidden",
                            "Hidden",
                            "int",
                            false,
                            &summary.hidden_count.to_string(),
                        ),
                        field(
                            "scene.locked",
                            "Locked",
                            "int",
                            false,
                            &summary.locked_count.to_string(),
                        ),
                        field(
                            "scene.prefab_instances",
                            "Prefab Instances",
                            "int",
                            false,
                            &summary.prefab_instance_count.to_string(),
                        ),
                        field(
                            "scene.scripts",
                            "Scripts",
                            "int",
                            false,
                            &summary.script_count.to_string(),
                        ),
                        field(
                            "scene.components",
                            "Components",
                            "int",
                            false,
                            &summary.component_count.to_string(),
                        ),
                    ],
                ),
                section("Layers", "layers", layer_fields),
                section("Tags", "tag", tag_fields),
                section("Components", "component", hot_components),
            ],
            asset_picker_hints: Vec::new(),
            recommended_actions: vec![
                "save_scene".to_string(),
                "validate_scene".to_string(),
                "build_runtime_manifest".to_string(),
                "open_content_browser".to_string(),
            ],
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
            target_kind: "asset".to_string(),
            sections: vec![section("Asset", "file", fields)],
            asset_picker_hints: Vec::new(),
            recommended_actions: vec![
                "open".to_string(),
                "reimport".to_string(),
                "find_references".to_string(),
            ],
        }
    }

    pub fn editable_field_count(&self) -> usize {
        self.sections
            .iter()
            .flat_map(|section| &section.fields)
            .filter(|field| field.editable)
            .count()
    }

    pub fn section_titles(&self) -> Vec<String> {
        self.sections
            .iter()
            .map(|section| section.title.clone())
            .collect()
    }

    pub fn search_fields(&self, query: &str) -> Vec<&DetailsField2D> {
        let query = query.to_lowercase();
        self.sections
            .iter()
            .flat_map(|section| &section.fields)
            .filter(|field| {
                query.is_empty()
                    || field.label.to_lowercase().contains(&query)
                    || field.path.to_lowercase().contains(&query)
                    || field.value_type.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn asset_picker_for(&self, field_path: &str) -> Option<&AssetPickerHint2D> {
        self.asset_picker_hints
            .iter()
            .find(|hint| hint.field_path == field_path)
    }

    pub fn section(&self, title: &str) -> Option<&DetailsSection2D> {
        self.sections.iter().find(|section| section.title == title)
    }

    pub fn editable_paths(&self) -> Vec<String> {
        self.sections
            .iter()
            .flat_map(|section| &section.fields)
            .filter(|field| field.editable)
            .map(|field| field.path.clone())
            .collect()
    }
}

fn asset_picker_hints_for_entity(entity: &GameObject) -> Vec<AssetPickerHint2D> {
    let mut hints = vec![
        AssetPickerHint2D {
            field_path: "components.SpriteRenderer.sprite_path".to_string(),
            asset_type: "Sprite2D".to_string(),
            action: "assign_sprite_to_selected".to_string(),
        },
        AssetPickerHint2D {
            field_path: "components.Animator2D.animation_blueprint".to_string(),
            asset_type: "AnimationBlueprint2D".to_string(),
            action: "assign_animation_to_selected".to_string(),
        },
        AssetPickerHint2D {
            field_path: "components.VisualScript.graph_path".to_string(),
            asset_type: "BlueprintGraph2D".to_string(),
            action: "attach_blueprint_to_selected".to_string(),
        },
    ];
    if entity.get_component("AudioSource").is_some() {
        hints.push(AssetPickerHint2D {
            field_path: "components.AudioSource.audio_name".to_string(),
            asset_type: "Audio2D".to_string(),
            action: "assign_audio_to_selected".to_string(),
        });
    }
    if entity.get_component("Material2D").is_some() {
        hints.push(AssetPickerHint2D {
            field_path: "components.Material2D.material_path".to_string(),
            asset_type: "Material2D".to_string(),
            action: "assign_material_to_selected".to_string(),
        });
        for (field, action) in [
            ("base_color_texture", "assign_base_color_texture"),
            ("normal_texture", "assign_normal_texture"),
            ("roughness_texture", "assign_roughness_texture"),
            ("metallic_texture", "assign_metallic_texture"),
            ("emissive_texture", "assign_emissive_texture"),
        ] {
            hints.push(AssetPickerHint2D {
                field_path: format!("components.Material2D.{field}"),
                asset_type: "Texture2D".to_string(),
                action: action.to_string(),
            });
        }
        hints.push(AssetPickerHint2D {
            field_path: "components.Material2D.shader".to_string(),
            asset_type: "Shader".to_string(),
            action: "assign_shader_to_material".to_string(),
        });
    }
    if entity.get_component("ScriptComponent").is_some() {
        hints.push(AssetPickerHint2D {
            field_path: "components.ScriptComponent.path".to_string(),
            asset_type: "LuauScript".to_string(),
            action: "attach_script_to_selected".to_string(),
        });
    }
    if entity.get_component("TilemapRenderer2D").is_some() {
        hints.push(AssetPickerHint2D {
            field_path: "components.TilemapRenderer2D.tilemap".to_string(),
            asset_type: "Tilemap2D".to_string(),
            action: "assign_tilemap_to_selected".to_string(),
        });
    }
    hints
}

fn recommended_actions_for_entity(entity: &GameObject) -> Vec<String> {
    let mut actions = Vec::new();
    if entity.get_component("SpriteRenderer").is_some() {
        actions.push("assign_sprite_from_content_browser".to_string());
        actions.push("swap_sprite_in_inspector".to_string());
    } else {
        actions.push("add_sprite_renderer".to_string());
    }
    if entity.get_component("Material2D").is_some() {
        actions.push("open_material_slot_panel".to_string());
        actions.push("assign_material_from_content_browser".to_string());
        actions.push("assign_texture_slot_from_content_browser".to_string());
    } else {
        actions.push("add_material2d".to_string());
    }
    if entity.get_component("Animator2D").is_some() || entity.get_component("Animator").is_some() {
        actions.push("assign_animation_blueprint".to_string());
    } else {
        actions.push("add_animator2d".to_string());
    }
    if entity.get_component("VisualScript").is_some() {
        actions.push("open_attached_blueprint".to_string());
    } else {
        actions.push("attach_blueprint_graph".to_string());
    }
    actions.push("create_prefab_from_selection".to_string());
    if entity.locked {
        actions.push("unlock_for_editing".to_string());
    } else {
        actions.push("lock_transform".to_string());
    }
    if entity.visible {
        actions.push("hide_in_scene".to_string());
    } else {
        actions.push("show_in_scene".to_string());
    }
    actions
}

pub fn scene_summary(scene_name: String, entities: &[GameObject]) -> SceneInspectorSummary2D {
    let mut summary = SceneInspectorSummary2D {
        scene_name,
        entity_count: entities.len(),
        ..Default::default()
    };
    for entity in entities {
        if entity.visible {
            summary.visible_count += 1;
        } else {
            summary.hidden_count += 1;
        }
        if entity.locked {
            summary.locked_count += 1;
        }
        if entity.is_prefab_instance {
            summary.prefab_instance_count += 1;
        }
        summary.script_count += entity.scripts.len() + usize::from(entity.script.is_some());
        summary.component_count += entity.components.len();
        *summary.by_layer.entry(entity.layer.clone()).or_insert(0) += 1;
        *summary.by_tag.entry(entity.tag.clone()).or_insert(0) += 1;
        for component in &entity.components {
            *summary
                .by_component
                .entry(component.component_type.clone())
                .or_insert(0) += 1;
        }
    }
    summary
}

fn section(title: &str, icon: &str, fields: Vec<DetailsField2D>) -> DetailsSection2D {
    DetailsSection2D {
        title: title.to_string(),
        fields,
        expanded: true,
        icon: icon.to_string(),
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
