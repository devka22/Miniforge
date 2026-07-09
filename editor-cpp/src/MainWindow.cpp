#include "MainWindow.h"

#include <QAction>
#include <QCoreApplication>
#include <QDockWidget>
#include <QFileInfo>
#include <QKeySequence>
#include <QLabel>
#include <QMenuBar>
#include <QQmlContext>
#include <QQuickWidget>
#include <QStatusBar>
#include <QStyle>
#include <QToolBar>
#include <QUrl>

#include "ViewportWidget.h"

MainWindow::MainWindow(const QString& projectPath, QWidget* parent)
    : QMainWindow(parent)
    , m_entities(&m_bridge, this)
    , m_inspector(&m_bridge, this)
    , m_assets(&m_bridge, this)
    , m_commands(&m_bridge, this)
    , m_console(&m_bridge, this)
    , m_readiness(&m_bridge, this)
    , m_controller(&m_bridge, &m_inspector, this)
{
    setWindowTitle(QStringLiteral("MiniForge 0.9.3.4 Qt Editor"));
    resize(1440, 920);
    setDockOptions(QMainWindow::AllowNestedDocks | QMainWindow::AllowTabbedDocks | QMainWindow::AnimatedDocks);

    applyEditorStyle();
    createMenus();
    createToolbar();

    createPanels();
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

    const QString path = projectPath.isEmpty()
        ? QFileInfo(QCoreApplication::applicationDirPath() + QStringLiteral("/../../projects/DefaultProject")).absoluteFilePath()
        : projectPath;
    if (!m_bridge.openProject(path)) {
        statusBar()->showMessage(m_bridge.lastError());
    } else {
        statusBar()->showMessage(m_bridge.projectSummary());
    }
    refreshModels();
    updateStatus();
}

QWidget* MainWindow::makeQmlPanel(const QString& qmlFile)
{
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
    view->setSource(QUrl::fromLocalFile(QStringLiteral(MF_QML_ROOT) + QStringLiteral("/panels/") + qmlFile));
    return view;
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

    auto* objects = menuBar()->addMenu(QStringLiteral("&Objects"));
    addMenuCommand(objects, QStringLiteral("Sprite Actor"), QStringLiteral("object.create_sprite_actor"));
    addMenuCommand(objects, QStringLiteral("Camera Rig"), QStringLiteral("object.create_camera"));
    addMenuCommand(objects, QStringLiteral("HUD Text"), QStringLiteral("object.create_ui_text"));

    auto* workspace = menuBar()->addMenu(QStringLiteral("&Workspace"));
    for (const QString& name : { QStringLiteral("2D"), QStringLiteral("Scripting"), QStringLiteral("Animation"), QStringLiteral("Debug"), QStringLiteral("Minimal") }) {
        QAction* action = workspace->addAction(name);
        action->setCheckable(true);
    }
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
}

void MainWindow::createPanels()
{
    setCentralWidget(new ViewportWidget(&m_bridge, this));
    auto* hierarchy = addDock(QStringLiteral("Hierarchy"), Qt::LeftDockWidgetArea, makeQmlPanel(QStringLiteral("HierarchyPanel.qml")));
    auto* inspector = addDock(QStringLiteral("Inspector"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("InspectorPanel.qml")));
    auto* readiness = addDock(QStringLiteral("Readiness"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ReadinessPanel.qml")));
    auto* objects = addDock(QStringLiteral("Object Studio"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ObjectStudioPanel.qml")));
    auto* content = addDock(QStringLiteral("Content Browser"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("ContentBrowserPanel.qml")));
    auto* console = addDock(QStringLiteral("Console"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("ConsolePanel.qml")));
    auto* sprite = addDock(QStringLiteral("Sprite Studio"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("SpriteStudioPanel.qml")));
    auto* luau = addDock(QStringLiteral("Luau Studio"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("LuauStudioPanel.qml")));
    addDock(QStringLiteral("Command Palette"), Qt::TopDockWidgetArea, makeQmlPanel(QStringLiteral("CommandPalette.qml")));

    tabifyDockWidget(inspector, readiness);
    tabifyDockWidget(inspector, objects);
    tabifyDockWidget(content, console);
    tabifyDockWidget(content, sprite);
    tabifyDockWidget(content, luau);
    inspector->raise();
    content->raise();
    resizeDocks({ hierarchy, inspector }, { 320, 360 }, Qt::Horizontal);
    resizeDocks({ content, console }, { 240, 240 }, Qt::Vertical);
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
}

void MainWindow::updateStatus()
{
    if (m_projectStatus) {
        m_projectStatus->setText(m_bridge.projectSummary());
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
