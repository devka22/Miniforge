#pragma once

#include <QObject>
#include <QImage>
#include <QString>
#include <QVector>

#include "miniforge_editor_bridge.h"

class QColor;
class QProcess;

struct MfEntityItem {
    quint64 id = 0;
    quint64 parentId = 0;
    bool hasParent = false;
    bool visible = false;
    bool enabled = false;
    bool locked = false;
    bool selected = false;
    qsizetype componentCount = 0;
    qsizetype childCount = 0;
    double x = 0.0;
    double y = 0.0;
    QString name;
    QString entityType;
    QString tag;
    QString layer;
};

struct MfInspectorItem {
    quint64 entityId = 0;
    bool editable = false;
    bool mixed = false;
    QString target;
    QString key;
    QString displayName;
    QString valueType;
    QString valueJson;
};

struct MfAssetItem {
    quint64 sizeBytes = 0;
    qsizetype dependencyCount = 0;
    QString guid;
    QString relativePath;
    QString name;
    QString assetType;
    QString labels;
};

struct MfCommandItem {
    bool enabled = false;
    QString id;
    QString label;
    QString category;
    QString shortcut;
};

struct MfConsoleItem {
    quint64 frame = 0;
    quint32 severity = 0;
    QString channel;
    QString message;
};

struct MfReadinessItem {
    quint8 score = 0;
    quint32 level = 0;
    qsizetype strengthCount = 0;
    qsizetype gapCount = 0;
    qsizetype actionCount = 0;
    QString system;
    QString levelLabel;
    QString topAction;
};

class MfBridge final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)
    Q_PROPERTY(QString projectPath READ projectPath NOTIFY projectChanged)
    Q_PROPERTY(QString projectName READ projectName NOTIFY projectChanged)
    Q_PROPERTY(QString projectSummary READ projectSummary NOTIFY dataChanged)
    Q_PROPERTY(int readinessScore READ readinessScore NOTIFY readinessChanged)
    Q_PROPERTY(bool projectOpen READ isOpen NOTIFY projectChanged)
    Q_PROPERTY(int entityCount READ entityCount NOTIFY entitiesChanged)
    Q_PROPERTY(int assetCount READ assetCount NOTIFY assetsChanged)
    Q_PROPERTY(int commandCount READ commandCount NOTIFY commandsChanged)
    Q_PROPERTY(int consoleCount READ consoleCount NOTIFY consoleChanged)
    Q_PROPERTY(int selectedEntityCount READ selectedEntityCount NOTIFY selectionChanged)
    Q_PROPERTY(qulonglong selectedEntityId READ selectedEntityId NOTIFY selectionChanged)
    Q_PROPERTY(bool externalLaunchRunning READ externalLaunchRunning NOTIFY dataChanged)
    Q_PROPERTY(QString workbenchSummary READ workbenchSummary NOTIFY dataChanged)

