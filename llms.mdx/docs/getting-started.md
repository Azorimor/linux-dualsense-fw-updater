# Getting Started & Installation (/docs/getting-started)

## Option 1: Portable Archive (`.tar.gz`) — Recommended [#option-1-portable-archive-targz--recommended]

Pre-built x86_64 portable `.tar.gz` archives (recommended) and AppImage releases are available via the [Download](/download) page (requiring safety acknowledgment) and on [GitHub Releases](https://github.com/Azorimor/linux-dualsense-fw-updater/releases).

Extract the archive and run the application directly:

```sh
tar -xzf linux-dualsense-fw-updater-*-x86_64.tar.gz
cd linux-dualsense-fw-updater-*-x86_64
./dualsense-updater
```

Optionally install the desktop menu entry and application icons for your user account (or system-wide / uninstall later):

```sh
./install-desktop.sh           # Install for current user (~/.local/share)
sudo ./install-desktop.sh --system  # Optional: install system-wide (/usr/local/share)
./install-desktop.sh --uninstall    # Remove installed desktop entry and icons
```

> [!INFO] Wired USB Connection Required
>
> Firmware inspection and updates operate over USB HID only. Connect your DualSense controller using a reliable USB-C data cable (not Bluetooth, and avoid unpowered USB hubs).

### Desktop Runtime Requirements [#desktop-runtime-requirements]

The portable `.tar.gz` release relies on the **Qt 6** and **KDE Frameworks 6 (Kirigami)** libraries installed on your Linux system rather than bundling them.

* **If you use KDE Plasma 6:** These packages are usually already installed, and the app should launch right away.
* **If you use another desktop environment (such as GNOME, Cinnamon, XFCE, or MATE):** You will likely need to install the required Qt 6 Kirigami runtime packages manually before running the app.

> [!WARNING] Maybe out of date
>
> Package names and installation commands vary across Linux distributions and change over time. Because we do not regularly test or maintain instructions for every distro, the commands below may become outdated. If a package name is not found, search your distribution's package manager for `kirigami` and `qqc2-desktop-style` (Qt 6 / KF6 versions).

#### Installing Required Packages [#installing-required-packages]

Open your **Terminal** application and run the command that matches your Linux distribution:

* **Ubuntu, Debian, Linux Mint, Pop!_OS:**
```bash
sudo apt update && sudo apt install qml6-module-org-kde-kirigami qml6-module-org-kde-desktop
```

* **Fedora:**
```bash
sudo dnf install kf6-kirigami kf6-qqc2-desktop-style
```

* **Arch Linux, Manjaro, EndeavourOS:**
```bash
sudo pacman -S kirigami qqc2-desktop-style
```

* **openSUSE (Tumbleweed / Leap):**
```bash
sudo zypper install kf6-kirigami kf6-qqc2-desktop-style
```

#### Theme & Styling Behavior [#theme--styling-behavior]

If the native KDE desktop style (`org.kde.desktop`) is not installed or unavailable on your system, the application will still run by automatically falling back to Qt's built-in `Fusion` theme. You can also force a specific Qt Quick Controls style by setting the `QT_QUICK_CONTROLS_STYLE` environment variable before launching the app (for example: `QT_QUICK_CONTROLS_STYLE=Fusion ./your-app`).

---

## Option 2: AppImage (Alternative) [#option-2-appimage-alternative]

If you prefer a single self-contained executable with a bundled Qt 6 + Kirigami runtime, download the `.AppImage` release instead:

```sh
chmod +x linux-dualsense-fw-updater-*-x86_64.AppImage
./linux-dualsense-fw-updater-*-x86_64.AppImage
```

---

## Option 3: Build from Source [#option-3-build-from-source]

### Prerequisites [#prerequisites]

- **Rust toolchain** (Edition 2024) and a **C++17 / `clang` toolchain** (for `cxx` / `cxx-qt-build`)
- **`mold` linker** (configured for Linux targets in `.cargo/config.toml`)
- **Qt 6** development packages (`Qt6Core`, `Qt6Gui`, `Qt6Qml`, `Qt6Quick`, `Qt6QuickControls2`, and `qmake6`)
- **KDE Frameworks 6** QML modules (`Kirigami` and `qqc2-desktop-style`)
- **`hidapi`** development library (`libhidapi-dev` / `hidapi`) and `pkg-config`

### Build & Run [#build--run]

`build.rs` automatically picks `/usr/bin/qmake6` when `QMAKE` is unset. If `qmake6` lives elsewhere on your distribution, export `QMAKE` explicitly:

```sh
git clone https://github.com/Azorimor/linux-dualsense-fw-updater.git
cd linux-dualsense-fw-updater
export QMAKE=/usr/bin/qmake6
cargo build --release
./target/release/dualsense-updater
```

Enable verbose HID and update diagnostics via `RUST_LOG`:

```sh
RUST_LOG=debug ./target/release/dualsense-updater
```