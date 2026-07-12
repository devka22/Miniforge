#pragma once

#include <QObject>
#include <QImage>
#include <QString>
#include <QVector>

#include "miniforge_editor_bridge.h"

class QColor;

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
    Q_PROPERTY(qulonglong selectedEntityId READ selectedEntityId NOTIFY selectionChanged)
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
    qulonglong selectedEntityId() const;
    QString workbenchSummary() const;
    bool isOpen() const;

    Q_INVOKABLE bool openProject(const QString& path);
    Q_INVOKABLE bool refreshAll();
    Q_INVOKABLE bool selectEntity(qulonglong entityId);
    Q_INVOKABLE bool executeCommand(const QString& commandId);
    Q_INVOKABLE bool setInspectorValueJson(qulonglong entityId, const QString& target, const QString& key, const QString& valueJson);
    Q_INVOKABLE QString luauScriptsJson();
    Q_INVOKABLE QString readLuauScript(const QString& relativePath);
    Q_INVOKABLE QString validateLuauSource(const QString& relativePath, const QString& source);
    Q_INVOKABLE bool saveLuauScript(const QString& relativePath, const QString& source);
    Q_INVOKABLE QString exportRuntime(const QString& profile);
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
    bool commitSpriteEdit();
    bool undoSprite();
    bool redoSprite();
    QString saveSprite(const QString& fallbackName);

signals:
    void lastErrorChanged();
    void projectChanged();
    void dataChanged();
    void entitiesChanged();
    void assetsChanged();
    void commandsChanged();
    void consoleChanged();
    void readinessChanged();
    void luauScriptsChanged();
    void spriteChanged();
    void exportCompleted();
    void selectionChanged(qulonglong entityId);

private:
    bool setError(const MfError& error, const QString& fallback) const;
    bool ensureOk(MfStatus status, const MfError& error, const QString& fallback) const;
    QString readProjectPath() const;

    MfEditorHandle* m_handle = nullptr;
    mutable QString m_lastError;
    QString m_projectPath;
};
