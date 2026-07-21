#include "miniforge_editor_bridge.h"

#include <array>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>

namespace {

std::string fixedString(const char* data, std::size_t capacity)
{
    return std::string(data, strnlen(data, capacity));
}

std::string errorMessage(const MfError& error)
{
    return fixedString(error.message, MF_ERROR_MESSAGE_CAPACITY);
}

void expect(bool condition, const std::string& message)
{
    if (!condition) {
        std::cerr << "bridge smoke failed: " << message << '\n';
        std::exit(1);
    }
}

void expectStatus(MfStatus actual, MfStatus expected, const MfError& error, const std::string& context)
{
    if (actual != expected) {
        std::cerr << "bridge smoke failed: " << context << " expected status " << expected
                  << " but got " << actual << ": " << errorMessage(error) << '\n';
        std::exit(1);
    }
}

std::filesystem::path makeTempProject()
{
    const auto stamp = std::chrono::steady_clock::now().time_since_epoch().count();
    auto path = std::filesystem::temp_directory_path() / ("miniforge_bridge_smoke_" + std::to_string(stamp));
    std::filesystem::create_directories(path);
    return path;
}

} // namespace

int main(int argc, char** argv)
{
    expect(argc <= 2, "expected zero or one project path argument");
    const bool useExternalProject = argc == 2 && std::getenv("MINIFORGE_BRIDGE_SMOKE_EXTERNAL") != nullptr;
    const std::filesystem::path tempProject = useExternalProject ? std::filesystem::path {} : makeTempProject();
    const std::string projectPath = useExternalProject ? std::string(argv[1]) : tempProject.string();
    if (!useExternalProject) {
        std::filesystem::create_directories(tempProject / "assets/data");
        std::ofstream(tempProject / "assets/data/ManagedSmoke.json") << "{\"value\":1}";
    }

    MfError error {};
    MfEditorHandle* editor = mf_editor_create(&error);
    expect(editor != nullptr, "mf_editor_create returned null: " + errorMessage(error));
    expect(mf_editor_is_project_open(editor) == 0, "new editor should not have an open project");

    std::size_t count = 0;
    expectStatus(mf_editor_entity_count(editor, &count, &error), MF_STATUS_NO_PROJECT_OPEN, error, "entity count before open");
    expect(!errorMessage(error).empty(), "no-project error should include a message");

    std::size_t required = 0;
    expectStatus(
        mf_editor_project_path(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "project path before open with null buffer"
    );
    expect(required == 1, "empty project path should require one byte for NUL");

    expectStatus(mf_editor_open_project(editor, projectPath.c_str(), &error), MF_STATUS_OK, error, "open project");
    expect(mf_editor_is_project_open(editor) == 1, "project should be open after mf_editor_open_project");

    std::array<char, MF_PATH_CAPACITY> pathBuffer {};
    required = 0;
    expectStatus(
        mf_editor_project_path(editor, pathBuffer.data(), pathBuffer.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read project path"
    );
    expect(!fixedString(pathBuffer.data(), pathBuffer.size()).empty(), "project path should be readable after open");

    expectStatus(mf_editor_entity_count(editor, &count, &error), MF_STATUS_OK, error, "entity count");
    expect(count > 0, "DefaultProject should expose at least one entity");

    std::vector<MfEntityRow> entityRows(count);
    std::size_t written = 0;
    std::size_t total = 0;
    expectStatus(
        mf_editor_entity_rows(editor, 0, entityRows.data(), entityRows.size(), &written, &total, &error),
        MF_STATUS_OK,
        error,
        "entity batch rows"
    );
    expect(written == count && total == count, "entity batch should write the full entity set");
    expect(entityRows.front().abi_version == MF_EDITOR_CORE_API_VERSION, "entity row ABI version mismatch");
    expect(entityRows.front().struct_size == sizeof(MfEntityRow), "entity row struct size mismatch");
    expect(entityRows.front().id != 0, "first entity id should be non-zero");
    expect(!fixedString(entityRows.front().name, MF_NAME_CAPACITY).empty(), "first entity should have a name");

    expectStatus(mf_editor_select_entity(editor, entityRows.front().id, &error), MF_STATUS_OK, error, "select first entity");
    std::size_t selectedCount = 0;
    expectStatus(mf_editor_selected_entity_count(editor, &selectedCount, &error), MF_STATUS_OK, error, "selected entity count");
    expect(selectedCount == 1, "selection should contain exactly one entity");
    MfEntityId selectedId = 0;
    expectStatus(mf_editor_selected_entity(editor, 0, &selectedId, &error), MF_STATUS_OK, error, "selected entity id");
    expect(selectedId == entityRows.front().id, "selected entity id should match selected row");

    std::size_t inspectorCount = 0;
    expectStatus(
        mf_editor_inspector_field_count(editor, selectedId, &inspectorCount, &error),
        MF_STATUS_OK,
        error,
        "inspector field count"
    );
    expect(inspectorCount > 0, "selected entity should expose inspector fields");
    std::vector<MfInspectorField> fields(inspectorCount);
    expectStatus(
        mf_editor_inspector_fields(editor, selectedId, 0, fields.data(), fields.size(), &written, &total, &error),
        MF_STATUS_OK,
        error,
        "inspector fields"
    );
    expect(written == inspectorCount && total == inspectorCount, "inspector batch should write the full field set");
    expect(fields.front().abi_version == MF_EDITOR_CORE_API_VERSION, "inspector field ABI version mismatch");
    expect(fields.front().entity_id == selectedId, "inspector field should belong to selected entity");

    MfInspectorField* editableString = nullptr;
    for (MfInspectorField& field : fields) {
        if (field.editable != 0 && fixedString(field.value_type, MF_SHORT_TEXT_CAPACITY) == "string") {
            editableString = &field;
            break;
        }
    }
    expect(editableString != nullptr, "selected entity should expose an editable string field");
    const std::string editTarget = fixedString(editableString->target, MF_SHORT_TEXT_CAPACITY);
    const std::string editKey = fixedString(editableString->key, MF_SHORT_TEXT_CAPACITY);
    expectStatus(
        mf_editor_set_inspector_value_json(editor, selectedId, editTarget.c_str(), editKey.c_str(), "\"BridgeSmokeName\"", &error),
        MF_STATUS_OK,
        error,
        "edit inspector value"
    );
    expectStatus(
        mf_editor_inspector_fields(editor, selectedId, 0, fields.data(), fields.size(), &written, &total, &error),
        MF_STATUS_OK,
        error,
        "inspector fields after edit"
    );
    bool sawEditedValue = false;
    for (const MfInspectorField& field : fields) {
        if (fixedString(field.target, MF_SHORT_TEXT_CAPACITY) == editTarget
            && fixedString(field.key, MF_SHORT_TEXT_CAPACITY) == editKey
            && fixedString(field.value_json, MF_VALUE_CAPACITY) == "\"BridgeSmokeName\"") {
            sawEditedValue = true;
            break;
        }
    }
    expect(sawEditedValue, "edited inspector value should be visible in the refreshed inspector batch");

    required = 0;
    expectStatus(
        mf_editor_scene_state_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query scene state size"
    );
    std::vector<char> sceneState(required);
    expectStatus(
        mf_editor_scene_state_json(editor, sceneState.data(), sceneState.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read scene state"
    );
    expect(std::string(sceneState.data()).find("\"scene_name\"") != std::string::npos, "scene state should name the active scene");

    required = 0;
    expectStatus(
        mf_editor_viewport_state_json(editor, 640, 360, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query professional viewport state"
    );
    std::vector<char> viewportState(required);
    expectStatus(
        mf_editor_viewport_state_json(editor, 640, 360, viewportState.data(), viewportState.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read professional viewport state"
    );
    expect(std::string(viewportState.data()).find("\"pixels_per_unit\"") != std::string::npos, "viewport state should expose its world-to-pixel scale");
    std::size_t transformed = 0;
    expectStatus(
        mf_editor_transform_selection(editor, "{\"mode\":\"delta\",\"dx\":0.5,\"rotation_delta\":5.0}", &transformed, &error),
        MF_STATUS_OK,
        error,
        "transform selected entity through gizmo bridge"
    );
    expect(transformed == 1, "single selection gizmo transform should change one entity");
    expectStatus(mf_editor_execute_command(editor, "edit.undo", &error), MF_STATUS_OK, error, "undo gizmo transform");
    expectStatus(mf_editor_update_selection(editor, selectedId, "replace", &error), MF_STATUS_OK, error, "restore selection after gizmo undo");

    required = 0;
    expectStatus(
        mf_editor_component_catalog_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query component catalog size"
    );
    std::vector<char> componentCatalog(required);
    expectStatus(
        mf_editor_component_catalog_json(editor, componentCatalog.data(), componentCatalog.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read component catalog"
    );
    expect(std::string(componentCatalog.data()).find("\"component_types\"") != std::string::npos, "component catalog should include component groups");

    required = 0;
    expectStatus(
        mf_editor_profiler_snapshot_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query profiler snapshot"
    );
    std::vector<char> profilerSnapshot(required);
    expectStatus(
        mf_editor_profiler_snapshot_json(editor, profilerSnapshot.data(), profilerSnapshot.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read profiler snapshot"
    );
    expect(std::string(profilerSnapshot.data()).find("\"frame_budget_ms\"") != std::string::npos, "profiler should expose the runtime frame budget");
    expectStatus(mf_editor_rebuild_asset_dependencies(editor, &error), MF_STATUS_OK, error, "rebuild asset dependencies");
    required = 0;
    expectStatus(
        mf_editor_asset_dependency_graph_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query asset dependency graph"
    );
    std::vector<char> dependencyGraph(required);
    expectStatus(
        mf_editor_asset_dependency_graph_json(editor, dependencyGraph.data(), dependencyGraph.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read asset dependency graph"
    );
    expect(std::string(dependencyGraph.data()).find("\"nodes\"") != std::string::npos, "dependency graph should expose asset nodes");
    required = 0;
    expectStatus(
        mf_editor_project_operations_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query project operations"
    );
    std::vector<char> projectOperations(required);
    expectStatus(
        mf_editor_project_operations_json(editor, projectOperations.data(), projectOperations.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read project operations"
    );
    expect(std::string(projectOperations.data()).find("\"autosave\"") != std::string::npos, "project operations should expose autosave state");
    if (!useExternalProject) {
        expectStatus(
            mf_editor_project_operation(editor, "autosave_now", "{}", &error),
            MF_STATUS_OK,
            error,
            "create autosave through project operations"
        );
        expectStatus(
            mf_editor_project_operation(editor, "package_export", "{}", &error),
            MF_STATUS_OK,
            error,
            "export project package through project operations"
        );
        expectStatus(
            mf_editor_manage_asset(
                editor,
                "duplicate",
                "{\"source\":\"assets/data/ManagedSmoke.json\",\"target_folder\":\"assets/data\"}",
                &error),
            MF_STATUS_OK,
            error,
            "duplicate asset through safe asset manager"
        );
    }

    const auto readToolState = [&](const char* tool) {
        required = 0;
        expectStatus(
            mf_editor_tool_state_json(editor, tool, nullptr, 0, &required, &error),
            MF_STATUS_BUFFER_TOO_SMALL,
            error,
            std::string("query ") + tool + " tool state"
        );
        std::vector<char> buffer(required);
        expectStatus(
            mf_editor_tool_state_json(editor, tool, buffer.data(), buffer.size(), &required, &error),
            MF_STATUS_OK,
            error,
            std::string("read ") + tool + " tool state"
        );
        return std::string(buffer.data());
    };
    const auto runToolAction = [&](const char* tool, const char* action, const char* payload) {
        std::vector<char> buffer(4 * 1024 * 1024);
        required = 0;
        expectStatus(
            mf_editor_tool_action_json(
                editor, tool, action, payload, buffer.data(), buffer.size(), &required, &error),
            MF_STATUS_OK,
            error,
            std::string(tool) + " action " + action
        );
        return std::string(buffer.data());
    };
    expect(readToolState("sequencer").find("\"sequence\"") != std::string::npos, "sequencer state should expose a sequence");
    expect(readToolState("tilemap").find("\"tilemap\"") != std::string::npos, "tilemap state should expose its layered map");
    expect(readToolState("ui_designer").find("\"preview\"") != std::string::npos, "UI Designer state should expose a preview");
    expect(runToolAction(
               "sequencer",
               "add_track",
               "{\"id\":\"SmokeTrack\",\"track_type\":\"transform\",\"target\":\"SmokeEntity\"}")
               .find("SmokeTrack")
            != std::string::npos,
        "sequencer action should return the updated state");
    runToolAction("sequencer", "undo", "{}");
    expect(runToolAction("tilemap", "paint_cells", "{\"cells\":[{\"x\":1,\"y\":1}],\"value\":4}")
               .find("\"dirty\":true")
            != std::string::npos,
        "tilemap paint should be undoable and mark the document dirty");
    runToolAction("tilemap", "undo", "{}");
    expect(runToolAction(
               "ui_designer",
               "add_widget",
               "{\"widget_type\":\"Button\",\"id\":\"SmokeUiButton\",\"x\":80,\"y\":80}")
               .find("SmokeUiButton")
            != std::string::npos,
        "UI Designer action should create widgets");
    runToolAction("ui_designer", "undo", "{}");

    expectStatus(
        mf_editor_update_selection(editor, selectedId, "toggle", &error),
        MF_STATUS_OK,
        error,
        "toggle selection"
    );
    expectStatus(mf_editor_selected_entity_count(editor, &selectedCount, &error), MF_STATUS_OK, error, "selection after toggle");
    expect(selectedCount == 0, "toggle should deselect the active entity");
    expectStatus(
        mf_editor_update_selection(editor, selectedId, "add", &error),
        MF_STATUS_OK,
        error,
        "add selection"
    );

    MfEntityId duplicateId = 0;
    expectStatus(
        mf_editor_entity_action(editor, selectedId, "duplicate", "{}", &duplicateId, &error),
        MF_STATUS_OK,
        error,
        "duplicate entity action"
    );
    expect(duplicateId != 0 && duplicateId != selectedId, "duplicate action should return a new entity id");
    std::size_t countAfterDuplicate = 0;
    expectStatus(mf_editor_entity_count(editor, &countAfterDuplicate, &error), MF_STATUS_OK, error, "entity count after duplicate");
    expect(countAfterDuplicate == count + 1, "duplicate action should add one entity");
    expectStatus(mf_editor_update_selection(editor, selectedId, "replace", &error), MF_STATUS_OK, error, "select original for multi-edit");
    expectStatus(mf_editor_update_selection(editor, duplicateId, "add", &error), MF_STATUS_OK, error, "add duplicate for multi-edit");
    std::size_t multiEdited = 0;
    expectStatus(
        mf_editor_set_selected_inspector_value_json(editor, "Identity", "tag", "\"BridgeBatch\"", &multiEdited, &error),
        MF_STATUS_OK,
        error,
        "edit common Inspector property"
    );
    expect(multiEdited == 2, "common Inspector edit should update both selected entities");
    expectStatus(mf_editor_execute_command(editor, "edit.undo", &error), MF_STATUS_OK, error, "undo common Inspector edit");
    MfEntityId ignoredEntityId = 0;
    expectStatus(
        mf_editor_entity_action(editor, duplicateId, "delete", "{}", &ignoredEntityId, &error),
        MF_STATUS_OK,
        error,
        "delete duplicated entity"
    );
    expectStatus(mf_editor_update_selection(editor, selectedId, "replace", &error), MF_STATUS_OK, error, "restore selected entity");

    std::size_t commandCount = 0;
    expectStatus(mf_editor_command_count(editor, &commandCount, &error), MF_STATUS_OK, error, "command count");
    expect(commandCount >= 5, "command palette should expose core editor commands");
    std::vector<MfCommandDescriptor> commands(commandCount);
    expectStatus(
        mf_editor_command_descriptors(editor, 0, commands.data(), commands.size(), &written, &total, &error),
        MF_STATUS_OK,
        error,
        "command descriptors"
    );
    expect(written == commandCount && total == commandCount, "command batch should write all commands");

    bool foundRefresh = false;
    bool foundAudit = false;
    bool foundForgeAiDoctor = false;
    for (const MfCommandDescriptor& command : commands) {
        if (fixedString(command.id, MF_NAME_CAPACITY) == "assets.refresh") {
            foundRefresh = true;
        }
        if (fixedString(command.id, MF_NAME_CAPACITY) == "project.audit") {
            foundAudit = true;
        }
        if (fixedString(command.id, MF_NAME_CAPACITY) == "forge_ai.project_doctor") {
            foundForgeAiDoctor = true;
        }
    }
    expect(foundRefresh, "command palette should include assets.refresh");
    expect(foundAudit, "command palette should include project.audit");
    expect(foundForgeAiDoctor, "command palette should include Forge AI Project Doctor");
    expectStatus(mf_editor_execute_command(editor, "project.audit", &error), MF_STATUS_OK, error, "execute project.audit");
    expectStatus(mf_editor_execute_command(editor, "assets.refresh", &error), MF_STATUS_OK, error, "execute assets.refresh");
    expectStatus(mf_editor_refresh(editor, &error), MF_STATUS_OK, error, "refresh editor caches");

    expectStatus(mf_editor_sprite_new_canvas(editor, 8, 8, &error), MF_STATUS_OK, error, "new sprite canvas");
    expectStatus(mf_editor_sprite_begin_edit(editor, &error), MF_STATUS_OK, error, "begin sprite edit");
    std::uint8_t spriteChanged = 0;
    expectStatus(
        mf_editor_sprite_set_pixel(editor, 2, 3, 240, 80, 120, 255, &spriteChanged, &error),
        MF_STATUS_OK,
        error,
        "paint sprite pixel"
    );
    expect(spriteChanged == 1, "painting a new sprite pixel should report a change");
    expectStatus(
        mf_editor_sprite_commit_edit(editor, &spriteChanged, &error),
        MF_STATUS_OK,
        error,
        "commit sprite edit"
    );
    expect(spriteChanged == 1, "committing the sprite stroke should create history");
    MfSpriteInfo spriteInfo {};
    expectStatus(
        mf_editor_sprite_snapshot_rgba(editor, nullptr, 0, &spriteInfo, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe sprite canvas"
    );
    expect(spriteInfo.width == 8 && spriteInfo.height == 8, "sprite dimensions should round-trip");
    std::vector<std::uint8_t> spritePixels(spriteInfo.required_bytes);
    expectStatus(
        mf_editor_sprite_snapshot_rgba(
            editor,
            spritePixels.data(),
            spritePixels.size(),
            &spriteInfo,
            &error
        ),
        MF_STATUS_OK,
        error,
        "read sprite canvas"
    );
    const std::size_t paintedPixel = (3 * 8 + 2) * 4;
    expect(
        spritePixels[paintedPixel] == 240 && spritePixels[paintedPixel + 3] == 255,
        "sprite snapshot should contain the painted pixel"
    );
    expectStatus(mf_editor_sprite_undo(editor, &spriteChanged, &error), MF_STATUS_OK, error, "undo sprite");
    expect(spriteChanged == 1, "sprite undo should report a change");
    expectStatus(mf_editor_sprite_redo(editor, &spriteChanged, &error), MF_STATUS_OK, error, "redo sprite");
    expect(spriteChanged == 1, "sprite redo should report a change");
    expectStatus(
        mf_editor_sprite_transform(editor, "flip_horizontal", "{}", &spriteChanged, &error),
        MF_STATUS_OK,
        error,
        "flip sprite horizontally"
    );
    expect(spriteChanged == 1, "sprite transform should create one history step");
    required = 0;
    expectStatus(
        mf_editor_sprite_animation_clip_json(editor, 2, 2, 12.0F, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe sprite animation timeline"
    );
    std::vector<char> spriteTimeline(required);
    expectStatus(
        mf_editor_sprite_animation_clip_json(
            editor, 2, 2, 12.0F, spriteTimeline.data(), spriteTimeline.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read sprite animation timeline"
    );
    expect(
        fixedString(spriteTimeline.data(), spriteTimeline.size()).find("\"frame_count\":16")
            != std::string::npos,
        "sprite sheet timeline should expose all 2x2 frames"
    );
    expectStatus(
        mf_editor_sprite_transform(editor, "unknown", "{}", &spriteChanged, &error),
        MF_STATUS_INVALID_ARGUMENT,
        error,
        "reject unknown sprite transform"
    );
    std::array<char, MF_PATH_CAPACITY> spritePath {};
    required = 0;
    expectStatus(
        mf_editor_sprite_save(
            editor,
            "BridgeSmokeSprite",
            spritePath.data(),
            spritePath.size(),
            &required,
            &error
        ),
        MF_STATUS_OK,
        error,
        "save sprite canvas"
    );
    expect(std::filesystem::exists(spritePath.data()), "saved sprite path should exist");

    const std::string luauPath = "scripts/BridgeSmokeController.luau";
    const std::string luauSource = "function on_update(dt)\n    move(100 * dt, 0)\nend\n";
    required = 0;
    expectStatus(
        mf_editor_luau_validate_json(editor, luauPath.c_str(), luauSource.c_str(), nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Luau validation"
    );
    std::vector<char> validation(required);
    expectStatus(
        mf_editor_luau_validate_json(
            editor,
            luauPath.c_str(),
            luauSource.c_str(),
            validation.data(),
            validation.size(),
            &required,
            &error
        ),
        MF_STATUS_OK,
        error,
        "validate Luau source"
    );
    expect(
        fixedString(validation.data(), validation.size()).find("\"valid\":true") != std::string::npos,
        "Luau validation JSON should report valid source"
    );
    expectStatus(
        mf_editor_luau_save(editor, luauPath.c_str(), luauSource.c_str(), &error),
        MF_STATUS_OK,
        error,
        "save Luau source"
    );

    required = 0;
    expectStatus(
        mf_editor_luau_read(editor, luauPath.c_str(), nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Luau source"
    );
    std::vector<char> luauContents(required);
    expectStatus(
        mf_editor_luau_read(
            editor,
            luauPath.c_str(),
            luauContents.data(),
            luauContents.size(),
            &required,
            &error
        ),
        MF_STATUS_OK,
        error,
        "read Luau source"
    );
    expect(
        fixedString(luauContents.data(), luauContents.size()) == luauSource,
        "saved Luau source should round-trip through the bridge"
    );

    required = 0;
    expectStatus(
        mf_editor_luau_scripts_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Luau script list"
    );
    std::vector<char> luauScripts(required);
    expectStatus(
        mf_editor_luau_scripts_json(
            editor,
            luauScripts.data(),
            luauScripts.size(),
            &required,
            &error
        ),
        MF_STATUS_OK,
        error,
        "read Luau script list"
    );
    expect(
        fixedString(luauScripts.data(), luauScripts.size()).find(luauPath) != std::string::npos,
        "Luau script list should include the saved document"
    );

    const std::string breakpointsJson = "[{\"path\":\"" + luauPath
        + "\",\"line\":null,\"function\":\"on_update\",\"enabled\":true}]";
    expectStatus(
        mf_editor_luau_set_breakpoints_json(editor, breakpointsJson.c_str(), &error),
        MF_STATUS_OK,
        error,
        "set Luau breakpoints"
    );
    required = 0;
    expectStatus(
        mf_editor_luau_debug_state_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Luau debugger state"
    );
    std::vector<char> debuggerState(required);
    expectStatus(
        mf_editor_luau_debug_state_json(
            editor, debuggerState.data(), debuggerState.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read Luau debugger state"
    );
    expect(
        fixedString(debuggerState.data(), debuggerState.size()).find(luauPath) != std::string::npos,
        "Luau debugger state should round-trip breakpoints"
    );
    expectStatus(
        mf_editor_luau_debug_command(editor, "pause", &error),
        MF_STATUS_OK,
        error,
        "request Luau debugger pause"
    );
    const std::string watchExpressions = "[\"self.speed\"]";
    required = 0;
    expectStatus(
        mf_editor_luau_watches_json(
            editor, watchExpressions.c_str(), nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Luau watches"
    );
    std::vector<char> watchResults(required);
    expectStatus(
        mf_editor_luau_watches_json(
            editor,
            watchExpressions.c_str(),
            watchResults.data(),
            watchResults.size(),
            &required,
            &error),
        MF_STATUS_OK,
        error,
        "read Luau watches"
    );
    expect(
        fixedString(watchResults.data(), watchResults.size()).find("runtime is not paused") != std::string::npos,
        "watch evaluation should report unavailable frame before the pause is reached"
    );

    const std::string graphPath = "scripts/visual_graphs/BridgeSmokeGraph.mfgraph";
    const std::string graphSource = R"({"format":"miniforge.visual-graph","schema_version":1,"engine_version":"0.9.3.4","kind":"MiniForgeVisualGraph","runtime":"rust_visual_graph","name":"BridgeSmokeGraph","variables":{},"nodes":[{"id":"start","type":"EventStart","next":null}]})";
    required = 0;
    expectStatus(
        mf_editor_visual_graph_validate_json(
            editor, graphPath.c_str(), graphSource.c_str(), nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Visual Graph validation"
    );
    std::vector<char> graphValidation(required);
    expectStatus(
        mf_editor_visual_graph_validate_json(
            editor,
            graphPath.c_str(),
            graphSource.c_str(),
            graphValidation.data(),
            graphValidation.size(),
            &required,
            &error),
        MF_STATUS_OK,
        error,
        "validate Visual Graph"
    );
    expect(
        fixedString(graphValidation.data(), graphValidation.size()).find("\"valid\":true") != std::string::npos,
        "Visual Graph validation should return a normalized valid payload"
    );
    expectStatus(
        mf_editor_visual_graph_save(editor, graphPath.c_str(), graphSource.c_str(), &error),
        MF_STATUS_OK,
        error,
        "save Visual Graph"
    );
    required = 0;
    expectStatus(
        mf_editor_visual_graph_catalog_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Visual Graph catalog"
    );
    std::vector<char> graphCatalog(required);
    expectStatus(
        mf_editor_visual_graph_catalog_json(
            editor, graphCatalog.data(), graphCatalog.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read Visual Graph catalog"
    );
    const std::string catalogJson = fixedString(graphCatalog.data(), graphCatalog.size());
    expect(catalogJson.find("output_pins") != std::string::npos,
        "Visual Graph catalog should expose typed output pins");
    expect(catalogJson.find("LogAndMove") != std::string::npos,
        "Visual Graph catalog should expose backend templates");
    expectStatus(
        mf_editor_visual_graph_create_template(
            editor,
            "scripts/visual_graphs/BridgeTemplate.mfgraph",
            "LogAndMove",
            &error),
        MF_STATUS_OK,
        error,
        "create Visual Graph from backend template"
    );

    expectStatus(
        mf_editor_python_install_tools(editor, &error),
        MF_STATUS_OK,
        error,
        "install trusted Python tools"
    );
    required = 0;
    expectStatus(
        mf_editor_python_tools_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Python tools"
    );
    std::vector<char> pythonTools(required);
    expectStatus(
        mf_editor_python_tools_json(
            editor, pythonTools.data(), pythonTools.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read Python tools"
    );
    expect(fixedString(pythonTools.data(), pythonTools.size()).find("scene_report") != std::string::npos,
        "installed Python suite should expose scene_report");
    expectStatus(
        mf_editor_python_run_tool(editor, "scene_report", "{}", &error),
        MF_STATUS_OK,
        error,
        "run trusted Python scene report"
    );
    required = 0;
    expectStatus(
        mf_editor_python_last_result_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "probe Python result"
    );
    std::vector<char> pythonResult(required);
    expectStatus(
        mf_editor_python_last_result_json(
            editor, pythonResult.data(), pythonResult.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read Python result"
    );
    expect(fixedString(pythonResult.data(), pythonResult.size()).find("\"success\":true") != std::string::npos,
        "trusted Python tool result should report success");

    expectStatus(
        mf_editor_export_runtime(editor, "debug", &error),
        MF_STATUS_OK,
        error,
        "export debug runtime"
    );
    required = 0;
    expectStatus(
        mf_editor_last_export_report_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query runtime export report size"
    );
    std::vector<char> exportReport(required);
    expectStatus(
        mf_editor_last_export_report_json(
            editor,
            exportReport.data(),
            exportReport.size(),
            &required,
            &error
        ),
        MF_STATUS_OK,
        error,
        "read runtime export report"
    );
    expect(
        fixedString(exportReport.data(), exportReport.size()).find("\"profile\":\"Debug\"") != std::string::npos,
        "runtime export report should describe the debug profile"
    );

    required = 0;
    expectStatus(
        mf_editor_runtime_health_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query runtime health size"
    );
    expect(required > 2, "runtime health JSON should include an object payload");
    std::vector<char> runtimeHealth(required);
    expectStatus(
        mf_editor_runtime_health_json(editor, runtimeHealth.data(), runtimeHealth.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read runtime health"
    );
    const std::string runtimeHealthJson(runtimeHealth.data());
    expect(runtimeHealthJson.find("\"level\"") != std::string::npos, "runtime health should include a stability level");
    expect(runtimeHealthJson.find("\"stability_score\"") != std::string::npos, "runtime health should include a stability score");
    expect(runtimeHealthJson.find("\"entity_count\"") != std::string::npos, "runtime health should include entity pressure data");

    required = 0;
    expectStatus(
        mf_editor_forge_ai_diagnostics_json(editor, nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query Forge AI diagnostics size"
    );
    expect(required > 2, "Forge AI diagnostics JSON should include at least an array payload");
    std::vector<char> aiDiagnostics(required);
    expectStatus(
        mf_editor_forge_ai_diagnostics_json(editor, aiDiagnostics.data(), aiDiagnostics.size(), &required, &error),
        MF_STATUS_OK,
        error,
        "read Forge AI diagnostics"
    );
    expect(aiDiagnostics.front() == '[', "Forge AI diagnostics should be a JSON array");

    required = 0;
    expectStatus(
        mf_editor_forge_ai_run_test_json(editor, "forge_ai_enemy_smoke", nullptr, 0, &required, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "query Forge AI test report size"
    );
    std::vector<char> aiTestReport(required);
    expectStatus(
        mf_editor_forge_ai_run_test_json(
            editor,
            "forge_ai_enemy_smoke",
            aiTestReport.data(),
            aiTestReport.size(),
            &required,
            &error
        ),
        MF_STATUS_OK,
        error,
        "run Forge AI enemy smoke test"
    );
    expect(
        std::string(aiTestReport.data()).find("forge_ai_enemy_smoke") != std::string::npos,
        "Forge AI test report should name the requested suite"
    );

    std::uint8_t readinessScore = 0;
    expectStatus(mf_editor_readiness_score(editor, &readinessScore, &error), MF_STATUS_OK, error, "readiness score");
    expect(readinessScore <= 100, "readiness score should be a percentage");
    std::size_t readinessCount = 0;
    expectStatus(mf_editor_readiness_count(editor, &readinessCount, &error), MF_STATUS_OK, error, "readiness count");
    expect(readinessCount > 0, "readiness audit should expose at least one system row");
    std::vector<MfReadinessRow> readinessRows(readinessCount);
    expectStatus(
        mf_editor_readiness_rows(editor, 0, readinessRows.data(), readinessRows.size(), &written, &total, &error),
        MF_STATUS_OK,
        error,
        "readiness rows"
    );
    expect(written == readinessCount && total == readinessCount, "readiness batch should write all rows");
    expect(readinessRows.front().abi_version == MF_EDITOR_CORE_API_VERSION, "readiness row ABI version mismatch");
    expect(readinessRows.front().struct_size == sizeof(MfReadinessRow), "readiness row struct size mismatch");
    expect(!fixedString(readinessRows.front().system, MF_SHORT_TEXT_CAPACITY).empty(), "readiness row should include a system name");

    std::size_t assetCount = 0;
    expectStatus(mf_editor_asset_count(editor, &assetCount, &error), MF_STATUS_OK, error, "asset count");
    std::vector<MfAssetRow> assets(assetCount == 0 ? 1 : assetCount);
    expectStatus(
        mf_editor_asset_rows(editor, 0, assets.data(), assetCount, &written, &total, &error),
        MF_STATUS_OK,
        error,
        "asset rows"
    );
    expect(total == assetCount, "asset batch total should match asset count");

    std::size_t consoleCount = 0;
    expectStatus(mf_editor_console_count(editor, &consoleCount, &error), MF_STATUS_OK, error, "console count");
    expect(consoleCount > 0, "opening a project should append a console entry");
    std::vector<MfConsoleEntry> consoleEntries(consoleCount);
    expectStatus(
        mf_editor_console_entries(editor, 0, consoleEntries.data(), consoleEntries.size(), &written, &total, &error),
        MF_STATUS_OK,
        error,
        "console entries"
    );
    expect(written == consoleCount && total == consoleCount, "console batch should write all entries");

    MfViewportInfo viewportInfo {};
    std::vector<std::uint8_t> tooSmall(4);
    expectStatus(
        mf_editor_viewport_snapshot_rgba(editor, 32, 24, tooSmall.data(), tooSmall.size(), &viewportInfo, &error),
        MF_STATUS_BUFFER_TOO_SMALL,
        error,
        "viewport buffer-too-small path"
    );
    expect(viewportInfo.required_bytes == 32u * 24u * 4u, "viewport required byte count should be width * height * rgba");

    std::vector<std::uint8_t> pixels(viewportInfo.required_bytes);
    expectStatus(
        mf_editor_viewport_snapshot_rgba(editor, 32, 24, pixels.data(), pixels.size(), &viewportInfo, &error),
        MF_STATUS_OK,
        error,
        "viewport snapshot"
    );
    expect(viewportInfo.abi_version == MF_EDITOR_CORE_API_VERSION, "viewport info ABI version mismatch");
    expect(viewportInfo.width == 32 && viewportInfo.height == 24, "viewport info dimensions mismatch");
    bool hasNonZeroPixel = false;
    for (std::uint8_t value : pixels) {
        if (value != 0) {
            hasNonZeroPixel = true;
            break;
        }
    }
    expect(hasNonZeroPixel, "viewport snapshot should contain rendered pixels");

    expectStatus(mf_editor_select_entity(editor, UINT64_MAX, &error), MF_STATUS_NOT_FOUND, error, "select missing entity");

    mf_editor_destroy(editor);
    if (!tempProject.empty()) {
        std::filesystem::remove_all(tempProject);
    }
    return 0;
}
