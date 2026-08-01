#include "ViewportWidget.h"

#include <algorithm>
#include <cmath>

#include <QAction>
#include <QContextMenuEvent>
#include <QDragEnterEvent>
#include <QDragLeaveEvent>
#include <QDragMoveEvent>
#include <QDropEvent>
#include <QFileInfo>
#include <QFontDatabase>
#include <QFontMetrics>
#include <QHash>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QKeyEvent>
#include <QKeySequence>
#include <QLineF>
#include <QMenu>
#include <QMouseEvent>
#include <QMimeData>
#include <QPainter>
#include <QPainterPath>
#include <QResizeEvent>
#include <QStringList>
#include <QWheelEvent>

namespace {
constexpr qreal kMinimumZoom = 0.2;
constexpr qreal kMaximumZoom = 8.0;
constexpr qreal kDragThreshold = 5.0;
constexpr qreal kPi = 3.14159265358979323846;

QRectF normalizedRect(const QPointF& a, const QPointF& b)
{
    return QRectF(a, b).normalized();
}

bool entityHasComponent(const QJsonObject& entity, const QString& componentType)
{
    const QJsonArray components = entity.value(QStringLiteral("component_types")).toArray();
    return std::any_of(components.cbegin(), components.cend(), [&componentType](const QJsonValue& value) {
        return value.toString() == componentType;
    });
}

QPointF radialPoint(const QPointF& center, qreal radius, qreal degrees)
{
    const qreal radians = degrees * kPi / 180.0;
    return center + QPointF(std::cos(radians) * radius, std::sin(radians) * radius);
}
}

ViewportWidget::ViewportWidget(MfBridge* bridge, QWidget* parent, bool sceneView)
    : QWidget(parent)
    , m_bridge(bridge)
    , m_sceneView(sceneView)
{
    setMinimumSize(320, 240);
    setAutoFillBackground(false);
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
    setAcceptDrops(m_sceneView);
    setContextMenuPolicy(Qt::DefaultContextMenu);
    if (!m_sceneView) {
        m_hudVisible = false;
    }
    connect(m_bridge, &MfBridge::dataChanged, this, &ViewportWidget::refreshImage);
    connect(m_bridge, &MfBridge::projectChanged, this, &ViewportWidget::refreshImage);
    connect(m_bridge, &MfBridge::selectionChanged, this, [this](qulonglong) { update(); });
    connect(m_bridge, &MfBridge::readinessChanged, this, [this] { update(); });
}

bool ViewportWidget::gridVisible() const
{
    return m_gridVisible;
}

bool ViewportWidget::hudVisible() const
{
    return m_hudVisible;
}

void ViewportWidget::setGridVisible(bool visible)
{
    if (m_gridVisible == visible) {
        return;
    }
    m_gridVisible = visible;
    emit gridVisibleChanged();
    update();
}

void ViewportWidget::setHudVisible(bool visible)
{
    if (m_hudVisible == visible) {
        return;
    }
    m_hudVisible = visible;
    emit hudVisibleChanged();
    update();
}

void ViewportWidget::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), QColor(18, 21, 28));
    painter.setRenderHint(QPainter::Antialiasing, false);
    if (!m_bridge || !m_bridge->isOpen()) {
        paintEmptyState(painter);
        return;
    }
    if (m_image.isNull() || m_image.size() != size() * devicePixelRatioF()) {
        refreshImage();
    }

    painter.save();
    painter.setTransform(viewTransform());
    painter.drawImage(rect(), m_image, m_image.rect());
    if (m_gridVisible && m_sceneView) {
        paintGrid(painter);
    }
    painter.restore();

    if (m_sceneView) {
        paintSceneOverlays(painter);
        paintGizmo(painter);
        paintBoxSelection(painter);
    }
    if (m_hudVisible) {
        paintHud(painter);
    }
    if (m_assetDropActive) {
        painter.setPen(QPen(QColor(95, 211, 154), 2, Qt::DashLine));
        painter.setBrush(QColor(95, 211, 154, 22));
        painter.drawRoundedRect(rect().adjusted(3, 3, -3, -3), 7, 7);
        painter.setPen(QColor(214, 247, 231));
        painter.drawText(rect().adjusted(0, 42, 0, 0), Qt::AlignHCenter | Qt::AlignTop,
            tr("Drop asset to create or bind it here"));
    }
}

void ViewportWidget::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    refreshImage();
}

void ViewportWidget::refreshImage()
{
    if (!m_bridge || !m_bridge->isOpen() || width() <= 0 || height() <= 0) {
        m_image = QImage {};
        update();
        return;
    }
    const QSize pixelSize = size() * devicePixelRatioF();
    m_image = m_bridge->viewportImage(pixelSize);
    m_image.setDevicePixelRatio(devicePixelRatioF());
    update();
}

QTransform ViewportWidget::viewTransform() const
{
    QTransform transform;
    const QPointF center = rect().center();
    transform.translate(center.x() + m_pan.x(), center.y() + m_pan.y());
    transform.scale(m_zoom, m_zoom);
    transform.translate(-center.x(), -center.y());
    return transform;
}

QPointF ViewportWidget::toSource(const QPointF& viewportPoint) const
{
    bool invertible = false;
    const QTransform inverse = viewTransform().inverted(&invertible);
    return invertible ? inverse.map(viewportPoint) : viewportPoint;
}

QJsonObject ViewportWidget::viewportState() const
{
    if (!m_bridge || !m_bridge->isOpen()) {
        return {};
    }
    const qreal dpr = devicePixelRatioF();
    const QJsonDocument document = QJsonDocument::fromJson(
        m_bridge->viewportStateJson(qRound(width() * dpr), qRound(height() * dpr)).toUtf8());
    return document.isObject() ? document.object() : QJsonObject {};
}

QJsonObject ViewportWidget::activeViewportEntity() const
{
    const qulonglong activeId = m_bridge ? m_bridge->selectedEntityId() : 0;
    const QJsonObject state = viewportState();
    const QJsonArray entities = state.value(QStringLiteral("entities")).toArray();
    for (const QJsonValue& value : entities) {
        const QJsonObject entity = value.toObject();
        if (entity.value(QStringLiteral("id")).toVariant().toULongLong() == activeId) {
            return entity;
        }
    }
    return {};
}

QPointF ViewportWidget::entitySourceCenter(const QJsonObject& entity) const
{
    const qreal dpr = devicePixelRatioF();
    return {
        entity.value(QStringLiteral("center_x")).toDouble() / dpr,
        entity.value(QStringLiteral("center_y")).toDouble() / dpr,
    };
}

QRectF ViewportWidget::entitySourceRect(const QJsonObject& entity) const
{
    const qreal dpr = devicePixelRatioF();
    const QPointF center = entitySourceCenter(entity);
    const QSizeF size(
        entity.value(QStringLiteral("width")).toDouble() / dpr,
        entity.value(QStringLiteral("height")).toDouble() / dpr);
    return QRectF(center - QPointF(size.width() * 0.5, size.height() * 0.5), size);
}

