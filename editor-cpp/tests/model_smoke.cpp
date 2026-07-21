#include <QCoreApplication>
#include <QColor>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
#include <QVariant>

#include <cstdlib>
#include <iostream>

#include "MfBridge.h"
#include "MfModels.h"

namespace {

void expect(bool condition, const char* message)
{
    if (condition) {
        return;
    }
    std::cerr << "model smoke failed: " << message << '\n';
    std::exit(EXIT_FAILURE);
}

void expect(bool condition, const QString& message)
{
    if (condition) {
        return;
    }
    std::cerr << "model smoke failed: " << message.toStdString() << '\n';
    std::exit(EXIT_FAILURE);
}

} // namespace

int main(int argc, char** argv)
{
    QCoreApplication app(argc, argv);
    expect(argc >= 2, "expected a project path argument");

    MfBridge bridge;
    EntityModel entities(&bridge);
    InspectorModel inspector(&bridge);
    AssetModel assets(&bridge);
    CommandModel commands(&bridge);
    ConsoleModel console(&bridge);
    ReadinessModel readiness(&bridge);

    expect(bridge.openProject(QString::fromLocal8Bit(argv[1])), "failed to open the smoke project");
    expect(entities.rowCount() > 0, "hierarchy should expose project entities");
    expect(commands.rowCount() > 0, "command palette should expose editor commands");
    expect(readiness.rowCount() > 0, "readiness panel should expose system rows");
    const QString runtimeHealth = bridge.runtimeHealthJson();
    expect(runtimeHealth.contains(QStringLiteral("\"stability_score\"")), "runtime health bridge should expose telemetry JSON");
    expect(bridge.sceneStateJson().contains(QStringLiteral("\"scene_name\"")), "scene state bridge should expose the active scene");
    expect(bridge.sceneBrowserStateJson().contains(QStringLiteral("\"scenes\"")), "Scene Browser bridge should expose the scene catalog");
    expect(bridge.componentCatalogJson().contains(QStringLiteral("\"component_types\"")), "component catalog should expose registry groups");
    expect(bridge.viewportStateJson(640, 360).contains(QStringLiteral("\"pixels_per_unit\"")), "viewport bridge should expose gizmo metadata");
    const QString luauApi = bridge.luauApiJson();
    expect(luauApi.contains(QStringLiteral("\"Time.delta_time\"")), "Luau API should include Time declarations");
    expect(luauApi.contains(QStringLiteral("\"Tilemap.set_tile\"")), "Luau API should include Tilemap declarations");
    expect(luauApi.contains(QStringLiteral("\"Tween.to\"")), "Luau API should include Tween declarations");
    expect(luauApi.contains(QStringLiteral("\"Audio2D.play\"")), "Luau API should include Audio2D declarations");
    expect(luauApi.contains(QStringLiteral("\"Scene.load\"")), "Luau API should include Scene declarations");
    expect(luauApi.contains(QStringLiteral("\"Game.save_slot\"")), "Luau API should include Game declarations");
    expect(luauApi.contains(QStringLiteral("\"Assets.exists\"")), "Luau API should include Assets declarations");
    expect(bridge.toolStateJson(QStringLiteral("sequencer")).contains(QStringLiteral("\"sequence\"")), "Qt bridge should expose the animation timeline");
    expect(bridge.toolStateJson(QStringLiteral("tilemap")).contains(QStringLiteral("\"tilemap\"")), "Qt bridge should expose the tilemap editor");
    expect(bridge.toolStateJson(QStringLiteral("ui_designer")).contains(QStringLiteral("\"preview\"")), "Qt bridge should expose the UI Designer");
    expect(bridge.toolActionJson(QStringLiteral("sequencer"), QStringLiteral("set_cursor"), QStringLiteral("{\"cursor\":0.25}"))
               .contains(QStringLiteral("\"cursor\":0.25")),
        "Qt bridge should round-trip tool actions");
    expect(bridge.prefabStateJson().contains(QStringLiteral("\"prefab_assets\"")), "Prefab Studio bridge should expose its asset list");
    expect(bridge.projectSettingsJson().contains(QStringLiteral("\"engine\"")), "Project Settings bridge should expose engine settings");
    expect(bridge.launcherSnapshotJson(QFileInfo(QString::fromLocal8Bit(argv[1])).absolutePath())
               .contains(QStringLiteral("\"templates\"")),
        "Project Launcher bridge should expose templates");
    expect(bridge.profilerSnapshotJson().contains(QStringLiteral("\"frame_budget_ms\"")), "Profiler bridge should expose runtime budgets");
    expect(bridge.assetDependencyGraphJson().contains(QStringLiteral("\"nodes\"")), "Asset graph bridge should expose dependency nodes");
    expect(bridge.projectOperationsJson().contains(QStringLiteral("\"autosave\"")), "Project Operations bridge should expose autosave/session state");
    expect(bridge.newSpriteCanvas(4, 2), "Qt bridge should create a sprite canvas");
    expect(bridge.beginSpriteEdit(), "Qt bridge should begin a sprite edit");
    expect(bridge.setSpritePixel(0, 0, QColor(255, 80, 40, 255)), "Qt bridge should paint a sprite pixel");
    expect(bridge.commitSpriteEdit(), "Qt bridge should commit sprite history");
    expect(bridge.transformSprite(QStringLiteral("flip_horizontal")), "Qt bridge should apply sprite transforms");
    expect(
        bridge.spriteAnimationClipJson(2, 2, 12.0).contains(QStringLiteral("\"frame_count\":2")),
        "Qt bridge should expose sprite-sheet timeline metadata");

    quint64 selectableEntity = 0;
    for (const MfEntityItem& entity : bridge.entities()) {
        if (entity.visible && entity.enabled && !entity.locked) {
            selectableEntity = entity.id;
            break;
        }
    }
    expect(selectableEntity != 0, "smoke project should contain a selectable entity");
    expect(bridge.updateSelection(selectableEntity, QStringLiteral("replace")), "replace selection should succeed");
    expect(bridge.selectedEntityCount() == 1, "replace selection should select one entity");
    expect(inspector.entityId() == selectableEntity && inspector.rowCount() > 0, "selection should feed Inspector fields");
    const int inspectorFieldCount = inspector.rowCount();
    inspector.setFilter(QStringLiteral("__missing_property__"));
    expect(inspector.rowCount() == 0, "Inspector filter should remove unmatched fields");
    inspector.setFilter({});
    expect(inspector.rowCount() == inspectorFieldCount, "clearing Inspector filter should restore fields");
    quint64 secondSelectable = 0;
    for (const MfEntityItem& entity : bridge.entities()) {
        if (entity.id != selectableEntity && entity.visible && entity.enabled && !entity.locked) {
            secondSelectable = entity.id;
            break;
        }
    }
    expect(secondSelectable != 0, "smoke project should contain a second selectable entity for multi-edit");
    expect(bridge.updateSelection(secondSelectable, QStringLiteral("add")), "multi-selection add should succeed");
    expect(bridge.selectedEntityCount() == 2, "Inspector multi-edit should receive two selected entities");
    inspector.refresh();
    expect(inspector.rowCount() > 0 && inspector.rowCount() <= inspectorFieldCount, "Inspector should expose only common multi-selection fields");
    expect(bridge.setSelectedInspectorValueJson(QStringLiteral("Identity"), QStringLiteral("tag"), QStringLiteral("\"QtBatch\"")), "common Inspector edit should succeed");
    expect(bridge.executeCommand(QStringLiteral("edit.undo")), "common Inspector edit should undo as one command");
    expect(bridge.updateSelection(selectableEntity, QStringLiteral("replace")), "selection should restore after multi-edit undo");
    expect(bridge.transformSelectionJson(QStringLiteral("{\"mode\":\"delta\",\"dx\":0.25}")), "viewport gizmo transform should succeed");
    expect(bridge.executeCommand(QStringLiteral("edit.undo")), "viewport gizmo transform should undo as one command");
    expect(bridge.updateSelection(selectableEntity, QStringLiteral("replace")), "selection should restore after gizmo undo");
    expect(bridge.updateSelection(selectableEntity, QStringLiteral("toggle")), "toggle selection should succeed");
    expect(bridge.selectedEntityCount() == 0, "toggle selection should deselect an active entity");
    expect(bridge.updateSelection(selectableEntity, QStringLiteral("add")), "add selection should succeed");

    const int entityCountBeforeDuplicate = bridge.entityCount();
    expect(bridge.performEntityAction(selectableEntity, QStringLiteral("duplicate")), "duplicate action should succeed");
    const quint64 duplicatedEntity = bridge.selectedEntityId();
    expect(duplicatedEntity != 0 && duplicatedEntity != selectableEntity, "duplicate action should select a new entity");
    expect(bridge.entityCount() == entityCountBeforeDuplicate + 1, "duplicate action should add one entity");
    expect(bridge.performEntityAction(duplicatedEntity, QStringLiteral("delete")), "delete action should succeed");
    expect(bridge.entityCount() == entityCountBeforeDuplicate, "delete action should restore the entity count");
    expect(bridge.clearSelection(), "clear selection should succeed");

    const int commandCount = commands.rowCount();
    commands.setFilter(QStringLiteral("project.audit"));
    expect(commands.rowCount() > 0, "command filter should find project.audit");
    expect(commands.rowCount() <= commandCount, "command filtering should not add rows");
    commands.setFilter(QStringLiteral("__missing_command__"));
    expect(commands.rowCount() == 0, "missing command filter should be empty");
    commands.setFilter({});
    expect(commands.rowCount() == commandCount, "clearing the command filter should restore rows");

    const int assetCount = assets.rowCount();
    assets.setFilter(QStringLiteral("__missing_asset__"));
    expect(assets.rowCount() == 0, "missing asset filter should be empty");
    assets.setFilter({});
    expect(assets.rowCount() == assetCount, "clearing the asset filter should restore rows");

    expect(bridge.executeCommand(QStringLiteral("project.audit")), "project.audit should execute");
    console.refresh();
    const int consoleCount = console.rowCount();
    expect(consoleCount > 0, "console should receive command feedback");
    console.setFilter(QStringLiteral("__missing_console_message__"));
    expect(console.rowCount() == 0, "console text filter should remove unmatched rows");
    console.setFilter({});
    expect(console.rowCount() == consoleCount, "clearing console text should restore rows");
    console.setMinimumSeverity(3);
    for (int row = 0; row < console.rowCount(); ++row) {
        const QModelIndex index = console.index(row, 0);
        expect(console.data(index, ConsoleModel::SeverityRole).toUInt() >= 3, "severity filter leaked a lower-severity row");
    }
    console.setMinimumSeverity(0);
    expect(console.rowCount() == consoleCount, "clearing severity filter should restore rows");

    QTemporaryDir contentProject;
    expect(contentProject.isValid(), "content browser test project should be created");
    expect(QDir().mkpath(QDir(contentProject.path()).filePath(QStringLiteral("assets"))), "content assets folder should be created");
    expect(bridge.openProject(contentProject.path()), "content browser test project should open");
    expect(bridge.createContentFolder(QStringLiteral("assets"), QStringLiteral("Managed")), "Content Browser should create folders");

    const QString luauPath = bridge.createContentFile(QStringLiteral("luau"), {}, QStringLiteral("ContentSmoke"));
    const QString luauError = bridge.lastError();
    const QString scenePath = bridge.createContentFile(QStringLiteral("scene"), {}, QStringLiteral("ContentScene"));
    const QString prefabPath = bridge.createContentFile(QStringLiteral("prefab"), {}, QStringLiteral("ContentPrefab"));
    const QString graphPath = bridge.createContentFile(QStringLiteral("visual_graph"), {}, QStringLiteral("ContentFlow"));
    const QString configPath = bridge.createContentFile(QStringLiteral("config"), {}, QStringLiteral("ContentConfig"));
    const QString materialPath = bridge.createContentFile(QStringLiteral("material"), {}, QStringLiteral("ContentMaterial"));
    const QString shaderPath = bridge.createContentFile(QStringLiteral("shader"), {}, QStringLiteral("ContentShader"));
    const QString uiPath = bridge.createContentFile(QStringLiteral("ui"), {}, QStringLiteral("ContentHud"));
    const QString tilemapPath = bridge.createContentFile(QStringLiteral("tilemap"), {}, QStringLiteral("ContentWorld"));
    const QString soundCuePath = bridge.createContentFile(QStringLiteral("sound_cue"), {}, QStringLiteral("ContentCue"));
    const QString dataPath = bridge.createContentFile(QStringLiteral("json"), {}, QStringLiteral("ContentData"));
    expect(
        !luauPath.isEmpty() && QFileInfo::exists(QDir(contentProject.path()).filePath(luauPath)),
        QStringLiteral("Content Browser should create Luau assets · path='%1' · error='%2'")
            .arg(luauPath, luauError));
    expect(!scenePath.isEmpty() && !prefabPath.isEmpty(), "Content Browser should create Scene and Prefab assets");
    expect(!configPath.isEmpty() && !materialPath.isEmpty() && !shaderPath.isEmpty(), "Content Browser should create config, Material and Shader assets");
    expect(graphPath.startsWith(QStringLiteral("scripts/visual_graphs/")), "Visual Graphs should stay in their supported editor folder");
    expect(bridge.readTextAsset(uiPath).contains(QStringLiteral("\"viewport_width\"")), "UI asset template should be editor-readable");
    expect(bridge.readTextAsset(tilemapPath).contains(QStringLiteral("\"tilemap\"")), "tilemap template should expose layered data");
    expect(bridge.readTextAsset(soundCuePath).contains(QStringLiteral("MiniForgeSoundCue")), "SoundCue template should identify its runtime kind");
    expect(bridge.contentFoldersJson().contains(QStringLiteral("assets/ui")), "folder tree JSON should include generated asset folders");
    expect(bridge.contentEntriesJson(QStringLiteral("assets/ui")).contains(QStringLiteral("\"asset_type\":\"UI\"")), "content entries should classify UI assets");
    expect(bridge.contentEntriesJson(QStringLiteral("assets/tilemaps")).contains(QStringLiteral("\"asset_type\":\"Tilemap\"")), "content entries should classify Tilemap assets");
    expect(bridge.contentEntriesJson(QStringLiteral("assets/audio")).contains(QStringLiteral("\"asset_type\":\"SoundCue\"")), "content entries should classify SoundCue assets");

    expect(bridge.manageAsset(
               QStringLiteral("rename"),
               QStringLiteral("{\"source\":\"") + dataPath + QStringLiteral("\",\"new_name\":\"ContentRenamed\"}")),
        "Content Browser rename should reach the safe asset backend");
    const QString renamedPath = QStringLiteral("assets/data/ContentRenamed.json");
    expect(QFileInfo::exists(QDir(contentProject.path()).filePath(renamedPath)), "renamed asset should exist");
    expect(bridge.manageAsset(
               QStringLiteral("duplicate"),
               QStringLiteral("{\"source\":\"") + renamedPath + QStringLiteral("\"}")),
        "Content Browser duplicate should reach the safe asset backend");
    expect(bridge.createContentFolder(QStringLiteral("assets"), QStringLiteral("Moved")), "move target should be created");
    expect(bridge.manageAsset(
               QStringLiteral("move"),
               QStringLiteral("{\"source\":\"") + renamedPath + QStringLiteral("\",\"target_folder\":\"assets/Moved\"}")),
        "Content Browser move should reach the safe asset backend");
    const QString movedPath = QStringLiteral("assets/Moved/ContentRenamed.json");
    expect(QFileInfo::exists(QDir(contentProject.path()).filePath(movedPath)), "moved asset should exist");
    expect(bridge.manageAsset(
               QStringLiteral("delete"),
               QStringLiteral("{\"source\":\"") + movedPath + QStringLiteral("\",\"confirm\":true}")),
        "Content Browser delete should move an asset to MiniForge Trash");
    expect(!QFileInfo::exists(QDir(contentProject.path()).filePath(movedPath)), "trashed asset should leave its original folder");

    QTemporaryDir importSource;
    expect(importSource.isValid(), "import source folder should be created");
    const QString externalPath = QDir(importSource.path()).filePath(QStringLiteral("content_import.txt"));
    QFile externalFile(externalPath);
    expect(externalFile.open(QIODevice::WriteOnly), "external import source should open");
    expect(externalFile.write("MiniForge import smoke\n") > 0, "external import source should be written");
    externalFile.close();
    QString escapedExternal = externalPath;
    escapedExternal.replace(QStringLiteral("\\"), QStringLiteral("\\\\"));
    escapedExternal.replace(QStringLiteral("\""), QStringLiteral("\\\""));
    expect(bridge.manageAsset(
               QStringLiteral("import"),
               QStringLiteral("{\"source_external\":\"") + escapedExternal
                   + QStringLiteral("\",\"target_folder\":\"assets/Managed\"}")),
        "Content Browser import should reach the safe asset backend");
    expect(bridge.contentEntriesJson(QStringLiteral("assets/Managed")).contains(QStringLiteral("content_import.txt")), "imported asset should appear in Content Browser entries");

    std::cout << "MiniForge Qt model smoke passed: "
              << commandCount << " commands, "
              << assetCount << " assets, "
              << consoleCount << " console rows\n";
    return EXIT_SUCCESS;
}
