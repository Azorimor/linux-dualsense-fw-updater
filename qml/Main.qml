pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import dev.dualsense.updater

Kirigami.ApplicationWindow {
    id: root

    title: "DualSense Firmware Updater"
    width: 720
    height: 660
    minimumWidth: 580
    minimumHeight: 540

    pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.None

    DualSenseBridge {
        id: updater
    }

    FileDialog {
        id: firmwareFileDialog
        title: "Choose DualSense Firmware Image"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Firmware images (*.bin)", "All files (*)"]
        onAccepted: updater.selectLocalFile(selectedFile.toString())
    }

    pageStack.initialPage: Kirigami.ScrollablePage {
        id: mainPage
        title: "DualSense Firmware Updater"
        padding: Kirigami.Units.largeSpacing

        Item {
            implicitHeight: rootColumn.implicitHeight

            ColumnLayout {
                id: rootColumn
                width: Math.min(parent.width, 660)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Kirigami.Units.largeSpacing

            StepIndicator {
                Layout.fillWidth: true
                currentStepIndex: updater.stepIndex
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Heading {
                    Layout.fillWidth: true
                    level: 2
                    text: updater.stepTitle
                    wrapMode: Text.WordWrap
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    text: updater.stepSubtitle
                    wrapMode: Text.WordWrap
                    opacity: 0.78
                }
            }

            Kirigami.InlineMessage {
                id: statusBanner
                Layout.fillWidth: true
                visible: updater.hasBanner && updater.stepIndex !== 5
                text: updater.bannerDisplayText
                type: {
                    switch (updater.bannerSeverity) {
                    case 1:
                        return Kirigami.MessageType.Positive
                    case 2:
                        return Kirigami.MessageType.Warning
                    case 3:
                        return Kirigami.MessageType.Error
                    default:
                        return Kirigami.MessageType.Information
                    }
                }
            }

            // Inline activity bar for non-flashing background steps (scan, catalog check, file inspect)
            Kirigami.AbstractCard {
                Layout.fillWidth: true
                visible: updater.progressVisible && updater.stepIndex !== 3

                contentItem: ColumnLayout {
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.Label {
                        Layout.fillWidth: true
                        text: updater.statusTitle
                        font.bold: true
                        wrapMode: Text.WordWrap
                    }

                    QQC2.ProgressBar {
                        Layout.fillWidth: true
                        indeterminate: updater.progressIndeterminate
                        from: 0.0
                        to: 1.0
                        value: updater.progressIndeterminate ? 0.0 : updater.progressValue
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        visible: updater.canCancel

                        Item {
                            Layout.fillWidth: true
                        }

                        QQC2.Button {
                            text: "Cancel"
                            icon.name: "dialog-cancel"
                            enabled: updater.canCancel
                            onClicked: updater.cancelOperation()
                        }
                    }
                }
            }

            StackLayout {
                id: stepStack
                Layout.fillWidth: true
                currentIndex: updater.stepIndex

                // ---------------------------------------------------------
                // Step 0: Detect Controller
                // ---------------------------------------------------------
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.PlaceholderMessage {
                        Layout.fillWidth: true
                        Layout.topMargin: Kirigami.Units.largeSpacing
                        Layout.bottomMargin: Kirigami.Units.largeSpacing
                        visible: !updater.hasDevices && !updater.isBusy
                        icon.name: "input-gamepad"
                        text: "No DualSense Controller Found"
                        explanation: "Connect your controller directly to a USB port with a data cable. Firmware updates cannot run over Bluetooth."

                        QQC2.Button {
                            Layout.alignment: Qt.AlignHCenter
                            text: "Check Again"
                            icon.name: "view-refresh"
                            enabled: updater.canScan
                            onClicked: updater.scanControllers()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        visible: updater.hasDevices
                        spacing: Kirigami.Units.smallSpacing

                        QQC2.Label {
                            Layout.fillWidth: true
                            text: updater.deviceCount > 1
                                ? "Select the controller you want to update:"
                                : "Detected controller:"
                            font.bold: true
                        }

                        Repeater {
                            model: updater.deviceCount

                            Kirigami.AbstractCard {
                                id: deviceCard
                                required property int index
                                readonly property bool isSelected: index === updater.selectedDeviceIndex

                                Layout.fillWidth: true
                                showClickFeedback: updater.deviceCount > 1 && !updater.isBusy

                                contentItem: RowLayout {
                                    spacing: Kirigami.Units.mediumSpacing

                                    QQC2.RadioButton {
                                        checked: deviceCard.isSelected
                                        enabled: !updater.isBusy
                                        onClicked: updater.selectDeviceAt(deviceCard.index)
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
                                            text: updater.deviceLabels[deviceCard.index] ?? ""
                                            font.bold: true
                                            wrapMode: Text.WordWrap
                                        }

                                        QQC2.Label {
                                            Layout.fillWidth: true
                                            text: updater.deviceDetails[deviceCard.index] ?? ""
                                            opacity: 0.78
                                            wrapMode: Text.WordWrap
                                        }
                                    }
                                }

                                onClicked: {
                                    if (!updater.isBusy) {
                                        updater.selectDeviceAt(deviceCard.index)
                                    }
                                }
                            }
                        }

                        Kirigami.AbstractCard {
                            Layout.fillWidth: true
                            visible: updater.selectedDeviceLabel.length > 0

                            contentItem: Kirigami.FormLayout {
                                QQC2.Label {
                                    Kirigami.FormData.label: "Controller:"
                                    text: updater.selectedDeviceLabel
                                    wrapMode: Text.WordWrap
                                }
                                QQC2.Label {
                                    Kirigami.FormData.label: "Board Model:"
                                    visible: updater.boardModel.length > 0
                                    text: updater.boardModel
                                    font.bold: true
                                }
                                QQC2.Label {
                                    Kirigami.FormData.label: "Color:"
                                    visible: updater.colorName.length > 0
                                    text: updater.colorName
                                }
                                QQC2.Label {
                                    Kirigami.FormData.label: "Current Firmware:"
                                    text: updater.selectedDeviceFirmware
                                    font.bold: true
                                }
                                QQC2.Label {
                                    Kirigami.FormData.label: "Build Info:"
                                    visible: updater.selectedDeviceBuildInfo.length > 0
                                    text: updater.selectedDeviceBuildInfo
                                }
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Kirigami.Units.smallSpacing

                        QQC2.Button {
                            visible: updater.hasDevices
                            text: "Check Again"
                            icon.name: "view-refresh"
                            enabled: updater.canScan
                            onClicked: updater.scanControllers()
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        QQC2.Button {
                            text: "Continue"
                            icon.name: "go-next"
                            highlighted: true
                            enabled: updater.canContinueToFirmware
                            onClicked: updater.continueToFirmware()
                        }
                    }
                }

                // ---------------------------------------------------------
                // Step 1: Choose Update Source
                // ---------------------------------------------------------
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.AbstractCard {
                        Layout.fillWidth: true
                        visible: updater.boardModel.length > 0

                        contentItem: Kirigami.FormLayout {
                            QQC2.Label {
                                Kirigami.FormData.label: "Board Model:"
                                text: updater.boardModel
                                font.bold: true
                            }
                            QQC2.Label {
                                Kirigami.FormData.label: "Color:"
                                visible: updater.colorName.length > 0
                                text: updater.colorName
                            }
                            QQC2.Label {
                                Kirigami.FormData.label: "FW Series:"
                                visible: updater.fwSeriesText.length > 0
                                text: updater.fwSeriesText
                            }
                        }
                    }

                    Kirigami.InlineMessage {
                        Layout.fillWidth: true
                        visible: updater.hasUnsupportedBoardWarning && !updater.useLocalFile
                        type: Kirigami.MessageType.Warning
                        text: updater.unsupportedBoardWarning
                    }

                    // Option A: Official Sony Firmware
                    Kirigami.AbstractCard {
                        Layout.fillWidth: true

                        contentItem: ColumnLayout {
                            spacing: Kirigami.Units.mediumSpacing

                            QQC2.RadioButton {
                                text: "Download Official Sony Firmware (Recommended)"
                                font.bold: true
                                checked: !updater.useLocalFile
                                enabled: !updater.isBusy
                                onClicked: updater.setUseLocalFile(false)
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: Kirigami.Units.largeSpacing
                                spacing: Kirigami.Units.smallSpacing
                                enabled: !updater.useLocalFile && !updater.isBusy

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Choose your controller hardware model:"
                                    wrapMode: Text.WordWrap
                                }

                                QQC2.ComboBox {
                                    id: targetComboBox
                                    Layout.fillWidth: true
                                    enabled: updater.canSelectTarget
                                    model: updater.targetLabels
                                    currentIndex: updater.selectedTargetIndex
                                    onActivated: index => {
                                        updater.selectTargetAt(index)
                                        targetComboBox.currentIndex = Qt.binding(() => updater.selectedTargetIndex)
                                    }
                                    onModelChanged: targetComboBox.currentIndex = Qt.binding(() => updater.selectedTargetIndex)
                                }

                                Kirigami.InlineMessage {
                                    Layout.fillWidth: true
                                    visible: !updater.selectedTargetTested
                                    type: Kirigami.MessageType.Warning
                                    text: "DualSense Edge has not been hardware-tested with this utility. Proceed with caution."
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Kirigami.Units.mediumSpacing

                                    QQC2.Button {
                                        text: updater.catalogChecked ? "Refresh Version" : "Check for Update"
                                        icon.name: "download"
                                        enabled: updater.canCheckUpdates
                                        onClicked: {
                                            updater.setUseLocalFile(false)
                                            updater.checkForUpdates()
                                        }
                                    }

                                    QQC2.Label {
                                        Layout.fillWidth: true
                                        text: updater.catalogChecked
                                            ? ("Latest Sony firmware: " + updater.selectedTargetLatestVersion)
                                            : "Click \"Check for Update\" to check Sony's server."
                                        wrapMode: Text.WordWrap
                                        opacity: 0.82
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
                                checked: updater.useLocalFile
                                enabled: !updater.isBusy
                                onClicked: updater.setUseLocalFile(true)
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
                                        enabled: updater.canSelectFile
                                        onClicked: {
                                            updater.setUseLocalFile(true)
                                            firmwareFileDialog.open()
                                        }
                                    }

                                    QQC2.Label {
                                        Layout.fillWidth: true
                                        text: updater.hasLocalFirmware
                                            ? (updater.localFirmwareName + " — version " + updater.localFirmwareVersion + " (" + updater.localFirmwareSize + ")")
                                            : "No firmware file selected."
                                        wrapMode: Text.WordWrap
                                        opacity: 0.82
                                    }
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
                            enabled: updater.canGoBack
                            onClicked: updater.goBack()
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        QQC2.Button {
                            text: "Review Update"
                            icon.name: "go-next"
                            highlighted: true
                            enabled: updater.canReviewUpdate
                            onClicked: updater.reviewUpdate()
                        }
                    }
                }

                // ---------------------------------------------------------
                // Step 2: Review & Confirm
                // ---------------------------------------------------------
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.AbstractCard {
                        Layout.fillWidth: true
                        visible: updater.hasUpdateSummary

                        contentItem: Kirigami.FormLayout {
                            QQC2.Label {
                                Kirigami.FormData.label: "Controller:"
                                text: updater.summaryController
                                wrapMode: Text.WordWrap
                            }
                            QQC2.Label {
                                Kirigami.FormData.label: "Current Firmware:"
                                text: updater.summaryCurrentVersion
                            }
                            QQC2.Label {
                                Kirigami.FormData.label: "New Firmware:"
                                text: updater.summaryTargetVersion
                                font.bold: true
                            }
                            QQC2.Label {
                                Kirigami.FormData.label: "Source:"
                                text: updater.summarySource
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    Kirigami.InlineMessage {
                        Layout.fillWidth: true
                        visible: true
                        type: Kirigami.MessageType.Warning
                        text: updater.irreversibleWarning
                    }

                    Kirigami.InlineMessage {
                        Layout.fillWidth: true
                        visible: updater.summaryIsSameOrOlder
                        type: Kirigami.MessageType.Warning
                        text: "Your controller already has this firmware version (or newer). Continuing may stop with an error because downgrades are blocked by Sony's firmware."
                    }

                    Kirigami.InlineMessage {
                        Layout.fillWidth: true
                        visible: updater.summaryIsUntested
                        type: Kirigami.MessageType.Warning
                        text: "This hardware target has not been tested by the maintainers. Proceed only if you accept the risk."
                    }

                    QQC2.CheckBox {
                        Layout.fillWidth: true
                        checked: updater.warningAcknowledged
                        enabled: !updater.isBusy
                        text: "I understand that firmware updates cannot be reversed and I will keep the USB cable connected."
                        onToggled: updater.setWarningAcknowledged(checked)
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Kirigami.Units.smallSpacing

                        QQC2.Button {
                            text: "Back"
                            icon.name: "go-previous"
                            enabled: updater.canGoBack
                            onClicked: updater.goBack()
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        QQC2.Button {
                            text: "Start Update"
                            icon.name: "system-software-update"
                            highlighted: true
                            enabled: updater.canStartUpdate
                            onClicked: updater.startUpdate()
                        }
                    }
                }

                // ---------------------------------------------------------
                // Step 3: Updating (Progress)
                // ---------------------------------------------------------
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.AbstractCard {
                        Layout.fillWidth: true

                        contentItem: ColumnLayout {
                            spacing: Kirigami.Units.mediumSpacing

                            Kirigami.Heading {
                                Layout.fillWidth: true
                                level: 3
                                text: updater.statusTitle.length > 0
                                    ? updater.statusTitle
                                    : "Working…"
                                wrapMode: Text.WordWrap
                            }

                            QQC2.ProgressBar {
                                Layout.fillWidth: true
                                indeterminate: updater.progressIndeterminate
                                from: 0.0
                                to: 1.0
                                value: updater.progressIndeterminate ? 0.0 : updater.progressValue
                            }

                            QQC2.Label {
                                Layout.fillWidth: true
                                visible: updater.statusDetail.length > 0
                                text: updater.statusDetail
                                opacity: 0.82
                                wrapMode: Text.WordWrap
                            }

                            QQC2.Label {
                                Layout.fillWidth: true
                                text: updater.canCancel
                                    ? "Downloading from Sony's server. You can cancel safely before installation begins."
                                    : "Writing firmware to your controller. Do not unplug the USB cable or close this window."
                                opacity: 0.72
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true

                        Item {
                            Layout.fillWidth: true
                        }

                        QQC2.Button {
                            text: "Cancel"
                            icon.name: "dialog-cancel"
                            visible: updater.canCancel
                            enabled: updater.canCancel
                            onClicked: updater.cancelOperation()
                        }
                    }
                }

                // ---------------------------------------------------------
                // Step 4: Completed
                // ---------------------------------------------------------
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.PlaceholderMessage {
                        Layout.fillWidth: true
                        Layout.topMargin: Kirigami.Units.largeSpacing
                        Layout.bottomMargin: Kirigami.Units.largeSpacing
                        icon.name: "dialog-ok-apply"
                        text: "Firmware Update Complete"
                        explanation: "Your DualSense controller is now running firmware "
                            + updater.completedVersion
                            + ". You can safely unplug the USB cable and use your controller normally."

                        RowLayout {
                            Layout.alignment: Qt.AlignHCenter
                            spacing: Kirigami.Units.mediumSpacing

                            QQC2.Button {
                                text: "Update Another Controller"
                                icon.name: "view-refresh"
                                onClicked: updater.startOver()
                            }

                            QQC2.Button {
                                text: "Close"
                                icon.name: "window-close"
                                highlighted: true
                                onClicked: Qt.quit()
                            }
                        }
                    }
                }

                // ---------------------------------------------------------
                // Step 5: Error
                // ---------------------------------------------------------
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.InlineMessage {
                        Layout.fillWidth: true
                        visible: true
                        type: Kirigami.MessageType.Error
                        text: updater.errorTitle.length > 0
                            ? (updater.errorTitle + " — " + updater.errorMessage)
                            : updater.errorMessage
                    }

                    Kirigami.AbstractCard {
                        Layout.fillWidth: true
                        visible: updater.recoveryHint.length > 0

                        contentItem: ColumnLayout {
                            spacing: Kirigami.Units.smallSpacing

                            QQC2.Label {
                                Layout.fillWidth: true
                                text: "What to try next"
                                font.bold: true
                            }

                            QQC2.Label {
                                Layout.fillWidth: true
                                text: updater.recoveryHint
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Kirigami.Units.smallSpacing

                        QQC2.Button {
                            text: "Start Over"
                            icon.name: "edit-undo"
                            onClicked: updater.startOver()
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        QQC2.Button {
                            text: "Scan Controllers"
                            icon.name: "view-refresh"
                            enabled: updater.canScan
                            onClicked: updater.scanControllers()
                        }

                        QQC2.Button {
                            text: "Try Again"
                            icon.name: "dialog-ok"
                            highlighted: true
                            onClicked: updater.dismissError()
                        }
                    }
                }
            }
        }
        }
    }
}
