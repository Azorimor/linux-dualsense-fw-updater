# Linux DualSense Firmware Updater

An unofficial Linux KDE Kirigami desktop application ([`Azorimor/linux-dualsense-fw-updater`](https://github.com/Azorimor/linux-dualsense-fw-updater)) for inspecting and updating Sony PlayStation DualSense (`054c:0ce6`) and DualSense Edge (`054c:0df2`) controller firmware over USB HID.

## Disclaimer

This is not an official Sony application. If you can, use Sony’s official updater
for Windows instead. If you don’t have convenient access to a Windows machine,
you can try this tool.

Use at your own risk. Firmware upgrades are auto-committed by the controller
even if the verify and finalize steps are never run. There are no guarantees
this won’t brick your controller, though the device appears to verify the image
before committing, so a failed update would normally leave the current firmware
intact. This has only been tested on the standard DualSense, not the DualSense
Edge.

## Features & Guided GUI Workflow

Launch `dualsense-updater` and follow the four guided KDE Kirigami wizard steps:

1. **Controller** — Connects over a wired USB data cable (updates cannot run over Bluetooth) and automatically inspects connected DualSense / DualSense Edge controllers, displaying:
   - Board revision (`BDM-020`, `BDM-030`, `BDM-040`, `BDM-050`, `BDM-060`, or `DualSense Edge`) and mapped hardware target (`0004`, `000B`, `000E`, `0044`)
   - Current firmware version, FW series, build date/time, serial number, and controller shell color
   - **Live USB unplug/replug detection** (background polling updates the UI automatically if the controller is disconnected or swapped while idle)
2. **Firmware** — Choose either:
   - **Download Official Sony Firmware**: checks Sony's catalog (`info.json`) for your controller target and downloads the latest official image automatically.
   - **Use a Local Firmware File (`.bin`)**: inspects a local firmware image on disk.
3. **Review** — Compares current and target firmware versions, warns if the controller is already on the same or newer version or if the target is untested (DualSense Edge), and requires explicit confirmation that firmware updates cannot be reversed.
4. **Update** — Streams download progress (with safe cancellation before flashing begins), initializes the update, writes firmware blocks, verifies, and finalizes. If an error occurs, plain-English recovery hints and `Scan Controllers` / `Try Again` / `Start Over` actions are shown.

- **Built-in Licenses Window**: Open **Help → Licenses** in the application menu bar at any time to inspect the bundled third-party crate notices and MIT license text.

## Installation (AppImage & Prebuilt Archives)

Download the latest release artifacts from [GitHub Releases](https://github.com/Azorimor/linux-dualsense-fw-updater/releases):

- **AppImage (`x86_64`)**:
  ```sh
  chmod +x linux-dualsense-fw-updater-<tag>-x86_64.AppImage
  ./linux-dualsense-fw-updater-<tag>-x86_64.AppImage
  ```
- **Tarball (`x86_64`)**:
  ```sh
  tar -xzf linux-dualsense-fw-updater-<tag>-x86_64.tar.gz
  cd linux-dualsense-fw-updater-<tag>-x86_64
  ./dualsense-updater
  ```

> **USB Permissions (`udev`)**: On Linux, you may need udev rules granting your user read/write access to DualSense HID devices (`/dev/hidraw*` / USB vendor ID `054c`) to inspect or flash without root privileges.

## Building from Source

### Linux Build Prerequisites

- **Rust toolchain** (Edition 2024, MSRV `1.85.0`) and a **C++17 / `clang` toolchain** (for `cxx` / `cxx-qt-build`)
- **`mold` linker** (configured for Linux targets via `.cargo/config.toml` with `-C link-arg=-fuse-ld=mold`)
- **Qt 6** development packages: `Qt6Core`, `Qt6Gui`, `Qt6Qml`, `Qt6Quick`, `Qt6QuickControls2`, and `qmake6`
- **KDE Frameworks 6 (KF6) runtime QML modules**: `Kirigami` (`org.kde.kirigami`) and `qqc2-desktop-style` (`org.kde.desktop`; falls back to Qt `Fusion` style if unavailable, or override with `QT_QUICK_CONTROLS_STYLE`)
- **`hidapi`** development library (`hidapi-hidraw` / `hidapi-libusb`) and `pkg-config`

### `qmake6` Note

`build.rs` automatically uses `/usr/bin/qmake6` when `QMAKE` is unset and `/usr/bin/qmake6` exists. If your distribution's default `qmake` points to Qt 5 or `qmake6` is installed elsewhere, export `QMAKE` explicitly:

```sh
export QMAKE=/usr/bin/qmake6
```

### Build & Run

```sh
QMAKE=/usr/bin/qmake6 cargo build --release
./target/release/dualsense-updater
```

Or run directly from the workspace root:

```sh
QMAKE=/usr/bin/qmake6 cargo run --release
```

Logging uses `env_logger` (default `info`). Set `RUST_LOG=debug` for verbose HID and update diagnostics.

### Packaging an AppImage Locally

After building the release binary, package both the portable `.tar.gz` and `.AppImage` into `dist/`:

```sh
./scripts/package-appimage.sh target/release/dualsense-updater v0.1.0 dist
```

Desktop integration assets live in `packaging/linux/dev.dualsense.updater.desktop` and `packaging/linux/dev.dualsense.updater.svg`.

## Testing, Coverage, and Structural Checks

All workspace unit and integration tests run headlessly without a display server, GPU, or attached DualSense controller:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/verify_structural_rules.py
cargo test --workspace
```

Optional license audit (requires `cargo-deny` using `deny.toml`):

```sh
cargo deny --locked --all-features check licenses
```

To verify QML loading against the offscreen Qt platform plugin in headless environments:

```sh
QT_QPA_PLATFORM=offscreen timeout 5s cargo run
```

To generate workspace code coverage reports (requires `cargo-llvm-cov` and the `llvm-tools` rustup component):

```sh
./scripts/coverage.sh
```

This writes LCOV data to `target/coverage/lcov.info`, HTML reports to `target/coverage/html/index.html`, and prints a workspace summary table.

GitHub Actions workflows under `.github/workflows/` (`ci.yml`, `coverage.yml`, `release.yml`, `deploy-website.yml`, `cleanup-pr-cache.yml`) target `ubuntu-24.04` and are enabled for manual `workflow_dispatch` runs (with automatic PR/tag/Pages triggers ready to uncomment once the repository is public).

## Sony Firmware Sources & Hardware Targets

Sony publishes the latest version metadata at:

```text
https://fwupdater.dl.playstation.net/fwupdater/info.json
```

Observed board-to-target mappings:

- DualSense BDM-020 -> `0004`
- DualSense BDM-030 -> `0004`
- DualSense BDM-040 -> `0004`
- DualSense BDM-050 -> `000B`
- DualSense BDM-060 -> `000E`
- DualSense Edge -> `0044`

In practice, attempting to flash firmware for the wrong SoC target fails verification rather than bricking the controller.

Firmware images can be downloaded directly from:

```text
https://fwupdater.dl.playstation.net/fwupdater/fwupdate<target>/<version>/FWUPDATE<target>.bin
```

Example: to get version `0x0630` for DualSense BDM-050 (`000B`):

```text
https://fwupdater.dl.playstation.net/fwupdater/fwupdate000B/0x0630/FWUPDATE000B.bin
```

## Notes

- A controller already running the latest firmware may not return success codes past the start-update phase; this is expected because Sony's firmware blocks downgrades/re-flashing the same version.
- On Linux, you may need udev permissions to access DualSense HID devices (`/dev/hidraw*` / USB VID `054c`) without root privileges.

## Acknowledgments

The core DualSense USB HID protocol and firmware update backend logic is adapted from the original CLI project by [nchie](https://github.com/nchie/dualsense-updater-rs) under the MIT License.

## License

MIT. See `LICENSE`.
