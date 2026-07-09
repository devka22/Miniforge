#pragma once

#include <QWidget>

#include "MfBridge.h"

class ViewportWidget final : public QWidget {
    Q_OBJECT
public:
    explicit ViewportWidget(MfBridge* bridge, QWidget* parent = nullptr);
protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
private slots:
    void refreshImage();
private:
    MfBridge* m_bridge = nullptr;
    QImage m_image;
};
