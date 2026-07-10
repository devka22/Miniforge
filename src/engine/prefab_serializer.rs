use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use crate::engine::document_schema::{
    MigrationReport, SchemaError, SchemaErrorKind, read_schema_version, require_object,
    validate_header,
};
use crate::engine::version::ENGINE_VERSION;

pub const PREFAB_FORMAT: &str = "miniforge.prefab";
pub const PREFAB_SCHEMA_VERSION: u32 = 2;

pub const DEFAULT_PREFAB_SETTINGS: &[&str] = &[
    "settings/input_map.json",
    "settings/tags.json",
    "settings/layers.json",
    "settings/runtime_config.json",
];

pub struct PrefabSerializer;

impl PrefabSerializer {
    pub fn try_migrate(mut data: Value) -> Result<MigrationReport, SchemaError> {
        if !data.is_object() {
            return Err(SchemaError::new(
                "prefab",
                SchemaErrorKind::RootMustBeObject,
                "document root must be a JSON object",
            ));
        }
        let from_version = read_schema_version(&data, "prefab")?;
        if from_version > PREFAB_SCHEMA_VERSION {
            return Err(SchemaError::new(
                "prefab",
                SchemaErrorKind::FutureSchemaVersion,
                format!(
                    "schema version {from_version} is newer than supported version {PREFAB_SCHEMA_VERSION}"
                ),
            ));
        }
        let mut changed = false;
        if from_version < 2 {
            let map = require_object(&mut data, "prefab")?;
            if let Some(format) = map.get("format").and_then(Value::as_str)
                && format != PREFAB_FORMAT
            {
                return Err(SchemaError::new(
                    "prefab",
                    SchemaErrorKind::FormatMismatch,
                    format!("expected format `{PREFAB_FORMAT}`, found `{format}`"),
                ));
            }
            apply_prefab_header(map);
            normalize_prefab_manifest(map);
            changed = true;
        }
        Self::validate(&data)?;
        Ok(MigrationReport {
            data,
            from_version,
            to_version: PREFAB_SCHEMA_VERSION,
            changed,
            warnings: Vec::new(),
        })
    }

    pub fn validate(data: &Value) -> Result<(), SchemaError> {
        validate_header(data, "prefab", PREFAB_FORMAT, PREFAB_SCHEMA_VERSION)?;
        let entity = data
            .get("entity")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                SchemaError::new(
                    "prefab",
                    SchemaErrorKind::MissingField,
                    "missing object field `entity`",
                )
            })?;
        if entity
            .get("name")
            .and_then(Value::as_str)
            .is_none_or(|name| name.trim().is_empty())
        {
            return Err(SchemaError::new(
                "prefab",
                SchemaErrorKind::InvalidField,
                "entity.name must be a non-empty string",
            ));
        }
        if !entity.get("components").is_some_and(Value::is_array) {
            return Err(SchemaError::new(
                "prefab",
                SchemaErrorKind::InvalidField,
                "entity.components must be an array",
            ));
        }
        validate_manifest_object(data, "scripts")?;
        validate_manifest_object(data, "settings")?;
        validate_string_array(data.get("dependencies"), "dependencies")?;
        Ok(())
    }

    pub fn stamp(mut data: Value) -> Result<Value, SchemaError> {
        let map = require_object(&mut data, "prefab")?;
        apply_prefab_header(map);
        Self::validate(&data)?;
        Ok(data)
    }
}

fn apply_prefab_header(map: &mut Map<String, Value>) {
    map.insert("format".to_string(), json!(PREFAB_FORMAT));
    map.insert("schema_version".to_string(), json!(PREFAB_SCHEMA_VERSION));
    map.insert("engine_version".to_string(), json!(ENGINE_VERSION));
    map.entry("version".to_string())
        .or_insert(json!(ENGINE_VERSION));
    normalize_prefab_manifest(map);
}

fn normalize_prefab_manifest(map: &mut Map<String, Value>) {
    let entity = map.get("entity").cloned().unwrap_or_else(|| json!({}));
    let required_scripts = collect_entity_scripts(&entity);
    let scripts = map.entry("scripts").or_insert_with(|| {
        json!({
            "required": required_scripts,
            "embedded": [],
            "policy": "validate_on_instantiate",
        })
    });
    if let Some(scripts) = scripts.as_object_mut() {
        merge_string_array(scripts, "required", &required_scripts);
        scripts
            .entry("embedded".to_string())
            .or_insert_with(|| json!([]));
        scripts
            .entry("policy".to_string())
            .or_insert_with(|| json!("validate_on_instantiate"));
    }

    let settings = map.entry("settings").or_insert_with(|| {
        json!({
            "required": DEFAULT_PREFAB_SETTINGS,
            "defaults": {},
            "policy": "merge_missing",
        })
    });
    if let Some(settings) = settings.as_object_mut() {
        merge_string_array(settings, "required", DEFAULT_PREFAB_SETTINGS);
        settings
            .entry("defaults".to_string())
            .or_insert_with(|| json!({}));
        settings
            .entry("policy".to_string())
            .or_insert_with(|| json!("merge_missing"));
    }

    map.entry("dependencies".to_string())
        .or_insert_with(|| json!([]));
    let component_count = entity
        .get("components")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let script_count = collect_entity_scripts(&entity).len();
    let metadata = map.entry("metadata").or_insert_with(|| json!({}));
    if let Some(metadata) = metadata.as_object_mut() {
        metadata
            .entry("component_count".to_string())
            .or_insert_with(|| json!(component_count));
        metadata
            .entry("script_count".to_string())
            .or_insert_with(|| json!(script_count));
        metadata
            .entry("source".to_string())
            .or_insert_with(|| json!("prefab_pipeline"));
    }
}

