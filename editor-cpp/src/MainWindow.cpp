#include "MainWindow.h"

#include <algorithm>

#include <QAction>
#include <QActionGroup>
#include <QCoreApplication>
#include <QDir>
#include <QDockWidget>
#include <QFileDialog>
#include <QFileInfo>
#include <QKeySequence>
#include <QLabel>
#include <QMenuBar>
#include <QMessageBox>
#include <QQmlContext>
#include <QQuickWidget>
#include <QSettings>
#include <QStatusBar>
#include <QStyle>
#include <QStringList>
#include <QToolBar>
#include <QTimer>
#include <QUrl>

#include "ViewportWidget.h"
#include "SpriteEditorWidget.h"

namespace {

QString existingDirectory(const QString& path)
{
    if (path.trimmed().isEmpty()) {
        return {};
    }
    const QFileInfo info(QDir::cleanPath(path));
    if (!info.exists() || !info.isDir()) {
        return {};
    }
    const QString canonicalPath = info.canonicalFilePath();
    return canonicalPath.isEmpty() ? info.absoluteFilePath() : canonicalPath;
}

QString firstExistingDirectory(const QStringList& candidates)
{
    for (const QString& candidate : candidates) {
        const QString path = existingDirectory(candidate);
        if (!path.isEmpty()) {
            return path;
        }
    }
    return {};
}

QSettings editorSettings()
{
    return QSettings(QStringLiteral("MiniForge"), QStringLiteral("MiniForgeQtEditor"));
}

QString resolveQmlRootPath()
{
    const QString applicationDir = QCoreApplication::applicationDirPath();
    QStringList candidates;
    candidates << QDir(applicationDir).filePath(QStringLiteral("../Resources/editor-qml"));
#ifdef MF_QML_INSTALL_SUBDIR
    const QString installSubdir = QString::fromUtf8(MF_QML_INSTALL_SUBDIR);
    candidates << QDir(applicationDir).filePath(QStringLiteral("../") + installSubdir)
               << QDir(applicationDir).filePath(QStringLiteral("../../../") + installSubdir)
               << QDir(applicationDir).filePath(installSubdir);
#endif
#ifdef MF_QML_ROOT
    candidates << QString::fromUtf8(MF_QML_ROOT);
#endif
    return firstExistingDirectory(candidates);
}

QString resolveStartupProjectPath(const QString& requestedPath)
{
    if (!requestedPath.trimmed().isEmpty()) {
        return QFileInfo(requestedPath).absoluteFilePath();
    }

    QSettings settings = editorSettings();
    const QString recentProject = existingDirectory(settings.value(QStringLiteral("projects/lastPath")).toString());
    if (!recentProject.isEmpty()) {
        return recentProject;
    }

    const QString applicationDir = QCoreApplication::applicationDirPath();
    QStringList candidates {
        QDir(applicationDir).filePath(QStringLiteral("../Resources/projects/DefaultProject")),
        QDir(applicationDir).filePath(QStringLiteral("../share/miniforge/projects/DefaultProject")),
        QDir(applicationDir).filePath(QStringLiteral("../../../share/miniforge/projects/DefaultProject")),
    };
#ifdef MF_DEFAULT_PROJECT
    candidates << QString::fromUtf8(MF_DEFAULT_PROJECT);
#endif
    return firstExistingDirectory(candidates);
}

} // namespace

