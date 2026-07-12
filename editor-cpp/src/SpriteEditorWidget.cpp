#include "SpriteEditorWidget.h"

#include <algorithm>
#include <cmath>
#include <functional>

#include <QAction>
#include <QColorDialog>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QToolBar>
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
    m_primaryAction = toolbar->addAction(tr("Primary"));
    m_secondaryAction = toolbar->addAction(tr("Secondary"));
    QAction* save = toolbar->addAction(tr("Save"));
    layout->addWidget(toolbar);

    m_canvas = new SpriteCanvasView(bridge, this);
    layout->addWidget(m_canvas, 1);
    m_status = new QLabel(this);
    m_status->setContentsMargins(9, 4, 9, 4);
    layout->addWidget(m_status);

    m_canvas->stateChanged = [this] { refreshState(); };
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
    connect(m_primaryAction, &QAction::triggered, this, [this] { choosePrimaryColor(); });
    connect(m_secondaryAction, &QAction::triggered, this, [this] { chooseSecondaryColor(); });
    connect(save, &QAction::triggered, this, [this] { saveCanvas(); });
    connect(m_bridge, &MfBridge::spriteChanged, m_canvas, &SpriteCanvasView::refreshImage);
    connect(m_bridge, &MfBridge::projectChanged, m_canvas, &SpriteCanvasView::refreshImage);
    refreshState();
}

void SpriteEditorWidget::refreshState()
{
    const MfSpriteInfo& info = m_canvas->info();
    m_undoAction->setEnabled(info.can_undo != 0);
    m_redoAction->setEnabled(info.can_redo != 0);
    m_primaryAction->setToolTip(m_canvas->primaryColor().name(QColor::HexArgb));
    m_secondaryAction->setToolTip(m_canvas->secondaryColor().name(QColor::HexArgb));
    m_status->setText(tr("%1 × %2 · %3 px zoom · left: primary · right: secondary · middle: pan")
        .arg(info.width)
        .arg(info.height)
        .arg(m_canvas->zoom()));
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
