#include "MfModels.h"

#include <QSet>

struct EntityTreeNode {
    MfEntityItem item;
    EntityTreeNode* parent = nullptr;
    QVector<EntityTreeNode*> children;
    int row = 0;
};

EntityModel::EntityModel(MfBridge* bridge, QObject* parent)
    : QAbstractItemModel(parent)
    , m_bridge(bridge)
    , m_root(std::make_unique<EntityTreeNode>())
{
    connect(m_bridge, &MfBridge::entitiesChanged, this, &EntityModel::refresh);
    connect(m_bridge, &MfBridge::selectionChanged, this, &EntityModel::updateSelection);
}

EntityModel::~EntityModel() = default;

QModelIndex EntityModel::index(int row, int column, const QModelIndex& parent) const
{
    if (row < 0 || column != 0) {
        return {};
    }
    const EntityTreeNode* parentNode = nodeFromIndex(parent);
    if (!parentNode || row >= parentNode->children.size()) {
        return {};
    }
    return createIndex(row, column, parentNode->children.at(row));
}

QModelIndex EntityModel::parent(const QModelIndex& child) const
{
    if (!child.isValid()) {
        return {};
    }
    const auto* node = static_cast<EntityTreeNode*>(child.internalPointer());
    if (!node || !node->parent || node->parent == m_root.get()) {
        return {};
    }
    return createIndex(node->parent->row, 0, node->parent);
}

int EntityModel::rowCount(const QModelIndex& parent) const
{
    if (parent.isValid() && parent.column() != 0) {
        return 0;
    }
    const EntityTreeNode* node = nodeFromIndex(parent);
    return node ? node->children.size() : 0;
}

int EntityModel::columnCount(const QModelIndex& parent) const
{
    Q_UNUSED(parent);
    return 1;
}

QVariant EntityModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid()) {
        return {};
    }
    const auto* node = static_cast<EntityTreeNode*>(index.internalPointer());
    if (!node) {
        return {};
    }
    const auto& row = node->item;
    switch (role) {
    case EntityIdRole: return QVariant::fromValue(row.id);
    case ParentIdRole: return QVariant::fromValue(row.parentId);
    case HasParentRole: return row.hasParent;
    case NameRole: return row.name;
    case TypeRole: return row.entityType;
    case TagRole: return row.tag;
    case LayerRole: return row.layer;
    case PositionRole: return QStringLiteral("%1, %2").arg(row.x, 0, 'f', 2).arg(row.y, 0, 'f', 2);
    case SelectedRole: return row.selected;
    case VisibleRole: return row.visible;
    case LockedRole: return row.locked;
    case ComponentCountRole: return row.componentCount;
    case ChildCountRole: return row.childCount;
    default: return {};
    }
}

QHash<int, QByteArray> EntityModel::roleNames() const
{
    return {
        { EntityIdRole, "entityId" },
        { ParentIdRole, "parentId" },
        { HasParentRole, "hasParent" },
        { NameRole, "name" },
        { TypeRole, "entityType" },
        { TagRole, "tag" },
        { LayerRole, "layer" },
        { PositionRole, "position" },
        { SelectedRole, "selected" },
        { VisibleRole, "visible" },
        { LockedRole, "locked" },
        { ComponentCountRole, "componentCount" },
        { ChildCountRole, "childCount" },
    };
}

QString EntityModel::filter() const
{
    return m_filter;
}

void EntityModel::setFilter(const QString& filter)
{
    if (m_filter == filter) {
        return;
    }
    m_filter = filter;
    beginResetModel();
    rebuildTree(filteredRows());
    endResetModel();
    emit filterChanged();
}

void EntityModel::refresh()
{
    beginResetModel();
    m_allRows = m_bridge->entities();
    rebuildTree(filteredRows());
    endResetModel();
}

void EntityModel::updateSelection(qulonglong entityId)
{
    for (const auto& node : m_nodes) {
        const bool selected = node->item.id == entityId;
        if (node->item.selected == selected) {
            continue;
        }
        node->item.selected = selected;
        const QModelIndex changed = indexForNode(node.get());
        if (changed.isValid()) {
            emit dataChanged(changed, changed, { SelectedRole });
        }
    }
}

EntityTreeNode* EntityModel::nodeFromIndex(const QModelIndex& index) const
{
    if (!index.isValid()) {
        return m_root.get();
    }
    return static_cast<EntityTreeNode*>(index.internalPointer());
}