pub fn collect_entity_scripts(entity: &Value) -> Vec<String> {
    let mut scripts = BTreeSet::new();
    if let Some(path) = entity.get("script").and_then(Value::as_str) {
        insert_script_path(&mut scripts, path);
    }
    if let Some(items) = entity.get("scripts").and_then(Value::as_array) {
        for item in items {
            if let Some(path) = item.as_str() {
                insert_script_path(&mut scripts, path);
            }
            if let Some(path) = item
                .get("path")
                .or_else(|| item.get("script"))
                .or_else(|| item.get("source"))
                .and_then(Value::as_str)
            {
                insert_script_path(&mut scripts, path);
            }
        }
    }
    if let Some(components) = entity.get("components").and_then(Value::as_array) {
        for component in components {
            let component_type = component
                .get("component_type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !component_type.contains("script")
                && !component_type.contains("luau")
                && !component_type.contains("rhai")
            {
                continue;
            }
            for key in ["path", "script", "script_path", "source", "file"] {
                if let Some(path) = component.get(key).and_then(Value::as_str) {
                    insert_script_path(&mut scripts, path);
                }
            }
        }
    }
    scripts.into_iter().collect()
}

fn insert_script_path(scripts: &mut BTreeSet<String>, path: &str) {
    let path = path.trim();
    if path.is_empty() {
        return;
    }
    if matches!(
        std::path::Path::new(path)
            .extension()
            .and_then(|value| value.to_str()),
        Some("luau" | "rhai" | "mfgraph")
    ) {
        scripts.insert(path.replace('\\', "/"));
    }
}

fn merge_string_array(map: &mut Map<String, Value>, key: &str, required: &[impl AsRef<str>]) {
    let mut values = map
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    for value in required {
        let value = value.as_ref().trim();
        if !value.is_empty() {
            values.insert(value.replace('\\', "/"));
        }
    }
    map.insert(
        key.to_string(),
        json!(values.into_iter().collect::<Vec<_>>()),
    );
}

fn validate_manifest_object(data: &Value, key: &str) -> Result<(), SchemaError> {
    let Some(manifest) = data.get(key).and_then(Value::as_object) else {
        return Err(SchemaError::new(
            "prefab",
            SchemaErrorKind::InvalidField,
            format!("{key} must be an object"),
        ));
    };
    validate_string_array(manifest.get("required"), &format!("{key}.required"))
}

fn validate_string_array(value: Option<&Value>, path: &str) -> Result<(), SchemaError> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Err(SchemaError::new(
            "prefab",
            SchemaErrorKind::InvalidField,
            format!("{path} must be an array"),
        ));
    };
    if values.iter().any(|item| item.as_str().is_none()) {
        return Err(SchemaError::new(
            "prefab",
            SchemaErrorKind::InvalidField,
            format!("{path} must only contain strings"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{PREFAB_SCHEMA_VERSION, PrefabSerializer};
    use crate::engine::document_schema::SchemaErrorKind;

    fn fixture(source: &str) -> Value {
        serde_json::from_str(source).expect("valid fixture JSON")
    }

    #[test]
    fn legacy_prefab_migration_matches_golden_document_and_is_idempotent() {
        let legacy = fixture(include_str!(
            "../../tests/fixtures/formats/prefab_v0.prefab"
        ));
        let expected = fixture(include_str!(
            "../../tests/fixtures/formats/prefab_v1.prefab"
        ));

        let migrated = PrefabSerializer::try_migrate(legacy).expect("legacy migration");
        assert_eq!(migrated.from_version, 0);
        assert_eq!(migrated.to_version, PREFAB_SCHEMA_VERSION);
        assert_eq!(migrated.data, expected);
        let second = PrefabSerializer::try_migrate(migrated.data.clone()).expect("idempotent");
        assert!(!second.changed);
        assert_eq!(second.data, migrated.data);
    }

    #[test]
    fn future_and_damaged_prefabs_are_rejected() {
        let future = fixture(include_str!(
            "../../tests/fixtures/formats/prefab_future.prefab"
        ));
        let error = PrefabSerializer::try_migrate(future).expect_err("future schema");
        assert_eq!(error.kind, SchemaErrorKind::FutureSchemaVersion);

        let broken = fixture(include_str!(
            "../../tests/fixtures/formats/prefab_broken.prefab"
        ));
        let error = PrefabSerializer::try_migrate(broken).expect_err("broken schema");
        assert_eq!(error.kind, SchemaErrorKind::InvalidField);
    }

    #[test]
    fn prefab_schema_v2_tracks_required_scripts_and_settings() {
        let prefab = serde_json::json!({
            "version": "0.9.3",
            "prefab_name": "Enemy",
            "entity": {
                "name": "Enemy",
                "script": "EnemyBrain.luau",
                "components": []
            }
        });
        let migrated = PrefabSerializer::try_migrate(prefab).expect("migrates prefab");
        assert_eq!(migrated.to_version, PREFAB_SCHEMA_VERSION);
        assert_eq!(migrated.data["schema_version"], 2);
        assert!(
            migrated.data["scripts"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item == "EnemyBrain.luau")
        );
        assert!(
            migrated.data["settings"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item == "settings/runtime_config.json")
        );
    }
}
