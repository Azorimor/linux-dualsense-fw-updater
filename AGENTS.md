# Coding Agent Instructions

This file governs all AI coding agents working in `dualsense-updater-rs`.

> **Context for Future Work:** The goal for future agents is to **almost completely redesign and rewrite this project**. Read the workflow rules and current codebase documentation below before making any changes.

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

`dualsense-updater` (`v0.1.0`, Rust `edition = "2024"`, MIT license) is an unofficial CLI utility for inspecting and updating Sony PlayStation DualSense controller firmware over USB HID.

### Hardware & Firmware Context (from `README.md`)
- **Upgrade irreversibility:** Firmware upgrades are auto-committed by the controller even if verify and finalize steps are not executed.
- **Hardware scope:** Tested on standard DualSense controllers; **not** tested on DualSense Edge.
- **Firmware source:** Sony publishes version metadata at `https://fwupdater.dl.playstation.net/fwupdater/info.json` and images at `https://fwupdater.dl.playstation.net/fwupdater/fwupdate<target>/<version>/FWUPDATE<target>.bin`.
- **Observed board-to-target mappings:**
  - DualSense BDM-020 / BDM-030 / BDM-040 -> `0004`
  - DualSense BDM-050 -> `000B`
  - DualSense BDM-060 -> `000E`
  - DualSense Edge -> `0044`
- **Known device behavior:** A controller already running the target/latest firmware may not return success codes past `--start-update`.

---

## 4. Current Architecture & File Map

```text
.
├── Cargo.toml        # Package manifest (edition 2024; wildcard "*" deps)
├── Cargo.lock        # Resolved dependency lockfile
├── README.md         # Build, CLI usage, firmware download URLs, disclaimer
├── LICENSE           # MIT license
├── .githooks/        # Enforced git hooks (pre-commit, commit-msg, pre-push)
├── .agents/          # Antigravity lifecycle hooks
└── src/
    ├── main.rs       # Entry point, logging init, interactive & flag modes, error formatting
    ├── cli.rs        # clap Args definition and hex/decimal VID/PID parsing
    ├── hid.rs        # hidapi wrapper (DualSenseHid) and HID feature report I/O
    ├── protocol.rs   # HID report IDs, FirmwareInfo, commands, and status code enums
    ├── update.rs     # DualSenseUpdater update state machine & polling loops
    └── error.rs      # thiserror error types for app/HID/protocol failures
```

### Dependencies (`Cargo.toml`)
- `clap` (`derive` feature) — CLI parsing
- `hidapi` — USB HID device enumeration and feature report I/O
- `log` & `env_logger` — logging (`Info` default, `Debug` with `-v`/`--verbose`)
- `thiserror` — typed error enums

### Module Details
- **`src/cli.rs`:**
  - `DEFAULT_VID = 0x054c`, `DEFAULT_PID = 0x0ce6`.
  - `Args`: `--vid`, `--pid` (`parse_u16` accepts `0x`-prefixed hex or decimal), positional `FW_IMAGE`, `--path`, `-v`/`--verbose`, `--print-firmware-info`, `--start-update-only`, `--write-update-image-only`, `--verify-update-image-only`, `--finalize-update-only`.
- **`src/main.rs`:**
  - Prints help and exits `0` when called with no arguments or on CLI parse errors (except `--help`/`--version`).
  - **Interactive mode** (when `FW_IMAGE` is given with no action flags): opens the first matching device (`find_first_device_path`), prints current firmware version (`0x20`), reads target version from `FW_IMAGE` at offset `0x78`, prompts `[y/N]`, and executes `start_update` -> `write_update_image` -> `verify_update_image` -> `finalize_update`.
  - **Flag mode**: opens `--path` if provided or first matching VID/PID device, then runs enabled flags in fixed order (`print_firmware_info`, `start_update`, `write_update_image`, `verify_update_image`, `finalize_update`).
  - `format_error` translates `AppError` / `UpdateFailure` variants into human-readable messages.
