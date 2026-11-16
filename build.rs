use std::path::Path;

use cxx_qt_build::CxxQtBuilder;

fn main() {
    configure_qmake6();

    CxxQtBuilder::new()
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .file("src/qt_bridge.rs")
        .build();
}

fn configure_qmake6() {
    println!("cargo::rerun-if-env-changed=QMAKE");
    if std::env::var_os("QMAKE").is_some() {
        return;
    }

    let qmake6 = Path::new("/usr/bin/qmake6");
    if qmake6.exists() {
        // SAFETY: build.rs executes single-threaded before invoking cxx-qt-build.
        unsafe {
            std::env::set_var("QMAKE", qmake6);
        }
    }
}
