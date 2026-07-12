use std::ffi::{CStr, c_char};
use std::ptr;

use crate::engine::developer_console::ConsoleSeverity;
use crate::engine::editor_core::{
    EDITOR_CORE_API_VERSION, EditorCore, EditorCoreError, EditorCoreErrorKind,
};
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

    fn temp_project(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("miniforge_editor_ffi_{name}_{stamp}"))
    }
}