- **`src/hid.rs`:**
  - `DualSenseHid` wraps `HidApi` and `HidDevice`.
  - `get_firmware_info`: reads Feature Report `0x20` (64 bytes), decodes ASCII `build_date` (`payload[..12]`), `build_time` (`payload[12..20]`), and LE `u16` `firmware_version` (`payload[44..=45]`).
  - `send_update_command`: sends payload in max `0x39` (57)-byte chunks as Feature Report `[0xF4, command, chunk_len, ...chunk]`.
  - `get_update_status`: reads 4-byte Feature Report `0xF5` `[report_id, command, status_raw, ...]`.
- **`src/protocol.rs`:**
  - Report IDs: `REPORT_ID_FIRMWARE_INFO = 0x20`, `REPORT_ID_UPDATE_COMMAND = 0xF4`, `REPORT_ID_UPDATE_STATUS = 0xF5`.
  - `UpdateCommand`: `StartUpdate (0x00)`, `WriteUpdateImage (0x01)`, `VerifyUpdateImage (0x02)`, `FinalizeUpdate (0x03)`, `Unknown (0xFF)`.
  - Status enums: `StartUpdateStatusCode`, `WriteUpdateStatusCode`, `VerifyUpdateStatusCode`.
- **`src/update.rs`:**
  - `firmware_version_from_image`: reads LE `u16` at byte offset `0x78`.
  - `start_update`: reads image (`>= 256` bytes), sends first 256 bytes via `StartUpdate`, polls `0xF5` every `10 ms` while `Processing (0x04)`.
  - `write_update_image`: splits the full image into `0x8000` (32 KiB) blocks, sending each block in `0x39`-byte `WriteUpdateImage` chunks and polling `0xF5` every `10 ms` on `Retry (0x01)` / `AlsoRetry (0x10)` until `SendNext (0x03)` or `Success (0x00)`.
  - `verify_update_image`: sends empty `VerifyUpdateImage`, polls `0xF5` every `10 ms` while `KeepPolling (0x10)`.
  - `finalize_update`: sends empty `FinalizeUpdate` with no status polling.
- **`src/error.rs`:**
  - Defines `AppError`, `UpdateFailure`, `StartUpdateError`, `WriteUpdateImageError`, `VerifyUpdateImageError`, and `FinalizeUpdateError`.

---

## 5. Current Technical Debt & Observations for the Rewrite

Future agents replacing or refactoring this codebase should keep these concrete gaps in mind:
- **No tests or CI:** `cargo test` currently has 0 tests, and `DualSenseHid` talks directly to `hidapi::HidDevice` with no transport abstraction for unit testing.
- **Wildcard dependencies:** `Cargo.toml` specifies `"*"` for all crates.
- **Repeated file reads:** `firmware_version_from_image`, `start_update`, and `write_update_image` each re-read `FW_IMAGE` from disk separately.
- **Interactive `--path` omission:** Interactive mode in `src/main.rs` calls `find_first_device_path` and ignores `args.path`.
- **Unbounded polling loops:** `start_update`, `write_update_image`, and `verify_update_image` in `src/update.rs` poll infinitely without a timeout or retry limit.
- **Mixed concerns & dead code:** `src/update.rs` prints directly to stdout via `println!`; `protocol.rs` and `error.rs` have `#[allow(dead_code)]` items (`FirmwareInfo::{unknown, raw}`, `UpdateStatus::{report_id, raw}`, `FinalizeUpdateError`).
- **CLI exit codes:** Invalid CLI arguments in `src/main.rs` currently print help and exit with code `0` instead of a non-zero exit code.

---

## 6. Build & Verification Commands

```sh
cargo check
cargo test
cargo fmt --check
cargo build --release
```

Note: Update `AGENTS.md` (and `README.md` if user-facing behavior changes) as part of your incremental commits whenever the architecture changes significantly.
