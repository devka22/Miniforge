#include "ViewportWidget.h"

#include <QPainter>
#include <QResizeEvent>

ViewportWidget::ViewportWidget(MfBridge* bridge, QWidget* parent)
    : QWidget(parent)
    , m_bridge(bridge)
{
    setMinimumSize(320, 240);
    setAutoFillBackground(false);
    connect(m_bridge, &MfBridge::dataChanged, this, &ViewportWidget::refreshImage);
}

void ViewportWidget::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), QColor(18, 21, 28));
    if (m_image.isNull() || m_image.size() != size()) {
        refreshImage();
    }
    painter.drawImage(rect(), m_image);
}

void ViewportWidget::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    refreshImage();
}

void ViewportWidget::refreshImage()
{
    if (!m_bridge || !m_bridge->isOpen() || width() <= 0 || height() <= 0) {
        return;
    }
    m_image = m_bridge->viewportImage(size());
    update();
}
