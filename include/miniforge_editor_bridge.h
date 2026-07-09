#ifndef MINIFORGE_EDITOR_BRIDGE_H
#define MINIFORGE_EDITOR_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#  define MINIFORGE_EDITOR_EXPORT __declspec(dllexport)
#else
#  define MINIFORGE_EDITOR_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define MF_EDITOR_CORE_API_VERSION 1u
#define MF_ERROR_MESSAGE_CAPACITY 1024u
#define MF_NAME_CAPACITY 128u
#define MF_SHORT_TEXT_CAPACITY 64u
#define MF_PATH_CAPACITY 512u
#define MF_VALUE_CAPACITY 512u

typedef uint64_t MfEntityId;
typedef uint64_t MfAssetId;
typedef uint64_t MfSceneId;
typedef uint64_t MfCommandId;

typedef enum MfStatus {
    MF_STATUS_OK = 0,
    MF_STATUS_ERROR = 1,
    MF_STATUS_NOT_FOUND = 2,
    MF_STATUS_INVALID_ARGUMENT = 3,
    MF_STATUS_NO_PROJECT_OPEN = 4,
    MF_STATUS_BUFFER_TOO_SMALL = 5
} MfStatus;

typedef struct MfError {
    MfStatus status;
    char message[MF_ERROR_MESSAGE_CAPACITY];
} MfError;

typedef struct MfEditorHandle MfEditorHandle;

typedef struct MfEntityRow {
    uint32_t abi_version;
    size_t struct_size;
    MfEntityId id;
    MfEntityId parent_id;
    uint8_t has_parent;
    uint8_t visible;
    uint8_t enabled;
    uint8_t locked;
    uint8_t selected;
    size_t component_count;
    size_t child_count;
    double x;
    double y;
    char name[MF_NAME_CAPACITY];
    char entity_type[MF_SHORT_TEXT_CAPACITY];
    char tag[MF_SHORT_TEXT_CAPACITY];
    char layer[MF_SHORT_TEXT_CAPACITY];
} MfEntityRow;

typedef struct MfInspectorField {
    uint32_t abi_version;
    size_t struct_size;
    MfEntityId entity_id;
    uint8_t editable;
    char target[MF_SHORT_TEXT_CAPACITY];
    char key[MF_SHORT_TEXT_CAPACITY];
    char display_name[MF_NAME_CAPACITY];
    char value_type[MF_SHORT_TEXT_CAPACITY];
    char value_json[MF_VALUE_CAPACITY];
} MfInspectorField;

typedef struct MfAssetRow {
    uint32_t abi_version;
    size_t struct_size;
    uint64_t size_bytes;
    size_t dependency_count;
    char guid[MF_NAME_CAPACITY];
    char relative_path[MF_PATH_CAPACITY];
    char name[MF_NAME_CAPACITY];
    char asset_type[MF_SHORT_TEXT_CAPACITY];
    char labels[MF_VALUE_CAPACITY];
} MfAssetRow;

typedef struct MfCommandDescriptor {
    uint32_t abi_version;
    size_t struct_size;
    uint8_t enabled;
    char id[MF_NAME_CAPACITY];
    char label[MF_NAME_CAPACITY];
    char category[MF_SHORT_TEXT_CAPACITY];
    char shortcut[MF_SHORT_TEXT_CAPACITY];
} MfCommandDescriptor;

typedef struct MfConsoleEntry {
    uint32_t abi_version;
    size_t struct_size;
    uint64_t frame;
    uint32_t severity;
    char channel[MF_SHORT_TEXT_CAPACITY];
    char message[MF_VALUE_CAPACITY];
} MfConsoleEntry;

typedef struct MfReadinessRow {
    uint32_t abi_version;
    size_t struct_size;
    uint8_t score;
    uint32_t level;
    size_t strength_count;
    size_t gap_count;
    size_t action_count;
    char system[MF_SHORT_TEXT_CAPACITY];
    char level_label[MF_SHORT_TEXT_CAPACITY];
    char top_action[MF_VALUE_CAPACITY];
} MfReadinessRow;

typedef struct MfViewportInfo {
    uint32_t abi_version;
    size_t struct_size;
    uint32_t width;
    uint32_t height;
    size_t required_bytes;
} MfViewportInfo;

MINIFORGE_EDITOR_EXPORT MfEditorHandle* mf_editor_create(MfError* error);
MINIFORGE_EDITOR_EXPORT void mf_editor_destroy(MfEditorHandle* editor);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_open_project(MfEditorHandle* editor, const char* path, MfError* error);
MINIFORGE_EDITOR_EXPORT uint8_t mf_editor_is_project_open(const MfEditorHandle* editor);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_project_path(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_entity_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_entity_row(const MfEditorHandle* editor, size_t index, MfEntityRow* out_row, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_entity_rows(const MfEditorHandle* editor, size_t start_index, MfEntityRow* rows, size_t row_capacity, size_t* out_written, size_t* out_total, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_selected_entity_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_selected_entity(const MfEditorHandle* editor, size_t index, MfEntityId* out_entity_id, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_select_entity(MfEditorHandle* editor, MfEntityId entity_id, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_field_count(const MfEditorHandle* editor, MfEntityId entity_id, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_field(const MfEditorHandle* editor, MfEntityId entity_id, size_t index, MfInspectorField* out_field, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_fields(const MfEditorHandle* editor, MfEntityId entity_id, size_t start_index, MfInspectorField* fields, size_t field_capacity, size_t* out_written, size_t* out_total, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_set_inspector_value_json(MfEditorHandle* editor, MfEntityId entity_id, const char* target, const char* key, const char* value_json, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_asset_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_asset_row(const MfEditorHandle* editor, size_t index, MfAssetRow* out_row, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_asset_rows(const MfEditorHandle* editor, size_t start_index, MfAssetRow* rows, size_t row_capacity, size_t* out_written, size_t* out_total, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_command_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_command_descriptor(const MfEditorHandle* editor, size_t index, MfCommandDescriptor* out_command, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_command_descriptors(const MfEditorHandle* editor, size_t start_index, MfCommandDescriptor* commands, size_t command_capacity, size_t* out_written, size_t* out_total, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_execute_command(MfEditorHandle* editor, const char* command_id, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_console_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_console_entry(const MfEditorHandle* editor, size_t index, MfConsoleEntry* out_entry, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_console_entries(const MfEditorHandle* editor, size_t start_index, MfConsoleEntry* entries, size_t entry_capacity, size_t* out_written, size_t* out_total, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_score(const MfEditorHandle* editor, uint8_t* out_score, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_row(const MfEditorHandle* editor, size_t index, MfReadinessRow* out_row, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_rows(const MfEditorHandle* editor, size_t start_index, MfReadinessRow* rows, size_t row_capacity, size_t* out_written, size_t* out_total, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_viewport_snapshot_rgba(const MfEditorHandle* editor, uint32_t width, uint32_t height, uint8_t* data, size_t capacity, MfViewportInfo* out_info, MfError* error);

#ifdef __cplusplus
}
#endif

#endif
