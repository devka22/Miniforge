#include "MfBridge.h"

#include <algorithm>
#include <utility>

#include <QByteArray>
#include <QColor>
#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QDesktopServices>
#include <QFile>
#include <QFileInfo>
#include <QHash>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QRegularExpression>
#include <QSaveFile>
#include <QSet>
#include <QUrl>
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

QString resolveLuauDeclarationsPath()
{
    const QString applicationDir = QCoreApplication::applicationDirPath();
    QStringList candidates {
        QDir(applicationDir).filePath(QStringLiteral("../Resources/types/miniforge.luau")),
        QDir(applicationDir).filePath(QStringLiteral("../share/miniforge/types/miniforge.luau")),
        QDir(applicationDir).filePath(QStringLiteral("../../../share/miniforge/types/miniforge.luau")),
    };
#ifdef MF_ROOT_PATH
    candidates.push_back(QDir(QString::fromUtf8(MF_ROOT_PATH)).filePath(QStringLiteral("types/miniforge.luau")));
#endif
    for (const QString& candidate : std::as_const(candidates)) {
        const QFileInfo info(candidate);
        if (info.isFile() && !info.isSymLink()) {
            return info.absoluteFilePath();
        }
    }
    return {};
}

QString luauApiCategory(const QString& namespaceName)
{
    static const QHash<QString, QString> categories {
        { QStringLiteral("Vector2"), QStringLiteral("Math") },
        { QStringLiteral("Input"), QStringLiteral("Input") },
        { QStringLiteral("Time"), QStringLiteral("Timing") },
        { QStringLiteral("Layers"), QStringLiteral("Physics") },
        { QStringLiteral("Physics2D"), QStringLiteral("Physics") },
        { QStringLiteral("Rigidbody2D"), QStringLiteral("Physics") },
        { QStringLiteral("CharacterBody2D"), QStringLiteral("Physics") },
        { QStringLiteral("Component"), QStringLiteral("Components") },
        { QStringLiteral("Camera"), QStringLiteral("Camera") },
        { QStringLiteral("AnimationPlayer"), QStringLiteral("Animation") },
        { QStringLiteral("AnimatedSprite"), QStringLiteral("Animation") },
        { QStringLiteral("Tilemap"), QStringLiteral("World") },
        { QStringLiteral("Tween"), QStringLiteral("Animation") },
        { QStringLiteral("Navigation2D"), QStringLiteral("Navigation") },
        { QStringLiteral("Audio2D"), QStringLiteral("Audio") },
        { QStringLiteral("Particles2D"), QStringLiteral("Rendering") },
        { QStringLiteral("Spawner"), QStringLiteral("Entity") },
        { QStringLiteral("Entity"), QStringLiteral("Entity") },
        { QStringLiteral("Transform2D"), QStringLiteral("Transform") },
        { QStringLiteral("Scene"), QStringLiteral("Scene") },
        { QStringLiteral("Game"), QStringLiteral("Game") },
        { QStringLiteral("Events"), QStringLiteral("Events") },
        { QStringLiteral("Assets"), QStringLiteral("Assets") },
        { QStringLiteral("Debug"), QStringLiteral("Debug") },
        { QStringLiteral("Task"), QStringLiteral("Timing") },
    };
    return categories.value(namespaceName, QStringLiteral("Compatibility"));
}

int matchingParenthesis(const QString& value, int openIndex)
{
    int depth = 0;
    for (int index = openIndex; index < value.size(); ++index) {
        if (value.at(index) == u'(') {
            ++depth;
        } else if (value.at(index) == u')' && --depth == 0) {
            return index;
        }
    }
    return -1;
}

QStringList splitLuauParameters(const QString& parameters)
{
    QStringList values;
    int start = 0;
    int roundDepth = 0;
    int braceDepth = 0;
    for (int index = 0; index <= parameters.size(); ++index) {
        const QChar character = index < parameters.size() ? parameters.at(index) : u',';
        if (character == u'(') ++roundDepth;
        else if (character == u')') --roundDepth;
        else if (character == u'{' || character == u'[') ++braceDepth;
        else if (character == u'}' || character == u']') --braceDepth;
        if (character == u',' && roundDepth == 0 && braceDepth == 0) {
            const QString parameter = parameters.mid(start, index - start).trimmed();
            if (!parameter.isEmpty()) values.push_back(parameter);
            start = index + 1;
        }
    }
    return values;
}

QString luauArgumentSample(const QString& rawName, const QString& type)
{
    QString name = rawName.trimmed();
    name.remove(u'?');
    if (name == QStringLiteral("self")) return {};
    if (type.contains(QStringLiteral("->"))) return QStringLiteral("function()\n    \nend");
    if (name.contains(QStringLiteral("target"), Qt::CaseInsensitive)
        || name == QStringLiteral("entity") || name == QStringLiteral("origin")) {
        return QStringLiteral("Entity.current()");
    }
    if (type.contains(QStringLiteral("string"))) {
        QString example = name;
        example.replace(u'_', u' ');
        if (!example.isEmpty()) example[0] = example.at(0).toUpper();
        return QStringLiteral("\"%1\"").arg(example);
    }
    if (type.contains(QStringLiteral("boolean"))) return QStringLiteral("true");
    if (type.contains(QStringLiteral("number"))) return QStringLiteral("0.0");
    if (type.contains(u'{') || name == QStringLiteral("options") || name == QStringLiteral("data")
        || name == QStringLiteral("payload")) {
        return QStringLiteral("{}");
    }
    return name.isEmpty() ? QStringLiteral("value") : name;
}

QString luauInsertText(const QString& label, const QString& type)
{
    const int open = type.indexOf(u'(');
    if (open < 0) return label;
    const int close = matchingParenthesis(type, open);
    if (close < 0) return label;
    QStringList arguments;
    for (const QString& parameter : splitLuauParameters(type.mid(open + 1, close - open - 1))) {
        const int colon = parameter.indexOf(u':');
        const QString name = colon >= 0 ? parameter.left(colon).trimmed() : parameter.trimmed();
        const QString parameterType = colon >= 0 ? parameter.mid(colon + 1).trimmed() : QString();
        const QString sample = luauArgumentSample(name, parameterType);
        if (!sample.isEmpty()) arguments.push_back(sample);
    }
    return QStringLiteral("%1(%2)").arg(label, arguments.join(QStringLiteral(", ")));
}

void appendLuauDeclarationRows(QJsonArray& rows, QSet<QString>& labels)
{
    const QString path = resolveLuauDeclarationsPath();
    QFile file(path);
    if (path.isEmpty() || !file.open(QIODevice::ReadOnly | QIODevice::Text)
        || file.size() > 1024 * 1024) {
        return;
    }
    const QString source = QString::fromUtf8(file.readAll());
    const QRegularExpression tableStart(QStringLiteral("^declare\\s+global\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*:\\s*\\{$"));
    const QRegularExpression alias(QStringLiteral("^declare\\s+global\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*:\\s*(typeof\\([^\\)]+\\)|[A-Za-z_][A-Za-z0-9_]*)$"));
    const QRegularExpression member(QStringLiteral("^([A-Za-z_][A-Za-z0-9_]*)\\s*:\\s*(.+),$"));
    const QRegularExpression function(QStringLiteral("^declare\\s+function\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*(\\(.+)$"));
    QString namespaceName;

    const auto append = [&rows, &labels](const QString& category, const QString& label,
                            const QString& signature, const QString& detail, const QString& insertText) {
        if (label.isEmpty() || labels.contains(label)) return;
        labels.insert(label);
        rows.append(QJsonObject {
            { QStringLiteral("category"), category },
            { QStringLiteral("label"), label },
            { QStringLiteral("signature"), signature },
            { QStringLiteral("detail"), detail },
            { QStringLiteral("insert_text"), insertText },
        });
    };

    const QStringList lines = source.split(u'\n');
    for (const QString& rawLine : lines) {
        const QString line = rawLine.trimmed();
        if (namespaceName.isEmpty()) {
            const auto tableMatch = tableStart.match(line);
            if (tableMatch.hasMatch()) {
                namespaceName = tableMatch.captured(1);
                if (namespaceName == QStringLiteral("miniforge")) namespaceName = QStringLiteral("__skip__");
                continue;
            }
            const auto aliasMatch = alias.match(line);
            if (aliasMatch.hasMatch()) {
                const QString label = aliasMatch.captured(1);
                append(luauApiCategory(label), label,
                    QStringLiteral("%1: %2").arg(label, aliasMatch.captured(2)),
                    QStringLiteral("Public MiniForge Luau alias declared by types/miniforge.luau."), label);
                continue;
            }
            const auto functionMatch = function.match(line);
            if (functionMatch.hasMatch()) {
                const QString label = functionMatch.captured(1);
                const QString type = functionMatch.captured(2);
                append(QStringLiteral("Compatibility"), label, label + type,
                    QStringLiteral("Compatibility helper declared by types/miniforge.luau."),
                    luauInsertText(label, type));
            }
            continue;
        }
        if (line == QStringLiteral("}")) {
            namespaceName.clear();
            continue;
        }
        if (namespaceName == QStringLiteral("__skip__")) continue;
        const auto memberMatch = member.match(line);
        if (!memberMatch.hasMatch()) continue;
        const QString memberName = memberMatch.captured(1);
        const QString type = memberMatch.captured(2).trimmed();
        const QString label = namespaceName + u'.' + memberName;
        const QString signature = type.startsWith(u'(')
            ? label + type
            : QStringLiteral("%1: %2").arg(label, type);
        append(luauApiCategory(namespaceName), label, signature,
            QStringLiteral("Public %1 API declared by types/miniforge.luau.").arg(namespaceName),
            luauInsertText(label, type));
    }
}

