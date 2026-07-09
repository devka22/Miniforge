use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScriptLanguage2D {
    Luau,
    Blueprint,
    Python,
    CSharp,
}

impl ScriptLanguage2D {
    pub fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".mftool.json"))
        {
            return Some(Self::Python);
        }
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "luau" | "lua" => Some(Self::Luau),
            "mfgraph" => Some(Self::Blueprint),
            "py" => Some(Self::Python),
            "cs" | "csproj" => Some(Self::CSharp),
            _ => None,
        }
    }

    pub fn runtime_safe(self) -> bool {
        matches!(self, Self::Luau | Self::Blueprint)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScriptBackendState2D {
    BuiltIn,
    Available,
    Planned,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScriptCapability2D {
    ReadScene,
    WriteScene,
    SpawnEntities,
    PhysicsQueries,
    Input,
    Audio,
    UserInterface,
    EditorTools,
    FileRead,
    FileWrite,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptFunction2D {
    pub name: String,
    #[serde(default)]
    pub parameters: Vec<String>,
    pub returns: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptModuleManifest2D {
    pub id: String,
    pub language: ScriptLanguage2D,
    pub source: String,
    pub api_version: u32,
    #[serde(default)]
    pub functions: Vec<ScriptFunction2D>,
    #[serde(default)]
    pub capabilities: BTreeSet<ScriptCapability2D>,
    #[serde(default)]
    pub editor_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptCall2D {
    pub module: String,
    pub function: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptCallResult2D {
    pub ok: bool,
    pub value: Value,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptBackend2D {
    pub language: ScriptLanguage2D,
    pub state: ScriptBackendState2D,
    pub adapter: String,
    pub sandboxed: bool,
    pub hot_reload: bool,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptHost2D {
    pub api_version: u32,
    #[serde(default)]
    pub backends: BTreeMap<ScriptLanguage2D, ScriptBackend2D>,
    #[serde(default)]
    pub modules: BTreeMap<String, ScriptModuleManifest2D>,
}

impl ScriptHost2D {
    pub fn foundation() -> Self {
        let backends = [
            backend(
                ScriptLanguage2D::Luau,
                ScriptBackendState2D::BuiltIn,
                "mlua-luau",
                true,
                true,
                "Typed gameplay scripting with the MiniForge command API",
            ),
            backend(
                ScriptLanguage2D::Blueprint,
                ScriptBackendState2D::BuiltIn,
                "mfgraph",
                true,
                true,
                "Compiled visual gameplay and editor-authored behavior",
            ),
            backend(
                ScriptLanguage2D::Python,
                ScriptBackendState2D::Available,
                "miniforge-editor-tool-v1",
                true,
                false,
                "Trusted editor automation, import pipelines and production tools",
            ),
            backend(
                ScriptLanguage2D::CSharp,
                ScriptBackendState2D::Available,
                "dotnet-plugin-manifest",
                true,
                false,
                "Editor/plugin tooling through generated C# plugin projects",
            ),
        ]
        .into_iter()
        .map(|backend| (backend.language, backend))
        .collect();
        Self {
            api_version: 1,
            backends,
            modules: BTreeMap::new(),
        }
    }

    pub fn register_module(&mut self, module: ScriptModuleManifest2D) -> Result<(), Vec<String>> {
        let diagnostics = self.validate_module(&module);
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        self.modules.insert(module.id.clone(), module);
        Ok(())
    }

    pub fn validate_module(&self, module: &ScriptModuleManifest2D) -> Vec<String> {
        let mut diagnostics = Vec::new();
        if module.id.trim().is_empty() {
            diagnostics.push("script module id cannot be empty".to_string());
        }
        if module.api_version != self.api_version {
            diagnostics.push(format!(
                "module API v{} is incompatible with host API v{}",
                module.api_version, self.api_version
            ));
        }
        let Some(backend) = self.backends.get(&module.language) else {
            diagnostics.push("script language has no registered backend".to_string());
            return diagnostics;
        };
        if matches!(
            backend.state,
            ScriptBackendState2D::Planned | ScriptBackendState2D::Disabled
        ) {
            diagnostics.push(format!(
                "{:?} backend is {:?}; the manifest is valid as a roadmap asset but cannot run",
                module.language, backend.state
            ));
        }
        let mut functions = BTreeSet::new();
        for function in &module.functions {
            if function.name.trim().is_empty() || !functions.insert(&function.name) {
                diagnostics.push(format!(
                    "function name '{}' is empty or duplicated",
                    function.name
                ));
            }
        }
        diagnostics
    }

    pub fn validate_call(&self, call: &ScriptCall2D) -> Result<&ScriptFunction2D, String> {
        let module = self
            .modules
            .get(&call.module)
            .ok_or_else(|| format!("unknown script module '{}'", call.module))?;
        let function = module
            .functions
            .iter()
            .find(|function| function.name == call.function)
            .ok_or_else(|| {
                format!(
                    "module '{}' does not expose '{}'",
                    call.module, call.function
                )
            })?;
        for parameter in &function.parameters {
            if !call.arguments.contains_key(parameter) {
                return Err(format!("missing argument '{parameter}'"));
            }
        }
        Ok(function)
    }

    pub fn language_matrix(&self) -> Vec<ScriptBackend2D> {
        self.backends.values().cloned().collect()
    }
}

impl Default for ScriptHost2D {
    fn default() -> Self {
        Self::foundation()
    }
}

fn backend(
    language: ScriptLanguage2D,
    state: ScriptBackendState2D,
    adapter: &str,
    sandboxed: bool,
    hot_reload: bool,
    purpose: &str,
) -> ScriptBackend2D {
    ScriptBackend2D {
        language,
        state,
        adapter: adapter.to_string(),
        sandboxed,
        hot_reload,
        purpose: purpose.to_string(),
    }
}