public:
    explicit MfBridge(QObject* parent = nullptr);
    ~MfBridge() override;

    QString lastError() const;
    QString projectPath() const;
    QString projectName() const;
    QString projectSummary() const;
    int readinessScore() const;
    int entityCount() const;
    int assetCount() const;
    int commandCount() const;
    int consoleCount() const;
    int selectedEntityCount() const;
    qulonglong selectedEntityId() const;
    bool externalLaunchRunning() const;
    QString workbenchSummary() const;
    bool isOpen() const;

    Q_INVOKABLE bool openProject(const QString& path);
    Q_INVOKABLE bool openProjectWithOptions(const QString& path, const QString& optionsJson = QStringLiteral("{}"));
    Q_INVOKABLE bool refreshAll();
    Q_INVOKABLE bool selectEntity(qulonglong entityId);
    Q_INVOKABLE bool updateSelection(qulonglong entityId, const QString& mode);
    Q_INVOKABLE bool clearSelection();
    Q_INVOKABLE bool performEntityAction(qulonglong entityId, const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    Q_INVOKABLE bool performSelectedEntityAction(const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    Q_INVOKABLE qulonglong pickEntity(int viewportWidth, int viewportHeight, double x, double y, const QString& selectionMode);
    Q_INVOKABLE QString viewportStateJson(int viewportWidth, int viewportHeight);
    Q_INVOKABLE bool transformSelectionJson(const QString& payloadJson);
    Q_INVOKABLE QString sceneStateJson();
    Q_INVOKABLE QString sceneBrowserStateJson();
    Q_INVOKABLE QString sceneBrowserActionJson(const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    Q_INVOKABLE QString componentCatalogJson();
    Q_INVOKABLE QString authoringCatalogJson();
    Q_INVOKABLE QString authoringPlanJson(const QString& presetId, const QString& parametersJson = QStringLiteral("{}"));
    Q_INVOKABLE QString sdkPackCatalogJson();
    Q_INVOKABLE QString sdkPackPlanJson(const QString& profileId, const QString& registryJson = QStringLiteral("{\"installed\":[]}"));
    Q_INVOKABLE QString toolStateJson(const QString& tool);
    Q_INVOKABLE QString toolActionJson(const QString& tool, const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    Q_INVOKABLE QString prefabStateJson();
    Q_INVOKABLE QString prefabActionJson(const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    Q_INVOKABLE QString projectSettingsJson();
    Q_INVOKABLE bool saveEngineSettingsJson(const QString& source);
    Q_INVOKABLE bool saveInputMapJson(const QString& source);
    Q_INVOKABLE bool saveTagsLayersJson(const QString& source);
    Q_INVOKABLE QString launcherSnapshotJson(const QString& workspaceRoot);
    Q_INVOKABLE QString createProject(const QString& workspaceRoot, const QString& location, const QString& name, const QString& templateName);
    Q_INVOKABLE QString repairProjectJson(const QString& workspaceRoot, const QString& projectPath);
    Q_INVOKABLE QString projectOperationsJson();
    Q_INVOKABLE bool runProjectOperation(const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    Q_INVOKABLE bool launchPreparedExternal();
    Q_INVOKABLE bool stopExternalLaunch();
    Q_INVOKABLE bool manageAsset(const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    Q_INVOKABLE QString profilerSnapshotJson();
    Q_INVOKABLE bool rebuildAssetDependencies();
    Q_INVOKABLE QString assetDependencyGraphJson();
    Q_INVOKABLE bool executeCommand(const QString& commandId);
    Q_INVOKABLE QString inspectorQuickActionsJson(qulonglong entityId) const;
    Q_INVOKABLE bool performInspectorQuickAction(qulonglong entityId, const QString& action, const QString& assetPath = QString());
    Q_INVOKABLE bool setInspectorValueJson(qulonglong entityId, const QString& target, const QString& key, const QString& valueJson);
    Q_INVOKABLE bool setSelectedInspectorValueJson(const QString& target, const QString& key, const QString& valueJson);
    Q_INVOKABLE QString luauScriptsJson();
    Q_INVOKABLE QString readLuauScript(const QString& relativePath);
    Q_INVOKABLE QString validateLuauSource(const QString& relativePath, const QString& source);
    Q_INVOKABLE bool saveLuauScript(const QString& relativePath, const QString& source);
    Q_INVOKABLE QString luauApiJson() const;
    Q_INVOKABLE QString luauDebugStateJson();
    Q_INVOKABLE bool setLuauBreakpointsJson(const QString& breakpointsJson);
    Q_INVOKABLE bool luauDebugCommand(const QString& command);
    Q_INVOKABLE QString luauWatchesJson(const QString& expressionsJson);
    Q_INVOKABLE QString workspaceStateJson();
    Q_INVOKABLE bool saveWorkspaceState(const QString& stateJson);
    Q_INVOKABLE QString visualGraphCatalogJson() const;
    Q_INVOKABLE QString createVisualGraphTemplate(const QString& name, const QString& templateName);
    Q_INVOKABLE QString validateVisualGraph(const QString& relativePath, const QString& source);
    Q_INVOKABLE bool saveVisualGraph(const QString& relativePath, const QString& source);
    Q_INVOKABLE QString pythonToolsJson();
    Q_INVOKABLE bool installPythonTools();
    Q_INVOKABLE QString runPythonTool(const QString& toolId, const QString& parametersJson = QStringLiteral("{}"));
    Q_INVOKABLE bool openExternalEditor(const QString& relativePath, const QString& command = QString());
    Q_INVOKABLE QString contentFoldersJson();
    Q_INVOKABLE QString contentEntriesJson(const QString& relativeDirectory);
    Q_INVOKABLE bool createContentFolder(const QString& relativeDirectory, const QString& name);
    Q_INVOKABLE QString createContentFile(const QString& kind, const QString& relativeDirectory, const QString& name);
    Q_INVOKABLE QString readTextAsset(const QString& relativePath);
    Q_INVOKABLE bool saveTextAsset(const QString& relativePath, const QString& source);
    Q_INVOKABLE void requestOpenContentAsset(const QString& relativePath, const QString& assetType);
    Q_INVOKABLE QString exportRuntime(const QString& profile);
    Q_INVOKABLE QString runtimeHealthJson();
    Q_INVOKABLE QString forgeAiDiagnosticsJson();
    Q_INVOKABLE QString runForgeAiTestJson(const QString& suiteId);

    QVector<MfEntityItem> entities() const;
    QVector<quint64> selectedEntities() const;
    QVector<MfInspectorItem> inspectorFields(quint64 entityId) const;
    QVector<MfAssetItem> assets() const;
    QVector<MfCommandItem> commands() const;
    QVector<MfConsoleItem> consoleEntries() const;
    QVector<MfReadinessItem> readinessRows() const;
    QImage viewportImage(const QSize& size) const;
    QImage spriteImage(MfSpriteInfo* info = nullptr) const;
    bool newSpriteCanvas(int width, int height);
    bool beginSpriteEdit();
    bool setSpritePixel(int x, int y, const QColor& color);
    bool clearSprite(const QColor& color);
    bool transformSprite(const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    QString spriteAnimationClipJson(int frameWidth, int frameHeight, double fps) const;
    bool commitSpriteEdit();
    bool undoSprite();
    bool redoSprite();
    QString saveSprite(const QString& fallbackName);

signals:
    void lastErrorChanged();
    void projectChanged();
    void dataChanged();
    void entitiesChanged();
    void sceneStateChanged();
    void editorToolChanged(const QString& tool);
    void prefabChanged();
    void settingsChanged();
    void assetsChanged();
    void commandsChanged();
    void consoleChanged();
    void readinessChanged();
    void runtimeHealthChanged();
    void luauScriptsChanged();
    void luauDebuggerChanged();
    void pythonToolsChanged();
    void spriteChanged();
    void exportCompleted();
    void operationCompleted(const QString& message, bool success);
    void contentAssetOpenRequested(const QString& relativePath, const QString& assetType);
    void selectionChanged(qulonglong entityId);

private:
    bool setError(const MfError& error, const QString& fallback) const;
    bool setLocalError(const QString& message) const;
    bool ensureOk(MfStatus status, const MfError& error, const QString& fallback) const;
    QString readProjectPath() const;

    MfEditorHandle* m_handle = nullptr;
    QProcess* m_externalProcess = nullptr;
    mutable QString m_lastError;
    QString m_projectPath;
};
