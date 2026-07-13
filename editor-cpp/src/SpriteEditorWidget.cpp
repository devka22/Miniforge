#include "SpriteEditorWidget.h"

#include <algorithm>
#include <cmath>
#include <functional>

#include <QAction>
#include <QColorDialog>
#include <QInputDialog>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QSignalBlocker>
#include <QSlider>
#include <QSpinBox>
#include <QToolBar>
#include <QTimer>
#include <QVBoxLayout>
#include <QWheelEvent>

class SpriteCanvasView final : public QWidget {
public:
    explicit SpriteCanvasView(MfBridge* bridge, QWidget* parent = nullptr)
        : QWidget(parent)
        , m_bridge(bridge)
    {
        setMinimumSize(260, 180);
        setFocusPolicy(Qt::StrongFocus);
        setMouseTracking(true);
        refreshImage();
    }

    void refreshImage()
    {
        m_image = m_bridge ? m_bridge->spriteImage(&m_info) : QImage {};
        update();
        if (stateChanged) {
            stateChanged();
        }
    }

    const MfSpriteInfo& info() const { return m_info; }
    QColor primaryColor() const { return m_primary; }
    QColor secondaryColor() const { return m_secondary; }
    int zoom() const { return m_zoom; }

    void setSheetGrid(int frameWidth, int frameHeight)
    {
        m_frameWidth = std::max(1, frameWidth);
        m_frameHeight = std::max(1, frameHeight);
        update();
    }

    void setFrameOverlayVisible(bool visible)
    {
        m_frameOverlayVisible = visible;
        update();
    }

    void setCurrentFrame(int frame)
    {
        m_currentFrame = std::max(0, frame);
        update();
    }

    void setPrimaryColor(const QColor& color)
    {
        if (color.isValid()) {
            m_primary = color;
            update();
            if (stateChanged) {
                stateChanged();
            }
        }
    }

    void setSecondaryColor(const QColor& color)
    {
        if (color.isValid()) {
            m_secondary = color;
            update();
            if (stateChanged) {
                stateChanged();
            }
        }
    }

    void setGridVisible(bool visible)
    {
        m_gridVisible = visible;
        update();
    }

    void fitToView()
    {
        if (m_image.isNull()) {
            return;
        }
        const int fitX = std::max(1, (width() - 32) / std::max(1, m_image.width()));
        const int fitY = std::max(1, (height() - 32) / std::max(1, m_image.height()));
        m_zoom = std::clamp(std::min(fitX, fitY), 1, 48);
        m_pan = {};
        update();
        if (stateChanged) {
            stateChanged();
        }
    }

    std::function<void()> stateChanged;

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.fillRect(rect(), QColor(18, 21, 28));
        if (m_image.isNull()) {
            painter.setPen(QColor(160, 168, 180));
            painter.drawText(rect(), Qt::AlignCenter, tr("Open a project to edit sprites"));
            return;
        }

        const QRect canvas = canvasRect();
        painter.fillRect(canvas, QColor(32, 36, 44));
        if (m_zoom >= 3) {
            const int firstX = std::max(0, (rect().left() - canvas.left()) / m_zoom);
            const int firstY = std::max(0, (rect().top() - canvas.top()) / m_zoom);
            const int lastX = std::min(m_image.width(), (rect().right() - canvas.left()) / m_zoom + 2);
            const int lastY = std::min(m_image.height(), (rect().bottom() - canvas.top()) / m_zoom + 2);
            for (int y = firstY; y < lastY; ++y) {
                for (int x = firstX; x < lastX; ++x) {
                    painter.fillRect(
                        canvas.left() + x * m_zoom,
                        canvas.top() + y * m_zoom,
                        m_zoom,
                        m_zoom,
                        (x + y) % 2 == 0 ? QColor(45, 50, 60) : QColor(29, 33, 41)
                    );
                }
            }
        }
        painter.setRenderHint(QPainter::SmoothPixmapTransform, false);
        painter.drawImage(canvas, m_image);

