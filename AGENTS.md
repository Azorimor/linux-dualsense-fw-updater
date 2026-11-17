<!--
SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Coding Agent Instructions

This file governs all AI coding agents working in `dualsense-updater-rs`.

> **Context:** This repository implements the 3-crate `dualsense-updater` workspace (`dualsense-core`, `dualsense-ui`, and the `cxx-qt` + KDE Kirigami GUI). Read the mandatory workflow rules and workspace documentation below before making any changes.

---

## 1. Mandatory Git & Commit Rules (Non-Negotiable)

### 1.1 Never Write to `main` or `master`
- **NEVER** edit repository files, commit, merge, rebase, cherry-pick, or push directly to `main` or `master`.
- At the start of every session, verify the active branch with `git branch --show-current`.
- If you are on `main` or `master`, immediately create and switch to a working branch **before** modifying any file:
  ```sh
  git switch -c <type>/<short-topic>
  # examples: feat/protocol-rewrite, refactor/hid-trait, chore/pin-deps
  ```
- Never run `git commit --no-verify`, `git commit -n`, or `git push --no-verify`.

### 1.2 Use Conventional Commits
Every commit message **must** follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/):

```text
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

- **Allowed types:** `feat`, `fix`, `refactor`, `perf`, `style`, `test`, `docs`, `build`, `ci`, `chore`, `revert`.
- **Scopes:** optional short module/area name in parentheses, e.g. `feat(protocol): ...`, `refactor(hid): ...`, `test(cli): ...`.
- **Breaking changes:** append `!` before `:` (e.g. `feat(cli)!: replace update flags`) and/or add a `BREAKING CHANGE:` footer.
- **Subject line:** imperative mood, concise, ideally `<= 72` characters.

### 1.3 Small, Incremental Commits (No Giant Final Commit)
- **NEVER** accumulate an entire rewrite or multi-part task into a single giant commit at the end.
- Commit incrementally after each small, self-contained logical unit of work—for example:
  1. Pinning or updating dependencies in `Cargo.toml`
  2. Defining a new error/protocol type or transport trait
  3. Implementing one module or command path
  4. Adding unit/integration tests for that unit
  5. Removing obsolete modules/code once replaced
- Keep the crate compiling (`cargo check` / `cargo test`) at each commit whenever practical.
- Check `git status` and `git diff --stat` frequently so each commit stays focused and reviewable.

### 1.4 Automated Enforcement in This Repo
- **Git hooks (`.githooks/`):**
  - `pre-commit`: blocks commits on `main`/`master` and warns on overly large staged diffs.
  - `commit-msg`: rejects commit subjects that do not follow Conventional Commits.
  - `pre-push`: blocks pushes targeting `main` or `master`.
  - If `git config --get core.hooksPath` is not `.githooks`, activate it immediately:
    ```sh
    git config core.hooksPath .githooks
    ```
- **Agent lifecycle hooks (`.agents/hooks.json`):**
  - `SessionStart` ensures `core.hooksPath` is set and reminds you of the active branch.
  - `PreToolUse` blocks file edits and commits while on `main`/`master`, blocks `--no-verify`, blocks pushes to `main`/`master`, and rejects non-conventional `-m` commit messages.

---

## 2. Testing & UI Architecture Requirements (Mandatory)

### 2.1 Full Unit & Integration Test Coverage for Business Logic
- **100% of newly added or rewritten business logic** must be covered by unit and integration tests—do not ship untested modules or defer testing to the end.
- Abstract hardware, OS, and external I/O boundaries (such as USB HID transport, timers/sleeps, and network/filesystem calls) behind traits so protocol parsing, chunking, update state machines, retry/timeout behavior, error handling, and CLI/application orchestration can be deterministically tested in CI without physical DualSense hardware.
- Include integration tests verifying end-to-end workflows (e.g., simulated update flows across mock HID transports, firmware image validation, and failure recovery paths).

### 2.2 Headless-Testable UI Design (No Desktop Environment in CI)
- Any UI (GUI, TUI, or interactive terminal interface) **must** keep presentation/rendering strictly decoupled from business logic, application state, and user-flow orchestration (e.g., Elm/TEA `State` + `Message` + `update`, MVVM/ViewModel, or presenter pattern).
- Keep toolkit/widget rendering code thin and purely declarative over testable state/controller types so that UI state transitions, input validation, action gating, progress/status formatting, and error handling can all be unit- and integration-tested directly.
- All automated tests (`cargo test`) **must run and pass in a headless CI environment** with no X11/Wayland display server, desktop environment, GPU, or attached controller required.

---

## 3. Current Project Overview

`dualsense-updater` (`v0.0.0`, Rust `edition = "2024"`, `GPL-3.0-or-later`) is a Linux KDE Kirigami desktop application (organized as a 3-crate Cargo workspace) for inspecting and updating Sony PlayStation DualSense (`0x054c:0x0ce6`) and DualSense Edge (`0x054c:0x0df2`) firmware over USB HID.

### Licensing & FSFE REUSE Compliance (Mandatory)
- **Project license:** `GPL-3.0-or-later` (`Cargo.toml`, `LICENSES/GPL-3.0-or-later.txt`).
- **Inherited core files:** The 10 files adapted from `nchie`'s original CLI project (`crates/dualsense-core/src/{error,image,protocol,transport,updater}.rs` and `crates/dualsense-core/src/tests/*_test.rs` for those 5 modules) retain `SPDX-FileCopyrightText: 2026 nchie`, `SPDX-FileCopyrightText: 2026 Azorimor`, and `SPDX-License-Identifier: MIT AND GPL-3.0-or-later` (`LICENSES/MIT.txt`).
- **New files rule:** Every newly added commentable file must include `SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>` and `SPDX-License-Identifier: GPL-3.0-or-later` headers (or be explicitly covered in `REUSE.toml` if non-commentable). Always verify with `reuse lint` and `cargo deny --locked --all-features check licenses`.

### Hardware & Firmware Context
- **Upgrade irreversibility:** Firmware upgrades are auto-committed by the controller even if verify and finalize steps are not executed.
- **Hardware scope:** Tested on standard DualSense controllers; **not** tested on DualSense Edge (`0044`). The GUI flags DualSense Edge as untested before flashing.
- **Firmware source:** Sony publishes version metadata at `https://fwupdater.dl.playstation.net/fwupdater/info.json` and images at `https://fwupdater.dl.playstation.net/fwupdater/fwupdate<target>/<version>/FWUPDATE<target>.bin`.
- **Observed board-to-target mappings (`KNOWN_TARGETS`):**
  - DualSense BDM-020 / BDM-030 / BDM-040 -> `0004`
  - DualSense BDM-050 -> `000B`
  - DualSense BDM-060 -> `000E`
  - DualSense Edge -> `0044`
- **Known device behavior:** A controller already running the target or newer firmware may fail past start-update because Sony's firmware blocks downgrades/re-flashing.

---

## 4. Current Architecture & File Map

```text
.
├── Cargo.toml                  # 3-member workspace manifest + pinned workspace dependencies
├── Cargo.lock                  # Resolved dependency lockfile
├── build.rs                    # Auto-sets QMAKE=/usr/bin/qmake6; builds cxx-qt, QML module, icon QResources, & generated_licenses.rs
├── deny.toml                   # cargo-deny license, advisory, ban, and source policy
├── rustfmt.toml                # Workspace Rust formatting configuration
├── clippy.toml                 # Workspace Clippy configuration (msrv = "1.85.0", complexity thresholds)
├── cliff.toml                  # git-cliff Conventional Commits changelog template
├── release-plz.toml            # release-plz automated changelog & GitHub release config
├── README.md                   # Linux prerequisites, build/run/test guide, Sony firmware notes, disclaimer
├── REUSE.toml                  # FSFE REUSE metadata for non-commentable and config assets
├── LICENSES/                   # Full SPDX license texts (GPL-3.0-or-later.txt, MIT.txt)
├── .githooks/                  # Enforced git hooks (pre-commit, commit-msg, pre-push)
├── .github/
│   ├── ISSUE_TEMPLATE/         # GitHub issue templates (`flash_report.md` prefill target; others live on main)
│   └── workflows/              # GitHub Actions workflows (Ubuntu 24.04)
│       ├── ci.yml              # Fast prechecks -> Clippy, test+coverage, tarball (Qt 6.4), & self-contained Qt 6.8 + Kirigami AppImage packaging/smoke check
│       ├── cleanup-pr-cache.yml # PR cache cleanup workflow
│       ├── release.yml         # Release & SHA256SUMS artifact publishing workflow
│       └── deploy-website.yml  # Fumadocs static site build & deploy workflow
├── .agents/                    # Antigravity lifecycle hooks
├── assets/                     # Brand SVGs (`logo.svg`, `logo-symbol.svg`), Vectorcraft source, & GUI screenshots (`assets/desktop/` -> `packaging/linux/icons/`, `assets/web/` -> `website/public/`)
├── packaging/                  # Linux desktop integration assets
│   └── linux/
│       ├── dev.dualsense.updater.desktop # XDG desktop entry (`dev.dualsense.updater`)
│       ├── install-desktop.sh  # User/system XDG desktop entry & hicolor icon installer (`--user`, `--system`, `--prefix`, `--exec`, `--uninstall`)
│       └── icons/              # Canonical app icons (`16x16`..`512x512/icon.png` + `scalable/icon.svg`; moved from `assets/desktop`)
├── scripts/
│   ├── coverage.sh             # cargo-llvm-cov HTML & LCOV report script
│   ├── package-appimage.sh     # Portable .tar.gz & .AppImage packager (`--tarball-only`/`--appimage-only`, Wayland/offscreen/SVG plugins, desktop entry, icons, `install-desktop.sh`)
│   └── verify_structural_rules.py # Checks file length, test layout, SPDX headers, & AGENTS.md paths
├── website/                    # Fumadocs / Next.js static documentation website
│   └── public/                 # Static web brand assets (`favicon.svg`, `favicon-16x16.png`, `favicon-32x32.png`, `favicon-48x48.png`, `icon-192x192.png`, `icon-512x512.png`, `apple-touch-icon.png`, `site.webmanifest`; moved from `assets/web`)
├── crates/
│   ├── dualsense-core/         # Core HID protocol, image validation, catalog client, update state machine
│   │   ├── src/
│   │   │   ├── lib.rs          # Public crate re-exports
│   │   │   ├── cancel.rs       # Thread-safe CancellationToken
│   │   │   ├── catalog.rs      # Sony info.json parser, KNOWN_TARGETS, HttpClient trait, FirmwareCatalogClient
│   │   │   ├── error.rs        # CoreError, UpdateFailure, UpdatePhase, per-phase error enums
│   │   │   ├── image.rs        # Single-read FirmwareImage loader and 0x78 version / 32 KiB block validation
│   │   │   ├── protocol.rs     # Report IDs (0x20, 0xF4, 0xF5), commands, status codes, chunk encoder
│   │   │   ├── transport.rs    # HidTransport, HidDeviceEnumerator, Sleeper traits + HidApiBackend
│   │   │   ├── updater.rs      # DualSenseUpdater bounded polling state machine & UpdateProgress
│   │   │   └── tests/          # Extracted unit tests (*_test.rs)
│   │   └── tests/              # Integration tests (board_detection_test.rs, update_flow_test.rs, opt-in real_firmware_test.rs)
│   └── dualsense-ui/           # Headless-testable ViewModel, HTTP client, friendly errors, async runtime
│       ├── src/
│       │   ├── lib.rs          # Public crate re-exports
│       │   ├── viewmodel/      # Modular Elm/TEA ViewModel (mod, types, state, transitions, snapshot, progress)
│       │   ├── controller.rs   # AppController & AppRuntime (Tokio workers + 800 ms live USB device polling)
│       │   ├── http.rs         # UreqHttpClient (ureq + rustls) with streaming progress & cancellation
│       │   ├── flash_report.rs # Anonymized OS/hardware/flash metrics & prefilled GitHub issue URL builder
│       │   ├── error_map.rs    # FriendlyError plain-English mapping for CoreError / UpdateFailure
│       │   └── tests/          # Extracted unit tests (*_test.rs)
│       └── tests/              # Integration tests (controller_flows_test.rs, disconnect_monitor_test.rs, http_live_test.rs)
├── src/                        # Root `dualsense-updater` Qt6/Kirigami desktop crate
│   ├── lib.rs                  # Exposes qt_bridge, app_icon, quick_style, snapshot_adapter, licenses, & license_report
│   ├── main.rs                 # QGuiApplication & QQmlApplicationEngine setup, desktop ID/window icon, runtime org.kde.desktop/Fusion style selection
│   ├── app_icon.rs             # cxx-qt FFI wrapper, `DESKTOP_FILE_ID`, `EMBEDDED_APP_ICON_URL`, & `qml_imports_path()`
│   ├── app_icon.h              # C++ header for `configure_application_window_icon()` & `qt_qml_imports_path()`
│   ├── app_icon.cpp            # Theme icon lookup (`dev.dualsense.updater`), embedded icon fallback, & `QLibraryInfo::QmlImportsPath` helper
│   ├── quick_style.rs          # Runtime `org.kde.desktop` QML module probe with automatic `Fusion` fallback & env override support
│   ├── qt_bridge.rs            # cxx-qt DualSenseBridge QObject binding DefaultAppRuntime & licensesText to QML
│   ├── snapshot_adapter.rs     # Pure Rust AdaptedSnapshot & file:// URL parser
│   ├── licenses.rs             # Bundled license text loader & query helpers
│   ├── license_report.rs       # Cargo.lock & crate metadata parser/formatter for third-party notices
│   └── tests/                  # Extracted unit tests (snapshot_adapter_test.rs, snapshot_state_test.rs, licenses_test.rs, quick_style_test.rs)
├── tests/                      # Root headless integration tests (snapshot_integration_test.rs, qml_bindings_test.rs, desktop_icon_test.rs)
└── qml/                        # Declarative KDE Kirigami QML views (dev.dualsense.updater module)
    ├── Main.qml                # Kirigami.ApplicationWindow + MenuBar + 6-view StackLayout wizard
    ├── StepIndicator.qml       # 4-step progress breadcrumb header
    ├── LicensesWindow.qml      # Third-party open-source licenses viewer window
    ├── AboutWindow.qml         # About dialog with version, disclaimer, and GitHub / Pages links
    └── FlashReportWindow.qml   # Optional post-flash anonymized report preview, copy, & GitHub issue opener
```

### Workspace Dependencies (`Cargo.toml`)
- **Core & Async:** `hidapi`, `thiserror`, `tokio`, `ureq` (with `rustls`), `log`, `env_logger`.
- **Qt / QML Bridge:** `cxx`, `cxx-qt`, `cxx-qt-build`, `cxx-qt-lib` (`qt_gui`, `qt_qml`, `qt_quickcontrols`).

### Layer Responsibilities
- **`dualsense-core`:** Zero UI/Qt dependencies. Encapsulates USB HID feature reports (`0x20`, `0xF4`, `0xF5`), `FirmwareImage` (read once into memory), `DualSenseUpdater` (bounded polling via `UpdateConfig` + `Sleeper` + `CancellationToken`), and Sony catalog/download logic behind `HidTransport`, `HidDeviceEnumerator`, and `HttpClient`.
- **`dualsense-ui`:** Pure Rust Elm/TEA state machine (`AppState::update` -> `Vec<AppCommand>`) and `AppController` / `AppRuntime` executing HID, disk, HTTPS, and live 800 ms USB disconnect/replug polling tasks (`DevicesPollUpdated`, skipped during active flashing) on a Tokio runtime, emitting `UiSnapshot` updates and plain-English `FriendlyError` guidance.
- **Root `dualsense-updater` (`src/` + `qml/`):** `DualSenseBridge` (`#[cxx_qt::bridge]`) marshals `UiSnapshot` via `AdaptedSnapshot` onto the Qt thread (`cxx_qt::Threading`). QML stays thin and declarative (`pragma ComponentBehavior: Bound`) over `updater` properties and `#[qinvokable]` slots. All unit and integration tests live in `src/tests/` and `tests/` across all three crates.

---

## 5. Current Technical Notes

- **Qt6 / `qmake6` discovery:** `build.rs` defaults `QMAKE` to `/usr/bin/qmake6` when unset and present; export `QMAKE=/path/to/qmake6` on systems where `qmake` defaults to Qt 5 or `qmake6` is elsewhere.
- **Headless CI compatibility:** All 3 workspace crates decouple business logic and snapshot adaptation from Qt event loops, display servers, and physical DualSense hardware so `cargo test --workspace` runs completely headlessly.
- **Kirigami style & QML linting:** `src/main.rs` + `src/quick_style.rs` select `org.kde.desktop` when present in QML import paths and fall back to `Fusion` otherwise (unless `QT_QUICK_CONTROLS_STYLE` is set). Keep QML strictly typed (`pragma ComponentBehavior: Bound`, `required property` on delegates) so `qmllint` passes with zero warnings.
- **Linux system dev libraries:** `libhidapi-dev`, `libudev-dev`, `libusb-1.0-0-dev`, `libgl-dev`, `libxkbcommon-dev`, `pkg-config`, and Qt 6 dev packages.
- **Offscreen smoke check:** Running with `QT_QPA_PLATFORM=offscreen` verifies `QQmlApplicationEngine` loads `qrc:/qt/qml/dev/dualsense/updater/qml/Main.qml` cleanly without X11/Wayland (using `timeout` because the Qt event loop stays open until closed).

---

## 6. Build & Verification Commands

Linux builds use the `mold` linker via `.cargo/config.toml` (`-C link-arg=-fuse-ld=mold`).

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/verify_structural_rules.py
cargo test --workspace
reuse lint
cargo deny --locked --all-features check licenses
./scripts/coverage.sh  # requires cargo-llvm-cov + llvm-tools; writes target/coverage/{lcov.info,html/index.html}
cargo check
/usr/lib/qt6/bin/qmllint -I target/cxxqt/qml_modules -I /usr/lib/qt6/qml qml/StepIndicator.qml qml/Main.qml
cargo build --release
QT_QPA_PLATFORM=offscreen timeout 3s ./target/release/dualsense-updater  # expect exit 124 (clean startup)
```

Note: Update `AGENTS.md` (and `README.md` if user-facing behavior changes) as part of your incremental commits whenever the architecture changes significantly.
