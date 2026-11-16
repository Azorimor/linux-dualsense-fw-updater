# DualSense Updater

An unofficial Linux KDE Kirigami desktop application for inspecting and updating Sony PlayStation DualSense controller firmware over USB HID.

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

## Guided GUI Workflow

Launch the desktop application (`dualsense-updater`) and follow the four guided steps:

1. **1. Controller** — Connects over USB HID (`054c:0ce6` DualSense, `054c:0df2` DualSense Edge) and displays the detected controller's current firmware version and build info. A wired USB data cable is required (updates cannot run over Bluetooth).
2. **2. Firmware** — Choose either:
   - **Download Official Sony Firmware**: selects your controller hardware target (`0004`, `000B`, `000E`, or `0044`), checks Sony's catalog (`info.json`), and downloads the latest image automatically.
   - **Use a Local Firmware File (`.bin`)**: inspects a local firmware image file on disk.
3. **3. Review** — Compares current and target firmware versions, warns if the controller is already on the same or newer version or if the target is untested (DualSense Edge), and requires explicit confirmation that firmware updates cannot be reversed.
4. **4. Update** — Streams download progress (with safe cancellation before flashing begins), initializes the update, writes firmware blocks, verifies, and finalizes. If an error occurs, plain-English recovery hints and `Scan Controllers` / `Try Again` / `Start Over` actions are shown.

## Linux Build Prerequisites

- **Rust toolchain** (Edition 2024) and a **C++17 / `clang` toolchain** (for `cxx` / `cxx-qt-build`)
- **Qt 6** development packages: `Qt6Core`, `Qt6Gui`, `Qt6Qml`, `Qt6Quick`, `Qt6QuickControls2`, and `qmake6`
- **KDE Frameworks 6 (KF6) runtime QML modules**: `Kirigami` (`org.kde.kirigami`) and `qqc2-desktop-style` (`org.kde.desktop`; falls back to Qt `Fusion` style if unavailable, or override with `QT_QUICK_CONTROLS_STYLE`)
- **`hidapi`** development library (`hidapi-hidraw` / `hidapi-libusb`) and `pkg-config`

### `qmake6` Note

`build.rs` automatically uses `/usr/bin/qmake6` when `QMAKE` is unset and `/usr/bin/qmake6` exists. If your distribution's default `qmake` points to Qt 5 or `qmake6` is installed at a different path, export `QMAKE` explicitly:

```sh
export QMAKE=/usr/bin/qmake6
```

## Build, Run, and Test

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

### Headless CI Testing

All workspace unit and integration tests run headlessly without a display server, GPU, or attached DualSense controller:

```sh
cargo test --workspace
```

To verify QML loading against the offscreen Qt platform plugin in headless environments:

```sh
QT_QPA_PLATFORM=offscreen timeout 5s cargo run
```

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