        if (m_gridVisible && m_zoom >= 6) {
            painter.setPen(QColor(8, 10, 14, 100));
            for (int x = 0; x <= m_image.width(); ++x) {
                const int px = canvas.left() + x * m_zoom;
                painter.drawLine(px, canvas.top(), px, canvas.bottom());
            }
            for (int y = 0; y <= m_image.height(); ++y) {
                const int py = canvas.top() + y * m_zoom;
                painter.drawLine(canvas.left(), py, canvas.right(), py);
            }
        }
        if (m_frameOverlayVisible && m_frameWidth > 0 && m_frameHeight > 0) {
            const int columns = std::max(1, m_image.width() / m_frameWidth);
            const int rows = std::max(1, m_image.height() / m_frameHeight);
            const int frameCount = columns * rows;
            if (frameCount > 0) {
                const int selected = std::clamp(m_currentFrame, 0, frameCount - 1);
                const int selectedX = selected % columns;
                const int selectedY = selected / columns;
                const QRect active(
                    canvas.left() + selectedX * m_frameWidth * m_zoom,
                    canvas.top() + selectedY * m_frameHeight * m_zoom,
                    m_frameWidth * m_zoom,
                    m_frameHeight * m_zoom
                );
                painter.fillRect(canvas, QColor(0, 0, 0, 72));
                painter.drawImage(active, m_image, QRect(
                    selectedX * m_frameWidth,
                    selectedY * m_frameHeight,
                    m_frameWidth,
                    m_frameHeight
                ));
                painter.setPen(QPen(QColor(108, 197, 143, 210), 1, Qt::DashLine));
                for (int x = 0; x <= columns; ++x) {
                    const int px = canvas.left() + x * m_frameWidth * m_zoom;
                    painter.drawLine(px, canvas.top(), px, canvas.top() + rows * m_frameHeight * m_zoom);
                }
                for (int y = 0; y <= rows; ++y) {
                    const int py = canvas.top() + y * m_frameHeight * m_zoom;
                    painter.drawLine(canvas.left(), py, canvas.left() + columns * m_frameWidth * m_zoom, py);
                }
                painter.setPen(QPen(QColor(255, 211, 91), 2));
                painter.drawRect(active.adjusted(1, 1, -2, -2));
                painter.setPen(QColor(255, 236, 168));
                painter.drawText(active.adjusted(5, 3, -3, -3), Qt::AlignLeft | Qt::AlignTop,
                    tr("F%1").arg(selected + 1));
            }
        }
        painter.setPen(QColor(108, 197, 143, 180));
        painter.drawRect(canvas.adjusted(0, 0, -1, -1));
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        setFocus();
        if (event->button() == Qt::MiddleButton) {
            m_panning = true;
            m_lastMouse = event->position();
            return;
        }
        if (event->button() != Qt::LeftButton && event->button() != Qt::RightButton) {
            return;
        }
        const QPoint pixel = pixelAt(event->position());
        if (pixel.x() < 0 || !m_bridge || !m_bridge->beginSpriteEdit()) {
            return;
        }
        m_painting = true;
        m_paintButton = event->button();
        m_lastPixel = pixel;
        paintLine(pixel, pixel);
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (m_panning) {
            m_pan += event->position() - m_lastMouse;
            m_lastMouse = event->position();
            update();
            return;
        }
        if (!m_painting) {
            return;
        }
        const QPoint pixel = pixelAt(event->position());
        if (pixel.x() < 0 || pixel == m_lastPixel) {
            return;
        }
        paintLine(m_lastPixel, pixel);
        m_lastPixel = pixel;
    }

    void mouseReleaseEvent(QMouseEvent* event) override
    {
        if (event->button() == Qt::MiddleButton) {
            m_panning = false;
            return;
        }
        if (!m_painting || event->button() != m_paintButton) {
            return;
        }
        m_painting = false;
        if (m_bridge) {
            m_bridge->commitSpriteEdit();
        }
        refreshImage();
    }

    void wheelEvent(QWheelEvent* event) override
    {
        const int direction = event->angleDelta().y() >= 0 ? 1 : -1;
        const int step = m_zoom >= 16 ? 4 : (m_zoom >= 8 ? 2 : 1);
        m_zoom = std::clamp(m_zoom + direction * step, 1, 64);
        update();
        if (stateChanged) {
            stateChanged();
        }
        event->accept();
    }