MainWindow::MainWindow(const QString& projectPath, QWidget* parent)
    : QMainWindow(parent)
    , m_entities(&m_bridge, this)
    , m_inspector(&m_bridge, this)
    , m_assets(&m_bridge, this)
    , m_commands(&m_bridge, this)
    , m_console(&m_bridge, this)
    , m_readiness(&m_bridge, this)
    , m_forgeAi(&m_bridge, this)
    , m_controller(&m_bridge, &m_inspector, this)
{
    setWindowTitle(QStringLiteral("MiniForge 0.9.3.4 Qt Editor"));
    resize(1440, 920);
    setDockOptions(QMainWindow::AllowNestedDocks | QMainWindow::AllowTabbedDocks | QMainWindow::AnimatedDocks);
    m_qmlRootPath = resolveQmlRootPath();

    applyEditorStyle();
    createMenus();
    createToolbar();

    createPanels();
    m_refreshTimer = new QTimer(this);
    m_refreshTimer->setInterval(1000);
    connect(m_refreshTimer, &QTimer::timeout, this, &MainWindow::refreshWorkbench);
    if (m_autoRefreshAction) {
        connect(m_autoRefreshAction, &QAction::toggled, this, [this](bool enabled) {
            if (enabled && m_bridge.isOpen()) {
                m_refreshTimer->start();
                refreshWorkbench();
            } else {
                m_refreshTimer->stop();
            }
        });
        m_autoRefreshAction->setChecked(true);
    }

    m_projectStatus = new QLabel(this);
    m_selectionStatus = new QLabel(this);
    m_readinessStatus = new QLabel(this);
    statusBar()->addPermanentWidget(m_projectStatus, 3);
    statusBar()->addPermanentWidget(m_selectionStatus, 1);
    statusBar()->addPermanentWidget(m_readinessStatus, 1);

    connect(&m_bridge, &MfBridge::projectChanged, this, &MainWindow::updateStatus);
    connect(&m_bridge, &MfBridge::selectionChanged, this, &MainWindow::updateStatus);
    connect(&m_bridge, &MfBridge::entitiesChanged, this, &MainWindow::updateStatus);
    connect(&m_bridge, &MfBridge::readinessChanged, this, &MainWindow::updateStatus);
    connect(&m_readiness, &ReadinessModel::scoreChanged, this, &MainWindow::updateStatus);

    const QString startupProject = resolveStartupProjectPath(projectPath);
    if (startupProject.isEmpty()) {
        statusBar()->showMessage(QStringLiteral("No project open - use File > Open Project"));
    } else {
        openProjectPath(startupProject, false);
    }
    const QString requestedWorkspace = qEnvironmentVariable("MINIFORGE_WORKSPACE", QStringLiteral("2D"));
    const QStringList workspaces {
        QStringLiteral("2D"),
        QStringLiteral("Scripting"),
        QStringLiteral("Animation"),
        QStringLiteral("AI"),
        QStringLiteral("Build"),
        QStringLiteral("Debug"),
        QStringLiteral("Minimal"),
    };
    const auto initialWorkspace = std::find_if(
        workspaces.cbegin(),
        workspaces.cend(),
        [&requestedWorkspace](const QString& workspace) {
            return workspace.compare(requestedWorkspace, Qt::CaseInsensitive) == 0;
        }
    );
    setWorkspace(initialWorkspace == workspaces.cend() ? QStringLiteral("2D") : *initialWorkspace);
    updateStatus();
}

MainWindow::~MainWindow()
{
    if (m_refreshTimer) {
        m_refreshTimer->stop();
    }
    // QML context properties reference C++ member models. Destroy every dock
    // and the native viewport while those members are still alive; QObject's
    // base destructor otherwise deletes child widgets after member teardown.
    const auto docks = findChildren<QDockWidget*>(QString(), Qt::FindDirectChildrenOnly);
    for (QDockWidget* dock : docks) {
        delete dock;
    }
    delete takeCentralWidget();
}

