// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use dualsense_updater::app_icon::{DESKTOP_FILE_ID, EMBEDDED_APP_ICON_URL};

const ICON_SIZES: &[(&str, u32)] = &[
    ("16x16", 16),
    ("22x22", 22),
    ("32x32", 32),
    ("48x48", 48),
    ("64x64", 64),
    ("128x128", 128),
    ("256x256", 256),
    ("512x512", 512),
];

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn create_temp_dir(label: &str) -> PathBuf {
    let unique = format!(
        "dualsense-updater-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    fs::create_dir_all(&dir).expect("create temporary test directory");
    dir
}

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn read_png_dimensions(path: &Path) -> (u32, u32) {
    let bytes = fs::read(path).expect("read png icon");
    assert!(bytes.len() >= 24, "PNG too small: {}", path.display());
    assert_eq!(
        &bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "invalid PNG signature: {}",
        path.display()
    );
    assert_eq!(
        &bytes[12..16],
        b"IHDR",
        "expected IHDR chunk in {}",
        path.display()
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (width, height)
}

#[test]
fn desktop_icons_are_canonical_and_free_of_duplicates() {
    assert!(
        !repo_path("assets/desktop").exists(),
        "assets/desktop should be moved into packaging/linux/icons without duplicates"
    );
    assert!(
        !repo_path("packaging/linux/dev.dualsense.updater.svg").exists(),
        "legacy packaging/linux/dev.dualsense.updater.svg should be removed"
    );

    for (size_dir, expected_px) in ICON_SIZES {
        let icon_path = repo_path(&format!("packaging/linux/icons/{size_dir}/icon.png"));
        assert!(icon_path.is_file(), "missing icon: {}", icon_path.display());
        let (w, h) = read_png_dimensions(&icon_path);
        assert_eq!(
            (w, h),
            (*expected_px, *expected_px),
            "unexpected PNG dimensions for {}",
            icon_path.display()
        );
    }

    let svg_path = repo_path("packaging/linux/icons/scalable/icon.svg");
    let svg = fs::read_to_string(&svg_path).expect("read scalable SVG icon");
    assert!(
        svg.contains("<svg"),
        "expected <svg element in {}",
        svg_path.display()
    );
}

#[test]
fn desktop_entry_and_qt_integration_match_app_id_and_icons() {
    assert_eq!(DESKTOP_FILE_ID, "dev.dualsense.updater");
    assert_eq!(
        EMBEDDED_APP_ICON_URL,
        "qrc:/qt/qml/dev/dualsense/updater/packaging/linux/icons/256x256/icon.png"
    );

    let desktop = fs::read_to_string(repo_path("packaging/linux/dev.dualsense.updater.desktop"))
        .expect("read desktop file");
    assert!(desktop.contains("[Desktop Entry]"));
    assert!(desktop.contains("Type=Application\n"));
    assert!(desktop.contains("Name=DualSense Firmware Updater\n"));
    assert!(desktop.contains("Exec=dualsense-updater\n"));
    assert!(desktop.contains("Icon=dev.dualsense.updater\n"));
    assert!(desktop.contains("StartupWMClass=dualsense-updater\n"));
    assert!(desktop.contains("Terminal=false\n"));
    assert!(desktop.contains("StartupNotify=true\n"));
    assert!(desktop.contains("Categories=Settings;HardwareSettings;Qt;KDE;\n"));

    let build_rs = fs::read_to_string(repo_path("build.rs")).expect("read build.rs");
    let app_icon_cpp =
        fs::read_to_string(repo_path("src/app_icon.cpp")).expect("read src/app_icon.cpp");
    for (size_dir, _) in ICON_SIZES {
        let rel_png = format!("packaging/linux/icons/{size_dir}/icon.png");
        assert!(build_rs.contains(&rel_png), "build.rs missing {rel_png}");
        assert!(
            app_icon_cpp.contains(&format!(":/qt/qml/dev/dualsense/updater/{rel_png}")),
            "src/app_icon.cpp missing qrc path for {rel_png}"
        );
    }
    let rel_svg = "packaging/linux/icons/scalable/icon.svg";
    assert!(build_rs.contains(rel_svg));
    assert!(app_icon_cpp.contains(&format!(":/qt/qml/dev/dualsense/updater/{rel_svg}")));

    let main_rs = fs::read_to_string(repo_path("src/main.rs")).expect("read src/main.rs");
    assert!(main_rs.contains("QGuiApplication::set_desktop_file_name"));
    assert!(main_rs.contains("configure_window_icon"));

    let about_qml =
        fs::read_to_string(repo_path("qml/AboutWindow.qml")).expect("read qml/AboutWindow.qml");
    assert!(about_qml.contains(EMBEDDED_APP_ICON_URL));
}

#[test]
fn install_desktop_script_installs_and_uninstalls_all_xdg_assets() {
    let temp = TempDirGuard(create_temp_dir("install-desktop"));
    let script = repo_path("packaging/linux/install-desktop.sh");

    let install = Command::new("bash")
        .arg(&script)
        .arg("--prefix")
        .arg(&temp.0)
        .arg("--exec")
        .arg("/opt/dualsense/dualsense-updater")
        .output()
        .expect("run install-desktop.sh");
    assert!(
        install.status.success(),
        "install-desktop.sh failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );

    let desktop_installed = temp
        .0
        .join("share/applications/dev.dualsense.updater.desktop");
    assert!(desktop_installed.is_file());
    let desktop_contents =
        fs::read_to_string(&desktop_installed).expect("read installed desktop file");
    assert!(desktop_contents.contains("Exec=/opt/dualsense/dualsense-updater\n"));
    assert!(desktop_contents.contains("Icon=dev.dualsense.updater\n"));

    for (size_dir, _) in ICON_SIZES {
        let icon_dest = temp.0.join(format!(
            "share/icons/hicolor/{size_dir}/apps/dev.dualsense.updater.png"
        ));
        assert!(
            icon_dest.is_file(),
            "missing installed icon: {}",
            icon_dest.display()
        );
    }
    let svg_dest = temp
        .0
        .join("share/icons/hicolor/scalable/apps/dev.dualsense.updater.svg");
    assert!(svg_dest.is_file(), "missing installed SVG icon");

    let uninstall = Command::new("bash")
        .arg(&script)
        .arg("--prefix")
        .arg(&temp.0)
        .arg("--uninstall")
        .output()
        .expect("run install-desktop.sh --uninstall");
    assert!(
        uninstall.status.success(),
        "install-desktop.sh --uninstall failed: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );

    assert!(!desktop_installed.exists());
    assert!(!svg_dest.exists());
    for (size_dir, _) in ICON_SIZES {
        let icon_dest = temp.0.join(format!(
            "share/icons/hicolor/{size_dir}/apps/dev.dualsense.updater.png"
        ));
        assert!(!icon_dest.exists());
    }
}

#[test]
fn package_appimage_stage_only_bundles_tarball_and_appdir_icons() {
    let temp = TempDirGuard(create_temp_dir("package-stage"));
    let dummy_bin = temp.0.join("dualsense-updater");
    fs::write(&dummy_bin, "#!/usr/bin/env sh\nexit 0\n").expect("write dummy executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dummy_bin)
            .expect("stat dummy bin")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dummy_bin, perms).expect("chmod dummy bin");
    }

    let out_dir = temp.0.join("dist");
    let script = repo_path("scripts/package-appimage.sh");
    let status = Command::new("bash")
        .arg(&script)
        .arg("--stage-only")
        .arg(&dummy_bin)
        .arg("v0.0.0-test")
        .arg(&out_dir)
        .output()
        .expect("run package-appimage.sh --stage-only");
    assert!(
        status.status.success(),
        "package-appimage.sh --stage-only failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    let tarball = out_dir.join("linux-dualsense-fw-updater-v0.0.0-test-x86_64.tar.gz");
    assert!(tarball.is_file());
    let tar_list = Command::new("tar")
        .arg("-tzf")
        .arg(&tarball)
        .output()
        .expect("list tarball contents");
    assert!(tar_list.status.success());
    let tar_entries = String::from_utf8_lossy(&tar_list.stdout);
    let prefix = "linux-dualsense-fw-updater-v0.0.0-test-x86_64";
    assert!(tar_entries.contains(&format!("{prefix}/dualsense-updater\n")));
    assert!(tar_entries.contains(&format!("{prefix}/dev.dualsense.updater.desktop\n")));
    assert!(tar_entries.contains(&format!("{prefix}/install-desktop.sh\n")));
    assert!(tar_entries.contains(&format!("{prefix}/icons/scalable/icon.svg\n")));
    for (size_dir, _) in ICON_SIZES {
        assert!(tar_entries.contains(&format!("{prefix}/icons/{size_dir}/icon.png\n")));
    }

    let appdir = out_dir.join("AppDir");
    assert!(appdir.join("usr/bin/dualsense-updater").is_file());
    assert!(
        appdir
            .join("usr/share/applications/dev.dualsense.updater.desktop")
            .is_file()
    );
    assert!(appdir.join(".DirIcon").is_file());
    assert!(
        appdir
            .join("usr/share/icons/hicolor/scalable/apps/dev.dualsense.updater.svg")
            .is_file()
    );
    for (size_dir, _) in ICON_SIZES {
        assert!(
            appdir
                .join(format!(
                    "usr/share/icons/hicolor/{size_dir}/apps/dev.dualsense.updater.png"
                ))
                .is_file()
        );
    }
}
