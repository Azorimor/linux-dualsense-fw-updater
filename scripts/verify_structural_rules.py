#!/usr/bin/env python3

# SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
#
# SPDX-License-Identifier: GPL-3.0-or-later

"""Verify structural, test-layout, licensing, and AGENTS.md path rules."""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MAX_LINES = 1000

SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    ".next",
    "out",
    ".source",
    "AppDir",
    "build",
}

ADAPTED_CORE_FILES = [
    "crates/dualsense-core/src/error.rs",
    "crates/dualsense-core/src/image.rs",
    "crates/dualsense-core/src/protocol.rs",
    "crates/dualsense-core/src/transport.rs",
    "crates/dualsense-core/src/updater.rs",
    "crates/dualsense-core/src/tests/error_test.rs",
    "crates/dualsense-core/src/tests/image_test.rs",
    "crates/dualsense-core/src/tests/protocol_test.rs",
    "crates/dualsense-core/src/tests/transport_test.rs",
    "crates/dualsense-core/src/tests/updater_test.rs",
]

CRATE_TEST_DIRS = [
    pathlib.Path("."),
    pathlib.Path("crates/dualsense-core"),
    pathlib.Path("crates/dualsense-ui"),
]

AGENTS_KEY_PATHS = [
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "REUSE.toml",
    "README.md",
    "LICENSES/GPL-3.0-or-later.txt",
    "LICENSES/MIT.txt",
    ".githooks",
    ".github/workflows/ci.yml",
    ".agents",
    "crates/dualsense-core/src/lib.rs",
    "crates/dualsense-core/src/cancel.rs",
    "crates/dualsense-core/src/catalog.rs",
    "crates/dualsense-core/src/error.rs",
    "crates/dualsense-core/src/image.rs",
    "crates/dualsense-core/src/protocol.rs",
    "crates/dualsense-core/src/transport.rs",
    "crates/dualsense-core/src/updater.rs",
    "crates/dualsense-core/src/tests",
    "crates/dualsense-core/tests/board_detection_test.rs",
    "crates/dualsense-core/tests/update_flow_test.rs",
    "crates/dualsense-ui/src/lib.rs",
    "crates/dualsense-ui/src/viewmodel",
    "crates/dualsense-ui/src/controller.rs",
    "crates/dualsense-ui/src/http.rs",
    "crates/dualsense-ui/src/error_map.rs",
    "crates/dualsense-ui/src/tests",
    "crates/dualsense-ui/tests/controller_flows_test.rs",
    "crates/dualsense-ui/tests/disconnect_monitor_test.rs",
    "crates/dualsense-ui/tests/http_live_test.rs",
    "src/lib.rs",
    "src/main.rs",
    "src/qt_bridge.rs",
    "src/snapshot_adapter.rs",
    "src/tests/snapshot_adapter_test.rs",
    "src/tests/snapshot_state_test.rs",
    "tests/snapshot_integration_test.rs",
    "tests/qml_bindings_test.rs",
    "qml/Main.qml",
    "qml/StepIndicator.qml",
]

CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
TEST_ATTR_RE = re.compile(r"^\s*#\s*\[\s*(?:tokio::)?test\b", re.MULTILINE)
EXTERNAL_MOD_TESTS_RE = re.compile(
    r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*(?:pub\s+)?mod\s+tests\s*;"
)


def iter_source_files() -> list[pathlib.Path]:
    """Return all tracked/workspace .rs and .qml files excluding generated dirs."""
    matched: list[pathlib.Path] = []
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file():
            continue
        if any(part in SKIP_DIRS for part in path.relative_to(ROOT).parts):
            continue
        if path.suffix in {".rs", ".qml"}:
            matched.append(path)
    return matched


def is_in_test_dir(rel_path: pathlib.Path) -> bool:
    """Return True if path lives inside src/tests/ or tests/."""
    parts = rel_path.parts
    if "tests" in parts:
        idx = parts.index("tests")
        # Either <crate>/tests/... or <crate>/src/tests/...
        return idx < len(parts) - 1
    return False