QWidget* MainWindow::makeQmlPanel(const QString& qmlFile)
{
    if (m_qmlRootPath.isEmpty()) {
        auto* message = new QLabel(QStringLiteral("Editor UI resources were not found. Reinstall MiniForge or check the editor-qml data directory."), this);
        message->setAlignment(Qt::AlignCenter);
        message->setWordWrap(true);
        return message;
    }
    auto* view = new QQuickWidget(this);
    view->setResizeMode(QQuickWidget::SizeRootObjectToView);
    view->rootContext()->setContextProperty(QStringLiteral("editorBridge"), &m_bridge);
    view->rootContext()->setContextProperty(QStringLiteral("editorController"), &m_controller);
    view->rootContext()->setContextProperty(QStringLiteral("hierarchyModel"), &m_entities);
    view->rootContext()->setContextProperty(QStringLiteral("inspectorModel"), &m_inspector);
    view->rootContext()->setContextProperty(QStringLiteral("contentModel"), &m_assets);
    view->rootContext()->setContextProperty(QStringLiteral("commandModel"), &m_commands);
    view->rootContext()->setContextProperty(QStringLiteral("consoleModel"), &m_console);
    view->rootContext()->setContextProperty(QStringLiteral("readinessModel"), &m_readiness);
    view->rootContext()->setContextProperty(QStringLiteral("forgeAiModel"), &m_forgeAi);
    view->setSource(QUrl::fromLocalFile(QDir(m_qmlRootPath).filePath(QStringLiteral("panels/") + qmlFile)));
    return view;
}

bool MainWindow::openProjectPath(const QString& path, bool showErrorDialog)
{
    const QString projectPath = existingDirectory(path);
    if (projectPath.isEmpty()) {
        const QString message = QStringLiteral("Project directory does not exist: %1").arg(QDir::toNativeSeparators(path));
        statusBar()->showMessage(message, 5000);
        if (showErrorDialog) {
            QMessageBox::warning(this, QStringLiteral("Open Project"), message);
        }
        return false;
    }
    if (!m_bridge.openProject(projectPath)) {
        const QString message = m_bridge.lastError();
        statusBar()->showMessage(message, 5000);
        if (showErrorDialog) {
            QMessageBox::warning(this, QStringLiteral("Open Project"), message);
        }
        return false;
    }

    QSettings settings = editorSettings();
    settings.setValue(QStringLiteral("projects/lastPath"), projectPath);
    refreshModels();
    updateStatus();
    setWindowTitle(QStringLiteral("MiniForge 0.9.3.4 Qt Editor - %1").arg(m_bridge.projectName()));
    statusBar()->showMessage(QStringLiteral("Opened %1").arg(m_bridge.projectSummary()), 3000);
    if (m_autoRefreshAction && m_autoRefreshAction->isChecked() && m_refreshTimer) {
        m_refreshTimer->start();
    }
    return true;
}

void MainWindow::chooseProject()
{
    QSettings settings = editorSettings();
    QString initialDirectory = m_bridge.isOpen()
        ? m_bridge.projectPath()
        : settings.value(QStringLiteral("projects/lastPath"), QDir::homePath()).toString();
    if (existingDirectory(initialDirectory).isEmpty()) {
        initialDirectory = QDir::homePath();
    }
    const QString path = QFileDialog::getExistingDirectory(
        this,
        QStringLiteral("Open MiniForge Project"),
        initialDirectory,
        QFileDialog::ShowDirsOnly | QFileDialog::DontResolveSymlinks
    );
    if (!path.isEmpty()) {
        openProjectPath(path, true);
    }
}

QDockWidget* MainWindow::addDock(const QString& title, Qt::DockWidgetArea area, QWidget* widget)
{
    auto* dock = new QDockWidget(title, this);
    dock->setObjectName(title);
    dock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable | QDockWidget::DockWidgetClosable);
    dock->setWidget(widget);
    addDockWidget(area, dock);
    return dock;
}

