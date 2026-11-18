<!--
SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Feature Ideas for Linux DualSense Firmware Updater

This document collects realistic, focused feature ideas based on the current `v0.0.2` state of the project (`dualsense-core`, `dualsense-ui`, the Qt6/Kirigami desktop GUI, and Linux packaging).

**Guiding principle:** Keep `dualsense-updater` a small, trustworthy, single-purpose tool for inspecting and updating DualSense firmware on Linux—no feature creep into controller remapping, RGB/haptic profiles, or background daemons.

---

## 1. Pre-Flash Safety & Validation

High-value improvements that prevent avoidable user mistakes before any HID write command is sent:

- **Full validation for local `.bin` files before flashing**
  - *Current gap:* `DownloadAndInstall` validates images with `FirmwareImage::validate_for_release()` (checking the `Sony Interactive Entertainment` header signature, embedded USB PID at `0x62`, and target ID at `0x74`), whereas `InspectLocalFile` in `crates/dualsense-ui/src/controller.rs` currently only calls `FirmwareImage::from_file()` (size and version at `0x78`).
  - *Idea:* Run `validate_sony_header()` and compare `embedded_pid()` / `embedded_target_id()` against the selected controller during local file inspection. Surface mismatches clearly in Step 1/Step 2 (or block mismatched targets by default) before the user clicks **Start Update**.

- **Bluetooth-connected controller detection & clearer guidance**
  - *Current gap:* `HidDeviceEnumerator::list_dualsense_devices()` enumerates `0x054c:0x0ce6` and `0x054c:0x0df2` without distinguishing Bluetooth HID connections from USB, so a controller paired only over Bluetooth either shows *"No DualSense Controller Found"* or fails later when reading USB-only feature reports.
  - *Idea:* Inspect the `hidapi` bus type / interface metadata and, if a DualSense is seen over Bluetooth with no USB connection, display a tailored hint: *"Controller detected over Bluetooth — connect it with a USB data cable to inspect and update firmware."*

- **Battery level check before flashing**
  - *Idea:* Read the controller's battery level / charging state before starting an update and warn (or require confirmation) if battery is critically low, reducing the chance of an unexpected brownout on flaky USB ports.

- **Window-close guard during active flashing**
  - *Idea:* Intercept `Closing` on `Kirigami.ApplicationWindow` in `qml/Main.qml` while `updater.isFlashing` is active so users cannot accidentally close the app mid-`WriteUpdateImage`.

- **Post-flash hardware verification**
  - *Current gap:* On success, `handle_update_finished` updates `dev.firmware_version` from `image.version()`.
  - *Idea:* After `FinalizeUpdate`, attempt a best-effort `0x20` `FirmwareInfo` re-read (with short retry if the controller re-enumerates) to confirm the hardware actually reports the new `fw_version` and `build_date`/`build_time`.

---

## 2. Smoother Firmware Update Flow

Small UX refinements that keep the 4-step wizard (`Connect → Firmware → Review → Update`) intact while cutting unnecessary clicks:

- **Automatic catalog check after controller detection**
  - *Current gap:* After a controller is detected, the user must click **Check for Update** in Step 1 to query `info.json`.
  - *Idea:* Optionally (or automatically once per session when a supported board is selected) fetch Sony's `info.json` in the background and show an immediate status badge on Step 0/Step 1 (e.g., *"Up to date (`0x0630`)"* vs. *"Update available: `0x0510` → `0x0630`"*), while keeping a **Refresh** button.

- **Auto-inspect all connected controllers when multiple are plugged in**
  - *Current gap:* On scan, only the first/selected controller reads `0x20` firmware info and `0x80`/`0x81` serial/colorway; additional devices show generic HID metadata until clicked.
  - *Idea:* Inspect all enumerated USB DualSense controllers on scan so the multi-controller selector shows each controller's board revision, colorway, and current firmware version side by side.

- **Reuse or save downloaded official `.bin` images**
  - *Idea:* When updating multiple controllers of the same hardware target (e.g., two `BDM-050` controllers), cache the verified image in memory for the session or offer an optional **"Save Firmware Copy…"** button after download so it doesn't have to be fetched repeatedly.