QPointF ViewportWidget::selectedSourceCenter() const
{
    const QJsonObject state = viewportState();
    const QJsonArray entities = state.value(QStringLiteral("entities")).toArray();
    QPointF total;
    int count = 0;
    for (const QJsonValue& value : entities) {
        const QJsonObject entity = value.toObject();
        if (entity.value(QStringLiteral("selected")).toBool()) {
            total += entitySourceCenter(entity);
            ++count;
        }
    }
    return count > 0 ? total / count : QPointF {};
}

void ViewportWidget::mousePressEvent(QMouseEvent* event)
{
    if (!m_sceneView || !m_bridge || !m_bridge->isOpen()) {
        QWidget::mousePressEvent(event);
        return;
    }
    setFocus(Qt::MouseFocusReason);
    m_pressPosition = event->position();
    m_lastPosition = event->position();
    m_pressModifiers = event->modifiers();
    m_dragDelta = {};
    m_rotationDelta = 0.0;
    m_scaleFactor = 1.0;
    m_boxSelecting = false;

    if (event->button() == Qt::LeftButton && m_collisionOverlay
        && event->modifiers().testFlag(Qt::AltModifier)
        && beginCollisionVertexEdit(event->position(), event->modifiers())) {
        event->accept();
        return;
    }

    if (event->button() == Qt::MiddleButton
        || (event->button() == Qt::LeftButton
            && (m_spaceHeld || event->modifiers().testFlag(Qt::AltModifier)))) {
        m_panning = true;
        setCursor(Qt::ClosedHandCursor);
        event->accept();
        return;
    }
    if (event->button() != Qt::LeftButton) {
        QWidget::mousePressEvent(event);
        return;
    }

    const int toolbarWidth = 4 * 44;
    const QRectF toolbar((width() - toolbarWidth) * 0.5, 8, toolbarWidth, 28);
    if (toolbar.contains(event->position())) {
        const int index = std::clamp(
            static_cast<int>((event->position().x() - toolbar.left()) / 44.0), 0, 3);
        m_gizmoTool = static_cast<GizmoTool>(index);
        update();
        event->accept();
        return;
    }

    if (m_bridge->selectedEntityCount() > 0 && m_gizmoTool != GizmoTool::Select) {
        const QPointF center = viewTransform().map(selectedSourceCenter());
        const QPointF delta = event->position() - center;
        const qreal distance = QLineF(center, event->position()).length();
        bool hit = false;
        if (m_gizmoTool == GizmoTool::Move) {
            hit = std::abs(delta.x()) <= 48.0 && std::abs(delta.y()) <= 48.0;
        } else if (m_gizmoTool == GizmoTool::Rotate) {
            hit = std::abs(distance - 34.0) <= 12.0 || distance < 22.0;
        } else if (m_gizmoTool == GizmoTool::Scale) {
            hit = std::abs(delta.x()) <= 48.0 && std::abs(delta.y()) <= 48.0;
        }
        if (hit) {
            m_gizmoDragging = true;
            event->accept();
            return;
        }
    }
    event->accept();
}

void ViewportWidget::mouseMoveEvent(QMouseEvent* event)
{
    const QPointF current = event->position();
    if (m_panning) {
        m_pan += current - m_lastPosition;
        m_lastPosition = current;
        update();
        event->accept();
        return;
    }
    if (m_gizmoDragging) {
        const QPointF center = viewTransform().map(selectedSourceCenter());
        m_dragDelta = current - m_pressPosition;
        if (m_gizmoTool == GizmoTool::Rotate) {
            const qreal startAngle = std::atan2(
                m_pressPosition.y() - center.y(), m_pressPosition.x() - center.x());
            const qreal currentAngle = std::atan2(current.y() - center.y(), current.x() - center.x());
            m_rotationDelta = (currentAngle - startAngle) * 180.0 / M_PI;
        } else if (m_gizmoTool == GizmoTool::Scale) {
            const qreal startDistance = std::max<qreal>(8.0, QLineF(center, m_pressPosition).length());
            m_scaleFactor = std::clamp(
                QLineF(center, current).length() / startDistance, 0.05, 20.0);
        }
        update();
        event->accept();
        return;
    }
    if (m_collisionVertexDragging) {
        m_collisionVertexLocal = collisionLocalPoint(activeViewportEntity(), current);
        update();
        event->accept();
        return;
    }
    if (event->buttons().testFlag(Qt::LeftButton)
        && QLineF(m_pressPosition, current).length() >= kDragThreshold) {
        m_boxSelecting = true;
        m_lastPosition = current;
        update();
        event->accept();
        return;
    }
    QWidget::mouseMoveEvent(event);
}