void MainWindow::createMenus()
{
    const auto addMenuCommand = [this](QMenu* menu, const QString& label, const QString& commandId, const QKeySequence& shortcut = {}) {
        auto* action = menu->addAction(label);
        if (!shortcut.isEmpty()) {
            action->setShortcut(shortcut);
        }
        connect(action, &QAction::triggered, this, [this, commandId] {
            m_bridge.executeCommand(commandId);
        });
        return action;
    };

    auto* file = menuBar()->addMenu(QStringLiteral("&File"));
    auto* openProject = file->addAction(QStringLiteral("Open Project..."));
    openProject->setShortcut(QKeySequence::Open);
    connect(openProject, &QAction::triggered, this, &MainWindow::chooseProject);
    file->addSeparator();
    auto* create = file->addMenu(QStringLiteral("New"));
    addMenuCommand(create, QStringLiteral("Pixel Art Sprite"), QStringLiteral("sprite.new_pixel_art"));
    addMenuCommand(create, QStringLiteral("Hero Sprite Template"), QStringLiteral("sprite.create_hero_template"));
    addMenuCommand(create, QStringLiteral("Luau 2D Controller"), QStringLiteral("luau.new_controller"));
    addMenuCommand(create, QStringLiteral("Sprite Actor"), QStringLiteral("object.create_sprite_actor"));
    file->addSeparator();
    addMenuCommand(file, QStringLiteral("Save Project"), QStringLiteral("project.save"), QKeySequence::Save);
    addMenuCommand(file, QStringLiteral("Save Scene"), QStringLiteral("scene.save"));

    auto* edit = menuBar()->addMenu(QStringLiteral("&Edit"));
    addMenuCommand(edit, QStringLiteral("Undo"), QStringLiteral("edit.undo"), QKeySequence::Undo);
    addMenuCommand(edit, QStringLiteral("Redo"), QStringLiteral("edit.redo"), QKeySequence::Redo);
    auto* spriteTools = edit->addMenu(QStringLiteral("Sprite Tools"));
    addMenuCommand(spriteTools, QStringLiteral("Create SpriteFrames"), QStringLiteral("sprite.export_frames"));
    addMenuCommand(spriteTools, QStringLiteral("Export Atlas Pages"), QStringLiteral("sprite.export_atlas_pages"));
    addMenuCommand(spriteTools, QStringLiteral("Create Palette Ramp"), QStringLiteral("sprite.optimize_palette"));

    auto* project = menuBar()->addMenu(QStringLiteral("&Project"));
    addMenuCommand(project, QStringLiteral("Run Audit"), QStringLiteral("project.audit"));
    addMenuCommand(project, QStringLiteral("Refresh Assets"), QStringLiteral("assets.refresh"));
    addMenuCommand(project, QStringLiteral("Validate Luau Scripts"), QStringLiteral("luau.validate_scripts"));
    addMenuCommand(project, QStringLiteral("Write 2D Render Profile"), QStringLiteral("render.write_2d_profile"));

    auto* build = menuBar()->addMenu(QStringLiteral("&Build"));
    auto* openBuild = build->addAction(QStringLiteral("Build && Export..."));
    connect(openBuild, &QAction::triggered, this, [this] {
        setWorkspace(QStringLiteral("Build"));
    });

    auto* forgeAi = menuBar()->addMenu(QStringLiteral("Forge &AI"));
    auto* runDoctor = forgeAi->addAction(QStringLiteral("Run Project Doctor"));
    connect(runDoctor, &QAction::triggered, this, [this] {
        m_forgeAi.runDoctor();
        setWorkspace(QStringLiteral("AI"));
    });
    auto* runEnemySmoke = forgeAi->addAction(QStringLiteral("Run Enemy Smoke Test"));
    connect(runEnemySmoke, &QAction::triggered, this, [this] {
        m_forgeAi.runEnemySmoke();
        setWorkspace(QStringLiteral("AI"));
    });

    auto* objects = menuBar()->addMenu(QStringLiteral("&Objects"));
    addMenuCommand(objects, QStringLiteral("Sprite Actor"), QStringLiteral("object.create_sprite_actor"));
    addMenuCommand(objects, QStringLiteral("Camera Rig"), QStringLiteral("object.create_camera"));
    addMenuCommand(objects, QStringLiteral("HUD Text"), QStringLiteral("object.create_ui_text"));

    auto* workspace = menuBar()->addMenu(QStringLiteral("&Workspace"));
    auto* workspaceGroup = new QActionGroup(this);
    for (const QString& name : { QStringLiteral("2D"), QStringLiteral("Scripting"), QStringLiteral("Animation"), QStringLiteral("AI"), QStringLiteral("Build"), QStringLiteral("Debug"), QStringLiteral("Minimal") }) {
        QAction* action = workspace->addAction(name);
        action->setObjectName(QStringLiteral("Workspace.%1").arg(name));
        action->setCheckable(true);
        action->setActionGroup(workspaceGroup);
        if (name == QStringLiteral("2D")) {
            action->setChecked(true);
        }
        connect(action, &QAction::triggered, this, [this, name] {
            setWorkspace(name);
        });
    }

    auto* view = menuBar()->addMenu(QStringLiteral("&View"));
    m_gridAction = view->addAction(QStringLiteral("Viewport Guides"));
    m_gridAction->setCheckable(true);
    m_gridAction->setChecked(false);
    m_gridAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+G")));
    m_hudAction = view->addAction(QStringLiteral("Viewport HUD"));
    m_hudAction->setCheckable(true);
    m_hudAction->setChecked(true);
    m_hudAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+H")));
    view->addSeparator();
    m_autoRefreshAction = view->addAction(QStringLiteral("Auto Refresh Workbench"));
    m_autoRefreshAction->setCheckable(true);
    m_autoRefreshAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+R")));
    QAction* refreshNow = view->addAction(QStringLiteral("Refresh Now"));
    refreshNow->setShortcut(QKeySequence::Refresh);
    connect(refreshNow, &QAction::triggered, this, &MainWindow::refreshWorkbench);
}

