#pragma once

#include <QPointer>
#include <QSet>
#include <QSyntaxHighlighter>
#include <QVariantList>
#include <QVector>

class QColor;
class QQuickTextDocument;
class QRegularExpression;
class QTextCharFormat;

class LuauSyntaxHighlighter final : public QSyntaxHighlighter {
    Q_OBJECT
    Q_PROPERTY(QObject* textDocument READ textDocument WRITE setTextDocument NOTIFY textDocumentChanged)
    Q_PROPERTY(int diagnosticLine READ diagnosticLine WRITE setDiagnosticLine NOTIFY diagnosticLineChanged)
    Q_PROPERTY(int currentLine READ currentLine WRITE setCurrentLine NOTIFY currentLineChanged)
    Q_PROPERTY(QVariantList breakpointLines READ breakpointLines WRITE setBreakpointLines NOTIFY breakpointLinesChanged)

public:
    explicit LuauSyntaxHighlighter(QObject* parent = nullptr);

    QObject* textDocument() const;
    void setTextDocument(QObject* documentObject);

    int diagnosticLine() const;
    void setDiagnosticLine(int line);

    int currentLine() const;
    void setCurrentLine(int line);

    QVariantList breakpointLines() const;
    void setBreakpointLines(const QVariantList& lines);

signals:
    void textDocumentChanged();
    void diagnosticLineChanged();
    void currentLineChanged();
    void breakpointLinesChanged();

protected:
    void highlightBlock(const QString& text) override;

private:
    void applyRule(
        const QString& text,
        const QRegularExpression& expression,
        const QTextCharFormat& format,
        const QVector<bool>& protectedCharacters,
        int capture = 0);
    void applyLineBackground(const QString& text, const QColor& color);

    QPointer<QObject> m_textDocumentObject;
    QMetaObject::Connection m_documentDestroyedConnection;
    int m_diagnosticLine = 0;
    int m_currentLine = 0;
    QSet<int> m_breakpointLines;
};
