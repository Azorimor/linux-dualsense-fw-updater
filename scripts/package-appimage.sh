#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
#
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

usage() {
    echo "Usage: $0 [--stage-only | --tarball-only | --appimage-only] <binary-path> <tag> <out-dir>" >&2
}

MODE="all"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --stage-only)
            if [[ "${MODE}" != "all" ]]; then
                echo "error: --stage-only, --tarball-only, and --appimage-only are mutually exclusive" >&2
                usage
                exit 2
            fi
            MODE="stage-only"
            shift
            ;;
        --tarball-only)
            if [[ "${MODE}" != "all" ]]; then
                echo "error: --stage-only, --tarball-only, and --appimage-only are mutually exclusive" >&2
                usage
                exit 2
            fi
            MODE="tarball-only"
            shift
            ;;
        --appimage-only)
            if [[ "${MODE}" != "all" ]]; then
                echo "error: --stage-only, --tarball-only, and --appimage-only are mutually exclusive" >&2
                usage
                exit 2
            fi
            MODE="appimage-only"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "error: unknown option: $1" >&2
            usage
            exit 2
            ;;
        *)
            break
            ;;
    esac
done

if [[ $# -ne 3 ]]; then
    usage
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
LGPL3_LICENSE_FILE="${REPO_ROOT}/LICENSES/LGPL-3.0-only.txt"
LGPL21_LICENSE_FILE="${REPO_ROOT}/LICENSES/LGPL-2.1-or-later.txt"
LGPL20_LICENSE_FILE="${REPO_ROOT}/LICENSES/LGPL-2.0-or-later.txt"
BSD3_LICENSE_FILE="${REPO_ROOT}/LICENSES/BSD-3-Clause.txt"
THIRD_PARTY_NOTICES_FILE="${REPO_ROOT}/packaging/linux/THIRD_PARTY_NOTICES.txt"
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
    "${MIT_LICENSE_FILE}" \
    "${LGPL3_LICENSE_FILE}" \
    "${LGPL21_LICENSE_FILE}" \
    "${LGPL20_LICENSE_FILE}" \
    "${BSD3_LICENSE_FILE}" \
    "${THIRD_PARTY_NOTICES_FILE}"
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

# 1. Build portable .tar.gz release archive (unless --appimage-only)
TAR_ARCHIVE="${OUT_DIR_ABS}/${APP_NAME}-${TAG}-x86_64.tar.gz"
if [[ "${MODE}" != "appimage-only" ]]; then
    TAR_STAGE="${STAGE_ROOT}/${APP_NAME}-${TAG}-x86_64"
    mkdir -p "${TAR_STAGE}"
    install -Dm755 "${BINARY_ABS}" "${TAR_STAGE}/dualsense-updater"
    for lic_file in "${REPO_ROOT}/LICENSES/"*.txt; do
        install -Dm644 "${lic_file}" "${TAR_STAGE}/LICENSES/$(basename "${lic_file}")"
    done
    install -Dm644 "${THIRD_PARTY_NOTICES_FILE}" "${TAR_STAGE}/THIRD_PARTY_NOTICES.txt"
    if [[ -f "${README_FILE}" ]]; then
        install -Dm644 "${README_FILE}" "${TAR_STAGE}/README.md"
    fi
    install -Dm644 "${DESKTOP_FILE}" "${TAR_STAGE}/${APP_ID}.desktop"
    install -Dm755 "${INSTALL_SCRIPT}" "${TAR_STAGE}/install-desktop.sh"
    for size in "${ICON_SIZES[@]}"; do
        install -Dm644 "${ICONS_DIR}/${size}/icon.png" "${TAR_STAGE}/icons/${size}/icon.png"
    done
    install -Dm644 "${SVG_ICON_FILE}" "${TAR_STAGE}/icons/scalable/icon.svg"

    tar -C "${STAGE_ROOT}" -czf "${TAR_ARCHIVE}" "${APP_NAME}-${TAG}-x86_64"
fi

if [[ "${MODE}" == "tarball-only" ]]; then
    echo "Packaged release artifacts:"
    echo "  Tarball: ${TAR_ARCHIVE}"
    exit 0
fi

# 2. Stage AppDir for linuxdeploy + linuxdeploy-plugin-qt
APPDIR="${STAGE_ROOT}/AppDir"
APPDIR_LICENSES_DIR="${APPDIR}/usr/share/licenses/${APP_NAME}"
mkdir -p "${APPDIR}/usr/bin" "${APPDIR_LICENSES_DIR}"
install -Dm755 "${BINARY_ABS}" "${APPDIR}/usr/bin/dualsense-updater"
install -Dm644 "${DESKTOP_FILE}" "${APPDIR}/usr/share/applications/${APP_ID}.desktop"
for size in "${ICON_SIZES[@]}"; do
    install -Dm644 "${ICONS_DIR}/${size}/icon.png" \
        "${APPDIR}/usr/share/icons/hicolor/${size}/apps/${APP_ID}.png"
done
install -Dm644 "${SVG_ICON_FILE}" \
    "${APPDIR}/usr/share/icons/hicolor/scalable/apps/${APP_ID}.svg"
install -Dm644 "${DIRICON_SOURCE}" "${APPDIR}/.DirIcon"
for lic_file in "${REPO_ROOT}/LICENSES/"*.txt; do
    install -Dm644 "${lic_file}" "${APPDIR_LICENSES_DIR}/$(basename "${lic_file}")"
done
install -Dm644 "${THIRD_PARTY_NOTICES_FILE}" "${APPDIR_LICENSES_DIR}/THIRD_PARTY_NOTICES.txt"

if [[ "${MODE}" == "stage-only" ]]; then
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
if [[ -z "${QMAKE:-}" ]]; then
    if command -v qmake6 >/dev/null 2>&1; then
        QMAKE="$(command -v qmake6)"
    else
        QMAKE="/usr/bin/qmake6"
    fi
fi
if [[ ! -x "${QMAKE}" ]]; then
    echo "error: qmake6 not found or not executable: ${QMAKE}" >&2
    exit 1
fi
export QMAKE
export NO_STRIP=1

QT_INSTALL_QML="$("${QMAKE}" -query QT_INSTALL_QML 2>/dev/null || true)"
QT_INSTALL_PLUGINS="$("${QMAKE}" -query QT_INSTALL_PLUGINS 2>/dev/null || true)"
QT_INSTALL_LIBS="$("${QMAKE}" -query QT_INSTALL_LIBS 2>/dev/null || true)"
if [[ -z "${QT_INSTALL_PLUGINS}" ]] || [[ ! -d "${QT_INSTALL_PLUGINS}" ]]; then
    echo "error: QT_INSTALL_PLUGINS directory not found: '${QT_INSTALL_PLUGINS}'" >&2
    exit 1
fi

# Always pass QML_SOURCES_PATHS=qml and require org.kde.kirigami in
# QT_INSTALL_QML or QML_MODULES_PATHS so linuxdeploy-plugin-qt bundles all
# required QML modules (QtQuick.Controls, Dialogs, Layouts, Kirigami, etc.).
export QML_SOURCES_PATHS="${REPO_ROOT}/qml"
KIRIGAMI_FOUND=0
QML_SEARCH_DIRS=()
if [[ -n "${QT_INSTALL_QML}" ]]; then
    QML_SEARCH_DIRS+=("${QT_INSTALL_QML}")
fi
if [[ -n "${QML_MODULES_PATHS:-}" ]]; then
    IFS=':' read -r -a EXTRA_QML_DIRS <<< "${QML_MODULES_PATHS}"
    for dir in "${EXTRA_QML_DIRS[@]}"; do
        if [[ -n "${dir}" ]]; then
            QML_SEARCH_DIRS+=("${dir}")
        fi
    done
fi
for dir in "${QML_SEARCH_DIRS[@]}"; do
    if [[ -d "${dir}/org/kde/kirigami" ]]; then
        KIRIGAMI_FOUND=1
        echo "Detected KF6 Kirigami at ${dir}/org/kde/kirigami; setting QML_SOURCES_PATHS=${QML_SOURCES_PATHS}"
        break
    fi
done
if [[ "${KIRIGAMI_FOUND}" -ne 1 ]]; then
    echo "error: KF6 Kirigami QML module (org/kde/kirigami) not found under QT_INSTALL_QML ('${QT_INSTALL_QML}') or QML_MODULES_PATHS ('${QML_MODULES_PATHS:-}')" >&2
    echo "hint: install/build KF6 Kirigami for this Qt6 toolchain or pass its QML directory in QML_MODULES_PATHS" >&2
    exit 1
fi

DEPLOY_LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}"
if [[ -n "${QT_INSTALL_LIBS}" ]]; then
    DEPLOY_LD_LIBRARY_PATH="${QT_INSTALL_LIBS}${DEPLOY_LD_LIBRARY_PATH:+:${DEPLOY_LD_LIBRARY_PATH}}"
fi

# Filter out optional imageformats plugins (e.g. kimg_jxr.so without jxrlib)
# that have unresolved shared library dependencies, which would otherwise abort
# linuxdeploy-plugin-qt on desktop hosts with partial optional KDE plugins.
EFFECTIVE_QT_PLUGINS="${QT_INSTALL_PLUGINS}"
BROKEN_IMAGEFORMAT_PLUGINS=()
if [[ -d "${QT_INSTALL_PLUGINS}/imageformats" ]]; then
    shopt -s nullglob
    for plugin_so in "${QT_INSTALL_PLUGINS}/imageformats/"*.so; do
        if LD_LIBRARY_PATH="${DEPLOY_LD_LIBRARY_PATH}" ldd "${plugin_so}" 2>/dev/null | grep -q '=> not found'; then
            BROKEN_IMAGEFORMAT_PLUGINS+=("$(basename "${plugin_so}")")
        fi
    done
    shopt -u nullglob
fi

if [[ "${#BROKEN_IMAGEFORMAT_PLUGINS[@]}" -gt 0 ]]; then
    FILTERED_PLUGINS_DIR="${STAGE_ROOT}/qt-plugins"
    mkdir -p "${FILTERED_PLUGINS_DIR}"
    shopt -s nullglob
    for entry in "${QT_INSTALL_PLUGINS}"/*; do
        entry_name="$(basename "${entry}")"
        if [[ "${entry_name}" == "imageformats" ]]; then
            mkdir -p "${FILTERED_PLUGINS_DIR}/imageformats"
            for plugin_so in "${QT_INSTALL_PLUGINS}/imageformats/"*; do
                plugin_name="$(basename "${plugin_so}")"
                skip_plugin=0
                for broken in "${BROKEN_IMAGEFORMAT_PLUGINS[@]}"; do
                    if [[ "${plugin_name}" == "${broken}" ]]; then
                        skip_plugin=1
                        break
                    fi
                done
                if [[ "${skip_plugin}" -eq 1 ]]; then
                    echo "Skipping imageformats plugin with unresolved dependencies: ${plugin_name}" >&2
                else
                    ln -sf "${plugin_so}" "${FILTERED_PLUGINS_DIR}/imageformats/${plugin_name}"
                fi
            done
        else
            ln -sf "${entry}" "${FILTERED_PLUGINS_DIR}/${entry_name}"
        fi
    done
    shopt -u nullglob

    REAL_QMAKE="${QMAKE}"
    QMAKE_WRAPPER="${TOOLS_BIN_DIR}/qmake6"
    cat > "${QMAKE_WRAPPER}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "-query" ]] && [[ "\${2:-}" == "QT_INSTALL_PLUGINS" ]]; then
    echo "${FILTERED_PLUGINS_DIR}"
    exit 0
fi
if [[ "\${1:-}" == "-query" ]] && [[ \$# -eq 1 ]]; then
    "${REAL_QMAKE}" -query | sed 's|^QT_INSTALL_PLUGINS:.*|QT_INSTALL_PLUGINS:${FILTERED_PLUGINS_DIR}|'
    exit 0
fi
exec "${REAL_QMAKE}" "\$@"
EOF
    chmod +x "${QMAKE_WRAPPER}"
    ln -sf "${QMAKE_WRAPPER}" "${TOOLS_BIN_DIR}/qmake"
    export QMAKE="${QMAKE_WRAPPER}"
    EFFECTIVE_QT_PLUGINS="${FILTERED_PLUGINS_DIR}"
fi

if [[ -z "${EXTRA_QT_MODULES+x}" ]]; then
    if [[ -f "${EFFECTIVE_QT_PLUGINS}/iconengines/libqsvgicon.so" ]] || [[ -f "${EFFECTIVE_QT_PLUGINS}/imageformats/libqsvg.so" ]]; then
        export EXTRA_QT_MODULES="svg"
        echo "Setting EXTRA_QT_MODULES=${EXTRA_QT_MODULES}"
    fi
fi

if [[ -z "${EXTRA_PLATFORM_PLUGINS+x}" ]]; then
    PLATFORM_PLUGINS=()
    for candidate in libqwayland.so libqwayland-egl.so libqwayland-generic.so libqoffscreen.so; do
        if [[ -f "${EFFECTIVE_QT_PLUGINS}/platforms/${candidate}" ]]; then
            PLATFORM_PLUGINS+=("${candidate}")
        fi
    done
    if [[ "${#PLATFORM_PLUGINS[@]}" -gt 0 ]]; then
        EXTRA_PLATFORM_PLUGINS="$(IFS=';'; echo "${PLATFORM_PLUGINS[*]}")"
        export EXTRA_PLATFORM_PLUGINS
        echo "Setting EXTRA_PLATFORM_PLUGINS=${EXTRA_PLATFORM_PLUGINS}"
    fi
fi

export LDAI_OUTPUT="${OUT_DIR_ABS}/${APP_NAME}-${TAG}-x86_64.AppImage"
WAYLAND_GFX_SRC="${EFFECTIVE_QT_PLUGINS}/wayland-graphics-integration-client"
WAYLAND_GFX_SOS=()
if [[ -d "${WAYLAND_GFX_SRC}" ]]; then
    shopt -s nullglob
    WAYLAND_GFX_SOS=("${WAYLAND_GFX_SRC}"/*.so)
    shopt -u nullglob
fi

collect_appdir_library_notices() {
    local appdir="$1"
    local licenses_dir="$2"
    local bundled_list_file="${licenses_dir}/BUNDLED_LIBRARIES.txt"

    local search_dirs=()
    for sub in lib plugins qml; do
        if [[ -d "${appdir}/usr/${sub}" ]]; then
            search_dirs+=("${appdir}/usr/${sub}")
        fi
    done

    local bundled_libs=""
    if [[ "${#search_dirs[@]}" -gt 0 ]]; then
        bundled_libs="$(find "${search_dirs[@]}" \( -type f -o -type l \) \( -name '*.so' -o -name '*.so.*' \) \
            | sed "s|^${appdir}/||" \
            | LC_ALL=C sort -u || true)"
    fi

    {
        echo "# Bundled shared libraries inside AppDir (relative to AppDir root)"
        echo "# See THIRD_PARTY_NOTICES.txt for Qt 6, KDE Kirigami, hidapi, and system package notices."
        if [[ -n "${bundled_libs}" ]]; then
            printf '%s\n' "${bundled_libs}"
        fi
    } > "${bundled_list_file}"

    if [[ -n "${QT_APPIMAGE_DIR:-}" ]]; then
        if [[ -d "${QT_APPIMAGE_DIR}/LICENSES" ]]; then
            shopt -s nullglob
            for lic_file in "${QT_APPIMAGE_DIR}/LICENSES/"*; do
                if [[ -f "${lic_file}" ]]; then
                    install -Dm644 "${lic_file}" \
                        "${licenses_dir}/upstream-licenses/$(basename "${lic_file}")"
                fi
            done
            shopt -u nullglob
        fi
        shopt -s nullglob
        for lic_file in "${QT_APPIMAGE_DIR}/"LICENSE*; do
            if [[ -f "${lic_file}" ]]; then
                install -Dm644 "${lic_file}" \
                    "${licenses_dir}/upstream-licenses/$(basename "${lic_file}")"
            fi
        done
        shopt -u nullglob
    fi

    if [[ -n "${bundled_libs}" ]] && command -v dpkg-query >/dev/null 2>&1; then
        mkdir -p "${licenses_dir}/system-packages"
        declare -A seen_pkgs=()
        while IFS= read -r rel_lib; do
            [[ -z "${rel_lib}" ]] && continue
            local so_name
            so_name="$(basename "${rel_lib}")"
            local query_out=""
            query_out="$(dpkg-query -S "*/${so_name}" 2>/dev/null || true)"
            [[ -z "${query_out}" ]] && continue
            while IFS= read -r line; do
                [[ "${line}" == diversion* ]] && continue
                [[ "${line}" == "local diversion"* ]] && continue
                [[ "${line}" == *": "* ]] || continue
                local pkg_part="${line%%: *}"
                local path_part="${line#*: }"
                [[ "$(basename "${path_part}")" == "${so_name}" ]] || continue
                local pkg_name="${pkg_part%%:*}"
                pkg_name="${pkg_name%%,*}"
                [[ -n "${pkg_name}" ]] || continue
                if [[ -z "${seen_pkgs[${pkg_name}]+x}" ]]; then
                    seen_pkgs["${pkg_name}"]=1
                    local copyright_file="/usr/share/doc/${pkg_name}/copyright"
                    if [[ ! -f "${copyright_file}" ]]; then
                        copyright_file="/usr/share/doc/${pkg_part}/copyright"
                    fi
                    if [[ -f "${copyright_file}" ]]; then
                        install -Dm644 "${copyright_file}" \
                            "${licenses_dir}/system-packages/${pkg_name}.copyright"
                    fi
                fi
            done <<< "${query_out}"
        done <<< "${bundled_libs}"
    fi
}