bool isSafeRelativePath(const QString& value)
{
    if (value.contains(QChar::Null) || QDir::isAbsolutePath(value)) {
        return false;
    }
    if (value.trimmed().isEmpty()) {
        return true;
    }
    const QString cleaned = QDir::cleanPath(value.trimmed());
    return cleaned == QStringLiteral(".")
        || (!cleaned.isEmpty()
            && cleaned != QStringLiteral("..")
            && !cleaned.startsWith(QStringLiteral("../"))
            && !cleaned.contains(QStringLiteral("/../")));
}

QString projectAbsolutePath(const QString& projectPath, const QString& relativePath)
{
    if (projectPath.isEmpty() || !isSafeRelativePath(relativePath)) {
        return {};
    }
    const QString cleaned = relativePath.trimmed().isEmpty()
        ? QStringLiteral(".")
        : QDir::cleanPath(relativePath.trimmed());
    return QDir(projectPath).absoluteFilePath(cleaned == QStringLiteral(".") ? QString() : cleaned);
}

bool isWithinProject(const QString& projectPath, const QString& absolutePath)
{
    const QString canonicalRoot = QFileInfo(projectPath).canonicalFilePath();
    if (canonicalRoot.isEmpty()) {
        return false;
    }
    QFileInfo candidateInfo(absolutePath);
    QFileInfo anchor = candidateInfo;
    while (!anchor.exists() && anchor.absoluteFilePath() != anchor.absolutePath()) {
        anchor.setFile(anchor.absolutePath());
    }
    QString canonicalCandidate = anchor.canonicalFilePath();
    if (canonicalCandidate.isEmpty()) {
        return false;
    }
    canonicalCandidate = QDir::cleanPath(canonicalCandidate);
    const Qt::CaseSensitivity sensitivity =
#ifdef Q_OS_WIN
        Qt::CaseInsensitive;
#else
        Qt::CaseSensitive;
#endif
    return canonicalCandidate.compare(canonicalRoot, sensitivity) == 0
        || canonicalCandidate.startsWith(canonicalRoot + QDir::separator(), sensitivity);
}

QString normalizedRelativePath(const QString& projectPath, const QString& absolutePath)
{
    return QDir::fromNativeSeparators(QDir(projectPath).relativeFilePath(absolutePath));
}

QString safeAssetStem(const QString& value)
{
    QString output;
    for (const QChar character : value.trimmed()) {
        if (character.isLetterOrNumber() || character == '_' || character == '-') {
            output.append(character);
        } else if (character.isSpace() || character == '.') {
            output.append('_');
        }
    }
    while (output.contains(QStringLiteral("__"))) {
        output.replace(QStringLiteral("__"), QStringLiteral("_"));
    }
    return output.isEmpty() ? QStringLiteral("NewAsset") : output.left(96);
}

}

MfBridge::MfBridge(QObject* parent)
    : QObject(parent)
{
    m_externalProcess = new QProcess(this);
    connect(m_externalProcess, &QProcess::finished, this, [this](int exitCode, QProcess::ExitStatus status) {
        const bool success = status == QProcess::NormalExit && exitCode == 0;
        emit operationCompleted(
            success
                ? QStringLiteral("External game process finished")
                : QStringLiteral("External game process exited with code %1").arg(exitCode),
            success);
        emit dataChanged();
    });
    connect(m_externalProcess, &QProcess::errorOccurred, this, [this](QProcess::ProcessError) {
        setLocalError(QStringLiteral("External game process failed · %1").arg(m_externalProcess->errorString()));
        emit operationCompleted(m_lastError, false);
        emit dataChanged();
    });
    MfError error {};
    m_handle = mf_editor_create(&error);
    if (!m_handle) {
        setError(error, QStringLiteral("Failed to create MiniForge editor handle"));
    }
}

