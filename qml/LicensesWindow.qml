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

    required property string licensesText

    title: "Licenses & Third-Party Notices"
    width: 720
    height: 560
    minimumWidth: 500
    minimumHeight: 400
    visible: false

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.mediumSpacing

        QQC2.Label {
            Layout.fillWidth: true
            text: "DualSense Firmware Updater is licensed under GPL-3.0-or-later (with core HID protocol logic adapted from nchie under MIT). Open-source licenses and third-party notices:"
            wrapMode: Text.WordWrap
            opacity: 0.82
        }

        QQC2.ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            QQC2.TextArea {
                readOnly: true
                text: root.licensesText
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                font.family: "monospace"
            }
        }

        RowLayout {
            Layout.fillWidth: true

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
