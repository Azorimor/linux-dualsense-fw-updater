#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
#
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

STAGE_ONLY=0
if [[ "${1:-}" == "--stage-only" ]]; then
    STAGE_ONLY=1
    shift
fi

if [[ $# -ne 3 ]]; then
    echo "Usage: $0 [--stage-only] <binary-path> <tag> <out-dir>" >&2
    exit 2
fi

BINARY_PATH="$1"
TAG="$2"
OUT_DIR="$3"

APP_ID="dev.dualsense.updater"
ICON_SIZES=(16x16 22x22 32x32 48x48 64x64 128x128 256x256 512x512)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_FILE="${REPO_ROOT}/packaging/linux/${APP_ID}.desktop"
INSTALL_SCRIPT="${REPO_ROOT}/packaging/linux/install-desktop.sh"
ICONS_DIR="${REPO_ROOT}/packaging/linux/icons"
SVG_ICON_FILE="${ICONS_DIR}/scalable/icon.svg"
DIRICON_SOURCE="${ICONS_DIR}/256x256/icon.png"
GPL_LICENSE_FILE="${REPO_ROOT}/LICENSES/GPL-3.0-or-later.txt"
MIT_LICENSE_FILE="${REPO_ROOT}/LICENSES/MIT.txt"
README_FILE="${REPO_ROOT}/README.md"

if [[ ! -x "${BINARY_PATH}" ]]; then
    echo "error: binary not found or not executable: ${BINARY_PATH}" >&2
    exit 1
fi
for required in \
    "${DESKTOP_FILE}" \
    "${INSTALL_SCRIPT}" \
    "${SVG_ICON_FILE}" \
    "${GPL_LICENSE_FILE}" \
    "${MIT_LICENSE_FILE}"
do
    if [[ ! -f "${required}" ]]; then
        echo "error: required packaging asset missing: ${required}" >&2
        exit 1
    fi
done
for size in "${ICON_SIZES[@]}"; do
    if [[ ! -f "${ICONS_DIR}/${size}/icon.png" ]]; then
        echo "error: required icon size missing: ${ICONS_DIR}/${size}/icon.png" >&2
        exit 1
    fi
done

mkdir -p "${OUT_DIR}"
OUT_DIR_ABS="$(cd "${OUT_DIR}" && pwd)"
BINARY_ABS="$(cd "$(dirname "${BINARY_PATH}")" && pwd)/$(basename "${BINARY_PATH}")"

APP_NAME="linux-dualsense-fw-updater"
STAGE_ROOT="$(mktemp -d)"
trap 'rm -rf "${STAGE_ROOT}"' EXIT

# 1. Build portable .tar.gz release archive
TAR_STAGE="${STAGE_ROOT}/${APP_NAME}-${TAG}-x86_64"
mkdir -p "${TAR_STAGE}"
install -Dm755 "${BINARY_ABS}" "${TAR_STAGE}/dualsense-updater"
install -Dm644 "${GPL_LICENSE_FILE}" "${TAR_STAGE}/LICENSES/GPL-3.0-or-later.txt"
install -Dm644 "${MIT_LICENSE_FILE}" "${TAR_STAGE}/LICENSES/MIT.txt"
if [[ -f "${README_FILE}" ]]; then
    install -Dm644 "${README_FILE}" "${TAR_STAGE}/README.md"
fi
install -Dm644 "${DESKTOP_FILE}" "${TAR_STAGE}/${APP_ID}.desktop"
install -Dm755 "${INSTALL_SCRIPT}" "${TAR_STAGE}/install-desktop.sh"
for size in "${ICON_SIZES[@]}"; do
    install -Dm644 "${ICONS_DIR}/${size}/icon.png" "${TAR_STAGE}/icons/${size}/icon.png"
done
install -Dm644 "${SVG_ICON_FILE}" "${TAR_STAGE}/icons/scalable/icon.svg"

TAR_ARCHIVE="${OUT_DIR_ABS}/${APP_NAME}-${TAG}-x86_64.tar.gz"
tar -C "${STAGE_ROOT}" -czf "${TAR_ARCHIVE}" "${APP_NAME}-${TAG}-x86_64"

# 2. Stage AppDir for linuxdeploy + linuxdeploy-plugin-qt
APPDIR="${STAGE_ROOT}/AppDir"
mkdir -p "${APPDIR}/usr/bin"
install -Dm755 "${BINARY_ABS}" "${APPDIR}/usr/bin/dualsense-updater"
install -Dm644 "${DESKTOP_FILE}" "${APPDIR}/usr/share/applications/${APP_ID}.desktop"
for size in "${ICON_SIZES[@]}"; do
    install -Dm644 "${ICONS_DIR}/${size}/icon.png" \
        "${APPDIR}/usr/share/icons/hicolor/${size}/apps/${APP_ID}.png"
done
install -Dm644 "${SVG_ICON_FILE}" \
    "${APPDIR}/usr/share/icons/hicolor/scalable/apps/${APP_ID}.svg"
install -Dm644 "${DIRICON_SOURCE}" "${APPDIR}/.DirIcon"
install -Dm644 "${GPL_LICENSE_FILE}" "${APPDIR}/usr/share/licenses/${APP_NAME}/GPL-3.0-or-later.txt"
install -Dm644 "${MIT_LICENSE_FILE}" "${APPDIR}/usr/share/licenses/${APP_NAME}/MIT.txt"

if [[ "${STAGE_ONLY}" -eq 1 ]]; then
    APPDIR_STAGE_OUT="${OUT_DIR_ABS}/AppDir"
    rm -rf "${APPDIR_STAGE_OUT}"
    cp -a "${APPDIR}" "${APPDIR_STAGE_OUT}"
    echo "Staged release artifacts (dry run):"
    echo "  Tarball: ${TAR_ARCHIVE}"
    echo "  AppDir:  ${APPDIR_STAGE_OUT}"
    exit 0
fi

TOOLS_DIR="${LINUXDEPLOY_CACHE_DIR:-${REPO_ROOT}/target/linuxdeploy}"
mkdir -p "${TOOLS_DIR}"

resolve_tool() {
    local env_path="$1"
    local cmd_name="$2"
    local url="$3"
    local target_file="${TOOLS_DIR}/${cmd_name}"

    if [[ -n "${env_path}" ]] && [[ -x "${env_path}" ]]; then
        echo "${env_path}"
        return 0
    fi
    if command -v "${cmd_name}" >/dev/null 2>&1; then
        command -v "${cmd_name}"
        return 0
    fi
    if [[ ! -x "${target_file}" ]]; then
        echo "Downloading ${cmd_name}..." >&2
        curl -fsSL "${url}" -o "${target_file}"
        chmod +x "${target_file}"
    fi
    echo "${target_file}"
}

LINUXDEPLOY_BIN="$(resolve_tool \
    "${LINUXDEPLOY:-}" \
    "linuxdeploy-x86_64.AppImage" \
    "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage")"

LINUXDEPLOY_QT_BIN="$(resolve_tool \
    "${LINUXDEPLOY_PLUGIN_QT:-}" \
    "linuxdeploy-plugin-qt-x86_64.AppImage" \
    "https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-x86_64.AppImage")"

# Ensure linuxdeploy discovers the Qt plugin binary in PATH.
TOOLS_BIN_DIR="${STAGE_ROOT}/tools-bin"
mkdir -p "${TOOLS_BIN_DIR}"
ln -sf "${LINUXDEPLOY_BIN}" "${TOOLS_BIN_DIR}/linuxdeploy"
ln -sf "${LINUXDEPLOY_QT_BIN}" "${TOOLS_BIN_DIR}/linuxdeploy-plugin-qt"
export PATH="${TOOLS_BIN_DIR}:${PATH}"

export APPIMAGE_EXTRACT_AND_RUN=1
export QMAKE="${QMAKE:-/usr/bin/qmake6}"
export NO_STRIP=1

# Only pass QML_SOURCES_PATHS=qml when KF6 Kirigami QML module exists on the
# build host, so Ubuntu 24.04 doesn't fail on missing org.kde.kirigami while
# linuxdeploy-plugin-qt still bundles core Qt6 libraries cleanly.
QT_INSTALL_QML="$("${QMAKE}" -query QT_INSTALL_QML 2>/dev/null || true)"
if [[ -n "${QT_INSTALL_QML}" ]] && [[ -d "${QT_INSTALL_QML}/org/kde/kirigami" ]]; then
    export QML_SOURCES_PATHS="${REPO_ROOT}/qml"
    echo "Detected KF6 Kirigami at ${QT_INSTALL_QML}/org/kde/kirigami; setting QML_SOURCES_PATHS=${QML_SOURCES_PATHS}"
else
    unset QML_SOURCES_PATHS || true
    echo "KF6 Kirigami QML module not found under '${QT_INSTALL_QML}'; skipping QML_SOURCES_PATHS"
fi

export LDAI_OUTPUT="${OUT_DIR_ABS}/${APP_NAME}-${TAG}-x86_64.AppImage"
pushd "${STAGE_ROOT}" >/dev/null
"${LINUXDEPLOY_BIN}" \
    --appdir "${APPDIR}" \
    --desktop-file "${APPDIR}/usr/share/applications/${APP_ID}.desktop" \
    --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png" \
    --plugin qt \
    --output appimage
popd >/dev/null

APPIMAGE_PATH="${OUT_DIR_ABS}/${APP_NAME}-${TAG}-x86_64.AppImage"
if [[ ! -f "${APPIMAGE_PATH}" ]]; then
    GENERATED="$(find "${STAGE_ROOT}" -maxdepth 1 -name '*.AppImage' | head -n1 || true)"
    if [[ -n "${GENERATED}" ]]; then
        mv -f "${GENERATED}" "${APPIMAGE_PATH}"
    else
        echo "error: expected AppImage was not produced" >&2
        exit 1
    fi
fi
chmod +x "${APPIMAGE_PATH}"

echo "Packaged release artifacts:"
echo "  Tarball:  ${TAR_ARCHIVE}"
echo "  AppImage: ${APPIMAGE_PATH}"
