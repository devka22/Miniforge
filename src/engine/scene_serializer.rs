use serde_json::{Map, Value, json};

use crate::engine::document_schema::{
    MigrationReport, SchemaError, SchemaErrorKind, read_schema_version, require_object,
    validate_header,
};
use crate::engine::version::ENGINE_VERSION;

pub const SCENE_FORMAT: &str = "miniforge.scene";
pub const SCENE_SCHEMA_VERSION: u32 = 1;

pub struct SceneSerializer;

impl SceneSerializer {
    /// Compatibility helper for older callers. File loaders should use
    /// `try_migrate` so future/corrupt schemas are reported instead of reduced
    /// to an empty scene.
    pub fn migrate(data: Value) -> Value {
        Self::try_migrate(data)
            .map(|report| report.data)
            .unwrap_or_else(|_| json!({}))
    }

    pub fn try_migrate(mut data: Value) -> Result<MigrationReport, SchemaError> {
        if !data.is_object() {
            return Err(SchemaError::new(
                "scene",
                SchemaErrorKind::RootMustBeObject,
                "document root must be a JSON object",
            ));
        }
        let from_version = read_schema_version(&data, "scene")?;
        if from_version > SCENE_SCHEMA_VERSION {
            return Err(SchemaError::new(
                "scene",
                SchemaErrorKind::FutureSchemaVersion,
                format!(
                    "schema version {from_version} is newer than supported version {SCENE_SCHEMA_VERSION}"
                ),
            ));
        }

        let mut changed = false;
        let mut warnings = Vec::new();
        if from_version == 0 {
            let map = require_object(&mut data, "scene")?;
            if let Some(format) = map.get("format").and_then(Value::as_str)
                && format != SCENE_FORMAT
            {
                return Err(SchemaError::new(
                    "scene",
                    SchemaErrorKind::FormatMismatch,
                    format!("expected format `{SCENE_FORMAT}`, found `{format}`"),
                ));
            }
            if let Some(objects) = map.remove("objects") {
                map.entry("entities".to_string()).or_insert(objects);
                warnings.push("legacy `objects` field migrated to `entities`".to_string());
            }
            apply_scene_defaults(map);
            apply_scene_header(map);
            changed = true;
        }

        Self::validate(&data)?;
        Ok(MigrationReport {
            data,
            from_version,
            to_version: SCENE_SCHEMA_VERSION,
            changed,
            warnings,
        })
    }

    pub fn validate(data: &Value) -> Result<(), SchemaError> {
        validate_header(data, "scene", SCENE_FORMAT, SCENE_SCHEMA_VERSION)?;
        require_non_empty_string(data, "scene_name")?;
        require_array(data, "entities")?;
        require_array(data, "ui_canvases")?;
        if !data.get("camera").is_some_and(Value::is_object) {
            return Err(SchemaError::new(
                "scene",
                SchemaErrorKind::InvalidField,
                "field `camera` must be an object",
            ));
        }
        Ok(())
    }

    pub fn stamp(mut data: Value) -> Result<Value, SchemaError> {
        let map = require_object(&mut data, "scene")?;
        apply_scene_defaults(map);
        apply_scene_header(map);
        Self::validate(&data)?;
        Ok(data)
    }
}

fn apply_scene_header(map: &mut Map<String, Value>) {
    map.insert("format".to_string(), json!(SCENE_FORMAT));
    map.insert("schema_version".to_string(), json!(SCENE_SCHEMA_VERSION));
    map.insert("engine_version".to_string(), json!(ENGINE_VERSION));
    map.entry("version".to_string())
        .or_insert(json!(ENGINE_VERSION));
}

fn apply_scene_defaults(map: &mut Map<String, Value>) {
    map.entry("scene_name".to_string()).or_insert(json!("main"));
    map.entry("mode".to_string()).or_insert(json!("EDITOR"));
    map.entry("active_tool".to_string())
        .or_insert(json!("Select"));
    map.entry("tile_brush".to_string()).or_insert(json!(0));
    map.entry("brush_size".to_string()).or_insert(json!(1));
    map.entry("camera".to_string())
        .or_insert(json!({"x": 0, "y": 0, "zoom": 1.0}));
    map.entry("control_groups".to_string()).or_insert(json!({}));
    map.entry("grid".to_string()).or_insert(Value::Null);
    map.entry("tiles".to_string()).or_insert(json!([]));
    let tiles = map.get("tiles").cloned().unwrap_or_else(|| json!([]));
    map.entry("tilemap_layers".to_string()).or_insert(tiles);
    map.entry("settings".to_string()).or_insert(json!({}));
    map.entry("entities".to_string()).or_insert(json!([]));
    map.entry("editor_view_settings".to_string())
        .or_insert(json!({}));
    map.entry("ui_canvases".to_string()).or_insert(json!([]));
}

fn require_non_empty_string(data: &Value, field: &str) -> Result<(), SchemaError> {
    if data
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(());
    }
    Err(SchemaError::new(
        "scene",
        SchemaErrorKind::InvalidField,
        format!("field `{field}` must be a non-empty string"),
    ))
}

fn require_array(data: &Value, field: &str) -> Result<(), SchemaError> {
    if data.get(field).is_some_and(Value::is_array) {
        return Ok(());
    }
    Err(SchemaError::new(
        "scene",
        SchemaErrorKind::InvalidField,
        format!("field `{field}` must be an array"),
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{SCENE_SCHEMA_VERSION, SceneSerializer};
    use crate::engine::document_schema::SchemaErrorKind;

    fn fixture(source: &str) -> Value {
        serde_json::from_str(source).expect("valid fixture JSON")
    }

    #[test]
    fn legacy_scene_migration_matches_golden_document_and_is_idempotent() {
        let legacy = fixture(include_str!("../../tests/fixtures/formats/scene_v0.scene"));
        let expected = fixture(include_str!("../../tests/fixtures/formats/scene_v1.scene"));

        let migrated = SceneSerializer::try_migrate(legacy).expect("legacy migration");
        assert_eq!(migrated.from_version, 0);
        assert_eq!(migrated.to_version, SCENE_SCHEMA_VERSION);
        assert_eq!(migrated.data, expected);
        let second = SceneSerializer::try_migrate(migrated.data.clone()).expect("idempotent");
        assert!(!second.changed);
        assert_eq!(second.data, migrated.data);
    }

    #[test]
    fn future_and_damaged_scenes_are_rejected() {
        let future = fixture(include_str!(
            "../../tests/fixtures/formats/scene_future.scene"
        ));
        let error = SceneSerializer::try_migrate(future).expect_err("future schema");
        assert_eq!(error.kind, SchemaErrorKind::FutureSchemaVersion);

        let broken = fixture(include_str!(
            "../../tests/fixtures/formats/scene_broken.scene"
        ));
        let error = SceneSerializer::try_migrate(broken).expect_err("broken schema");
        assert_eq!(error.kind, SchemaErrorKind::InvalidField);
    }
}
