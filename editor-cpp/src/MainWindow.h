#pragma once

#include <QDockWidget>
#include <QMainWindow>
#include <QLabel>
#include <QString>

#include "MfBridge.h"
#include "MfModels.h"

class MainWindow final : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(const QString& projectPath, QWidget* parent = nullptr);

private:
    QWidget* makeQmlPanel(const QString& qmlFile);
    QDockWidget* addDock(const QString& title, Qt::DockWidgetArea area, QWidget* widget);
    void createMenus();
    void createToolbar();
    void createPanels();
    void applyEditorStyle();
    void refreshModels();
    void updateStatus();

    MfBridge m_bridge;
    EntityModel m_entities;
    InspectorModel m_inspector;
    AssetModel m_assets;
    CommandModel m_commands;
    ConsoleModel m_console;
    ReadinessModel m_readiness;
    MfEditorController m_controller;
    QLabel* m_projectStatus = nullptr;
    QLabel* m_selectionStatus = nullptr;
    QLabel* m_readinessStatus = nullptr;
};
