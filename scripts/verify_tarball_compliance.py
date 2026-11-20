#!/usr/bin/env python3

# SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
#
# SPDX-License-Identifier: GPL-3.0-or-later

"""Verify license compliance, contents, ELF arch, and dynamic links of a portable .tar.gz release."""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys
import tarfile
import tempfile

APP_ID = "io.github.azorimor.linux_dualsense_fw_updater"

ICON_SIZES = (
    "16x16",
    "22x22",
    "32x32",
    "48x48",
    "64x64",
    "128x128",
    "256x256",
    "512x512",
)

REQUIRED_LICENSES = (
    "GPL-3.0-or-later.txt",
    "MIT.txt",
    "Apache-2.0.txt",
    "BSD-3-Clause.txt",
    "LGPL-3.0-only.txt",
    "LGPL-2.1-or-later.txt",
    "LGPL-2.0-or-later.txt",
)

COMMON_ALLOWED_HOST_DT_NEEDED = {
    "libc.so.6",
    "libm.so.6",
    "libdl.so.2",
    "libpthread.so.0",
    "librt.so.1",
    "libgcc_s.so.1",
    "libstdc++.so.6",
    "libudev.so.1",
    "libQt6Core.so.6",
    "libQt6Gui.so.6",
    "libQt6Network.so.6",
    "libQt6Qml.so.6",
    "libQt6Quick.so.6",
    "libQt6QuickControls2.so.6",
    "libQt6Widgets.so.6",
}

ARCH_DYNAMIC_LINKERS = {
    "x86_64": "ld-linux-x86-64.so.2",
    "aarch64": "ld-linux-aarch64.so.1",
}

ARCH_ELF_MACHINE_MARKERS = {
    "x86_64": "X86-64",
    "aarch64": "AArch64",
}

TARBALL_NAME_RE = re.compile(
    r"^(linux-dualsense-fw-updater-.+-(x86_64|aarch64))\.tar\.gz$"
)

LICENSE_START_MARKER = b"DualSense Firmware Updater (v"
LICENSE_END_MARKER = b"=== END OF THIRD-PARTY LICENSES ==="


def parse_tarball_arch(tarball_name: str) -> tuple[str, str]:
    """Return `(expected_top_dir, arch)` from a release tarball filename."""
    match = TARBALL_NAME_RE.match(tarball_name)
    if not match:
        raise ValueError(
            f"unexpected tarball filename (expected linux-dualsense-fw-updater-<tag>-<x86_64|aarch64>.tar.gz): {tarball_name}"
        )
    return match.group(1), match.group(2)


def validate_elf_machine(readelf_header: str, arch: str) -> list[str]:
    """Validate that `readelf -h` reports the ELF machine matching `arch`."""
    expected = ARCH_ELF_MACHINE_MARKERS.get(arch)
    if expected is None:
        return [f"unsupported archive architecture: {arch}"]

    machine_match = re.search(r"(?m)^\s*Machine:\s*(.+)$", readelf_header)
    if not machine_match:
        return ["could not parse ELF Machine field from readelf -h output"]

    machine_value = machine_match.group(1).strip()
    if expected not in machine_value:
        return [
            f"ELF machine mismatch for {arch} archive: expected '{expected}', found '{machine_value}'"
        ]
    return []


def validate_dt_needed(dt_needed: list[str], arch: str) -> list[str]:
    """Validate that `DT_NEEDED` libraries only reference expected host libraries for `arch`."""
    errors: list[str] = []
    if not dt_needed:
        return ["no DT_NEEDED entries found in dualsense-updater binary"]

    expected_linker = ARCH_DYNAMIC_LINKERS.get(arch)
    if expected_linker is None:
        return [f"unsupported archive architecture: {arch}"]

    allowed = set(COMMON_ALLOWED_HOST_DT_NEEDED)
    allowed.add(expected_linker)

    unexpected_needed = [lib for lib in dt_needed if lib not in allowed]
    if unexpected_needed:
        errors.append(
            f"unexpected DT_NEEDED shared libraries for {arch} release binary: {unexpected_needed}"
        )
    if "libudev.so.1" not in dt_needed:
        errors.append("expected libudev.so.1 in DT_NEEDED (static hidapi-hidraw backend)")
    if any("hidapi" in lib for lib in dt_needed):
        errors.append(f"unexpected dynamic hidapi dependency in DT_NEEDED: {dt_needed}")
    return errors


def extract_embedded_licenses_from_bytes(data: bytes) -> str:
    """Slice and decode the exact embedded BUNDLED_LICENSES_TEXT payload using explicit sentinels."""
    idx = data.find(LICENSE_START_MARKER)
    if idx == -1:
        raise ValueError("embedded BUNDLED_LICENSES_TEXT start marker not found in binary")

    end_idx = data.find(LICENSE_END_MARKER, idx)
    if end_idx == -1:
        raise ValueError(
            "embedded BUNDLED_LICENSES_TEXT end marker not found in binary"
        )

    payload = data[idx : end_idx + len(LICENSE_END_MARKER)]
    return payload.decode("utf-8", errors="strict")


def extract_embedded_licenses(binary_path: pathlib.Path) -> str:
    """Read `binary_path` and decode the embedded BUNDLED_LICENSES_TEXT string."""
    return extract_embedded_licenses_from_bytes(binary_path.read_bytes())


