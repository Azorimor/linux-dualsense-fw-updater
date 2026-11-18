// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include "rust/cxx.h"

#include <memory>
#include <string>
#include <vector>

class QApplication;

namespace dualsense_updater {

class NativeApplication {
public:
    explicit NativeApplication(rust::Vec<rust::String> args);
    ~NativeApplication();

    NativeApplication(const NativeApplication&) = delete;
    NativeApplication& operator=(const NativeApplication&) = delete;
    NativeApplication(NativeApplication&&) = delete;
    NativeApplication& operator=(NativeApplication&&) = delete;

    int exec();
    void set_application_name(rust::Str name);
    void set_application_display_name(rust::Str name);
    void set_application_version(rust::Str version);
    bool is_widgets_application() const;

private:
    std::vector<std::string> arg_storage_;
    std::vector<char*> argv_;
    int argc_;
    std::unique_ptr<QApplication> app_;
};

std::unique_ptr<NativeApplication> create_native_application(rust::Vec<rust::String> args);

} // namespace dualsense_updater