def check_line_limits_and_inline_tests(files: list[pathlib.Path]) -> list[str]:
    """Ensure all .rs/.qml files <= 1000 lines and no inline #[cfg(test)] in prod files."""
    errors: list[str] = []
    for path in files:
        rel = path.relative_to(ROOT)
        text = path.read_text(encoding="utf-8")
        line_count = len(text.splitlines())
        if line_count > MAX_LINES:
            errors.append(f"line limit exceeded ({line_count} > {MAX_LINES}): {rel}")

        if path.suffix == ".rs" and not is_in_test_dir(rel):
            # Allow `#[cfg(test)] mod tests;` in crate roots / module wiring,
            # but reject any inline test blocks or #[test] functions.
            stripped = EXTERNAL_MOD_TESTS_RE.sub("", text)
            if CFG_TEST_RE.search(stripped):
                errors.append(
                    f"inline #[cfg(test)] found outside src/tests/ or tests/: {rel}"
                )
            if TEST_ATTR_RE.search(text):
                errors.append(
                    f"inline #[test] found outside src/tests/ or tests/: {rel}"
                )
    return errors


def check_test_directories() -> list[str]:
    """Ensure every crate's src/tests/ and tests/ has at least two *_test.rs files."""
    errors: list[str] = []
    for crate_dir in CRATE_TEST_DIRS:
        for sub in ("src/tests", "tests"):
            test_dir = ROOT / crate_dir / sub
            rel_dir = test_dir.relative_to(ROOT)
            if not test_dir.is_dir():
                errors.append(f"missing required test directory: {rel_dir}")
                continue
            test_files = sorted(test_dir.glob("*_test.rs"))
            if len(test_files) < 2:
                errors.append(
                    f"expected >= 2 *_test.rs files in {rel_dir}, found {len(test_files)}"
                )
    return errors


def check_spdx_headers() -> list[str]:
    """Ensure adapted dualsense-core files and REUSE license files retain required headers."""
    errors: list[str] = []
    for rel_str in ("LICENSES/GPL-3.0-or-later.txt", "LICENSES/MIT.txt", "REUSE.toml"):
        if not (ROOT / rel_str).is_file():
            errors.append(f"missing required REUSE file: {rel_str}")

    # REUSE-IgnoreStart
    required_license_line = "SPDX-License-Identifier: MIT AND GPL-3.0-or-later"
    # REUSE-IgnoreEnd

    for rel_str in ADAPTED_CORE_FILES:
        path = ROOT / rel_str
        if not path.is_file():
            errors.append(f"missing adapted dualsense-core file: {rel_str}")
            continue
        head = "\n".join(path.read_text(encoding="utf-8").splitlines()[:10])
        if "SPDX-FileCopyrightText: 2026 nchie" not in head:
            errors.append(f"missing nchie SPDX-FileCopyrightText header in {rel_str}")
        if "SPDX-FileCopyrightText: 2026 Azorimor" not in head:
            errors.append(f"missing Azorimor SPDX-FileCopyrightText header in {rel_str}")
        if required_license_line not in head:
            errors.append(f"missing '{required_license_line}' header in {rel_str}")
    return errors


def check_agents_paths() -> list[str]:
    """Ensure AGENTS.md exists and all documented key paths exist."""
    errors: list[str] = []
    agents_md = ROOT / "AGENTS.md"
    if not agents_md.is_file():
        return ["missing AGENTS.md"]
    agents_text = agents_md.read_text(encoding="utf-8")
    for rel_str in AGENTS_KEY_PATHS:
        target = ROOT / rel_str
        if not target.exists():
            errors.append(f"AGENTS.md key path missing on disk: {rel_str}")
        leaf = pathlib.Path(rel_str).name
        if leaf not in agents_text and rel_str not in agents_text:
            errors.append(f"key path not referenced in AGENTS.md: {rel_str}")
    return errors


def main() -> int:
    source_files = iter_source_files()
    errors: list[str] = []
    errors.extend(check_line_limits_and_inline_tests(source_files))
    errors.extend(check_test_directories())
    errors.extend(check_spdx_headers())
    errors.extend(check_agents_paths())

    if errors:
        print("Structural verification FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        f"Structural verification PASSED "
        f"({len(source_files)} .rs/.qml files checked, "
        f"{len(ADAPTED_CORE_FILES)} SPDX files, "
        f"{len(AGENTS_KEY_PATHS)} AGENTS.md paths)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
