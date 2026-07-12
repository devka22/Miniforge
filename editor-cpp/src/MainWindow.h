#pragma once

#include <QDockWidget>
#include <QMainWindow>
#include <QLabel>
#include <QString>
#include <QTimer>

#include "MfBridge.h"
#include "MfModels.h"

class QAction;
class ViewportWidget;

class MainWindow final : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(const QString& projectPath, QWidget* parent = nullptr);
    ~MainWindow() override;

private:
    QWidget* makeQmlPanel(const QString& qmlFile);
    QDockWidget* addDock(const QString& title, Qt::DockWidgetArea area, QWidget* widget);
    bool openProjectPath(const QString& path, bool showErrorDialog);
    void chooseProject();
    void createMenus();
    void createToolbar();
    void createPanels();
    void applyEditorStyle();
    void refreshModels();
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
    QTimer* m_refreshTimer = nullptr;
    QAction* m_autoRefreshAction = nullptr;
    QAction* m_gridAction = nullptr;
    QAction* m_hudAction = nullptr;
    QDockWidget* m_hierarchyDock = nullptr;
    QDockWidget* m_inspectorDock = nullptr;
    QDockWidget* m_readinessDock = nullptr;
    QDockWidget* m_aiDock = nullptr;
    QDockWidget* m_buildDock = nullptr;
    QDockWidget* m_objectsDock = nullptr;
    QDockWidget* m_contentDock = nullptr;
    QDockWidget* m_consoleDock = nullptr;
    QDockWidget* m_spriteDock = nullptr;
    QDockWidget* m_luauDock = nullptr;
    QDockWidget* m_commandDock = nullptr;
    QLabel* m_projectStatus = nullptr;
    QLabel* m_selectionStatus = nullptr;
    QLabel* m_readinessStatus = nullptr;
};
