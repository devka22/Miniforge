#include "ViewportWidget.h"

#include <algorithm>
#include <QAction>
#include <QContextMenuEvent>
#include <QFontMetrics>
#include <QKeySequence>
#include <QMenu>
#include <QPainter>
#include <QResizeEvent>
#include <QStaticText>
#include <QStringList>

ViewportWidget::ViewportWidget(MfBridge* bridge, QWidget* parent)
    : QWidget(parent)
    , m_bridge(bridge)
{
    setMinimumSize(320, 240);
    setAutoFillBackground(false);
    setContextMenuPolicy(Qt::DefaultContextMenu);
    connect(m_bridge, &MfBridge::dataChanged, this, &ViewportWidget::refreshImage);
    connect(m_bridge, &MfBridge::projectChanged, this, &ViewportWidget::refreshImage);
    connect(m_bridge, &MfBridge::selectionChanged, this, [this](qulonglong) {
        update();
    });
    connect(m_bridge, &MfBridge::readinessChanged, this, [this] {
        update();
    });
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
    painter.drawImage(rect(), m_image, m_image.rect());
    if (m_gridVisible) {
        paintGrid(painter);
    }
    if (m_hudVisible) {
        paintHud(painter);
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

void ViewportWidget::contextMenuEvent(QContextMenuEvent* event)
{
    QMenu menu(this);
    QAction* refresh = menu.addAction(tr("Refresh View"));
    refresh->setShortcut(QKeySequence::Refresh);
    menu.addSeparator();
    QAction* grid = menu.addAction(tr("Show Guides"));
    grid->setCheckable(true);
    grid->setChecked(m_gridVisible);
    QAction* hud = menu.addAction(tr("Show HUD"));
    hud->setCheckable(true);
    hud->setChecked(m_hudVisible);
    menu.addSeparator();
    QAction* audit = menu.addAction(tr("Run Project Audit"));
    QAction* selected = menu.exec(event->globalPos());
    if (selected == refresh) {
        refreshImage();
    } else if (selected == grid) {
        setGridVisible(grid->isChecked());
    } else if (selected == hud) {
        setHudVisible(hud->isChecked());
    } else if (selected == audit && m_bridge) {
        m_bridge->executeCommand(QStringLiteral("project.audit"));
    }
}

void ViewportWidget::paintEmptyState(QPainter& painter)
{
    painter.setPen(QColor(120, 128, 140));
    painter.setFont(QFont(QStringLiteral("Inter"), 13));
    const QString title = tr("MiniForge Qt Editor");
    const QString detail = tr("Open a project to render the Rust viewport snapshot");
    const QFontMetrics metrics(painter.font());
    const int titleWidth = metrics.horizontalAdvance(title);
    const int detailWidth = metrics.horizontalAdvance(detail);
    const QPoint center = rect().center();
    painter.setPen(QColor(232, 234, 238));
    painter.drawText(center.x() - titleWidth / 2, center.y() - 10, title);
    painter.setPen(QColor(150, 157, 168));
    painter.drawText(center.x() - detailWidth / 2, center.y() + 16, detail);
}

void ViewportWidget::paintGrid(QPainter& painter)
{
    painter.save();
    const QColor line(255, 255, 255, 22);
    const QColor axis(110, 196, 143, 80);
    painter.setPen(line);
    const int spacing = 64;
    for (int x = rect().left(); x < rect().right(); x += spacing) {
        painter.drawLine(x, rect().top(), x, rect().bottom());
    }
    for (int y = rect().top(); y < rect().bottom(); y += spacing) {
        painter.drawLine(rect().left(), y, rect().right(), y);
    }
    painter.setPen(axis);
    painter.drawLine(rect().center().x(), rect().top(), rect().center().x(), rect().bottom());
    painter.drawLine(rect().left(), rect().center().y(), rect().right(), rect().center().y());
    painter.restore();
}

void ViewportWidget::paintHud(QPainter& painter)
{
    if (!m_bridge) {
        return;
    }
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    const QString selected = m_bridge->selectedEntityId() == 0
        ? tr("Selection: none")
        : tr("Selection: #%1").arg(m_bridge->selectedEntityId());
    const QStringList lines {
        m_bridge->projectName(),
        m_bridge->workbenchSummary(),
        selected,
        tr("Readiness %1%").arg(m_bridge->readinessScore()),
    };
    const QFont font(QStringLiteral("Inter"), 10);
    painter.setFont(font);
    const QFontMetrics metrics(font);
    int panelWidth = 220;
    for (const QString& line : lines) {
        panelWidth = std::max(panelWidth, metrics.horizontalAdvance(line) + 24);
    }
    const QRect panel(12, 12, panelWidth, 88);
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
