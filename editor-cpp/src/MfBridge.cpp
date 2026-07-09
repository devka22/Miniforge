#include "MfBridge.h"

#include <QByteArray>
#include <QColor>
#include <QFileInfo>
#include <QSize>

namespace {
QString mfString(const char* value)
{
    return QString::fromUtf8(value ? value : "");
}

QByteArray utf8(const QString& value)
{
    return value.toUtf8();
}
}

MfBridge::MfBridge(QObject* parent)
    : QObject(parent)
{
    MfError error {};
    m_handle = mf_editor_create(&error);
    if (!m_handle) {
        setError(error, QStringLiteral("Failed to create MiniForge editor handle"));
    }
}

MfBridge::~MfBridge()
{
    mf_editor_destroy(m_handle);
    m_handle = nullptr;
}

QString MfBridge::lastError() const
{
    return m_lastError;
}

QString MfBridge::projectPath() const
{
    return m_projectPath;
}

QString MfBridge::projectName() const
{
    if (m_projectPath.isEmpty()) {
        return QStringLiteral("No project");
    }
    const QString fileName = QFileInfo(m_projectPath).fileName();
    return fileName.isEmpty() ? m_projectPath : fileName;
}

QString MfBridge::projectSummary() const
{
    if (!isOpen()) {
        return QStringLiteral("MiniForge 0.9.3.4 | No project");
    }
    return QStringLiteral("MiniForge 0.9.3.4 | %1 | Readiness %2%")
        .arg(projectName())
        .arg(readinessScore());
}

int MfBridge::readinessScore() const
{
    if (!m_handle) {
        return 0;
    }
    MfError error {};
    uint8_t score = 0;
    const MfStatus status = mf_editor_readiness_score(m_handle, &score, &error);
    if (status != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read readiness score"));
        return 0;
    }
    return static_cast<int>(score);
}

bool MfBridge::isOpen() const
{
    return m_handle && mf_editor_is_project_open(m_handle) != 0;
}

bool MfBridge::openProject(const QString& path)
{
    if (!m_handle) {
        m_lastError = QStringLiteral("MiniForge editor handle is not available");
        emit lastErrorChanged();
        return false;
    }

    MfError error {};
    const QByteArray bytes = utf8(path);
    const MfStatus status = mf_editor_open_project(m_handle, bytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to open project"))) {
        return false;
    }
    m_projectPath = readProjectPath();
    emit projectChanged();
    emit entitiesChanged();
    emit assetsChanged();
    emit commandsChanged();
    emit consoleChanged();
    emit readinessChanged();
    emit dataChanged();
    return true;
}

bool MfBridge::selectEntity(qulonglong entityId)
{
    MfError error {};
    const MfStatus status = mf_editor_select_entity(m_handle, static_cast<MfEntityId>(entityId), &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to select entity"))) {
        return false;
    }
    emit selectionChanged(entityId);
    emit dataChanged();
    return true;
}

bool MfBridge::executeCommand(const QString& commandId)
{
    MfError error {};
    const QByteArray bytes = utf8(commandId);
    const MfStatus status = mf_editor_execute_command(m_handle, bytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to execute command"))) {
        return false;
    }
    emit entitiesChanged();
    emit assetsChanged();
    emit commandsChanged();
    emit consoleChanged();
    emit readinessChanged();
    emit dataChanged();
    return true;
}

bool MfBridge::setInspectorValueJson(qulonglong entityId, const QString& target, const QString& key, const QString& valueJson)
{
    MfError error {};
    const QByteArray targetBytes = utf8(target);
    const QByteArray keyBytes = utf8(key);
    const QByteArray valueBytes = utf8(valueJson);
    const MfStatus status = mf_editor_set_inspector_value_json(
        m_handle,
        static_cast<MfEntityId>(entityId),
        targetBytes.constData(),
        keyBytes.constData(),
        valueBytes.constData(),
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to update inspector value"))) {
        return false;
    }
    emit entitiesChanged();
    emit consoleChanged();
    emit dataChanged();
    return true;
}

