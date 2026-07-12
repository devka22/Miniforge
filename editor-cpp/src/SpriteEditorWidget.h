#pragma once

#include <QColor>
#include <QWidget>

#include "MfBridge.h"

class QAction;
class QLabel;
class SpriteCanvasView;

class SpriteEditorWidget final : public QWidget {
public:
    explicit SpriteEditorWidget(MfBridge* bridge, QWidget* parent = nullptr);

private:
    void refreshState();
    void newCanvas(int size);
    void clearCanvas();
    void choosePrimaryColor();
    void chooseSecondaryColor();
    void saveCanvas();

    MfBridge* m_bridge = nullptr;
    SpriteCanvasView* m_canvas = nullptr;
    QLabel* m_status = nullptr;
    QAction* m_undoAction = nullptr;
    QAction* m_redoAction = nullptr;
    QAction* m_primaryAction = nullptr;
    QAction* m_secondaryAction = nullptr;
};