QModelIndex EntityModel::indexForNode(const EntityTreeNode* node) const
{
    if (!node || node == m_root.get() || !node->parent) {
        return {};
    }
    return createIndex(node->row, 0, const_cast<EntityTreeNode*>(node));
}

QVector<MfEntityItem> EntityModel::filteredRows() const
{
    const QString query = m_filter.trimmed();
    if (query.isEmpty()) {
        return m_allRows;
    }

    QHash<quint64, MfEntityItem> byId;
    byId.reserve(m_allRows.size());
    for (const MfEntityItem& row : m_allRows) {
        byId.insert(row.id, row);
    }

    QSet<quint64> included;
    for (const MfEntityItem& row : m_allRows) {
        const bool matches = row.name.contains(query, Qt::CaseInsensitive)
            || row.entityType.contains(query, Qt::CaseInsensitive)
            || row.tag.contains(query, Qt::CaseInsensitive)
            || row.layer.contains(query, Qt::CaseInsensitive)
            || QString::number(row.id).contains(query, Qt::CaseInsensitive);
        if (!matches) {
            continue;
        }

        MfEntityItem current = row;
        while (true) {
            included.insert(current.id);
            if (!current.hasParent || included.contains(current.parentId)) {
                break;
            }
            const auto parent = byId.constFind(current.parentId);
            if (parent == byId.constEnd()) {
                break;
            }
            current = parent.value();
        }
    }

    QVector<MfEntityItem> filtered;
    filtered.reserve(included.size());
    for (const MfEntityItem& row : m_allRows) {
        if (included.contains(row.id)) {
            filtered.push_back(row);
        }
    }
    return filtered;
}

void EntityModel::rebuildTree(const QVector<MfEntityItem>& rows)
{
    m_nodes.clear();
    m_lookup.clear();
    m_root = std::make_unique<EntityTreeNode>();
    m_nodes.reserve(static_cast<size_t>(rows.size()));

    for (const MfEntityItem& row : rows) {
        auto node = std::make_unique<EntityTreeNode>();
        node->item = row;
        m_lookup.insert(row.id, node.get());
        m_nodes.push_back(std::move(node));
    }

    for (const auto& node : m_nodes) {
        EntityTreeNode* parent = m_root.get();
        if (node->item.hasParent) {
            parent = m_lookup.value(node->item.parentId, m_root.get());
            if (parent == node.get()) {
                parent = m_root.get();
            }
        }
        node->parent = parent;
        parent->children.push_back(node.get());
    }

    const auto assignRows = [](const auto& self, EntityTreeNode* parent) -> void {
        for (int row = 0; row < parent->children.size(); ++row) {
            EntityTreeNode* child = parent->children.at(row);
            child->row = row;
            self(self, child);
        }
    };
    assignRows(assignRows, m_root.get());
}

void EntityModel::resetTree()
{
    rebuildTree({});
}

InspectorModel::InspectorModel(MfBridge* bridge, QObject* parent)
    : QAbstractListModel(parent)
    , m_bridge(bridge)
{
    connect(m_bridge, &MfBridge::selectionChanged, this, &InspectorModel::setEntityId);
    connect(m_bridge, &MfBridge::entitiesChanged, this, &InspectorModel::refresh);
}

qulonglong InspectorModel::entityId() const
{
    return m_entityId;
}

void InspectorModel::setEntityId(qulonglong entityId)
{
    if (m_entityId == entityId) {
        return;
    }
    m_entityId = entityId;
    emit entityIdChanged();
    refresh();
}

int InspectorModel::rowCount(const QModelIndex& parent) const
{
    return parent.isValid() ? 0 : static_cast<int>(m_rows.size());
}

QVariant InspectorModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_rows.size()) {
        return {};
    }
    const auto& row = m_rows[index.row()];
    switch (role) {
    case TargetRole: return row.target;
    case EntityIdRole: return QVariant::fromValue(row.entityId);
    case KeyRole: return row.key;
    case DisplayNameRole: return row.displayName;
    case ValueJsonRole: return row.valueJson;
    case ValueTypeRole: return row.valueType;
    case EditableRole: return row.editable;
    default: return {};
    }
}

QHash<int, QByteArray> InspectorModel::roleNames() const
{
    return {
        { TargetRole, "target" },
        { EntityIdRole, "entityId" },
        { KeyRole, "key" },
        { DisplayNameRole, "displayName" },
        { ValueJsonRole, "valueJson" },
        { ValueTypeRole, "valueType" },
        { EditableRole, "editable" },
    };
}

void InspectorModel::refresh()
{
    beginResetModel();
    m_rows = m_entityId == 0 ? QVector<MfInspectorItem> {} : m_bridge->inspectorFields(m_entityId);
    endResetModel();
}

