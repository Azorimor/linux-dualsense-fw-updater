// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

RowLayout {
    id: root

    required property int currentStepIndex

    readonly property var steps: [
        "1. Controller",
        "2. Firmware",
        "3. Review",
        "4. Update"
    ]

    spacing: Kirigami.Units.smallSpacing

    Repeater {
        model: root.steps.length

        RowLayout {
            id: stepDelegate
            required property int index

            readonly property bool isCurrent: root.currentStepIndex === stepDelegate.index
                || (root.currentStepIndex === 4 && stepDelegate.index === 3)
            readonly property bool isDone: root.currentStepIndex > stepDelegate.index
                && root.currentStepIndex !== 5

            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: labelItem.implicitHeight + Kirigami.Units.smallSpacing * 2
                radius: Kirigami.Units.smallSpacing

                color: stepDelegate.isCurrent
                    ? Kirigami.Theme.highlightColor
                    : (stepDelegate.isDone
                        ? Kirigami.Theme.positiveBackgroundColor
                        : Kirigami.Theme.alternateBackgroundColor)
                border.width: 1
                border.color: stepDelegate.isCurrent
                    ? Kirigami.Theme.highlightColor
                    : (stepDelegate.isDone
                        ? Kirigami.Theme.positiveTextColor
                        : Kirigami.Theme.disabledTextColor)

                QQC2.Label {
                    id: labelItem
                    anchors.centerIn: parent
                    width: parent.width - Kirigami.Units.smallSpacing * 2
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                    text: root.steps[stepDelegate.index]
                    font.bold: stepDelegate.isCurrent
                    color: stepDelegate.isCurrent
                        ? Kirigami.Theme.highlightedTextColor
                        : (stepDelegate.isDone
                            ? Kirigami.Theme.positiveTextColor
                            : Kirigami.Theme.textColor)
                }
            }
        }
    }
}
