use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub type MFResult<T> = Result<T, MiniForgeError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiniForgeError {
    Io(String),
    Json(String),
    AssetMissing(String),
    ScriptError(String),
    GraphError(String),
    SceneError(String),
    RenderError(String),
    PluginError(String),
    ExportError(String),
    ValidationError(String),
    Panic(String),
}

impl MiniForgeError {
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::AssetMissing(_) => "asset_missing",
            Self::ScriptError(_) => "script",
            Self::GraphError(_) => "graph",
            Self::SceneError(_) => "scene",
            Self::RenderError(_) => "render",
            Self::PluginError(_) => "plugin",
            Self::ExportError(_) => "export",
            Self::ValidationError(_) => "validation",
            Self::Panic(_) => "panic",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Io(message)
            | Self::Json(message)
            | Self::AssetMissing(message)
            | Self::ScriptError(message)
            | Self::GraphError(message)
            | Self::SceneError(message)
            | Self::RenderError(message)
            | Self::PluginError(message)
            | Self::ExportError(message)
            | Self::ValidationError(message)
            | Self::Panic(message) => message,
        }
    }
}

impl fmt::Display for MiniForgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.category(), self.message())
    }
}

impl std::error::Error for MiniForgeError {}

impl From<std::io::Error> for MiniForgeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for MiniForgeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ErrorHandler {
    pub errors: Vec<String>,
    pub last_call_failed: bool,
}

impl ErrorHandler {
    pub fn safe_call<F>(&mut self, name: &str, mut callback: F)
    where
        F: FnMut() -> Result<(), String>,
    {
        match callback() {
            Ok(()) => self.last_call_failed = false,
            Err(error) => {
                self.last_call_failed = true;
                self.errors.push(format!("{name}: {error}"));
            }
        }
    }

    pub fn safe_result<T, F>(&mut self, name: &str, fallback: T, callback: F) -> T
    where
        F: FnOnce() -> MFResult<T>,
    {
        match catch_unwind(AssertUnwindSafe(callback)) {
            Ok(Ok(value)) => {
                self.last_call_failed = false;
                value
            }
            Ok(Err(error)) => {
                self.last_call_failed = true;
                self.errors.push(format!("{name}: {error}"));
                fallback
            }
            Err(payload) => {
                self.last_call_failed = true;
                let message = if let Some(text) = payload.downcast_ref::<&str>() {
                    (*text).to_string()
                } else if let Some(text) = payload.downcast_ref::<String>() {
                    text.clone()
                } else {
                    "panic desconocido".to_string()
                };
                self.errors
                    .push(format!("{name}: {}", MiniForgeError::Panic(message)));
                fallback
            }
        }
    }

    pub fn report(&mut self, name: &str, error: impl fmt::Display) {
        self.last_call_failed = true;
        self.errors.push(format!("{name}: {error}"));
    }
}