AssetModel::AssetModel(MfBridge* bridge, QObject* parent)
    : QAbstractListModel(parent)
    , m_bridge(bridge)
{
    connect(m_bridge, &MfBridge::assetsChanged, this, &AssetModel::refresh);
}

QString AssetModel::filter() const
{
    return m_filter;
}

void AssetModel::setFilter(const QString& filter)
{
    if (m_filter == filter) {
        return;
    }
    m_filter = filter;
    resetRows();
    emit filterChanged();
}

int AssetModel::rowCount(const QModelIndex& parent) const
{
    return parent.isValid() ? 0 : static_cast<int>(m_rows.size());
}

QVariant AssetModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_rows.size()) {
        return {};
    }
    const auto& row = m_rows[index.row()];
    switch (role) {
    case GuidRole: return row.guid;
    case PathRole: return row.relativePath;
    case NameRole: return row.name;
    case TypeRole: return row.assetType;
    case SizeRole: return QVariant::fromValue(row.sizeBytes);
    case LabelsRole: return row.labels;
    default: return {};
    }
}

QHash<int, QByteArray> AssetModel::roleNames() const
{
    return {
        { GuidRole, "guid" },
        { PathRole, "relativePath" },
        { NameRole, "name" },
        { TypeRole, "assetType" },
        { SizeRole, "sizeBytes" },
        { LabelsRole, "labels" },
    };
}

void AssetModel::refresh()
{
    m_allRows = m_bridge->assets();
    resetRows();
}

QVector<MfAssetItem> AssetModel::filteredRows() const
{
    const QString query = m_filter.trimmed();
    if (query.isEmpty()) {
        return m_allRows;
    }

    QVector<MfAssetItem> filtered;
    filtered.reserve(m_allRows.size());
    for (const MfAssetItem& row : m_allRows) {
        if (row.guid.contains(query, Qt::CaseInsensitive)
            || row.relativePath.contains(query, Qt::CaseInsensitive)
            || row.name.contains(query, Qt::CaseInsensitive)
            || row.assetType.contains(query, Qt::CaseInsensitive)
            || row.labels.contains(query, Qt::CaseInsensitive)) {
            filtered.push_back(row);
        }
    }
    return filtered;
}

void AssetModel::resetRows()
{
    beginResetModel();
    m_rows = filteredRows();
    endResetModel();
}

CommandModel::CommandModel(MfBridge* bridge, QObject* parent)
    : QAbstractListModel(parent)
    , m_bridge(bridge)
{
    connect(m_bridge, &MfBridge::commandsChanged, this, &CommandModel::refresh);
}

QString CommandModel::filter() const
{
    return m_filter;
}

void CommandModel::setFilter(const QString& filter)
{
    if (m_filter == filter) {
        return;
    }
    m_filter = filter;
    resetRows();
    emit filterChanged();
}

int CommandModel::rowCount(const QModelIndex& parent) const
{
    return parent.isValid() ? 0 : static_cast<int>(m_rows.size());
}

QVariant CommandModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_rows.size()) {
        return {};
    }
    const auto& row = m_rows[index.row()];
    switch (role) {
    case IdRole: return row.id;
    case LabelRole: return row.label;
    case CategoryRole: return row.category;
    case ShortcutRole: return row.shortcut;
    case EnabledRole: return row.enabled;
    default: return {};
    }
}

QHash<int, QByteArray> CommandModel::roleNames() const
{
    return {
        { IdRole, "commandId" },
        { LabelRole, "label" },
        { CategoryRole, "category" },
        { ShortcutRole, "shortcut" },
        { EnabledRole, "enabled" },
    };
}

void CommandModel::refresh()
{
    m_allRows = m_bridge->commands();
    resetRows();
}

QVector<MfCommandItem> CommandModel::filteredRows() const
{
    const QString query = m_filter.trimmed();
    if (query.isEmpty()) {
        return m_allRows;
    }

    QVector<MfCommandItem> filtered;
    filtered.reserve(m_allRows.size());
    for (const MfCommandItem& row : m_allRows) {
        if (row.id.contains(query, Qt::CaseInsensitive)
            || row.label.contains(query, Qt::CaseInsensitive)
            || row.category.contains(query, Qt::CaseInsensitive)
            || row.shortcut.contains(query, Qt::CaseInsensitive)) {
            filtered.push_back(row);
        }
    }
    return filtered;
}

void CommandModel::resetRows()
{
    beginResetModel();
    m_rows = filteredRows();
    endResetModel();
}