private:
    QRect canvasRect() const
    {
        const QSize size(m_image.width() * m_zoom, m_image.height() * m_zoom);
        const QPoint origin(
            (width() - size.width()) / 2 + qRound(m_pan.x()),
            (height() - size.height()) / 2 + qRound(m_pan.y())
        );
        return QRect(origin, size);
    }

    QPoint pixelAt(const QPointF& position) const
    {
        if (m_image.isNull()) {
            return QPoint(-1, -1);
        }
        const QRect canvas = canvasRect();
        if (!canvas.contains(position.toPoint())) {
            return QPoint(-1, -1);
        }
        const int x = static_cast<int>((position.x() - canvas.left()) / m_zoom);
        const int y = static_cast<int>((position.y() - canvas.top()) / m_zoom);
        if (x < 0 || y < 0 || x >= m_image.width() || y >= m_image.height()) {
            return QPoint(-1, -1);
        }
        return QPoint(x, y);
    }

    void paintLine(QPoint from, const QPoint& to)
    {
        const QColor color = m_paintButton == Qt::RightButton ? m_secondary : m_primary;
        int x0 = from.x();
        int y0 = from.y();
        const int x1 = to.x();
        const int y1 = to.y();
        const int dx = std::abs(x1 - x0);
        const int sx = x0 < x1 ? 1 : -1;
        const int dy = -std::abs(y1 - y0);
        const int sy = y0 < y1 ? 1 : -1;
        int error = dx + dy;
        while (true) {
            if (m_bridge->setSpritePixel(x0, y0, color)) {
                m_image.setPixelColor(x0, y0, color);
            }
            if (x0 == x1 && y0 == y1) {
                break;
            }
            const int doubled = error * 2;
            if (doubled >= dy) {
                error += dy;
                x0 += sx;
            }
            if (doubled <= dx) {
                error += dx;
                y0 += sy;
            }
        }
        update();
    }

    MfBridge* m_bridge = nullptr;
    QImage m_image;
    MfSpriteInfo m_info {};
    QColor m_primary { 108, 197, 143, 255 };
    QColor m_secondary { 0, 0, 0, 0 };
    int m_zoom = 12;
    bool m_gridVisible = true;
    bool m_painting = false;
    bool m_panning = false;
    bool m_frameOverlayVisible = false;
    int m_frameWidth = 16;
    int m_frameHeight = 16;
    int m_currentFrame = 0;
    Qt::MouseButton m_paintButton = Qt::NoButton;
    QPoint m_lastPixel;
    QPointF m_lastMouse;
    QPointF m_pan;
};

