// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import dev.dualsense.updater

ColumnLayout {
    id: root

    required property DualSenseBridge updater

    spacing: Kirigami.Units.largeSpacing

    Kirigami.PlaceholderMessage {
        Layout.fillWidth: true
        Layout.topMargin: Kirigami.Units.largeSpacing
        Layout.bottomMargin: Kirigami.Units.largeSpacing
        visible: !root.updater.hasDevices && !root.updater.isBusy
        icon.name: root.updater.bluetoothOnlyDetected ? "network-bluetooth" : "input-gamepad"
        text: root.updater.bluetoothOnlyDetected
            ? (root.updater.bluetoothDeviceCount > 1
                ? "DualSense Controllers Detected Over Bluetooth"
                : "DualSense Controller Detected Over Bluetooth")
            : "No DualSense Controller Found"
        explanation: root.updater.bluetoothOnlyDetected
            ? "Your controller is connected over Bluetooth. Connect it directly to a USB port with a data cable to inspect and update its firmware."
            : "Connect your controller directly to a USB port with a data cable. Firmware updates cannot run over Bluetooth."

        QQC2.Button {
            Layout.alignment: Qt.AlignHCenter
            text: "Check Again"
            icon.name: "view-refresh"
            enabled: root.updater.canScan
            onClicked: root.updater.scanControllers()
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        visible: root.updater.hasDevices
        spacing: Kirigami.Units.smallSpacing

        QQC2.Label {
            Layout.fillWidth: true
            text: root.updater.deviceCount > 1
                ? "Select the controller you want to update:"
                : "Detected controller:"
            font.bold: true
        }

        Repeater {
            model: root.updater.deviceCount

            Kirigami.AbstractCard {
                id: deviceCard
                required property int index
                readonly property bool isSelected: index === root.updater.selectedDeviceIndex

                Layout.fillWidth: true
                showClickFeedback: root.updater.deviceCount > 1 && !root.updater.isBusy

                contentItem: RowLayout {
                    spacing: Kirigami.Units.mediumSpacing

                    QQC2.RadioButton {
                        checked: deviceCard.isSelected
                        enabled: !root.updater.isBusy
                        onClicked: root.updater.selectDeviceAt(deviceCard.index)
                    }

                    Kirigami.Icon {
                        source: "input-gamepad"
                        Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                        Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        QQC2.Label {
                            Layout.fillWidth: true
                            text: root.updater.deviceLabels[deviceCard.index] ?? ""
                            font.bold: true
                            wrapMode: Text.WordWrap
                        }

                        QQC2.Label {
                            Layout.fillWidth: true
                            text: root.updater.deviceDetails[deviceCard.index] ?? ""
                            opacity: 0.78
                            wrapMode: Text.WordWrap
                        }
                    }
                }

                onClicked: {
                    if (!root.updater.isBusy) {
                        root.updater.selectDeviceAt(deviceCard.index)
                    }
                }
            }
        }

        Kirigami.AbstractCard {
            Layout.fillWidth: true
            visible: root.updater.selectedDeviceLabel.length > 0

            contentItem: Kirigami.FormLayout {
                QQC2.Label {
                    Kirigami.FormData.label: "Controller:"
                    text: root.updater.selectedDeviceLabel
                    wrapMode: Text.WordWrap
                }
                QQC2.Label {
                    Kirigami.FormData.label: "Board Model:"
                    visible: root.updater.boardModel.length > 0
                    text: root.updater.boardModel
                    font.bold: true
                }
                QQC2.Label {
                    Kirigami.FormData.label: "Color:"
                    visible: root.updater.colorName.length > 0
                    text: root.updater.colorName
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
                    text: root.updater.selectedDeviceFirmware
                    font.bold: true
                }
                QQC2.Label {
                    Kirigami.FormData.label: "Build Info:"
                    visible: root.updater.selectedDeviceBuildInfo.length > 0
                    text: root.updater.selectedDeviceBuildInfo
                }
                QQC2.Label {
                    Kirigami.FormData.label: "Update Status:"
                    visible: root.updater.catalogChecking || root.updater.versionComparisonText.length > 0
                    text: root.updater.versionComparisonText
                    font.bold: root.updater.updateAvailable || root.updater.isUpToDate
                    color: root.updater.updateAvailable
                        ? Kirigami.Theme.highlightColor
                        : (root.updater.isUpToDate
                            ? Kirigami.Theme.positiveTextColor
                            : Kirigami.Theme.textColor)
                    wrapMode: Text.WordWrap
                }
            }
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        QQC2.Button {
            visible: root.updater.hasDevices
            text: "Check Again"
            icon.name: "view-refresh"
            enabled: root.updater.canScan
            onClicked: root.updater.scanControllers()
        }

        Item {
            Layout.fillWidth: true
        }

        QQC2.Button {
            text: "Continue"
            icon.name: "go-next"
            highlighted: true
            enabled: root.updater.canContinueToFirmware
            onClicked: root.updater.continueToFirmware()
        }
    }
}
