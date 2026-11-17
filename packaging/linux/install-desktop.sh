#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
#
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

APP_ID="dev.dualsense.updater"
ICON_SIZES=(16x16 22x22 32x32 48x48 64x64 128x128 256x256 512x512)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP_SRC="${SCRIPT_DIR}/${APP_ID}.desktop"
ICONS_SRC="${SCRIPT_DIR}/icons"

MODE="user"
ACTION="install"
CUSTOM_PREFIX=""
EXEC_OVERRIDE=""

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Install or uninstall the ${APP_ID} desktop entry and hicolor icons.

Options:
  --user            Install into \${XDG_DATA_HOME:-\$HOME/.local/share} (default)
  --system          Install into /usr/local/share (usually requires root/sudo)
  --prefix DIR      Install into DIR/share (overrides --user and --system)
  --exec PATH       Override the Exec= command in the installed .desktop file
  --uninstall       Remove the installed .desktop file and icons
  -h, --help        Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --user)
            MODE="user"
            shift
            ;;
        --system)
            MODE="system"
            shift
            ;;
        --prefix)
            if [[ $# -lt 2 ]] || [[ -z "$2" ]]; then
                echo "error: --prefix requires a non-empty directory argument" >&2
                exit 2
            fi
            MODE="prefix"
            CUSTOM_PREFIX="$2"
            shift 2
            ;;
        --exec)
            if [[ $# -lt 2 ]] || [[ -z "$2" ]]; then
                echo "error: --exec requires a non-empty command or path" >&2
                exit 2
            fi
            EXEC_OVERRIDE="$2"
            shift 2
            ;;
        --uninstall)
            ACTION="uninstall"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

case "${MODE}" in
    user)
        DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
        ;;
    system)
        DATA_DIR="/usr/local/share"
        ;;
    prefix)
        DATA_DIR="${CUSTOM_PREFIX%/}/share"
        ;;
esac

APPS_DIR="${DATA_DIR}/applications"
ICON_THEME_DIR="${DATA_DIR}/icons/hicolor"
DESKTOP_DEST="${APPS_DIR}/${APP_ID}.desktop"

refresh_caches() {
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "${ICON_THEME_DIR}" >/dev/null 2>&1 || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "${APPS_DIR}" >/dev/null 2>&1 || true
    fi
}

if [[ "${ACTION}" == "uninstall" ]]; then
    rm -f "${DESKTOP_DEST}"
    for size in "${ICON_SIZES[@]}"; do
        rm -f "${ICON_THEME_DIR}/${size}/apps/${APP_ID}.png"
    done
    rm -f "${ICON_THEME_DIR}/scalable/apps/${APP_ID}.svg"
    refresh_caches
    echo "Uninstalled ${APP_ID} desktop entry and icons from ${DATA_DIR}"
    exit 0
fi

if [[ ! -f "${DESKTOP_SRC}" ]]; then
    echo "error: desktop file not found: ${DESKTOP_SRC}" >&2
    exit 1
fi
for size in "${ICON_SIZES[@]}"; do
    if [[ ! -f "${ICONS_SRC}/${size}/icon.png" ]]; then
        echo "error: icon missing for size ${size}: ${ICONS_SRC}/${size}/icon.png" >&2
        exit 1
    fi
done
if [[ ! -f "${ICONS_SRC}/scalable/icon.svg" ]]; then
    echo "error: scalable SVG icon missing: ${ICONS_SRC}/scalable/icon.svg" >&2
    exit 1
fi

for size in "${ICON_SIZES[@]}"; do
    install -Dm644 "${ICONS_SRC}/${size}/icon.png" \
        "${ICON_THEME_DIR}/${size}/apps/${APP_ID}.png"
done
install -Dm644 "${ICONS_SRC}/scalable/icon.svg" \
    "${ICON_THEME_DIR}/scalable/apps/${APP_ID}.svg"

mkdir -p "${APPS_DIR}"
if [[ -z "${EXEC_OVERRIDE}" ]] && [[ -x "${SCRIPT_DIR}/dualsense-updater" ]] && ! command -v dualsense-updater >/dev/null 2>&1; then
    EXEC_OVERRIDE="${SCRIPT_DIR}/dualsense-updater"
fi

if [[ -n "${EXEC_OVERRIDE}" ]]; then
    TMP_DESKTOP="$(mktemp)"
    trap 'rm -f "${TMP_DESKTOP}"' EXIT
    ESCAPED_EXEC="${EXEC_OVERRIDE//\\/\\\\}"
    ESCAPED_EXEC="${ESCAPED_EXEC//&/\\&}"
    ESCAPED_EXEC="${ESCAPED_EXEC//|/\\|}"
    sed "s|^Exec=.*|Exec=${ESCAPED_EXEC}|" "${DESKTOP_SRC}" > "${TMP_DESKTOP}"
    install -Dm644 "${TMP_DESKTOP}" "${DESKTOP_DEST}"
else
    install -Dm644 "${DESKTOP_SRC}" "${DESKTOP_DEST}"
fi

refresh_caches
echo "Installed ${APP_ID} desktop entry and hicolor icons into ${DATA_DIR}"
