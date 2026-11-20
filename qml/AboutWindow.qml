// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.ApplicationWindow {
    id: root

    required property string appVersion
    required property string githubUrl
    required property string githubPagesUrl
    required property string githubIssuesUrl

    signal openLicensesRequested()

    title: "About Unofficial DualSense Firmware Updater"
    width: 580
    height: 510
    minimumWidth: 480
    minimumHeight: 440
    visible: false

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.largeSpacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.mediumSpacing

            Kirigami.Icon {
                source: "qrc:/qt/qml/io/github/azorimor/linux_dualsense_fw_updater/packaging/linux/icons/256x256/icon.png"
                fallback: "input-gamepad"
                Layout.preferredWidth: Kirigami.Units.iconSizes.huge
                Layout.preferredHeight: Kirigami.Units.iconSizes.huge
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Heading {
                    Layout.fillWidth: true
                    level: 2
                    text: "Unofficial DualSense Firmware Updater"
                    wrapMode: Text.WordWrap
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    text: "Version " + root.appVersion
                    font.bold: true
                    opacity: 0.85
                }
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            text: "A community-maintained Linux KDE Kirigami desktop application for inspecting and updating PlayStation DualSense and DualSense Edge controller firmware over USB HID.\n\nThis project is unofficial and has no affiliation with Sony Interactive Entertainment or PlayStation."
            wrapMode: Text.WordWrap
        }

        Kirigami.AbstractCard {
            Layout.fillWidth: true

            contentItem: ColumnLayout {
                spacing: Kirigami.Units.smallSpacing

                QQC2.Label {
                    Layout.fillWidth: true
                    text: "Project Links"
                    font.bold: true
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    textFormat: Text.RichText
                    wrapMode: Text.Wrap
                    text: "<b>GitHub Repository:</b> <a href=\"" + root.githubUrl + "\">" + root.githubUrl + "</a>"
                    onLinkActivated: function (link) {
                        Qt.openUrlExternally(link)
                    }

                    HoverHandler {
                        enabled: parent.hoveredLink.length > 0
                        cursorShape: Qt.PointingHandCursor
                    }
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    textFormat: Text.RichText
                    wrapMode: Text.Wrap
                    text: "<b>Website (GitHub Pages):</b> <a href=\"" + root.githubPagesUrl + "\">" + root.githubPagesUrl + "</a>"
                    onLinkActivated: function (link) {
                        Qt.openUrlExternally(link)
                    }

                    HoverHandler {
                        enabled: parent.hoveredLink.length > 0
                        cursorShape: Qt.PointingHandCursor
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: Kirigami.Units.smallSpacing
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.Button {
                        text: "Open GitHub"
                        icon.name: "internet-web-browser"
                        onClicked: Qt.openUrlExternally(root.githubUrl)
                    }

                    QQC2.Button {
                        text: "Open Website"
                        icon.name: "globe"
                        onClicked: Qt.openUrlExternally(root.githubPagesUrl)
                    }

                    QQC2.Button {
                        text: "Issues"
                        icon.name: "tools-report-bug"
                        onClicked: Qt.openUrlExternally(root.githubIssuesUrl)
                    }
                }
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            text: "Copyright (C) 2026 Azorimor <azorimor@gmail.com>\nLicensed under the GNU General Public License v3.0 or later (GPL-3.0-or-later); core HID protocol logic adapted from nchie under MIT. You may redistribute and/or modify this program under the terms of GPL-3.0-or-later.\n\nThis program comes with ABSOLUTELY NO WARRANTY, to the extent permitted by applicable law."
            wrapMode: Text.WordWrap
            opacity: 0.82
        }

        Item {
            Layout.fillHeight: true
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            QQC2.Button {
                text: "View Licenses"
                icon.name: "license"
                onClicked: root.openLicensesRequested()
            }

            Item {
                Layout.fillWidth: true
            }

            QQC2.Button {
                text: "Close"
                icon.name: "window-close"
                onClicked: root.close()
            }
        }
    }
}
