use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::prefab_serializer::PrefabSerializer;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PrefabInstanceReport {
    pub entity_id: u64,
    pub prefab_source: Option<String>,
    pub prefab_guid: Option<String>,
    pub component_count: usize,
    pub script_count: usize,
    pub override_count: usize,
    pub missing_source: bool,
    pub can_apply: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AdvancedPrefabSystem {
    pub variants_created: usize,
    pub prefabs_created: usize,
    pub instances_tracked: usize,
    pub last_report: Option<PrefabInstanceReport>,
}

impl AdvancedPrefabSystem {
    pub fn create_prefab_from_entity(
        &mut self,
        project_path: impl AsRef<Path>,
        entity: &mut GameObject,
        include_children: bool,
        dependencies: Vec<String>,
    ) -> io::Result<PathBuf> {
        let paths = AssetTools::get_project_paths(project_path);
        fs::create_dir_all(&paths.prefabs)?;
        entity.sync_to_components();
        let filename = format!("{}.prefab", AssetTools::safe_name(&entity.name, "Prefab"));
        let path = AssetTools::unique_path(&paths.prefabs, &filename);
        let guid = crate::engine::asset_database::stable_guid(&path.to_string_lossy());
        let data = PrefabSerializer::stamp(json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "kind": "MiniForgeAdvancedPrefab",
            "prefab_name": entity.name,
            "guid": guid,
            "include_children": include_children,
            "dependencies": dependencies,
            "variant": false,
            "entity": entity.serialize(),
            "metadata": {
                "component_count": entity.components.len(),
                "script_count": entity.scripts.len(),
                "source": "rust_editor",
            }
        }))
        .map_err(io::Error::from)?;
        AssetTools::write_json(&path, &data)?;
        entity.prefab_source = Some(path.to_string_lossy().to_string());
        entity.prefab_guid = Some(guid);
        entity.is_prefab_instance = true;
        self.prefabs_created += 1;
        self.instances_tracked += 1;
        self.last_report = Some(self.analyze_instance(entity, Some(&data)));
        Ok(path)
    }

    pub fn create_variant_from_entity(
        &mut self,
        project_path: impl AsRef<Path>,
        entity: &mut GameObject,
    ) -> io::Result<PathBuf> {
        let paths = AssetTools::get_project_paths(project_path);
        fs::create_dir_all(&paths.prefabs)?;
        entity.sync_to_components();
        let filename = format!(
            "{}_variant.prefab",
            AssetTools::safe_name(&entity.name, "Variant")
        );
        let path = AssetTools::unique_path(paths.prefabs, &filename);
        let guid = crate::engine::asset_database::stable_guid(&path.to_string_lossy());
        let data = PrefabSerializer::stamp(json!({
            "version": crate::engine::version::ENGINE_VERSION,
            "kind": "MiniForgeAdvancedPrefab",
            "prefab_name": entity.name,
            "guid": guid,
            "parent_guid": entity.prefab_guid,
            "parent_source": entity.prefab_source,
            "variant": true,
            "overrides": self.calculate_override_count(entity, None),
            "entity": entity.serialize(),
        }))
        .map_err(io::Error::from)?;
        AssetTools::write_json(&path, &data)?;
        self.variants_created += 1;
        Ok(path)
    }

    pub fn analyze_instance(
        &mut self,
        entity: &GameObject,
        source_data: Option<&Value>,
    ) -> PrefabInstanceReport {
        let missing_source = entity
            .prefab_source
            .as_deref()
            .map(|path| !Path::new(path).exists())
            .unwrap_or(!entity.is_prefab_instance);
        let override_count = self.calculate_override_count(entity, source_data);
        let report = PrefabInstanceReport {
            entity_id: entity.id,
            prefab_source: entity.prefab_source.clone(),
            prefab_guid: entity.prefab_guid.clone(),
            component_count: entity.components.len(),
            script_count: entity.scripts.len(),
            override_count,
            missing_source,
            can_apply: entity.is_prefab_instance && !missing_source,
        };
        self.last_report = Some(report.clone());
        report
    }

    pub fn calculate_override_count(
        &self,
        entity: &GameObject,
        source_data: Option<&Value>,
    ) -> usize {
        let Some(source) = source_data
            .and_then(|data| data.get("entity"))
            .and_then(Value::as_object)
        else {
            return usize::from(entity.is_prefab_instance)
                + entity.components.len()
                + entity.scripts.len();
        };

        let mut count = 0;
        let fields = [
            ("name", json!(entity.name)),
            ("tag", json!(entity.tag)),
            ("layer", json!(entity.layer)),
            ("x", json!(entity.x)),
            ("y", json!(entity.y)),
            ("rotation", json!(entity.rotation)),
            ("scale_x", json!(entity.scale_x)),
            ("scale_y", json!(entity.scale_y)),
        ];
        for (key, value) in fields {
            if source.get(key) != Some(&value) {
                count += 1;
            }
        }
        let source_components = source
            .get("components")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default();
        count + entity.components.len().abs_diff(source_components)
    }

    pub fn status_line(&self) -> String {
        format!(
            "{} prefabs | {} variants | {} tracked instances",
            self.prefabs_created, self.variants_created, self.instances_tracked
        )
    }
}
