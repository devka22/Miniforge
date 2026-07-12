#pragma once

#include <QImage>
#include <QWidget>

#include "MfBridge.h"

class ViewportWidget final : public QWidget {
    Q_OBJECT
    Q_PROPERTY(bool gridVisible READ gridVisible WRITE setGridVisible NOTIFY gridVisibleChanged)
    Q_PROPERTY(bool hudVisible READ hudVisible WRITE setHudVisible NOTIFY hudVisibleChanged)
public:
    explicit ViewportWidget(MfBridge* bridge, QWidget* parent = nullptr);
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
    void contextMenuEvent(QContextMenuEvent* event) override;
private:
    void paintEmptyState(QPainter& painter);
    void paintGrid(QPainter& painter);
    void paintHud(QPainter& painter);

    MfBridge* m_bridge = nullptr;
    QImage m_image;
    bool m_gridVisible = false;
    bool m_hudVisible = true;
};