SpriteEditorWidget::SpriteEditorWidget(MfBridge* bridge, QWidget* parent)
    : QWidget(parent)
    , m_bridge(bridge)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    auto* toolbar = new QToolBar(this);
    toolbar->setIconSize(QSize(16, 16));
    QAction* new16 = toolbar->addAction(tr("New 16"));
    QAction* new32 = toolbar->addAction(tr("New 32"));
    toolbar->addSeparator();
    m_undoAction = toolbar->addAction(tr("Undo"));
    m_redoAction = toolbar->addAction(tr("Redo"));
    QAction* clear = toolbar->addAction(tr("Clear"));
    QAction* fit = toolbar->addAction(tr("Fit"));
    QAction* grid = toolbar->addAction(tr("Grid"));
    grid->setCheckable(true);
    grid->setChecked(true);
    toolbar->addSeparator();
    QAction* flipH = toolbar->addAction(tr("Flip H"));
    QAction* flipV = toolbar->addAction(tr("Flip V"));
    QAction* rotate = toolbar->addAction(tr("Rotate 90°"));
    QAction* crop = toolbar->addAction(tr("Crop"));
    QAction* outline = toolbar->addAction(tr("Outline"));
    toolbar->addSeparator();
    m_primaryAction = toolbar->addAction(tr("Primary"));
    m_secondaryAction = toolbar->addAction(tr("Secondary"));
    QAction* save = toolbar->addAction(tr("Save"));
    layout->addWidget(toolbar);

    auto* animationToolbar = new QToolBar(this);
    animationToolbar->setIconSize(QSize(16, 16));
    animationToolbar->addWidget(new QLabel(tr("Sprite sheet"), animationToolbar));
    m_frameWidth = new QSpinBox(animationToolbar);
    m_frameWidth->setRange(1, 512);
    m_frameWidth->setValue(16);
    m_frameWidth->setPrefix(tr(" W "));
    m_frameWidth->setToolTip(tr("Frame width in pixels"));
    animationToolbar->addWidget(m_frameWidth);
    m_frameHeight = new QSpinBox(animationToolbar);
    m_frameHeight->setRange(1, 512);
    m_frameHeight->setValue(16);
    m_frameHeight->setPrefix(tr(" H "));
    m_frameHeight->setToolTip(tr("Frame height in pixels"));
    animationToolbar->addWidget(m_frameHeight);
    m_fps = new QSpinBox(animationToolbar);
    m_fps->setRange(1, 120);
    m_fps->setValue(12);
    m_fps->setSuffix(tr(" fps"));
    animationToolbar->addWidget(m_fps);
    QAction* overlay = animationToolbar->addAction(tr("Frames"));
    overlay->setCheckable(true);
    overlay->setChecked(true);
    m_playAction = animationToolbar->addAction(tr("Play"));
    m_playAction->setCheckable(true);
    m_frameSlider = new QSlider(Qt::Horizontal, animationToolbar);
    m_frameSlider->setRange(0, 0);
    m_frameSlider->setMinimumWidth(120);
    m_frameSlider->setToolTip(tr("Animation frame scrubber"));
    animationToolbar->addWidget(m_frameSlider);
    layout->addWidget(animationToolbar);

    m_canvas = new SpriteCanvasView(bridge, this);
    layout->addWidget(m_canvas, 1);
    m_status = new QLabel(this);
    m_status->setContentsMargins(9, 4, 9, 4);
    layout->addWidget(m_status);

    m_canvas->stateChanged = [this] {
        refreshAnimationTimeline();
        refreshState();
    };
    connect(new16, &QAction::triggered, this, [this] { newCanvas(16); });
    connect(new32, &QAction::triggered, this, [this] { newCanvas(32); });
    connect(m_undoAction, &QAction::triggered, this, [this] {
        if (m_bridge->undoSprite()) {
            m_canvas->refreshImage();
        }
    });
    connect(m_redoAction, &QAction::triggered, this, [this] {
        if (m_bridge->redoSprite()) {
            m_canvas->refreshImage();
        }
    });
    connect(clear, &QAction::triggered, this, [this] { clearCanvas(); });
    connect(fit, &QAction::triggered, m_canvas, [this] { m_canvas->fitToView(); });
    connect(grid, &QAction::toggled, m_canvas, &SpriteCanvasView::setGridVisible);
    connect(flipH, &QAction::triggered, this, [this] {
        transformCanvas(QStringLiteral("flip_horizontal"));
    });
    connect(flipV, &QAction::triggered, this, [this] {
        transformCanvas(QStringLiteral("flip_vertical"));
    });
    connect(rotate, &QAction::triggered, this, [this] {
        transformCanvas(QStringLiteral("rotate_right"));
    });
    connect(crop, &QAction::triggered, this, [this] {
        bool accepted = false;
        const int padding = QInputDialog::getInt(
            this, tr("Crop to content"), tr("Padding"), 0, 0, 64, 1, &accepted);
        if (!accepted) {
            return;
        }
        transformCanvas(QStringLiteral("crop_to_content"),
            QStringLiteral("{\"padding\":%1}").arg(padding));
    });
    connect(outline, &QAction::triggered, this, [this] {
        bool accepted = false;
        const int thickness = QInputDialog::getInt(
            this, tr("Sprite outline"), tr("Thickness"), 1, 1, 16, 1, &accepted);
        if (!accepted) {
            return;
        }
        const QColor color = m_canvas->primaryColor();
        transformCanvas(QStringLiteral("outline"),
            QStringLiteral("{\"thickness\":%1,\"color\":{\"r\":%2,\"g\":%3,\"b\":%4,\"a\":%5}}")
                .arg(thickness)
                .arg(color.red())
                .arg(color.green())
                .arg(color.blue())
                .arg(color.alpha()));
    });
    connect(m_primaryAction, &QAction::triggered, this, [this] { choosePrimaryColor(); });
    connect(m_secondaryAction, &QAction::triggered, this, [this] { chooseSecondaryColor(); });
    connect(save, &QAction::triggered, this, [this] { saveCanvas(); });
    connect(m_bridge, &MfBridge::spriteChanged, m_canvas, &SpriteCanvasView::refreshImage);
    connect(m_bridge, &MfBridge::projectChanged, m_canvas, &SpriteCanvasView::refreshImage);
    connect(overlay, &QAction::toggled, m_canvas, &SpriteCanvasView::setFrameOverlayVisible);
    connect(m_frameWidth, &QSpinBox::valueChanged, this, &SpriteEditorWidget::refreshAnimationTimeline);
    connect(m_frameHeight, &QSpinBox::valueChanged, this, &SpriteEditorWidget::refreshAnimationTimeline);
    connect(m_fps, &QSpinBox::valueChanged, this, &SpriteEditorWidget::refreshAnimationTimeline);
    connect(m_frameSlider, &QSlider::valueChanged, m_canvas, &SpriteCanvasView::setCurrentFrame);
    connect(m_frameSlider, &QSlider::valueChanged, this, [this] { refreshState(); });
    connect(m_playAction, &QAction::toggled, this, &SpriteEditorWidget::setAnimationPlaying);
    m_animationTimer = new QTimer(this);
    connect(m_animationTimer, &QTimer::timeout, this, [this] {
        const int next = m_frameSlider->value() >= m_frameSlider->maximum()
            ? m_frameSlider->minimum()
            : m_frameSlider->value() + 1;
        m_frameSlider->setValue(next);
    });
    m_canvas->setFrameOverlayVisible(true);
    refreshState();
    refreshAnimationTimeline();
}

