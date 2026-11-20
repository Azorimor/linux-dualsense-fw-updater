// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.azorimor.linux_dualsense_fw_updater

Kirigami.ApplicationWindow {
    id: root

    title: updater.noFlashActive
        ? "DualSense Firmware Updater [Simulation: --no-flash]"
        : "DualSense Firmware Updater"
    width: 720
    height: 660
    minimumWidth: 580
    minimumHeight: 540

    pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.None

    onClosing: close => {
        if (updater.isFlashing) {
            close.accepted = false
            flashingCloseWarningDialog.open()
        }
    }

    DualSenseBridge {
        id: updater
    }

    LicensesWindow {
        id: licensesWindow
        licensesText: updater.licensesText
    }

    AboutWindow {
        id: aboutWindow
        appVersion: updater.appVersion
        githubUrl: updater.githubUrl
        githubPagesUrl: updater.githubPagesUrl
        githubIssuesUrl: updater.githubIssuesUrl
        onOpenLicensesRequested: {
            licensesWindow.show()
            licensesWindow.raise()
            licensesWindow.requestActivate()
        }
    }

    FlashReportWindow {
        id: flashReportWindow
        reportTitle: updater.flashReportTitle
        reportMarkdown: updater.flashReportMarkdown
        issueUrl: updater.flashReportIssueUrl
        issuesUrl: updater.githubIssuesUrl
        reportSucceeded: updater.flashReportSucceeded
    }

    QQC2.Dialog {
        id: flashingCloseWarningDialog
        title: "Firmware Update in Progress"
        modal: true
        parent: QQC2.Overlay.overlay
        anchors.centerIn: parent
        standardButtons: QQC2.Dialog.Ok

        QQC2.Label {
            width: Math.min(root.width - Kirigami.Units.largeSpacing * 4, 420)
            text: "Firmware is currently being written to your DualSense controller. Do not close this window or unplug the USB cable until the update finishes."
            wrapMode: Text.WordWrap
        }
    }

    menuBar: QQC2.MenuBar {
        QQC2.Menu {
            title: "File"

            QQC2.MenuItem {
                text: "Scan Controllers"
                enabled: updater.canScan
                onTriggered: updater.scanControllers()
            }

            QQC2.MenuSeparator {}

            QQC2.MenuItem {
                text: "Quit"
                enabled: !updater.isFlashing
                onTriggered: root.close()
            }
        }

        QQC2.Menu {
            title: "Help"

            QQC2.MenuItem {
                text: "Licenses"
                onTriggered: {
                    licensesWindow.show()
                    licensesWindow.raise()
                    licensesWindow.requestActivate()
                }
            }

            QQC2.MenuItem {
                text: "About Unofficial DualSense Firmware Updater"
                onTriggered: {
                    aboutWindow.show()
                    aboutWindow.raise()
                    aboutWindow.requestActivate()
                }
            }
        }
    }

    FileDialog {
        id: firmwareFileDialog
        title: "Choose DualSense Firmware Image"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Firmware images (*.bin)", "All files (*)"]
        onAccepted: updater.selectLocalFile(selectedFile.toString())
    }

    FileDialog {
        id: saveFirmwareFileDialog
        title: "Save Firmware Copy"
        fileMode: FileDialog.SaveFile
        nameFilters: ["Firmware images (*.bin)", "All files (*)"]
        onAccepted: updater.saveCachedFirmware(selectedFile.toString())
    }

    Connections {
        target: updater
        function onStepIndexChanged() {
            if (updater.stepIndex !== 1 && firmwareFileDialog.visible) {
                firmwareFileDialog.close()
            }
            if (updater.stepIndex === 3 && saveFirmwareFileDialog.visible) {
                saveFirmwareFileDialog.close()
            }
        }
    }

    pageStack.initialPage: Kirigami.ScrollablePage {
        id: mainPage
        title: updater.noFlashActive
        ? "DualSense Firmware Updater [Simulation: --no-flash]"
        : "DualSense Firmware Updater"
        padding: Kirigami.Units.largeSpacing

        Item {
            implicitHeight: rootColumn.implicitHeight

            ColumnLayout {
                id: rootColumn
                width: Math.min(parent.width, 660)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Kirigami.Units.largeSpacing

                Kirigami.InlineMessage {
                    id: simulationBanner
                    Layout.fillWidth: true
                    visible: updater.noFlashActive
                    type: Kirigami.MessageType.Information
                    text: "Simulation mode (--no-flash) is active. No firmware will be flashed to real hardware."
                }

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
                    // Kirigami's BasicTheme fallback binds activeTextColor to highlightedText (#ffffff in Fusion).
                    Kirigami.Theme.activeTextColor: Qt.colorEqual(
                            root.Kirigami.Theme.activeTextColor,
                            root.Kirigami.Theme.highlightedTextColor)
                        ? root.Kirigami.Theme.highlightColor
                        : root.Kirigami.Theme.activeTextColor
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

                    // Step 0: Detect Controller
                    DetectControllerStep {
                        Layout.fillWidth: true
                        updater: updater
                    }

                    // Step 1: Choose Update Source
                    SelectFirmwareStep {
                        Layout.fillWidth: true
                        updater: updater
                        onChooseLocalFileRequested: firmwareFileDialog.open()
                        onSaveFirmwareRequested: saveFirmwareFileDialog.open()
                    }

                    // Step 2: Review & Confirm
                    ConfirmUpdateStep {
                        Layout.fillWidth: true
                        updater: updater
                    }

                    // Step 3: Updating (Progress)
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

                    // Step 4: Completed
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
                                    onClicked: root.close()
                                }
                            }
                        }

                        Kirigami.InlineMessage {
                            Layout.fillWidth: true
                            visible: updater.completedVerificationNote.length > 0
                            type: updater.postFlashVerified
                                ? Kirigami.MessageType.Positive
                                : Kirigami.MessageType.Warning
                            text: updater.completedVerificationNote
                        }

                        Kirigami.AbstractCard {
                            Layout.fillWidth: true
                            visible: updater.canSaveCachedFirmware

                            contentItem: RowLayout {
                                spacing: Kirigami.Units.mediumSpacing

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Optional: Save a copy of "
                                        + updater.cachedFirmwareSuggestedName
                                        + " for updating another controller offline."
                                    wrapMode: Text.WordWrap
                                    opacity: 0.82
                                }

                                QQC2.Button {
                                    text: "Save Firmware Copy…"
                                    icon.name: "document-save-as"
                                    enabled: updater.canSaveCachedFirmware
                                    onClicked: saveFirmwareFileDialog.open()
                                }
                            }
                        }

                        Kirigami.AbstractCard {
                            Layout.fillWidth: true
                            visible: updater.hasFlashReport

                            contentItem: ColumnLayout {
                                spacing: Kirigami.Units.smallSpacing

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Optional: Share Flash Result on GitHub"
                                    font.bold: true
                                }

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Open a prefilled GitHub issue with anonymized controller, firmware, and OS metrics (no serial numbers). You can review and edit everything before submitting."
                                    wrapMode: Text.WordWrap
                                    opacity: 0.82
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Kirigami.Units.smallSpacing

                                    QQC2.Button {
                                        text: "Flash Run Infos"
                                        icon.name: "document-properties"
                                        onClicked: {
                                            flashReportWindow.show()
                                            flashReportWindow.raise()
                                            flashReportWindow.requestActivate()
                                        }
                                    }

                                    QQC2.Button {
                                        text: "Open GitHub Issue"
                                        icon.name: "internet-web-browser"
                                        enabled: updater.flashReportIssueUrl.length > 0
                                        onClicked: Qt.openUrlExternally(updater.flashReportIssueUrl)
                                    }
                                }
                            }
                        }
                    }

                    // Step 5: Error
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

                        Kirigami.AbstractCard {
                            Layout.fillWidth: true
                            visible: updater.hasFlashReport

                            contentItem: ColumnLayout {
                                spacing: Kirigami.Units.smallSpacing

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Optional: Report Flash Attempt on GitHub"
                                    font.bold: true
                                }

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Open a prefilled GitHub issue with anonymized controller, firmware, error, and OS metrics (no serial numbers) so maintainers can investigate compatibility."
                                    wrapMode: Text.WordWrap
                                    opacity: 0.82
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Kirigami.Units.smallSpacing

                                    QQC2.Button {
                                        text: "Flash Run Infos"
                                        icon.name: "document-properties"
                                        onClicked: {
                                            flashReportWindow.show()
                                            flashReportWindow.raise()
                                            flashReportWindow.requestActivate()
                                        }
                                    }

                                    QQC2.Button {
                                        text: "Open GitHub Issue"
                                        icon.name: "internet-web-browser"
                                        enabled: updater.flashReportIssueUrl.length > 0
                                        onClicked: Qt.openUrlExternally(updater.flashReportIssueUrl)
                                    }
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
