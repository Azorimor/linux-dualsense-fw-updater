// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.azorimor.linux_dualsense_fw_updater

ColumnLayout {
    id: root

    required property DualSenseBridge updater

    spacing: Kirigami.Units.largeSpacing

    Kirigami.AbstractCard {
        Layout.fillWidth: true
        visible: root.updater.hasUpdateSummary

        contentItem: Kirigami.FormLayout {
            QQC2.Label {
                Kirigami.FormData.label: "Controller:"
                text: root.updater.summaryController
                wrapMode: Text.WordWrap
            }
            QQC2.Label {
                Kirigami.FormData.label: "Battery:"
                visible: root.updater.hasBatterySummary
                text: root.updater.batterySummary
                color: root.updater.batteryIsCriticallyLow
                    ? Kirigami.Theme.negativeTextColor
                    : (root.updater.batteryIsLow
                        ? Kirigami.Theme.neutralTextColor
                        : Kirigami.Theme.textColor)
            }
            QQC2.Label {
                Kirigami.FormData.label: "Current Firmware:"
                text: root.updater.summaryCurrentVersion
            }
            QQC2.Label {
                Kirigami.FormData.label: "New Firmware:"
                text: root.updater.summaryTargetVersion
                font.bold: true
            }
            QQC2.Label {
                Kirigami.FormData.label: "Source:"
                text: root.updater.summarySource
                wrapMode: Text.WordWrap
            }
        }
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: true
        type: Kirigami.MessageType.Warning
        text: root.updater.irreversibleWarning
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: root.updater.batteryWarning.length > 0
        type: root.updater.batteryIsCriticallyLow
            ? Kirigami.MessageType.Error
            : Kirigami.MessageType.Warning
        text: root.updater.batteryWarning
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: root.updater.useLocalFile && root.updater.localFirmwareWarning.length > 0
        type: Kirigami.MessageType.Warning
        text: root.updater.localFirmwareWarning
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: root.updater.summaryIsSameOrOlder
        type: Kirigami.MessageType.Warning
        text: "Your controller already has this firmware version (or newer). Continuing may stop with an error because downgrades are blocked by Sony's firmware."
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: root.updater.summaryIsUntested
        type: Kirigami.MessageType.Warning
        text: "This hardware target has not been tested by the maintainers. Proceed only if you accept the risk."
    }

    QQC2.CheckBox {
        Layout.fillWidth: true
        checked: root.updater.warningAcknowledged
        enabled: !root.updater.isBusy
        text: "I understand that firmware updates cannot be reversed and I will keep the USB cable connected."
        onToggled: root.updater.setWarningAcknowledged(checked)
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        QQC2.Button {
            text: "Back"
            icon.name: "go-previous"
            enabled: root.updater.canGoBack
            onClicked: root.updater.goBack()
        }

        Item {
            Layout.fillWidth: true
        }

        QQC2.Button {
            text: "Start Update"
            icon.name: "system-software-update"
            highlighted: true
            enabled: root.updater.canStartUpdate
            onClicked: root.updater.startUpdate()
        }
    }
}