void SpriteEditorWidget::refreshState()
{
    const MfSpriteInfo& info = m_canvas->info();
    m_undoAction->setEnabled(info.can_undo != 0);
    m_redoAction->setEnabled(info.can_redo != 0);
    m_primaryAction->setToolTip(m_canvas->primaryColor().name(QColor::HexArgb));
    m_secondaryAction->setToolTip(m_canvas->secondaryColor().name(QColor::HexArgb));
    m_status->setText(tr("%1 × %2 · %3 px zoom · frame %4/%5 · left: primary · right: secondary · middle: pan")
        .arg(info.width)
        .arg(info.height)
        .arg(m_canvas->zoom())
        .arg(m_frameSlider ? m_frameSlider->value() + 1 : 1)
        .arg(m_frameSlider ? m_frameSlider->maximum() + 1 : 1));
}

void SpriteEditorWidget::newCanvas(int size)
{
    if (m_bridge->newSpriteCanvas(size, size)) {
        m_canvas->refreshImage();
        m_canvas->fitToView();
    }
}

void SpriteEditorWidget::clearCanvas()
{
    if (!m_bridge->beginSpriteEdit()) {
        return;
    }
    m_bridge->clearSprite(Qt::transparent);
    m_bridge->commitSpriteEdit();
    m_canvas->refreshImage();
}