void MainWindow::createToolbar()
{
    auto* toolbar = addToolBar(QStringLiteral("Main"));
    toolbar->setObjectName(QStringLiteral("MainToolbar"));
    toolbar->setMovable(false);
    toolbar->setIconSize(QSize(18, 18));

    const auto addCommand = [this, toolbar](const QString& label, const QString& commandId, QStyle::StandardPixmap icon, const QKeySequence& shortcut = {}) {
        auto* action = toolbar->addAction(style()->standardIcon(icon), label, this, [this, commandId] {
            m_bridge.executeCommand(commandId);
        });
        if (!shortcut.isEmpty()) {
            action->setShortcut(shortcut);
        }
        action->setToolTip(label);
        return action;
    };

    addCommand(QStringLiteral("Save"), QStringLiteral("project.save"), QStyle::SP_DialogSaveButton, QKeySequence::Save);
    addCommand(QStringLiteral("Undo"), QStringLiteral("edit.undo"), QStyle::SP_ArrowBack, QKeySequence::Undo);
    addCommand(QStringLiteral("Redo"), QStringLiteral("edit.redo"), QStyle::SP_ArrowForward, QKeySequence::Redo);
    toolbar->addSeparator();
    addCommand(QStringLiteral("New Entity"), QStringLiteral("entity.create_empty"), QStyle::SP_FileIcon);
    addCommand(QStringLiteral("Sprite Actor"), QStringLiteral("object.create_sprite_actor"), QStyle::SP_ComputerIcon);
    addCommand(QStringLiteral("Refresh Assets"), QStringLiteral("assets.refresh"), QStyle::SP_BrowserReload);
    addCommand(QStringLiteral("Run Audit"), QStringLiteral("project.audit"), QStyle::SP_FileDialogDetailedView);
    addCommand(QStringLiteral("2D Profile"), QStringLiteral("render.write_2d_profile"), QStyle::SP_DesktopIcon);
    toolbar->addSeparator();
    addCommand(QStringLiteral("New Sprite"), QStringLiteral("sprite.new_pixel_art"), QStyle::SP_FileDialogNewFolder);
    addCommand(QStringLiteral("Atlas"), QStringLiteral("sprite.export_atlas_pages"), QStyle::SP_DriveHDIcon);
    addCommand(QStringLiteral("New Luau"), QStringLiteral("luau.new_controller"), QStyle::SP_FileIcon);
    toolbar->addSeparator();
    addCommand(QStringLiteral("Play"), QStringLiteral("play.enter"), QStyle::SP_MediaPlay);
    addCommand(QStringLiteral("Stop"), QStringLiteral("play.stop"), QStyle::SP_MediaStop);
    toolbar->addSeparator();
    auto* refresh = toolbar->addAction(style()->standardIcon(QStyle::SP_BrowserReload), QStringLiteral("Refresh"), this, &MainWindow::refreshWorkbench);
    refresh->setShortcut(QKeySequence::Refresh);
    refresh->setToolTip(QStringLiteral("Refresh Qt models and the Rust viewport snapshot"));
}

