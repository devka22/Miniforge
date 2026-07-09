use serde_json::{Map, Value, json};

use crate::engine::document_schema::{
    MigrationReport, SchemaError, SchemaErrorKind, read_schema_version, require_object,
    validate_header,
};
use crate::engine::version::ENGINE_VERSION;

pub const PREFAB_FORMAT: &str = "miniforge.prefab";
pub const PREFAB_SCHEMA_VERSION: u32 = 1;

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
        if from_version == 0 {
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
}
