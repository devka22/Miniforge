#include "LuauSyntaxHighlighter.h"

#include <algorithm>

#include <QColor>
#include <QQuickTextDocument>
#include <QRegularExpression>
#include <QTextCharFormat>
#include <QTextDocument>
#include <QTextFormat>

namespace {

constexpr int kCommentStateBase = 1000;
constexpr int kStringStateBase = 2000;

QTextCharFormat foregroundFormat(const char* color, bool bold = false, bool italic = false)
{
    QTextCharFormat format;
    format.setForeground(QColor(QString::fromLatin1(color)));
    format.setFontWeight(bold ? QFont::DemiBold : QFont::Normal);
    format.setFontItalic(italic);
    return format;
}

int longBracketEquals(const QString& text, int offset)
{
    if (offset < 0 || offset >= text.size() || text.at(offset) != u'[') {
        return -1;
    }
    int cursor = offset + 1;
    while (cursor < text.size() && text.at(cursor) == u'=') {
        ++cursor;
    }
    return cursor < text.size() && text.at(cursor) == u'[' ? cursor - offset - 1 : -1;
}

QString longBracketClose(int equals)
{
    return QStringLiteral("]") + QString(equals, u'=') + QStringLiteral("]");
}

} // namespace

LuauSyntaxHighlighter::LuauSyntaxHighlighter(QObject* parent)
    : QSyntaxHighlighter(parent)
{
}

QObject* LuauSyntaxHighlighter::textDocument() const
{
    return m_textDocumentObject.data();
}

void LuauSyntaxHighlighter::setTextDocument(QObject* documentObject)
{
    if (m_textDocumentObject == documentObject) {
        return;
    }
    disconnect(m_documentDestroyedConnection);
    m_textDocumentObject = documentObject;

    QTextDocument* document = qobject_cast<QTextDocument*>(documentObject);
    if (!document) {
        if (auto* quickDocument = qobject_cast<QQuickTextDocument*>(documentObject)) {
            document = quickDocument->textDocument();
        }
    }
    setDocument(document);
    if (documentObject) {
        m_documentDestroyedConnection = connect(documentObject, &QObject::destroyed, this, [this] {
            m_textDocumentObject.clear();
            setDocument(nullptr);
            emit textDocumentChanged();
        });
    }
    emit textDocumentChanged();
}

int LuauSyntaxHighlighter::diagnosticLine() const
{
    return m_diagnosticLine;
}

void LuauSyntaxHighlighter::setDiagnosticLine(int line)
{
    line = std::max(0, line);
    if (m_diagnosticLine == line) {
        return;
    }
    m_diagnosticLine = line;
    emit diagnosticLineChanged();
    rehighlight();
}

int LuauSyntaxHighlighter::currentLine() const
{
    return m_currentLine;
}

void LuauSyntaxHighlighter::setCurrentLine(int line)
{
    line = std::max(0, line);
    if (m_currentLine == line) {
        return;
    }
    m_currentLine = line;
    emit currentLineChanged();
    rehighlight();
}

QVariantList LuauSyntaxHighlighter::breakpointLines() const
{
    QList<int> sorted = m_breakpointLines.values();
    std::sort(sorted.begin(), sorted.end());
    QVariantList values;
    values.reserve(sorted.size());
    for (int line : sorted) {
        values.push_back(line);
    }
    return values;
}

void LuauSyntaxHighlighter::setBreakpointLines(const QVariantList& lines)
{
    QSet<int> normalized;
    for (const QVariant& value : lines) {
        bool valid = false;
        const int line = value.toInt(&valid);
        if (valid && line > 0) {
            normalized.insert(line);
        }
    }
    if (m_breakpointLines == normalized) {
        return;
    }
    m_breakpointLines = std::move(normalized);
    emit breakpointLinesChanged();
    rehighlight();
}

