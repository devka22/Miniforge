use std::fmt;
use std::io;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaErrorKind {
    RootMustBeObject,
    InvalidSchemaVersion,
    FutureSchemaVersion,
    FormatMismatch,
    MissingField,
    InvalidField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub document_type: &'static str,
    pub kind: SchemaErrorKind,
    pub message: String,
}

impl SchemaError {
    pub fn new(
        document_type: &'static str,
        kind: SchemaErrorKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            document_type,
            kind,
            message: message.into(),
        }
    }

    pub fn is_future_version(&self) -> bool {
        self.kind == SchemaErrorKind::FutureSchemaVersion
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid {} document: {}",
            self.document_type, self.message
        )
    }
}

impl std::error::Error for SchemaError {}

impl From<SchemaError> for io::Error {
    fn from(error: SchemaError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationReport {
    pub data: Value,
    pub from_version: u32,
    pub to_version: u32,
    pub changed: bool,
    pub warnings: Vec<String>,
}

pub fn read_schema_version(data: &Value, document_type: &'static str) -> Result<u32, SchemaError> {
    let Some(value) = data.get("schema_version") else {
        return Ok(0);
    };
    let version = value.as_u64().ok_or_else(|| {
        SchemaError::new(
            document_type,
            SchemaErrorKind::InvalidSchemaVersion,
            "schema_version must be a non-negative integer",
        )
    })?;
    u32::try_from(version).map_err(|_| {
        SchemaError::new(
            document_type,
            SchemaErrorKind::InvalidSchemaVersion,
            format!("schema_version {version} exceeds the supported integer range"),
        )
    })
}

pub fn require_object<'a>(
    data: &'a mut Value,
    document_type: &'static str,
) -> Result<&'a mut serde_json::Map<String, Value>, SchemaError> {
    data.as_object_mut().ok_or_else(|| {
        SchemaError::new(
            document_type,
            SchemaErrorKind::RootMustBeObject,
            "document root must be a JSON object",
        )
    })
}

pub fn validate_header(
    data: &Value,
    document_type: &'static str,
    expected_format: &'static str,
    supported_version: u32,
) -> Result<(), SchemaError> {
    let format = data.get("format").and_then(Value::as_str).ok_or_else(|| {
        SchemaError::new(
            document_type,
            SchemaErrorKind::MissingField,
            "missing string field `format`",
        )
    })?;
    if format != expected_format {
        return Err(SchemaError::new(
            document_type,
            SchemaErrorKind::FormatMismatch,
            format!("expected format `{expected_format}`, found `{format}`"),
        ));
    }
    let version = read_schema_version(data, document_type)?;
    if version > supported_version {
        return Err(SchemaError::new(
            document_type,
            SchemaErrorKind::FutureSchemaVersion,
            format!("schema version {version} is newer than supported version {supported_version}"),
        ));
    }
    if version != supported_version {
        return Err(SchemaError::new(
            document_type,
            SchemaErrorKind::InvalidSchemaVersion,
            format!("document remained at schema version {version} after migration"),
        ));
    }
    if data
        .get("engine_version")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(SchemaError::new(
            document_type,
            SchemaErrorKind::MissingField,
            "missing non-empty string field `engine_version`",
        ));
    }
    Ok(())
}
