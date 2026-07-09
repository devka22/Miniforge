#include "miniforge_editor_bridge.h"

#include <array>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <filesystem>
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
    for (const MfCommandDescriptor& command : commands) {
        if (fixedString(command.id, MF_NAME_CAPACITY) == "assets.refresh") {
            foundRefresh = true;
        }
        if (fixedString(command.id, MF_NAME_CAPACITY) == "project.audit") {
            foundAudit = true;
        }
    }
    expect(foundRefresh, "command palette should include assets.refresh");
    expect(foundAudit, "command palette should include project.audit");
    expectStatus(mf_editor_execute_command(editor, "project.audit", &error), MF_STATUS_OK, error, "execute project.audit");
    expectStatus(mf_editor_execute_command(editor, "assets.refresh", &error), MF_STATUS_OK, error, "execute assets.refresh");

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