void LuauSyntaxHighlighter::highlightBlock(const QString& text)
{
    const QTextCharFormat commentFormat = foregroundFormat("#6f8799", false, true);
    const QTextCharFormat stringFormat = foregroundFormat("#d7ba7d");
    QVector<bool> protectedCharacters(text.size(), false);

    const auto protect = [this, &protectedCharacters](int start, int length, const QTextCharFormat& format) {
        if (length <= 0) {
            return;
        }
        setFormat(start, length, format);
        const int end = std::min(start + length, static_cast<int>(protectedCharacters.size()));
        for (int cursor = std::max(0, start); cursor < end; ++cursor) {
            protectedCharacters[cursor] = true;
        }
    };

    setCurrentBlockState(0);
    int cursor = 0;
    const int previousState = previousBlockState();
    if (previousState >= kCommentStateBase) {
        const bool comment = previousState < kStringStateBase;
        const int equals = comment ? previousState - kCommentStateBase : previousState - kStringStateBase;
        const QString close = longBracketClose(equals);
        const int end = text.indexOf(close);
        if (end < 0) {
            protect(0, text.size(), comment ? commentFormat : stringFormat);
            setCurrentBlockState(previousState);
            applyLineBackground(text,
                currentBlock().blockNumber() + 1 == m_diagnosticLine ? QColor(QStringLiteral("#5b2530"))
                : m_breakpointLines.contains(currentBlock().blockNumber() + 1) ? QColor(QStringLiteral("#55451f"))
                : currentBlock().blockNumber() + 1 == m_currentLine ? QColor(QStringLiteral("#242b35"))
                : QColor());
            return;
        }
        cursor = end + close.size();
        protect(0, cursor, comment ? commentFormat : stringFormat);
    }

    while (cursor < text.size()) {
        if (text.mid(cursor, 2) == QStringLiteral("--")) {
            const int equals = longBracketEquals(text, cursor + 2);
            if (equals >= 0) {
                const QString close = longBracketClose(equals);
                const int end = text.indexOf(close, cursor + 4 + equals);
                if (end < 0) {
                    protect(cursor, text.size() - cursor, commentFormat);
                    setCurrentBlockState(kCommentStateBase + equals);
                    break;
                }
                const int length = end + close.size() - cursor;
                protect(cursor, length, commentFormat);
                cursor += length;
                continue;
            }
            protect(cursor, text.size() - cursor, commentFormat);
            break;
        }

        const QChar character = text.at(cursor);
        if (character == u'\'' || character == u'"' || character == u'`') {
            const QChar quote = character;
            int end = cursor + 1;
            bool escaped = false;
            while (end < text.size()) {
                const QChar current = text.at(end++);
                if (!escaped && current == quote) {
                    break;
                }
                if (!escaped && current == u'\\') {
                    escaped = true;
                } else {
                    escaped = false;
                }
            }
            protect(cursor, end - cursor, stringFormat);
            cursor = end;
            continue;
        }

        const int equals = longBracketEquals(text, cursor);
        if (equals >= 0) {
            const QString close = longBracketClose(equals);
            const int end = text.indexOf(close, cursor + 2 + equals);
            if (end < 0) {
                protect(cursor, text.size() - cursor, stringFormat);
                setCurrentBlockState(kStringStateBase + equals);
                break;
            }
            const int length = end + close.size() - cursor;
            protect(cursor, length, stringFormat);
            cursor += length;
            continue;
        }
        ++cursor;
    }

    const QTextCharFormat keywordFormat = foregroundFormat("#c586c0", true);
    const QTextCharFormat literalFormat = foregroundFormat("#569cd6", true);
    const QTextCharFormat numberFormat = foregroundFormat("#b5cea8");
    const QTextCharFormat builtinFormat = foregroundFormat("#4ec9b0");
    const QTextCharFormat apiNamespaceFormat = foregroundFormat("#6cc58f", true);
    const QTextCharFormat typeFormat = foregroundFormat("#9cdcfe");
    const QTextCharFormat functionFormat = foregroundFormat("#dcdcaa", true);
    const QTextCharFormat propertyFormat = foregroundFormat("#9cdcfe");
    const QTextCharFormat annotationFormat = foregroundFormat("#c8a2ff");

    applyRule(text,
        QRegularExpression(QStringLiteral("\\b(?:and|break|continue|do|else|elseif|end|export|for|function|if|in|local|not|or|repeat|return|then|type|until|while)\\b")),
        keywordFormat, protectedCharacters);
    applyRule(text, QRegularExpression(QStringLiteral("\\b(?:false|nil|true)\\b")), literalFormat, protectedCharacters);
    applyRule(text,
        QRegularExpression(QStringLiteral("(?<![A-Za-z0-9_])(?:0[xX][0-9A-Fa-f_]+|0[bB][01_]+|(?:\\d[\\d_]*\\.?[\\d_]*|\\.\\d[\\d_]*)(?:[eE][+-]?[\\d_]+)?)(?![A-Za-z0-9_])")),
        numberFormat, protectedCharacters);
    applyRule(text,
        QRegularExpression(QStringLiteral("\\b(?:assert|error|getmetatable|ipairs|next|pairs|pcall|print|rawget|rawset|require|select|setmetatable|tonumber|tostring|type|typeof|unpack|warn|xpcall|self)\\b")),
        builtinFormat, protectedCharacters);
    applyRule(text,
        QRegularExpression(QStringLiteral("\\b(?:Vector2|Input|Time|Layers|Physics2D|Component|Rigidbody2D|CharacterBody2D|Camera|AnimationPlayer|AnimatedSprite|Tilemap|Tween|Navigation2D|Audio2D|Particles2D|Spawner|Entity|Transform2D|Scene|Game|Events|Assets|Debug|Task|miniforge)\\b")),
        apiNamespaceFormat, protectedCharacters);
    applyRule(text,
        QRegularExpression(QStringLiteral("\\b(?:any|boolean|buffer|CFrame|Color3|Enum|Instance|never|number|string|table|thread|unknown|Vector2|Vector3|void)\\b")),
        typeFormat, protectedCharacters);
    applyRule(text,
        QRegularExpression(QStringLiteral("\\b(?:local\\s+)?function\\s+([A-Za-z_][A-Za-z0-9_.:]*)")),
        functionFormat, protectedCharacters, 1);
    applyRule(text,
        QRegularExpression(QStringLiteral("\\b([A-Za-z_][A-Za-z0-9_.:]*)\\s*=\\s*function\\b")),
        functionFormat, protectedCharacters, 1);
    applyRule(text, QRegularExpression(QStringLiteral("(?<=\\.)[A-Za-z_][A-Za-z0-9_]*")), propertyFormat, protectedCharacters);
    applyRule(text, QRegularExpression(QStringLiteral("@[A-Za-z_][A-Za-z0-9_]*")), annotationFormat, protectedCharacters);

    const int line = currentBlock().blockNumber() + 1;
    if (line == m_diagnosticLine) {
        applyLineBackground(text, QColor(QStringLiteral("#5b2530")));
    } else if (m_breakpointLines.contains(line)) {
        applyLineBackground(text, QColor(QStringLiteral("#55451f")));
    } else if (line == m_currentLine) {
        applyLineBackground(text, QColor(QStringLiteral("#242b35")));
    }
}