void MainWindow::createPanels()
{
    m_viewport = new ViewportWidget(&m_bridge, this);
    setCentralWidget(m_viewport);
    if (m_gridAction) {
        connect(m_gridAction, &QAction::toggled, m_viewport, &ViewportWidget::setGridVisible);
        connect(m_viewport, &ViewportWidget::gridVisibleChanged, this, [this] {
            m_gridAction->setChecked(m_viewport->gridVisible());
        });
    }
    if (m_hudAction) {
        connect(m_hudAction, &QAction::toggled, m_viewport, &ViewportWidget::setHudVisible);
        connect(m_viewport, &ViewportWidget::hudVisibleChanged, this, [this] {
            m_hudAction->setChecked(m_viewport->hudVisible());
        });
    }

    m_hierarchyDock = addDock(QStringLiteral("Hierarchy"), Qt::LeftDockWidgetArea, makeQmlPanel(QStringLiteral("HierarchyPanel.qml")));
    m_inspectorDock = addDock(QStringLiteral("Inspector"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("InspectorPanel.qml")));
    m_readinessDock = addDock(QStringLiteral("Readiness"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ReadinessPanel.qml")));
    m_aiDock = addDock(QStringLiteral("Forge AI"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ForgeAiPanel.qml")));
    m_buildDock = addDock(QStringLiteral("Build & Export"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("BuildExportPanel.qml")));
    m_objectsDock = addDock(QStringLiteral("Object Studio"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ObjectStudioPanel.qml")));
    m_contentDock = addDock(QStringLiteral("Content Browser"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("ContentBrowserPanel.qml")));
    m_consoleDock = addDock(QStringLiteral("Console"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("ConsolePanel.qml")));
    m_spriteDock = addDock(QStringLiteral("Sprite Studio"), Qt::BottomDockWidgetArea, new SpriteEditorWidget(&m_bridge, this));
    m_luauDock = addDock(QStringLiteral("Luau Studio"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("LuauStudioPanel.qml")));
    m_commandDock = addDock(QStringLiteral("Command Palette"), Qt::TopDockWidgetArea, makeQmlPanel(QStringLiteral("CommandPalette.qml")));

    tabifyDockWidget(m_inspectorDock, m_readinessDock);
    tabifyDockWidget(m_inspectorDock, m_aiDock);
    tabifyDockWidget(m_inspectorDock, m_buildDock);
    tabifyDockWidget(m_inspectorDock, m_objectsDock);
    tabifyDockWidget(m_contentDock, m_consoleDock);
    tabifyDockWidget(m_contentDock, m_spriteDock);
    tabifyDockWidget(m_contentDock, m_luauDock);
    m_inspectorDock->raise();
    m_contentDock->raise();
    resizeDocks({ m_hierarchyDock, m_inspectorDock }, { 320, 360 }, Qt::Horizontal);
    resizeDocks({ m_contentDock, m_consoleDock }, { 240, 240 }, Qt::Vertical);
}

void MainWindow::applyEditorStyle()
{
    setStyleSheet(QStringLiteral(R"(
        QMainWindow { background: #16181d; color: #e8eaee; }
        QMenuBar, QMenu, QToolBar, QStatusBar { background: #202329; color: #e8eaee; border: 0; }
        QMenuBar { border-bottom: 1px solid #2f3540; }
        QMenuBar::item { padding: 5px 9px; }
        QMenuBar::item:selected, QMenu::item:selected { background: #292d35; }
        QMenu { border: 1px solid #3b414b; }
        QMenu::item { padding: 5px 22px; }
        QToolBar { border-bottom: 1px solid #2f3540; spacing: 4px; padding: 5px; }
        QToolButton { color: #e8eaee; padding: 5px 8px; border: 1px solid transparent; border-radius: 5px; }
        QToolButton:hover { background: #292d35; border-color: #3b414b; }
        QToolButton:pressed { background: #3b414b; }
        QDockWidget { color: #e8eaee; titlebar-close-icon: none; titlebar-normal-icon: none; }
        QDockWidget::title { background: #202329; padding: 7px 8px; border-bottom: 1px solid #3b414b; }
        QStatusBar { border-top: 1px solid #2f3540; }
        QLabel { color: #aab0bb; }
    )"));
}

void MainWindow::refreshModels()
{
    m_entities.refresh();
    m_assets.refresh();
    m_commands.refresh();
    m_console.refresh();
    m_readiness.refresh();
    m_forgeAi.runDoctor();
    if (m_viewport) {
        m_viewport->refreshImage();
    }
}

void MainWindow::refreshWorkbench()
{
    m_bridge.refreshAll();
    if (m_viewport) {
        m_viewport->refreshImage();
    }
    updateStatus();
}

void MainWindow::updateStatus()
{
    if (m_projectStatus) {
        m_projectStatus->setText(QStringLiteral("%1 | %2")
            .arg(m_bridge.projectSummary(), m_bridge.workbenchSummary()));
    }
    if (m_selectionStatus) {
        const QVector<quint64> selected = m_bridge.selectedEntities();
        m_selectionStatus->setText(selected.isEmpty()
            ? QStringLiteral("Selection: none")
            : QStringLiteral("Selection: #%1").arg(selected.first()));
    }
    if (m_readinessStatus) {
        m_readinessStatus->setText(QStringLiteral("Readiness: %1%").arg(m_readiness.score()));
    }
}

void MainWindow::setWorkspace(const QString& workspace)
{
    if (auto* action = findChild<QAction*>(QStringLiteral("Workspace.%1").arg(workspace))) {
        action->setChecked(true);
    }
    const bool scene = workspace == QStringLiteral("2D");
    const bool scripting = workspace == QStringLiteral("Scripting");
    const bool animation = workspace == QStringLiteral("Animation");
    const bool ai = workspace == QStringLiteral("AI");
    const bool build = workspace == QStringLiteral("Build");
    const bool debug = workspace == QStringLiteral("Debug");
    const bool minimal = workspace == QStringLiteral("Minimal");

    if (m_hierarchyDock) {
        m_hierarchyDock->setVisible(!minimal && !debug);
    }
    if (m_inspectorDock) {
        m_inspectorDock->setVisible(!minimal || scene);
    }
    if (m_readinessDock) {
        m_readinessDock->setVisible(scene || build || debug);
    }
    if (m_aiDock) {
        m_aiDock->setVisible(ai);
    }
    if (m_buildDock) {
        m_buildDock->setVisible(build);
    }
    if (m_objectsDock) {
        m_objectsDock->setVisible(scene);
    }
    if (m_contentDock) {
        m_contentDock->setVisible(scene || animation);
    }
    if (m_consoleDock) {
        m_consoleDock->setVisible(scripting || ai || build || debug);
    }
    if (m_spriteDock) {
        m_spriteDock->setVisible(animation);
    }
    if (m_luauDock) {
        m_luauDock->setVisible(scripting);
    }
    if (m_commandDock) {
        m_commandDock->setVisible(!minimal);
    }

    if (scene && m_contentDock) {
        m_contentDock->raise();
    } else if (scripting && m_luauDock) {
        m_luauDock->raise();
    } else if (animation && m_spriteDock) {
        m_spriteDock->raise();
    } else if (ai && m_aiDock) {
        m_aiDock->raise();
        if (m_consoleDock) {
            m_consoleDock->raise();
        }
    } else if (build && m_buildDock) {
        m_buildDock->raise();
        if (m_consoleDock) {
            m_consoleDock->raise();
        }
    } else if (debug && m_consoleDock) {
        m_consoleDock->raise();
    }
    statusBar()->showMessage(QStringLiteral("Workspace: %1").arg(workspace), 1800);
}
