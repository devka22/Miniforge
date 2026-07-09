//! Versioned C ABI for native MiniForge middleware.
//!
//! C++ libraries must expose `miniforge_native_entry_v1` from an `extern "C"`
//! wrapper. The dynamic library stays loaded for the lifetime of its descriptor,
//! so all copied function pointers remain valid.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::fs;
use std::path::{Path, PathBuf};

use libloading::Library;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MINIFORGE_NATIVE_ABI_VERSION: u32 = 1;
pub const MINIFORGE_NATIVE_ENTRY_V1: &[u8] = b"miniforge_native_entry_v1\0";

pub type NativeInitializeFn = unsafe extern "C" fn(*const MiniForgeNativeHostV1) -> i32;
pub type NativeShutdownFn = unsafe extern "C" fn();
pub type NativeInvokeJsonFn = unsafe extern "C" fn(
    operation: *const c_char,
    request_json: *const c_char,
    response_json: *mut *mut c_char,
) -> i32;
pub type NativeFreeStringFn = unsafe extern "C" fn(*mut c_char);
pub type NativeEntryV1Fn = unsafe extern "C" fn() -> *const MiniForgeNativePluginV1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MiniForgeNativeHostV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub user_data: *mut c_void,
    pub log: Option<unsafe extern "C" fn(level: u32, message: *const c_char)>,
}

// The callbacks are immutable and `user_data` is null in the process-wide host table.
unsafe impl Sync for MiniForgeNativeHostV1 {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MiniForgeNativePluginV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub name: *const c_char,
    pub version: *const c_char,
    pub category: *const c_char,
    pub initialize: Option<NativeInitializeFn>,
    pub shutdown: Option<NativeShutdownFn>,
    pub invoke_json: Option<NativeInvokeJsonFn>,
    pub free_string: Option<NativeFreeStringFn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct NativeLibraryManifest {
    pub id: String,
    pub library: PathBuf,
    pub enabled: bool,
    pub required: bool,
    pub abi_version: u32,
    pub category: NativeLibraryCategory,
    pub platforms: Vec<String>,
    pub services: Vec<String>,
}

impl Default for NativeLibraryManifest {
    fn default() -> Self {
        Self {
            id: String::new(),
            library: PathBuf::new(),
            enabled: true,
            required: false,
            abi_version: MINIFORGE_NATIVE_ABI_VERSION,
            category: NativeLibraryCategory::Middleware,
            platforms: Vec::new(),
            services: Vec::new(),
        }
    }
}

impl NativeLibraryManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("native library id cannot be empty".to_string());
        }
        if self.library.as_os_str().is_empty() {
            return Err(format!("{}: library path cannot be empty", self.id));
        }
        if self.abi_version != MINIFORGE_NATIVE_ABI_VERSION {
            return Err(format!(
                "{}: manifest ABI v{} is incompatible with host ABI v{}",
                self.id, self.abi_version, MINIFORGE_NATIVE_ABI_VERSION
            ));
        }
        Ok(())
    }

    pub fn supports_current_platform(&self) -> bool {
        self.platforms.is_empty()
            || self.platforms.iter().any(|platform| {
                platform.eq_ignore_ascii_case(std::env::consts::OS)
                    || platform.eq_ignore_ascii_case(std::env::consts::ARCH)
            })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NativeLibraryCategory {
    Codec,
    PlatformSdk,
    Audio,
    Navigation,
    Middleware,
    Steam,
    Console,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeCallResult {
    pub status: i32,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeLibraryInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub path: PathBuf,
    pub services: Vec<String>,
}

struct LoadedNativeLibrary {
    _library: Library,
    info: NativeLibraryInfo,
    shutdown: Option<NativeShutdownFn>,
    invoke_json: NativeInvokeJsonFn,
    free_string: NativeFreeStringFn,
}

impl std::fmt::Debug for LoadedNativeLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedNativeLibrary")
            .field("info", &self.info)
            .finish()
    }
}

