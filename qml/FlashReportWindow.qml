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

    required property string reportTitle
    required property string reportMarkdown
    required property string issueUrl
    required property string issuesUrl
    required property bool reportSucceeded

    property bool copiedFeedback: false

    title: "Flash Run Infos"
    width: 700
    height: 600
    minimumWidth: 520
    minimumHeight: 440
    visible: false

    onVisibleChanged: {
        if (!visible) {
            copiedFeedback = false
        }
    }

    onReportMarkdownChanged: copiedFeedback = false

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.mediumSpacing

        Kirigami.Heading {
            Layout.fillWidth: true
            level: 2
            text: root.reportSucceeded
                ? "Flash Run Report (Success)"
                : "Flash Run Report (Failed Attempt)"
            wrapMode: Text.WordWrap
        }

        QQC2.Label {
            Layout.fillWidth: true
            text: "Optional: share these anonymized controller and system metrics to help track firmware compatibility across DualSense board revisions and Linux environments. Serial numbers and local file paths are never included, and you can edit everything on GitHub before submitting."
            wrapMode: Text.WordWrap
            opacity: 0.82
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: root.copiedFeedback
            type: Kirigami.MessageType.Positive
            text: "Flash report copied to your clipboard."
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            QQC2.Label {
                Layout.fillWidth: true
                text: "Suggested GitHub Issue Title:"
                font.bold: true
            }

            QQC2.TextField {
                Layout.fillWidth: true
                readOnly: true
                selectByMouse: true
                text: root.reportTitle
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            text: "Issue Body (Markdown):"
            font.bold: true
        }

        QQC2.ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            QQC2.TextArea {
                id: reportBodyArea
                readOnly: true
                selectByMouse: true
                wrapMode: TextEdit.Wrap
                font.family: "monospace"
                text: root.reportMarkdown
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            QQC2.Button {
                text: "Copy Report"
                icon.name: "edit-copy"
                enabled: root.reportMarkdown.length > 0
                onClicked: {
                    reportBodyArea.selectAll()
                    reportBodyArea.copy()
                    reportBodyArea.deselect()
                    root.copiedFeedback = true
                }
            }

            QQC2.Button {
                text: "View GitHub Issues"
                icon.name: "tools-report-bug"
                onClicked: Qt.openUrlExternally(root.issuesUrl)
            }

            Item {
                Layout.fillWidth: true
            }

            QQC2.Button {
                text: "Open GitHub Issue"
                icon.name: "internet-web-browser"
                highlighted: true
                enabled: root.issueUrl.length > 0
                onClicked: Qt.openUrlExternally(root.issueUrl)
            }

            QQC2.Button {
                text: "Close"
                icon.name: "window-close"
                onClicked: root.close()
            }
        }
    }
}