void SpriteEditorWidget::choosePrimaryColor()
{
    const QColor color = QColorDialog::getColor(
        m_canvas->primaryColor(),
        this,
        tr("Primary Sprite Color"),
        QColorDialog::ShowAlphaChannel
    );
    m_canvas->setPrimaryColor(color);
}

void SpriteEditorWidget::chooseSecondaryColor()
{
    const QColor color = QColorDialog::getColor(
        m_canvas->secondaryColor(),
        this,
        tr("Secondary Sprite Color"),
        QColorDialog::ShowAlphaChannel
    );
    m_canvas->setSecondaryColor(color);
}

void SpriteEditorWidget::saveCanvas()
{
    const QString path = m_bridge->saveSprite(QStringLiteral("Sprite"));
    if (!path.isEmpty()) {
        m_status->setText(tr("Saved %1").arg(path));
    }
}

void SpriteEditorWidget::transformCanvas(const QString& action, const QString& payloadJson)
{
    if (m_bridge->transformSprite(action, payloadJson)) {
        m_canvas->refreshImage();
    }
}

void SpriteEditorWidget::refreshAnimationTimeline()
{
    if (!m_frameWidth || !m_frameHeight || !m_fps || !m_frameSlider || !m_canvas) {
        return;
    }
    const MfSpriteInfo& info = m_canvas->info();
    if (info.width == 0 || info.height == 0) {
        const QSignalBlocker blocker(m_frameSlider);
        m_frameSlider->setRange(0, 0);
        return;
    }
    {
        const QSignalBlocker widthBlocker(m_frameWidth);
        const QSignalBlocker heightBlocker(m_frameHeight);
        m_frameWidth->setMaximum(static_cast<int>(info.width));
        m_frameHeight->setMaximum(static_cast<int>(info.height));
        m_frameWidth->setValue(std::min(m_frameWidth->value(), static_cast<int>(info.width)));
        m_frameHeight->setValue(std::min(m_frameHeight->value(), static_cast<int>(info.height)));
    }
    m_canvas->setSheetGrid(m_frameWidth->value(), m_frameHeight->value());
    const QJsonDocument document = QJsonDocument::fromJson(
        m_bridge->spriteAnimationClipJson(m_frameWidth->value(), m_frameHeight->value(), m_fps->value()).toUtf8());
    const int frameCount = document.object()
                               .value(QStringLiteral("timeline"))
                               .toObject()
                               .value(QStringLiteral("frame_count"))
                               .toInt(1);
    const int oldFrame = m_frameSlider->value();
    {
        const QSignalBlocker blocker(m_frameSlider);
        m_frameSlider->setRange(0, std::max(0, frameCount - 1));
        m_frameSlider->setValue(std::min(oldFrame, m_frameSlider->maximum()));
    }
    m_canvas->setCurrentFrame(m_frameSlider->value());
    if (m_animationTimer && m_animationTimer->isActive()) {
        m_animationTimer->setInterval(std::max(8, 1000 / m_fps->value()));
    }
}

void SpriteEditorWidget::setAnimationPlaying(bool playing)
{
    m_playAction->setText(playing ? tr("Pause") : tr("Play"));
    if (!m_animationTimer) {
        return;
    }
    if (playing && m_frameSlider->maximum() > 0) {
        m_animationTimer->start(std::max(8, 1000 / m_fps->value()));
    } else {
        m_animationTimer->stop();
        if (playing) {
            const QSignalBlocker blocker(m_playAction);
            m_playAction->setChecked(false);
            m_playAction->setText(tr("Play"));
        }
    }
}