- **Show firmware version comparison right in Step 1**
  - *Idea:* Display current vs. latest version (and whether the controller is already up to date) directly alongside the **Check for Update** result before advancing to the Review step.

---

## 3. Linux Onboarding & Packaging

Addressing the most common Linux desktop friction points:

- **Bundled `udev` rules & helper in `packaging/linux/`**
  - *Current gap:* `udev` rules for `/dev/hidraw*` (`054c:0ce6` and `054c:0df2`) are documented in `website/content/docs/safety-and-troubleshooting.mdx` and referenced in `error_map.rs`, but no `.rules` file is shipped in `packaging/linux/`.
  - *Idea:*
    1. Add `packaging/linux/70-dualsense-updater.rules` to the repo and release tarball/AppImage.
    2. Add an `--udev` flag (or prompt) in `packaging/linux/install-desktop.sh` to install the rules and reload `udevadm`.
    3. On a USB permission error (`EACCES` / `EPERM`), optionally offer a one-click **"Install USB Permissions…"** action (via `pkexec`) or a **"Copy udev Command"** button right inside the Error screen.

- **Flatpak & distro packaging**
  - *Idea:* Complement the existing AppImage and `.tar.gz` releases with a Flatpak manifest (with `--device=all` / `hidraw` access) and/or community packaging recipes (AUR, `.deb`, Fedora COPR) so desktop users get automatic updates through their software center.

- **Lightweight headless CLI flags (`--info`, `--check`, `--update`)**
  - *Idea:* Since `dualsense-core` and `dualsense-ui` already have zero Qt dependencies, exposing a small CLI subcommand mode on the binary (or a companion `dualsense-updater-cli` binary in the workspace) would help SteamOS / Bazzite / headless SSH users inspect firmware versions and update without needing a display server, while launching the GUI by default when invoked with no arguments.

---

## 4. Hardware Coverage & Diagnostics

- **Promote DualSense Edge (`0044`) once hardware-verified**
  - *Idea:* Use incoming `[Flash Report]` issues to verify `0x0df2` / `0044` end-to-end on real DualSense Edge hardware and update `KNOWN_TARGETS` (`tested: true`) and docs accordingly.

- **Keep board revision & colorway tables current**
  - *Idea:* Extend `BoardModel` and `decode_ds5_color` in `crates/dualsense-core/src/protocol.rs` as Sony introduces newer DualSense board revisions (beyond `BDM-060`) or new shell colorways, and log raw `hw_info` / `fw_series` clearly when an unknown board is encountered so users can easily report it.

- **In-app diagnostic log viewer / copy button**
  - *Current gap:* Detailed HID packet/status diagnostics (`RUST_LOG=debug`) require launching the app from a terminal.
  - *Idea:* Capture recent sanitized `log` records in a small in-memory ring buffer and add a **"Copy Diagnostics"** / **"Save Log…"** button on the Error screen and in `FlashReportWindow.qml` (never including serial numbers unless the user explicitly copies them).

---

## 5. Desktop Polish & Accessibility

- **Keyboard navigation & shortcuts**
  - *Idea:* Wire standard desktop shortcuts in `qml/Main.qml` (`F5` / `Ctrl+R` to scan controllers, `Ctrl+Q` to quit, `Escape` to cancel when safe).
- **Remember last local firmware folder**
  - *Idea:* Remember the folder from the last `FileDialog` selection during the session (or via lightweight XDG state) when browsing `.bin` files.
- **Internationalization (i18n)**
  - *Idea:* Wrap user-facing strings in `qsTr()` (QML) and prepare translation files so community contributors can localize the wizard.
- **Subtle colorway-aware controller badge**
  - *Idea:* `color_name` is already decoded from the serial number; pair it with a small color swatch next to the controller card in Step 0/Step 1 for quick visual confirmation when multiple DualSense controllers are plugged in.

---

## Non-Goals (Keeping It Simple)

To preserve reliability and keep the codebase small and auditable, the following are intentionally **out of scope**:

- Button remapping, stick calibration, deadzone tuning, or trigger/haptic profile editors.
- RGB LED / player-indicator customization.
- Background system-tray daemons or auto-flashing without explicit user confirmation.
- Unofficial/custom firmware patching or downgrade bypasses (firmware downgrades are blocked by the DualSense bootloader itself).
