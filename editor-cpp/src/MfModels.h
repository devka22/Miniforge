#pragma once

#include <QAbstractItemModel>
#include <QAbstractListModel>
#include <QAbstractTableModel>
#include <QHash>
#include <QObject>
#include <QStringList>

#include <memory>
#include <vector>

#include "MfBridge.h"

struct EntityTreeNode;

class EntityModel final : public QAbstractItemModel {
    Q_OBJECT
    Q_PROPERTY(QString filter READ filter WRITE setFilter NOTIFY filterChanged)
public:
    enum Roles {
        EntityIdRole = Qt::UserRole + 1,
        ParentIdRole,
        HasParentRole,
        NameRole,
        TypeRole,
        TagRole,
        LayerRole,
        PositionRole,
        SelectedRole,
        VisibleRole,
        LockedRole,
        ComponentCountRole,
        ChildCountRole,
    };

    explicit EntityModel(MfBridge* bridge, QObject* parent = nullptr);
    ~EntityModel() override;
    QString filter() const;
    void setFilter(const QString& filter);
    QModelIndex index(int row, int column, const QModelIndex& parent = QModelIndex()) const override;
    QModelIndex parent(const QModelIndex& child) const override;
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    int columnCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
signals:
    void filterChanged();
public slots:
    void refresh();
    void updateSelection(qulonglong entityId);
private:
    EntityTreeNode* nodeFromIndex(const QModelIndex& index) const;
    QModelIndex indexForNode(const EntityTreeNode* node) const;
    QVector<MfEntityItem> filteredRows() const;
    void rebuildTree(const QVector<MfEntityItem>& rows);
    void resetTree();

    MfBridge* m_bridge = nullptr;
    QString m_filter;
    QVector<MfEntityItem> m_allRows;
    std::unique_ptr<EntityTreeNode> m_root;
    std::vector<std::unique_ptr<EntityTreeNode>> m_nodes;
    QHash<quint64, EntityTreeNode*> m_lookup;
};

class InspectorModel final : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(qulonglong entityId READ entityId WRITE setEntityId NOTIFY entityIdChanged)
public:
    enum Roles {
        TargetRole = Qt::UserRole + 1,
        EntityIdRole,
        KeyRole,
        DisplayNameRole,
        ValueJsonRole,
        ValueTypeRole,
        EditableRole,
    };

    explicit InspectorModel(MfBridge* bridge, QObject* parent = nullptr);
    qulonglong entityId() const;
    void setEntityId(qulonglong entityId);
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
signals:
    void entityIdChanged();
public slots:
    void refresh();
private:
    MfBridge* m_bridge = nullptr;
    quint64 m_entityId = 0;
    QVector<MfInspectorItem> m_rows;
};

class AssetModel final : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(QString filter READ filter WRITE setFilter NOTIFY filterChanged)
public:
    enum Roles {
        GuidRole = Qt::UserRole + 1,
        PathRole,
        NameRole,
        TypeRole,
        SizeRole,
        LabelsRole,
    };

    explicit AssetModel(MfBridge* bridge, QObject* parent = nullptr);
    QString filter() const;
    void setFilter(const QString& filter);
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
signals:
    void filterChanged();
public slots:
    void refresh();
private:
    QVector<MfAssetItem> filteredRows() const;
    void resetRows();

    MfBridge* m_bridge = nullptr;
    QString m_filter;
    QVector<MfAssetItem> m_allRows;
    QVector<MfAssetItem> m_rows;
};

class CommandModel final : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(QString filter READ filter WRITE setFilter NOTIFY filterChanged)
public:
    enum Roles {
        IdRole = Qt::UserRole + 1,
        LabelRole,
        CategoryRole,
        ShortcutRole,
        EnabledRole,
    };

    explicit CommandModel(MfBridge* bridge, QObject* parent = nullptr);
    QString filter() const;
    void setFilter(const QString& filter);
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
signals:
    void filterChanged();
public slots:
    void refresh();
private:
    QVector<MfCommandItem> filteredRows() const;
    void resetRows();

    MfBridge* m_bridge = nullptr;
    QString m_filter;
    QVector<MfCommandItem> m_allRows;
    QVector<MfCommandItem> m_rows;
};