QVector<MfEntityItem> MfBridge::entities() const
{
    QVector<MfEntityItem> rows;
    MfError error {};
    size_t count = 0;
    if (mf_editor_entity_count(m_handle, &count, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read entities"));
        return rows;
    }
    QVector<MfEntityRow> rawRows(static_cast<qsizetype>(count));
    size_t written = 0;
    size_t total = 0;
    if (count > 0 && mf_editor_entity_rows(m_handle, 0, rawRows.data(), count, &written, &total, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read entity rows"));
        return rows;
    }
    Q_UNUSED(total);
    rows.reserve(static_cast<qsizetype>(written));
    for (size_t index = 0; index < written; ++index) {
        const MfEntityRow& row = rawRows[static_cast<qsizetype>(index)];
        rows.push_back(MfEntityItem {
            row.id,
            row.parent_id,
            row.has_parent != 0,
            row.visible != 0,
            row.enabled != 0,
            row.locked != 0,
            row.selected != 0,
            static_cast<qsizetype>(row.component_count),
            static_cast<qsizetype>(row.child_count),
            row.x,
            row.y,
            mfString(row.name),
            mfString(row.entity_type),
            mfString(row.tag),
            mfString(row.layer),
        });
    }
    return rows;
}

QVector<quint64> MfBridge::selectedEntities() const
{
    QVector<quint64> ids;
    MfError error {};
    size_t count = 0;
    if (mf_editor_selected_entity_count(m_handle, &count, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read selected entities"));
        return ids;
    }
    ids.reserve(static_cast<qsizetype>(count));
    for (size_t index = 0; index < count; ++index) {
        MfEntityId entityId = 0;
        if (mf_editor_selected_entity(m_handle, index, &entityId, &error) != MF_STATUS_OK) {
            setError(error, QStringLiteral("Failed to read selected entity"));
            break;
        }
        ids.push_back(entityId);
    }
    return ids;
}

QVector<MfInspectorItem> MfBridge::inspectorFields(quint64 entityId) const
{
    QVector<MfInspectorItem> fields;
    MfError error {};
    size_t count = 0;
    if (mf_editor_inspector_field_count(m_handle, entityId, &count, &error) != MF_STATUS_OK) {
        return fields;
    }
    QVector<MfInspectorField> rawFields(static_cast<qsizetype>(count));
    size_t written = 0;
    size_t total = 0;
    if (count > 0 && mf_editor_inspector_fields(m_handle, entityId, 0, rawFields.data(), count, &written, &total, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read inspector fields"));
        return fields;
    }
    Q_UNUSED(total);
    fields.reserve(static_cast<qsizetype>(written));
    for (size_t index = 0; index < written; ++index) {
        const MfInspectorField& field = rawFields[static_cast<qsizetype>(index)];
        fields.push_back(MfInspectorItem {
            field.entity_id,
            field.editable != 0,
            mfString(field.target),
            mfString(field.key),
            mfString(field.display_name),
            mfString(field.value_type),
            mfString(field.value_json),
        });
    }
    return fields;
}

QVector<MfAssetItem> MfBridge::assets() const
{
    QVector<MfAssetItem> rows;
    MfError error {};
    size_t count = 0;
    if (mf_editor_asset_count(m_handle, &count, &error) != MF_STATUS_OK) {
        return rows;
    }
    QVector<MfAssetRow> rawRows(static_cast<qsizetype>(count));
    size_t written = 0;
    size_t total = 0;
    if (count > 0 && mf_editor_asset_rows(m_handle, 0, rawRows.data(), count, &written, &total, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read asset rows"));
        return rows;
    }
    Q_UNUSED(total);
    rows.reserve(static_cast<qsizetype>(written));
    for (size_t index = 0; index < written; ++index) {
        const MfAssetRow& row = rawRows[static_cast<qsizetype>(index)];
        rows.push_back(MfAssetItem {
            row.size_bytes,
            static_cast<qsizetype>(row.dependency_count),
            mfString(row.guid),
            mfString(row.relative_path),
            mfString(row.name),
            mfString(row.asset_type),
            mfString(row.labels),
        });
    }
    return rows;
}

QVector<MfCommandItem> MfBridge::commands() const
{
    QVector<MfCommandItem> rows;
    MfError error {};
    size_t count = 0;
    if (mf_editor_command_count(m_handle, &count, &error) != MF_STATUS_OK) {
        return rows;
    }
    QVector<MfCommandDescriptor> rawRows(static_cast<qsizetype>(count));
    size_t written = 0;
    size_t total = 0;
    if (count > 0 && mf_editor_command_descriptors(m_handle, 0, rawRows.data(), count, &written, &total, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read commands"));
        return rows;
    }
    Q_UNUSED(total);
    rows.reserve(static_cast<qsizetype>(written));
    for (size_t index = 0; index < written; ++index) {
        const MfCommandDescriptor& command = rawRows[static_cast<qsizetype>(index)];
        rows.push_back(MfCommandItem {
            command.enabled != 0,
            mfString(command.id),
            mfString(command.label),
            mfString(command.category),
            mfString(command.shortcut),
        });
    }
    return rows;
}

QVector<MfConsoleItem> MfBridge::consoleEntries() const
{
    QVector<MfConsoleItem> rows;
    MfError error {};
    size_t count = 0;
    if (mf_editor_console_count(m_handle, &count, &error) != MF_STATUS_OK) {
        return rows;
    }
    QVector<MfConsoleEntry> rawRows(static_cast<qsizetype>(count));
    size_t written = 0;
    size_t total = 0;
    if (count > 0 && mf_editor_console_entries(m_handle, 0, rawRows.data(), count, &written, &total, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read console entries"));
        return rows;
    }
    Q_UNUSED(total);
    rows.reserve(static_cast<qsizetype>(written));
    for (size_t index = 0; index < written; ++index) {
        const MfConsoleEntry& entry = rawRows[static_cast<qsizetype>(index)];
        rows.push_back(MfConsoleItem {
            entry.frame,
            entry.severity,
            mfString(entry.channel),
            mfString(entry.message),
        });
    }
    return rows;
}

QVector<MfReadinessItem> MfBridge::readinessRows() const
{
    QVector<MfReadinessItem> rows;
    MfError error {};
    size_t count = 0;
    if (mf_editor_readiness_count(m_handle, &count, &error) != MF_STATUS_OK) {
        return rows;
    }
    QVector<MfReadinessRow> rawRows(static_cast<qsizetype>(count));
    size_t written = 0;
    size_t total = 0;
    if (count > 0 && mf_editor_readiness_rows(m_handle, 0, rawRows.data(), count, &written, &total, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read readiness rows"));
        return rows;
    }
    Q_UNUSED(total);
    rows.reserve(static_cast<qsizetype>(written));
    for (size_t index = 0; index < written; ++index) {
        const MfReadinessRow& row = rawRows[static_cast<qsizetype>(index)];
        rows.push_back(MfReadinessItem {
            row.score,
            row.level,
            static_cast<qsizetype>(row.strength_count),
            static_cast<qsizetype>(row.gap_count),
            static_cast<qsizetype>(row.action_count),
            mfString(row.system),
            mfString(row.level_label),
            mfString(row.top_action),
        });
    }
    return rows;
}

QImage MfBridge::viewportImage(const QSize& size) const
{
    const QSize clamped(size.width() > 0 ? size.width() : 1, size.height() > 0 ? size.height() : 1);
    QImage image(clamped, QImage::Format_RGBA8888);
    MfViewportInfo info {};
    MfError error {};
    const MfStatus status = mf_editor_viewport_snapshot_rgba(
        m_handle,
        static_cast<uint32_t>(clamped.width()),
        static_cast<uint32_t>(clamped.height()),
        image.bits(),
        static_cast<size_t>(image.sizeInBytes()),
        &info,
        &error
    );
    if (status != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to render viewport snapshot"));
        image.fill(QColor(22, 25, 32));
    }
    return image;
}

bool MfBridge::setError(const MfError& error, const QString& fallback) const
{
    m_lastError = mfString(error.message);
    if (m_lastError.isEmpty()) {
        m_lastError = fallback;
    }
    emit const_cast<MfBridge*>(this)->lastErrorChanged();
    return false;
}

bool MfBridge::ensureOk(MfStatus status, const MfError& error, const QString& fallback) const
{
    if (status == MF_STATUS_OK) {
        return true;
    }
    return setError(error, fallback);
}

QString MfBridge::readProjectPath() const
{
    char buffer[1024] {};
    size_t required = 0;
    MfError error {};
    const MfStatus status = mf_editor_project_path(m_handle, buffer, sizeof(buffer), &required, &error);
    if (status != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read project path"));
        return {};
    }
    return mfString(buffer);
}
