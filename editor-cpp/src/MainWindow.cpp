#include "MainWindow.h"

#include <algorithm>
#include <utility>

#include <QAction>
#include <QActionGroup>
#include <QApplication>
#include <QCloseEvent>
#include <QColor>
#include <QComboBox>
#include <QCoreApplication>
#include <QDir>
#include <QDockWidget>
#include <QFileDialog>
#include <QFileInfo>
#include <QFrame>
#include <QHash>
#include <QKeySequence>
#include <QKeyEvent>
#include <QLabel>
#include <QInputDialog>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLineEdit>
#include <QMenuBar>
#include <QMessageBox>
#include <QQmlContext>
#include <QQmlError>
#include <QQuickItem>
#include <QQuickWidget>
#include <QSettings>
#include <QSignalBlocker>
#include <QShortcut>
#include <QStatusBar>
#include <QStyle>
#include <QStringList>
#include <QTabWidget>
#include <QToolBar>
#include <QToolButton>
#include <QTimer>
#include <QUrl>
#include <QVariant>
#include <QWidgetAction>

#include "ViewportWidget.h"
#include "SpriteEditorWidget.h"
#include "LuauSyntaxHighlighter.h"

namespace {

constexpr int kWorkbenchStateVersion = 7;

const QStringList& editorWorkspaceNames()
{
    static const QStringList workspaces {
        QStringLiteral("2D"),
        QStringLiteral("Scripting"),
        QStringLiteral("Animation"),
        QStringLiteral("World"),
        QStringLiteral("UI"),
        QStringLiteral("Prefab"),
        QStringLiteral("Project"),
        QStringLiteral("Assets"),
        QStringLiteral("Profiler"),
        QStringLiteral("Automation"),
        QStringLiteral("AI"),
        QStringLiteral("Build"),
        QStringLiteral("Debug"),
        QStringLiteral("Minimal"),
        QStringLiteral("Sprites"),
        QStringLiteral("Systems"),
        QStringLiteral("SDK"),
    };
    return workspaces;
}

QString shortcutHint(const QString& label, const QKeySequence& shortcut)
{
    if (shortcut.isEmpty()) {
        return label;
    }
    return QStringLiteral("%1 (%2)").arg(label, shortcut.toString(QKeySequence::NativeText));
}

QString workspaceStateKey(const QString& workspace)
{
    return QStringLiteral("workbench/workspaces/%1").arg(workspace);
}

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
    setCorner(Qt::BottomLeftCorner, Qt::BottomDockWidgetArea);
    setCorner(Qt::BottomRightCorner, Qt::RightDockWidgetArea);
    m_qmlRootPath = resolveQmlRootPath();

    applyEditorStyle();
    createMenus();
    createToolbar();

    createPanels();
    connect(&m_bridge, &MfBridge::contentAssetOpenRequested, this,
        [this](const QString&, const QString& assetType) {
            QDockWidget* targetDock = nullptr;
            if (assetType == QStringLiteral("LuauScript")) {
                targetDock = m_luauDock;
            } else if (assetType == QStringLiteral("VisualGraph")) {
                targetDock = m_blueprintsDock;
            }
            if (targetDock) {
                targetDock->show();
                targetDock->raise();
            }
        });
    QSettings settings = editorSettings();
    const QByteArray geometry = settings.value(QStringLiteral("workbench/windowGeometry")).toByteArray();
    if (!geometry.isEmpty()) {
        restoreGeometry(geometry);
    }
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
        m_autoRefreshAction->setChecked(settings.value(QStringLiteral("workbench/autoRefresh"), true).toBool());
    }
    if (m_gridAction) {
        m_gridAction->setChecked(settings.value(QStringLiteral("workbench/viewportGuides"), false).toBool());
    }
    if (m_hudAction) {
        m_hudAction->setChecked(settings.value(QStringLiteral("workbench/viewportHud"), true).toBool());
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
    connect(&m_bridge, &MfBridge::sceneStateChanged, this, &MainWindow::updateStatus);
    connect(&m_readiness, &ReadinessModel::scoreChanged, this, &MainWindow::updateStatus);
    connect(&m_bridge, &MfBridge::operationCompleted, this, [this](const QString& message, bool success) {
        statusBar()->showMessage(message, success ? 3500 : 6500);
    });
    connect(&m_bridge, &MfBridge::lastErrorChanged, this, [this] {
        if (!m_bridge.lastError().isEmpty()) {
            statusBar()->showMessage(QStringLiteral("Operation failed · %1").arg(m_bridge.lastError()), 6500);
        }
    });

    const QString startupProject = resolveStartupProjectPath(projectPath);
    if (startupProject.isEmpty()) {
        statusBar()->showMessage(QStringLiteral("No project open - use File > Open Project"));
    } else {
        openProjectPath(startupProject, false);
    }
    const QString environmentWorkspace = qEnvironmentVariable("MINIFORGE_WORKSPACE");
    const QString requestedWorkspace = environmentWorkspace.isEmpty()
        ? settings.value(QStringLiteral("workbench/lastWorkspace"), QStringLiteral("2D")).toString()
        : environmentWorkspace;
    const QStringList& workspaces = editorWorkspaceNames();
    const auto initialWorkspace = std::find_if(
        workspaces.cbegin(),
        workspaces.cend(),
        [&requestedWorkspace](const QString& workspace) {
            return workspace.compare(requestedWorkspace, Qt::CaseInsensitive) == 0;
        }
    );
    setWorkspace(initialWorkspace == workspaces.cend() ? QStringLiteral("2D") : *initialWorkspace);
    if (qEnvironmentVariableIntValue("MINIFORGE_SHOW_LAUNCHER") != 0 && m_projectLauncherDock) {
        m_projectLauncherDock->show();
        m_projectLauncherDock->raise();
    }
    if (qEnvironmentVariableIntValue("MINIFORGE_START_RUNTIME") != 0 && m_bridge.isOpen()) {
        QTimer::singleShot(0, this, [this] { m_bridge.executeCommand(QStringLiteral("play.enter")); });
    }
    updateStatus();
}