class ConsoleModel final : public QAbstractTableModel {
    Q_OBJECT
public:
    enum Columns {
        FrameColumn = 0,
        SeverityColumn,
        ChannelColumn,
        MessageColumn,
        ColumnCount,
    };

    enum Roles {
        FrameRole = Qt::UserRole + 1,
        SeverityRole,
        ChannelRole,
        MessageRole,
    };

    explicit ConsoleModel(MfBridge* bridge, QObject* parent = nullptr);
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    int columnCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QVariant headerData(int section, Qt::Orientation orientation, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
public slots:
    void refresh();
private:
    MfBridge* m_bridge = nullptr;
    QVector<MfConsoleItem> m_rows;
};

class ReadinessModel final : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(int score READ score NOTIFY scoreChanged)
public:
    enum Roles {
        SystemRole = Qt::UserRole + 1,
        LevelRole,
        LevelLabelRole,
        ScoreRole,
        StrengthCountRole,
        GapCountRole,
        ActionCountRole,
        TopActionRole,
    };

    explicit ReadinessModel(MfBridge* bridge, QObject* parent = nullptr);
    int score() const;
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
signals:
    void scoreChanged();
public slots:
    void refresh();
private:
    MfBridge* m_bridge = nullptr;
    int m_score = 0;
    QVector<MfReadinessItem> m_rows;
};

struct ForgeAiDiagnosticItem {
    QString severity;
    QString code;
    QString message;
    QString evidence;
    QString proposedFix;
};

class ForgeAiModel final : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(int diagnosticCount READ diagnosticCount NOTIFY summaryChanged)
    Q_PROPERTY(int criticalCount READ criticalCount NOTIFY summaryChanged)
    Q_PROPERTY(int errorCount READ errorCount NOTIFY summaryChanged)
    Q_PROPERTY(int warningCount READ warningCount NOTIFY summaryChanged)
    Q_PROPERTY(int suggestionCount READ suggestionCount NOTIFY summaryChanged)
    Q_PROPERTY(QString scanSummary READ scanSummary NOTIFY summaryChanged)
    Q_PROPERTY(QString testStatus READ testStatus NOTIFY testChanged)
    Q_PROPERTY(QString testSummary READ testSummary NOTIFY testChanged)
    Q_PROPERTY(QStringList testFailures READ testFailures NOTIFY testChanged)
public:
    enum Roles {
        SeverityRole = Qt::UserRole + 1,
        CodeRole,
        MessageRole,
        EvidenceRole,
        ProposedFixRole,
    };

    explicit ForgeAiModel(MfBridge* bridge, QObject* parent = nullptr);
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
    int diagnosticCount() const;
    int criticalCount() const;
    int errorCount() const;
    int warningCount() const;
    int suggestionCount() const;
    QString scanSummary() const;
    QString testStatus() const;
    QString testSummary() const;
    QStringList testFailures() const;
    Q_INVOKABLE void runDoctor();
    Q_INVOKABLE void runEnemySmoke();
signals:
    void summaryChanged();
    void testChanged();
private:
    MfBridge* m_bridge = nullptr;
    QVector<ForgeAiDiagnosticItem> m_rows;
    int m_criticalCount = 0;
    int m_errorCount = 0;
    int m_warningCount = 0;
    int m_suggestionCount = 0;
    QString m_scanSummary = QStringLiteral("Not scanned");
    QString m_testStatus = QStringLiteral("NotRun");
    QString m_testSummary = QStringLiteral("Enemy smoke test has not run");
    QStringList m_testFailures;
};

class MfEditorController final : public QObject {
    Q_OBJECT
public:
    MfEditorController(MfBridge* bridge, InspectorModel* inspector, QObject* parent = nullptr);
    Q_INVOKABLE void selectEntity(qulonglong entityId);
    Q_INVOKABLE void executeCommand(const QString& commandId);
    Q_INVOKABLE void setInspectorValue(qulonglong entityId, const QString& target, const QString& key, const QString& valueJson);
private:
    MfBridge* m_bridge = nullptr;
    InspectorModel* m_inspector = nullptr;
};
