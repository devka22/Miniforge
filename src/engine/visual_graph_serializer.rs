use serde_json::{Map, Value, json};

use crate::engine::document_schema::{
    MigrationReport, SchemaError, SchemaErrorKind, read_schema_version, require_object,
    validate_header,
};
use crate::engine::version::ENGINE_VERSION;

pub const VISUAL_GRAPH_FORMAT: &str = "miniforge.visual-graph";
pub const VISUAL_GRAPH_SCHEMA_VERSION: u32 = 1;

pub struct VisualGraphSerializer;

impl VisualGraphSerializer {
    pub fn try_migrate(mut data: Value) -> Result<MigrationReport, SchemaError> {
        if !data.is_object() {
            return Err(SchemaError::new(
                "visual graph",
                SchemaErrorKind::RootMustBeObject,
                "document root must be a JSON object",
            ));
        }
        let from_version = read_schema_version(&data, "visual graph")?;
        if from_version > VISUAL_GRAPH_SCHEMA_VERSION {
            return Err(SchemaError::new(
                "visual graph",
                SchemaErrorKind::FutureSchemaVersion,
                format!(
                    "schema version {from_version} is newer than supported version {VISUAL_GRAPH_SCHEMA_VERSION}"
                ),
            ));
        }
        let mut changed = false;
        if from_version == 0 {
            let map = require_object(&mut data, "visual graph")?;
            if let Some(format) = map.get("format").and_then(Value::as_str)
                && format != VISUAL_GRAPH_FORMAT
            {
                return Err(SchemaError::new(
                    "visual graph",
                    SchemaErrorKind::FormatMismatch,
                    format!("expected format `{VISUAL_GRAPH_FORMAT}`, found `{format}`"),
                ));
            }
            apply_defaults(map);
            apply_header(map);
            changed = true;
        }
        Self::validate(&data)?;
        Ok(MigrationReport {
            data,
            from_version,
            to_version: VISUAL_GRAPH_SCHEMA_VERSION,
            changed,
            warnings: if changed {
                vec!["legacy visual graph migrated in memory".to_string()]
            } else {
                Vec::new()
            },
        })
    }

    pub fn stamp(mut data: Value) -> Result<Value, SchemaError> {
        let map = require_object(&mut data, "visual graph")?;
        apply_defaults(map);
        apply_header(map);
        Self::validate(&data)?;
        Ok(data)
    }

    pub fn validate(data: &Value) -> Result<(), SchemaError> {
        validate_header(
            data,
            "visual graph",
            VISUAL_GRAPH_FORMAT,
            VISUAL_GRAPH_SCHEMA_VERSION,
        )?;
        if data
            .get("name")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(SchemaError::new(
                "visual graph",
                SchemaErrorKind::InvalidField,
                "field `name` must be a non-empty string",
            ));
        }
        if !data.get("nodes").is_some_and(Value::is_array) {
            return Err(SchemaError::new(
                "visual graph",
                SchemaErrorKind::InvalidField,
                "field `nodes` must be an array",
            ));
        }
        Ok(())
    }
}

fn apply_header(map: &mut Map<String, Value>) {
    map.insert("format".to_string(), json!(VISUAL_GRAPH_FORMAT));
    map.insert(
        "schema_version".to_string(),
        json!(VISUAL_GRAPH_SCHEMA_VERSION),
    );
    map.insert("engine_version".to_string(), json!(ENGINE_VERSION));
    map.entry("version".to_string())
        .or_insert(json!(ENGINE_VERSION));
}

fn apply_defaults(map: &mut Map<String, Value>) {
    map.entry("kind".to_string())
        .or_insert(json!("MiniForgeVisualGraph"));
    map.entry("runtime".to_string())
        .or_insert(json!("rust_visual_graph"));
    map.entry("name".to_string())
        .or_insert(json!("VisualGraph"));
    map.entry("variables".to_string()).or_insert(json!({}));
    map.entry("nodes".to_string()).or_insert(json!([]));
    map.entry("editor".to_string()).or_insert(json!({}));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_graph_migrates_and_future_graph_is_rejected() {
        let legacy = json!({
            "version": "0.9.3",
            "kind": "MiniForgeVisualGraph",
            "name": "Test",
            "nodes": [],
        });
        let migrated = VisualGraphSerializer::try_migrate(legacy).expect("legacy graph");
        assert!(migrated.changed);
        assert_eq!(migrated.data["format"], VISUAL_GRAPH_FORMAT);
        let mut future = migrated.data;
        future["schema_version"] = json!(VISUAL_GRAPH_SCHEMA_VERSION + 1);
        assert!(
            VisualGraphSerializer::try_migrate(future)
                .expect_err("future graph")
                .is_future_version()
        );
    }
}