pushd "${STAGE_ROOT}" >/dev/null
if [[ "${#WAYLAND_GFX_SOS[@]}" -gt 0 ]]; then
    "${LINUXDEPLOY_BIN}" \
        --appdir "${APPDIR}" \
        --desktop-file "${APPDIR}/usr/share/applications/${APP_ID}.desktop" \
        --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png" \
        --plugin qt

    mkdir -p "${APPDIR}/usr/plugins/wayland-graphics-integration-client"
    cp -L "${WAYLAND_GFX_SOS[@]}" "${APPDIR}/usr/plugins/wayland-graphics-integration-client/"

    LD_LIBRARY_PATH="${DEPLOY_LD_LIBRARY_PATH}" \
        "${LINUXDEPLOY_BIN}" \
        --appdir "${APPDIR}" \
        --desktop-file "${APPDIR}/usr/share/applications/${APP_ID}.desktop" \
        --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png" \
        --deploy-deps-only "${APPDIR}/usr/plugins/wayland-graphics-integration-client"
else
    "${LINUXDEPLOY_BIN}" \
        --appdir "${APPDIR}" \
        --desktop-file "${APPDIR}/usr/share/applications/${APP_ID}.desktop" \
        --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png" \
        --plugin qt
fi

collect_appdir_library_notices "${APPDIR}" "${APPDIR_LICENSES_DIR}"

"${LINUXDEPLOY_BIN}" \
    --appdir "${APPDIR}" \
    --desktop-file "${APPDIR}/usr/share/applications/${APP_ID}.desktop" \
    --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png" \
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
if [[ "${MODE}" != "appimage-only" ]]; then
    echo "  Tarball:  ${TAR_ARCHIVE}"
fi
echo "  AppImage: ${APPIMAGE_PATH}"
