#include <QApplication>
#include <QByteArray>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QDebug>
#include <QDir>
#include <QFileInfo>
#include <QProcess>
#include <QSettings>
#include <QStringList>
#include <QTimer>

#include "MainWindow.h"
#include "MfBridge.h"

int main(int argc, char** argv)
{
    // MiniForge supplies custom backgrounds/content items for Qt Quick
    // Controls. The Basic style guarantees those customizations are honored
    // consistently instead of being rejected by a platform-native style.
    qputenv("QT_QUICK_CONTROLS_STYLE", QByteArrayLiteral("Basic"));
    for (int index = 1; index < argc; ++index) {
        if (QByteArray(argv[index]) == QByteArrayLiteral("--headless-once")
            && qEnvironmentVariableIsEmpty("QT_QPA_PLATFORM")) {
            qputenv("QT_QPA_PLATFORM", QByteArrayLiteral("offscreen"));
            break;
        }
    }
    QApplication app(argc, argv);
    QCoreApplication::setOrganizationName(QStringLiteral("MiniForge"));
    QCoreApplication::setOrganizationDomain(QStringLiteral("miniforge.io"));
    QCoreApplication::setApplicationName(QStringLiteral("MiniForge Editor"));
    QCoreApplication::setApplicationVersion(QStringLiteral("0.9.3.4"));
    app.setApplicationDisplayName(QStringLiteral("MiniForge Editor"));

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral(
        "MiniForge native Qt editor. Open, create and recover projects from the same executable."));
    parser.addHelpOption();
    parser.addVersionOption();

    const QCommandLineOption projectOption(
        { QStringLiteral("p"), QStringLiteral("project") },
        QStringLiteral("Open a MiniForge project directory."),
        QStringLiteral("path"));
    const QCommandLineOption workspaceOption(
        { QStringLiteral("w"), QStringLiteral("workspace") },
        QStringLiteral("Open the named workspace (2D, Scripting, World, UI, Assets, and others)."),
        QStringLiteral("name"));
    const QCommandLineOption createProjectOption(
        QStringLiteral("create-project"),
        QStringLiteral("Create a project at the requested location, then open it."),
        QStringLiteral("path"));
    const QCommandLineOption templateOption(
        { QStringLiteral("t"), QStringLiteral("template") },
        QStringLiteral("Template for --create-project: Empty, TopDown, Platformer or RTS."),
        QStringLiteral("name"),
        QStringLiteral("TopDown"));
    const QCommandLineOption safeModeOption(
        QStringLiteral("safe-mode"),
        QStringLiteral("Open with scripts, visual graphs and plugins disabled for recovery."));
    const QCommandLineOption runtimeOption(
        QStringLiteral("runtime"),
        QStringLiteral("Open the project and immediately enter Play Mode."));
    const QCommandLineOption launcherOption(
        QStringLiteral("launcher"),
        QStringLiteral("Open the native Project Launcher on startup."));
    const QCommandLineOption noLauncherOption(
        QStringLiteral("no-launcher"),
        QStringLiteral("Open the requested or default project directly."));
    const QCommandLineOption headlessOnceOption(
        QStringLiteral("headless-once"),
        QStringLiteral("Run one deterministic headless runtime step and exit."));
    const QCommandLineOption forceOption(
        { QStringLiteral("force"), QStringLiteral("overwrite") },
        QStringLiteral("Allow --create-project to write its template into an existing directory."));
    const QCommandLineOption resetLayoutOption(
        QStringLiteral("reset-layout"),
        QStringLiteral("Discard saved window and workspace layouts before opening."));
    const QCommandLineOption screenshotOption(
        QStringLiteral("screenshot"),
        QStringLiteral("Capture the initialized editor window to a PNG and exit (UI regression testing)."),
        QStringLiteral("path"));

    parser.addOption(projectOption);
    parser.addOption(workspaceOption);
    parser.addOption(createProjectOption);
    parser.addOption(templateOption);
    parser.addOption(safeModeOption);
    parser.addOption(runtimeOption);
    parser.addOption(launcherOption);
    parser.addOption(noLauncherOption);
    parser.addOption(headlessOnceOption);
    parser.addOption(forceOption);
    parser.addOption(resetLayoutOption);
    parser.addOption(screenshotOption);
    parser.addPositionalArgument(
        QStringLiteral("project"),
        QStringLiteral("Project directory to open (legacy positional form)."),
        QStringLiteral("[project]"));
    parser.process(app);

    const QStringList positionalArguments = parser.positionalArguments();
    if (positionalArguments.size() > 1) {
        parser.showHelp(2);
    }
    if (parser.isSet(projectOption) && !positionalArguments.isEmpty()) {
        parser.showHelp(2);
    }
    if (parser.isSet(projectOption) && parser.isSet(createProjectOption)) {
        parser.showHelp(2);
    }
    if (parser.isSet(launcherOption) && parser.isSet(noLauncherOption)) {
        qCritical("--launcher and --no-launcher cannot be used together");
        return 2;
    }
    if (parser.isSet(headlessOnceOption) && parser.isSet(createProjectOption)) {
        qCritical("--headless-once cannot be combined with --create-project");
        return 2;
    }

    if (parser.isSet(workspaceOption)) {
        qputenv("MINIFORGE_WORKSPACE", parser.value(workspaceOption).trimmed().toUtf8());
    }
    if (parser.isSet(safeModeOption)) {
        qputenv("MINIFORGE_SAFE_MODE", QByteArrayLiteral("1"));
    }
    if (parser.isSet(runtimeOption)) {
        qputenv("MINIFORGE_START_RUNTIME", QByteArrayLiteral("1"));
    }
    if (parser.isSet(launcherOption)) {
        qputenv("MINIFORGE_SHOW_LAUNCHER", QByteArrayLiteral("1"));
    }
    if (parser.isSet(resetLayoutOption)) {
        QSettings settings(QStringLiteral("MiniForge"), QStringLiteral("MiniForgeQtEditor"));
        settings.remove(QStringLiteral("workbench"));
        settings.sync();
    }

    QString projectPath = parser.isSet(projectOption)
        ? parser.value(projectOption)
        : positionalArguments.value(0);
    if (parser.isSet(createProjectOption)) {
        const QFileInfo requestedProject(QDir::cleanPath(parser.value(createProjectOption)));
        const QString projectName = requestedProject.fileName().trimmed();
        const QString location = requestedProject.absoluteDir().absolutePath();
        if (projectName.isEmpty()) {
            qCritical("--create-project requires a directory path with a project name");
            return 2;
        }
        if (requestedProject.exists() && !parser.isSet(forceOption)) {
            qCritical().noquote()
                << QStringLiteral("Project already exists: %1. Use --force to apply a template there.")
                       .arg(requestedProject.absoluteFilePath());
            return 2;
        }
        MfBridge bootstrapBridge;
        projectPath = bootstrapBridge.createProject(
            location,
            location,
            projectName,
            parser.value(templateOption));
        if (projectPath.isEmpty()) {
            qCritical().noquote() << bootstrapBridge.lastError();
            return 2;
        }
    }

    if (parser.isSet(headlessOnceOption)) {
        if (projectPath.trimmed().isEmpty()) {
#ifdef MF_DEFAULT_PROJECT
            projectPath = QString::fromUtf8(MF_DEFAULT_PROJECT);
#else
            projectPath = QDir::current().filePath(QStringLiteral("projects/DefaultProject"));
#endif
        }
        projectPath = QFileInfo(projectPath).absoluteFilePath();
        if (!QFileInfo(projectPath).isDir()) {
            qCritical().noquote() << QStringLiteral("Project directory does not exist: %1").arg(projectPath);
            return 2;
        }
#ifdef MF_ROOT_PATH
        const QString rootPath = QString::fromUtf8(MF_ROOT_PATH);
#else
        const QString rootPath = QDir::currentPath();
#endif
        const QStringList candidates {
            QDir(QCoreApplication::applicationDirPath()).filePath(QStringLiteral("miniforge_headless")),
            QDir(rootPath).filePath(QStringLiteral("target/debug/miniforge_headless")),
            QDir(rootPath).filePath(QStringLiteral("target/release/miniforge_headless")),
        };
        QString program;
        QStringList arguments;
        for (const QString& candidate : candidates) {
            if (QFileInfo(candidate).isExecutable()) {
                program = candidate;
                arguments = { projectPath, QStringLiteral("1") };
                break;
            }
        }
        if (program.isEmpty()) {
            program = QStringLiteral("cargo");
            arguments = {
                QStringLiteral("run"), QStringLiteral("--quiet"), QStringLiteral("--locked"),
                QStringLiteral("--no-default-features"), QStringLiteral("--features"),
                QStringLiteral("runtime"), QStringLiteral("--bin"),
                QStringLiteral("miniforge_headless"), QStringLiteral("--"),
                projectPath, QStringLiteral("1"),
            };
        }
        QProcess process;
        process.setWorkingDirectory(rootPath);
        process.setProcessChannelMode(QProcess::ForwardedChannels);
        process.start(program, arguments);
        if (!process.waitForStarted(10000)) {
            qCritical().noquote() << QStringLiteral("Could not start headless runtime: %1").arg(process.errorString());
            return 2;
        }
        process.waitForFinished(-1);
        return process.exitStatus() == QProcess::NormalExit ? process.exitCode() : 2;
    }

    MainWindow window(projectPath);
    window.show();
    if (parser.isSet(screenshotOption)) {
        const QString screenshotPath = QFileInfo(parser.value(screenshotOption)).absoluteFilePath();
        QDir().mkpath(QFileInfo(screenshotPath).absolutePath());
        QTimer::singleShot(1400, &app, [&app, &window, screenshotPath] {
            const bool saved = window.grab().save(screenshotPath, "PNG");
            if (!saved) {
                qCritical().noquote() << QStringLiteral("Failed to save editor screenshot: %1").arg(screenshotPath);
            }
            app.exit(saved ? 0 : 3);
        });
    }
    return app.exec();
}
