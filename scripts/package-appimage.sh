#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "Usage: $0 <binary-path> <tag> <out-dir>" >&2
    exit 2
fi

BINARY_PATH="$1"
TAG="$2"
OUT_DIR="$3"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_FILE="${REPO_ROOT}/packaging/linux/dev.dualsense.updater.desktop"
ICON_FILE="${REPO_ROOT}/packaging/linux/dev.dualsense.updater.svg"
LICENSE_FILE="${REPO_ROOT}/LICENSE"
README_FILE="${REPO_ROOT}/README.md"

if [[ ! -x "${BINARY_PATH}" ]]; then
    echo "error: binary not found or not executable: ${BINARY_PATH}" >&2
    exit 1
fi
for required in "${DESKTOP_FILE}" "${ICON_FILE}" "${LICENSE_FILE}"; do
    if [[ ! -f "${required}" ]]; then
        echo "error: required packaging asset missing: ${required}" >&2
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
install -Dm644 "${LICENSE_FILE}" "${TAR_STAGE}/LICENSE"
if [[ -f "${README_FILE}" ]]; then
    install -Dm644 "${README_FILE}" "${TAR_STAGE}/README.md"
fi
install -Dm644 "${DESKTOP_FILE}" "${TAR_STAGE}/dev.dualsense.updater.desktop"
install -Dm644 "${ICON_FILE}" "${TAR_STAGE}/dev.dualsense.updater.svg"

TAR_ARCHIVE="${OUT_DIR_ABS}/${APP_NAME}-${TAG}-x86_64.tar.gz"
tar -C "${STAGE_ROOT}" -czf "${TAR_ARCHIVE}" "${APP_NAME}-${TAG}-x86_64"

# 2. Stage AppDir for linuxdeploy + linuxdeploy-plugin-qt
APPDIR="${STAGE_ROOT}/AppDir"
mkdir -p "${APPDIR}/usr/bin"
install -Dm755 "${BINARY_ABS}" "${APPDIR}/usr/bin/dualsense-updater"
install -Dm644 "${DESKTOP_FILE}" "${APPDIR}/usr/share/applications/dev.dualsense.updater.desktop"
install -Dm644 "${ICON_FILE}" "${APPDIR}/usr/share/icons/hicolor/scalable/apps/dev.dualsense.updater.svg"
install -Dm644 "${LICENSE_FILE}" "${APPDIR}/usr/share/licenses/${APP_NAME}/LICENSE"

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
    --desktop-file "${APPDIR}/usr/share/applications/dev.dualsense.updater.desktop" \
    --icon-file "${APPDIR}/usr/share/icons/hicolor/scalable/apps/dev.dualsense.updater.svg" \
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