void LuauSyntaxHighlighter::applyRule(
    const QString& text,
    const QRegularExpression& expression,
    const QTextCharFormat& format,
    const QVector<bool>& protectedCharacters,
    int capture)
{
    auto matches = expression.globalMatch(text);
    while (matches.hasNext()) {
        const QRegularExpressionMatch match = matches.next();
        const int start = match.capturedStart(capture);
        const int length = match.capturedLength(capture);
        int runStart = -1;
        for (int offset = 0; offset <= length; ++offset) {
            const int position = start + offset;
            const bool allowed = offset < length
                && position >= 0
                && position < protectedCharacters.size()
                && !protectedCharacters.at(position);
            if (allowed && runStart < 0) {
                runStart = position;
            } else if (!allowed && runStart >= 0) {
                setFormat(runStart, position - runStart, format);
                runStart = -1;
            }
        }
    }
}

void LuauSyntaxHighlighter::applyLineBackground(const QString& text, const QColor& color)
{
    if (!color.isValid()) {
        return;
    }
    const int textSize = static_cast<int>(text.size());
    const int length = std::max(1, textSize);
    for (int index = 0; index < length; ++index) {
        QTextCharFormat format = this->format(std::min(index, std::max(0, textSize - 1)));
        format.setBackground(color);
        if (index == 0) {
            format.setProperty(QTextFormat::FullWidthSelection, true);
        }
        setFormat(index, 1, format);
    }
}