impl Drop for LoadedNativeLibrary {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown {
            // SAFETY: the callback came from the still-loaded descriptor and takes no arguments.
            unsafe { shutdown() };
        }
    }
}

#[derive(Debug, Default)]
pub struct NativeLibraryManager {
    project_path: PathBuf,
    loaded: BTreeMap<String, LoadedNativeLibrary>,
    pub diagnostics: Vec<String>,
}

impl NativeLibraryManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            project_path: project_path.as_ref().to_path_buf(),
            ..Self::default()
        }
    }

    pub fn discover_manifests(&self) -> Result<Vec<(PathBuf, NativeLibraryManifest)>, String> {
        let root = self.project_path.join("native");
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut manifests = Vec::new();
        for entry in fs::read_dir(&root).map_err(|error| format!("{}: {error}", root.display()))? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let manifest_path = if path.is_dir() {
                path.join("native.json")
            } else {
                path.clone()
            };
            if manifest_path.file_name().and_then(|name| name.to_str()) != Some("native.json") {
                continue;
            }
            let source = fs::read_to_string(&manifest_path)
                .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
            let manifest = serde_json::from_str::<NativeLibraryManifest>(&source)
                .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
            manifest.validate()?;
            manifests.push((manifest_path, manifest));
        }
        manifests.sort_by(|left, right| left.1.id.cmp(&right.1.id));
        Ok(manifests)
    }

    pub fn load_enabled(&mut self) -> Result<usize, Vec<String>> {
        self.diagnostics.clear();
        let manifests = self.discover_manifests().map_err(|error| vec![error])?;
        for (manifest_path, manifest) in manifests {
            if !manifest.enabled || !manifest.supports_current_platform() {
                continue;
            }
            let base = manifest_path.parent().unwrap_or(&self.project_path);
            let library_path = if manifest.library.is_absolute() {
                manifest.library.clone()
            } else {
                base.join(&manifest.library)
            };
            if let Err(error) = self.load(&manifest, &library_path) {
                let diagnostic = format!("{}: {error}", manifest.id);
                self.diagnostics.push(diagnostic.clone());
                if manifest.required {
                    return Err(self.diagnostics.clone());
                }
            }
        }
        Ok(self.loaded.len())
    }

    pub fn load(
        &mut self,
        manifest: &NativeLibraryManifest,
        path: impl AsRef<Path>,
    ) -> Result<NativeLibraryInfo, String> {
        manifest.validate()?;
        if self.loaded.contains_key(&manifest.id) {
            return Err(format!("{} is already loaded", manifest.id));
        }
        let path = path.as_ref().to_path_buf();
        // SAFETY: opening arbitrary native code is explicitly requested by a native manifest.
        let library = unsafe { Library::new(&path) }
            .map_err(|error| format!("{}: {error}", path.display()))?;
        // SAFETY: symbol type and name are the documented MiniForge ABI contract.
        let entry = unsafe { library.get::<NativeEntryV1Fn>(MINIFORGE_NATIVE_ENTRY_V1) }
            .map_err(|error| format!("missing miniforge_native_entry_v1: {error}"))?;
        // SAFETY: entry has no arguments and must return a descriptor with static library lifetime.
        let descriptor_ptr = unsafe { entry() };
        if descriptor_ptr.is_null() {
            return Err("entry returned a null descriptor".to_string());
        }
        // SAFETY: non-null descriptor pointer is owned by the loaded library and checked before use.
        let descriptor = unsafe { *descriptor_ptr };
        if descriptor.struct_size < std::mem::size_of::<MiniForgeNativePluginV1>() as u32 {
            return Err(format!(
                "descriptor is too small: {} bytes",
                descriptor.struct_size
            ));
        }
        if descriptor.abi_version != MINIFORGE_NATIVE_ABI_VERSION {
            return Err(format!(
                "plugin ABI v{} is incompatible with host ABI v{}",
                descriptor.abi_version, MINIFORGE_NATIVE_ABI_VERSION
            ));
        }
        let invoke_json = descriptor
            .invoke_json
            .ok_or_else(|| "invoke_json callback is required".to_string())?;
        let free_string = descriptor
            .free_string
            .ok_or_else(|| "free_string callback is required".to_string())?;
        let info = NativeLibraryInfo {
            id: manifest.id.clone(),
            name: read_c_string(descriptor.name, "name")?,
            version: read_c_string(descriptor.version, "version")?,
            category: read_c_string(descriptor.category, "category")?,
            path,
            services: manifest.services.clone(),
        };
        if let Some(initialize) = descriptor.initialize {
            // SAFETY: host table is static and ABI-compatible with the validated descriptor.
            let status = unsafe { initialize(&NATIVE_HOST_V1) };
            if status != 0 {
                return Err(format!("initialize failed with status {status}"));
            }
        }
        self.loaded.insert(
            manifest.id.clone(),
            LoadedNativeLibrary {
                _library: library,
                info: info.clone(),
                shutdown: descriptor.shutdown,
                invoke_json,
                free_string,
            },
        );
        Ok(info)
    }

    pub fn invoke(
        &self,
        id: &str,
        operation: &str,
        request: &Value,
    ) -> Result<NativeCallResult, String> {
        let plugin = self
            .loaded
            .get(id)
            .ok_or_else(|| format!("native library '{id}' is not loaded"))?;
        let operation =
            CString::new(operation).map_err(|_| "operation contains a NUL byte".to_string())?;
        let request =
            CString::new(serde_json::to_string(request).map_err(|error| error.to_string())?)
                .map_err(|_| "request JSON contains a NUL byte".to_string())?;
        let mut response = std::ptr::null_mut();
        // SAFETY: function pointer belongs to a loaded library; input strings live for the call.
        let status =
            unsafe { (plugin.invoke_json)(operation.as_ptr(), request.as_ptr(), &mut response) };
        let value = if response.is_null() {
            Value::Null
        } else {
            // SAFETY: plugin promises a NUL-terminated response and matching free_string callback.
            let text = unsafe { CStr::from_ptr(response) }
                .to_string_lossy()
                .into_owned();
            // SAFETY: response is returned by this plugin and freed exactly once.
            unsafe { (plugin.free_string)(response) };
            serde_json::from_str(&text)
                .map_err(|error| format!("invalid native response JSON: {error}"))?
        };
        Ok(NativeCallResult { status, value })
    }

    pub fn unload(&mut self, id: &str) -> bool {
        self.loaded.remove(id).is_some()
    }
    pub fn unload_all(&mut self) {
        self.loaded.clear();
    }
    pub fn loaded(&self) -> Vec<NativeLibraryInfo> {
        self.loaded
            .values()
            .map(|plugin| plugin.info.clone())
            .collect()
    }
}

fn read_c_string(pointer: *const c_char, field: &str) -> Result<String, String> {
    if pointer.is_null() {
        return Err(format!("descriptor {field} is null"));
    }
    // SAFETY: descriptor strings are required to be NUL-terminated and live with the library.
    Ok(unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned())
}

unsafe extern "C" fn native_host_log(level: u32, message: *const c_char) {
    if message.is_null() {
        return;
    }
    // SAFETY: native callers must provide a NUL-terminated message for the duration of this call.
    let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
    eprintln!("[MiniForge Native/{level}] {message}");
}

static NATIVE_HOST_V1: MiniForgeNativeHostV1 = MiniForgeNativeHostV1 {
    abi_version: MINIFORGE_NATIVE_ABI_VERSION,
    struct_size: std::mem::size_of::<MiniForgeNativeHostV1>() as u32,
    user_data: std::ptr::null_mut(),
    log: Some(native_host_log),
};

pub fn dynamic_library_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}