MainWindow::~MainWindow()
{
    saveWorkbenchState();
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

void MainWindow::closeEvent(QCloseEvent* event)
{
    saveWorkbenchState();
    QMainWindow::closeEvent(event);
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
    view->setClearColor(QColor(QStringLiteral("#202329")));
    view->rootContext()->setContextProperty(QStringLiteral("editorShell"), this);
    view->rootContext()->setContextProperty(QStringLiteral("editorBridge"), &m_bridge);
    view->rootContext()->setContextProperty(QStringLiteral("editorController"), &m_controller);
    view->rootContext()->setContextProperty(QStringLiteral("hierarchyModel"), &m_entities);
    view->rootContext()->setContextProperty(QStringLiteral("inspectorModel"), &m_inspector);
    view->rootContext()->setContextProperty(QStringLiteral("contentModel"), &m_assets);
    view->rootContext()->setContextProperty(QStringLiteral("commandModel"), &m_commands);
    view->rootContext()->setContextProperty(QStringLiteral("consoleModel"), &m_console);
    view->rootContext()->setContextProperty(QStringLiteral("readinessModel"), &m_readiness);
    view->rootContext()->setContextProperty(QStringLiteral("forgeAiModel"), &m_forgeAi);
    view->rootContext()->setContextProperty(
        QStringLiteral("luauSyntaxHighlighter"),
        new LuauSyntaxHighlighter(view));
    view->setSource(QUrl::fromLocalFile(QDir(m_qmlRootPath).filePath(QStringLiteral("panels/") + qmlFile)));
    if (view->status() == QQuickWidget::Error) {
        const QList<QQmlError> errors = view->errors();
        QStringList details;
        details.reserve(errors.size());
        for (const QQmlError& error : errors) {
            details.push_back(error.toString());
        }
        statusBar()->showMessage(
            QStringLiteral("Failed to load %1 · %2").arg(qmlFile, details.join(QStringLiteral(" | "))),
            8000
        );
    }
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
    const bool safeMode = qEnvironmentVariableIntValue("MINIFORGE_SAFE_MODE") != 0;
    const QString openOptions = safeMode
        ? QString::fromUtf8(QJsonDocument(QJsonObject {
              { QStringLiteral("safe_mode"), true },
              { QStringLiteral("safe_mode_reason"), QStringLiteral("Qt CLI recovery") },
              { QStringLiteral("disable_asset_importers"), false },
          }).toJson(QJsonDocument::Compact))
        : QStringLiteral("{}");
    if (!m_bridge.openProjectWithOptions(projectPath, openOptions)) {
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
    dock->setAllowedAreas(Qt::AllDockWidgetAreas);
    dock->setMinimumSize(180, 120);
    dock->setWidget(widget);
    dock->toggleViewAction()->setStatusTip(QStringLiteral("Show or hide the %1 panel").arg(title));
    addDockWidget(area, dock);
    return dock;
}

void MainWindow::createMenus()
{
    const auto addMenuCommand = [this](
                                    QMenu* menu,
                                    const QString& label,
                                    const QString& commandId,
                                    const QKeySequence& shortcut = {},
                                    bool requiresSelection = false) {
        auto* action = menu->addAction(label);
        action->setObjectName(QStringLiteral("Command.%1").arg(commandId));
        if (!shortcut.isEmpty()) {
            action->setShortcut(shortcut);
            action->setShortcutContext(Qt::ApplicationShortcut);
        }
        action->setStatusTip(shortcutHint(label, shortcut));
        action->setToolTip(shortcutHint(label, shortcut));
        connect(action, &QAction::triggered, this, [this, commandId] {
            m_bridge.executeCommand(commandId);
        });
        m_projectActions.push_back(action);
        if (requiresSelection) {
            m_selectionActions.push_back(action);
        }
        return action;
    };

    const auto addObjectCommands = [&addMenuCommand](QMenu* menu) {
        auto* core2d = menu->addMenu(QStringLiteral("Core 2D"));
        addMenuCommand(core2d, QStringLiteral("Empty Entity"), QStringLiteral("entity.create_empty"));
        addMenuCommand(core2d, QStringLiteral("Node2D"), QStringLiteral("object.create_node2d"));
        addMenuCommand(core2d, QStringLiteral("Sprite Actor"), QStringLiteral("object.create_sprite_actor"));
        addMenuCommand(core2d, QStringLiteral("Camera Rig"), QStringLiteral("object.create_camera"));
        addMenuCommand(core2d, QStringLiteral("Camera to Texture 2D"), QStringLiteral("object.create_camera_texture2d"));
        addMenuCommand(core2d, QStringLiteral("HUD Text"), QStringLiteral("object.create_ui_text"));

        auto* physics = menu->addMenu(QStringLiteral("Physics & Sensors"));
        addMenuCommand(physics, QStringLiteral("CharacterBody2D"), QStringLiteral("object.create_character_body2d"));
        addMenuCommand(physics, QStringLiteral("Rigidbody2D"), QStringLiteral("object.create_rigidbody2d"));
        addMenuCommand(physics, QStringLiteral("StaticBody2D"), QStringLiteral("object.create_static_body2d"));
        addMenuCommand(physics, QStringLiteral("Area2D"), QStringLiteral("object.create_area2d"));
        addMenuCommand(physics, QStringLiteral("Trigger Volume 2D"), QStringLiteral("object.create_trigger_volume2d"));
        addMenuCommand(physics, QStringLiteral("One-Way Platform 2D"), QStringLiteral("object.create_one_way_platform2d"));

        auto* lighting = menu->addMenu(QStringLiteral("Lighting & Raycast Shadows"));
        addMenuCommand(lighting, QStringLiteral("Point Light 2D"), QStringLiteral("object.create_point_light2d"));
        addMenuCommand(lighting, QStringLiteral("Spot Light 2D"), QStringLiteral("object.create_spot_light2d"));
        addMenuCommand(lighting, QStringLiteral("Lit Sprite 2D (Normal Map)"), QStringLiteral("object.create_lit_sprite2d"));
        addMenuCommand(lighting, QStringLiteral("Shadow Occluder 2D"), QStringLiteral("object.create_shadow_occluder2d"));

        auto* ai = menu->addMenu(QStringLiteral("AI & Navigation"));
        addMenuCommand(ai, QStringLiteral("Navigation Agent 2D"), QStringLiteral("object.create_nav_agent2d"));

        auto* effects = menu->addMenu(QStringLiteral("Effects & Audio"));
        addMenuCommand(effects, QStringLiteral("Particle Emitter 2D"), QStringLiteral("object.create_particle_emitter2d"));
        addMenuCommand(effects, QStringLiteral("GPU Particle Emitter 2D"), QStringLiteral("object.create_gpu_particle_emitter2d"));
        addMenuCommand(effects, QStringLiteral("Post Process Volume 2D"), QStringLiteral("object.create_post_process_volume2d"));
        addMenuCommand(effects, QStringLiteral("Spatial Audio Emitter 2D"), QStringLiteral("object.create_audio_emitter2d"));

        auto* survival = menu->addMenu(QStringLiteral("Survival"));
        addMenuCommand(survival, QStringLiteral("Survival Actor 2D"), QStringLiteral("object.create_survival_actor2d"));
        addMenuCommand(survival, QStringLiteral("Survival Hunter / Zombie 2D"), QStringLiteral("object.create_survival_hunter2d"));
        addMenuCommand(survival, QStringLiteral("Barricadable Door 2D"), QStringLiteral("object.create_survival_door2d"));
        addMenuCommand(survival, QStringLiteral("Survival Environment 2D"), QStringLiteral("object.create_survival_environment2d"));

        auto* hybrid = menu->addMenu(QStringLiteral("Hybrid 2D + 3D"));
        addMenuCommand(hybrid, QStringLiteral("Hybrid World 2D + 3D"), QStringLiteral("object.create_hybrid_world2d3d"));
        addMenuCommand(hybrid, QStringLiteral("Hybrid Billboard Actor"), QStringLiteral("object.create_hybrid_billboard3d"));
    };

    const auto addExportAction = [this](QMenu* menu, const QString& label, const QString& profile) {
        QAction* action = menu->addAction(label);
        action->setObjectName(QStringLiteral("Export.%1").arg(profile));
        connect(action, &QAction::triggered, this, [this, profile] {
            m_bridge.exportRuntime(profile);
        });
        m_projectActions.push_back(action);
        return action;
    };

    const auto addSelectionAction = [this](
                                        QMenu* menu,
                                        const QString& label,
                                        const QString& entityAction,
                                        const QKeySequence& shortcut = {}) {
        QAction* action = menu->addAction(label);
        action->setObjectName(QStringLiteral("Selection.%1").arg(entityAction));
        if (!shortcut.isEmpty()) {
            action->setShortcut(shortcut);
            action->setShortcutContext(Qt::ApplicationShortcut);
        }
        action->setStatusTip(shortcutHint(label, shortcut));
        action->setToolTip(shortcutHint(label, shortcut));
        connect(action, &QAction::triggered, this, [this, entityAction] {
            if (entityAction == QStringLiteral("duplicate") || entityAction == QStringLiteral("delete")) {
                m_bridge.performSelectedEntityAction(entityAction);
                return;
            }
            const QVector<quint64> selected = m_bridge.selectedEntities();
            for (quint64 entityId : selected) {
                m_bridge.performEntityAction(entityId, entityAction);
            }
        });
        m_projectActions.push_back(action);
        m_selectionActions.push_back(action);
        return action;
    };

    auto* file = menuBar()->addMenu(QStringLiteral("&File"));
    auto* openProject = file->addAction(QStringLiteral("Open Project..."));
    openProject->setShortcut(QKeySequence::Open);
    openProject->setShortcutContext(Qt::ApplicationShortcut);
    openProject->setStatusTip(QStringLiteral("Open an existing MiniForge project folder"));
    connect(openProject, &QAction::triggered, this, &MainWindow::chooseProject);
    file->addSeparator();
    auto* create = file->addMenu(QStringLiteral("New"));
    auto* sceneObjects = create->addMenu(QStringLiteral("Scene Object"));
    addObjectCommands(sceneObjects);
    auto* asset = create->addMenu(QStringLiteral("Asset"));
    addMenuCommand(asset, QStringLiteral("Pixel Art Sprite"), QStringLiteral("sprite.new_pixel_art"));
    addMenuCommand(asset, QStringLiteral("Hero Sprite Template"), QStringLiteral("sprite.create_hero_template"));
    addMenuCommand(asset, QStringLiteral("Luau 2D Controller"), QStringLiteral("luau.new_controller"));
    file->addSeparator();
    addMenuCommand(file, QStringLiteral("Save Scene"), QStringLiteral("scene.save"), QKeySequence::Save);
    addMenuCommand(file, QStringLiteral("Save All"), QStringLiteral("project.save"), QKeySequence::SaveAs);
    auto* exportMenu = file->addMenu(QStringLiteral("Export Runtime"));
    addExportAction(exportMenu, QStringLiteral("Debug"), QStringLiteral("debug"));
    addExportAction(exportMenu, QStringLiteral("Release"), QStringLiteral("release"));
    addExportAction(exportMenu, QStringLiteral("Shipping"), QStringLiteral("shipping"));
    file->addSeparator();
    auto* exitAction = file->addAction(QStringLiteral("Exit MiniForge"));
    exitAction->setShortcut(QKeySequence::Quit);
    exitAction->setShortcutContext(Qt::ApplicationShortcut);
    connect(exitAction, &QAction::triggered, this, &QWidget::close);

    auto* edit = menuBar()->addMenu(QStringLiteral("&Edit"));
    addMenuCommand(edit, QStringLiteral("Undo"), QStringLiteral("edit.undo"), QKeySequence::Undo);
    addMenuCommand(edit, QStringLiteral("Redo"), QStringLiteral("edit.redo"), QKeySequence::Redo);
    edit->addSeparator();
    addSelectionAction(edit, QStringLiteral("Duplicate Selected"), QStringLiteral("duplicate"), QKeySequence(QStringLiteral("Ctrl+D")));
    addSelectionAction(edit, QStringLiteral("Delete Selected"), QStringLiteral("delete"), QKeySequence::Delete);
    addSelectionAction(edit, QStringLiteral("Reset Transform"), QStringLiteral("reset_transform"));
    addSelectionAction(edit, QStringLiteral("Move to Scene Root"), QStringLiteral("unparent"));
    auto* renameSelected = edit->addAction(QStringLiteral("Rename Selected..."));
    renameSelected->setShortcut(QKeySequence(QStringLiteral("F2")));
    connect(renameSelected, &QAction::triggered, this, [this] {
        const quint64 entityId = m_bridge.selectedEntityId();
        if (entityId == 0) {
            return;
        }
        bool accepted = false;
        const QString name = QInputDialog::getText(
            this,
            QStringLiteral("Rename Entity"),
            QStringLiteral("Name"),
            QLineEdit::Normal,
            QString(),
            &accepted
        ).trimmed();
        if (!accepted || name.isEmpty()) {
            return;
        }
        const QByteArray payload = QJsonDocument(QJsonObject { { QStringLiteral("name"), name } })
                                       .toJson(QJsonDocument::Compact);
        m_bridge.performEntityAction(entityId, QStringLiteral("rename"), QString::fromUtf8(payload));
    });
    m_projectActions.push_back(renameSelected);
    m_selectionActions.push_back(renameSelected);
    auto* clearSelection = edit->addAction(QStringLiteral("Clear Selection"));
    clearSelection->setShortcut(QKeySequence(QStringLiteral("Escape")));
    connect(clearSelection, &QAction::triggered, &m_bridge, &MfBridge::clearSelection);
    m_projectActions.push_back(clearSelection);
    m_selectionActions.push_back(clearSelection);
    edit->addSeparator();
    auto* arrange = edit->addMenu(QStringLiteral("Arrange Selection"));
    addMenuCommand(arrange, QStringLiteral("Align Left"), QStringLiteral("selection.align_left"), {}, true);
    addMenuCommand(arrange, QStringLiteral("Align Center X"), QStringLiteral("selection.align_center_x"), {}, true);
    addMenuCommand(arrange, QStringLiteral("Align Right"), QStringLiteral("selection.align_right"), {}, true);
    arrange->addSeparator();
    addMenuCommand(arrange, QStringLiteral("Align Top"), QStringLiteral("selection.align_top"), {}, true);
    addMenuCommand(arrange, QStringLiteral("Align Center Y"), QStringLiteral("selection.align_center_y"), {}, true);
    addMenuCommand(arrange, QStringLiteral("Align Bottom"), QStringLiteral("selection.align_bottom"), {}, true);
    arrange->addSeparator();
    addMenuCommand(arrange, QStringLiteral("Distribute Horizontally"), QStringLiteral("selection.distribute_x"), {}, true);
    addMenuCommand(arrange, QStringLiteral("Distribute Vertically"), QStringLiteral("selection.distribute_y"), {}, true);
    auto* grouping = edit->addMenu(QStringLiteral("Groups && Layers"));
    addMenuCommand(grouping, QStringLiteral("Group"), QStringLiteral("selection.group"), QKeySequence(QStringLiteral("Ctrl+Shift+G")), true);
    addMenuCommand(grouping, QStringLiteral("Ungroup"), QStringLiteral("selection.ungroup"), QKeySequence(QStringLiteral("Ctrl+Shift+U")), true);
    grouping->addSeparator();
    addMenuCommand(grouping, QStringLiteral("Move to Next Layer"), QStringLiteral("selection.cycle_layer"), {}, true);
    addMenuCommand(grouping, QStringLiteral("Toggle Layer Lock"), QStringLiteral("selection.toggle_layer_lock"), {}, true);
    addMenuCommand(grouping, QStringLiteral("Toggle Layer Visibility"), QStringLiteral("selection.toggle_layer_visibility"), {}, true);
    edit->addSeparator();
    auto* spriteTools = edit->addMenu(QStringLiteral("Sprite Tools"));
    addMenuCommand(spriteTools, QStringLiteral("Create SpriteFrames"), QStringLiteral("sprite.export_frames"));
    addMenuCommand(spriteTools, QStringLiteral("Export Atlas Pages"), QStringLiteral("sprite.export_atlas_pages"));
    addMenuCommand(spriteTools, QStringLiteral("Create Palette Ramp"), QStringLiteral("sprite.optimize_palette"));

    auto* scene = menuBar()->addMenu(QStringLiteral("&Scene"));
    addMenuCommand(scene, QStringLiteral("Save Scene"), QStringLiteral("scene.save"));
    addMenuCommand(scene, QStringLiteral("Audit Scene Tree"), QStringLiteral("scene.audit_tree"));
    addMenuCommand(scene, QStringLiteral("Pack Selected Branch"), QStringLiteral("scene.pack_selected"), {}, true);
    scene->addSeparator();
    auto* sceneCreate = scene->addMenu(QStringLiteral("Create"));
    addObjectCommands(sceneCreate);
    auto* starterScenes = scene->addMenu(QStringLiteral("Starter Scene"));
    addMenuCommand(starterScenes, QStringLiteral("Top-Down Starter"), QStringLiteral("scene.starter_topdown"));
    addMenuCommand(starterScenes, QStringLiteral("Platformer Starter"), QStringLiteral("scene.starter_platformer"));
    addMenuCommand(starterScenes, QStringLiteral("RTS Skirmish Starter"), QStringLiteral("scene.starter_rts"));
    scene->addSeparator();
    addMenuCommand(scene, QStringLiteral("Enter Play Mode"), QStringLiteral("play.enter"), QKeySequence(QStringLiteral("F5")));
    addMenuCommand(scene, QStringLiteral("Stop Play Mode"), QStringLiteral("play.stop"), QKeySequence(QStringLiteral("Shift+F5")));
    scene->addSeparator();
    auto* sceneBrowser = scene->addAction(QStringLiteral("Scene Browser..."));
    sceneBrowser->setShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+O")));
    sceneBrowser->setShortcutContext(Qt::ApplicationShortcut);
    connect(sceneBrowser, &QAction::triggered, this, [this] {
        setWorkspace(QStringLiteral("2D"));
        if (m_sceneBrowserDock) {
            m_sceneBrowserDock->show();
            m_sceneBrowserDock->raise();
        }
    });

    auto* project = menuBar()->addMenu(QStringLiteral("&Project"));
    addMenuCommand(project, QStringLiteral("Run Audit"), QStringLiteral("project.audit"));
    addMenuCommand(project, QStringLiteral("Refresh Assets"), QStringLiteral("assets.refresh"));
    addMenuCommand(project, QStringLiteral("Validate Luau Scripts"), QStringLiteral("luau.validate_scripts"));
    addMenuCommand(project, QStringLiteral("Write 2D Render Profile"), QStringLiteral("render.write_2d_profile"));
    auto* projectTemplates = project->addMenu(QStringLiteral("Create from Template"));
    addMenuCommand(projectTemplates, QStringLiteral("RTS Project"), QStringLiteral("project.template_rts"));
    addMenuCommand(projectTemplates, QStringLiteral("Action RPG Project"), QStringLiteral("project.template_actionrpg"));
    addMenuCommand(projectTemplates, QStringLiteral("Survival Project"), QStringLiteral("project.template_survival"));
    project->addSeparator();
    auto* projectSettings = project->addAction(QStringLiteral("Project Settings..."));
    projectSettings->setShortcut(QKeySequence(QStringLiteral("Ctrl+,")));
    projectSettings->setShortcutContext(Qt::ApplicationShortcut);
    connect(projectSettings, &QAction::triggered, this, [this] { setWorkspace(QStringLiteral("Project")); });
    auto* projectLauncher = project->addAction(QStringLiteral("Project Launcher..."));
    connect(projectLauncher, &QAction::triggered, this, [this] {
        if (m_projectLauncherDock) {
            m_projectLauncherDock->show();
            m_projectLauncherDock->raise();
        }
    });
    auto* projectOperations = project->addAction(QStringLiteral("Project Operations..."));
    connect(projectOperations, &QAction::triggered, this, [this] {
        setWorkspace(QStringLiteral("Project"));
        if (m_projectOperationsDock) {
            m_projectOperationsDock->show();
            m_projectOperationsDock->raise();
        }
    });
    auto* sdkPacks = project->addAction(QStringLiteral("SDK && Content Packs..."));
    connect(sdkPacks, &QAction::triggered, this, [this] {
        setWorkspace(QStringLiteral("SDK"));
        focusDock(m_sdkPacksDock);
    });

    auto* systems = menuBar()->addMenu(QStringLiteral("&Systems"));
    auto* allSystems = systems->addAction(QStringLiteral("Mega Authoring Hub..."));
    connect(allSystems, &QAction::triggered, this, [this] { openAuthoringHub(); });
    systems->addSeparator();
    for (const auto& entry : {
             std::pair { QStringLiteral("Players && Actors"), QStringLiteral("actor") },
             std::pair { QStringLiteral("Gameplay Systems"), QStringLiteral("gameplay") },
             std::pair { QStringLiteral("Physics Profiles"), QStringLiteral("physics") },
             std::pair { QStringLiteral("World Building"), QStringLiteral("world") },
             std::pair { QStringLiteral("Effects && Audio"), QStringLiteral("effects") },
             std::pair { QStringLiteral("User Interface"), QStringLiteral("user_interface") },
             std::pair { QStringLiteral("Strategy && RTS"), QStringLiteral("strategy") },
         }) {
        QAction* action = systems->addAction(entry.first);
        connect(action, &QAction::triggered, this, [this, kind = entry.second] {
            openAuthoringHub(kind);
        });
    }

    auto* build = menuBar()->addMenu(QStringLiteral("&Build"));
    auto* openBuild = build->addAction(QStringLiteral("Build && Export..."));
    connect(openBuild, &QAction::triggered, this, [this] {
        setWorkspace(QStringLiteral("Build"));
    });
    m_projectActions.push_back(openBuild);
    auto* packages = build->addAction(QStringLiteral("Packages && External Play..."));
    connect(packages, &QAction::triggered, this, [this] {
        setWorkspace(QStringLiteral("Project"));
        if (m_projectOperationsDock) {
            m_projectOperationsDock->show();
            m_projectOperationsDock->raise();
        }
    });
    m_projectActions.push_back(packages);
    build->addSeparator();
    addExportAction(build, QStringLiteral("Export Debug"), QStringLiteral("debug"));
    addExportAction(build, QStringLiteral("Export Release"), QStringLiteral("release"));
    addExportAction(build, QStringLiteral("Export Shipping"), QStringLiteral("shipping"));

    auto* forgeAi = menuBar()->addMenu(QStringLiteral("Forge &AI"));
    auto* runDoctor = forgeAi->addAction(QStringLiteral("Run Project Doctor"));
    connect(runDoctor, &QAction::triggered, this, [this] {
        m_forgeAi.runDoctor();
        setWorkspace(QStringLiteral("AI"));
    });
    m_projectActions.push_back(runDoctor);
    auto* runEnemySmoke = forgeAi->addAction(QStringLiteral("Run Enemy Smoke Test"));
    connect(runEnemySmoke, &QAction::triggered, this, [this] {
        m_forgeAi.runEnemySmoke();
        setWorkspace(QStringLiteral("AI"));
    });
    m_projectActions.push_back(runEnemySmoke);

    auto* objects = menuBar()->addMenu(QStringLiteral("&Create"));
    addObjectCommands(objects);
    auto* gameplayActors = objects->addMenu(QStringLiteral("Gameplay Actor"));
    addMenuCommand(gameplayActors, QStringLiteral("Player Unit"), QStringLiteral("gameplay.spawn_unit"));
    addMenuCommand(gameplayActors, QStringLiteral("Enemy"), QStringLiteral("gameplay.spawn_enemy"));
    addMenuCommand(gameplayActors, QStringLiteral("Resource Node"), QStringLiteral("gameplay.spawn_resource"));
    auto* rtsObjects = objects->addMenu(QStringLiteral("RTS"));
    addMenuCommand(rtsObjects, QStringLiteral("Base"), QStringLiteral("rts.spawn_base"));
    addMenuCommand(rtsObjects, QStringLiteral("Queue Worker"), QStringLiteral("rts.queue_worker"));
    addMenuCommand(rtsObjects, QStringLiteral("Barracks"), QStringLiteral("rts.place_barracks"));
    objects->addSeparator();
    auto* spriteAssets = objects->addMenu(QStringLiteral("Sprite Asset"));
    addMenuCommand(spriteAssets, QStringLiteral("Pixel Art Sprite"), QStringLiteral("sprite.new_pixel_art"));
    addMenuCommand(spriteAssets, QStringLiteral("Hero Template"), QStringLiteral("sprite.create_hero_template"));
    addMenuCommand(objects, QStringLiteral("Luau 2D Controller"), QStringLiteral("luau.new_controller"));

    auto* tools = menuBar()->addMenu(QStringLiteral("&Tools"));
    const auto addToolWorkspace = [this, tools](const QString& label, const QString& workspaceName) {
        QAction* action = tools->addAction(label);
        connect(action, &QAction::triggered, this, [this, workspaceName] { setWorkspace(workspaceName); });
    };
    addToolWorkspace(QStringLiteral("Tilemap && Terrain"), QStringLiteral("World"));
    addToolWorkspace(QStringLiteral("Animation Timeline"), QStringLiteral("Animation"));
    addToolWorkspace(QStringLiteral("Sprite Studio"), QStringLiteral("Sprites"));
    addToolWorkspace(QStringLiteral("UI Designer"), QStringLiteral("UI"));
    addToolWorkspace(QStringLiteral("Prefab Studio"), QStringLiteral("Prefab"));
    addToolWorkspace(QStringLiteral("Blueprints / Visual Graph"), QStringLiteral("Scripting"));
    tools->addSeparator();
    addToolWorkspace(QStringLiteral("Project Settings"), QStringLiteral("Project"));
    addToolWorkspace(QStringLiteral("Asset Management && Dependencies"), QStringLiteral("Assets"));
    addToolWorkspace(QStringLiteral("Profiler"), QStringLiteral("Profiler"));
    addToolWorkspace(QStringLiteral("Python Automation"), QStringLiteral("Automation"));
    tools->addSeparator();
    addToolWorkspace(QStringLiteral("Mega Authoring Hub"), QStringLiteral("Systems"));
    addToolWorkspace(QStringLiteral("SDK && Content Packs"), QStringLiteral("SDK"));

    auto* workspace = menuBar()->addMenu(QStringLiteral("&Workspace"));
    auto* workspaceGroup = new QActionGroup(this);
    const QStringList workspaceShortcuts {
        QStringLiteral("Ctrl+1"), QStringLiteral("Ctrl+2"), QStringLiteral("Ctrl+3"), QStringLiteral("Ctrl+4"),
        QStringLiteral("Ctrl+5"), QStringLiteral("Ctrl+6"), QStringLiteral("Ctrl+7"), QStringLiteral("Ctrl+8"),
        QStringLiteral("Ctrl+9"), QStringLiteral("Ctrl+0"), QString(), QString(), QString(), QString(),
    };
    const QStringList& workspaces = editorWorkspaceNames();
    for (qsizetype index = 0; index < workspaces.size(); ++index) {
        const QString& name = workspaces.at(index);
        QAction* action = workspace->addAction(name);
        action->setObjectName(QStringLiteral("Workspace.%1").arg(name));
        action->setCheckable(true);
        action->setActionGroup(workspaceGroup);
        if (index < workspaceShortcuts.size() && !workspaceShortcuts.at(index).isEmpty()) {
            action->setShortcut(QKeySequence(workspaceShortcuts.at(index)));
            action->setShortcutContext(Qt::ApplicationShortcut);
        }
        action->setStatusTip(QStringLiteral("Switch to the %1 workspace").arg(name));
        if (name == QStringLiteral("2D")) {
            action->setChecked(true);
        }
        connect(action, &QAction::triggered, this, [this, name] {
            setWorkspace(name);
        });
    }

    auto* view = menuBar()->addMenu(QStringLiteral("&View"));
    auto* commandPalette = view->addAction(QStringLiteral("Command Palette"));
    commandPalette->setShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+P")));
    commandPalette->setShortcutContext(Qt::ApplicationShortcut);
    connect(commandPalette, &QAction::triggered, this, &MainWindow::showCommandPalette);
    m_panelMenu = view->addMenu(QStringLiteral("Panels"));
    view->addSeparator();
    m_gridAction = view->addAction(QStringLiteral("Guides"));
    m_gridAction->setCheckable(true);
    m_gridAction->setChecked(false);
    m_gridAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+Alt+G")));
    m_gridAction->setShortcutContext(Qt::ApplicationShortcut);
    m_gridAction->setToolTip(QStringLiteral("Toggle viewport guides (Ctrl+Alt+G)"));
    m_hudAction = view->addAction(QStringLiteral("HUD"));
    m_hudAction->setCheckable(true);
    m_hudAction->setChecked(true);
    m_hudAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+H")));
    m_hudAction->setShortcutContext(Qt::ApplicationShortcut);
    m_hudAction->setToolTip(QStringLiteral("Toggle viewport HUD (Ctrl+H)"));
    view->addSeparator();
    m_autoRefreshAction = view->addAction(QStringLiteral("Auto Refresh Workbench"));
    m_autoRefreshAction->setCheckable(true);
    m_autoRefreshAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+Alt+R")));
    m_autoRefreshAction->setShortcutContext(Qt::ApplicationShortcut);
    QAction* refreshNow = view->addAction(QStringLiteral("Refresh Now"));
    refreshNow->setShortcut(QKeySequence::Refresh);
    connect(refreshNow, &QAction::triggered, this, &MainWindow::refreshWorkbench);
    view->addSeparator();
    auto* resetWorkspace = view->addAction(QStringLiteral("Reset Current Workspace"));
    connect(resetWorkspace, &QAction::triggered, this, &MainWindow::resetCurrentWorkspace);
    auto* fullScreen = view->addAction(QStringLiteral("Toggle Full Screen"));
    fullScreen->setShortcut(QKeySequence(QStringLiteral("F11")));
    fullScreen->setShortcutContext(Qt::ApplicationShortcut);
    connect(fullScreen, &QAction::triggered, this, [this] {
        isFullScreen() ? showNormal() : showFullScreen();
    });

    auto* help = menuBar()->addMenu(QStringLiteral("&Help"));
    auto* about = help->addAction(QStringLiteral("About MiniForge"));
    connect(about, &QAction::triggered, this, [this] {
        QMessageBox::about(
            this,
            QStringLiteral("About MiniForge"),
            QStringLiteral("MiniForge 0.9.3.4\nQt/C++ editor powered by the Rust EditorCore runtime.")
        );
    });
}

void MainWindow::createToolbar()
{
    m_mainToolbar = addToolBar(QStringLiteral("Level Editor"));
    m_mainToolbar->setObjectName(QStringLiteral("LevelEditorToolbar"));
    m_mainToolbar->setMovable(true);
    m_mainToolbar->setFloatable(false);
    m_mainToolbar->setAllowedAreas(Qt::TopToolBarArea);
    m_mainToolbar->setIconSize(QSize(17, 17));
    m_mainToolbar->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);

    const auto addCommand = [this](const QString& label,
                                const QString& commandId,
                                QStyle::StandardPixmap icon,
                                const QString& shortcutText = QString()) {
        auto* action = m_mainToolbar->addAction(style()->standardIcon(icon), label, this, [this, commandId] {
            m_bridge.executeCommand(commandId);
        });
        action->setObjectName(QStringLiteral("Command.%1").arg(commandId));
        action->setToolTip(shortcutText.isEmpty()
            ? label
            : QStringLiteral("%1 (%2)").arg(label, shortcutText));
        action->setStatusTip(action->toolTip());
        m_projectActions.push_back(action);
        return action;
    };

    addCommand(
        QStringLiteral("Save"),
        QStringLiteral("scene.save"),
        QStyle::SP_DialogSaveButton,
        QKeySequence(QKeySequence::Save).toString(QKeySequence::NativeText));

    auto* addMenu = new QMenu(QStringLiteral("Add"), m_mainToolbar);
    const auto addCreateAction = [this](QMenu* targetMenu, const QString& label, const QString& commandId) {
        QAction* action = targetMenu->addAction(label);
        action->setObjectName(QStringLiteral("ToolbarCommand.%1").arg(commandId));
        action->setStatusTip(QStringLiteral("Create %1 in the active scene").arg(label));
        connect(action, &QAction::triggered, this, [this, commandId] { m_bridge.executeCommand(commandId); });
        m_projectActions.push_back(action);
    };
    auto* core2dMenu = addMenu->addMenu(QStringLiteral("Core 2D"));
    addCreateAction(core2dMenu, QStringLiteral("Empty Entity"), QStringLiteral("entity.create_empty"));
    addCreateAction(core2dMenu, QStringLiteral("Node2D"), QStringLiteral("object.create_node2d"));
    addCreateAction(core2dMenu, QStringLiteral("Sprite Actor"), QStringLiteral("object.create_sprite_actor"));
    addCreateAction(core2dMenu, QStringLiteral("Camera Rig"), QStringLiteral("object.create_camera"));
    addCreateAction(core2dMenu, QStringLiteral("Camera to Texture 2D"), QStringLiteral("object.create_camera_texture2d"));
    addCreateAction(core2dMenu, QStringLiteral("HUD Text"), QStringLiteral("object.create_ui_text"));

    auto* physicsMenu = addMenu->addMenu(QStringLiteral("Physics & Sensors"));
    addCreateAction(physicsMenu, QStringLiteral("CharacterBody2D"), QStringLiteral("object.create_character_body2d"));
    addCreateAction(physicsMenu, QStringLiteral("Rigidbody2D"), QStringLiteral("object.create_rigidbody2d"));
    addCreateAction(physicsMenu, QStringLiteral("StaticBody2D"), QStringLiteral("object.create_static_body2d"));
    addCreateAction(physicsMenu, QStringLiteral("Area2D"), QStringLiteral("object.create_area2d"));
    addCreateAction(physicsMenu, QStringLiteral("Trigger Volume 2D"), QStringLiteral("object.create_trigger_volume2d"));
    addCreateAction(physicsMenu, QStringLiteral("One-Way Platform 2D"), QStringLiteral("object.create_one_way_platform2d"));

    auto* lightingMenu = addMenu->addMenu(QStringLiteral("Lighting & Raycast Shadows"));
    addCreateAction(lightingMenu, QStringLiteral("Point Light 2D"), QStringLiteral("object.create_point_light2d"));
    addCreateAction(lightingMenu, QStringLiteral("Spot Light 2D"), QStringLiteral("object.create_spot_light2d"));
    addCreateAction(lightingMenu, QStringLiteral("Lit Sprite 2D (Normal Map)"), QStringLiteral("object.create_lit_sprite2d"));
    addCreateAction(lightingMenu, QStringLiteral("Shadow Occluder 2D"), QStringLiteral("object.create_shadow_occluder2d"));

    auto* aiMenu = addMenu->addMenu(QStringLiteral("AI & Navigation"));
    addCreateAction(aiMenu, QStringLiteral("Navigation Agent 2D"), QStringLiteral("object.create_nav_agent2d"));

    auto* effectsMenu = addMenu->addMenu(QStringLiteral("Effects & Audio"));
    addCreateAction(effectsMenu, QStringLiteral("Particle Emitter 2D"), QStringLiteral("object.create_particle_emitter2d"));
    addCreateAction(effectsMenu, QStringLiteral("GPU Particle Emitter 2D"), QStringLiteral("object.create_gpu_particle_emitter2d"));
    addCreateAction(effectsMenu, QStringLiteral("Post Process Volume 2D"), QStringLiteral("object.create_post_process_volume2d"));
    addCreateAction(effectsMenu, QStringLiteral("Spatial Audio Emitter 2D"), QStringLiteral("object.create_audio_emitter2d"));

    auto* survivalMenu = addMenu->addMenu(QStringLiteral("Survival"));
    addCreateAction(survivalMenu, QStringLiteral("Survival Actor 2D"), QStringLiteral("object.create_survival_actor2d"));
    addCreateAction(survivalMenu, QStringLiteral("Survival Hunter / Zombie 2D"), QStringLiteral("object.create_survival_hunter2d"));
    addCreateAction(survivalMenu, QStringLiteral("Barricadable Door 2D"), QStringLiteral("object.create_survival_door2d"));
    addCreateAction(survivalMenu, QStringLiteral("Survival Environment 2D"), QStringLiteral("object.create_survival_environment2d"));

    auto* hybridMenu = addMenu->addMenu(QStringLiteral("Hybrid 2D + 3D"));
    addCreateAction(hybridMenu, QStringLiteral("Hybrid World 2D + 3D"), QStringLiteral("object.create_hybrid_world2d3d"));
    addCreateAction(hybridMenu, QStringLiteral("Hybrid Billboard Actor"), QStringLiteral("object.create_hybrid_billboard3d"));

    auto* gameplayMenu = addMenu->addMenu(QStringLiteral("Gameplay"));
    const auto addGameplayAction = [this, gameplayMenu](const QString& label, const QString& commandId) {
        QAction* action = gameplayMenu->addAction(label);
        connect(action, &QAction::triggered, this, [this, commandId] { m_bridge.executeCommand(commandId); });
        m_projectActions.push_back(action);
    };
    addGameplayAction(QStringLiteral("Player Unit"), QStringLiteral("gameplay.spawn_unit"));
    addGameplayAction(QStringLiteral("Enemy"), QStringLiteral("gameplay.spawn_enemy"));
    addGameplayAction(QStringLiteral("Resource Node"), QStringLiteral("gameplay.spawn_resource"));
    auto* addButton = new QToolButton(m_mainToolbar);
    addButton->setObjectName(QStringLiteral("AddObjectButton"));
    addButton->setText(QStringLiteral("Add"));
    addButton->setIcon(style()->standardIcon(QStyle::SP_FileDialogNewFolder));
    addButton->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    addButton->setPopupMode(QToolButton::InstantPopup);
    addButton->setMenu(addMenu);
    addButton->setToolTip(QStringLiteral("Add an object to the active scene"));
    m_mainToolbar->addWidget(addButton);

    auto* contentButton = new QToolButton(m_mainToolbar);
    contentButton->setObjectName(QStringLiteral("ContentDrawerButton"));
    contentButton->setText(QStringLiteral("Content"));
    contentButton->setIcon(style()->standardIcon(QStyle::SP_DirOpenIcon));
    contentButton->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    contentButton->setToolTip(QStringLiteral("Open Content Browser (Ctrl+Space)"));
    connect(contentButton, &QToolButton::clicked, this, [this] { focusDock(m_contentDock); });
    m_mainToolbar->addWidget(contentButton);

    m_mainToolbar->addSeparator();
    m_workspaceSelector = new QComboBox(m_mainToolbar);
    m_workspaceSelector->setObjectName(QStringLiteral("WorkspaceSelector"));
    m_workspaceSelector->setMinimumWidth(126);
    m_workspaceSelector->addItems(editorWorkspaceNames());
    m_workspaceSelector->setToolTip(QStringLiteral("Switch editor workspace (Ctrl+1 ... Ctrl+0)"));
    connect(m_workspaceSelector, &QComboBox::currentTextChanged, this, &MainWindow::setWorkspace);
    m_mainToolbar->addWidget(m_workspaceSelector);

    m_mainToolbar->addSeparator();
    const auto addViewportTool = [this](const QString& label, int key) {
        QAction* action = m_mainToolbar->addAction(label);
        action->setObjectName(QStringLiteral("ViewportTool.%1").arg(label));
        action->setToolTip(QStringLiteral("%1 tool (%2 while the Scene viewport is focused)")
                               .arg(label, QKeySequence(key).toString(QKeySequence::NativeText)));
        connect(action, &QAction::triggered, this, [this, key] { activateViewportTool(key); });
        return action;
    };
    addViewportTool(QStringLiteral("Select"), Qt::Key_Q);
    addViewportTool(QStringLiteral("Move"), Qt::Key_W);
    addViewportTool(QStringLiteral("Rotate"), Qt::Key_E);
    addViewportTool(QStringLiteral("Scale"), Qt::Key_R);

    if (m_gridAction) {
        m_mainToolbar->addAction(m_gridAction);
    }
    if (m_hudAction) {
        m_mainToolbar->addAction(m_hudAction);
    }

    auto* spacer = new QWidget(m_mainToolbar);
    spacer->setObjectName(QStringLiteral("ToolbarSpacer"));
    spacer->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    m_mainToolbar->addWidget(spacer);

    addCommand(QStringLiteral("Play"), QStringLiteral("play.enter"), QStyle::SP_MediaPlay, QStringLiteral("F5"));
    addCommand(QStringLiteral("Stop"), QStringLiteral("play.stop"), QStyle::SP_MediaStop, QStringLiteral("Shift+F5"));

    auto* panelsButton = new QToolButton(m_mainToolbar);
    panelsButton->setObjectName(QStringLiteral("PanelsButton"));
    panelsButton->setText(QStringLiteral("Panels"));
    panelsButton->setIcon(style()->standardIcon(QStyle::SP_FileDialogDetailedView));
    panelsButton->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    panelsButton->setPopupMode(QToolButton::InstantPopup);
    panelsButton->setMenu(m_panelMenu);
    panelsButton->setToolTip(QStringLiteral("Show, hide, or focus editor panels"));
    m_mainToolbar->addWidget(panelsButton);
}

void MainWindow::createPanels()
{
    m_viewportTabs = new QTabWidget(this);
    m_viewportTabs->setObjectName(QStringLiteral("ViewportTabs"));
    m_viewportTabs->setDocumentMode(true);
    m_viewportTabs->setMovable(false);
    m_viewportTabs->setTabPosition(QTabWidget::North);
    m_viewport = new ViewportWidget(&m_bridge, m_viewportTabs, true);
    m_gameViewport = new ViewportWidget(&m_bridge, m_viewportTabs, false);
    m_viewportTabs->addTab(m_viewport, QStringLiteral("Scene"));
    m_viewportTabs->addTab(m_gameViewport, QStringLiteral("Game"));
    m_viewportTabs->setTabToolTip(0, QStringLiteral("Authoring viewport · Q/W/E/R tools · F focuses selection"));
    m_viewportTabs->setTabToolTip(1, QStringLiteral("Game preview viewport"));
    setCentralWidget(m_viewportTabs);
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

    m_hierarchyDock = addDock(QStringLiteral("Hierarchy"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("HierarchyPanel.qml")));
    m_inspectorDock = addDock(QStringLiteral("Inspector"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("InspectorPanel.qml")));
    m_readinessDock = addDock(QStringLiteral("Readiness"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ReadinessPanel.qml")));
    m_healthDock = addDock(QStringLiteral("Runtime Health"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("RuntimeHealthPanel.qml")));
    m_aiDock = addDock(QStringLiteral("Forge AI"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ForgeAiPanel.qml")));
    m_buildDock = addDock(QStringLiteral("Build & Export"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("BuildExportPanel.qml")));
    m_objectsDock = addDock(QStringLiteral("Object Studio"), Qt::LeftDockWidgetArea, makeQmlPanel(QStringLiteral("ObjectStudioPanel.qml")));
    m_authoringDock = addDock(QStringLiteral("Mega Authoring Hub"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("AuthoringHubPanel.qml")));
    m_sdkPacksDock = addDock(QStringLiteral("SDK & Content Packs"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("SdkPacksPanel.qml")));
    m_prefabDock = addDock(QStringLiteral("Prefab Studio"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("PrefabStudioPanel.qml")));
    m_projectSettingsDock = addDock(QStringLiteral("Project Settings"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ProjectSettingsPanel.qml")));
    m_projectLauncherDock = addDock(QStringLiteral("Project Launcher"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ProjectLauncherPanel.qml")));
    m_projectOperationsDock = addDock(QStringLiteral("Project Operations"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ProjectOperationsPanel.qml")));
    m_assetManagementDock = addDock(QStringLiteral("Asset Management"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("AssetManagementPanel.qml")));
    m_profilerDock = addDock(QStringLiteral("Profiler"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("ProfilerPanel.qml")));
    m_sceneBrowserDock = addDock(QStringLiteral("Scene Browser"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("SceneBrowserPanel.qml")));
    m_pythonToolsDock = addDock(QStringLiteral("Python Automation"), Qt::RightDockWidgetArea, makeQmlPanel(QStringLiteral("PythonToolsPanel.qml")));
    m_contentDock = addDock(QStringLiteral("Content Browser"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("ContentBrowserPanel.qml")));
    m_consoleDock = addDock(QStringLiteral("Console"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("ConsolePanel.qml")));
    m_spriteDock = addDock(QStringLiteral("Sprite Studio"), Qt::BottomDockWidgetArea, new SpriteEditorWidget(&m_bridge, this));
    m_luauDock = addDock(QStringLiteral("Luau Studio"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("LuauStudioPanel.qml")));
    m_animationTimelineDock = addDock(QStringLiteral("Animation Timeline"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("AnimationTimelinePanel.qml")));
    m_tilemapDock = addDock(QStringLiteral("Tilemap & Terrain"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("TilemapEditorPanel.qml")));
    m_uiDesignerDock = addDock(QStringLiteral("UI Designer"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("UiDesignerPanel.qml")));
    m_blueprintsDock = addDock(QStringLiteral("Blueprints"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("VisualGraphPanel.qml")));
    m_assetGraphDock = addDock(QStringLiteral("Asset Dependency Graph"), Qt::BottomDockWidgetArea, makeQmlPanel(QStringLiteral("AssetDependencyGraphPanel.qml")));
    m_commandDock = addDock(QStringLiteral("Command Palette"), Qt::TopDockWidgetArea, makeQmlPanel(QStringLiteral("CommandPalette.qml")));
    m_commandDock->setAllowedAreas(Qt::NoDockWidgetArea);
    m_commandDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetClosable);
    m_commandDock->setMinimumSize(620, 360);
    m_commandDock->resize(680, 460);
    auto* closePaletteShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), m_commandDock);
    closePaletteShortcut->setContext(Qt::WidgetWithChildrenShortcut);
    connect(closePaletteShortcut, &QShortcut::activated, this, &MainWindow::closeCommandPalette);

    splitDockWidget(m_hierarchyDock, m_inspectorDock, Qt::Vertical);
    tabifyDockWidget(m_inspectorDock, m_readinessDock);
    tabifyDockWidget(m_inspectorDock, m_healthDock);
    tabifyDockWidget(m_inspectorDock, m_aiDock);
    tabifyDockWidget(m_inspectorDock, m_buildDock);
    tabifyDockWidget(m_inspectorDock, m_authoringDock);
    tabifyDockWidget(m_inspectorDock, m_sdkPacksDock);
    tabifyDockWidget(m_inspectorDock, m_prefabDock);
    tabifyDockWidget(m_inspectorDock, m_projectSettingsDock);
    tabifyDockWidget(m_inspectorDock, m_projectLauncherDock);
    tabifyDockWidget(m_inspectorDock, m_projectOperationsDock);
    tabifyDockWidget(m_inspectorDock, m_assetManagementDock);
    tabifyDockWidget(m_inspectorDock, m_profilerDock);
    tabifyDockWidget(m_inspectorDock, m_sceneBrowserDock);
    tabifyDockWidget(m_inspectorDock, m_pythonToolsDock);
    tabifyDockWidget(m_contentDock, m_consoleDock);
    tabifyDockWidget(m_contentDock, m_spriteDock);
    tabifyDockWidget(m_contentDock, m_luauDock);
    tabifyDockWidget(m_contentDock, m_animationTimelineDock);
    tabifyDockWidget(m_contentDock, m_tilemapDock);
    tabifyDockWidget(m_contentDock, m_uiDesignerDock);
    tabifyDockWidget(m_contentDock, m_blueprintsDock);
    tabifyDockWidget(m_contentDock, m_assetGraphDock);
    m_inspectorDock->raise();
    m_contentDock->raise();
    m_commandDock->hide();
    resizeDocks({ m_objectsDock, m_hierarchyDock }, { 270, 350 }, Qt::Horizontal);
    resizeDocks({ m_hierarchyDock, m_inspectorDock }, { 300, 460 }, Qt::Vertical);
    resizeDocks({ m_contentDock, m_consoleDock }, { 250, 250 }, Qt::Vertical);
    populatePanelMenu();

    const auto setPanelShortcut = [](QDockWidget* dock, const QKeySequence& shortcut) {
        if (!dock) {
            return;
        }
        QAction* action = dock->toggleViewAction();
        action->setShortcut(shortcut);
        action->setShortcutContext(Qt::ApplicationShortcut);
        action->setToolTip(shortcutHint(action->text(), shortcut));
    };
    setPanelShortcut(m_contentDock, QKeySequence(QStringLiteral("Ctrl+Space")));
    setPanelShortcut(m_consoleDock, QKeySequence(QStringLiteral("Ctrl+`")));
    setPanelShortcut(m_hierarchyDock, QKeySequence(QStringLiteral("Ctrl+Shift+H")));
    setPanelShortcut(m_inspectorDock, QKeySequence(QStringLiteral("Ctrl+Shift+I")));
    setPanelShortcut(m_luauDock, QKeySequence(QStringLiteral("Ctrl+Shift+L")));
}

void MainWindow::populatePanelMenu()
{
    if (!m_panelMenu) {
        return;
    }
    m_panelMenu->clear();
    for (QDockWidget* dock : {
             m_hierarchyDock,
             m_inspectorDock,
             m_readinessDock,
             m_healthDock,
             m_aiDock,
             m_buildDock,
             m_objectsDock,
             m_authoringDock,
             m_sdkPacksDock,
             m_prefabDock,
             m_projectSettingsDock,
             m_projectLauncherDock,
             m_projectOperationsDock,
             m_assetManagementDock,
             m_profilerDock,
             m_sceneBrowserDock,
             m_pythonToolsDock,
             m_contentDock,
             m_consoleDock,
             m_spriteDock,
             m_luauDock,
             m_animationTimelineDock,
             m_tilemapDock,
             m_uiDesignerDock,
             m_blueprintsDock,
             m_assetGraphDock,
             m_commandDock,
         }) {
        if (dock) {
            m_panelMenu->addAction(dock->toggleViewAction());
        }
    }
}

void MainWindow::applyEditorStyle()
{
    setStyleSheet(QStringLiteral(R"(
        QMainWindow { background: #101318; color: #e7eaf0; }
        QMenuBar, QMenu, QToolBar, QStatusBar { background: #181c22; color: #e7eaf0; border: 0; }
        QMenuBar { border-bottom: 1px solid #2b313a; padding: 1px 5px; }
        QMenuBar::item { padding: 5px 9px; border-radius: 3px; }
        QMenuBar::item:selected, QMenuBar::item:pressed { background: #2a3039; }
        QMenu { border: 1px solid #343b46; padding: 4px; }
        QMenu::item { padding: 6px 30px 6px 24px; border-radius: 3px; }
        QMenu::item:selected { background: #304238; color: #f4fff8; }
        QMenu::item:disabled { color: #676f7c; }
        QMenu::separator { height: 1px; background: #303640; margin: 5px 8px; }
        QTabWidget::pane { border: 0; background: #0d1117; }
        QTabBar { background: #171b21; }
        QTabBar::tab { background: #171b21; color: #929aa7; padding: 7px 20px; border-right: 1px solid #2a3039; border-bottom: 1px solid #2a3039; }
        QTabBar::tab:hover { background: #20252d; color: #d9dde5; }
        QTabBar::tab:selected { background: #242a33; color: #f1f4f8; border-bottom: 2px solid #61cf93; }
        QToolBar { background: #1b2027; border-bottom: 1px solid #303640; spacing: 3px; padding: 4px 7px; }
        QToolBar::separator { background: #343b46; width: 1px; margin: 5px 7px; }
        QToolButton { color: #dfe3ea; padding: 5px 8px; border: 1px solid transparent; border-radius: 4px; }
        QToolButton:hover { background: #293039; border-color: #3b4552; }
        QToolButton:pressed, QToolButton:checked { background: #2d493a; border-color: #529871; color: #f2fff7; }
        QToolButton::menu-indicator { image: none; width: 0; }
        QToolButton#AddObjectButton { background: #243a31; border-color: #3f7055; }
        QToolButton#AddObjectButton:hover { background: #2c493b; border-color: #61cf93; }
        QLabel#ToolbarSectionLabel { color: #7f8997; font-size: 10px; padding-left: 3px; }
        QComboBox { background: #11151b; color: #e6e9ee; border: 1px solid #3a424d; border-radius: 4px; padding: 5px 24px 5px 8px; }
        QComboBox:hover, QComboBox:focus { border-color: #61cf93; }
        QComboBox::drop-down { border: 0; width: 22px; }
        QComboBox QAbstractItemView { background: #1c2128; color: #e6e9ee; border: 1px solid #3a424d; selection-background-color: #304238; selection-color: #f4fff8; }
        QDockWidget { color: #dfe3ea; titlebar-close-icon: none; titlebar-normal-icon: none; }
        QDockWidget::title { background: #1b2027; padding: 7px 9px; border-bottom: 1px solid #343b46; text-align: left; }
        QDockWidget::close-button, QDockWidget::float-button { border: 0; padding: 2px; }
        QDockWidget::close-button:hover, QDockWidget::float-button:hover { background: #323945; }
        QSplitter::handle { background: #2b313a; }
        QSplitter::handle:hover { background: #4a5564; }
        QStatusBar { background: #15191f; border-top: 1px solid #2b313a; color: #9ca5b2; }
        QStatusBar QLabel { color: #9ca5b2; padding: 0 5px; }
        QScrollBar:vertical { background: #14181e; width: 10px; margin: 0; }
        QScrollBar::handle:vertical { background: #3a424d; min-height: 28px; border-radius: 4px; margin: 2px; }
        QScrollBar::handle:vertical:hover { background: #526071; }
        QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0; }
        QToolTip { background: #242a33; color: #f1f4f8; border: 1px solid #46505d; padding: 5px; }
        QLabel { color: #aab1bd; }
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
    updateActionStates();
    QJsonObject sceneState;
    if (m_bridge.isOpen()) {
        const QJsonDocument document = QJsonDocument::fromJson(m_bridge.sceneStateJson().toUtf8());
        if (document.isObject()) {
            sceneState = document.object();
        }
    }
    const QString sceneName = sceneState.value(QStringLiteral("scene_name")).toString(QStringLiteral("Scene"));
    const QString sceneMode = sceneState.value(QStringLiteral("mode")).toString(QStringLiteral("EDITOR"));
    const bool sceneDirty = sceneState.value(QStringLiteral("dirty")).toBool(false);
    if (m_projectStatus) {
        m_projectStatus->setText(QStringLiteral("%1%2 · %3 | %4")
            .arg(sceneName, sceneDirty ? QStringLiteral(" *") : QString(), sceneMode, m_bridge.workbenchSummary()));
    }
    if (m_selectionStatus) {
        const QVector<quint64> selected = m_bridge.selectedEntities();
        if (selected.isEmpty()) {
            m_selectionStatus->setText(QStringLiteral("Selection: none"));
        } else if (selected.size() == 1) {
            m_selectionStatus->setText(QStringLiteral("Selection: #%1").arg(selected.first()));
        } else {
            m_selectionStatus->setText(QStringLiteral("Selection: %1 entities").arg(selected.size()));
        }
    }
    if (m_readinessStatus) {
        m_readinessStatus->setText(QStringLiteral("Readiness: %1%").arg(m_readiness.score()));
    }
    setWindowTitle(m_bridge.isOpen()
        ? QStringLiteral("MiniForge 0.9.3.4 Qt Editor - %1 · %2%3")
              .arg(m_bridge.projectName(), sceneName, sceneDirty ? QStringLiteral(" *") : QString())
        : QStringLiteral("MiniForge 0.9.3.4 Qt Editor"));
}

void MainWindow::updateActionStates()
{
    const bool projectOpen = m_bridge.isOpen();
    const bool hasSelection = projectOpen && m_bridge.selectedEntityCount() > 0;
    for (QAction* action : std::as_const(m_projectActions)) {
        if (action) {
            action->setEnabled(projectOpen);
        }
    }
    for (QAction* action : std::as_const(m_selectionActions)) {
        if (action) {
            action->setEnabled(hasSelection);
        }
    }
    QHash<QString, bool> commandAvailability;
    if (projectOpen) {
        for (const MfCommandItem& command : m_bridge.commands()) {
            commandAvailability.insert(command.id, command.enabled);
        }
    }
    for (QAction* action : findChildren<QAction*>()) {
        const QString objectName = action->objectName();
        if (objectName.startsWith(QStringLiteral("Command."))) {
            const QString commandId = objectName.mid(QStringLiteral("Command.").size());
            action->setEnabled(projectOpen && commandAvailability.value(commandId, false));
        }
    }
}

void MainWindow::setWorkspace(const QString& workspace)
{
    const QStringList& workspaces = editorWorkspaceNames();
    if (!workspaces.contains(workspace)) {
        return;
    }

    QSettings settings = editorSettings();
    if (!m_currentWorkspace.isEmpty() && m_currentWorkspace != workspace) {
        settings.setValue(workspaceStateKey(m_currentWorkspace), saveState(kWorkbenchStateVersion));
    }
    m_currentWorkspace = workspace;
    if (auto* action = findChild<QAction*>(QStringLiteral("Workspace.%1").arg(workspace))) {
        action->setChecked(true);
    }
    if (m_workspaceSelector && m_workspaceSelector->currentText() != workspace) {
        const QSignalBlocker blocker(m_workspaceSelector);
        m_workspaceSelector->setCurrentText(workspace);
    }
    const QByteArray savedState = settings.value(workspaceStateKey(workspace)).toByteArray();
    if (savedState.isEmpty() || !restoreState(savedState, kWorkbenchStateVersion)) {
        applyWorkspacePreset(workspace);
    }
    settings.setValue(QStringLiteral("workbench/lastWorkspace"), workspace);

    const bool scene = workspace == QStringLiteral("2D");
    const bool scripting = workspace == QStringLiteral("Scripting");
    const bool animation = workspace == QStringLiteral("Animation");
    const bool world = workspace == QStringLiteral("World");
    const bool ui = workspace == QStringLiteral("UI");
    const bool prefab = workspace == QStringLiteral("Prefab");
    const bool project = workspace == QStringLiteral("Project");
    const bool assets = workspace == QStringLiteral("Assets");
    const bool profiler = workspace == QStringLiteral("Profiler");
    const bool automation = workspace == QStringLiteral("Automation");
    const bool ai = workspace == QStringLiteral("AI");
    const bool build = workspace == QStringLiteral("Build");
    const bool debug = workspace == QStringLiteral("Debug");
    const bool sprites = workspace == QStringLiteral("Sprites");
    const bool systems = workspace == QStringLiteral("Systems");
    const bool sdk = workspace == QStringLiteral("SDK");

    if ((scene || scripting || animation || world || ui) && m_inspectorDock) {
        m_inspectorDock->raise();
    }
    if (scene && m_contentDock) {
        m_contentDock->raise();
    } else if (scripting && m_luauDock) {
        m_luauDock->raise();
    } else if (animation && m_animationTimelineDock) {
        m_animationTimelineDock->raise();
    } else if (world && m_tilemapDock) {
        m_tilemapDock->raise();
    } else if (ui && m_uiDesignerDock) {
        m_uiDesignerDock->raise();
    } else if (prefab && m_prefabDock) {
        m_prefabDock->raise();
    } else if (project && m_projectSettingsDock) {
        m_projectSettingsDock->raise();
    } else if (assets && m_assetManagementDock) {
        m_assetManagementDock->raise();
        if (m_assetGraphDock) m_assetGraphDock->raise();
    } else if (profiler && m_profilerDock) {
        m_profilerDock->raise();
        if (m_consoleDock) m_consoleDock->raise();
    } else if (automation && m_pythonToolsDock) {
        m_pythonToolsDock->raise();
        if (m_consoleDock) m_consoleDock->raise();
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
        if (m_healthDock) {
            m_healthDock->raise();
        }
        m_consoleDock->raise();
    } else if (sprites && m_spriteDock) {
        m_spriteDock->raise();
    } else if (systems && m_authoringDock) {
        m_authoringDock->raise();
    } else if (sdk && m_sdkPacksDock) {
        m_sdkPacksDock->raise();
    }
    statusBar()->showMessage(QStringLiteral("Workspace: %1").arg(workspace), 1800);
}

void MainWindow::applyWorkspacePreset(const QString& workspace)
{
    const bool scene = workspace == QStringLiteral("2D");
    const bool scripting = workspace == QStringLiteral("Scripting");
    const bool animation = workspace == QStringLiteral("Animation");
    const bool world = workspace == QStringLiteral("World");
    const bool ui = workspace == QStringLiteral("UI");
    const bool prefab = workspace == QStringLiteral("Prefab");
    const bool project = workspace == QStringLiteral("Project");
    const bool assets = workspace == QStringLiteral("Assets");
    const bool profiler = workspace == QStringLiteral("Profiler");
    const bool automation = workspace == QStringLiteral("Automation");
    const bool ai = workspace == QStringLiteral("AI");
    const bool build = workspace == QStringLiteral("Build");
    const bool debug = workspace == QStringLiteral("Debug");
    const bool minimal = workspace == QStringLiteral("Minimal");
    const bool sprites = workspace == QStringLiteral("Sprites");
    const bool systems = workspace == QStringLiteral("Systems");
    const bool sdk = workspace == QStringLiteral("SDK");

    const auto dockAt = [this](Qt::DockWidgetArea area, QDockWidget* dock) {
        if (!dock) {
            return;
        }
        dock->setFloating(false);
        addDockWidget(area, dock);
    };
    dockAt(scene ? Qt::RightDockWidgetArea : Qt::LeftDockWidgetArea, m_hierarchyDock);
    dockAt(scene ? Qt::LeftDockWidgetArea : Qt::RightDockWidgetArea, m_objectsDock);
    for (QDockWidget* dock : { m_inspectorDock, m_readinessDock, m_healthDock, m_aiDock, m_buildDock, m_authoringDock, m_sdkPacksDock, m_prefabDock, m_projectSettingsDock, m_projectLauncherDock, m_projectOperationsDock, m_assetManagementDock, m_profilerDock, m_sceneBrowserDock, m_pythonToolsDock }) {
        dockAt(Qt::RightDockWidgetArea, dock);
    }
    for (QDockWidget* dock : { m_contentDock, m_consoleDock, m_spriteDock, m_luauDock, m_animationTimelineDock, m_tilemapDock, m_uiDesignerDock, m_blueprintsDock, m_assetGraphDock }) {
        dockAt(Qt::BottomDockWidgetArea, dock);
    }
    if (scene && m_hierarchyDock && m_inspectorDock) {
        splitDockWidget(m_hierarchyDock, m_inspectorDock, Qt::Vertical);
    }
    if (m_inspectorDock && m_readinessDock) {
        tabifyDockWidget(m_inspectorDock, m_readinessDock);
    }
    if (m_inspectorDock && m_healthDock) {
        tabifyDockWidget(m_inspectorDock, m_healthDock);
    }
    if (m_inspectorDock && m_aiDock) {
        tabifyDockWidget(m_inspectorDock, m_aiDock);
    }
    if (m_inspectorDock && m_buildDock) {
        tabifyDockWidget(m_inspectorDock, m_buildDock);
    }
    if (m_inspectorDock && m_authoringDock) {
        tabifyDockWidget(m_inspectorDock, m_authoringDock);
    }
    if (m_inspectorDock && m_sdkPacksDock) {
        tabifyDockWidget(m_inspectorDock, m_sdkPacksDock);
    }
    if (m_inspectorDock && m_prefabDock) {
        tabifyDockWidget(m_inspectorDock, m_prefabDock);
    }
    if (m_inspectorDock && m_projectSettingsDock) {
        tabifyDockWidget(m_inspectorDock, m_projectSettingsDock);
    }
    if (m_inspectorDock && m_projectLauncherDock) {
        tabifyDockWidget(m_inspectorDock, m_projectLauncherDock);
    }
    if (m_inspectorDock && m_projectOperationsDock) {
        tabifyDockWidget(m_inspectorDock, m_projectOperationsDock);
    }
    if (m_inspectorDock && m_assetManagementDock) {
        tabifyDockWidget(m_inspectorDock, m_assetManagementDock);
    }
    if (m_inspectorDock && m_profilerDock) {
        tabifyDockWidget(m_inspectorDock, m_profilerDock);
    }
    if (m_inspectorDock && m_sceneBrowserDock) {
        tabifyDockWidget(m_inspectorDock, m_sceneBrowserDock);
    }
    if (m_inspectorDock && m_pythonToolsDock) {
        tabifyDockWidget(m_inspectorDock, m_pythonToolsDock);
    }
    if (m_contentDock && m_consoleDock) {
        tabifyDockWidget(m_contentDock, m_consoleDock);
    }
    if (m_contentDock && m_spriteDock) {
        tabifyDockWidget(m_contentDock, m_spriteDock);
    }
    if (m_contentDock && m_luauDock) {
        tabifyDockWidget(m_contentDock, m_luauDock);
    }
    if (m_contentDock && m_animationTimelineDock) {
        tabifyDockWidget(m_contentDock, m_animationTimelineDock);
    }
    if (m_contentDock && m_tilemapDock) {
        tabifyDockWidget(m_contentDock, m_tilemapDock);
    }
    if (m_contentDock && m_uiDesignerDock) {
        tabifyDockWidget(m_contentDock, m_uiDesignerDock);
    }
    if (m_contentDock && m_blueprintsDock) {
        tabifyDockWidget(m_contentDock, m_blueprintsDock);
    }
    if (m_contentDock && m_assetGraphDock) {
        tabifyDockWidget(m_contentDock, m_assetGraphDock);
    }
    if (scene) {
        resizeDocks({ m_objectsDock, m_hierarchyDock }, { 270, 350 }, Qt::Horizontal);
        resizeDocks({ m_hierarchyDock, m_inspectorDock }, { 300, 460 }, Qt::Vertical);
    } else {
        resizeDocks({ m_hierarchyDock, m_inspectorDock }, { 300, 360 }, Qt::Horizontal);
    }
    resizeDocks({ m_contentDock, m_consoleDock }, { 250, 250 }, Qt::Vertical);

    if (m_hierarchyDock) {
        m_hierarchyDock->setVisible(!minimal && !debug && !project && !assets && !profiler && !automation && !sprites);
    }
    if (m_inspectorDock) {
        m_inspectorDock->setVisible((!minimal || scene) && !project && !prefab && !assets && !profiler && !automation && !sprites);
    }
    if (m_readinessDock) {
        m_readinessDock->setVisible(scene || build || debug || project);
    }
    if (m_healthDock) {
        m_healthDock->setVisible(scene || build || debug || profiler);
    }
    if (m_aiDock) {
        m_aiDock->setVisible(ai);
    }
    if (m_buildDock) {
        m_buildDock->setVisible(build);
    }
    if (m_authoringDock) {
        m_authoringDock->setVisible(systems);
    }
    if (m_sdkPacksDock) {
        m_sdkPacksDock->setVisible(sdk);
    }
    if (m_objectsDock) {
        m_objectsDock->setVisible(scene);
    }
    if (m_prefabDock) {
        m_prefabDock->setVisible(prefab);
    }
    if (m_projectSettingsDock) {
        m_projectSettingsDock->setVisible(project);
    }
    if (m_projectLauncherDock) {
        m_projectLauncherDock->setVisible(project);
    }
    if (m_projectOperationsDock) {
        m_projectOperationsDock->setVisible(project || build);
    }
    if (m_assetManagementDock) {
        m_assetManagementDock->setVisible(assets);
    }
    if (m_profilerDock) {
        m_profilerDock->setVisible(profiler || debug);
    }
    if (m_sceneBrowserDock) {
        m_sceneBrowserDock->setVisible(scene);
    }
    if (m_pythonToolsDock) {
        m_pythonToolsDock->setVisible(automation);
    }
    if (m_contentDock) {
        m_contentDock->setVisible(scene || animation || world || ui || prefab || scripting || assets);
    }
    if (m_consoleDock) {
        m_consoleDock->setVisible(scripting || ai || build || debug || profiler || automation);
    }
    if (m_spriteDock) {
        m_spriteDock->setVisible(animation || sprites);
        if (sprites) {
            resizeDocks({ m_spriteDock }, { std::max(480, height() - 180) }, Qt::Vertical);
        }
    }
    if (m_luauDock) {
        m_luauDock->setVisible(scripting);
    }
    if (m_animationTimelineDock) {
        m_animationTimelineDock->setVisible(animation);
    }
    if (m_tilemapDock) {
        m_tilemapDock->setVisible(world);
    }
    if (m_uiDesignerDock) {
        m_uiDesignerDock->setVisible(ui);
        if (ui) {
            resizeDocks({ m_uiDesignerDock }, { std::max(460, height() - 260) }, Qt::Vertical);
        }
    }
    if (m_blueprintsDock) {
        m_blueprintsDock->setVisible(scripting);
    }
    if (m_assetGraphDock) {
        m_assetGraphDock->setVisible(assets);
    }
    if (m_commandDock) {
        m_commandDock->setVisible(false);
    }
}

void MainWindow::saveWorkbenchState()
{
    if (m_currentWorkspace.isEmpty()) {
        return;
    }
    QSettings settings = editorSettings();
    settings.setValue(QStringLiteral("workbench/windowGeometry"), saveGeometry());
    settings.setValue(QStringLiteral("workbench/lastWorkspace"), m_currentWorkspace);
    settings.setValue(workspaceStateKey(m_currentWorkspace), saveState(kWorkbenchStateVersion));
    settings.setValue(
        QStringLiteral("workbench/autoRefresh"),
        m_autoRefreshAction && m_autoRefreshAction->isChecked()
    );
    settings.setValue(QStringLiteral("workbench/viewportGuides"), m_gridAction && m_gridAction->isChecked());
    settings.setValue(QStringLiteral("workbench/viewportHud"), m_hudAction && m_hudAction->isChecked());
    settings.sync();
}

void MainWindow::activateViewportTool(int key)
{
    if (!m_viewport || !m_viewportTabs) {
        return;
    }
    m_viewportTabs->setCurrentWidget(m_viewport);
    m_viewport->setFocus(Qt::ShortcutFocusReason);
    QKeyEvent press(QEvent::KeyPress, key, Qt::NoModifier);
    QApplication::sendEvent(m_viewport, &press);
    QKeyEvent release(QEvent::KeyRelease, key, Qt::NoModifier);
    QApplication::sendEvent(m_viewport, &release);
}

void MainWindow::focusDock(QDockWidget* dock)
{
    if (!dock) {
        return;
    }
    dock->show();
    dock->raise();
    if (QWidget* panel = dock->widget()) {
        panel->setFocus(Qt::ShortcutFocusReason);
    }
}

void MainWindow::openAuthoringHub(const QString& kind)
{
    setWorkspace(QStringLiteral("Systems"));
    focusDock(m_authoringDock);
    auto* view = m_authoringDock
        ? qobject_cast<QQuickWidget*>(m_authoringDock->widget())
        : nullptr;
    if (view && view->rootObject()) {
        QMetaObject::invokeMethod(
            view->rootObject(),
            "selectKind",
            Qt::DirectConnection,
            Q_ARG(QVariant, QVariant(kind))
        );
    }
}

void MainWindow::executeShellCommand(const QString& commandId)
{
    closeCommandPalette();
    static const QHash<QString, QString> workspaceCommands {
        { QStringLiteral("workspace.2d"), QStringLiteral("2D") },
        { QStringLiteral("workspace.scripting"), QStringLiteral("Scripting") },
        { QStringLiteral("workspace.animation"), QStringLiteral("Animation") },
        { QStringLiteral("workspace.world"), QStringLiteral("World") },
        { QStringLiteral("workspace.ui"), QStringLiteral("UI") },
        { QStringLiteral("workspace.prefab"), QStringLiteral("Prefab") },
        { QStringLiteral("workspace.project"), QStringLiteral("Project") },
        { QStringLiteral("workspace.assets"), QStringLiteral("Assets") },
        { QStringLiteral("workspace.profiler"), QStringLiteral("Profiler") },
        { QStringLiteral("workspace.automation"), QStringLiteral("Automation") },
        { QStringLiteral("workspace.ai"), QStringLiteral("AI") },
        { QStringLiteral("workspace.build"), QStringLiteral("Build") },
        { QStringLiteral("workspace.debug"), QStringLiteral("Debug") },
        { QStringLiteral("workspace.sprites"), QStringLiteral("Sprites") },
        { QStringLiteral("workspace.systems"), QStringLiteral("Systems") },
        { QStringLiteral("workspace.sdk"), QStringLiteral("SDK") },
    };
    const auto workspace = workspaceCommands.constFind(commandId);
    if (workspace != workspaceCommands.cend()) {
        setWorkspace(*workspace);
        return;
    }

    if (commandId == QStringLiteral("view.reset_workspace")) {
        resetCurrentWorkspace();
        return;
    }

    QDockWidget* panel = nullptr;
    if (commandId == QStringLiteral("panel.content")) {
        panel = m_contentDock;
    } else if (commandId == QStringLiteral("panel.console")) {
        panel = m_consoleDock;
    } else if (commandId == QStringLiteral("panel.hierarchy")) {
        panel = m_hierarchyDock;
    } else if (commandId == QStringLiteral("panel.inspector")) {
        panel = m_inspectorDock;
    } else if (commandId == QStringLiteral("panel.luau")) {
        setWorkspace(QStringLiteral("Scripting"));
        panel = m_luauDock;
    } else if (commandId == QStringLiteral("panel.sprite")) {
        setWorkspace(QStringLiteral("Sprites"));
        panel = m_spriteDock;
    } else if (commandId == QStringLiteral("panel.scenes")) {
        setWorkspace(QStringLiteral("2D"));
        panel = m_sceneBrowserDock;
    } else if (commandId == QStringLiteral("panel.assets")) {
        setWorkspace(QStringLiteral("Assets"));
        panel = m_assetManagementDock;
    } else if (commandId == QStringLiteral("panel.profiler")) {
        setWorkspace(QStringLiteral("Profiler"));
        panel = m_profilerDock;
    } else if (commandId == QStringLiteral("panel.project_settings")) {
        setWorkspace(QStringLiteral("Project"));
        panel = m_projectSettingsDock;
    } else if (commandId == QStringLiteral("panel.forge_ai")) {
        setWorkspace(QStringLiteral("AI"));
        panel = m_aiDock;
    } else if (commandId == QStringLiteral("panel.build")) {
        setWorkspace(QStringLiteral("Build"));
        panel = m_buildDock;
    } else if (commandId == QStringLiteral("panel.launcher")) {
        panel = m_projectLauncherDock;
    }
    focusDock(panel);
}

void MainWindow::closeCommandPalette()
{
    if (m_commandDock) {
        m_commandDock->hide();
    }
}

void MainWindow::resetCurrentWorkspace()
{
    if (m_currentWorkspace.isEmpty()) {
        return;
    }
    QSettings settings = editorSettings();
    settings.remove(workspaceStateKey(m_currentWorkspace));
    applyWorkspacePreset(m_currentWorkspace);
    statusBar()->showMessage(QStringLiteral("Reset workspace: %1").arg(m_currentWorkspace), 2500);
}

void MainWindow::showCommandPalette()
{
    if (!m_commandDock) {
        return;
    }
    if (!m_commandDock->isFloating()) {
        m_commandDock->setFloating(true);
    }
    m_commandDock->resize(680, 460);
    const QPoint palettePosition = mapToGlobal(rect().center())
        - QPoint(m_commandDock->width() / 2, m_commandDock->height() / 2);
    m_commandDock->move(palettePosition);
    m_commandDock->show();
    m_commandDock->raise();
    m_commandDock->activateWindow();
    if (auto* view = qobject_cast<QQuickWidget*>(m_commandDock->widget())) {
        view->setFocus(Qt::ShortcutFocusReason);
        if (QObject* root = view->rootObject()) {
            QMetaObject::invokeMethod(root, "focusSearch", Qt::QueuedConnection);
        }
    }
}
