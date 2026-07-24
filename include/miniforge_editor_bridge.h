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

typedef struct MfContentMutationResult {
    uint32_t abi_version;
    size_t struct_size;
    char relative_path[MF_PATH_CAPACITY];
} MfContentMutationResult;

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

typedef struct MfSpriteInfo {
    uint32_t abi_version;
    size_t struct_size;
    uint32_t width;
    uint32_t height;
    size_t required_bytes;
    uint8_t can_undo;
    uint8_t can_redo;
} MfSpriteInfo;

MINIFORGE_EDITOR_EXPORT MfEditorHandle* mf_editor_create(MfError* error);
MINIFORGE_EDITOR_EXPORT void mf_editor_destroy(MfEditorHandle* editor);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_open_project(MfEditorHandle* editor, const char* path, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_open_project_with_options(MfEditorHandle* editor, const char* path, const char* options_json, MfError* error);
MINIFORGE_EDITOR_EXPORT uint8_t mf_editor_is_project_open(const MfEditorHandle* editor);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_refresh(MfEditorHandle* editor, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_project_path(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_project_settings_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_save_engine_settings_json(MfEditorHandle* editor, const char* source, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_save_input_map_json(MfEditorHandle* editor, const char* source, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_save_tags_layers_json(MfEditorHandle* editor, const char* source, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_launcher_snapshot_json(const MfEditorHandle* editor, const char* workspace_root, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_launcher_create_project(const MfEditorHandle* editor, const char* workspace_root, const char* location, const char* name, const char* template_name, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_launcher_repair_project_json(const MfEditorHandle* editor, const char* workspace_root, const char* project_path, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_project_operations_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_project_operation(MfEditorHandle* editor, const char* action, const char* payload_json, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_entity_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_entity_row(const MfEditorHandle* editor, size_t index, MfEntityRow* out_row, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_entity_rows(const MfEditorHandle* editor, size_t start_index, MfEntityRow* rows, size_t row_capacity, size_t* out_written, size_t* out_total, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_selected_entity_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_selected_entity(const MfEditorHandle* editor, size_t index, MfEntityId* out_entity_id, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_select_entity(MfEditorHandle* editor, MfEntityId entity_id, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_update_selection(MfEditorHandle* editor, MfEntityId entity_id, const char* mode, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_clear_selection(MfEditorHandle* editor, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_entity_action(MfEditorHandle* editor, MfEntityId entity_id, const char* action, const char* payload_json, MfEntityId* out_entity_id, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_selected_entity_action(MfEditorHandle* editor, const char* action, const char* payload_json, size_t* out_changed, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_pick_entity(MfEditorHandle* editor, uint32_t viewport_width, uint32_t viewport_height, double x, double y, const char* selection_mode, MfEntityId* out_entity_id, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_viewport_state_json(const MfEditorHandle* editor, uint32_t viewport_width, uint32_t viewport_height, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_transform_selection(MfEditorHandle* editor, const char* payload_json, size_t* out_changed, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_scene_state_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_scene_browser_state_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_scene_browser_action_json(MfEditorHandle* editor, const char* action, const char* payload_json, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_component_catalog_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_authoring_catalog_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_tool_state_json(const MfEditorHandle* editor, const char* tool, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_tool_action_json(MfEditorHandle* editor, const char* tool, const char* action, const char* payload_json, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_prefab_state_json(MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_prefab_action_json(MfEditorHandle* editor, const char* action, const char* payload_json, char* data, size_t capacity, size_t* required, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_field_count(const MfEditorHandle* editor, MfEntityId entity_id, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_field(const MfEditorHandle* editor, MfEntityId entity_id, size_t index, MfInspectorField* out_field, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_fields(const MfEditorHandle* editor, MfEntityId entity_id, size_t start_index, MfInspectorField* fields, size_t field_capacity, size_t* out_written, size_t* out_total, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_quick_actions_json(const MfEditorHandle* editor, MfEntityId entity_id, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_inspector_quick_action(MfEditorHandle* editor, MfEntityId entity_id, const char* action, const char* asset_path, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_set_inspector_value_json(MfEditorHandle* editor, MfEntityId entity_id, const char* target, const char* key, const char* value_json, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_set_selected_inspector_value_json(MfEditorHandle* editor, const char* target, const char* key, const char* value_json, size_t* out_changed, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_asset_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_asset_row(const MfEditorHandle* editor, size_t index, MfAssetRow* out_row, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_asset_rows(const MfEditorHandle* editor, size_t start_index, MfAssetRow* rows, size_t row_capacity, size_t* out_written, size_t* out_total, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_manage_asset(MfEditorHandle* editor, const char* action, const char* payload_json, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_content_folders_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_content_entries_json(const MfEditorHandle* editor, const char* relative_directory, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_content_create_folder(MfEditorHandle* editor, const char* relative_directory, const char* name, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_content_create_file(MfEditorHandle* editor, const char* kind, const char* relative_directory, const char* name, MfContentMutationResult* out_result, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_content_read_text(const MfEditorHandle* editor, const char* relative_path, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_content_save_text(MfEditorHandle* editor, const char* relative_path, const char* source, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_profiler_snapshot_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_rebuild_asset_dependencies(MfEditorHandle* editor, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_asset_dependency_graph_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_command_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_command_descriptor(const MfEditorHandle* editor, size_t index, MfCommandDescriptor* out_command, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_command_descriptors(const MfEditorHandle* editor, size_t start_index, MfCommandDescriptor* commands, size_t command_capacity, size_t* out_written, size_t* out_total, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_execute_command(MfEditorHandle* editor, const char* command_id, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_scripts_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_read(const MfEditorHandle* editor, const char* relative_path, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_validate_json(const MfEditorHandle* editor, const char* relative_path, const char* source, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_save(MfEditorHandle* editor, const char* relative_path, const char* source, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_debug_state_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_set_breakpoints_json(MfEditorHandle* editor, const char* breakpoints_json, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_debug_command(MfEditorHandle* editor, const char* command, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_luau_watches_json(const MfEditorHandle* editor, const char* expressions_json, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_visual_graph_validate_json(const MfEditorHandle* editor, const char* relative_path, const char* source, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_visual_graph_save(MfEditorHandle* editor, const char* relative_path, const char* source, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_visual_graph_catalog_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_visual_graph_create_template(MfEditorHandle* editor, const char* relative_path, const char* template_name, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_python_tools_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_python_install_tools(MfEditorHandle* editor, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_python_run_tool(MfEditorHandle* editor, const char* tool_id, const char* parameters_json, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_python_last_result_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_export_runtime(MfEditorHandle* editor, const char* profile, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_last_export_report_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_runtime_health_json(const MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_forge_ai_diagnostics_json(MfEditorHandle* editor, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_forge_ai_run_test_json(MfEditorHandle* editor, const char* suite_id, char* data, size_t capacity, size_t* required, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_console_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_console_entry(const MfEditorHandle* editor, size_t index, MfConsoleEntry* out_entry, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_console_entries(const MfEditorHandle* editor, size_t start_index, MfConsoleEntry* entries, size_t entry_capacity, size_t* out_written, size_t* out_total, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_score(const MfEditorHandle* editor, uint8_t* out_score, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_count(const MfEditorHandle* editor, size_t* out_count, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_row(const MfEditorHandle* editor, size_t index, MfReadinessRow* out_row, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_readiness_rows(const MfEditorHandle* editor, size_t start_index, MfReadinessRow* rows, size_t row_capacity, size_t* out_written, size_t* out_total, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_snapshot_rgba(const MfEditorHandle* editor, uint8_t* data, size_t capacity, MfSpriteInfo* out_info, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_new_canvas(MfEditorHandle* editor, uint32_t width, uint32_t height, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_begin_edit(MfEditorHandle* editor, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_set_pixel(MfEditorHandle* editor, uint32_t x, uint32_t y, uint8_t red, uint8_t green, uint8_t blue, uint8_t alpha, uint8_t* out_changed, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_clear(MfEditorHandle* editor, uint8_t red, uint8_t green, uint8_t blue, uint8_t alpha, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_transform(MfEditorHandle* editor, const char* action, const char* payload_json, uint8_t* out_changed, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_animation_clip_json(const MfEditorHandle* editor, uint32_t frame_width, uint32_t frame_height, float fps, char* data, size_t capacity, size_t* required, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_commit_edit(MfEditorHandle* editor, uint8_t* out_changed, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_undo(MfEditorHandle* editor, uint8_t* out_changed, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_redo(MfEditorHandle* editor, uint8_t* out_changed, MfError* error);
MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_sprite_save(MfEditorHandle* editor, const char* fallback_name, char* data, size_t capacity, size_t* required, MfError* error);

MINIFORGE_EDITOR_EXPORT MfStatus mf_editor_viewport_snapshot_rgba(const MfEditorHandle* editor, uint32_t width, uint32_t height, uint8_t* data, size_t capacity, MfViewportInfo* out_info, MfError* error);

#ifdef __cplusplus
}
#endif

#endif
