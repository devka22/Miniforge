use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_database::{AssetDatabase, AssetRecord, stable_guid};
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentAsset2D {
    pub guid: String,
    pub path: String,
    pub name: String,
    pub asset_type: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub preview: AssetPreview2D,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub valid: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPreview2D {
    pub preview_type: String,
    pub thumbnail_path: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentFilter2D {
    pub search: String,
    pub asset_types: BTreeSet<String>,
    pub labels: BTreeSet<String>,
    pub include_invalid: bool,
    #[serde(default)]
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContentBrowserCatalog2D {
    pub assets: BTreeMap<String, ContentAsset2D>,
    #[serde(default)]
    pub view_mode: ContentBrowserViewMode2D,
    #[serde(default)]
    pub selected_asset: Option<String>,
    #[serde(default)]
    pub favorites: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentBrowserViewMode2D {
    #[default]
    Grid,
    List,
    Columns,
    Details,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentSortMode2D {
    #[default]
    Name,
    Type,
    Path,
    Size,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentFolderNode2D {
    pub path: String,
    pub name: String,
    pub asset_count: usize,
    #[serde(default)]
    pub children: BTreeMap<String, ContentFolderNode2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentDragIntent2D {
    pub asset_path: String,
    pub asset_type: String,
    pub drop_action: String,
    pub target_hint: String,
    #[serde(default)]
    pub target_component: Option<String>,
    #[serde(default)]
    pub can_spawn: bool,
    #[serde(default)]
    pub can_assign_to_selection: bool,
    #[serde(default)]
    pub action_label: String,
}

impl ContentBrowserCatalog2D {
    pub fn from_asset_database(database: &AssetDatabase) -> Self {
        Self {
            assets: database
                .assets
                .iter()
                .map(|(path, record)| (path.clone(), asset_from_record(record)))
                .collect(),
            view_mode: ContentBrowserViewMode2D::Grid,
            selected_asset: None,
            favorites: BTreeSet::new(),
        }
    }

    pub fn insert_json_asset(
        &mut self,
        path: impl Into<String>,
        asset_type: impl Into<String>,
        metadata: Value,
    ) -> String {
        let path = path.into();
        let asset_type = asset_type.into();
        let guid = stable_guid(&path);
        let name = std::path::Path::new(&path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("asset")
            .to_string();
        self.assets.insert(
            path.clone(),
            ContentAsset2D {
                guid: guid.clone(),
                path,
                name,
                asset_type,
                labels: Vec::new(),
                preview: AssetPreview2D {
                    preview_type: "json".to_string(),
                    thumbnail_path: None,
                    summary: "MiniForge2D JSON asset".to_string(),
                },
                dependencies: Vec::new(),
                valid: true,
                metadata,
            },
        );
        guid
    }

    pub fn select_asset(&mut self, path: &str) -> bool {
        if self.assets.contains_key(path) {
            self.selected_asset = Some(path.to_string());
            true
        } else {
            false
        }
    }

    pub fn selected_asset(&self) -> Option<&ContentAsset2D> {
        self.selected_asset
            .as_ref()
            .and_then(|path| self.assets.get(path))
    }

    pub fn toggle_favorite(&mut self, path: &str) -> bool {
        if !self.assets.contains_key(path) {
            return false;
        }
        if !self.favorites.insert(path.to_string()) {
            self.favorites.remove(path);
        }
        true
    }

    pub fn drag_payload_for_scene(&self, path: &str) -> Option<ContentDragIntent2D> {
        let asset = self.assets.get(path)?;
        let (drop_action, target_hint, target_component, can_spawn, can_assign) = match asset
            .asset_type
            .as_str()
        {
            "Sprite2D" | "SpriteSheet" => (
                "assign_sprite",
                "SpriteRenderer",
                Some("SpriteRenderer"),
                true,
                true,
            ),
            "AnimationBlueprint2D" | "FlipbookAnimation2D" | "SpriteFrames2D" | "Animation" => (
                "assign_animation",
                "Animator2D",
                Some("Animator2D"),
                false,
                true,
            ),
            "BlueprintGraph2D" => (
                "attach_blueprint",
                "VisualScript",
                Some("VisualScript"),
                true,
                true,
            ),
            "LuauScript" | "Script" => (
                "attach_script",
                "ScriptComponent",
                Some("ScriptComponent"),
                true,
                true,
            ),
            "Prefab2D" => ("instantiate_prefab", "Scene", None, true, false),
            "Scene2D" => ("open_scene", "SceneManager", None, false, false),
            "Audio" | "Audio2D" | "AudioEvent" => (
                "assign_audio",
                "AudioSource",
                Some("AudioSource"),
                true,
                true,
            ),
            "Material" | "Material2D" => (
                "assign_material",
                "Material2D",
                Some("Material2D"),
                true,
                true,
            ),
            "Shader" => (
                "assign_shader",
                "Material2D",
                Some("Material2D"),
                true,
                true,
            ),
            "Texture" | "Texture2D" | "Image" | "ImageTexture2D" => (
                "assign_texture_slot",
                "Material2D texture slot",
                Some("Material2D"),
                true,
                true,
            ),
            "Tilemap" | "Tilemap2D" | "Tileset2D" => (
                "attach_tilemap",
                "TilemapRenderer2D",
                Some("TilemapRenderer2D"),
                true,
                true,
            ),
            "ParticlePreset" | "Particles2D" => (
                "attach_particles",
                "ParticleEmitter",
                Some("ParticleEmitter"),
                true,
                true,
            ),
            "Font" => (
                "assign_font",
                "WidgetCanvas2D",
                Some("WidgetCanvas2D"),
                true,
                true,
            ),
            "DataAsset2D" | "Data" => (
                "attach_asset_identity",
                "AssetIdentity2D",
                Some("AssetIdentity2D"),
                true,
                true,
            ),
            _ => ("inspect_asset", "Details", None, false, false),
        };
        Some(ContentDragIntent2D {
            asset_path: asset.path.clone(),
            asset_type: asset.asset_type.clone(),
            drop_action: drop_action.to_string(),
            target_hint: target_hint.to_string(),
            target_component: target_component.map(ToString::to_string),
            can_spawn,
            can_assign_to_selection: can_assign,
            action_label: format!("{} -> {}", asset.name, target_hint),
        })
    }

    pub fn drop_intents_for_selection(
        &self,
        path: &str,
        selected_entity_count: usize,
    ) -> Vec<ContentDragIntent2D> {
        let Some(intent) = self.drag_payload_for_scene(path) else {
            return Vec::new();
        };
        let mut intents = Vec::new();
        if intent.can_assign_to_selection && selected_entity_count > 0 {
            intents.push(ContentDragIntent2D {
                drop_action: format!("{}_to_selection", intent.drop_action),
                target_hint: format!("{} selected", selected_entity_count),
                action_label: format!("Assign to {selected_entity_count} selected"),
                ..intent.clone()
            });
        }
        if intent.can_spawn {
            intents.push(ContentDragIntent2D {
                drop_action: format!("{}_at_cursor", intent.drop_action),
                target_hint: "Scene cursor".to_string(),
                action_label: "Drop at cursor".to_string(),
                ..intent.clone()
            });
        }
        if intents.is_empty() {
            intents.push(intent);
        }
        intents
    }

    pub fn filter(&self, filter: &ContentFilter2D) -> Vec<&ContentAsset2D> {
        let search = filter.search.to_lowercase();
        self.assets
            .values()
            .filter(|asset| filter.include_invalid || asset.valid)
            .filter(|asset| {
                filter
                    .folder
                    .as_ref()
                    .is_none_or(|folder| asset.path.starts_with(folder))
            })
            .filter(|asset| {
                filter.asset_types.is_empty() || filter.asset_types.contains(&asset.asset_type)
            })
            .filter(|asset| {
                filter.labels.is_empty()
                    || asset
                        .labels
                        .iter()
                        .any(|label| filter.labels.contains(label))
            })
            .filter(|asset| {
                search.is_empty()
                    || asset.name.to_lowercase().contains(&search)
                    || asset.path.to_lowercase().contains(&search)
                    || asset.guid.contains(&search)
            })
            .collect()
    }

    pub fn filter_sorted(
        &self,
        filter: &ContentFilter2D,
        sort: ContentSortMode2D,
    ) -> Vec<&ContentAsset2D> {
        let mut assets = self.filter(filter);
        assets.sort_by(|a, b| match sort {
            ContentSortMode2D::Name => a.name.cmp(&b.name).then(a.path.cmp(&b.path)),
            ContentSortMode2D::Type => a.asset_type.cmp(&b.asset_type).then(a.name.cmp(&b.name)),
            ContentSortMode2D::Path => a.path.cmp(&b.path),
            ContentSortMode2D::Size => asset_size(a).cmp(&asset_size(b)).then(a.name.cmp(&b.name)),
        });
        assets
    }

    pub fn folder_tree(&self) -> ContentFolderNode2D {
        let mut root = ContentFolderNode2D {
            path: String::new(),
            name: "Project".to_string(),
            asset_count: self.assets.len(),
            children: BTreeMap::new(),
        };
        for asset in self.assets.values() {
            insert_folder_path(&mut root, &asset.path);
        }
        root
    }

    pub fn assets_in_folder(&self, folder: &str) -> Vec<&ContentAsset2D> {
        let prefix = folder.trim_matches('/');
        self.assets
            .values()
            .filter(|asset| asset.path.starts_with(prefix))
            .collect()
    }

    pub fn quick_actions_for(asset: &ContentAsset2D) -> Vec<&'static str> {
        match asset.asset_type.as_str() {
            "Sprite2D" => vec![
                "open_sprite_editor",
                "preview",
                "assign_to_selected",
                "create_sprite_component",
                "create_flipbook",
                "reimport",
            ],
            "SpriteSheet" => vec![
                "slice_sheet",
                "create_flipbook",
                "assign_first_frame",
                "open_sprite_editor",
                "reimport",
            ],
            "BlueprintGraph2D" => vec![
                "open_blueprint",
                "attach_to_selected",
                "show_compile_summary",
                "duplicate",
            ],
            "AnimationBlueprint2D" | "FlipbookAnimation2D" | "SpriteFrames2D" | "Animation" => {
                vec![
                    "open_animation",
                    "assign_to_selected",
                    "preview",
                    "duplicate",
                    "reimport",
                ]
            }
            "Prefab2D" => vec!["instantiate", "open_prefab", "find_references"],
            "Scene2D" => vec!["open_scene", "load_additive", "set_start_scene"],
            "Audio" | "Audio2D" => vec!["preview_audio", "create_sound_cue", "reimport"],
            "AudioEvent" => vec!["preview_audio", "open_sound_event", "assign_to_selected"],
            "Material" | "Material2D" => {
                vec![
                    "open_material",
                    "open_texture_slots",
                    "assign_to_selected",
                    "create_material_instance",
                    "duplicate",
                    "find_references",
                ]
            }
            "Texture" | "Texture2D" | "Image" | "ImageTexture2D" => {
                vec![
                    "preview_texture",
                    "assign_to_material_slot",
                    "assign_base_color_to_selected",
                    "create_material_from_texture",
                    "reimport",
                ]
            }
            "Shader" => vec!["open_shader", "assign_to_material", "validate_shader"],
            "LuauScript" | "Script" => {
                vec![
                    "open_script",
                    "attach_to_selected",
                    "run_lint",
                    "hot_reload",
                ]
            }
            "Tilemap" | "Tilemap2D" | "Tileset2D" => {
                vec!["open_tilemap", "drop_to_scene", "rebuild_collision"]
            }
            "ParticlePreset" | "Particles2D" => {
                vec!["preview_particles", "attach_to_selected", "drop_emitter"]
            }
            "Font" => vec!["preview_font", "assign_to_ui", "find_references"],
            "DataAsset2D" | "Data" => vec!["open", "attach_asset_identity", "find_references"],
            _ => vec!["open", "rename", "duplicate", "delete"],
        }
    }

    pub fn asset_counts_by_type(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for asset in self.assets.values() {
            *counts.entry(asset.asset_type.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub fn mark_validity(&mut self, path: &str, valid: bool) -> bool {
        let Some(asset) = self.assets.get_mut(path) else {
            return false;
        };
        asset.valid = valid;
        true
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        let paths = self.assets.keys().cloned().collect::<BTreeSet<_>>();
        let mut guids = BTreeSet::new();
        for asset in self.assets.values() {
            if !guids.insert(asset.guid.clone()) {
                report.error(
                    "duplicate_guid",
                    asset.path.clone(),
                    format!("GUID duplicado en Content Browser: {}", asset.guid),
                );
            }
            for dependency in &asset.dependencies {
                if !paths.contains(dependency) {
                    report.warning(
                        "missing_dependency",
                        asset.path.clone(),
                        format!("Dependencia rota: {dependency}"),
                    );
                }
            }
        }
        report
    }

    pub fn to_value(&self) -> Value {
        json!({
            "runtime": "miniforge_content_browser_2d",
            "view_mode": self.view_mode,
            "folder_tree": self.folder_tree(),
            "assets": self.assets,
            "selected_asset": self.selected_asset.clone(),
            "favorites": self.favorites.clone()
        })
    }
}

pub fn supported_content_asset_types() -> Vec<&'static str> {
    crate::engine::miniforge_2d::asset_registry2d::supported_asset_types()
}

pub fn supported_content_operations() -> Vec<&'static str> {
    vec![
        "folder_view",
        "grid_view",
        "list_view",
        "search",
        "filter_by_type",
        "labels",
        "preview",
        "guid",
        "path",
        "size",
        "dependencies",
        "reverse_dependencies",
        "drag_drop_to_scene",
        "assign_sprite_to_selection",
        "assign_animation_to_selection",
        "attach_blueprint_to_selection",
        "double_click_open",
        "create_asset",
        "import_2d_asset_picker",
        "import_sprite_with_animation",
        "rename",
        "duplicate",
        "delete",
        "reimport",
        "broken_references",
        "unused_assets",
        "missing_assets",
        "folder_tree",
        "sort_by_name",
        "sort_by_type",
        "details_view",
        "quick_actions",
        "favorites",
        "sprite_sheet_actions",
    ]
}

fn asset_from_record(record: &AssetRecord) -> ContentAsset2D {
    ContentAsset2D {
        guid: record.guid.clone(),
        path: record.relative_path.clone(),
        name: record.name.clone(),
        asset_type: normalize_type(&record.asset_type),
        labels: record.labels.clone(),
        preview: preview_for(record),
        dependencies: record.dependencies.clone(),
        valid: true,
        metadata: json!({
            "size_bytes": record.size_bytes,
            "modified_unix": record.modified_unix,
            "import_settings": record.import_settings,
            "compatibility": record.compatibility,
        }),
    }
}

fn normalize_type(asset_type: &str) -> String {
    match asset_type {
        "Sprite" | "sprite" | "png" | "jpg" | "jpeg" | "webp" | "image" => "Sprite2D",
        "SpriteSheet" => "SpriteSheet",
        "SpriteFrames2D" => "SpriteFrames2D",
        "AnimationBlueprint2D" | "FlipbookAnimation2D" | "Animation" => asset_type,
        "prefab" => "Prefab2D",
        "scene" => "Scene2D",
        "mfgraph" | "visual_graph" => "BlueprintGraph2D",
        "json" => "DataAsset2D",
        other => other,
    }
    .to_string()
}

fn preview_for(record: &AssetRecord) -> AssetPreview2D {
    let preview_type = match normalize_type(&record.asset_type).as_str() {
        "Sprite2D" => "sprite",
        "SpriteFrames2D" | "AnimationBlueprint2D" | "FlipbookAnimation2D" => "animation",
        "BlueprintGraph2D" => "graph",
        "Scene2D" => "scene",
        "Prefab2D" => "prefab",
        _ => "metadata",
    };
    AssetPreview2D {
        preview_type: preview_type.to_string(),
        thumbnail_path: None,
        summary: format!("{} | {} bytes", record.asset_type, record.size_bytes),
    }
}

fn asset_size(asset: &ContentAsset2D) -> u64 {
    asset
        .metadata
        .get("size_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn insert_folder_path(root: &mut ContentFolderNode2D, asset_path: &str) {
    let parts = asset_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() <= 1 {
        return;
    }
    let mut current = root;
    let mut path = String::new();
    for part in parts.iter().take(parts.len() - 1) {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(part);
        current = current
            .children
            .entry((*part).to_string())
            .or_insert_with(|| ContentFolderNode2D {
                path: path.clone(),
                name: (*part).to_string(),
                asset_count: 0,
                children: BTreeMap::new(),
            });
        current.asset_count += 1;
    }
}
