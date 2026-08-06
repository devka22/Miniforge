use std::ffi::{CStr, c_char};
use std::path::Path;
use std::ptr;

use crate::engine::developer_console::ConsoleSeverity;
use crate::engine::editor_core::{
    EDITOR_CORE_API_VERSION, EditorCore, EditorCoreError, EditorCoreErrorKind, EditorOpenOptions,
};
use crate::engine::luau_scripting::ScriptBreakpoint;
use crate::engine::system_audit::SystemReadinessLevel;

pub const MF_ERROR_MESSAGE_CAPACITY: usize = 1024;
pub const MF_NAME_CAPACITY: usize = 128;
pub const MF_SHORT_TEXT_CAPACITY: usize = 64;
pub const MF_PATH_CAPACITY: usize = 512;
pub const MF_VALUE_CAPACITY: usize = 512;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfStatus {
    Ok = 0,
    Error = 1,
    NotFound = 2,
    InvalidArgument = 3,
    NoProjectOpen = 4,
    BufferTooSmall = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfError {
    pub status: MfStatus,
    pub message: [c_char; MF_ERROR_MESSAGE_CAPACITY],
}

#[repr(C)]
pub struct MfEditorHandle {
    core: EditorCore,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfEntityRow {
    pub abi_version: u32,
    pub struct_size: usize,
    pub id: u64,
    pub parent_id: u64,
    pub has_parent: u8,
    pub visible: u8,
    pub enabled: u8,
    pub locked: u8,
    pub selected: u8,
    pub component_count: usize,
    pub child_count: usize,
    pub x: f64,
    pub y: f64,
    pub name: [c_char; MF_NAME_CAPACITY],
    pub entity_type: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub tag: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub layer: [c_char; MF_SHORT_TEXT_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfInspectorField {
    pub abi_version: u32,
    pub struct_size: usize,
    pub entity_id: u64,
    pub editable: u8,
    pub target: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub key: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub display_name: [c_char; MF_NAME_CAPACITY],
    pub value_type: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub value_json: [c_char; MF_VALUE_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfAssetRow {
    pub abi_version: u32,
    pub struct_size: usize,
    pub size_bytes: u64,
    pub dependency_count: usize,
    pub guid: [c_char; MF_NAME_CAPACITY],
    pub relative_path: [c_char; MF_PATH_CAPACITY],
    pub name: [c_char; MF_NAME_CAPACITY],
    pub asset_type: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub labels: [c_char; MF_VALUE_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfContentMutationResult {
    pub abi_version: u32,
    pub struct_size: usize,
    pub relative_path: [c_char; MF_PATH_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfCommandDescriptor {
    pub abi_version: u32,
    pub struct_size: usize,
    pub enabled: u8,
    pub id: [c_char; MF_NAME_CAPACITY],
    pub label: [c_char; MF_NAME_CAPACITY],
    pub category: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub shortcut: [c_char; MF_SHORT_TEXT_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfConsoleEntry {
    pub abi_version: u32,
    pub struct_size: usize,
    pub frame: u64,
    pub severity: u32,
    pub channel: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub message: [c_char; MF_VALUE_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfReadinessRow {
    pub abi_version: u32,
    pub struct_size: usize,
    pub score: u8,
    pub level: u32,
    pub strength_count: usize,
    pub gap_count: usize,
    pub action_count: usize,
    pub system: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub level_label: [c_char; MF_SHORT_TEXT_CAPACITY],
    pub top_action: [c_char; MF_VALUE_CAPACITY],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfViewportInfo {
    pub abi_version: u32,
    pub struct_size: usize,
    pub width: u32,
    pub height: u32,
    pub required_bytes: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MfSpriteInfo {
    pub abi_version: u32,
    pub struct_size: usize,
    pub width: u32,
    pub height: u32,
    pub required_bytes: usize,
    pub can_undo: u8,
    pub can_redo: u8,
}

#[unsafe(no_mangle)]
/// Creates an editor handle owned by the C ABI caller.
///
/// # Safety
/// `error`, when non-null, must point to writable storage for one `MfError`.
/// The returned pointer must be released exactly once with `mf_editor_destroy`.
pub unsafe extern "C" fn mf_editor_create(error: *mut MfError) -> *mut MfEditorHandle {
    clear_error(error);
    Box::into_raw(Box::new(MfEditorHandle {
        core: EditorCore::new(),
    }))
}

#[unsafe(no_mangle)]
/// Destroys an editor handle created by `mf_editor_create`.
///
/// # Safety
/// `handle` must be null or a live pointer returned by `mf_editor_create`.
/// Passing the same non-null handle more than once is undefined behavior.
pub unsafe extern "C" fn mf_editor_destroy(handle: *mut MfEditorHandle) {
    if !handle.is_null() {
        // SAFETY: the handle was allocated by `mf_editor_create` and is destroyed once by caller.
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

#[unsafe(no_mangle)]
/// Opens a MiniForge project in an existing editor handle.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `path` must be a valid,
/// null-terminated UTF-8 string. `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_open_project(
    handle: *mut MfEditorHandle,
    path: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(path) = read_cstr(path) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Project path pointer is null or not valid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.open_project(path), error)
}

#[unsafe(no_mangle)]
/// Opens a MiniForge project with explicit native-editor startup options.
///
/// `options_json` accepts `{}` or an object with `safe_mode`,
/// `safe_mode_reason`, and `disable_asset_importers`. Unknown fields are
/// rejected so a misspelled recovery option cannot silently start normally.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `path` and `options_json`
/// must be valid null-terminated UTF-8 strings; `error`, when non-null, must be
/// writable.
pub unsafe extern "C" fn mf_editor_open_project_with_options(
    handle: *mut MfEditorHandle,
    path: *const c_char,
    options_json: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(path), Ok(options_json)) = (read_cstr(path), read_cstr(options_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Project path or open options are invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    let options = match serde_json::from_str::<EditorOpenOptions>(options_json) {
        Ok(options) => options,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    status_from_result(core.open_project_with_options(path, options), error)
}

#[unsafe(no_mangle)]
/// Returns whether the editor handle currently has a project open.
///
/// # Safety
/// `handle` must be null or a valid immutable editor handle.
pub unsafe extern "C" fn mf_editor_is_project_open(handle: *const MfEditorHandle) -> u8 {
    if handle.is_null() {
        return 0;
    }
    // SAFETY: null was checked; immutable borrow only.
    unsafe { u8::from((*handle).core.is_project_open()) }
}

#[unsafe(no_mangle)]
/// Refreshes live scene and asset views from the current Rust engine state.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `error`, when non-null,
/// must point to writable storage.
pub unsafe extern "C" fn mf_editor_refresh(
    handle: *mut MfEditorHandle,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.refresh(), error)
}

#[unsafe(no_mangle)]
/// Writes the open project path into a caller-provided string buffer.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `data` must point to
/// `capacity` writable bytes when `capacity > 0`; `required` and `error`, when
/// non-null, must point to writable storage.
pub unsafe extern "C" fn mf_editor_project_path(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let value = core
        .project_path()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    write_string_buffer(&value, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes the open project's editable settings bundle as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `data` must point to
/// `capacity` writable bytes when `capacity > 0`; `required` and `error`, when
/// non-null, must point to writable storage.
pub unsafe extern "C" fn mf_editor_project_settings_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(core.project_settings(), data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Validates and persists the engine settings JSON for the open project.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `source` must point to a
/// valid null-terminated UTF-8 string; `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_save_engine_settings_json(
    handle: *mut MfEditorHandle,
    source: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(source) = read_cstr(source) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Settings JSON is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.save_engine_settings_json(source), error)
}

#[unsafe(no_mangle)]
/// Validates and persists the input-map JSON for the open project.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `source` must point to a
/// valid null-terminated UTF-8 string; `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_save_input_map_json(
    handle: *mut MfEditorHandle,
    source: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(source) = read_cstr(source) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Input Map JSON is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.save_input_map_json(source), error)
}

#[unsafe(no_mangle)]
/// Validates and persists the tag and layer definitions for the open project.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `source` must point to a
/// valid null-terminated UTF-8 string; `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_save_tags_layers_json(
    handle: *mut MfEditorHandle,
    source: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(source) = read_cstr(source) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Tags/Layers JSON is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.save_tags_layers_json(source), error)
}

#[unsafe(no_mangle)]
/// Writes a launcher snapshot for projects under `workspace_root` as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle and `workspace_root` a
/// valid null-terminated UTF-8 string. Output buffer pointers follow
/// `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_launcher_snapshot_json(
    handle: *const MfEditorHandle,
    workspace_root: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(workspace_root) = read_cstr(workspace_root) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Workspace path is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.launcher_snapshot(workspace_root),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Creates a project from a launcher template and writes its path to `data`.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. All input string pointers
/// must reference valid null-terminated UTF-8 strings. Output buffer pointers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_launcher_create_project(
    handle: *const MfEditorHandle,
    workspace_root: *const c_char,
    location: *const c_char,
    name: *const c_char,
    template_name: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(workspace_root), Ok(location), Ok(name), Ok(template_name)) = (
        read_cstr(workspace_root),
        read_cstr(location),
        read_cstr(name),
        read_cstr(template_name),
    ) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Launcher argument is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    match core.launcher_create_project(workspace_root, location, name, template_name) {
        Ok(path) => write_string_buffer(&path.to_string_lossy(), data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Repairs launcher metadata for a project and writes the repair report as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `workspace_root` and
/// `project_path` must reference valid null-terminated UTF-8 strings. Output
/// buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_launcher_repair_project_json(
    handle: *const MfEditorHandle,
    workspace_root: *const c_char,
    project_path: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(workspace_root), Ok(project_path)) =
        (read_cstr(workspace_root), read_cstr(project_path))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Launcher path is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.launcher_repair_project(workspace_root, project_path),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Serializes project package, autosave/session, and external launch state.
///
/// # Safety
/// `handle` must be valid. `data` must point to `capacity` writable bytes when
/// non-null; `required` and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_project_operations_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(core.project_operations(), data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Executes exactly one project operation. Structured results are queried
/// separately with `mf_editor_project_operations_json`.
///
/// # Safety
/// `handle` must be valid and mutable. `action` and `payload_json` must be
/// valid null-terminated UTF-8 strings; `error`, when non-null, is writable.
pub unsafe extern "C" fn mf_editor_project_operation(
    handle: *mut MfEditorHandle,
    action: *const c_char,
    payload_json: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(payload_json)) = (read_cstr(action), read_cstr(payload_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Project operation received invalid text pointers",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.project_operation(action, payload_json), error)
}

#[unsafe(no_mangle)]
/// Writes the total number of hierarchy entity rows.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_count` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_entity_count(
    handle: *const MfEditorHandle,
    out_count: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_count) = out_ptr(out_count, error, "out_count") else {
        return MfStatus::InvalidArgument;
    };
    match core.entity_count() {
        Ok(count) => {
            *out_count = count;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes one hierarchy entity row by index.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_row` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_entity_row(
    handle: *const MfEditorHandle,
    index: usize,
    out_row: *mut MfEntityRow,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_row) = out_ptr(out_row, error, "out_row") else {
        return MfStatus::InvalidArgument;
    };
    match core.entity_at(index) {
        Ok(row) => {
            *out_row = row.into();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes a page of hierarchy entity rows.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `rows` must point to
/// `row_capacity` writable rows when `row_capacity > 0`. `out_written`,
/// `out_total`, and `error` when non-null must be writable.
pub unsafe extern "C" fn mf_editor_entity_rows(
    handle: *const MfEditorHandle,
    start_index: usize,
    rows: *mut MfEntityRow,
    row_capacity: usize,
    out_written: *mut usize,
    out_total: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_written) = out_ptr(out_written, error, "out_written") else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_total) = out_ptr(out_total, error, "out_total") else {
        return MfStatus::InvalidArgument;
    };
    if row_capacity > 0 && rows.is_null() {
        set_error(error, MfStatus::InvalidArgument, "rows pointer is null");
        return MfStatus::InvalidArgument;
    }

    let total = match core.entity_count() {
        Ok(count) => count,
        Err(error_value) => return set_core_error(error, error_value),
    };
    if start_index > total {
        set_error(error, MfStatus::NotFound, "Entity start index out of range");
        return MfStatus::NotFound;
    }
    let written = row_capacity.min(total.saturating_sub(start_index));
    for offset in 0..written {
        let row = match core.entity_at(start_index + offset) {
            Ok(row) => row,
            Err(error_value) => return set_core_error(error, error_value),
        };
        // SAFETY: null was checked for non-zero capacity and offset is within `written`.
        unsafe {
            *rows.add(offset) = row.into();
        }
    }
    *out_written = written;
    *out_total = total;
    MfStatus::Ok
}

#[unsafe(no_mangle)]
/// Writes the number of selected entities.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_count` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_selected_entity_count(
    handle: *const MfEditorHandle,
    out_count: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_count) = out_ptr(out_count, error, "out_count") else {
        return MfStatus::InvalidArgument;
    };
    match core.selected_entities() {
        Ok(selected) => {
            *out_count = selected.len();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes one selected entity id by selection index.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_entity_id` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_selected_entity(
    handle: *const MfEditorHandle,
    index: usize,
    out_entity_id: *mut u64,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_entity_id) = out_ptr(out_entity_id, error, "out_entity_id") else {
        return MfStatus::InvalidArgument;
    };
    match core.selected_entities() {
        Ok(selected) => match selected.get(index) {
            Some(entity_id) => {
                *out_entity_id = *entity_id;
                MfStatus::Ok
            }
            None => {
                set_error(
                    error,
                    MfStatus::NotFound,
                    "Selected entity index out of range",
                );
                MfStatus::NotFound
            }
        },
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Selects an entity by id.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `error`, when non-null,
/// must point to writable storage.
pub unsafe extern "C" fn mf_editor_select_entity(
    handle: *mut MfEditorHandle,
    entity_id: u64,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.select_entity(entity_id), error)
}

#[unsafe(no_mangle)]
/// Updates selection using `replace`, `add`, or `toggle` semantics.
///
/// # Safety
/// String pointers must be valid null-terminated UTF-8 and `handle` must be
/// exclusively borrowed for this call.
pub unsafe extern "C" fn mf_editor_update_selection(
    handle: *mut MfEditorHandle,
    entity_id: u64,
    mode: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(mode) = read_cstr(mode) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Selection mode is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.update_selection(entity_id, mode), error)
}

#[unsafe(no_mangle)]
/// Clears the current editor selection.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed for this call.
pub unsafe extern "C" fn mf_editor_clear_selection(
    handle: *mut MfEditorHandle,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.clear_selection(), error)
}

#[unsafe(no_mangle)]
/// Executes a parameterized hierarchy or Inspector quick action.
///
/// # Safety
/// `action` and `payload_json` must be valid null-terminated UTF-8 strings;
/// `out_entity_id` must point to writable storage.
pub unsafe extern "C" fn mf_editor_entity_action(
    handle: *mut MfEditorHandle,
    entity_id: u64,
    action: *const c_char,
    payload_json: *const c_char,
    out_entity_id: *mut u64,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_entity_id) = out_ptr(out_entity_id, error, "out_entity_id") else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(payload_json)) = (read_cstr(action), read_cstr(payload_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Entity action strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    match core.entity_action(entity_id, action, payload_json) {
        Ok(created_id) => {
            *out_entity_id = created_id.unwrap_or_default();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Executes a supported action for the complete current selection as one
/// editor-history command.
///
/// # Safety
/// Text pointers must be valid null-terminated UTF-8 and `out_changed` must
/// point to writable storage.
pub unsafe extern "C" fn mf_editor_selected_entity_action(
    handle: *mut MfEditorHandle,
    action: *const c_char,
    payload_json: *const c_char,
    out_changed: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_changed) = out_ptr(out_changed, error, "out_changed") else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(payload_json)) = (read_cstr(action), read_cstr(payload_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Selected entity action strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    match core.selected_entity_action(action, payload_json) {
        Ok(changed) => {
            *out_changed = changed;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Picks the frontmost selectable entity using the viewport snapshot mapping.
///
/// # Safety
/// `selection_mode` must be valid UTF-8 and `out_entity_id` writable.
pub unsafe extern "C" fn mf_editor_pick_entity(
    handle: *mut MfEditorHandle,
    viewport_width: u32,
    viewport_height: u32,
    x: f64,
    y: f64,
    selection_mode: *const c_char,
    out_entity_id: *mut u64,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_entity_id) = out_ptr(out_entity_id, error, "out_entity_id") else {
        return MfStatus::InvalidArgument;
    };
    let Ok(selection_mode) = read_cstr(selection_mode) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Selection mode is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    match core.pick_entity(viewport_width, viewport_height, x, y, selection_mode) {
        Ok(entity_id) => {
            *out_entity_id = entity_id.unwrap_or_default();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes professional Scene viewport metadata, including screen-space entity
/// bounds and the world-to-pixel scale used for picking and gizmos.
///
/// # Safety
/// Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_viewport_state_json(
    handle: *const MfEditorHandle,
    viewport_width: u32,
    viewport_height: u32,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.viewport_state(viewport_width, viewport_height),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Applies one undoable transform to every unlocked selected entity.
///
/// # Safety
/// `payload_json` must be valid null-terminated UTF-8 and `handle` must be
/// exclusively borrowed.
pub unsafe extern "C" fn mf_editor_transform_selection(
    handle: *mut MfEditorHandle,
    payload_json: *const c_char,
    out_changed: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_changed) = out_ptr(out_changed, error, "out_changed") else {
        return MfStatus::InvalidArgument;
    };
    let Ok(payload_json) = read_cstr(payload_json) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Selection transform payload is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    match core.transform_selection_json(payload_json) {
        Ok(changed) => {
            *out_changed = changed;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes active scene, dirty state, mode, and selection counts as JSON.
///
/// # Safety
/// Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_scene_state_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let state = match core.scene_state() {
        Ok(state) => state,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&state) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes the current Scene Browser state as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. Output buffer pointers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_scene_browser_state_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(core.scene_browser_state(), data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Executes a Scene Browser action and writes the resulting state as JSON.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `action` and `payload_json`
/// must reference valid null-terminated UTF-8 strings. Output buffer pointers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_scene_browser_action_json(
    handle: *mut MfEditorHandle,
    action: *const c_char,
    payload_json: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(payload_json)) = (read_cstr(action), read_cstr(payload_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Scene Browser action strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.scene_browser_action(action, payload_json),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Writes the categorized, creatable component catalog as JSON.
///
/// # Safety
/// Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_component_catalog_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let catalog = match core.component_catalog() {
        Ok(catalog) => catalog,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&catalog) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes the unified ready-made authoring preset catalog as JSON.
///
/// The catalog is project-independent and includes entity bundles, physics
/// profiles, workflow guidance, parameters and compatibility metadata.
///
/// # Safety
/// Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_authoring_catalog_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        Ok::<_, EditorCoreError>(core.authoring_catalog()),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Builds a read-only plan for applying one authoring preset to the current
/// selection, including per-entity component changes and world settings.
///
/// # Safety
/// `preset_id` and `parameters_json` must be valid null-terminated UTF-8.
/// Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_authoring_plan_json(
    handle: *const MfEditorHandle,
    preset_id: *const c_char,
    parameters_json: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(preset_id), Ok(parameters_json)) = (read_cstr(preset_id), read_cstr(parameters_json))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Authoring plan strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.authoring_application_plan(preset_id, parameters_json),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Writes the optional, versioned SDK/content pack catalog and validation
/// report as JSON.
///
/// # Safety
/// Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_sdk_pack_catalog_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        Ok::<_, EditorCoreError>(core.sdk_pack_catalog()),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Builds a dependency-resolved SDK/content pack installation plan.
///
/// # Safety
/// `profile_id` and `registry_json` must be valid null-terminated UTF-8.
/// Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_sdk_pack_plan_json(
    handle: *const MfEditorHandle,
    profile_id: *const c_char,
    registry_json: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(profile_id), Ok(registry_json)) = (read_cstr(profile_id), read_cstr(registry_json))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "SDK pack plan strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.sdk_pack_install_plan(profile_id, registry_json),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Verifies and atomically installs a downloaded SDK/content pack ZIP.
///
/// # Safety
/// All string pointers must reference valid null-terminated UTF-8. Buffer
/// pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_sdk_pack_install_archive_json(
    handle: *const MfEditorHandle,
    pack_id: *const c_char,
    artifact_json: *const c_char,
    archive_path: *const c_char,
    install_root: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(pack_id), Ok(artifact_json), Ok(archive_path), Ok(install_root)) = (
        read_cstr(pack_id),
        read_cstr(artifact_json),
        read_cstr(archive_path),
        read_cstr(install_root),
    ) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "SDK pack installation strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.sdk_pack_install_archive(
            pack_id,
            artifact_json,
            Path::new(archive_path),
            Path::new(install_root),
        ),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Writes the persisted state for an advanced editor tool as JSON.
///
/// Supported tool ids are `sequencer`, `tilemap`, and `ui_designer`.
///
/// # Safety
/// `tool` must be valid null-terminated UTF-8. Buffer pointers follow
/// `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_tool_state_json(
    handle: *const MfEditorHandle,
    tool: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(tool) = read_cstr(tool) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Editor tool id is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let state = match core.editor_tool_state(tool) {
        Ok(state) => state,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&state) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Executes an undoable advanced editor-tool operation and returns its new state.
///
/// # Safety
/// String pointers must be valid null-terminated UTF-8. Buffer pointers follow
/// `mf_editor_project_path` semantics and `handle` must be exclusively borrowed.
pub unsafe extern "C" fn mf_editor_tool_action_json(
    handle: *mut MfEditorHandle,
    tool: *const c_char,
    action: *const c_char,
    payload_json: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(tool), Ok(action), Ok(payload_json)) =
        (read_cstr(tool), read_cstr(action), read_cstr(payload_json))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Editor tool action strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let state = match core.editor_tool_action(tool, action, payload_json) {
        Ok(state) => state,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&state) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes the Prefab Studio asset list and selected-instance report as JSON.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. Buffer pointers follow
/// `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_prefab_state_json(
    handle: *mut MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let state = match core.prefab_studio_state() {
        Ok(state) => state,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&state) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Executes a Prefab Studio operation and writes its structured result.
///
/// # Safety
/// String pointers must be valid null-terminated UTF-8. Buffer pointers follow
/// `mf_editor_project_path` semantics and `handle` must be exclusively borrowed.
pub unsafe extern "C" fn mf_editor_prefab_action_json(
    handle: *mut MfEditorHandle,
    action: *const c_char,
    payload_json: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(payload_json)) = (read_cstr(action), read_cstr(payload_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Prefab action strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let result = match core.prefab_action(action, payload_json) {
        Ok(result) => result,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes the number of inspector fields for an entity.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_count` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_inspector_field_count(
    handle: *const MfEditorHandle,
    entity_id: u64,
    out_count: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_count) = out_ptr(out_count, error, "out_count") else {
        return MfStatus::InvalidArgument;
    };
    match core.inspector_fields(entity_id) {
        Ok(fields) => {
            *out_count = fields.len();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes one inspector field by entity id and field index.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_field` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_inspector_field(
    handle: *const MfEditorHandle,
    entity_id: u64,
    index: usize,
    out_field: *mut MfInspectorField,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_field) = out_ptr(out_field, error, "out_field") else {
        return MfStatus::InvalidArgument;
    };
    match core.inspector_fields(entity_id) {
        Ok(fields) => match fields.into_iter().nth(index) {
            Some(field) => {
                *out_field = field.into();
                MfStatus::Ok
            }
            None => {
                set_error(
                    error,
                    MfStatus::NotFound,
                    "Inspector field index out of range",
                );
                MfStatus::NotFound
            }
        },
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes a page of inspector fields for an entity.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `fields` must point to
/// `field_capacity` writable rows when `field_capacity > 0`. `out_written`,
/// `out_total`, and `error` when non-null must be writable.
pub unsafe extern "C" fn mf_editor_inspector_fields(
    handle: *const MfEditorHandle,
    entity_id: u64,
    start_index: usize,
    fields: *mut MfInspectorField,
    field_capacity: usize,
    out_written: *mut usize,
    out_total: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_written) = out_ptr(out_written, error, "out_written") else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_total) = out_ptr(out_total, error, "out_total") else {
        return MfStatus::InvalidArgument;
    };
    if field_capacity > 0 && fields.is_null() {
        set_error(error, MfStatus::InvalidArgument, "fields pointer is null");
        return MfStatus::InvalidArgument;
    }

    let inspector_fields = match core.inspector_fields(entity_id) {
        Ok(fields) => fields,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let total = inspector_fields.len();
    if start_index > total {
        set_error(
            error,
            MfStatus::NotFound,
            "Inspector field start index out of range",
        );
        return MfStatus::NotFound;
    }
    let written = field_capacity.min(total.saturating_sub(start_index));
    for offset in 0..written {
        let field = inspector_fields[start_index + offset].clone();
        // SAFETY: null was checked for non-zero capacity and offset is within `written`.
        unsafe {
            *fields.add(offset) = field.into();
        }
    }
    *out_written = written;
    *out_total = total;
    MfStatus::Ok
}

#[unsafe(no_mangle)]
/// Serializes the available Inspector quick actions and their compatible
/// indexed assets for one entity.
///
/// # Safety
/// Buffer arguments follow the standard MiniForge two-call string contract.
pub unsafe extern "C" fn mf_editor_inspector_quick_actions_json(
    handle: *const MfEditorHandle,
    entity_id: u64,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.inspector_quick_actions(entity_id),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Executes one validated Inspector quick action.
///
/// # Safety
/// `action` and `asset_path` must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn mf_editor_inspector_quick_action(
    handle: *mut MfEditorHandle,
    entity_id: u64,
    action: *const c_char,
    asset_path: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(asset_path)) = (read_cstr(action), read_cstr(asset_path)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Inspector quick action strings are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(
        core.execute_inspector_quick_action(entity_id, action, asset_path)
            .map(|_| ()),
        error,
    )
}

#[unsafe(no_mangle)]
/// Applies an inspector edit using JSON text.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `target`, `key`, and
/// `value_json` must be valid null-terminated UTF-8 strings. `error`, when
/// non-null, must be writable.
pub unsafe extern "C" fn mf_editor_set_inspector_value_json(
    handle: *mut MfEditorHandle,
    entity_id: u64,
    target: *const c_char,
    key: *const c_char,
    value_json: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(target), Ok(key), Ok(value_json)) =
        (read_cstr(target), read_cstr(key), read_cstr(value_json))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Inspector edit received invalid text pointers",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(
        core.edit_inspector_value_json(entity_id, target, key, value_json)
            .map(|_| ()),
        error,
    )
}

#[unsafe(no_mangle)]
/// Edits one property shared by every selected entity as a single undo step.
///
/// # Safety
/// Text pointers must be valid null-terminated UTF-8 and `out_changed`, when
/// non-null, must be writable.
pub unsafe extern "C" fn mf_editor_set_selected_inspector_value_json(
    handle: *mut MfEditorHandle,
    target: *const c_char,
    key: *const c_char,
    value_json: *const c_char,
    out_changed: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_changed) = out_ptr(out_changed, error, "out_changed") else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(target), Ok(key), Ok(value_json)) =
        (read_cstr(target), read_cstr(key), read_cstr(value_json))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Common Inspector edit received invalid text pointers",
        );
        return MfStatus::InvalidArgument;
    };
    match core.edit_selected_inspector_value_json(target, key, value_json) {
        Ok(changed) => {
            *out_changed = changed;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes the number of project assets visible to the editor.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_count` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_asset_count(
    handle: *const MfEditorHandle,
    out_count: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_count) = out_ptr(out_count, error, "out_count") else {
        return MfStatus::InvalidArgument;
    };
    match core.asset_count() {
        Ok(count) => {
            *out_count = count;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes one asset browser row by index.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_row` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_asset_row(
    handle: *const MfEditorHandle,
    index: usize,
    out_row: *mut MfAssetRow,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_row) = out_ptr(out_row, error, "out_row") else {
        return MfStatus::InvalidArgument;
    };
    match core.asset_at(index) {
        Ok(row) => {
            *out_row = row.into();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes a page of asset browser rows.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `rows` must point to
/// `row_capacity` writable rows when `row_capacity > 0`. `out_written`,
/// `out_total`, and `error` when non-null must be writable.
pub unsafe extern "C" fn mf_editor_asset_rows(
    handle: *const MfEditorHandle,
    start_index: usize,
    rows: *mut MfAssetRow,
    row_capacity: usize,
    out_written: *mut usize,
    out_total: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_written) = out_ptr(out_written, error, "out_written") else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_total) = out_ptr(out_total, error, "out_total") else {
        return MfStatus::InvalidArgument;
    };
    if row_capacity > 0 && rows.is_null() {
        set_error(error, MfStatus::InvalidArgument, "rows pointer is null");
        return MfStatus::InvalidArgument;
    }

    let total = match core.asset_count() {
        Ok(count) => count,
        Err(error_value) => return set_core_error(error, error_value),
    };
    if start_index > total {
        set_error(error, MfStatus::NotFound, "Asset start index out of range");
        return MfStatus::NotFound;
    }
    let written = row_capacity.min(total.saturating_sub(start_index));
    for offset in 0..written {
        let row = match core.asset_at(start_index + offset) {
            Ok(row) => row,
            Err(error_value) => return set_core_error(error, error_value),
        };
        // SAFETY: null was checked for non-zero capacity and offset is within `written`.
        unsafe {
            *rows.add(offset) = row.into();
        }
    }
    *out_written = written;
    *out_total = total;
    MfStatus::Ok
}

#[unsafe(no_mangle)]
/// Serializes the safe project-folder tree used by native Content Browsers.
///
/// # Safety
/// Output buffers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_content_folders_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(core.content_folders(), data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Serializes one confined Content Browser directory and AssetDatabase metadata.
///
/// # Safety
/// `relative_directory` must be valid null-terminated UTF-8. Output buffers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_content_entries_json(
    handle: *const MfEditorHandle,
    relative_directory: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(relative_directory) = read_cstr(relative_directory) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Content directory is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.content_entries(relative_directory),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Creates one confined project folder and refreshes AssetDatabase metadata.
///
/// # Safety
/// String pointers must be valid null-terminated UTF-8.
pub unsafe extern "C" fn mf_editor_content_create_folder(
    handle: *mut MfEditorHandle,
    relative_directory: *const c_char,
    name: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(relative_directory), Ok(name)) = (read_cstr(relative_directory), read_cstr(name))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Content folder arguments are invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(
        core.create_content_folder(relative_directory, name)
            .map(|_| ()),
        error,
    )
}

#[unsafe(no_mangle)]
/// Creates one confined project asset and returns its relative path in a fixed
/// ABI result. Unlike a probe/write string API, this mutation executes once.
///
/// # Safety
/// String pointers must be valid null-terminated UTF-8 and `out_result` must
/// point to writable `MfContentMutationResult` storage.
pub unsafe extern "C" fn mf_editor_content_create_file(
    handle: *mut MfEditorHandle,
    kind: *const c_char,
    relative_directory: *const c_char,
    name: *const c_char,
    out_result: *mut MfContentMutationResult,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_result) = out_ptr(out_result, error, "out_result") else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(kind), Ok(relative_directory), Ok(name)) = (
        read_cstr(kind),
        read_cstr(relative_directory),
        read_cstr(name),
    ) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Content file arguments are invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    match core.create_content_file(kind, relative_directory, name) {
        Ok(relative_path) => {
            let mut result = MfContentMutationResult::default();
            write_fixed(&mut result.relative_path, &relative_path);
            *out_result = result;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Reads one confined editable UTF-8 project asset through a dynamic buffer.
///
/// # Safety
/// `relative_path` must be valid null-terminated UTF-8. Output buffers follow
/// `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_content_read_text(
    handle: *const MfEditorHandle,
    relative_path: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(relative_path) = read_cstr(relative_path) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Content text path is invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    let source = match core.read_text_asset(relative_path) {
        Ok(source) => source,
        Err(error_value) => return set_core_error(error, error_value),
    };
    write_string_buffer(&source, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Saves one confined editable project asset atomically.
///
/// # Safety
/// String pointers must be valid null-terminated UTF-8.
pub unsafe extern "C" fn mf_editor_content_save_text(
    handle: *mut MfEditorHandle,
    relative_path: *const c_char,
    source: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(relative_path), Ok(source)) = (read_cstr(relative_path), read_cstr(source)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Content text arguments are invalid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.save_text_asset(relative_path, source), error)
}

#[unsafe(no_mangle)]
/// Applies exactly one Content Browser mutation and refreshes asset metadata.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `action` and
/// `payload_json` must be valid null-terminated UTF-8 strings. This function
/// is deliberately status-only so callers never repeat a destructive action
/// while probing an output buffer size.
pub unsafe extern "C" fn mf_editor_manage_asset(
    handle: *mut MfEditorHandle,
    action: *const c_char,
    payload_json: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(payload_json)) = (read_cstr(action), read_cstr(payload_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Asset management received invalid text pointers",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.manage_asset(action, payload_json).map(|_| ()), error)
}

#[unsafe(no_mangle)]
/// Serializes the latest real runtime profiler sample as JSON.
///
/// # Safety
/// `handle` must be valid. `data` must point to `capacity` writable bytes when
/// non-null; `required` and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_profiler_snapshot_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(core.profiler_snapshot(), data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Re-scans project files and rebuilds the persistent asset dependency graph.
///
/// # Safety
/// `handle` must be a valid mutable editor handle and `error`, when non-null,
/// must be writable.
pub unsafe extern "C" fn mf_editor_rebuild_asset_dependencies(
    handle: *mut MfEditorHandle,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.rebuild_asset_dependencies(), error)
}

#[unsafe(no_mangle)]
/// Serializes the current dependency graph without mutating project files.
///
/// # Safety
/// `handle` must be valid. `data` must point to `capacity` writable bytes when
/// non-null; `required` and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_asset_dependency_graph_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.asset_dependency_graph(),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Writes the number of commands exposed by the editor command palette.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_count` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_command_count(
    handle: *const MfEditorHandle,
    out_count: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_count) = out_ptr(out_count, error, "out_count") else {
        return MfStatus::InvalidArgument;
    };
    *out_count = core.command_count();
    MfStatus::Ok
}

#[unsafe(no_mangle)]
/// Writes one command descriptor by index.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_command` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_command_descriptor(
    handle: *const MfEditorHandle,
    index: usize,
    out_command: *mut MfCommandDescriptor,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_command) = out_ptr(out_command, error, "out_command") else {
        return MfStatus::InvalidArgument;
    };
    match core.command_at(index) {
        Ok(command) => {
            *out_command = command.into();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes a page of command descriptors.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `commands` must point to
/// `command_capacity` writable rows when `command_capacity > 0`. `out_written`,
/// `out_total`, and `error` when non-null must be writable.
pub unsafe extern "C" fn mf_editor_command_descriptors(
    handle: *const MfEditorHandle,
    start_index: usize,
    commands: *mut MfCommandDescriptor,
    command_capacity: usize,
    out_written: *mut usize,
    out_total: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_written) = out_ptr(out_written, error, "out_written") else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_total) = out_ptr(out_total, error, "out_total") else {
        return MfStatus::InvalidArgument;
    };
    if command_capacity > 0 && commands.is_null() {
        set_error(error, MfStatus::InvalidArgument, "commands pointer is null");
        return MfStatus::InvalidArgument;
    }

    let total = core.command_count();
    if start_index > total {
        set_error(
            error,
            MfStatus::NotFound,
            "Command start index out of range",
        );
        return MfStatus::NotFound;
    }
    let written = command_capacity.min(total.saturating_sub(start_index));
    for offset in 0..written {
        let command = match core.command_at(start_index + offset) {
            Ok(command) => command,
            Err(error_value) => return set_core_error(error, error_value),
        };
        // SAFETY: null was checked for non-zero capacity and offset is within `written`.
        unsafe {
            *commands.add(offset) = command.into();
        }
    }
    *out_written = written;
    *out_total = total;
    MfStatus::Ok
}

#[unsafe(no_mangle)]
/// Executes an editor command by id.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `command_id` must be a
/// valid null-terminated UTF-8 string. `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_execute_command(
    handle: *mut MfEditorHandle,
    command_id: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(command_id) = read_cstr(command_id) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Command id pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.execute_command(command_id).map(|_| ()), error)
}

#[unsafe(no_mangle)]
/// Writes the indexed Luau script list as a UTF-8 JSON array.
///
/// # Safety
/// `handle` must be a valid editor handle. `data` must point to `capacity`
/// writable bytes when non-null; `required` and `error`, when non-null, must
/// be writable.
pub unsafe extern "C" fn mf_editor_luau_scripts_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let scripts = match core.luau_scripts() {
        Ok(scripts) => scripts,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&scripts) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes one project Luau script into a caller-provided UTF-8 buffer.
///
/// # Safety
/// `handle` must be valid and `relative_path` must be a valid null-terminated
/// UTF-8 string. Buffer pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_luau_read(
    handle: *const MfEditorHandle,
    relative_path: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(relative_path) = read_cstr(relative_path) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Luau path pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let source = match core.read_luau_script(relative_path) {
        Ok(source) => source,
        Err(error_value) => return set_core_error(error, error_value),
    };
    write_string_buffer(&source, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Validates unsaved Luau source and writes `{valid, diagnostic}` as JSON.
///
/// # Safety
/// `handle` must be valid; `relative_path` and `source` must be valid
/// null-terminated UTF-8 strings. Output pointers follow string-buffer rules.
pub unsafe extern "C" fn mf_editor_luau_validate_json(
    handle: *const MfEditorHandle,
    relative_path: *const c_char,
    source: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(relative_path), Ok(source)) = (read_cstr(relative_path), read_cstr(source)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Luau path or source pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let validation = match core.validate_luau_source(relative_path, source) {
        Ok(validation) => validation,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&validation) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Atomically saves a validated Luau script and rotates recovery backups.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed for the call;
/// `relative_path` and `source` must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn mf_editor_luau_save(
    handle: *mut MfEditorHandle,
    relative_path: *const c_char,
    source: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(relative_path), Ok(source)) = (read_cstr(relative_path), read_cstr(source)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Luau path or source pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.save_luau_script(relative_path, source), error)
}

#[unsafe(no_mangle)]
/// Writes the current Luau debugger state as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. Output buffer pointers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_luau_debug_state_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let state = match core.luau_debug_state() {
        Ok(state) => state,
        Err(error_value) => return set_core_error(error, error_value),
    };
    match serde_json::to_string(&state) {
        Ok(json) => write_string_buffer(&json, data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value.into()),
    }
}

#[unsafe(no_mangle)]
/// Replaces all Luau debugger breakpoints from a JSON array.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `breakpoints_json` must
/// reference a valid null-terminated UTF-8 string; `error`, when non-null, must
/// be writable.
pub unsafe extern "C" fn mf_editor_luau_set_breakpoints_json(
    handle: *mut MfEditorHandle,
    breakpoints_json: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(breakpoints_json) = read_cstr(breakpoints_json) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "breakpoints JSON pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let breakpoints = match serde_json::from_str::<Vec<ScriptBreakpoint>>(breakpoints_json) {
        Ok(breakpoints) => breakpoints,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    status_from_result(core.set_luau_breakpoints(breakpoints), error)
}

#[unsafe(no_mangle)]
/// Applies one command to the Luau debugger.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `command` must reference a
/// valid null-terminated UTF-8 string; `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_luau_debug_command(
    handle: *mut MfEditorHandle,
    command: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(command) = read_cstr(command) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "debug command pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.luau_debug_command(command).map(|_| ()), error)
}

#[unsafe(no_mangle)]
/// Evaluates Luau watch expressions and writes their values as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `expressions_json` must
/// reference a valid null-terminated UTF-8 string. Output buffer pointers follow
/// `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_luau_watches_json(
    handle: *const MfEditorHandle,
    expressions_json: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(expressions_json) = read_cstr(expressions_json) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "watch expressions pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let expressions = match serde_json::from_str::<Vec<String>>(expressions_json) {
        Ok(expressions) => expressions,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    let watches = match core.evaluate_luau_watches(&expressions) {
        Ok(watches) => watches,
        Err(error_value) => return set_core_error(error, error_value),
    };
    match serde_json::to_string(&watches) {
        Ok(json) => write_string_buffer(&json, data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value.into()),
    }
}

#[unsafe(no_mangle)]
/// Validates a visual-graph source and writes the diagnostics as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `relative_path` and
/// `source` must reference valid null-terminated UTF-8 strings. Output buffer
/// pointers follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_visual_graph_validate_json(
    handle: *const MfEditorHandle,
    relative_path: *const c_char,
    source: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(relative_path), Ok(source)) = (read_cstr(relative_path), read_cstr(source)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Visual Graph path or source pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let validation = match core.validate_visual_graph_source(relative_path, source) {
        Ok(validation) => validation,
        Err(error_value) => return set_core_error(error, error_value),
    };
    match serde_json::to_string(&validation) {
        Ok(json) => write_string_buffer(&json, data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value.into()),
    }
}

#[unsafe(no_mangle)]
/// Validates and persists a visual-graph source file.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `relative_path` and `source`
/// must reference valid null-terminated UTF-8 strings; `error`, when non-null,
/// must be writable.
pub unsafe extern "C" fn mf_editor_visual_graph_save(
    handle: *mut MfEditorHandle,
    relative_path: *const c_char,
    source: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(relative_path), Ok(source)) = (read_cstr(relative_path), read_cstr(source)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Visual Graph path or source pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.save_visual_graph(relative_path, source), error)
}

#[unsafe(no_mangle)]
/// Writes the visual-graph node and template catalog as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. Output buffer pointers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_visual_graph_catalog_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    match core
        .visual_graph_catalog()
        .and_then(|value| serde_json::to_string(&value).map_err(EditorCoreError::from))
    {
        Ok(json) => write_string_buffer(&json, data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Creates a visual graph from a named template.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `relative_path` and
/// `template_name` must reference valid null-terminated UTF-8 strings; `error`,
/// when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_visual_graph_create_template(
    handle: *mut MfEditorHandle,
    relative_path: *const c_char,
    template_name: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(relative_path), Ok(template_name)) =
        (read_cstr(relative_path), read_cstr(template_name))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Visual Graph template arguments are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(
        core.create_visual_graph_from_template(relative_path, template_name),
        error,
    )
}

#[unsafe(no_mangle)]
/// Writes the Python toolchain and tool catalog state as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. Output buffer pointers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_python_tools_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    match core
        .python_tools_state()
        .and_then(|value| serde_json::to_string(&value).map_err(EditorCoreError::from))
    {
        Ok(json) => write_string_buffer(&json, data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Installs the engine-managed Python tooling for the open project.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed; `error`, when non-null,
/// must point to writable storage.
pub unsafe extern "C" fn mf_editor_python_install_tools(
    handle: *mut MfEditorHandle,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.install_python_tools().map(|_| ()), error)
}

#[unsafe(no_mangle)]
/// Runs a registered Python tool with JSON parameters.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed. `tool_id` and
/// `parameters_json` must reference valid null-terminated UTF-8 strings;
/// `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_python_run_tool(
    handle: *mut MfEditorHandle,
    tool_id: *const c_char,
    parameters_json: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(tool_id), Ok(parameters_json)) = (read_cstr(tool_id), read_cstr(parameters_json))
    else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Python tool arguments are invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let parameters = match serde_json::from_str(parameters_json) {
        Ok(value) => value,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    status_from_result(core.run_python_tool(tool_id, parameters).map(|_| ()), error)
}

#[unsafe(no_mangle)]
/// Writes the most recent Python tool result as JSON.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. Output buffer pointers
/// follow `mf_editor_project_path` semantics.
pub unsafe extern "C" fn mf_editor_python_last_result_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    match core
        .last_python_result()
        .and_then(|value| serde_json::to_string(value).map_err(EditorCoreError::from))
    {
        Ok(json) => write_string_buffer(&json, data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Exports the open project with a named runtime profile.
///
/// The resulting report remains available through
/// `mf_editor_last_export_report_json` so the export is never repeated merely
/// to size an output buffer.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed; `profile` must be a valid
/// null-terminated UTF-8 string.
pub unsafe extern "C" fn mf_editor_export_runtime(
    handle: *mut MfEditorHandle,
    profile: *const c_char,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(profile) = read_cstr(profile) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Runtime export profile pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.export_runtime_profile(profile), error)
}

#[unsafe(no_mangle)]
/// Writes the most recent runtime export report as UTF-8 JSON.
///
/// # Safety
/// `handle` must be a valid editor handle. Output pointers follow the standard
/// MiniForge string-buffer contract.
pub unsafe extern "C" fn mf_editor_last_export_report_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let json = match core
        .last_export_report()
        .and_then(|report| serde_json::to_string(report).map_err(EditorCoreError::from))
    {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes the current runtime stability and frame-health snapshot as UTF-8 JSON.
///
/// # Safety
/// `handle` must be a valid editor handle. Output pointers follow the standard
/// MiniForge string-buffer contract.
pub unsafe extern "C" fn mf_editor_runtime_health_json(
    handle: *const MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let json = match core
        .runtime_health()
        .and_then(|health| serde_json::to_string(&health).map_err(EditorCoreError::from))
    {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes Forge AI Project Doctor diagnostics as a UTF-8 JSON array.
///
/// Call with a null `data` pointer or insufficient `capacity` to query the
/// required byte count (including the trailing NUL) through `required`.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `data` must point to
/// `capacity` writable bytes when non-null; `required` and `error`, when
/// non-null, must point to writable storage.
pub unsafe extern "C" fn mf_editor_forge_ai_diagnostics_json(
    handle: *mut MfEditorHandle,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let diagnostics = match core.forge_ai_diagnostics() {
        Ok(diagnostics) => diagnostics,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&diagnostics) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Runs a named Forge AI test suite and writes its report as UTF-8 JSON.
///
/// Call with a null `data` pointer or insufficient `capacity` to query the
/// required byte count (including the trailing NUL) through `required`.
///
/// # Safety
/// `handle` must be a valid mutable editor handle. `suite_id` must be a valid,
/// null-terminated UTF-8 string. `data` must point to `capacity` writable bytes
/// when non-null; `required` and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_forge_ai_run_test_json(
    handle: *mut MfEditorHandle,
    suite_id: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(suite_id) = read_cstr(suite_id) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Forge AI suite id pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    let report = match core.forge_ai_run_test(suite_id) {
        Ok(report) => report,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&report) {
        Ok(json) => json,
        Err(error_value) => return set_core_error(error, error_value.into()),
    };
    write_string_buffer(&json, data, capacity, required, error)
}

#[unsafe(no_mangle)]
/// Writes the number of console entries.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_count` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_console_count(
    handle: *const MfEditorHandle,
    out_count: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_count) = out_ptr(out_count, error, "out_count") else {
        return MfStatus::InvalidArgument;
    };
    match core.console_count() {
        Ok(count) => {
            *out_count = count;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes one console entry by index.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_entry` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_console_entry(
    handle: *const MfEditorHandle,
    index: usize,
    out_entry: *mut MfConsoleEntry,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_entry) = out_ptr(out_entry, error, "out_entry") else {
        return MfStatus::InvalidArgument;
    };
    match core.console_entry_at(index) {
        Ok(entry) => {
            *out_entry = entry.into();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes a page of console entries.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `entries` must point to
/// `entry_capacity` writable rows when `entry_capacity > 0`. `out_written`,
/// `out_total`, and `error` when non-null must be writable.
pub unsafe extern "C" fn mf_editor_console_entries(
    handle: *const MfEditorHandle,
    start_index: usize,
    entries: *mut MfConsoleEntry,
    entry_capacity: usize,
    out_written: *mut usize,
    out_total: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_written) = out_ptr(out_written, error, "out_written") else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_total) = out_ptr(out_total, error, "out_total") else {
        return MfStatus::InvalidArgument;
    };
    if entry_capacity > 0 && entries.is_null() {
        set_error(error, MfStatus::InvalidArgument, "entries pointer is null");
        return MfStatus::InvalidArgument;
    }

    let total = match core.console_count() {
        Ok(count) => count,
        Err(error_value) => return set_core_error(error, error_value),
    };
    if start_index > total {
        set_error(
            error,
            MfStatus::NotFound,
            "Console start index out of range",
        );
        return MfStatus::NotFound;
    }
    let written = entry_capacity.min(total.saturating_sub(start_index));
    for offset in 0..written {
        let entry = match core.console_entry_at(start_index + offset) {
            Ok(entry) => entry,
            Err(error_value) => return set_core_error(error, error_value),
        };
        // SAFETY: null was checked for non-zero capacity and offset is within `written`.
        unsafe {
            *entries.add(offset) = entry.into();
        }
    }
    *out_written = written;
    *out_total = total;
    MfStatus::Ok
}

#[unsafe(no_mangle)]
/// Writes the current project readiness score.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_score` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_readiness_score(
    handle: *const MfEditorHandle,
    out_score: *mut u8,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_score) = out_ptr(out_score, error, "out_score") else {
        return MfStatus::InvalidArgument;
    };
    match core.readiness_score() {
        Ok(score) => {
            *out_score = score;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes the number of readiness rows.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_count` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_readiness_count(
    handle: *const MfEditorHandle,
    out_count: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_count) = out_ptr(out_count, error, "out_count") else {
        return MfStatus::InvalidArgument;
    };
    match core.readiness_count() {
        Ok(count) => {
            *out_count = count;
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes one readiness row by index.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `out_row` must be
/// writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_readiness_row(
    handle: *const MfEditorHandle,
    index: usize,
    out_row: *mut MfReadinessRow,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_row) = out_ptr(out_row, error, "out_row") else {
        return MfStatus::InvalidArgument;
    };
    match core.readiness_at(index) {
        Ok(row) => {
            *out_row = row.into();
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Writes a page of readiness rows.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `rows` must point to
/// `row_capacity` writable rows when `row_capacity > 0`. `out_written`,
/// `out_total`, and `error` when non-null must be writable.
pub unsafe extern "C" fn mf_editor_readiness_rows(
    handle: *const MfEditorHandle,
    start_index: usize,
    rows: *mut MfReadinessRow,
    row_capacity: usize,
    out_written: *mut usize,
    out_total: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_written) = out_ptr(out_written, error, "out_written") else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_total) = out_ptr(out_total, error, "out_total") else {
        return MfStatus::InvalidArgument;
    };
    if row_capacity > 0 && rows.is_null() {
        set_error(error, MfStatus::InvalidArgument, "rows pointer is null");
        return MfStatus::InvalidArgument;
    }

    let total = match core.readiness_count() {
        Ok(count) => count,
        Err(error_value) => return set_core_error(error, error_value),
    };
    if start_index > total {
        set_error(
            error,
            MfStatus::NotFound,
            "Readiness start index out of range",
        );
        return MfStatus::NotFound;
    }
    let written = row_capacity.min(total.saturating_sub(start_index));
    for offset in 0..written {
        let row = match core.readiness_at(start_index + offset) {
            Ok(row) => row,
            Err(error_value) => return set_core_error(error, error_value),
        };
        // SAFETY: null was checked for non-zero capacity and offset is within `written`.
        unsafe {
            *rows.add(offset) = row.into();
        }
    }
    *out_written = written;
    *out_total = total;
    MfStatus::Ok
}

#[unsafe(no_mangle)]
/// Writes the active sprite canvas as tightly packed RGBA pixels.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `data` must point to
/// `capacity` writable bytes unless probing. `out_info` and `error`, when
/// non-null, must point to writable storage.
pub unsafe extern "C" fn mf_editor_sprite_snapshot_rgba(
    handle: *const MfEditorHandle,
    data: *mut u8,
    capacity: usize,
    out_info: *mut MfSpriteInfo,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_info) = out_ptr(out_info, error, "out_info") else {
        return MfStatus::InvalidArgument;
    };
    match core.sprite_snapshot() {
        Ok(snapshot) => {
            let required = snapshot.rgba.len();
            *out_info = MfSpriteInfo {
                abi_version: EDITOR_CORE_API_VERSION,
                struct_size: std::mem::size_of::<MfSpriteInfo>(),
                width: snapshot.width,
                height: snapshot.height,
                required_bytes: required,
                can_undo: u8::from(core.sprite_can_undo().unwrap_or(false)),
                can_redo: u8::from(core.sprite_can_redo().unwrap_or(false)),
            };
            if data.is_null() || capacity < required {
                set_error(
                    error,
                    MfStatus::BufferTooSmall,
                    "Sprite RGBA buffer is too small",
                );
                return MfStatus::BufferTooSmall;
            }
            // SAFETY: caller provided a buffer with at least `required` bytes.
            unsafe {
                ptr::copy_nonoverlapping(snapshot.rgba.as_ptr(), data, required);
            }
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Creates a new active sprite canvas.
///
/// # Safety
/// `handle` must be a valid mutable editor handle and `error`, when non-null,
/// must point to writable storage.
pub unsafe extern "C" fn mf_editor_sprite_new_canvas(
    handle: *mut MfEditorHandle,
    width: u32,
    height: u32,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.sprite_new_canvas(width, height), error)
}

#[unsafe(no_mangle)]
/// Begins a coalesced sprite edit (normally one pointer stroke).
///
/// # Safety
/// `handle` must be valid and exclusively borrowed for this call.
pub unsafe extern "C" fn mf_editor_sprite_begin_edit(
    handle: *mut MfEditorHandle,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(core.sprite_begin_edit(), error)
}

#[unsafe(no_mangle)]
/// Sets one pixel on the active sprite canvas.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed for this call.
pub unsafe extern "C" fn mf_editor_sprite_set_pixel(
    handle: *mut MfEditorHandle,
    x: u32,
    y: u32,
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
    out_changed: *mut u8,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_changed) = out_ptr(out_changed, error, "out_changed") else {
        return MfStatus::InvalidArgument;
    };
    match core.sprite_set_pixel(
        x,
        y,
        crate::engine::sprite_editor::SpriteColor {
            r: red,
            g: green,
            b: blue,
            a: alpha,
        },
    ) {
        Ok(changed) => {
            *out_changed = u8::from(changed);
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Clears the active sprite canvas without closing the current edit.
///
/// # Safety
/// `handle` must be valid and exclusively borrowed for this call.
pub unsafe extern "C" fn mf_editor_sprite_clear(
    handle: *mut MfEditorHandle,
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    status_from_result(
        core.sprite_clear(crate::engine::sprite_editor::SpriteColor {
            r: red,
            g: green,
            b: blue,
            a: alpha,
        }),
        error,
    )
}

#[unsafe(no_mangle)]
/// Applies an advanced sprite utility as one undoable edit.
///
/// # Safety
/// `handle` must be valid, string pointers must reference NUL-terminated UTF-8
/// and `out_changed`, when non-null, must point to writable memory.
pub unsafe extern "C" fn mf_editor_sprite_transform(
    handle: *mut MfEditorHandle,
    action: *const c_char,
    payload_json: *const c_char,
    out_changed: *mut u8,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_changed) = out_ptr(out_changed, error, "out_changed") else {
        return MfStatus::InvalidArgument;
    };
    let (Ok(action), Ok(payload_json)) = (read_cstr(action), read_cstr(payload_json)) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Sprite transform strings must be valid UTF-8",
        );
        return MfStatus::InvalidArgument;
    };
    match core.sprite_transform(action, payload_json) {
        Ok(changed) => {
            *out_changed = u8::from(changed);
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Returns the generated animation clip and timeline for a sprite-sheet grid.
///
/// # Safety
/// `handle` must be valid and output pointers must reference writable memory.
pub unsafe extern "C" fn mf_editor_sprite_animation_clip_json(
    handle: *const MfEditorHandle,
    frame_width: u32,
    frame_height: u32,
    fps: f32,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    write_serialized_result(
        core.sprite_animation_clip(frame_width, frame_height, fps),
        data,
        capacity,
        required,
        error,
    )
}

#[unsafe(no_mangle)]
/// Commits a coalesced sprite edit and reports whether pixels changed.
///
/// # Safety
/// `handle` and `out_changed` must be valid writable pointers.
pub unsafe extern "C" fn mf_editor_sprite_commit_edit(
    handle: *mut MfEditorHandle,
    out_changed: *mut u8,
    error: *mut MfError,
) -> MfStatus {
    sprite_history_action(handle, out_changed, error, EditorCore::sprite_commit_edit)
}

#[unsafe(no_mangle)]
/// Undoes the latest sprite edit.
///
/// # Safety
/// `handle` and `out_changed` must be valid writable pointers.
pub unsafe extern "C" fn mf_editor_sprite_undo(
    handle: *mut MfEditorHandle,
    out_changed: *mut u8,
    error: *mut MfError,
) -> MfStatus {
    sprite_history_action(handle, out_changed, error, EditorCore::sprite_undo)
}

#[unsafe(no_mangle)]
/// Redoes the latest reverted sprite edit.
///
/// # Safety
/// `handle` and `out_changed` must be valid writable pointers.
pub unsafe extern "C" fn mf_editor_sprite_redo(
    handle: *mut MfEditorHandle,
    out_changed: *mut u8,
    error: *mut MfError,
) -> MfStatus {
    sprite_history_action(handle, out_changed, error, EditorCore::sprite_redo)
}

#[unsafe(no_mangle)]
/// Saves the active sprite, overwriting its current path when available.
///
/// # Safety
/// `handle` must be valid, `fallback_name` must be a NUL-terminated UTF-8
/// string, and output pointers must reference writable storage.
pub unsafe extern "C" fn mf_editor_sprite_save(
    handle: *mut MfEditorHandle,
    fallback_name: *const c_char,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Ok(fallback_name) = read_cstr(fallback_name) else {
        set_error(
            error,
            MfStatus::InvalidArgument,
            "Sprite fallback name pointer is invalid",
        );
        return MfStatus::InvalidArgument;
    };
    match core.sprite_save_current(fallback_name) {
        Ok(path) => write_string_buffer(&path, data, capacity, required, error),
        Err(error_value) => set_core_error(error, error_value),
    }
}

#[unsafe(no_mangle)]
/// Renders an RGBA snapshot of the editor viewport into a caller buffer.
///
/// # Safety
/// `handle` must be a valid immutable editor handle. `data` must point to
/// `capacity` writable bytes unless probing for the required size. `out_info`
/// must be writable and `error`, when non-null, must be writable.
pub unsafe extern "C" fn mf_editor_viewport_snapshot_rgba(
    handle: *const MfEditorHandle,
    width: u32,
    height: u32,
    data: *mut u8,
    capacity: usize,
    out_info: *mut MfViewportInfo,
    error: *mut MfError,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_ref(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_info) = out_ptr(out_info, error, "out_info") else {
        return MfStatus::InvalidArgument;
    };
    match core.viewport_snapshot(width, height) {
        Ok(snapshot) => {
            let required = snapshot.rgba.len();
            *out_info = MfViewportInfo {
                abi_version: EDITOR_CORE_API_VERSION,
                struct_size: std::mem::size_of::<MfViewportInfo>(),
                width,
                height,
                required_bytes: required,
            };
            if data.is_null() || capacity < required {
                set_error(
                    error,
                    MfStatus::BufferTooSmall,
                    "Viewport RGBA buffer is too small",
                );
                return MfStatus::BufferTooSmall;
            }
            // SAFETY: caller provided a buffer with at least `required` bytes.
            unsafe {
                ptr::copy_nonoverlapping(snapshot.rgba.as_ptr(), data, required);
            }
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

impl Default for MfError {
    fn default() -> Self {
        Self {
            status: MfStatus::Ok,
            message: [0; MF_ERROR_MESSAGE_CAPACITY],
        }
    }
}

impl Default for MfEntityRow {
    fn default() -> Self {
        Self {
            abi_version: EDITOR_CORE_API_VERSION,
            struct_size: std::mem::size_of::<Self>(),
            id: 0,
            parent_id: 0,
            has_parent: 0,
            visible: 0,
            enabled: 0,
            locked: 0,
            selected: 0,
            component_count: 0,
            child_count: 0,
            x: 0.0,
            y: 0.0,
            name: [0; MF_NAME_CAPACITY],
            entity_type: [0; MF_SHORT_TEXT_CAPACITY],
            tag: [0; MF_SHORT_TEXT_CAPACITY],
            layer: [0; MF_SHORT_TEXT_CAPACITY],
        }
    }
}

impl From<crate::engine::editor_core::EntityRow> for MfEntityRow {
    fn from(row: crate::engine::editor_core::EntityRow) -> Self {
        let mut out = Self {
            id: row.id,
            parent_id: row.parent_id.unwrap_or_default(),
            has_parent: u8::from(row.parent_id.is_some()),
            visible: u8::from(row.visible),
            enabled: u8::from(row.enabled),
            locked: u8::from(row.locked),
            selected: u8::from(row.selected),
            component_count: row.component_count,
            child_count: row.child_count,
            x: row.x,
            y: row.y,
            ..Self::default()
        };
        write_fixed(&mut out.name, &row.name);
        write_fixed(&mut out.entity_type, &row.entity_type);
        write_fixed(&mut out.tag, &row.tag);
        write_fixed(&mut out.layer, &row.layer);
        out
    }
}

impl Default for MfInspectorField {
    fn default() -> Self {
        Self {
            abi_version: EDITOR_CORE_API_VERSION,
            struct_size: std::mem::size_of::<Self>(),
            entity_id: 0,
            editable: 0,
            target: [0; MF_SHORT_TEXT_CAPACITY],
            key: [0; MF_SHORT_TEXT_CAPACITY],
            display_name: [0; MF_NAME_CAPACITY],
            value_type: [0; MF_SHORT_TEXT_CAPACITY],
            value_json: [0; MF_VALUE_CAPACITY],
        }
    }
}

impl From<crate::engine::editor_core::InspectorFieldDto> for MfInspectorField {
    fn from(field: crate::engine::editor_core::InspectorFieldDto) -> Self {
        let mut out = Self {
            entity_id: field.entity_id,
            editable: u8::from(field.editable),
            ..Self::default()
        };
        write_fixed(&mut out.target, &field.target);
        write_fixed(&mut out.key, &field.key);
        write_fixed(&mut out.display_name, &field.display_name);
        write_fixed(&mut out.value_type, &field.value_type);
        write_fixed(&mut out.value_json, &field.value_json);
        out
    }
}

impl Default for MfAssetRow {
    fn default() -> Self {
        Self {
            abi_version: EDITOR_CORE_API_VERSION,
            struct_size: std::mem::size_of::<Self>(),
            size_bytes: 0,
            dependency_count: 0,
            guid: [0; MF_NAME_CAPACITY],
            relative_path: [0; MF_PATH_CAPACITY],
            name: [0; MF_NAME_CAPACITY],
            asset_type: [0; MF_SHORT_TEXT_CAPACITY],
            labels: [0; MF_VALUE_CAPACITY],
        }
    }
}

impl From<crate::engine::editor_core::AssetRow> for MfAssetRow {
    fn from(row: crate::engine::editor_core::AssetRow) -> Self {
        let mut out = Self {
            size_bytes: row.size_bytes,
            dependency_count: row.dependency_count,
            ..Self::default()
        };
        write_fixed(&mut out.guid, &row.guid);
        write_fixed(&mut out.relative_path, &row.relative_path);
        write_fixed(&mut out.name, &row.name);
        write_fixed(&mut out.asset_type, &row.asset_type);
        write_fixed(&mut out.labels, &row.labels.join(","));
        out
    }
}

impl Default for MfContentMutationResult {
    fn default() -> Self {
        Self {
            abi_version: EDITOR_CORE_API_VERSION,
            struct_size: std::mem::size_of::<Self>(),
            relative_path: [0; MF_PATH_CAPACITY],
        }
    }
}

impl Default for MfCommandDescriptor {
    fn default() -> Self {
        Self {
            abi_version: EDITOR_CORE_API_VERSION,
            struct_size: std::mem::size_of::<Self>(),
            enabled: 0,
            id: [0; MF_NAME_CAPACITY],
            label: [0; MF_NAME_CAPACITY],
            category: [0; MF_SHORT_TEXT_CAPACITY],
            shortcut: [0; MF_SHORT_TEXT_CAPACITY],
        }
    }
}

impl From<crate::engine::editor_core::CommandDescriptor> for MfCommandDescriptor {
    fn from(command: crate::engine::editor_core::CommandDescriptor) -> Self {
        let mut out = Self {
            enabled: u8::from(command.enabled),
            ..Self::default()
        };
        write_fixed(&mut out.id, &command.id);
        write_fixed(&mut out.label, &command.label);
        write_fixed(&mut out.category, &command.category);
        write_fixed(&mut out.shortcut, command.shortcut.as_deref().unwrap_or(""));
        out
    }
}

impl Default for MfConsoleEntry {
    fn default() -> Self {
        Self {
            abi_version: EDITOR_CORE_API_VERSION,
            struct_size: std::mem::size_of::<Self>(),
            frame: 0,
            severity: 0,
            channel: [0; MF_SHORT_TEXT_CAPACITY],
            message: [0; MF_VALUE_CAPACITY],
        }
    }
}

impl From<crate::engine::developer_console::ConsoleEntry> for MfConsoleEntry {
    fn from(entry: crate::engine::developer_console::ConsoleEntry) -> Self {
        let mut out = Self {
            frame: entry.frame,
            severity: match entry.severity {
                ConsoleSeverity::Debug => 0,
                ConsoleSeverity::Info => 1,
                ConsoleSeverity::Warning => 2,
                ConsoleSeverity::Error => 3,
            },
            ..Self::default()
        };
        write_fixed(&mut out.channel, &entry.channel);
        write_fixed(&mut out.message, &entry.message);
        out
    }
}

impl Default for MfReadinessRow {
    fn default() -> Self {
        Self {
            abi_version: EDITOR_CORE_API_VERSION,
            struct_size: std::mem::size_of::<Self>(),
            score: 0,
            level: 0,
            strength_count: 0,
            gap_count: 0,
            action_count: 0,
            system: [0; MF_SHORT_TEXT_CAPACITY],
            level_label: [0; MF_SHORT_TEXT_CAPACITY],
            top_action: [0; MF_VALUE_CAPACITY],
        }
    }
}

impl From<crate::engine::editor_core::ReadinessRow> for MfReadinessRow {
    fn from(row: crate::engine::editor_core::ReadinessRow) -> Self {
        let mut out = Self {
            score: row.score,
            level: readiness_level_code(row.level),
            strength_count: row.strength_count,
            gap_count: row.gap_count,
            action_count: row.action_count,
            ..Self::default()
        };
        write_fixed(&mut out.system, &row.system);
        write_fixed(&mut out.level_label, readiness_level_label(row.level));
        write_fixed(&mut out.top_action, &row.top_action);
        out
    }
}

fn readiness_level_code(level: SystemReadinessLevel) -> u32 {
    match level {
        SystemReadinessLevel::Ready => 0,
        SystemReadinessLevel::Watch => 1,
        SystemReadinessLevel::Weak => 2,
        SystemReadinessLevel::Blocked => 3,
    }
}

fn readiness_level_label(level: SystemReadinessLevel) -> &'static str {
    match level {
        SystemReadinessLevel::Ready => "Ready",
        SystemReadinessLevel::Watch => "Watch",
        SystemReadinessLevel::Weak => "Weak",
        SystemReadinessLevel::Blocked => "Blocked",
    }
}

fn sprite_history_action(
    handle: *mut MfEditorHandle,
    out_changed: *mut u8,
    error: *mut MfError,
    action: fn(&mut EditorCore) -> Result<bool, EditorCoreError>,
) -> MfStatus {
    clear_error(error);
    let Some(core) = core_mut(handle, error) else {
        return MfStatus::InvalidArgument;
    };
    let Some(out_changed) = out_ptr(out_changed, error, "out_changed") else {
        return MfStatus::InvalidArgument;
    };
    match action(core) {
        Ok(changed) => {
            *out_changed = u8::from(changed);
            MfStatus::Ok
        }
        Err(error_value) => set_core_error(error, error_value),
    }
}

fn core_ref<'a>(handle: *const MfEditorHandle, error: *mut MfError) -> Option<&'a EditorCore> {
    if handle.is_null() {
        set_error(error, MfStatus::InvalidArgument, "Editor handle is null");
        return None;
    }
    // SAFETY: null was checked and caller owns the handle lifetime.
    Some(unsafe { &(*handle).core })
}

fn core_mut<'a>(handle: *mut MfEditorHandle, error: *mut MfError) -> Option<&'a mut EditorCore> {
    if handle.is_null() {
        set_error(error, MfStatus::InvalidArgument, "Editor handle is null");
        return None;
    }
    // SAFETY: null was checked and C API requires exclusive mutable access per call.
    Some(unsafe { &mut (*handle).core })
}

fn out_ptr<'a, T>(ptr: *mut T, error: *mut MfError, name: &str) -> Option<&'a mut T> {
    if ptr.is_null() {
        set_error(
            error,
            MfStatus::InvalidArgument,
            format!("{name} pointer is null"),
        );
        return None;
    }
    // SAFETY: null was checked; caller provides writable storage.
    Some(unsafe { &mut *ptr })
}

fn read_cstr<'a>(value: *const c_char) -> Result<&'a str, ()> {
    if value.is_null() {
        return Err(());
    }
    // SAFETY: pointer is non-null and expected to reference a NUL-terminated C string.
    unsafe { CStr::from_ptr(value) }.to_str().map_err(|_| ())
}

fn status_from_result(result: Result<(), EditorCoreError>, error: *mut MfError) -> MfStatus {
    match result {
        Ok(()) => MfStatus::Ok,
        Err(error_value) => set_core_error(error, error_value),
    }
}

fn set_core_error(error: *mut MfError, error_value: EditorCoreError) -> MfStatus {
    let status = match error_value.kind {
        EditorCoreErrorKind::NoProjectOpen => MfStatus::NoProjectOpen,
        EditorCoreErrorKind::InvalidArgument | EditorCoreErrorKind::Serde => {
            MfStatus::InvalidArgument
        }
        EditorCoreErrorKind::NotFound => MfStatus::NotFound,
        EditorCoreErrorKind::Io | EditorCoreErrorKind::CommandFailed => MfStatus::Error,
    };
    set_error(error, status, error_value.message);
    status
}

fn clear_error(error: *mut MfError) {
    if error.is_null() {
        return;
    }
    // SAFETY: null was checked and caller supplied writable storage.
    unsafe {
        *error = MfError::default();
    }
}

fn set_error(error: *mut MfError, status: MfStatus, message: impl AsRef<str>) {
    if error.is_null() {
        return;
    }
    // SAFETY: null was checked and caller supplied writable storage.
    unsafe {
        (*error).status = status;
        write_fixed(&mut (*error).message, message.as_ref());
    }
}

fn write_serialized_result<T: serde::Serialize>(
    result: Result<T, EditorCoreError>,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    let value = match result {
        Ok(value) => value,
        Err(error_value) => return set_core_error(error, error_value),
    };
    let json = match serde_json::to_string(&value) {
        Ok(json) => json,
        Err(error_value) => {
            return set_core_error(error, EditorCoreError::from(error_value));
        }
    };
    write_string_buffer(&json, data, capacity, required, error)
}

fn write_string_buffer(
    value: &str,
    data: *mut c_char,
    capacity: usize,
    required: *mut usize,
    error: *mut MfError,
) -> MfStatus {
    let required_bytes = value.len() + 1;
    if !required.is_null() {
        // SAFETY: null was checked.
        unsafe {
            *required = required_bytes;
        }
    }
    if data.is_null() || capacity < required_bytes {
        set_error(
            error,
            MfStatus::BufferTooSmall,
            "String buffer is too small",
        );
        return MfStatus::BufferTooSmall;
    }
    // SAFETY: caller supplied a buffer with capacity >= required_bytes.
    unsafe {
        ptr::copy_nonoverlapping(value.as_ptr().cast::<c_char>(), data, value.len());
        *data.add(value.len()) = 0;
    }
    MfStatus::Ok
}

fn write_fixed<const N: usize>(target: &mut [c_char; N], value: &str) {
    target.fill(0);
    if N == 0 {
        return;
    }
    let bytes = value.as_bytes();
    let len = bytes.len().min(N.saturating_sub(1));
    for index in 0..len {
        target[index] = bytes[index] as c_char;
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn ffi_reports_no_project_before_open() {
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            assert!(!handle.is_null());

            let mut count = 0;
            let status = mf_editor_entity_count(handle, &mut count, &mut error);
            assert_eq!(status, MfStatus::NoProjectOpen);

            mf_editor_destroy(handle);
        }
    }

    #[test]
    fn ffi_opens_project_and_reads_rows() {
        let root = temp_project("ffi_rows");
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let path = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, path.as_ptr(), &mut error),
                MfStatus::Ok
            );

            let mut count = 0;
            assert_eq!(
                mf_editor_entity_count(handle, &mut count, &mut error),
                MfStatus::Ok
            );
            assert!(count >= 1);

            let mut row = MfEntityRow::default();
            assert_eq!(
                mf_editor_entity_row(handle, 0, &mut row, &mut error),
                MfStatus::Ok
            );
            assert!(row.id > 0);

            let mut entity_rows = vec![MfEntityRow::default(); count];
            let mut written = 0;
            let mut total = 0;
            assert_eq!(
                mf_editor_entity_rows(
                    handle,
                    0,
                    entity_rows.as_mut_ptr(),
                    entity_rows.len(),
                    &mut written,
                    &mut total,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(written, count);
            assert_eq!(total, count);
            assert_eq!(entity_rows[0].id, row.id);

            assert_eq!(
                mf_editor_select_entity(handle, row.id, &mut error),
                MfStatus::Ok
            );
            let mut selected_count = 0;
            assert_eq!(
                mf_editor_selected_entity_count(handle, &mut selected_count, &mut error),
                MfStatus::Ok
            );
            assert_eq!(selected_count, 1);
            let mut selected_id = 0;
            assert_eq!(
                mf_editor_selected_entity(handle, 0, &mut selected_id, &mut error),
                MfStatus::Ok
            );
            assert_eq!(selected_id, row.id);

            let toggle = CString::new("toggle").unwrap();
            assert_eq!(
                mf_editor_update_selection(handle, row.id, toggle.as_ptr(), &mut error),
                MfStatus::Ok
            );
            assert_eq!(
                mf_editor_selected_entity_count(handle, &mut selected_count, &mut error),
                MfStatus::Ok
            );
            assert_eq!(selected_count, 0);
            let replace = CString::new("replace").unwrap();
            assert_eq!(
                mf_editor_update_selection(handle, row.id, replace.as_ptr(), &mut error),
                MfStatus::Ok
            );

            let duplicate = CString::new("duplicate").unwrap();
            let empty_payload = CString::new("{}").unwrap();
            let mut duplicated_id = 0;
            assert_eq!(
                mf_editor_entity_action(
                    handle,
                    row.id,
                    duplicate.as_ptr(),
                    empty_payload.as_ptr(),
                    &mut duplicated_id,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_ne!(duplicated_id, 0);

            let create_prefab = CString::new("create_from_selected").unwrap();
            let mut prefab_action_json = vec![0 as c_char; 4096];
            let mut prefab_action_required = 0;
            assert_eq!(
                mf_editor_prefab_action_json(
                    handle,
                    create_prefab.as_ptr(),
                    empty_payload.as_ptr(),
                    prefab_action_json.as_mut_ptr(),
                    prefab_action_json.len(),
                    &mut prefab_action_required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let prefab_action: serde_json::Value = serde_json::from_str(
                CStr::from_ptr(prefab_action_json.as_ptr())
                    .to_str()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(prefab_action["changed"], true);

            let mut prefab_state_required = 0;
            assert_eq!(
                mf_editor_prefab_state_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut prefab_state_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            assert!(prefab_state_required > 3);

            let mut scene_required = 0;
            assert_eq!(
                mf_editor_scene_state_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut scene_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut scene_json = vec![0 as c_char; scene_required];
            assert_eq!(
                mf_editor_scene_state_json(
                    handle,
                    scene_json.as_mut_ptr(),
                    scene_json.len(),
                    &mut scene_required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let scene: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(scene_json.as_ptr()).to_str().unwrap())
                    .unwrap();
            assert_eq!(scene["dirty"], true);

            let mut catalog_required = 0;
            assert_eq!(
                mf_editor_component_catalog_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut catalog_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            assert!(catalog_required > 3);

            let mut authoring_required = 0;
            assert_eq!(
                mf_editor_authoring_catalog_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut authoring_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut authoring_json = vec![0 as c_char; authoring_required];
            assert_eq!(
                mf_editor_authoring_catalog_json(
                    handle,
                    authoring_json.as_mut_ptr(),
                    authoring_json.len(),
                    &mut authoring_required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let authoring: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(authoring_json.as_ptr()).to_str().unwrap())
                    .unwrap();
            assert!(authoring["presets"].as_array().unwrap().len() >= 40);
            assert!(authoring["kinds"]["physics"].as_u64().unwrap() >= 8);

            let preset_id = CString::new("topdown_player").unwrap();
            let preset_parameters = CString::new(r#"{"speed":8.5,"max_health":140}"#).unwrap();
            let mut plan_required = 0;
            assert_eq!(
                mf_editor_authoring_plan_json(
                    handle,
                    preset_id.as_ptr(),
                    preset_parameters.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    &mut plan_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut plan_json = vec![0 as c_char; plan_required];
            assert_eq!(
                mf_editor_authoring_plan_json(
                    handle,
                    preset_id.as_ptr(),
                    preset_parameters.as_ptr(),
                    plan_json.as_mut_ptr(),
                    plan_json.len(),
                    &mut plan_required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let plan: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(plan_json.as_ptr()).to_str().unwrap()).unwrap();
            assert_eq!(plan["preset_id"], "topdown_player");
            assert_eq!(plan["target_count"], 1);
            assert!(plan["total_components_to_add"].as_u64().unwrap() > 0);

            let mut sdk_catalog_required = 0;
            assert_eq!(
                mf_editor_sdk_pack_catalog_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut sdk_catalog_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut sdk_catalog_json = vec![0 as c_char; sdk_catalog_required];
            assert_eq!(
                mf_editor_sdk_pack_catalog_json(
                    handle,
                    sdk_catalog_json.as_mut_ptr(),
                    sdk_catalog_json.len(),
                    &mut sdk_catalog_required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let sdk_catalog: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(sdk_catalog_json.as_ptr()).to_str().unwrap())
                    .unwrap();
            assert_eq!(sdk_catalog["validation"]["valid"], true);

            let studio_profile = CString::new("studio-heavy").unwrap();
            let empty_registry = CString::new(r#"{"installed":[]}"#).unwrap();
            let mut sdk_plan_required = 0;
            assert_eq!(
                mf_editor_sdk_pack_plan_json(
                    handle,
                    studio_profile.as_ptr(),
                    empty_registry.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    &mut sdk_plan_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut sdk_plan_json = vec![0 as c_char; sdk_plan_required];
            assert_eq!(
                mf_editor_sdk_pack_plan_json(
                    handle,
                    studio_profile.as_ptr(),
                    empty_registry.as_ptr(),
                    sdk_plan_json.as_mut_ptr(),
                    sdk_plan_json.len(),
                    &mut sdk_plan_required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let sdk_plan: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(sdk_plan_json.as_ptr()).to_str().unwrap())
                    .unwrap();
            assert_eq!(sdk_plan["plan"]["meets_profile_target"], true);
            assert!(
                sdk_plan["plan"]["projected_installed_bytes"]
                    .as_u64()
                    .unwrap()
                    > 8_000_000_000
            );

            let mut inspector_count = 0;
            assert_eq!(
                mf_editor_inspector_field_count(handle, row.id, &mut inspector_count, &mut error),
                MfStatus::Ok
            );
            assert!(inspector_count > 0);
            let mut inspector_fields = vec![MfInspectorField::default(); inspector_count];
            assert_eq!(
                mf_editor_inspector_fields(
                    handle,
                    row.id,
                    0,
                    inspector_fields.as_mut_ptr(),
                    inspector_fields.len(),
                    &mut written,
                    &mut total,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(written, inspector_count);
            assert_eq!(total, inspector_count);
            assert_eq!(inspector_fields[0].entity_id, row.id);

            let mut command_count = 0;
            assert_eq!(
                mf_editor_command_count(handle, &mut command_count, &mut error),
                MfStatus::Ok
            );
            let mut command_rows = vec![MfCommandDescriptor::default(); command_count];
            assert_eq!(
                mf_editor_command_descriptors(
                    handle,
                    0,
                    command_rows.as_mut_ptr(),
                    command_rows.len(),
                    &mut written,
                    &mut total,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(written, command_count);
            assert_eq!(total, command_count);

            let mut readiness_score = 0;
            assert_eq!(
                mf_editor_readiness_score(handle, &mut readiness_score, &mut error),
                MfStatus::Ok
            );
            assert!(readiness_score <= 100);
            let mut readiness_count = 0;
            assert_eq!(
                mf_editor_readiness_count(handle, &mut readiness_count, &mut error),
                MfStatus::Ok
            );
            assert!(readiness_count > 0);
            let mut readiness_rows = vec![MfReadinessRow::default(); readiness_count];
            assert_eq!(
                mf_editor_readiness_rows(
                    handle,
                    0,
                    readiness_rows.as_mut_ptr(),
                    readiness_rows.len(),
                    &mut written,
                    &mut total,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(written, readiness_count);
            assert_eq!(total, readiness_count);
            assert!(readiness_rows[0].score <= 100);

            let mut health_required = 0;
            assert_eq!(
                mf_editor_runtime_health_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut health_required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut health_json = vec![0; health_required];
            assert_eq!(
                mf_editor_runtime_health_json(
                    handle,
                    health_json.as_mut_ptr(),
                    health_json.len(),
                    &mut health_required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let health: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(health_json.as_ptr()).to_str().unwrap())
                    .unwrap();
            assert_eq!(health["level"], "stable");
            assert!(health["entity_count"].as_u64().unwrap() >= 1);
            assert!(health["frame_budget_ms"].as_f64().unwrap() > 0.0);

            assert_eq!(
                mf_editor_asset_rows(
                    handle,
                    0,
                    std::ptr::null_mut(),
                    0,
                    &mut written,
                    &mut total,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(written, 0);

            let mut console_count = 0;
            assert_eq!(
                mf_editor_console_count(handle, &mut console_count, &mut error),
                MfStatus::Ok
            );
            assert!(console_count > 0);
            let mut console_entries = vec![MfConsoleEntry::default(); console_count];
            assert_eq!(
                mf_editor_console_entries(
                    handle,
                    0,
                    console_entries.as_mut_ptr(),
                    console_entries.len(),
                    &mut written,
                    &mut total,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(written, console_count);
            assert_eq!(total, console_count);

            let mut info = MfViewportInfo {
                abi_version: 0,
                struct_size: 0,
                width: 0,
                height: 0,
                required_bytes: 0,
            };
            let mut pixels = vec![0; 32 * 32 * 4];
            assert_eq!(
                mf_editor_viewport_snapshot_rgba(
                    handle,
                    32,
                    32,
                    pixels.as_mut_ptr(),
                    pixels.len(),
                    &mut info,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(info.required_bytes, pixels.len());
            assert!(pixels.iter().any(|value| *value != 0));

            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ffi_open_options_reject_typos_and_enable_safe_mode() {
        let root = temp_project("ffi_safe_mode");
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let path = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            let options = CString::new(
                r#"{"safe_mode":true,"safe_mode_reason":"ffi recovery","disable_asset_importers":true}"#,
            )
            .unwrap();
            assert_eq!(
                mf_editor_open_project_with_options(
                    handle,
                    path.as_ptr(),
                    options.as_ptr(),
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert!((*handle).core.runtime_health().unwrap().safe_mode_active);

            let invalid = CString::new(r#"{"safe_mod":true}"#).unwrap();
            assert_eq!(
                mf_editor_open_project_with_options(
                    handle,
                    path.as_ptr(),
                    invalid.as_ptr(),
                    &mut error,
                ),
                MfStatus::InvalidArgument
            );
            assert_eq!(mf_editor_is_project_open(handle), 1);
            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ffi_refreshes_and_exposes_forge_ai_reports_as_json() {
        let root = temp_project("ffi_forge_ai");
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let path = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, path.as_ptr(), &mut error),
                MfStatus::Ok
            );
            assert_eq!(mf_editor_refresh(handle, &mut error), MfStatus::Ok);

            let mut required = 0;
            assert_eq!(
                mf_editor_forge_ai_diagnostics_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            assert!(required > 2);
            let mut diagnostics = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_forge_ai_diagnostics_json(
                    handle,
                    diagnostics.as_mut_ptr(),
                    diagnostics.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let diagnostics_json = CStr::from_ptr(diagnostics.as_ptr()).to_str().unwrap();
            assert!(
                serde_json::from_str::<serde_json::Value>(diagnostics_json)
                    .unwrap()
                    .is_array()
            );

            let suite = CString::new("forge_ai_enemy_smoke").unwrap();
            required = 0;
            assert_eq!(
                mf_editor_forge_ai_run_test_json(
                    handle,
                    suite.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut report = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_forge_ai_run_test_json(
                    handle,
                    suite.as_ptr(),
                    report.as_mut_ptr(),
                    report.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let report_json = CStr::from_ptr(report.as_ptr()).to_str().unwrap();
            let report_value = serde_json::from_str::<serde_json::Value>(report_json).unwrap();
            assert_eq!(
                report_value["suite_id"],
                serde_json::Value::String("forge_ai_enemy_smoke".to_string())
            );

            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ffi_exposes_a_complete_luau_document_workflow() {
        let root = temp_project("ffi_luau_documents");
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let project = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, project.as_ptr(), &mut error),
                MfStatus::Ok
            );

            let path = CString::new("scripts/BridgeController.luau").unwrap();
            let source =
                CString::new("function on_update(dt)\n    move(100 * dt, 0)\nend\n").unwrap();
            let mut required = 0;
            assert_eq!(
                mf_editor_luau_validate_json(
                    handle,
                    path.as_ptr(),
                    source.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut validation = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_luau_validate_json(
                    handle,
                    path.as_ptr(),
                    source.as_ptr(),
                    validation.as_mut_ptr(),
                    validation.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let validation_json = CStr::from_ptr(validation.as_ptr()).to_str().unwrap();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(validation_json).unwrap()["valid"],
                serde_json::Value::Bool(true)
            );

            assert_eq!(
                mf_editor_luau_save(handle, path.as_ptr(), source.as_ptr(), &mut error),
                MfStatus::Ok
            );

            required = 0;
            assert_eq!(
                mf_editor_luau_scripts_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut scripts = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_luau_scripts_json(
                    handle,
                    scripts.as_mut_ptr(),
                    scripts.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let scripts_json = CStr::from_ptr(scripts.as_ptr()).to_str().unwrap();
            assert!(scripts_json.contains("scripts/BridgeController.luau"));

            required = 0;
            assert_eq!(
                mf_editor_luau_read(
                    handle,
                    path.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut contents = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_luau_read(
                    handle,
                    path.as_ptr(),
                    contents.as_mut_ptr(),
                    contents.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(
                CStr::from_ptr(contents.as_ptr()).to_str().unwrap(),
                source.to_str().unwrap()
            );

            let invalid_path = CString::new("../escape.luau").unwrap();
            assert_eq!(
                mf_editor_luau_read(
                    handle,
                    invalid_path.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::InvalidArgument
            );

            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ffi_exports_runtime_once_and_reads_the_cached_report() {
        let root = temp_project("ffi_runtime_export");
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let project = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, project.as_ptr(), &mut error),
                MfStatus::Ok
            );
            let profile = CString::new("debug").unwrap();
            assert_eq!(
                mf_editor_export_runtime(handle, profile.as_ptr(), &mut error),
                MfStatus::Ok
            );

            let mut required = 0;
            assert_eq!(
                mf_editor_last_export_report_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut report = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_last_export_report_json(
                    handle,
                    report.as_mut_ptr(),
                    report.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let report_json = CStr::from_ptr(report.as_ptr()).to_str().unwrap();
            let value = serde_json::from_str::<serde_json::Value>(report_json).unwrap();
            assert_eq!(value["profile"], serde_json::Value::String("Debug".into()));
            assert!(value["copied_files"].as_u64().unwrap_or_default() > 0);
            assert!(root.join("build/debug").exists());

            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ffi_round_trips_native_settings_and_project_launcher() {
        let root = temp_project("ffi_native_settings");
        let workspace = temp_project("ffi_native_launcher");
        fs::create_dir_all(&workspace).unwrap();
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let project = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, project.as_ptr(), &mut error),
                MfStatus::Ok
            );

            let mut required = 0;
            assert_eq!(
                mf_editor_project_settings_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut settings = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_project_settings_json(
                    handle,
                    settings.as_mut_ptr(),
                    settings.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let settings_json = CStr::from_ptr(settings.as_ptr()).to_str().unwrap();
            let mut value = serde_json::from_str::<serde_json::Value>(settings_json).unwrap();
            value["engine"]["autosave"] = serde_json::Value::Bool(false);
            let engine = CString::new(value["engine"].to_string()).unwrap();
            assert_eq!(
                mf_editor_save_engine_settings_json(handle, engine.as_ptr(), &mut error),
                MfStatus::Ok
            );
            value["input"]["bindings"]["Dash"] = serde_json::json!(["shift"]);
            let input = CString::new(value["input"].to_string()).unwrap();
            assert_eq!(
                mf_editor_save_input_map_json(handle, input.as_ptr(), &mut error),
                MfStatus::Ok
            );
            let tags_layers = CString::new(
                serde_json::json!({"tags":["Untagged","Boss"],"layers":["Default","Gameplay"]})
                    .to_string(),
            )
            .unwrap();
            assert_eq!(
                mf_editor_save_tags_layers_json(handle, tags_layers.as_ptr(), &mut error),
                MfStatus::Ok
            );

            let workspace_c = CString::new(workspace.to_string_lossy().as_bytes()).unwrap();
            let name = CString::new("QtNativeProject").unwrap();
            let template = CString::new("Empty").unwrap();
            let mut created = vec![0 as c_char; 4096];
            assert_eq!(
                mf_editor_launcher_create_project(
                    handle,
                    workspace_c.as_ptr(),
                    workspace_c.as_ptr(),
                    name.as_ptr(),
                    template.as_ptr(),
                    created.as_mut_ptr(),
                    created.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let created_path = CStr::from_ptr(created.as_ptr()).to_str().unwrap();
            assert!(
                std::path::Path::new(created_path)
                    .join("project.json")
                    .exists()
            );

            required = 0;
            assert_eq!(
                mf_editor_launcher_snapshot_json(
                    handle,
                    workspace_c.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut launcher = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_launcher_snapshot_json(
                    handle,
                    workspace_c.as_ptr(),
                    launcher.as_mut_ptr(),
                    launcher.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let launcher_json = CStr::from_ptr(launcher.as_ptr()).to_str().unwrap();
            assert!(launcher_json.contains("QtNativeProject"));

            let created_c = CString::new(created_path).unwrap();
            let mut repair = vec![0 as c_char; 65_536];
            assert_eq!(
                mf_editor_launcher_repair_project_json(
                    handle,
                    workspace_c.as_ptr(),
                    created_c.as_ptr(),
                    repair.as_mut_ptr(),
                    repair.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let repair_json = CStr::from_ptr(repair.as_ptr()).to_str().unwrap();
            assert!(
                serde_json::from_str::<serde_json::Value>(repair_json).unwrap()["notes"]
                    .as_array()
                    .is_some_and(|notes| !notes.is_empty())
            );

            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn ffi_exposes_safe_asset_operations_profiler_and_dependency_graph() {
        let root = temp_project("ffi_asset_tools");
        let external_root = temp_project("ffi_asset_external");
        crate::engine::asset_tools::AssetTools::ensure_project_folders(&root).unwrap();
        fs::create_dir_all(&external_root).unwrap();
        fs::write(external_root.join("icon.png"), b"ffi-import").unwrap();
        fs::write(
            root.join("assets/data/a.json"),
            br#"{"dependency":"assets/data/b.json"}"#,
        )
        .unwrap();
        fs::write(
            root.join("assets/data/b.json"),
            br#"{"dependency":"assets/data/a.json"}"#,
        )
        .unwrap();

        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let project = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, project.as_ptr(), &mut error),
                MfStatus::Ok
            );

            let action = CString::new("import").unwrap();
            let payload = CString::new(
                serde_json::json!({
                    "source_external": external_root.join("icon.png"),
                    "target_folder": "assets/sprites"
                })
                .to_string(),
            )
            .unwrap();
            assert_eq!(
                mf_editor_manage_asset(handle, action.as_ptr(), payload.as_ptr(), &mut error),
                MfStatus::Ok
            );
            assert!(root.join("assets/sprites/icon.png").is_file());
            assert!(root.join("assets/sprites/icon.png.import.json").is_file());

            let mut required = 0;
            assert_eq!(
                mf_editor_profiler_snapshot_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut profiler = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_profiler_snapshot_json(
                    handle,
                    profiler.as_mut_ptr(),
                    profiler.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let profiler_json = CStr::from_ptr(profiler.as_ptr()).to_str().unwrap();
            assert!(
                serde_json::from_str::<serde_json::Value>(profiler_json).unwrap()
                    ["frame_budget_ms"]
                    .as_f64()
                    .is_some_and(|budget| budget > 0.0)
            );

            assert_eq!(
                mf_editor_rebuild_asset_dependencies(handle, &mut error),
                MfStatus::Ok
            );
            required = 0;
            assert_eq!(
                mf_editor_asset_dependency_graph_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut graph = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_asset_dependency_graph_json(
                    handle,
                    graph.as_mut_ptr(),
                    graph.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let graph_json = CStr::from_ptr(graph.as_ptr()).to_str().unwrap();
            let graph_value = serde_json::from_str::<serde_json::Value>(graph_json).unwrap();
            assert!(graph_value["edge_count"].as_u64().unwrap_or_default() >= 2);
            assert!(
                graph_value["cycles"]
                    .as_array()
                    .is_some_and(|cycles| !cycles.is_empty())
            );

            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(external_root);
    }

    #[test]
    fn ffi_project_operations_are_single_shot_and_publish_structured_state() {
        let root = temp_project("ffi_project_operations");
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let project = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, project.as_ptr(), &mut error),
                MfStatus::Ok
            );
            let autosave = CString::new("autosave_now").unwrap();
            let payload = CString::new("{}").unwrap();
            assert_eq!(
                mf_editor_project_operation(
                    handle,
                    autosave.as_ptr(),
                    payload.as_ptr(),
                    &mut error,
                ),
                MfStatus::Ok
            );

            let package = CString::new("package_export").unwrap();
            assert_eq!(
                mf_editor_project_operation(handle, package.as_ptr(), payload.as_ptr(), &mut error,),
                MfStatus::Ok
            );
            let mut required = 0;
            assert_eq!(
                mf_editor_project_operations_json(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut state = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_project_operations_json(
                    handle,
                    state.as_mut_ptr(),
                    state.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let state_json = CStr::from_ptr(state.as_ptr()).to_str().unwrap();
            let value = serde_json::from_str::<serde_json::Value>(state_json).unwrap();
            assert_eq!(value["autosave"]["exists"], true);
            assert_eq!(value["last_operation"]["action"], "package_export");
            assert!(
                Path::new(value["last_operation"]["artifact_path"].as_str().unwrap()).is_file()
            );

            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ffi_exposes_inspector_quick_actions_and_atomic_selection_actions() {
        let root = temp_project("ffi_inspector_quick_batch");
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts/FfiInspector.luau"), "return {}\n").unwrap();
        unsafe {
            let mut error = MfError::default();
            let handle = mf_editor_create(&mut error);
            let project = CString::new(root.to_string_lossy().as_bytes()).unwrap();
            assert_eq!(
                mf_editor_open_project(handle, project.as_ptr(), &mut error),
                MfStatus::Ok
            );
            let entity_id = (*handle)
                .core
                .forge_ai_create_entity("FfiInspectorEntity", 0.0, 0.0)
                .unwrap();
            assert_eq!(
                mf_editor_select_entity(handle, entity_id, &mut error),
                MfStatus::Ok
            );
            let mut required = 0;
            assert_eq!(
                mf_editor_inspector_quick_actions_json(
                    handle,
                    entity_id,
                    std::ptr::null_mut(),
                    0,
                    &mut required,
                    &mut error,
                ),
                MfStatus::BufferTooSmall
            );
            let mut actions = vec![0 as c_char; required];
            assert_eq!(
                mf_editor_inspector_quick_actions_json(
                    handle,
                    entity_id,
                    actions.as_mut_ptr(),
                    actions.len(),
                    &mut required,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let actions_json = CStr::from_ptr(actions.as_ptr()).to_str().unwrap();
            assert!(actions_json.contains("attach_script"));
            assert!(actions_json.contains("scripts/FfiInspector.luau"));
            let attach = CString::new("attach_script").unwrap();
            let script = CString::new("scripts/FfiInspector.luau").unwrap();
            assert_eq!(
                mf_editor_inspector_quick_action(
                    handle,
                    entity_id,
                    attach.as_ptr(),
                    script.as_ptr(),
                    &mut error,
                ),
                MfStatus::Ok
            );

            let duplicate = CString::new("duplicate").unwrap();
            let payload = CString::new("{}").unwrap();
            let mut duplicate_id = 0;
            assert_eq!(
                mf_editor_entity_action(
                    handle,
                    entity_id,
                    duplicate.as_ptr(),
                    payload.as_ptr(),
                    &mut duplicate_id,
                    &mut error,
                ),
                MfStatus::Ok
            );
            let replace = CString::new("replace").unwrap();
            let add = CString::new("add").unwrap();
            assert_eq!(
                mf_editor_update_selection(handle, entity_id, replace.as_ptr(), &mut error),
                MfStatus::Ok
            );
            assert_eq!(
                mf_editor_update_selection(handle, duplicate_id, add.as_ptr(), &mut error),
                MfStatus::Ok
            );
            let add_component = CString::new("add_component").unwrap();
            let component_payload = CString::new(r#"{"component_type":"Material2D"}"#).unwrap();
            let mut changed = 0;
            assert_eq!(
                mf_editor_selected_entity_action(
                    handle,
                    add_component.as_ptr(),
                    component_payload.as_ptr(),
                    &mut changed,
                    &mut error,
                ),
                MfStatus::Ok
            );
            assert_eq!(changed, 2);
            let undo = CString::new("edit.undo").unwrap();
            assert_eq!(
                mf_editor_execute_command(handle, undo.as_ptr(), &mut error),
                MfStatus::Ok
            );
            for id in [entity_id, duplicate_id] {
                assert!(
                    (*handle)
                        .core
                        .inspector_fields(id)
                        .unwrap()
                        .iter()
                        .all(|field| field.target != "Material2D")
                );
            }
            mf_editor_destroy(handle);
        }
        let _ = fs::remove_dir_all(root);
    }

    fn temp_project(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("miniforge_editor_ffi_{name}_{stamp}"))
    }
}
