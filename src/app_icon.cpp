// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

#include "dualsense-updater/src/app_icon.h"

#include <QByteArray>
#include <QFile>
#include <QGuiApplication>
#include <QIcon>
#include <QLibraryInfo>
#include <QSize>
#include <QString>

#include <cstddef>

namespace dualsense_updater {

namespace {

struct EmbeddedIconSpec {
    const char* resource_path;
    int size_px;
};

constexpr EmbeddedIconSpec kEmbeddedPngIcons[] = {
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/16x16/icon.png", 16},
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/22x22/icon.png", 22},
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/32x32/icon.png", 32},
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/48x48/icon.png", 48},
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/64x64/icon.png", 64},
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/128x128/icon.png", 128},
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/256x256/icon.png", 256},
    {":/qt/qml/dev/dualsense/updater/packaging/linux/icons/512x512/icon.png", 512},
};

constexpr const char* kEmbeddedSvgIcon =
    ":/qt/qml/dev/dualsense/updater/packaging/linux/icons/scalable/icon.svg";

QIcon build_embedded_icon() {
    QIcon fallback;
    for (const EmbeddedIconSpec& spec : kEmbeddedPngIcons) {
        const QString path = QString::fromLatin1(spec.resource_path);
        if (QFile::exists(path)) {
            fallback.addFile(path, QSize(spec.size_px, spec.size_px));
        }
    }
    const QString svg_path = QString::fromLatin1(kEmbeddedSvgIcon);
    if (QFile::exists(svg_path)) {
        fallback.addFile(svg_path);
    }
    return fallback;
}

} // namespace

void configure_application_window_icon() {
    const QIcon embedded = build_embedded_icon();
    const QIcon resolved =
        QIcon::fromTheme(QStringLiteral("dev.dualsense.updater"), embedded);
    QGuiApplication::setWindowIcon(resolved);
}

rust::String qt_qml_imports_path() {
    const QByteArray utf8 = QLibraryInfo::path(QLibraryInfo::QmlImportsPath).toUtf8();
    return rust::String(utf8.constData(), static_cast<std::size_t>(utf8.size()));
}

} // namespace dualsense_updater
