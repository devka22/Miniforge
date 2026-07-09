#include <QApplication>

#include "MainWindow.h"

int main(int argc, char** argv)
{
    QApplication app(argc, argv);
    const QString projectPath = argc > 1 ? QString::fromLocal8Bit(argv[1]) : QString();
    MainWindow window(projectPath);
    window.show();
    return app.exec();
}