def verify_tarball(tarball_path: pathlib.Path) -> list[str]:
    """Run all compliance, layout, ELF arch, DT_NEEDED, and embedded-notice assertions."""
    errors: list[str] = []

    if not tarball_path.is_file():
        return [f"tarball does not exist: {tarball_path}"]

    try:
        expected_top_dir, arch = parse_tarball_arch(tarball_path.name)
    except ValueError as exc:
        return [str(exc)]

    with tempfile.TemporaryDirectory(prefix="verify-tarball-") as tmp:
        tmp_dir = pathlib.Path(tmp)
        with tarfile.open(tarball_path, "r:gz") as tf:
            members = tf.getmembers()
            tf.extractall(tmp_dir, filter="data")

        top_dirs = sorted(
            {
                m.name.split("/", 1)[0]
                for m in members
                if m.name and not m.name.startswith("/")
            }
        )
        if top_dirs != [expected_top_dir]:
            return [
                f"expected single top-level directory {expected_top_dir!r} in tarball, found: {top_dirs}"
            ]

        root_dir = tmp_dir / top_dirs[0]

        # 1. Verify expected archive layout and absence of bundled shared objects.
        required_paths = [
            "dualsense-updater",
            "THIRD_PARTY_NOTICES.txt",
            "README.md",
            f"{APP_ID}.desktop",
            "install-desktop.sh",
            "icons/scalable/icon.svg",
        ]
        required_paths.extend(f"LICENSES/{lic}" for lic in REQUIRED_LICENSES)
        required_paths.extend(f"icons/{size}/icon.png" for size in ICON_SIZES)

        for rel in required_paths:
            if not (root_dir / rel).is_file():
                errors.append(f"missing expected tarball file: {rel}")

        bundled_sos = sorted(
            str(p.relative_to(root_dir))
            for p in root_dir.rglob("*")
            if (p.is_file() or p.is_symlink())
            and re.search(r"\.so(\.\d+)*$", p.name)
        )
        if bundled_sos:
            errors.append(
                f"portable tarball must not bundle shared libraries, found: {bundled_sos}"
            )

        binary_path = root_dir / "dualsense-updater"
        if not binary_path.is_file():
            return errors

        # 2. Verify ELF header matches the tarball architecture.
        try:
            readelf_hdr = subprocess.check_output(
                ["readelf", "-h", str(binary_path)], text=True
            )
            errors.extend(validate_elf_machine(readelf_hdr, arch))
        except (OSError, subprocess.CalledProcessError) as exc:
            errors.append(f"failed to inspect ELF header with readelf: {exc}")

        # 3. Verify DT_NEEDED entries only reference expected host system libraries.
        try:
            readelf_out = subprocess.check_output(
                ["readelf", "-d", str(binary_path)], text=True
            )
        except (OSError, subprocess.CalledProcessError) as exc:
            errors.append(f"failed to inspect DT_NEEDED with readelf: {exc}")
            readelf_out = ""

        dt_needed = sorted(
            re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", readelf_out)
        )
        errors.extend(validate_dt_needed(dt_needed, arch))

        # 4. Verify embedded BUNDLED_LICENSES_TEXT inside the compiled release binary.
        try:
            embedded = extract_embedded_licenses(binary_path)
        except ValueError as exc:
            errors.append(str(exc))
            return errors

        forbidden_markers = [
            "Development build shows crate metadata summary.",
            "See license file / unspecified",
            "(No LICENSE/COPYING/NOTICE file bundled in crate archive.)",
            "windows-sys",
            "Qt ShaderTools",
            "qt-everywhere-src-6.8.3",
            "kirigami-6.10.0",
        ]
        for marker in forbidden_markers:
            if marker in embedded:
                errors.append(f"forbidden text found in embedded licenses: {marker!r}")

        required_embedded_markers = [
            "GNU GENERAL PUBLIC LICENSE",
            "MIT License",
            "--- etc/hidapi/LICENSE-bsd.txt ---",
            "--- etc/hidapi/LICENSE-gpl3.txt ---",
            "--- etc/hidapi/LICENSE-orig.txt ---",
            "Alan Ott",
            "Klarälvdalens Datakonsult AB",
            "* cxx-qt v",
            "* cxx-qt-lib v",
            "--- LICENSE-MIT (fallback notice) ---",
            "--- LICENSE-APACHE (fallback notice) ---",
            "--- third_party/fiat/LICENSE ---",
            "--- src/polyfill/once_cell/LICENSE-MIT ---",
            "--- src/unicode_tables/LICENSE-UNICODE ---",
            "--- COPYRIGHT ---",
            "--- UNLICENSE ---",
            "=== END OF THIRD-PARTY LICENSES ===",
        ]
        for marker in required_embedded_markers:
            if marker not in embedded:
                errors.append(f"missing required text in embedded licenses: {marker!r}")

        crate_count = sum(
            1
            for line in embedded.splitlines()
            if line.startswith("* ") and " — " in line
        )
        if crate_count == 0:
            errors.append("no third-party crate entries found in embedded licenses")

        if not errors:
            print(
                f"Tarball compliance verification PASSED ({tarball_path.name} [{arch}]: "
                f"{len(dt_needed)} host DT_NEEDED libs, "
                f"{crate_count} embedded Rust crates, "
                f"{len(embedded)} bytes embedded notices)."
            )

    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"Usage: {argv[0]} <tarball.tar.gz>", file=sys.stderr)
        return 2

    errors = verify_tarball(pathlib.Path(argv[1]))
    if errors:
        print("Tarball compliance verification FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
