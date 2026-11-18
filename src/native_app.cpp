// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

#include "dualsense-updater/src/native_app.h"

#include <QApplication>
#include <QCoreApplication>
#include <QGuiApplication>
#include <QString>

namespace dualsense_updater {

namespace {

std::vector<std::string> normalize_args(const rust::Vec<rust::String>& args) {
    std::vector<std::string> out;
    if (args.empty()) {
        out.emplace_back("dualsense-updater");
        return out;
    }
    out.reserve(args.size());
    for (const rust::String& arg : args) {
        out.emplace_back(arg.data(), arg.size());
    }
    if (out.front().empty()) {
        out.front() = "dualsense-updater";
    }
    return out;
}

QString to_qstring(rust::Str value) {
    return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
}

} // namespace

NativeApplication::NativeApplication(rust::Vec<rust::String> args)
    : arg_storage_(normalize_args(args)),
      argc_(static_cast<int>(arg_storage_.size())) {
    argv_.reserve(arg_storage_.size() + 1);
    for (std::string& arg : arg_storage_) {
        argv_.push_back(arg.data());
    }
    argv_.push_back(nullptr);
    app_ = std::make_unique<QApplication>(argc_, argv_.data());
}

NativeApplication::~NativeApplication() = default;

int NativeApplication::exec() {
    return app_ ? app_->exec() : 1;
}

void NativeApplication::set_application_name(rust::Str name) {
    QApplication::setApplicationName(to_qstring(name));
}

void NativeApplication::set_application_display_name(rust::Str name) {
    QGuiApplication::setApplicationDisplayName(to_qstring(name));
}

void NativeApplication::set_application_version(rust::Str version) {
    QApplication::setApplicationVersion(to_qstring(version));
}

bool NativeApplication::is_widgets_application() const {
    return qobject_cast<QApplication*>(QCoreApplication::instance()) != nullptr;
}

std::unique_ptr<NativeApplication> create_native_application(rust::Vec<rust::String> args) {
    return std::make_unique<NativeApplication>(std::move(args));
}

} // namespace dualsense_updater
