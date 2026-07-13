#pragma once

#include <QImage>
#include <QJsonObject>
#include <QPointF>
#include <QRectF>
#include <QTransform>
#include <QWidget>

#include "MfBridge.h"

class QDragEnterEvent;
class QDragLeaveEvent;
class QDragMoveEvent;
class QDropEvent;
class QMimeData;

class ViewportWidget final : public QWidget {
    Q_OBJECT
    Q_PROPERTY(bool gridVisible READ gridVisible WRITE setGridVisible NOTIFY gridVisibleChanged)
    Q_PROPERTY(bool hudVisible READ hudVisible WRITE setHudVisible NOTIFY hudVisibleChanged)
public:
    enum class GizmoTool { Select, Move, Rotate, Scale };

    explicit ViewportWidget(MfBridge* bridge, QWidget* parent = nullptr, bool sceneView = true);
    bool gridVisible() const;
    bool hudVisible() const;
public slots:
    void refreshImage();
    void setGridVisible(bool visible);
    void setHudVisible(bool visible);
signals:
    void gridVisibleChanged();
    void hudVisibleChanged();
protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void wheelEvent(QWheelEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    void keyReleaseEvent(QKeyEvent* event) override;
    void contextMenuEvent(QContextMenuEvent* event) override;
    void dragEnterEvent(QDragEnterEvent* event) override;
    void dragMoveEvent(QDragMoveEvent* event) override;
    void dragLeaveEvent(QDragLeaveEvent* event) override;
    void dropEvent(QDropEvent* event) override;
private:
    void paintEmptyState(QPainter& painter);
    void paintGrid(QPainter& painter);
    void paintHud(QPainter& painter);
    void paintGizmo(QPainter& painter);
    void paintBoxSelection(QPainter& painter);
    void paintSceneOverlays(QPainter& painter);
    QTransform viewTransform() const;
    QPointF toSource(const QPointF& viewportPoint) const;
    QJsonObject viewportState() const;
    QJsonObject activeViewportEntity() const;
    QPointF entitySourceCenter(const QJsonObject& entity) const;
    QRectF entitySourceRect(const QJsonObject& entity) const;
    QPointF selectedSourceCenter() const;
    void applyBoxSelection(Qt::KeyboardModifiers modifiers);
    void commitGizmoDrag();
    void focusSelection();
    void resetView();
    QString gizmoToolName() const;
    bool beginCollisionVertexEdit(const QPointF& viewportPoint, Qt::KeyboardModifiers modifiers);
    QPointF collisionLocalPoint(const QJsonObject& entity, const QPointF& viewportPoint) const;
    QString droppedAssetPath(const QMimeData* mimeData) const;

    MfBridge* m_bridge = nullptr;
    QImage m_image;
    bool m_sceneView = true;
    bool m_gridVisible = false;
    bool m_hudVisible = true;
    GizmoTool m_gizmoTool = GizmoTool::Select;
    qreal m_zoom = 1.0;
    QPointF m_pan;
    QPointF m_pressPosition;
    QPointF m_lastPosition;
    QPointF m_dragDelta;
    qreal m_rotationDelta = 0.0;
    qreal m_scaleFactor = 1.0;
    bool m_panning = false;
    bool m_spaceHeld = false;
    bool m_boxSelecting = false;
    bool m_gizmoDragging = false;
    bool m_smartSnap = true;
    bool m_collisionOverlay = false;
    bool m_assetDropActive = false;
    bool m_collisionVertexDragging = false;
    int m_collisionVertexIndex = -1;
    qulonglong m_collisionEntityId = 0;
    QPointF m_collisionVertexLocal;
    bool m_cameraFrame = true;
    Qt::KeyboardModifiers m_pressModifiers {};
};
