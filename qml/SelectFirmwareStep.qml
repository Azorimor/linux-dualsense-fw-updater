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

    signal chooseLocalFileRequested()
    signal saveFirmwareRequested()

    spacing: Kirigami.Units.largeSpacing

    Kirigami.AbstractCard {
        Layout.fillWidth: true
        visible: root.updater.boardModel.length > 0

        contentItem: Kirigami.FormLayout {
            QQC2.Label {
                Kirigami.FormData.label: "Board Model:"
                text: root.updater.boardModel
                font.bold: true
            }
            QQC2.Label {
                Kirigami.FormData.label: "Color:"
                visible: root.updater.colorName.length > 0
                text: root.updater.colorName
            }
            QQC2.Label {
                Kirigami.FormData.label: "FW Series:"
                visible: root.updater.fwSeriesText.length > 0
                text: root.updater.fwSeriesText
            }
        }
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: root.updater.hasUnsupportedBoardWarning && !root.updater.useLocalFile
        type: Kirigami.MessageType.Warning
        text: root.updater.unsupportedBoardWarning
    }

    // Option A: Official Sony Firmware
    Kirigami.AbstractCard {
        Layout.fillWidth: true

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.mediumSpacing

            QQC2.RadioButton {
                text: "Download Official Sony Firmware (Recommended)"
                font.bold: true
                checked: !root.updater.useLocalFile
                enabled: !root.updater.isBusy
                onClicked: root.updater.setUseLocalFile(false)
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: Kirigami.Units.largeSpacing
                spacing: Kirigami.Units.smallSpacing
                enabled: !root.updater.useLocalFile && !root.updater.isBusy

                QQC2.Label {
                    Layout.fillWidth: true
                    text: "Choose your controller hardware model:"
                    wrapMode: Text.WordWrap
                }

                QQC2.ComboBox {
                    id: targetComboBox
                    Layout.fillWidth: true
                    enabled: root.updater.canSelectTarget
                    model: root.updater.targetLabels
                    currentIndex: root.updater.selectedTargetIndex
                    onActivated: index => {
                        root.updater.selectTargetAt(index)
                        targetComboBox.currentIndex = Qt.binding(() => root.updater.selectedTargetIndex)
                    }
                    onModelChanged: targetComboBox.currentIndex = Qt.binding(() => root.updater.selectedTargetIndex)
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: !root.updater.selectedTargetTested
                    type: Kirigami.MessageType.Warning
                    text: "DualSense Edge has not been hardware-tested with this utility. Proceed with caution."
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.mediumSpacing

                    QQC2.Button {
                        text: root.updater.catalogChecking
                            ? "Checking…"
                            : (root.updater.catalogChecked ? "Refresh Version" : "Check for Update")
                        icon.name: "download"
                        enabled: root.updater.canCheckUpdates
                        onClicked: {
                            root.updater.setUseLocalFile(false)
                            root.updater.checkForUpdates()
                        }
                    }

                    QQC2.Label {
                        Layout.fillWidth: true
                        text: root.updater.versionComparisonText.length > 0
                            ? root.updater.versionComparisonText
                            : (root.updater.catalogChecked
                                ? ("Latest Sony firmware: " + root.updater.selectedTargetLatestVersion)
                                : "Click \"Check for Update\" to check Sony's server.")
                        font.bold: root.updater.updateAvailable || root.updater.isUpToDate
                        color: root.updater.updateAvailable
                            ? Kirigami.Theme.highlightColor
                            : (root.updater.isUpToDate
                                ? Kirigami.Theme.positiveTextColor
                                : Kirigami.Theme.textColor)
                        wrapMode: Text.WordWrap
                        opacity: (root.updater.updateAvailable || root.updater.isUpToDate) ? 1.0 : 0.82
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    visible: root.updater.canSaveCachedFirmware
                    spacing: Kirigami.Units.mediumSpacing

                    QQC2.Button {
                        text: "Save Firmware Copy…"
                        icon.name: "document-save-as"
                        enabled: root.updater.canSaveCachedFirmware
                        onClicked: root.saveFirmwareRequested()
                    }

                    QQC2.Label {
                        Layout.fillWidth: true
                        visible: root.updater.hasCachedFirmware
                        text: "Cached: " + root.updater.cachedFirmwareSuggestedName
                        opacity: 0.78
                        wrapMode: Text.WordWrap
                    }
                }
            }
        }
    }

    // Option B: Local Firmware Image (.bin)
    Kirigami.AbstractCard {
        Layout.fillWidth: true

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.mediumSpacing

            QQC2.RadioButton {
                text: "Use a Local Firmware File (.bin)"
                font.bold: true
                checked: root.updater.useLocalFile
                enabled: !root.updater.isBusy
                onClicked: root.updater.setUseLocalFile(true)
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: Kirigami.Units.largeSpacing
                spacing: Kirigami.Units.smallSpacing

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.mediumSpacing

                    QQC2.Button {
                        text: "Choose File…"
                        icon.name: "document-open"
                        enabled: root.updater.canSelectFile
                        onClicked: {
                            root.updater.setUseLocalFile(true)
                            root.chooseLocalFileRequested()
                        }
                    }

                    QQC2.Label {
                        Layout.fillWidth: true
                        text: root.updater.hasLocalFirmware
                            ? (root.updater.localFirmwareName + " — version " + root.updater.localFirmwareVersion + " (" + root.updater.localFirmwareSize + ")")
                            : "No firmware file selected."
                        wrapMode: Text.WordWrap
                        opacity: 0.82
                    }
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: root.updater.localFirmwareError.length > 0
                    type: Kirigami.MessageType.Error
                    text: root.updater.localFirmwareError
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: root.updater.localFirmwareError.length === 0 && root.updater.localFirmwareWarning.length > 0
                    type: Kirigami.MessageType.Warning
                    text: root.updater.localFirmwareWarning
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: root.updater.hasLocalFirmware
                        && root.updater.localFirmwareCompatible
                        && root.updater.localFirmwareError.length === 0
                        && root.updater.localFirmwareWarning.length === 0
                    type: Kirigami.MessageType.Positive
                    text: "Verified Sony firmware image matches this controller."
                }
            }
        }
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
            text: "Review Update"
            icon.name: "go-next"
            highlighted: true
            enabled: root.updater.canReviewUpdate
            onClicked: root.updater.reviewUpdate()
        }
    }
}