MfBridge::~MfBridge()
{
    if (externalLaunchRunning()) {
        stopExternalLaunch();
    }
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

int MfBridge::entityCount() const
{
    if (!m_handle || !isOpen()) {
        return 0;
    }
    MfError error {};
    size_t count = 0;
    if (mf_editor_entity_count(m_handle, &count, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read entity count"));
        return 0;
    }
    return static_cast<int>(count);
}

int MfBridge::assetCount() const
{
    if (!m_handle || !isOpen()) {
        return 0;
    }
    MfError error {};
    size_t count = 0;
    if (mf_editor_asset_count(m_handle, &count, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read asset count"));
        return 0;
    }
    return static_cast<int>(count);
}

int MfBridge::commandCount() const
{
    if (!m_handle) {
        return 0;
    }
    MfError error {};
    size_t count = 0;
    if (mf_editor_command_count(m_handle, &count, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read command count"));
        return 0;
    }
    return static_cast<int>(count);
}

int MfBridge::consoleCount() const
{
    if (!m_handle || !isOpen()) {
        return 0;
    }
    MfError error {};
    size_t count = 0;
    if (mf_editor_console_count(m_handle, &count, &error) != MF_STATUS_OK) {
        setError(error, QStringLiteral("Failed to read console count"));
        return 0;
    }
    return static_cast<int>(count);
}

int MfBridge::selectedEntityCount() const
{
    return static_cast<int>(selectedEntities().size());
}

qulonglong MfBridge::selectedEntityId() const
{
    const QVector<quint64> selected = selectedEntities();
    return selected.isEmpty() ? 0 : selected.first();
}

bool MfBridge::externalLaunchRunning() const
{
    return m_externalProcess && m_externalProcess->state() != QProcess::NotRunning;
}

QString MfBridge::workbenchSummary() const
{
    if (!isOpen()) {
        return QStringLiteral("No project open");
    }
    return QStringLiteral("%1 entities | %2 assets | %3 commands | %4 console entries")
        .arg(entityCount())
        .arg(assetCount())
        .arg(commandCount())
        .arg(consoleCount());
}

bool MfBridge::isOpen() const
{
    return m_handle && mf_editor_is_project_open(m_handle) != 0;
}

bool MfBridge::openProject(const QString& path)
{
    return openProjectWithOptions(path, QStringLiteral("{}"));
}

bool MfBridge::openProjectWithOptions(const QString& path, const QString& optionsJson)
{
    if (!m_handle) {
        m_lastError = QStringLiteral("MiniForge editor handle is not available");
        emit lastErrorChanged();
        return false;
    }

    MfError error {};
    const QByteArray bytes = utf8(path);
    const QByteArray optionsBytes = utf8(optionsJson.trimmed().isEmpty()
        ? QStringLiteral("{}")
        : optionsJson);
    const MfStatus status = mf_editor_open_project_with_options(
        m_handle,
        bytes.constData(),
        optionsBytes.constData(),
        &error);
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
    emit runtimeHealthChanged();
    emit luauScriptsChanged();
    emit sceneStateChanged();
    emit editorToolChanged(QStringLiteral("sequencer"));
    emit editorToolChanged(QStringLiteral("tilemap"));
    emit editorToolChanged(QStringLiteral("ui_designer"));
    emit prefabChanged();
    emit settingsChanged();
    emit dataChanged();
    return true;
}

bool MfBridge::refreshAll()
{
    MfError error {};
    const MfStatus status = mf_editor_refresh(m_handle, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to refresh the Rust editor state"))) {
        return false;
    }
    emit entitiesChanged();
    emit assetsChanged();
    emit commandsChanged();
    emit consoleChanged();
    emit readinessChanged();
    emit runtimeHealthChanged();
    emit luauScriptsChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    return true;
}

bool MfBridge::selectEntity(qulonglong entityId)
{
    return updateSelection(entityId, QStringLiteral("replace"));
}

bool MfBridge::updateSelection(qulonglong entityId, const QString& mode)
{
    MfError error {};
    const QByteArray modeBytes = utf8(mode);
    const MfStatus status = mf_editor_update_selection(
        m_handle,
        static_cast<MfEntityId>(entityId),
        modeBytes.constData(),
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to select entity"))) {
        return false;
    }
    emit entitiesChanged();
    emit commandsChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    return true;
}

bool MfBridge::clearSelection()
{
    MfError error {};
    if (!ensureOk(
            mf_editor_clear_selection(m_handle, &error),
            error,
            QStringLiteral("Failed to clear selection")
        )) {
        return false;
    }
    emit entitiesChanged();
    emit commandsChanged();
    emit selectionChanged(0);
    emit sceneStateChanged();
    emit dataChanged();
    return true;
}

bool MfBridge::performEntityAction(
    qulonglong entityId,
    const QString& action,
    const QString& payloadJson)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    MfEntityId outEntityId = 0;
    MfError error {};
    const MfStatus status = mf_editor_entity_action(
        m_handle,
        static_cast<MfEntityId>(entityId),
        actionBytes.constData(),
        payloadBytes.constData(),
        &outEntityId,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Entity action failed"))) {
        emit operationCompleted(QStringLiteral("%1 failed · %2").arg(action, m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit commandsChanged();
    emit consoleChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    const QString detail = outEntityId == 0
        ? QStringLiteral("Entity action complete · %1").arg(action)
        : QStringLiteral("Entity action complete · %1 → #%2").arg(action).arg(outEntityId);
    emit operationCompleted(detail, true);
    return true;
}

bool MfBridge::performSelectedEntityAction(const QString& action, const QString& payloadJson)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    size_t changed = 0;
    MfError error {};
    const MfStatus status = mf_editor_selected_entity_action(
        m_handle,
        actionBytes.constData(),
        payloadBytes.constData(),
        &changed,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Selection action failed"))) {
        emit operationCompleted(QStringLiteral("%1 selection failed · %2").arg(action, m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit commandsChanged();
    emit consoleChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    emit operationCompleted(
        QStringLiteral("Selection action complete · %1 · %2 object(s)").arg(action).arg(changed),
        true);
    return true;
}

qulonglong MfBridge::pickEntity(
    int viewportWidth,
    int viewportHeight,
    double x,
    double y,
    const QString& selectionMode)
{
    const QByteArray modeBytes = utf8(selectionMode);
    MfEntityId entityId = 0;
    MfError error {};
    const MfStatus status = mf_editor_pick_entity(
        m_handle,
        static_cast<uint32_t>(qMax(1, viewportWidth)),
        static_cast<uint32_t>(qMax(1, viewportHeight)),
        x,
        y,
        modeBytes.constData(),
        &entityId,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Viewport selection failed"))) {
        return 0;
    }
    emit entitiesChanged();
    emit commandsChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    return entityId;
}

QString MfBridge::viewportStateJson(int viewportWidth, int viewportHeight)
{
    MfError error {};
    size_t required = 0;
    const auto width = static_cast<uint32_t>(qMax(1, viewportWidth));
    const auto height = static_cast<uint32_t>(qMax(1, viewportHeight));
    const MfStatus probeStatus = mf_editor_viewport_state_json(
        m_handle, width, height, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query viewport metadata"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_viewport_state_json(
        m_handle,
        width,
        height,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read viewport metadata"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::transformSelectionJson(const QString& payloadJson)
{
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    size_t changed = 0;
    MfError error {};
    if (!ensureOk(
            mf_editor_transform_selection(m_handle, payloadBytes.constData(), &changed, &error),
            error,
            QStringLiteral("Failed to transform the current selection"))) {
        emit operationCompleted(QStringLiteral("Transform failed · %1").arg(m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Transformed %1 selected object(s)").arg(changed), true);
    return true;
}

QString MfBridge::sceneStateJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_scene_state_json(m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query scene state"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_scene_state_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read scene state"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::sceneBrowserStateJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_scene_browser_state_json(m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Scene Browser"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_scene_browser_state_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Scene Browser"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::sceneBrowserActionJson(const QString& action, const QString& payloadJson)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    QByteArray buffer(64 * 1024, '\0');
    size_t required = 0;
    MfError error {};
    const MfStatus status = mf_editor_scene_browser_action_json(
        m_handle,
        actionBytes.constData(),
        payloadBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Scene Browser action failed"))) {
        emit operationCompleted(QStringLiteral("Scene %1 failed · %2").arg(action, m_lastError), false);
        return {};
    }
    emit entitiesChanged();
    emit assetsChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Scene %1 complete").arg(action), true);
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::componentCatalogJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_component_catalog_json(m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query component catalog"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_component_catalog_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read component catalog"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::authoringCatalogJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_authoring_catalog_json(
        m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query authoring catalog"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_authoring_catalog_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read authoring catalog"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::authoringPlanJson(const QString& presetId, const QString& parametersJson)
{
    const QByteArray presetBytes = utf8(presetId);
    const QByteArray parameterBytes = utf8(parametersJson);
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_authoring_plan_json(
        m_handle,
        presetBytes.constData(),
        parameterBytes.constData(),
        nullptr,
        0,
        &required,
        &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to build authoring application plan"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_authoring_plan_json(
        m_handle,
        presetBytes.constData(),
        parameterBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read authoring application plan"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::sdkPackCatalogJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_sdk_pack_catalog_json(
        m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query SDK pack catalog"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_sdk_pack_catalog_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read SDK pack catalog"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::sdkPackPlanJson(const QString& profileId, const QString& registryJson)
{
    const QByteArray profileBytes = utf8(profileId);
    const QByteArray registryBytes = utf8(registryJson);
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_sdk_pack_plan_json(
        m_handle,
        profileBytes.constData(),
        registryBytes.constData(),
        nullptr,
        0,
        &required,
        &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to build SDK pack installation plan"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_sdk_pack_plan_json(
        m_handle,
        profileBytes.constData(),
        registryBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read SDK pack installation plan"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::toolStateJson(const QString& tool)
{
    const QByteArray toolBytes = utf8(tool);
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_tool_state_json(
        m_handle, toolBytes.constData(), nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query %1 editor state").arg(tool));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_tool_state_json(
        m_handle,
        toolBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read %1 editor state").arg(tool))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::toolActionJson(
    const QString& tool,
    const QString& action,
    const QString& payloadJson)
{
    const QByteArray toolBytes = utf8(tool);
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    QByteArray buffer(4 * 1024 * 1024, '\0');
    size_t required = 0;
    MfError error {};
    const MfStatus status = mf_editor_tool_action_json(
        m_handle,
        toolBytes.constData(),
        actionBytes.constData(),
        payloadBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("%1 action failed").arg(tool))) {
        emit operationCompleted(QStringLiteral("%1 · %2").arg(action, m_lastError), false);
        return {};
    }
    emit editorToolChanged(tool);
    if (action == QStringLiteral("save")) {
        emit assetsChanged();
    }
    emit dataChanged();
    emit operationCompleted(QStringLiteral("%1 · %2 complete").arg(tool, action), true);
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::prefabStateJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_prefab_state_json(m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Prefab Studio state"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_prefab_state_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Prefab Studio state"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::prefabActionJson(const QString& action, const QString& payloadJson)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    QByteArray buffer(8 * 1024, '\0');
    size_t required = 0;
    MfError error {};
    const MfStatus status = mf_editor_prefab_action_json(
        m_handle,
        actionBytes.constData(),
        payloadBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Prefab action failed"))) {
        emit operationCompleted(QStringLiteral("Prefab %1 failed · %2").arg(action, m_lastError), false);
        return {};
    }
    emit prefabChanged();
    emit entitiesChanged();
    emit assetsChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Prefab · %1 complete").arg(action), true);
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::projectSettingsJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_project_settings_json(m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query project settings"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_project_settings_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read project settings"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::saveEngineSettingsJson(const QString& source)
{
    const QByteArray bytes = utf8(source);
    MfError error {};
    if (!ensureOk(mf_editor_save_engine_settings_json(m_handle, bytes.constData(), &error), error,
            QStringLiteral("Failed to save engine settings"))) {
        return false;
    }
    emit settingsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Engine settings saved"), true);
    return true;
}

bool MfBridge::saveInputMapJson(const QString& source)
{
    const QByteArray bytes = utf8(source);
    MfError error {};
    if (!ensureOk(mf_editor_save_input_map_json(m_handle, bytes.constData(), &error), error,
            QStringLiteral("Failed to save Input Map"))) {
        return false;
    }
    emit settingsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Input Map saved"), true);
    return true;
}

bool MfBridge::saveTagsLayersJson(const QString& source)
{
    const QByteArray bytes = utf8(source);
    MfError error {};
    if (!ensureOk(mf_editor_save_tags_layers_json(m_handle, bytes.constData(), &error), error,
            QStringLiteral("Failed to save Tags and Layers"))) {
        return false;
    }
    emit settingsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Tags and Layers saved"), true);
    return true;
}

QString MfBridge::launcherSnapshotJson(const QString& workspaceRoot)
{
    const QByteArray workspaceBytes = utf8(workspaceRoot);
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_launcher_snapshot_json(
        m_handle, workspaceBytes.constData(), nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Project Launcher"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_launcher_snapshot_json(
        m_handle,
        workspaceBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Project Launcher"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::createProject(
    const QString& workspaceRoot,
    const QString& location,
    const QString& name,
    const QString& templateName)
{
    const QByteArray workspaceBytes = utf8(workspaceRoot);
    const QByteArray locationBytes = utf8(location);
    const QByteArray nameBytes = utf8(name);
    const QByteArray templateBytes = utf8(templateName);
    QByteArray buffer(MF_PATH_CAPACITY, '\0');
    size_t required = 0;
    MfError error {};
    const MfStatus status = mf_editor_launcher_create_project(
        m_handle,
        workspaceBytes.constData(),
        locationBytes.constData(),
        nameBytes.constData(),
        templateBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to create project"))) {
        emit operationCompleted(QStringLiteral("Project creation failed · %1").arg(m_lastError), false);
        return {};
    }
    const QString path = QString::fromUtf8(buffer.constData());
    emit operationCompleted(QStringLiteral("Project created · %1").arg(path), true);
    return path;
}

QString MfBridge::repairProjectJson(const QString& workspaceRoot, const QString& projectPath)
{
    const QByteArray workspaceBytes = utf8(workspaceRoot);
    const QByteArray projectBytes = utf8(projectPath);
    QByteArray buffer(64 * 1024, '\0');
    size_t required = 0;
    MfError error {};
    const MfStatus status = mf_editor_launcher_repair_project_json(
        m_handle,
        workspaceBytes.constData(),
        projectBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to repair project"))) {
        emit operationCompleted(QStringLiteral("Project repair failed · %1").arg(m_lastError), false);
        return {};
    }
    emit operationCompleted(QStringLiteral("Project repaired · %1").arg(projectPath), true);
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::projectOperationsJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_project_operations_json(
        m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query project operations"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_project_operations_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read project operations"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::runProjectOperation(const QString& action, const QString& payloadJson)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    MfError error {};
    const MfStatus status = mf_editor_project_operation(
        m_handle, actionBytes.constData(), payloadBytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Project operation failed"))) {
        emit operationCompleted(QStringLiteral("Project operation failed · %1").arg(m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Project operation complete · %1").arg(action), true);
    return true;
}

bool MfBridge::launchPreparedExternal()
{
    const QJsonDocument document = QJsonDocument::fromJson(projectOperationsJson().toUtf8());
    const QJsonObject plan = document.object().value(QStringLiteral("external_launch")).toObject();
    if (plan.isEmpty() || !plan.value(QStringLiteral("ready")).toBool()) {
        return setLocalError(QStringLiteral("No launch-ready external build is prepared"));
    }
    const QString executable = plan.value(QStringLiteral("executable")).toString();
    const QString workingDirectory = plan.value(QStringLiteral("working_directory")).toString();
    QStringList arguments;
    for (const QJsonValue& argument : plan.value(QStringLiteral("arguments")).toArray()) {
        arguments.push_back(argument.toString());
    }
    if (executable.isEmpty() || !QFileInfo::exists(executable)) {
        return setLocalError(QStringLiteral("Prepared runtime executable is missing"));
    }
    if (m_externalProcess->state() != QProcess::NotRunning) {
        stopExternalLaunch();
    }
    m_externalProcess->setProgram(executable);
    m_externalProcess->setArguments(arguments);
    m_externalProcess->setWorkingDirectory(workingDirectory);
    m_externalProcess->setProcessChannelMode(QProcess::MergedChannels);
    m_externalProcess->start();
    if (!m_externalProcess->waitForStarted(1500)) {
        return setLocalError(
            QStringLiteral("Could not start external runtime · %1").arg(m_externalProcess->errorString()));
    }
    emit operationCompleted(
        QStringLiteral("External %1 launched · %2")
            .arg(plan.value(QStringLiteral("kind")).toString(), plan.value(QStringLiteral("artifact_path")).toString()),
        true);
    emit dataChanged();
    return true;
}

bool MfBridge::stopExternalLaunch()
{
    if (!m_externalProcess || m_externalProcess->state() == QProcess::NotRunning) {
        return setLocalError(QStringLiteral("No external game process is running"));
    }
    m_externalProcess->terminate();
    if (!m_externalProcess->waitForFinished(1000)) {
        m_externalProcess->kill();
        m_externalProcess->waitForFinished(1000);
    }
    emit operationCompleted(QStringLiteral("External game process stopped"), true);
    emit dataChanged();
    return true;
}

bool MfBridge::manageAsset(const QString& action, const QString& payloadJson)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson.isEmpty() ? QStringLiteral("{}") : payloadJson);
    MfError error {};
    const MfStatus status = mf_editor_manage_asset(
        m_handle, actionBytes.constData(), payloadBytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Asset operation failed"))) {
        emit operationCompleted(QStringLiteral("Asset %1 failed · %2").arg(action, m_lastError), false);
        return false;
    }
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Asset %1 complete").arg(action), true);
    return true;
}

QString MfBridge::profilerSnapshotJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_profiler_snapshot_json(m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query profiler snapshot"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_profiler_snapshot_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read profiler snapshot"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::rebuildAssetDependencies()
{
    MfError error {};
    if (!ensureOk(
            mf_editor_rebuild_asset_dependencies(m_handle, &error),
            error,
            QStringLiteral("Failed to rebuild asset dependencies"))) {
        emit operationCompleted(QStringLiteral("Dependency rebuild failed · %1").arg(m_lastError), false);
        return false;
    }
    emit assetsChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Asset dependencies rebuilt"), true);
    return true;
}

QString MfBridge::assetDependencyGraphJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_asset_dependency_graph_json(
        m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query asset dependency graph"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_asset_dependency_graph_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read asset dependency graph"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::executeCommand(const QString& commandId)
{
    MfError error {};
    const QByteArray bytes = utf8(commandId);
    const MfStatus status = mf_editor_execute_command(m_handle, bytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to execute command"))) {
        emit operationCompleted(QStringLiteral("%1 · %2").arg(commandId, m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit assetsChanged();
    emit commandsChanged();
    emit consoleChanged();
    emit readinessChanged();
    emit runtimeHealthChanged();
    emit luauScriptsChanged();
    emit sceneStateChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Command complete · %1").arg(commandId), true);
    return true;
}

QString MfBridge::inspectorQuickActionsJson(qulonglong entityId) const
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_inspector_quick_actions_json(
        m_handle,
        static_cast<MfEntityId>(entityId),
        nullptr,
        0,
        &required,
        &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Inspector quick actions"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_inspector_quick_actions_json(
        m_handle,
        static_cast<MfEntityId>(entityId),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Inspector quick actions"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::performInspectorQuickAction(
    qulonglong entityId,
    const QString& action,
    const QString& assetPath)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray pathBytes = utf8(assetPath);
    MfError error {};
    const MfStatus status = mf_editor_inspector_quick_action(
        m_handle,
        static_cast<MfEntityId>(entityId),
        actionBytes.constData(),
        pathBytes.constData(),
        &error);
    if (!ensureOk(status, error, QStringLiteral("Inspector quick action failed"))) {
        emit operationCompleted(QStringLiteral("%1 failed · %2").arg(action, m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit assetsChanged();
    emit commandsChanged();
    emit consoleChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    if (action == QStringLiteral("open_script")) {
        requestOpenContentAsset(assetPath, QStringLiteral("LuauScript"));
    } else if (action == QStringLiteral("open_blueprint")) {
        requestOpenContentAsset(assetPath, QStringLiteral("VisualGraph"));
    }
    emit operationCompleted(QStringLiteral("Inspector quick action complete · %1").arg(action), true);
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
        emit operationCompleted(QStringLiteral("Inspector update failed · %1").arg(m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit consoleChanged();
    emit sceneStateChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Inspector value updated"), true);
    return true;
}

bool MfBridge::setSelectedInspectorValueJson(
    const QString& target,
    const QString& key,
    const QString& valueJson)
{
    MfError error {};
    const QByteArray targetBytes = utf8(target);
    const QByteArray keyBytes = utf8(key);
    const QByteArray valueBytes = utf8(valueJson);
    size_t changed = 0;
    const MfStatus status = mf_editor_set_selected_inspector_value_json(
        m_handle,
        targetBytes.constData(),
        keyBytes.constData(),
        valueBytes.constData(),
        &changed,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to edit the common Inspector property"))) {
        emit operationCompleted(QStringLiteral("Multi-edit failed · %1").arg(m_lastError), false);
        return false;
    }
    emit entitiesChanged();
    emit selectionChanged(selectedEntityId());
    emit sceneStateChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Updated %1 selected object(s)").arg(changed), true);
    return true;
}

QString MfBridge::luauScriptsJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_luau_scripts_json(
        m_handle,
        nullptr,
        0,
        &required,
        &error
    );
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Luau scripts"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_luau_scripts_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read Luau scripts"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::readLuauScript(const QString& relativePath)
{
    const QByteArray pathBytes = utf8(relativePath);
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_luau_read(
        m_handle,
        pathBytes.constData(),
        nullptr,
        0,
        &required,
        &error
    );
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Luau source"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_luau_read(
        m_handle,
        pathBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read Luau source"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::validateLuauSource(const QString& relativePath, const QString& source)
{
    const QByteArray pathBytes = utf8(relativePath);
    const QByteArray sourceBytes = utf8(source);
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_luau_validate_json(
        m_handle,
        pathBytes.constData(),
        sourceBytes.constData(),
        nullptr,
        0,
        &required,
        &error
    );
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to validate Luau source"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_luau_validate_json(
        m_handle,
        pathBytes.constData(),
        sourceBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to validate Luau source"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::saveLuauScript(const QString& relativePath, const QString& source)
{
    const QByteArray pathBytes = utf8(relativePath);
    const QByteArray sourceBytes = utf8(source);
    MfError error {};
    const MfStatus status = mf_editor_luau_save(
        m_handle,
        pathBytes.constData(),
        sourceBytes.constData(),
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to save Luau source"))) {
        emit operationCompleted(QStringLiteral("Luau save failed · %1").arg(m_lastError), false);
        return false;
    }
    emit assetsChanged();
    emit consoleChanged();
    emit luauScriptsChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Saved Luau script · %1").arg(relativePath), true);
    return true;
}

QString MfBridge::luauApiJson() const
{
    QJsonArray rows;
    QSet<QString> labels;
    const auto add = [&rows, &labels](const QString& category, const QString& label, const QString& signature,
                         const QString& detail, const QString& insertText) {
        if (labels.contains(label)) {
            return;
        }
        labels.insert(label);
        rows.append(QJsonObject {
            { QStringLiteral("category"), category },
            { QStringLiteral("label"), label },
            { QStringLiteral("signature"), signature },
            { QStringLiteral("detail"), detail },
            { QStringLiteral("insert_text"), insertText },
        });
    };

    add(QStringLiteral("Lifecycle"), QStringLiteral("on_ready"), QStringLiteral("function Script:on_ready()"),
        QStringLiteral("Runs once after creation and before frame updates."),
        QStringLiteral("function Script:on_ready()\n    \nend"));
    add(QStringLiteral("Lifecycle"), QStringLiteral("on_update"), QStringLiteral("function Script:on_update(dt: number)"),
        QStringLiteral("Runs every scheduled frame. Multiply rates by dt."),
        QStringLiteral("function Script:on_update(dt: number)\n    \nend"));
    add(QStringLiteral("Lifecycle"), QStringLiteral("on_fixed_update"), QStringLiteral("function Script:on_fixed_update(dt: number)"),
        QStringLiteral("Runs on deterministic physics steps."),
        QStringLiteral("function Script:on_fixed_update(dt: number)\n    \nend"));
    add(QStringLiteral("Lifecycle"), QStringLiteral("on_event"), QStringLiteral("function Script:on_event(name: string, payload: any)"),
        QStringLiteral("Receives Events.emit payloads."),
        QStringLiteral("function Script:on_event(name: string, payload: any)\n    \nend"));
    add(QStringLiteral("Entity"), QStringLiteral("Entity.current"), QStringLiteral("Entity.current() -> EntityHandle"),
        QStringLiteral("Returns the current script entity handle."), QStringLiteral("Entity.current()"));
    add(QStringLiteral("Entity"), QStringLiteral("Entity.spawn"), QStringLiteral("Entity.spawn(name, x, y, data?) -> number"),
        QStringLiteral("Queues an entity and immediately returns its stable id."),
        QStringLiteral("Entity.spawn(\"EntityName\", x, y, { tag = \"Gameplay\" })"));
    add(QStringLiteral("Entity"), QStringLiteral("Entity.find"), QStringLiteral("Entity.find(target) -> Entity?"),
        QStringLiteral("Indexed lookup by id, name, or entity handle."), QStringLiteral("Entity.find(\"EntityName\")"));
    add(QStringLiteral("Entity"), QStringLiteral("Entity.nearby"), QStringLiteral("Entity.nearby(origin, radius, options?) -> {Entity}"),
        QStringLiteral("Spatially indexed radius query with tag/layer filters."),
        QStringLiteral("Entity.nearby(Entity.current(), 12.0, { tag = \"Enemy\" })"));
    add(QStringLiteral("Entity"), QStringLiteral("Entity.nearest"), QStringLiteral("Entity.nearest(origin, radius, options?) -> Entity?"),
        QStringLiteral("Returns the closest indexed match and excludes the origin by default."),
        QStringLiteral("Entity.nearest(Entity.current(), 20.0, { tag = \"Enemy\" })"));
    add(QStringLiteral("Entity"), QStringLiteral("Entity.exists"), QStringLiteral("Entity.exists(target) -> boolean"),
        QStringLiteral("Fast existence check without allocating an entity table."), QStringLiteral("Entity.exists(\"EntityName\")"));
    add(QStringLiteral("Components"), QStringLiteral("Component.add"), QStringLiteral("Component.add(target, component, data?)"),
        QStringLiteral("Queues a component addition."), QStringLiteral("Component.add(Entity.current(), \"Health\", {})"));
    add(QStringLiteral("Components"), QStringLiteral("Component.get"), QStringLiteral("Component.get(target, component, key, default?) -> any"),
        QStringLiteral("Reads a component field from the current world snapshot."),
        QStringLiteral("Component.get(Entity.current(), \"Health\", \"health\", 0)"));
    add(QStringLiteral("Components"), QStringLiteral("Component.set"), QStringLiteral("Component.set(target, component, key, value)"),
        QStringLiteral("Queues a component field update."),
        QStringLiteral("Component.set(Entity.current(), \"Health\", \"health\", value)"));
    add(QStringLiteral("Components"), QStringLiteral("Component.remove"), QStringLiteral("Component.remove(target, component)"),
        QStringLiteral("Queues removal of a component if present."),
        QStringLiteral("Component.remove(Entity.current(), \"ComponentName\")"));
    add(QStringLiteral("Input"), QStringLiteral("Input.axis"), QStringLiteral("Input.axis(negative, positive) -> number"),
        QStringLiteral("Returns -1, 0, or 1 from named input actions."), QStringLiteral("Input.axis(\"move_left\", \"move_right\")"));
    add(QStringLiteral("Input"), QStringLiteral("Input.action_pressed"), QStringLiteral("Input.action_pressed(action) -> boolean"),
        QStringLiteral("Tests an action and its built-in keyboard aliases."), QStringLiteral("Input.action_pressed(\"jump\")"));
    add(QStringLiteral("Math"), QStringLiteral("Vector2.new"), QStringLiteral("Vector2.new(x?, y?) -> Vector2"),
        QStringLiteral("Creates a serializable 2D vector table."), QStringLiteral("Vector2.new(0.0, 0.0)"));
    add(QStringLiteral("Math"), QStringLiteral("Vector2.distance"), QStringLiteral("Vector2.distance(a, b) -> number"),
        QStringLiteral("Euclidean distance between two vectors."), QStringLiteral("Vector2.distance(a, b)"));
    add(QStringLiteral("Math"), QStringLiteral("Vector2.move_towards"), QStringLiteral("Vector2.move_towards(current, target, max_delta) -> Vector2"),
        QStringLiteral("Moves without overshooting and handles zero distance."),
        QStringLiteral("Vector2.move_towards(current, target, speed * dt)"));
    add(QStringLiteral("Physics"), QStringLiteral("Physics2D.raycast"), QStringLiteral("Physics2D.raycast(origin, target, options?) -> RaycastHit?"),
        QStringLiteral("Filtered raycast against the frame world snapshot."),
        QStringLiteral("Physics2D.raycast(origin, target, { mask = Layers.WORLD })"));
    add(QStringLiteral("Physics"), QStringLiteral("Rigidbody2D.set_velocity"), QStringLiteral("Rigidbody2D.set_velocity(target, x, y)"),
        QStringLiteral("Sets velocity and wakes the body."),
        QStringLiteral("Rigidbody2D.set_velocity(Entity.current(), x, y)"));
    add(QStringLiteral("Physics"), QStringLiteral("Rigidbody2D.apply_impulse"), QStringLiteral("Rigidbody2D.apply_impulse(target, x, y)"),
        QStringLiteral("Applies an instantaneous physics impulse."),
        QStringLiteral("Rigidbody2D.apply_impulse(Entity.current(), 0.0, -240.0)"));
    add(QStringLiteral("Rendering"), QStringLiteral("AnimationPlayer.play"), QStringLiteral("AnimationPlayer.play(target, animation)"),
        QStringLiteral("Starts an animation on the selected target."),
        QStringLiteral("AnimationPlayer.play(Entity.current(), \"Run\")"));
    add(QStringLiteral("Rendering"), QStringLiteral("Particles2D.burst"), QStringLiteral("Particles2D.burst(target, count)"),
        QStringLiteral("Requests a one-shot particle burst."), QStringLiteral("Particles2D.burst(Entity.current(), 16)"));
    add(QStringLiteral("Camera"), QStringLiteral("Camera.main"), QStringLiteral("Camera.main() -> CameraHandle"),
        QStringLiteral("Returns the main camera command handle."), QStringLiteral("Camera.main():follow(Entity.current())"));
    add(QStringLiteral("Navigation"), QStringLiteral("Navigation2D.set_destination"), QStringLiteral("Navigation2D.set_destination(target, x, y)"),
        QStringLiteral("Queues a navigation destination."),
        QStringLiteral("Navigation2D.set_destination(Entity.current(), x, y)"));
    add(QStringLiteral("Timing"), QStringLiteral("Task.delay"), QStringLiteral("Task.delay(seconds, callback) -> TaskHandle"),
        QStringLiteral("Schedules a callback in the persistent script context."),
        QStringLiteral("Task.delay(1.0, function()\n    \nend)"));
    add(QStringLiteral("Events"), QStringLiteral("Events.emit"), QStringLiteral("Events.emit(name, payload?)"),
        QStringLiteral("Dispatches a structured event after the callback."),
        QStringLiteral("Events.emit(\"EventName\", { source = entity_name })"));
    add(QStringLiteral("Debug"), QStringLiteral("Debug.log"), QStringLiteral("Debug.log(message)"),
        QStringLiteral("Writes an info message to the MiniForge console."), QStringLiteral("Debug.log(\"message\")"));

    appendLuauDeclarationRows(rows, labels);

    return QString::fromUtf8(QJsonDocument(rows).toJson(QJsonDocument::Compact));
}

QString MfBridge::luauDebugStateJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_luau_debug_state_json(m_handle, nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Luau debugger state"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_luau_debug_state_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Luau debugger state"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::setLuauBreakpointsJson(const QString& breakpointsJson)
{
    MfError error {};
    const QByteArray bytes = breakpointsJson.toUtf8();
    const MfStatus status = mf_editor_luau_set_breakpoints_json(m_handle, bytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to update Luau breakpoints"))) {
        return false;
    }
    emit luauDebuggerChanged();
    return true;
}

bool MfBridge::luauDebugCommand(const QString& command)
{
    MfError error {};
    const QByteArray bytes = command.toUtf8();
    const MfStatus status = mf_editor_luau_debug_command(m_handle, bytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Luau debugger command failed"))) {
        return false;
    }
    emit luauDebuggerChanged();
    emit operationCompleted(QStringLiteral("Luau debugger · %1").arg(command), true);
    return true;
}

QString MfBridge::luauWatchesJson(const QString& expressionsJson)
{
    MfError error {};
    const QByteArray expressions = expressionsJson.toUtf8();
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_luau_watches_json(
        m_handle, expressions.constData(), nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Luau watches"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_luau_watches_json(
        m_handle, expressions.constData(), buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to evaluate Luau watches"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::workspaceStateJson()
{
    if (!isOpen()) {
        setLocalError(QStringLiteral("Open a project before restoring workspace state"));
        return QStringLiteral("{}");
    }
    const QString path = QDir(m_projectPath).filePath(QStringLiteral(".miniforge/qt_workspace.json"));
    QFile file(path);
    if (!file.exists()) {
        ensureOk(MF_STATUS_OK, MfError {}, {});
        return QStringLiteral("{\"tabs\":[],\"breakpoints\":[],\"watches\":[],\"active\":\"\"}");
    }
    if (!file.open(QIODevice::ReadOnly) || file.size() > 1024 * 1024) {
        setLocalError(QStringLiteral("Failed to read Qt workspace recovery state"));
        return QStringLiteral("{}");
    }
    const QByteArray data = file.readAll();
    QJsonParseError parseError {};
    const QJsonDocument document = QJsonDocument::fromJson(data, &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        setLocalError(QStringLiteral("Qt workspace recovery state is invalid JSON"));
        return QStringLiteral("{}");
    }
    ensureOk(MF_STATUS_OK, MfError {}, {});
    return QString::fromUtf8(document.toJson(QJsonDocument::Compact));
}

bool MfBridge::saveWorkspaceState(const QString& stateJson)
{
    if (!isOpen()) {
        return setLocalError(QStringLiteral("Open a project before saving workspace state"));
    }
    if (stateJson.toUtf8().size() > 1024 * 1024) {
        return setLocalError(QStringLiteral("Workspace state exceeds the 1 MiB limit"));
    }
    QJsonParseError parseError {};
    const QJsonDocument document = QJsonDocument::fromJson(stateJson.toUtf8(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return setLocalError(QStringLiteral("Workspace state must be a JSON object"));
    }
    const QString directory = QDir(m_projectPath).filePath(QStringLiteral(".miniforge"));
    if (!QDir().mkpath(directory)) {
        return setLocalError(QStringLiteral("Failed to create workspace recovery folder"));
    }
    QSaveFile file(QDir(directory).filePath(QStringLiteral("qt_workspace.json")));
    const QByteArray bytes = document.toJson(QJsonDocument::Indented);
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size() || !file.commit()) {
        return setLocalError(QStringLiteral("Failed to save workspace recovery state atomically"));
    }
    ensureOk(MF_STATUS_OK, MfError {}, {});
    return true;
}

QString MfBridge::visualGraphCatalogJson() const
{
    MfError error {};
    size_t required = 0;
    const MfStatus probe = mf_editor_visual_graph_catalog_json(
        m_handle, nullptr, 0, &required, &error);
    if (probe != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Visual Graph catalog"));
        return QStringLiteral("{}");
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_visual_graph_catalog_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Visual Graph catalog"))) {
        return QStringLiteral("{}");
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::createVisualGraphTemplate(const QString& name, const QString& templateName)
{
    const QString stem = safeAssetStem(name);
    const QString relativePath = QStringLiteral("scripts/visual_graphs/%1.mfgraph").arg(stem);
    const QByteArray path = relativePath.toUtf8();
    const QByteArray templateBytes = templateName.toUtf8();
    MfError error {};
    if (!ensureOk(
            mf_editor_visual_graph_create_template(
                m_handle, path.constData(), templateBytes.constData(), &error),
            error,
            QStringLiteral("Failed to create Visual Graph template"))) {
        return {};
    }
    emit assetsChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Created Visual Graph · %1").arg(relativePath), true);
    return relativePath;
}

QString MfBridge::validateVisualGraph(const QString& relativePath, const QString& source)
{
    const QByteArray pathBytes = relativePath.toUtf8();
    const QByteArray sourceBytes = source.toUtf8();
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_visual_graph_validate_json(
        m_handle, pathBytes.constData(), sourceBytes.constData(), nullptr, 0, &required, &error);
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Visual Graph validation failed"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_visual_graph_validate_json(
        m_handle,
        pathBytes.constData(),
        sourceBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Visual Graph validation failed"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::saveVisualGraph(const QString& relativePath, const QString& source)
{
    const QByteArray pathBytes = relativePath.toUtf8();
    const QByteArray sourceBytes = source.toUtf8();
    MfError error {};
    const MfStatus status = mf_editor_visual_graph_save(
        m_handle, pathBytes.constData(), sourceBytes.constData(), &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to save Visual Graph"))) {
        emit operationCompleted(QStringLiteral("Visual Graph save failed · %1").arg(m_lastError), false);
        return false;
    }
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Saved Visual Graph · %1").arg(relativePath), true);
    return true;
}

QString MfBridge::pythonToolsJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probe = mf_editor_python_tools_json(m_handle, nullptr, 0, &required, &error);
    if (probe != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Python tools"));
        return QStringLiteral("{}");
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_python_tools_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Python tools"))) {
        return QStringLiteral("{}");
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::installPythonTools()
{
    MfError error {};
    if (!ensureOk(
            mf_editor_python_install_tools(m_handle, &error),
            error,
            QStringLiteral("Failed to install Python tools"))) {
        return false;
    }
    emit pythonToolsChanged();
    emit consoleChanged();
    emit operationCompleted(QStringLiteral("Python automation tools installed"), true);
    return true;
}

QString MfBridge::runPythonTool(const QString& toolId, const QString& parametersJson)
{
    const QByteArray tool = toolId.toUtf8();
    const QByteArray parameters = parametersJson.toUtf8();
    MfError error {};
    if (!ensureOk(
            mf_editor_python_run_tool(m_handle, tool.constData(), parameters.constData(), &error),
            error,
            QStringLiteral("Python tool failed"))) {
        return {};
    }
    size_t required = 0;
    const MfStatus probe = mf_editor_python_last_result_json(
        m_handle, nullptr, 0, &required, &error);
    if (probe != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Python result"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_python_last_result_json(
        m_handle, buffer.data(), static_cast<size_t>(buffer.size()), &required, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read Python result"))) {
        return {};
    }
    emit pythonToolsChanged();
    emit entitiesChanged();
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::openExternalEditor(const QString& relativePath, const QString& command)
{
    if (!isOpen() || !isSafeRelativePath(relativePath)) {
        return setLocalError(QStringLiteral("External editor path must stay inside the project"));
    }
    const QString path = projectAbsolutePath(m_projectPath, relativePath);
    const QFileInfo info(path);
    if (!isWithinProject(m_projectPath, path) || !info.isFile() || info.isSymLink()) {
        return setLocalError(QStringLiteral("External editor target must be a regular project file"));
    }
    const QString executable = command.trimmed();
    const bool opened = executable.isEmpty()
        ? QDesktopServices::openUrl(QUrl::fromLocalFile(path))
        : QProcess::startDetached(executable, QStringList { path });
    if (!opened) {
        return setLocalError(QStringLiteral("Failed to launch external editor"));
    }
    ensureOk(MF_STATUS_OK, MfError {}, {});
    emit operationCompleted(QStringLiteral("Opened external editor · %1").arg(relativePath), true);
    return true;
}

QString MfBridge::contentEntriesJson(const QString& relativeDirectory)
{
    const QByteArray directoryBytes = utf8(relativeDirectory);
    MfError error {};
    size_t required = 0;
    const MfStatus probe = mf_editor_content_entries_json(
        m_handle, directoryBytes.constData(), nullptr, 0, &required, &error);
    if (probe != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query content entries"));
        return QStringLiteral("[]");
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_content_entries_json(
        m_handle,
        directoryBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read content entries"))) {
        return QStringLiteral("[]");
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::contentFoldersJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probe = mf_editor_content_folders_json(
        m_handle, nullptr, 0, &required, &error);
    if (probe != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query content folders"));
        return QStringLiteral("[]");
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_content_folders_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read content folders"))) {
        return QStringLiteral("[]");
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::createContentFolder(const QString& relativeDirectory, const QString& name)
{
    const QByteArray directoryBytes = utf8(relativeDirectory);
    const QByteArray nameBytes = utf8(name);
    MfError error {};
    if (!ensureOk(
            mf_editor_content_create_folder(
                m_handle, directoryBytes.constData(), nameBytes.constData(), &error),
            error,
            QStringLiteral("Failed to create content folder"))) {
        return false;
    }
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(
        QStringLiteral("Created folder · %1/%2").arg(relativeDirectory, name.trimmed()),
        true);
    return true;
}

QString MfBridge::createContentFile(const QString& kind, const QString& relativeDirectory, const QString& name)
{
    const QByteArray kindBytes = utf8(kind);
    const QByteArray directoryBytes = utf8(relativeDirectory);
    const QByteArray nameBytes = utf8(name);
    MfContentMutationResult result {};
    MfError error {};
    if (!ensureOk(
            mf_editor_content_create_file(
                m_handle,
                kindBytes.constData(),
                directoryBytes.constData(),
                nameBytes.constData(),
                &result,
                &error),
            error,
            QStringLiteral("Failed to create content asset"))) {
        return {};
    }
    const QString relativePath = mfString(result.relative_path);
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Created asset · %1").arg(relativePath), true);
    return relativePath;
}

QString MfBridge::readTextAsset(const QString& relativePath)
{
    const QByteArray pathBytes = utf8(relativePath);
    MfError error {};
    size_t required = 0;
    const MfStatus probe = mf_editor_content_read_text(
        m_handle, pathBytes.constData(), nullptr, 0, &required, &error);
    if (probe != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query text asset"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_content_read_text(
        m_handle,
        pathBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to read text asset"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::saveTextAsset(const QString& relativePath, const QString& source)
{
    const QByteArray pathBytes = utf8(relativePath);
    const QByteArray sourceBytes = utf8(source);
    MfError error {};
    if (!ensureOk(
            mf_editor_content_save_text(
                m_handle, pathBytes.constData(), sourceBytes.constData(), &error),
            error,
            QStringLiteral("Failed to save text asset"))) {
        return false;
    }
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Saved text asset · %1").arg(relativePath), true);
    return true;
}

void MfBridge::requestOpenContentAsset(const QString& relativePath, const QString& assetType)
{
    if (!isOpen() || !isSafeRelativePath(relativePath)) {
        setLocalError(QStringLiteral("Cannot open an asset outside the project"));
        return;
    }
    emit contentAssetOpenRequested(QDir::fromNativeSeparators(relativePath), assetType);
}

QString MfBridge::exportRuntime(const QString& profile)
{
    const QByteArray profileBytes = utf8(profile);
    MfError error {};
    const MfStatus exportStatus = mf_editor_export_runtime(
        m_handle,
        profileBytes.constData(),
        &error
    );
    if (!ensureOk(exportStatus, error, QStringLiteral("Runtime export failed"))) {
        emit operationCompleted(QStringLiteral("Runtime export failed · %1").arg(m_lastError), false);
        return {};
    }

    size_t required = 0;
    const MfStatus probeStatus = mf_editor_last_export_report_json(
        m_handle,
        nullptr,
        0,
        &required,
        &error
    );
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query runtime export report"));
        return {};
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus reportStatus = mf_editor_last_export_report_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(reportStatus, error, QStringLiteral("Failed to read runtime export report"))) {
        return {};
    }
    emit assetsChanged();
    emit consoleChanged();
    emit readinessChanged();
    emit exportCompleted();
    emit dataChanged();
    emit operationCompleted(QStringLiteral("Runtime export complete · %1").arg(profile), true);
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::runtimeHealthJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_runtime_health_json(
        m_handle,
        nullptr,
        0,
        &required,
        &error
    );
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query runtime health"));
        return {};
    }

    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_runtime_health_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read runtime health"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::forgeAiDiagnosticsJson()
{
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_forge_ai_diagnostics_json(
        m_handle,
        nullptr,
        0,
        &required,
        &error
    );
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query Forge AI diagnostics"));
        return {};
    }

    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_forge_ai_diagnostics_json(
        m_handle,
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read Forge AI diagnostics"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
}

QString MfBridge::runForgeAiTestJson(const QString& suiteId)
{
    const QByteArray suiteBytes = utf8(suiteId);
    MfError error {};
    size_t required = 0;
    const MfStatus probeStatus = mf_editor_forge_ai_run_test_json(
        m_handle,
        suiteBytes.constData(),
        nullptr,
        0,
        &required,
        &error
    );
    if (probeStatus != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to query the Forge AI test report"));
        return {};
    }

    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_forge_ai_run_test_json(
        m_handle,
        suiteBytes.constData(),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to run the Forge AI test"))) {
        return {};
    }
    return QString::fromUtf8(buffer.constData());
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
            false,
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

QImage MfBridge::spriteImage(MfSpriteInfo* info) const
{
    MfSpriteInfo spriteInfo {};
    MfError error {};
    MfStatus status = mf_editor_sprite_snapshot_rgba(
        m_handle,
        nullptr,
        0,
        &spriteInfo,
        &error
    );
    if (status != MF_STATUS_BUFFER_TOO_SMALL || spriteInfo.required_bytes == 0) {
        setError(error, QStringLiteral("Failed to query sprite canvas"));
        return {};
    }
    QImage image(
        static_cast<int>(spriteInfo.width),
        static_cast<int>(spriteInfo.height),
        QImage::Format_RGBA8888
    );
    status = mf_editor_sprite_snapshot_rgba(
        m_handle,
        image.bits(),
        static_cast<size_t>(image.sizeInBytes()),
        &spriteInfo,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read sprite canvas"))) {
        return {};
    }
    if (info) {
        *info = spriteInfo;
    }
    return image;
}

bool MfBridge::newSpriteCanvas(int width, int height)
{
    MfError error {};
    const MfStatus status = mf_editor_sprite_new_canvas(
        m_handle,
        static_cast<uint32_t>(width),
        static_cast<uint32_t>(height),
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to create sprite canvas"))) {
        return false;
    }
    emit spriteChanged();
    return true;
}

bool MfBridge::beginSpriteEdit()
{
    MfError error {};
    return ensureOk(
        mf_editor_sprite_begin_edit(m_handle, &error),
        error,
        QStringLiteral("Failed to begin sprite edit")
    );
}

bool MfBridge::setSpritePixel(int x, int y, const QColor& color)
{
    MfError error {};
    uint8_t changed = 0;
    const MfStatus status = mf_editor_sprite_set_pixel(
        m_handle,
        static_cast<uint32_t>(x),
        static_cast<uint32_t>(y),
        static_cast<uint8_t>(color.red()),
        static_cast<uint8_t>(color.green()),
        static_cast<uint8_t>(color.blue()),
        static_cast<uint8_t>(color.alpha()),
        &changed,
        &error
    );
    return ensureOk(status, error, QStringLiteral("Failed to paint sprite pixel")) && changed != 0;
}

bool MfBridge::clearSprite(const QColor& color)
{
    MfError error {};
    return ensureOk(
        mf_editor_sprite_clear(
            m_handle,
            static_cast<uint8_t>(color.red()),
            static_cast<uint8_t>(color.green()),
            static_cast<uint8_t>(color.blue()),
            static_cast<uint8_t>(color.alpha()),
            &error
        ),
        error,
        QStringLiteral("Failed to clear sprite canvas")
    );
}

bool MfBridge::transformSprite(const QString& action, const QString& payloadJson)
{
    const QByteArray actionBytes = utf8(action);
    const QByteArray payloadBytes = utf8(payloadJson);
    MfError error {};
    uint8_t changed = 0;
    const MfStatus status = mf_editor_sprite_transform(
        m_handle,
        actionBytes.constData(),
        payloadBytes.constData(),
        &changed,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to transform sprite"))) {
        return false;
    }
    if (changed != 0) {
        emit spriteChanged();
        emit dataChanged();
    }
    return changed != 0;
}

QString MfBridge::spriteAnimationClipJson(int frameWidth, int frameHeight, double fps) const
{
    MfError error {};
    size_t required = 0;
    const MfStatus probe = mf_editor_sprite_animation_clip_json(
        m_handle,
        static_cast<uint32_t>(std::max(0, frameWidth)),
        static_cast<uint32_t>(std::max(0, frameHeight)),
        static_cast<float>(fps),
        nullptr,
        0,
        &required,
        &error
    );
    if (probe != MF_STATUS_BUFFER_TOO_SMALL || required == 0) {
        setError(error, QStringLiteral("Failed to prepare sprite animation timeline"));
        return QStringLiteral("{}");
    }
    QByteArray buffer(static_cast<qsizetype>(required), '\0');
    const MfStatus status = mf_editor_sprite_animation_clip_json(
        m_handle,
        static_cast<uint32_t>(frameWidth),
        static_cast<uint32_t>(frameHeight),
        static_cast<float>(fps),
        buffer.data(),
        static_cast<size_t>(buffer.size()),
        &required,
        &error
    );
    if (!ensureOk(status, error, QStringLiteral("Failed to read sprite animation timeline"))) {
        return QStringLiteral("{}");
    }
    return QString::fromUtf8(buffer.constData());
}

bool MfBridge::commitSpriteEdit()
{
    MfError error {};
    uint8_t changed = 0;
    const MfStatus status = mf_editor_sprite_commit_edit(m_handle, &changed, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to commit sprite edit"))) {
        return false;
    }
    if (changed != 0) {
        emit spriteChanged();
    }
    return changed != 0;
}

bool MfBridge::undoSprite()
{
    MfError error {};
    uint8_t changed = 0;
    const MfStatus status = mf_editor_sprite_undo(m_handle, &changed, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to undo sprite edit"))) {
        return false;
    }
    if (changed != 0) {
        emit spriteChanged();
    }
    return changed != 0;
}

bool MfBridge::redoSprite()
{
    MfError error {};
    uint8_t changed = 0;
    const MfStatus status = mf_editor_sprite_redo(m_handle, &changed, &error);
    if (!ensureOk(status, error, QStringLiteral("Failed to redo sprite edit"))) {
        return false;
    }
    if (changed != 0) {
        emit spriteChanged();
    }
    return changed != 0;
}

QString MfBridge::saveSprite(const QString& fallbackName)
{
    const QByteArray name = utf8(fallbackName);
    char buffer[MF_PATH_CAPACITY] {};
    size_t required = 0;
    MfError error {};
    const MfStatus status = mf_editor_sprite_save(
        m_handle,
        name.constData(),
        buffer,
        sizeof(buffer),
        &required,
        &error
    );
    Q_UNUSED(required);
    if (!ensureOk(status, error, QStringLiteral("Failed to save sprite canvas"))) {
        emit operationCompleted(QStringLiteral("Sprite save failed · %1").arg(m_lastError), false);
        return {};
    }
    emit assetsChanged();
    emit consoleChanged();
    emit dataChanged();
    const QString path = mfString(buffer);
    emit operationCompleted(QStringLiteral("Saved sprite · %1").arg(path), true);
    return path;
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

bool MfBridge::setLocalError(const QString& message) const
{
    m_lastError = message;
    emit const_cast<MfBridge*>(this)->lastErrorChanged();
    return false;
}

bool MfBridge::ensureOk(MfStatus status, const MfError& error, const QString& fallback) const
{
    if (status == MF_STATUS_OK) {
        if (!m_lastError.isEmpty()) {
            m_lastError.clear();
            emit const_cast<MfBridge*>(this)->lastErrorChanged();
        }
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
