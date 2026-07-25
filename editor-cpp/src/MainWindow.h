#pragma once

#include <QDockWidget>
#include <QMainWindow>
#include <QLabel>
#include <QString>
#include <QTimer>
#include <QVector>

#include "MfBridge.h"
#include "MfModels.h"

class QAction;
class QCloseEvent;
class QComboBox;
class QMenu;
class QTabWidget;
class QToolBar;
class ViewportWidget;

class MainWindow final : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(const QString& projectPath, QWidget* parent = nullptr);
    ~MainWindow() override;
    Q_INVOKABLE void executeShellCommand(const QString& commandId);
    Q_INVOKABLE void closeCommandPalette();

protected:
    void closeEvent(QCloseEvent* event) override;

private:
    QWidget* makeQmlPanel(const QString& qmlFile);
    QDockWidget* addDock(const QString& title, Qt::DockWidgetArea area, QWidget* widget);
    bool openProjectPath(const QString& path, bool showErrorDialog);
    void chooseProject();
    void createMenus();
    void createToolbar();
    void createPanels();
    void populatePanelMenu();
    void applyEditorStyle();
    void applyWorkspacePreset(const QString& workspace);
    void activateViewportTool(int key);
    void focusDock(QDockWidget* dock);
    void openAuthoringHub(const QString& kind = QStringLiteral("all"));
    void saveWorkbenchState();
    void resetCurrentWorkspace();
    void showCommandPalette();
    void refreshModels();
    void updateActionStates();
    void updateStatus();
    void refreshWorkbench();
    void setWorkspace(const QString& workspace);

    MfBridge m_bridge;
    EntityModel m_entities;
    InspectorModel m_inspector;
    AssetModel m_assets;
    CommandModel m_commands;
    ConsoleModel m_console;
    ReadinessModel m_readiness;
    ForgeAiModel m_forgeAi;
    MfEditorController m_controller;
    QString m_qmlRootPath;
    ViewportWidget* m_viewport = nullptr;
    ViewportWidget* m_gameViewport = nullptr;
    QTabWidget* m_viewportTabs = nullptr;
    QTimer* m_refreshTimer = nullptr;
    QAction* m_autoRefreshAction = nullptr;
    QAction* m_gridAction = nullptr;
    QAction* m_hudAction = nullptr;
    QMenu* m_panelMenu = nullptr;
    QComboBox* m_workspaceSelector = nullptr;
    QToolBar* m_mainToolbar = nullptr;
    QDockWidget* m_hierarchyDock = nullptr;
    QDockWidget* m_inspectorDock = nullptr;
    QDockWidget* m_readinessDock = nullptr;
    QDockWidget* m_healthDock = nullptr;
    QDockWidget* m_aiDock = nullptr;
    QDockWidget* m_buildDock = nullptr;
    QDockWidget* m_objectsDock = nullptr;
    QDockWidget* m_authoringDock = nullptr;
    QDockWidget* m_sdkPacksDock = nullptr;
    QDockWidget* m_contentDock = nullptr;
    QDockWidget* m_consoleDock = nullptr;
    QDockWidget* m_spriteDock = nullptr;
    QDockWidget* m_luauDock = nullptr;
    QDockWidget* m_animationTimelineDock = nullptr;
    QDockWidget* m_tilemapDock = nullptr;
    QDockWidget* m_uiDesignerDock = nullptr;
    QDockWidget* m_prefabDock = nullptr;
    QDockWidget* m_blueprintsDock = nullptr;
    QDockWidget* m_projectSettingsDock = nullptr;
    QDockWidget* m_projectLauncherDock = nullptr;
    QDockWidget* m_projectOperationsDock = nullptr;
    QDockWidget* m_assetManagementDock = nullptr;
    QDockWidget* m_profilerDock = nullptr;
    QDockWidget* m_sceneBrowserDock = nullptr;
    QDockWidget* m_pythonToolsDock = nullptr;
    QDockWidget* m_assetGraphDock = nullptr;
    QDockWidget* m_commandDock = nullptr;
    QLabel* m_projectStatus = nullptr;
    QLabel* m_selectionStatus = nullptr;
    QLabel* m_readinessStatus = nullptr;
    QString m_currentWorkspace;
    QVector<QAction*> m_projectActions;
    QVector<QAction*> m_selectionActions;
};
