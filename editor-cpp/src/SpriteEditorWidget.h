#pragma once

#include <QColor>
#include <QWidget>

#include "MfBridge.h"

class QAction;
class QLabel;
class QSlider;
class QSpinBox;
class QTimer;
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
    void transformCanvas(const QString& action, const QString& payloadJson = QStringLiteral("{}"));
    void refreshAnimationTimeline();
    void setAnimationPlaying(bool playing);

    MfBridge* m_bridge = nullptr;
    SpriteCanvasView* m_canvas = nullptr;
    QLabel* m_status = nullptr;
    QAction* m_undoAction = nullptr;
    QAction* m_redoAction = nullptr;
    QAction* m_primaryAction = nullptr;
    QAction* m_secondaryAction = nullptr;
    QAction* m_playAction = nullptr;
    QSpinBox* m_frameWidth = nullptr;
    QSpinBox* m_frameHeight = nullptr;
    QSpinBox* m_fps = nullptr;
    QSlider* m_frameSlider = nullptr;
    QTimer* m_animationTimer = nullptr;
};
