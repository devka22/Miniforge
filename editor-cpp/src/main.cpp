#include <QApplication>
#include <QByteArray>

#include "MainWindow.h"

int main(int argc, char** argv)
{
    // MiniForge supplies custom backgrounds/content items for Qt Quick
    // Controls. The Basic style guarantees those customizations are honored
    // consistently instead of being rejected by a platform-native style.
    qputenv("QT_QUICK_CONTROLS_STYLE", QByteArrayLiteral("Basic"));
    QApplication app(argc, argv);
    const QString projectPath = argc > 1 ? QString::fromLocal8Bit(argv[1]) : QString();
    MainWindow window(projectPath);
    window.show();
    return app.exec();
}