void ViewportWidget::mouseReleaseEvent(QMouseEvent* event)
{
    if (m_panning) {
        m_panning = false;
        unsetCursor();
        event->accept();
        return;
    }
    if (m_gizmoDragging) {
        commitGizmoDrag();
        m_gizmoDragging = false;
        m_dragDelta = {};
        m_rotationDelta = 0.0;
        m_scaleFactor = 1.0;
        update();
        event->accept();
        return;
    }
    if (m_collisionVertexDragging) {
        const QJsonObject payload {
            { QStringLiteral("index"), m_collisionVertexIndex },
            { QStringLiteral("x"), m_collisionVertexLocal.x() },
            { QStringLiteral("y"), m_collisionVertexLocal.y() },
        };
        m_bridge->performEntityAction(
            m_collisionEntityId,
            QStringLiteral("collision_vertex_move"),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        m_collisionVertexDragging = false;
        m_collisionVertexIndex = -1;
        m_collisionEntityId = 0;
        update();
        event->accept();
        return;
    }
    if (!m_sceneView || !m_bridge || event->button() != Qt::LeftButton) {
        QWidget::mouseReleaseEvent(event);
        return;
    }
    if (m_boxSelecting) {
        m_lastPosition = event->position();
        applyBoxSelection(m_pressModifiers);
        m_boxSelecting = false;
        update();
        event->accept();
        return;
    }

    const bool toggle = m_pressModifiers.testFlag(Qt::ControlModifier)
        || m_pressModifiers.testFlag(Qt::MetaModifier);
    const QString selectionMode = toggle
        ? QStringLiteral("toggle")
        : (m_pressModifiers.testFlag(Qt::ShiftModifier)
                ? QStringLiteral("add")
                : QStringLiteral("replace"));
    const qreal dpr = devicePixelRatioF();
    const QPointF source = toSource(event->position()) * dpr;
    m_bridge->pickEntity(
        qRound(width() * dpr),
        qRound(height() * dpr),
        source.x(),
        source.y(),
        selectionMode);
    event->accept();
}

void ViewportWidget::applyBoxSelection(Qt::KeyboardModifiers modifiers)
{
    const QRectF selection = normalizedRect(m_pressPosition, m_lastPosition);
    const bool toggle = modifiers.testFlag(Qt::ControlModifier)
        || modifiers.testFlag(Qt::MetaModifier);
    const bool additive = modifiers.testFlag(Qt::ShiftModifier);
    if (!toggle && !additive) {
        m_bridge->clearSelection();
    }
    const QJsonArray entities = viewportState().value(QStringLiteral("entities")).toArray();
    for (const QJsonValue& value : entities) {
        const QJsonObject entity = value.toObject();
        if (!entity.value(QStringLiteral("visible")).toBool()
            || !entity.value(QStringLiteral("enabled")).toBool()
            || entity.value(QStringLiteral("locked")).toBool()) {
            continue;
        }
        const QRectF sourceRect = entitySourceRect(entity);
        const QPolygonF mapped = viewTransform().map(QPolygonF(sourceRect));
        if (!selection.intersects(mapped.boundingRect())) {
            continue;
        }
        const qulonglong id = entity.value(QStringLiteral("id")).toVariant().toULongLong();
        m_bridge->updateSelection(id, toggle ? QStringLiteral("toggle") : QStringLiteral("add"));
    }
}

void ViewportWidget::commitGizmoDrag()
{
    const QJsonObject state = viewportState();
    const qreal pixelsPerUnit = state.value(QStringLiteral("pixels_per_unit")).toDouble()
        / devicePixelRatioF();
    QJsonObject payload { { QStringLiteral("mode"), QStringLiteral("delta") } };
    if (m_gizmoTool == GizmoTool::Move && pixelsPerUnit > 0.0) {
        qreal dx = m_dragDelta.x() / (pixelsPerUnit * m_zoom);
        qreal dy = m_dragDelta.y() / (pixelsPerUnit * m_zoom);
        if (m_smartSnap) {
            dx = std::round(dx * 4.0) / 4.0;
            dy = std::round(dy * 4.0) / 4.0;
        }
        payload.insert(QStringLiteral("dx"), dx);
        payload.insert(QStringLiteral("dy"), dy);
    } else if (m_gizmoTool == GizmoTool::Rotate) {
        payload.insert(QStringLiteral("rotation_delta"), m_rotationDelta);
    } else if (m_gizmoTool == GizmoTool::Scale) {
        payload.insert(QStringLiteral("scale_x_factor"), m_scaleFactor);
        payload.insert(QStringLiteral("scale_y_factor"), m_scaleFactor);
    } else {
        return;
    }
    m_bridge->transformSelectionJson(
        QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
}

void ViewportWidget::wheelEvent(QWheelEvent* event)
{
    if (!m_sceneView) {
        QWidget::wheelEvent(event);
        return;
    }
    const QPointF cursor = event->position();
    const QPointF sourceBefore = toSource(cursor);
    const qreal factor = std::pow(1.0015, event->angleDelta().y());
    m_zoom = std::clamp(m_zoom * factor, kMinimumZoom, kMaximumZoom);
    const QPointF center = rect().center();
    m_pan = cursor - center - (sourceBefore - center) * m_zoom;
    update();
    event->accept();
}

void ViewportWidget::keyPressEvent(QKeyEvent* event)
{
    if (!m_sceneView) {
        QWidget::keyPressEvent(event);
        return;
    }
    if (event->key() == Qt::Key_Space) {
        m_spaceHeld = true;
        setCursor(Qt::OpenHandCursor);
    } else if (event->key() == Qt::Key_Q) {
        m_gizmoTool = GizmoTool::Select;
    } else if (event->key() == Qt::Key_W) {
        m_gizmoTool = GizmoTool::Move;
    } else if (event->key() == Qt::Key_E) {
        m_gizmoTool = GizmoTool::Rotate;
    } else if (event->key() == Qt::Key_R) {
        m_gizmoTool = GizmoTool::Scale;
    } else if (event->key() == Qt::Key_F) {
        focusSelection();
    } else if (event->key() == Qt::Key_Home) {
        resetView();
    } else {
        QWidget::keyPressEvent(event);
        return;
    }
    update();
    event->accept();
}

void ViewportWidget::keyReleaseEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Space) {
        m_spaceHeld = false;
        if (!m_panning) {
            unsetCursor();
        }
        event->accept();
        return;
    }
    QWidget::keyReleaseEvent(event);
}

void ViewportWidget::focusSelection()
{
    const QJsonArray entities = viewportState().value(QStringLiteral("entities")).toArray();
    QRectF bounds;
    bool found = false;
    for (const QJsonValue& value : entities) {
        const QJsonObject entity = value.toObject();
        if (!entity.value(QStringLiteral("selected")).toBool()) {
            continue;
        }
        bounds = found ? bounds.united(entitySourceRect(entity)) : entitySourceRect(entity);
        found = true;
    }
    if (!found) {
        return;
    }
    const qreal fitX = width() * 0.55 / std::max<qreal>(24.0, bounds.width());
    const qreal fitY = height() * 0.55 / std::max<qreal>(24.0, bounds.height());
    m_zoom = std::clamp(std::min(fitX, fitY), 0.6, 4.0);
    const QPointF center = rect().center();
    m_pan = -(bounds.center() - center) * m_zoom;
    update();
}

void ViewportWidget::resetView()
{
    m_zoom = 1.0;
    m_pan = {};
    update();
}

QPointF ViewportWidget::collisionLocalPoint(
    const QJsonObject& entity,
    const QPointF& viewportPoint) const
{
    const QJsonObject state = viewportState();
    const qreal unit = state.value(QStringLiteral("pixels_per_unit")).toDouble()
        / devicePixelRatioF();
    if (unit <= 0.0 || entity.isEmpty()) {
        return {};
    }
    QPointF local = (toSource(viewportPoint) - entitySourceCenter(entity)) / unit;
    if (m_smartSnap) {
        local.setX(std::round(local.x() * 8.0) / 8.0);
        local.setY(std::round(local.y() * 8.0) / 8.0);
    }
    return local;
}