ConsoleModel::ConsoleModel(MfBridge* bridge, QObject* parent)
    : QAbstractTableModel(parent)
    , m_bridge(bridge)
{
    connect(m_bridge, &MfBridge::consoleChanged, this, &ConsoleModel::refresh);
}

int ConsoleModel::rowCount(const QModelIndex& parent) const
{
    return parent.isValid() ? 0 : static_cast<int>(m_rows.size());
}

int ConsoleModel::columnCount(const QModelIndex& parent) const
{
    return parent.isValid() ? 0 : ColumnCount;
}

QVariant ConsoleModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_rows.size()) {
        return {};
    }
    const auto& row = m_rows[index.row()];
    if (role == Qt::DisplayRole) {
        switch (index.column()) {
        case FrameColumn: return QVariant::fromValue(row.frame);
        case SeverityColumn: return row.severity;
        case ChannelColumn: return row.channel;
        case MessageColumn: return row.message;
        default: return {};
        }
    }
    switch (role) {
    case FrameRole: return QVariant::fromValue(row.frame);
    case SeverityRole: return row.severity;
    case ChannelRole: return row.channel;
    case MessageRole: return row.message;
    default: return {};
    }
}

QVariant ConsoleModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (orientation != Qt::Horizontal || role != Qt::DisplayRole) {
        return {};
    }
    switch (section) {
    case FrameColumn: return QStringLiteral("Frame");
    case SeverityColumn: return QStringLiteral("Severity");
    case ChannelColumn: return QStringLiteral("Channel");
    case MessageColumn: return QStringLiteral("Message");
    default: return {};
    }
}

QHash<int, QByteArray> ConsoleModel::roleNames() const
{
    return {
        { FrameRole, "frame" },
        { SeverityRole, "severity" },
        { ChannelRole, "channel" },
        { MessageRole, "message" },
    };
}

void ConsoleModel::refresh()
{
    beginResetModel();
    m_rows = m_bridge->consoleEntries();
    endResetModel();
}

ReadinessModel::ReadinessModel(MfBridge* bridge, QObject* parent)
    : QAbstractListModel(parent)
    , m_bridge(bridge)
{
    connect(m_bridge, &MfBridge::readinessChanged, this, &ReadinessModel::refresh);
}

int ReadinessModel::score() const
{
    return m_score;
}

int ReadinessModel::rowCount(const QModelIndex& parent) const
{
    return parent.isValid() ? 0 : static_cast<int>(m_rows.size());
}

QVariant ReadinessModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_rows.size()) {
        return {};
    }
    const auto& row = m_rows[index.row()];
    switch (role) {
    case SystemRole: return row.system;
    case LevelRole: return row.level;
    case LevelLabelRole: return row.levelLabel;
    case ScoreRole: return row.score;
    case StrengthCountRole: return row.strengthCount;
    case GapCountRole: return row.gapCount;
    case ActionCountRole: return row.actionCount;
    case TopActionRole: return row.topAction;
    default: return {};
    }
}

QHash<int, QByteArray> ReadinessModel::roleNames() const
{
    return {
        { SystemRole, "system" },
        { LevelRole, "level" },
        { LevelLabelRole, "levelLabel" },
        { ScoreRole, "score" },
        { StrengthCountRole, "strengthCount" },
        { GapCountRole, "gapCount" },
        { ActionCountRole, "actionCount" },
        { TopActionRole, "topAction" },
    };
}

void ReadinessModel::refresh()
{
    const int nextScore = m_bridge->readinessScore();
    beginResetModel();
    m_rows = m_bridge->readinessRows();
    endResetModel();
    if (m_score != nextScore) {
        m_score = nextScore;
        emit scoreChanged();
    }
}

MfEditorController::MfEditorController(MfBridge* bridge, InspectorModel* inspector, QObject* parent)
    : QObject(parent)
    , m_bridge(bridge)
    , m_inspector(inspector)
{
}

void MfEditorController::selectEntity(qulonglong entityId)
{
    if (m_bridge->selectEntity(entityId)) {
        m_inspector->setEntityId(entityId);
    }
}

void MfEditorController::executeCommand(const QString& commandId)
{
    m_bridge->executeCommand(commandId);
}

void MfEditorController::setInspectorValue(qulonglong entityId, const QString& target, const QString& key, const QString& valueJson)
{
    if (m_bridge->setInspectorValueJson(entityId, target, key, valueJson)) {
        m_inspector->setEntityId(entityId);
        m_inspector->refresh();
    }
}
