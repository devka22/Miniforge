#include "LuauSyntaxHighlighter.h"

#include <QColor>
#include <QCoreApplication>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>

#include <cstdlib>
#include <iostream>

namespace {

void expect(bool condition, const char* message)
{
    if (condition) {
        return;
    }
    std::cerr << "Luau highlighter smoke failed: " << message << '\n';
    std::exit(EXIT_FAILURE);
}

QTextCharFormat formatAt(const QTextBlock& block, int column)
{
    const QList<QTextLayout::FormatRange> ranges = block.layout()->formats();
    for (auto iterator = ranges.crbegin(); iterator != ranges.crend(); ++iterator) {
        if (column >= iterator->start && column < iterator->start + iterator->length) {
            return iterator->format;
        }
    }
    return {};
}

} // namespace

int main(int argc, char** argv)
{
    QCoreApplication application(argc, argv);
    QTextDocument document;
    LuauSyntaxHighlighter highlighter;
    highlighter.setTextDocument(&document);
    document.setPlainText(QStringLiteral(
        "local speed: number = 12\n"
        "function Controller:on_update(dt: number)\n"
        "    print(\"tick\", speed) -- comment\n"
        "end\n"
        "--[[ block comment\n"
        "local hidden = true\n"
        "]]\n"
        "Time.delta_time\n"));
    highlighter.setBreakpointLines(QVariantList { 1 });
    highlighter.setDiagnosticLine(2);
    highlighter.setCurrentLine(3);
    highlighter.rehighlight();
    QCoreApplication::processEvents();

    const QTextBlock first = document.findBlockByNumber(0);
    const QTextBlock second = document.findBlockByNumber(1);
    const QTextBlock third = document.findBlockByNumber(2);
    const QTextBlock sixth = document.findBlockByNumber(5);
    const QTextBlock eighth = document.findBlockByNumber(7);

    expect(formatAt(first, 0).foreground().color() == QColor(QStringLiteral("#c586c0")),
        "Luau keyword should receive the keyword color");
    expect(formatAt(first, 22).foreground().color() == QColor(QStringLiteral("#b5cea8")),
        "numeric literal should receive the number color");
    expect(formatAt(second, 9).foreground().color() == QColor(QStringLiteral("#dcdcaa")),
        "callback name should receive the function color");
    expect(formatAt(third, 10).foreground().color() == QColor(QStringLiteral("#d7ba7d")),
        "quoted text should receive the string color");
    expect(formatAt(sixth, 0).foreground().color() == QColor(QStringLiteral("#6f8799")),
        "multiline comments should protect nested keyword-looking text");
    expect(formatAt(eighth, 0).foreground().color() == QColor(QStringLiteral("#6cc58f")),
        "MiniForge API namespaces should receive the API color");
    expect(formatAt(first, 0).background().color() == QColor(QStringLiteral("#55451f")),
        "breakpoint line should receive the breakpoint background");
    expect(formatAt(second, 0).background().color() == QColor(QStringLiteral("#5b2530")),
        "diagnostic line should receive the diagnostic background");
    expect(formatAt(third, 0).background().color() == QColor(QStringLiteral("#242b35")),
        "current line should receive the active line background");

    highlighter.setTextDocument(nullptr);
    expect(highlighter.document() == nullptr, "highlighter should detach cleanly from a document");
    std::cout << "MiniForge Luau syntax highlighter smoke passed\n";
    return EXIT_SUCCESS;
}