bool ViewportWidget::beginCollisionVertexEdit(
    const QPointF& viewportPoint,
    Qt::KeyboardModifiers modifiers)
{
    const QJsonObject entity = activeViewportEntity();
    if (entity.isEmpty()) {
        return false;
    }
    const QJsonObject state = viewportState();
    const qreal unit = state.value(QStringLiteral("pixels_per_unit")).toDouble()
        / devicePixelRatioF();
    const QJsonArray points = entity.value(QStringLiteral("collision_points")).toArray();
    int hitIndex = -1;
    qreal bestDistance = 10.0;
    for (int index = 0; index < points.size(); ++index) {
        const QJsonArray point = points.at(index).toArray();
        if (point.size() < 2) {
            continue;
        }
        const QPointF sourcePoint = entitySourceCenter(entity)
            + QPointF(point.at(0).toDouble() * unit, point.at(1).toDouble() * unit);
        const qreal distance = QLineF(viewTransform().map(sourcePoint), viewportPoint).length();
        if (distance < bestDistance) {
            hitIndex = index;
            bestDistance = distance;
        }
    }
    const qulonglong entityId = entity.value(QStringLiteral("id")).toVariant().toULongLong();
    if (hitIndex >= 0 && modifiers.testFlag(Qt::ShiftModifier)) {
        const QJsonObject payload { { QStringLiteral("index"), hitIndex } };
        m_bridge->performEntityAction(
            entityId,
            QStringLiteral("collision_vertex_remove"),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        return true;
    }
    if (hitIndex < 0) {
        const QPointF local = collisionLocalPoint(entity, viewportPoint);
        const QJsonObject payload {
            { QStringLiteral("x"), local.x() },
            { QStringLiteral("y"), local.y() },
        };
        m_bridge->performEntityAction(
            entityId,
            QStringLiteral("collision_vertex_add"),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        return true;
    }
    m_collisionVertexDragging = true;
    m_collisionVertexIndex = hitIndex;
    m_collisionEntityId = entityId;
    m_collisionVertexLocal = collisionLocalPoint(entity, viewportPoint);
    return true;
}

QString ViewportWidget::droppedAssetPath(const QMimeData* mimeData) const
{
    if (!mimeData) {
        return {};
    }
    if (mimeData->hasFormat(QStringLiteral("application/x-miniforge-asset"))) {
        return QString::fromUtf8(
            mimeData->data(QStringLiteral("application/x-miniforge-asset"))).trimmed();
    }
    return mimeData->hasText() ? mimeData->text().trimmed() : QString {};
}

void ViewportWidget::dragEnterEvent(QDragEnterEvent* event)
{
    if (m_sceneView && m_bridge && m_bridge->isOpen()
        && !droppedAssetPath(event->mimeData()).isEmpty()) {
        m_assetDropActive = true;
        event->acceptProposedAction();
        update();
        return;
    }
    QWidget::dragEnterEvent(event);
}

void ViewportWidget::dragMoveEvent(QDragMoveEvent* event)
{
    if (m_assetDropActive) {
        event->acceptProposedAction();
        return;
    }
    QWidget::dragMoveEvent(event);
}

void ViewportWidget::dragLeaveEvent(QDragLeaveEvent* event)
{
    m_assetDropActive = false;
    update();
    QWidget::dragLeaveEvent(event);
}

void ViewportWidget::dropEvent(QDropEvent* event)
{
    m_assetDropActive = false;
    const QString relativePath = droppedAssetPath(event->mimeData());
    if (relativePath.isEmpty() || !m_bridge || !m_bridge->isOpen()) {
        event->ignore();
        update();
        return;
    }
    const qulonglong previousSelection = m_bridge->selectedEntityId();
    const qreal dpr = devicePixelRatioF();
    const QPointF source = toSource(event->position()) * dpr;
    qulonglong target = m_bridge->pickEntity(
        qRound(width() * dpr), qRound(height() * dpr), source.x(), source.y(), QStringLiteral("replace"));
    if (target == 0 && previousSelection != 0) {
        m_bridge->selectEntity(previousSelection);
        target = previousSelection;
    }
    const QString suffix = QFileInfo(relativePath).suffix().toLower();
    const bool createsVisual = QStringList {
        QStringLiteral("png"), QStringLiteral("jpg"), QStringLiteral("jpeg"),
        QStringLiteral("webp"), QStringLiteral("bmp"), QStringLiteral("gif"),
    }.contains(suffix) || relativePath.endsWith(QStringLiteral(".sprite.json"), Qt::CaseInsensitive);
    if (target == 0 && createsVisual && m_bridge->executeCommand(QStringLiteral("object.create"))) {
        target = m_bridge->selectedEntityId();
        const QJsonObject state = viewportState();
        const qreal pixelsPerUnit = state.value(QStringLiteral("pixels_per_unit")).toDouble();
        if (target != 0 && pixelsPerUnit > 0.0) {
            const QJsonObject transform {
                { QStringLiteral("mode"), QStringLiteral("absolute") },
                { QStringLiteral("x"), (source.x() - state.value(QStringLiteral("offset_x")).toDouble()) / pixelsPerUnit },
                { QStringLiteral("y"), (source.y() - state.value(QStringLiteral("offset_y")).toDouble()) / pixelsPerUnit },
            };
            m_bridge->transformSelectionJson(
                QString::fromUtf8(QJsonDocument(transform).toJson(QJsonDocument::Compact)));
        }
    }
    const QJsonObject payload {
        { QStringLiteral("relative_path"), relativePath },
        { QStringLiteral("drop_x"), source.x() },
        { QStringLiteral("drop_y"), source.y() },
    };
    const bool applied = target != 0 && m_bridge->performEntityAction(
        target,
        QStringLiteral("apply_asset"),
        QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
    applied ? event->acceptProposedAction() : event->ignore();
    refreshImage();
    update();
}

QString ViewportWidget::gizmoToolName() const
{
    switch (m_gizmoTool) {
    case GizmoTool::Move: return tr("Move");
    case GizmoTool::Rotate: return tr("Rotate");
    case GizmoTool::Scale: return tr("Scale");
    case GizmoTool::Select: return tr("Select");
    }
    return tr("Select");
}

void ViewportWidget::contextMenuEvent(QContextMenuEvent* event)
{
    QMenu menu(this);
    QMenu* tools = menu.addMenu(tr("Gizmo Tool"));
    QAction* selectTool = tools->addAction(tr("Select (Q)"));
    QAction* moveTool = tools->addAction(tr("Move (W)"));
    QAction* rotateTool = tools->addAction(tr("Rotate (E)"));
    QAction* scaleTool = tools->addAction(tr("Scale (R)"));
    menu.addSeparator();
    QAction* focus = menu.addAction(tr("Focus Selection (F)"));
    focus->setEnabled(m_bridge && m_bridge->selectedEntityCount() > 0);
    QAction* reset = menu.addAction(tr("Reset View (Home)"));
    QAction* refresh = menu.addAction(tr("Refresh View"));
    refresh->setShortcut(QKeySequence::Refresh);
    menu.addSeparator();
    QAction* grid = menu.addAction(tr("Show Guides"));
    grid->setCheckable(true);
    grid->setChecked(m_gridVisible);
    QAction* hud = menu.addAction(tr("Show HUD"));
    hud->setCheckable(true);
    hud->setChecked(m_hudVisible);
    QAction* smartSnap = menu.addAction(tr("Smart Snap"));
    smartSnap->setCheckable(true);
    smartSnap->setChecked(m_smartSnap);
    QAction* collisions = menu.addAction(tr("Collision Overlay"));
    collisions->setCheckable(true);
    collisions->setChecked(m_collisionOverlay);
    QMenu* physics = menu.addMenu(tr("Physics"));
    QAction* connectDistance =
        physics->addAction(tr("Connect Selection · Distance Joint"));
    QAction* connectSpring =
        physics->addAction(tr("Connect Selection · Spring Joint"));
    const bool canConnect = m_bridge && m_bridge->selectedEntityCount() == 2;
    connectDistance->setEnabled(canConnect);
    connectSpring->setEnabled(canConnect);
    QAction* cameraFrame = menu.addAction(tr("Camera Frame"));
    cameraFrame->setCheckable(true);
    cameraFrame->setChecked(m_cameraFrame);
    menu.addSeparator();
    const qulonglong entityId = m_bridge ? m_bridge->selectedEntityId() : 0;
    QAction* duplicate = menu.addAction(tr("Duplicate Selected"));
    duplicate->setEnabled(entityId != 0);
    QAction* resetTransform = menu.addAction(tr("Reset Transform"));
    resetTransform->setEnabled(entityId != 0);
    QAction* unparent = menu.addAction(tr("Move to Scene Root"));
    unparent->setEnabled(entityId != 0);
    QAction* pack = menu.addAction(tr("Pack Selected Branch"));
    pack->setEnabled(entityId != 0);
    QAction* remove = menu.addAction(tr("Delete Selected"));
    remove->setEnabled(entityId != 0);
    QMenu* arrange = menu.addMenu(tr("Arrange Selection"));
    QAction* alignLeft = arrange->addAction(tr("Align Left"));
    QAction* alignCenterX = arrange->addAction(tr("Align Center X"));
    QAction* alignTop = arrange->addAction(tr("Align Top"));
    QAction* alignCenterY = arrange->addAction(tr("Align Center Y"));
    QAction* distributeX = arrange->addAction(tr("Distribute Horizontally"));
    QAction* distributeY = arrange->addAction(tr("Distribute Vertically"));
    QMenu* grouping = menu.addMenu(tr("Groups & Layers"));
    QAction* group = grouping->addAction(tr("Group"));
    QAction* ungroup = grouping->addAction(tr("Ungroup"));
    QAction* cycleLayer = grouping->addAction(tr("Move to Next Layer"));
    QAction* layerLock = grouping->addAction(tr("Toggle Layer Lock"));
    QAction* layerVisibility = grouping->addAction(tr("Toggle Layer Visibility"));
    QAction* selected = menu.exec(event->globalPos());
    if (selected == selectTool) m_gizmoTool = GizmoTool::Select;
    else if (selected == moveTool) m_gizmoTool = GizmoTool::Move;
    else if (selected == rotateTool) m_gizmoTool = GizmoTool::Rotate;
    else if (selected == scaleTool) m_gizmoTool = GizmoTool::Scale;
    else if (selected == focus) focusSelection();
    else if (selected == reset) resetView();
    else if (selected == refresh) refreshImage();
    else if (selected == grid) setGridVisible(grid->isChecked());
    else if (selected == hud) setHudVisible(hud->isChecked());
    else if (selected == smartSnap) m_smartSnap = smartSnap->isChecked();
    else if (selected == collisions) m_collisionOverlay = collisions->isChecked();
    else if (selected == connectDistance && m_bridge)
        m_bridge->executeCommand(QStringLiteral("physics.connect_selection_distance"));
    else if (selected == connectSpring && m_bridge)
        m_bridge->executeCommand(QStringLiteral("physics.connect_selection_spring"));
    else if (selected == cameraFrame) m_cameraFrame = cameraFrame->isChecked();
    else if (selected == duplicate && m_bridge) m_bridge->performEntityAction(entityId, QStringLiteral("duplicate"));
    else if (selected == resetTransform && m_bridge) m_bridge->performEntityAction(entityId, QStringLiteral("reset_transform"));
    else if (selected == unparent && m_bridge) m_bridge->performEntityAction(entityId, QStringLiteral("unparent"));
    else if (selected == pack && m_bridge) m_bridge->executeCommand(QStringLiteral("scene.pack_selected"));
    else if (selected == remove && m_bridge) m_bridge->performEntityAction(entityId, QStringLiteral("delete"));
    else if (selected == alignLeft) m_bridge->executeCommand(QStringLiteral("selection.align_left"));
    else if (selected == alignCenterX) m_bridge->executeCommand(QStringLiteral("selection.align_center_x"));
    else if (selected == alignTop) m_bridge->executeCommand(QStringLiteral("selection.align_top"));
    else if (selected == alignCenterY) m_bridge->executeCommand(QStringLiteral("selection.align_center_y"));
    else if (selected == distributeX) m_bridge->executeCommand(QStringLiteral("selection.distribute_x"));
    else if (selected == distributeY) m_bridge->executeCommand(QStringLiteral("selection.distribute_y"));
    else if (selected == group) m_bridge->executeCommand(QStringLiteral("selection.group"));
    else if (selected == ungroup) m_bridge->executeCommand(QStringLiteral("selection.ungroup"));
    else if (selected == cycleLayer) m_bridge->executeCommand(QStringLiteral("selection.cycle_layer"));
    else if (selected == layerLock) m_bridge->executeCommand(QStringLiteral("selection.toggle_layer_lock"));
    else if (selected == layerVisibility) m_bridge->executeCommand(QStringLiteral("selection.toggle_layer_visibility"));
    update();
}

void ViewportWidget::paintEmptyState(QPainter& painter)
{
    QFont titleFont = QFontDatabase::systemFont(QFontDatabase::GeneralFont);
    titleFont.setPointSize(13);
    painter.setFont(titleFont);
    const QString title = tr("MiniForge Qt Editor");
    const QString detail = tr("Open a project to render the Rust viewport snapshot");
    const QFontMetrics metrics(painter.font());
    const QPoint center = rect().center();
    painter.setPen(QColor(232, 234, 238));
    painter.drawText(center.x() - metrics.horizontalAdvance(title) / 2, center.y() - 10, title);
    painter.setPen(QColor(150, 157, 168));
    painter.drawText(center.x() - metrics.horizontalAdvance(detail) / 2, center.y() + 16, detail);
}

void ViewportWidget::paintGrid(QPainter& painter)
{
    painter.save();
    painter.setPen(QColor(255, 255, 255, 20));
    constexpr int spacing = 64;
    for (int x = rect().left(); x < rect().right(); x += spacing) {
        painter.drawLine(x, rect().top(), x, rect().bottom());
    }
    for (int y = rect().top(); y < rect().bottom(); y += spacing) {
        painter.drawLine(rect().left(), y, rect().right(), y);
    }
    painter.setPen(QColor(110, 196, 143, 86));
    painter.drawLine(rect().center().x(), rect().top(), rect().center().x(), rect().bottom());
    painter.drawLine(rect().left(), rect().center().y(), rect().right(), rect().center().y());
    painter.restore();
}

void ViewportWidget::paintGizmo(QPainter& painter)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    const QStringList tools { tr("Q"), tr("W"), tr("E"), tr("R") };
    const int toolbarWidth = tools.size() * 44;
    const QRectF toolbar((width() - toolbarWidth) * 0.5, 8, toolbarWidth, 28);
    painter.setPen(QColor(72, 80, 94));
    painter.setBrush(QColor(24, 27, 34, 232));
    painter.drawRoundedRect(toolbar, 5, 5);
    for (int index = 0; index < tools.size(); ++index) {
        const QRectF cell(toolbar.left() + index * 44, toolbar.top(), 44, toolbar.height());
        const bool current = static_cast<int>(m_gizmoTool) == index;
        if (current) {
            painter.fillRect(cell.adjusted(2, 2, -2, -2), QColor(62, 111, 89));
        }
        painter.setPen(current ? QColor(235, 246, 241) : QColor(160, 168, 180));
        painter.drawText(cell, Qt::AlignCenter, tools.at(index));
    }

    if (!m_bridge || m_bridge->selectedEntityCount() == 0) {
        painter.restore();
        return;
    }
    QPointF center = viewTransform().map(selectedSourceCenter());
    if (m_gizmoDragging && m_gizmoTool == GizmoTool::Move) {
        center += m_dragDelta;
    }
    const QJsonObject active = activeViewportEntity();
    if (!active.isEmpty()) {
        const QPolygonF mapped = viewTransform().map(QPolygonF(entitySourceRect(active)));
        painter.setPen(QPen(QColor(104, 190, 255), 1.5, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawPolygon(mapped);
    }

    if (m_gizmoTool == GizmoTool::Move) {
        painter.setPen(QPen(QColor(238, 82, 82), 3));
        painter.drawLine(center, center + QPointF(44, 0));
        painter.drawLine(center + QPointF(44, 0), center + QPointF(35, -6));
        painter.drawLine(center + QPointF(44, 0), center + QPointF(35, 6));
        painter.setPen(QPen(QColor(85, 210, 126), 3));
        painter.drawLine(center, center + QPointF(0, -44));
        painter.drawLine(center + QPointF(0, -44), center + QPointF(-6, -35));
        painter.drawLine(center + QPointF(0, -44), center + QPointF(6, -35));
        painter.setBrush(QColor(104, 190, 255));
        painter.setPen(Qt::NoPen);
        painter.drawRect(QRectF(center - QPointF(5, 5), QSizeF(10, 10)));
    } else if (m_gizmoTool == GizmoTool::Rotate) {
        painter.setPen(QPen(QColor(255, 194, 88), 3));
        painter.setBrush(Qt::NoBrush);
        painter.drawEllipse(center, 34, 34);
        painter.setBrush(QColor(255, 194, 88));
        painter.drawEllipse(center + QPointF(34, 0), 4, 4);
        if (m_gizmoDragging) {
            painter.drawText(QRectF(center + QPointF(42, -12), QSizeF(90, 24)),
                QStringLiteral("%1°").arg(m_rotationDelta, 0, 'f', 1));
        }
    } else if (m_gizmoTool == GizmoTool::Scale) {
        painter.setPen(QPen(QColor(177, 116, 255), 3));
        painter.drawLine(center, center + QPointF(38, 38));
        painter.setBrush(QColor(177, 116, 255));
        painter.drawRect(QRectF(center + QPointF(33, 33), QSizeF(11, 11)));
        painter.setBrush(QColor(104, 190, 255));
        painter.drawRect(QRectF(center - QPointF(5, 5), QSizeF(10, 10)));
        if (m_gizmoDragging) {
            painter.drawText(QRectF(center + QPointF(45, 28), QSizeF(90, 24)),
                QStringLiteral("%1×").arg(m_scaleFactor, 0, 'f', 2));
        }
    } else {
        painter.setPen(QPen(QColor(104, 190, 255), 2));
        painter.setBrush(Qt::NoBrush);
        painter.drawEllipse(center, 5, 5);
    }
    painter.restore();
}

void ViewportWidget::paintBoxSelection(QPainter& painter)
{
    if (!m_boxSelecting) {
        return;
    }
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    const QRectF selection = normalizedRect(m_pressPosition, m_lastPosition);
    painter.setPen(QPen(QColor(104, 190, 255), 1));
    painter.setBrush(QColor(72, 142, 205, 48));
    painter.drawRect(selection);
    painter.restore();
}

void ViewportWidget::paintSceneOverlays(QPainter& painter)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    if (m_cameraFrame) {
        QRectF frame = rect().adjusted(34, 34, -34, -34);
        const qreal targetAspect = 16.0 / 9.0;
        if (frame.width() / frame.height() > targetAspect) {
            const qreal nextWidth = frame.height() * targetAspect;
            frame.adjust((frame.width() - nextWidth) * 0.5, 0, -(frame.width() - nextWidth) * 0.5, 0);
        } else {
            const qreal nextHeight = frame.width() / targetAspect;
            frame.adjust(0, (frame.height() - nextHeight) * 0.5, 0, -(frame.height() - nextHeight) * 0.5);
        }
        painter.setPen(QPen(QColor(255, 255, 255, 58), 1));
        painter.setBrush(Qt::NoBrush);
        painter.drawRect(frame);
        painter.drawRect(frame.adjusted(frame.width() * 0.05, frame.height() * 0.05,
            -frame.width() * 0.05, -frame.height() * 0.05));
    }
    const QJsonObject state = viewportState();
    const QJsonArray entities = state.value(QStringLiteral("entities")).toArray();
    const qreal unit = state.value(QStringLiteral("pixels_per_unit")).toDouble()
        / devicePixelRatioF();
    for (const QJsonValue& value : entities) {
        const QJsonObject entity = value.toObject();
        if (!entity.value(QStringLiteral("visible")).toBool()
            || !entity.value(QStringLiteral("enabled")).toBool()) {
            continue;
        }
        const QPointF center = viewTransform().map(entitySourceCenter(entity));
        const bool selected = entity.value(QStringLiteral("selected")).toBool();

        if (entityHasComponent(entity, QStringLiteral("Light2D"))) {
            const qreal radius = std::max<qreal>(10.0,
                entity.value(QStringLiteral("light_radius")).toDouble(5.0) * unit * m_zoom);
            const qreal angle = entity.value(QStringLiteral("light_angle")).toDouble(360.0);
            const qreal direction = entity.value(QStringLiteral("light_direction")).toDouble();
            painter.setPen(QPen(QColor(255, 205, 86, selected ? 220 : 115), selected ? 2.0 : 1.0,
                Qt::DashLine));
            painter.setBrush(QColor(255, 194, 72, selected ? 22 : 9));
            if (angle >= 359.0) {
                painter.drawEllipse(center, radius, radius);
            } else {
                QPainterPath cone(center);
                cone.lineTo(radialPoint(center, radius, direction - angle * 0.5));
                cone.arcTo(QRectF(center - QPointF(radius, radius), QSizeF(radius * 2.0, radius * 2.0)),
                    -(direction - angle * 0.5), -angle);
                cone.closeSubpath();
                painter.drawPath(cone);
            }
            painter.setPen(QPen(QColor(255, 225, 135, 230), 2));
            painter.setBrush(QColor(255, 205, 86, 210));
            painter.drawEllipse(center, 5, 5);
        }

        if (entityHasComponent(entity, QStringLiteral("ShadowCaster2D"))) {
            painter.setPen(QPen(QColor(255, 151, 73, selected ? 230 : 125), selected ? 2.0 : 1.0,
                Qt::DashLine));
            painter.setBrush(QColor(255, 112, 55, selected ? 22 : 8));
            painter.drawPolygon(viewTransform().map(QPolygonF(entitySourceRect(entity))));
        }

        if (entityHasComponent(entity, QStringLiteral("NormalMap2D"))) {
            const QColor normalColor(105, 153, 255, selected ? 245 : 165);
            painter.setPen(QPen(normalColor, selected ? 2.0 : 1.25));
            painter.setBrush(QColor(92, 116, 255, selected ? 48 : 22));
            painter.drawEllipse(center, selected ? 11 : 8, selected ? 11 : 8);
            painter.drawLine(center, center + QPointF(7, -6));
            painter.drawLine(center + QPointF(7, -6), center + QPointF(3, -5));
            painter.drawLine(center + QPointF(7, -6), center + QPointF(6, -2));
            painter.setPen(QColor(221, 230, 255, selected ? 250 : 190));
            painter.drawText(QRectF(center + QPointF(-5, -7), QSizeF(10, 14)),
                Qt::AlignCenter, QStringLiteral("N"));
            if (selected) {
                painter.drawText(QRectF(center + QPointF(15, -10), QSizeF(150, 20)),
                    Qt::AlignLeft | Qt::AlignVCenter, tr("WGPU normal lighting"));
            }
        }

        if (entityHasComponent(entity, QStringLiteral("NavAgent"))) {
            painter.setPen(QPen(QColor(87, 219, 159, selected ? 230 : 125), 1.5, Qt::DotLine));
            painter.setBrush(Qt::NoBrush);
            painter.drawEllipse(center, selected ? 20 : 12, selected ? 20 : 12);
        }
        if (entityHasComponent(entity, QStringLiteral("ParticleEmitter"))) {
            const bool gpu = entityHasComponent(entity, QStringLiteral("GpuParticles2D"));
            painter.setPen(QPen(gpu ? QColor(151, 118, 255, selected ? 250 : 175)
                                    : QColor(103, 203, 255, selected ? 240 : 150),
                gpu ? 2.5 : 2.0));
            for (int ray = 0; ray < (gpu ? 12 : 8); ++ray) {
                const qreal angle = ray * (gpu ? 30.0 : 45.0);
                painter.drawLine(radialPoint(center, 4, angle), radialPoint(center, gpu ? 14 : 11, angle));
            }
            if (gpu && selected) {
                const int capacity = entity.value(QStringLiteral("gpu_particle_capacity")).toInt();
                const qreal rate = entity.value(QStringLiteral("gpu_particle_emission_rate")).toDouble();
                const QString state = entity.value(QStringLiteral("gpu_particle_playing")).toBool()
                    ? tr("Compute")
                    : tr("Paused");
                painter.setPen(QColor(203, 189, 255, 235));
                painter.drawText(QRectF(center + QPointF(17, -14), QSizeF(180, 24)),
                    QStringLiteral("%1 · %2 · %3/s")
                        .arg(state)
                        .arg(capacity)
                        .arg(rate, 0, 'f', 0));
            }
        }
        if (entityHasComponent(entity, QStringLiteral("AudioSource"))) {
            painter.setPen(QPen(QColor(184, 129, 255, selected ? 235 : 140), 1.5));
            painter.setBrush(Qt::NoBrush);
            painter.drawEllipse(center, 8, 8);
            painter.drawEllipse(center, 14, 14);
        }

        if (selected && !entity.value(QStringLiteral("name")).toString().isEmpty()) {
            painter.setPen(QColor(226, 231, 238, 220));
            painter.drawText(QRectF(center + QPointF(12, 10), QSizeF(220, 22)),
                Qt::AlignLeft | Qt::AlignVCenter,
                entity.value(QStringLiteral("name")).toString());
        }
    }
    if (m_collisionOverlay) {
        const qreal dpr = devicePixelRatioF();
        const qreal sourceOffsetX = state.value(QStringLiteral("offset_x")).toDouble() / dpr;
        const qreal sourceOffsetY = state.value(QStringLiteral("offset_y")).toDouble() / dpr;
        QHash<qulonglong, QPointF> centers;
        for (const QJsonValue& value : entities) {
            const QJsonObject entity = value.toObject();
            if (!entity.value(QStringLiteral("visible")).toBool()) {
                continue;
            }
            const qulonglong entityId =
                entity.value(QStringLiteral("id")).toVariant().toULongLong();
            const QPointF sourceCenter = entitySourceCenter(entity);
            const QPointF center = viewTransform().map(sourceCenter);
            centers.insert(entityId, center);
            const bool selected = entity.value(QStringLiteral("selected")).toBool();

            if (entityHasComponent(entity, QStringLiteral("ForceField2D"))) {
                const qreal radius = entity.value(QStringLiteral("force_field_radius"))
                                         .toDouble(8.0)
                    * unit * m_zoom;
                const qreal strength =
                    entity.value(QStringLiteral("force_field_strength")).toDouble(10.0);
                const QString fieldType =
                    entity.value(QStringLiteral("force_field_type")).toString();
                painter.setPen(QPen(QColor(78, 207, 255, selected ? 235 : 155),
                    selected ? 2.0 : 1.0, Qt::DashLine));
                painter.setBrush(QColor(65, 184, 255, selected ? 28 : 12));
                painter.drawEllipse(center, radius, radius);
                if (fieldType == QStringLiteral("directional")) {
                    QPointF direction(
                        entity.value(QStringLiteral("force_field_direction_x")).toDouble(1.0),
                        entity.value(QStringLiteral("force_field_direction_y")).toDouble());
                    const qreal length = std::hypot(direction.x(), direction.y());
                    if (length > 0.0001) {
                        direction /= length;
                        if (strength < 0.0) {
                            direction = -direction;
                        }
                        const QPointF tip = center + direction * std::min<qreal>(radius, 54.0);
                        painter.drawLine(center, tip);
                        painter.setBrush(QColor(78, 207, 255, 220));
                        painter.drawEllipse(tip, 4, 4);
                    }
                } else {
                    painter.drawText(QRectF(center + QPointF(8, -18), QSizeF(120, 20)),
                        fieldType == QStringLiteral("vortex") ? tr("Vortex") : tr("Radial"));
                }
            }

            if (entityHasComponent(entity, QStringLiteral("Joint2D"))
                && !entity.value(QStringLiteral("joint_broken")).toBool()) {
                const QJsonValue targetX = entity.value(QStringLiteral("joint_target_x"));
                const QJsonValue targetY = entity.value(QStringLiteral("joint_target_y"));
                if (targetX.isDouble() && targetY.isDouble()) {
                    const QPointF targetSource(
                        sourceOffsetX + targetX.toDouble() * unit,
                        sourceOffsetY + targetY.toDouble() * unit);
                    const QPointF target = viewTransform().map(targetSource);
                    painter.setPen(QPen(QColor(255, 194, 88, selected ? 245 : 180),
                        selected ? 2.5 : 1.5,
                        entity.value(QStringLiteral("joint_type")).toString()
                                    == QStringLiteral("spring")
                            ? Qt::DotLine
                            : Qt::DashLine));
                    painter.drawLine(center, target);
                    painter.setBrush(QColor(255, 194, 88, 220));
                    painter.drawEllipse(center, 4, 4);
                    painter.drawEllipse(target, 4, 4);
                }
            }

            const qreal velocityX = entity.value(QStringLiteral("velocity_x")).toDouble();
            const qreal velocityY = entity.value(QStringLiteral("velocity_y")).toDouble();
            if (std::hypot(velocityX, velocityY) > 0.001) {
                const QPointF tip = center
                    + QPointF(velocityX, velocityY) * unit * m_zoom * 0.2;
                painter.setPen(QPen(QColor(93, 238, 163, 215), 2));
                painter.drawLine(center, tip);
                painter.setBrush(QColor(93, 238, 163, 230));
                painter.drawEllipse(tip, 3, 3);
            } else if (entity.value(QStringLiteral("physics_sleeping")).toBool()) {
                painter.setPen(QColor(142, 164, 190, 220));
                painter.drawText(QRectF(center + QPointF(7, -19), QSizeF(22, 18)),
                    QStringLiteral("Zz"));
            }

            if (entity.value(QStringLiteral("has_collision")).toBool()) {
                const bool trigger = entity.value(QStringLiteral("is_trigger")).toBool();
                painter.setPen(QPen(trigger ? QColor(186, 108, 255, 210) : QColor(255, 92, 88, 190),
                    1.5, Qt::DashLine));
                painter.setBrush(trigger ? QColor(178, 82, 255, 20) : QColor(255, 70, 70, 18));
                const QJsonArray points = entity.value(QStringLiteral("collision_points")).toArray();
                if (points.size() >= 2) {
                    QPolygonF polygon;
                    const QPointF center = entitySourceCenter(entity);
                    for (const QJsonValue& pointValue : points) {
                        const QJsonArray point = pointValue.toArray();
                        if (point.size() >= 2) {
                            polygon << center + QPointF(point.at(0).toDouble() * unit,
                                point.at(1).toDouble() * unit);
                        }
                    }
                    painter.drawPolygon(viewTransform().map(polygon));
                    for (const QPointF& point : viewTransform().map(polygon)) {
                        painter.drawEllipse(point, 3, 3);
                    }
                    if (m_collisionVertexDragging
                        && entity.value(QStringLiteral("id")).toVariant().toULongLong() == m_collisionEntityId) {
                        const QPointF preview = viewTransform().map(
                            center + QPointF(m_collisionVertexLocal.x() * unit,
                                m_collisionVertexLocal.y() * unit));
                        painter.setBrush(QColor(255, 194, 88));
                        painter.setPen(QPen(QColor(255, 240, 188), 2));
                        painter.drawEllipse(preview, 6, 6);
                        painter.setBrush(QColor(255, 70, 70, 18));
                        painter.setPen(QPen(QColor(255, 92, 88, 190), 1.5, Qt::DashLine));
                    }
                } else {
                    painter.drawPolygon(viewTransform().map(QPolygonF(entitySourceRect(entity))));
                }
            }
        }
        const QJsonArray contacts = state.value(QStringLiteral("physics_debug"))
                                        .toObject()
                                        .value(QStringLiteral("contacts"))
                                        .toArray();
        for (const QJsonValue& value : contacts) {
            const QJsonObject contact = value.toObject();
            const qulonglong firstId =
                contact.value(QStringLiteral("first_id")).toVariant().toULongLong();
            const qulonglong secondId =
                contact.value(QStringLiteral("second_id")).toVariant().toULongLong();
            if (!centers.contains(firstId) || !centers.contains(secondId)) {
                continue;
            }
            const QPointF midpoint = (centers.value(firstId) + centers.value(secondId)) * 0.5;
            const QJsonArray normal = contact.value(QStringLiteral("normal")).toArray();
            if (normal.size() < 2) {
                continue;
            }
            const QPointF tip = midpoint
                + QPointF(normal.at(0).toDouble(), normal.at(1).toDouble()) * 24.0;
            painter.setPen(QPen(contact.value(QStringLiteral("trigger")).toBool()
                    ? QColor(198, 121, 255, 220)
                    : QColor(255, 226, 112, 230),
                2));
            painter.drawLine(midpoint, tip);
            painter.setBrush(painter.pen().color());
            painter.drawEllipse(midpoint, 3, 3);
        }
    }
    if (m_gizmoDragging && m_gizmoTool == GizmoTool::Move && m_smartSnap) {
        const QPointF center = viewTransform().map(selectedSourceCenter()) + m_dragDelta;
        painter.setPen(QPen(QColor(95, 211, 154, 150), 1, Qt::DashLine));
        painter.drawLine(QPointF(center.x(), 0), QPointF(center.x(), height()));
        painter.drawLine(QPointF(0, center.y()), QPointF(width(), center.y()));
    }
    painter.restore();
}

void ViewportWidget::paintHud(QPainter& painter)
{
    if (!m_bridge) {
        return;
    }
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    const int selectedCount = m_bridge->selectedEntityCount();
    const QString selected = selectedCount == 0
        ? tr("Selection: none")
        : (selectedCount == 1
                ? tr("Selection: #%1").arg(m_bridge->selectedEntityId())
                : tr("Selection: %1 entities").arg(selectedCount));
    QString sceneLine = m_bridge->projectName();
    const QJsonDocument sceneDocument = QJsonDocument::fromJson(m_bridge->sceneStateJson().toUtf8());
    if (sceneDocument.isObject()) {
        const QJsonObject scene = sceneDocument.object();
        sceneLine = tr("%1%2 · %3")
                        .arg(
                            scene.value(QStringLiteral("scene_name")).toString(tr("Scene")),
                            scene.value(QStringLiteral("dirty")).toBool() ? QStringLiteral(" *") : QString(),
                            scene.value(QStringLiteral("mode")).toString(QStringLiteral("EDITOR")));
    }
    QStringList lines {
        sceneLine,
        m_bridge->workbenchSummary(),
        selected,
        tr("%1 tool · %2× zoom").arg(gizmoToolName()).arg(m_zoom, 0, 'f', 2),
        tr("Q/W/E/R tools · drag: box · middle/space: pan · wheel: zoom · F: focus"),
    };
    if (m_collisionOverlay) {
        lines.push_back(tr("Collision: Alt+click add/drag · Alt+Shift+click remove"));
        const QJsonObject stats = viewportState()
                                      .value(QStringLiteral("physics_debug"))
                                      .toObject()
                                      .value(QStringLiteral("stats"))
                                      .toObject();
        lines.push_back(tr("Physics: %1 bodies · %2 contacts · %3 joints · %4 sleeping")
                            .arg(stats.value(QStringLiteral("bodies")).toInt())
                            .arg(stats.value(QStringLiteral("contacts")).toInt())
                            .arg(stats.value(QStringLiteral("joints")).toInt())
                            .arg(stats.value(QStringLiteral("sleeping_bodies")).toInt()));
    }
    QFont font = QFontDatabase::systemFont(QFontDatabase::GeneralFont);
    font.setPointSize(10);
    painter.setFont(font);
    const QFontMetrics metrics(font);
    int panelWidth = 230;
    for (const QString& line : lines) {
        panelWidth = std::max(panelWidth, metrics.horizontalAdvance(line) + 24);
    }
    const QRect panel(12, 12, panelWidth, lines.size() * 18 + 16);
    painter.setPen(QColor(70, 78, 92, 190));
    painter.setBrush(QColor(22, 25, 32, 218));
    painter.drawRoundedRect(panel, 6, 6);
    int y = panel.top() + 20;
    for (int index = 0; index < lines.size(); ++index) {
        painter.setPen(index == 0 ? QColor(232, 234, 238) : QColor(170, 176, 187));
        painter.drawText(panel.left() + 12, y, lines.at(index));
        y += 18;
    }
    painter.restore();
}
