// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use common::{
    MockSonyHttp, NoopSleeper, SimDualSenseController, synthetic_firmware,
    synthetic_official_firmware,
};
use dualsense_core::{
    CancellationToken, CoreError, DEFAULT_PID, DEFAULT_VID, DualSenseUpdater,
    FirmwareCatalogClient, FirmwareImage, HidBusType, HidDeviceEnumerator, HidDeviceInfo,
    HidTransport, StartUpdateError, StartUpdateStatusCode, UpdateConfig, UpdateFailure,
    UpdatePhase, UpdateProgress, VerifyUpdateImageError, VerifyUpdateStatusCode,
    WriteUpdateImageError, WriteUpdateStatusCode,
};

struct MockEnumerator;

impl HidDeviceEnumerator for MockEnumerator {
    fn list_devices(&self, vid: u16, pid: u16) -> dualsense_core::Result<Vec<HidDeviceInfo>> {
        if vid == DEFAULT_VID && pid == DEFAULT_PID {
            Ok(vec![HidDeviceInfo {
                path: "/dev/hidraw-sim0".to_string(),
                vendor_id: vid,
                product_id: pid,
                interface_number: 3,
                usage_page: 0x0001,
                usage: 0x0005,
                product_string: "DualSense Wireless Controller".to_string(),
                serial_number: "00:11:22:33:44:55".to_string(),
                bus_type: HidBusType::Usb,
            }])
        } else {
            Ok(Vec::new())
        }
    }

    fn open(
        &self,
        _vid: u16,
        _pid: u16,
        path: Option<&str>,
    ) -> dualsense_core::Result<Box<dyn HidTransport>> {
        if let Some(p) = path
            && p != "/dev/hidraw-sim0"
        {
            return Err(CoreError::DevicePathNotMatched(p.to_string()));
        }
        Ok(Box::new(SimDualSenseController::new(0x0520)))
    }
}

#[test]
fn end_to_end_catalog_download_and_multi_block_update_succeeds() {
    // >= 256 KiB (8+ blocks) to verify official Sony header validation, 0x8000 splitting, and 0x39 chunking.
    let raw_fw = synthetic_official_firmware(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES + 513,
        DEFAULT_PID,
        0x000B,
        0x0630,
    );
    let catalog_client = FirmwareCatalogClient::new(MockSonyHttp {
        image_bytes: raw_fw.clone(),
    });
    let cancel = CancellationToken::new();

    let catalog = catalog_client
        .fetch_catalog(&cancel)
        .expect("fetch catalog");
    let release = catalog.release_for_target("000B").expect("target 000B");
    let mut dl_events = Vec::new();
    let image = catalog_client
        .download_firmware(release, &cancel, &mut |p| dl_events.push(p))
        .expect("download firmware");
    assert_eq!(dl_events.len(), 2);
    assert_eq!(image.version(), 0x0630);
    assert_eq!(image.block_count(), 9);

    let sim = SimDualSenseController::new(0x0520);
    let updater = DualSenseUpdater::with_sleeper_and_config(
        sim,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );

    let current = updater.read_firmware_info().expect("read firmware info");
    assert_eq!(current.firmware_version, 0x0520);
    assert_eq!(current.formatted_version(), "0x0520");

    let mut progress_log: Vec<UpdateProgress> = Vec::new();
    updater
        .run_full_update(&image, &cancel, |p| progress_log.push(p))
        .expect("full update succeeds");

    let sim_ref = updater.device().transport();
    assert_eq!(sim_ref.received_header.borrow().as_slice(), image.header());
    assert_eq!(sim_ref.received_image.borrow().as_slice(), image.as_bytes());
    assert!(*sim_ref.finalized.borrow());
    assert!(sim_ref.status_queue.borrow().is_empty());

    assert_eq!(progress_log.first().unwrap().phase, UpdatePhase::Starting);
    assert_eq!(progress_log.last().unwrap().phase, UpdatePhase::Completed);
    assert_eq!(progress_log.last().unwrap().percentage, 100.0);
    assert!(
        progress_log
            .windows(2)
            .all(|w| w[0].current_bytes <= w[1].current_bytes)
    );
}

#[test]
fn end_to_end_enumerator_opens_selected_device_and_rejects_unknown_path() {
    let enumerator = MockEnumerator;
    let devices = enumerator.list_dualsense_devices().expect("list devices");
    assert_eq!(devices.len(), 1);

    let transport = enumerator
        .open(DEFAULT_VID, DEFAULT_PID, Some(&devices[0].path))
        .expect("open known device");
    let updater = DualSenseUpdater::new(transport);
    let info = updater.read_firmware_info().expect("firmware info");
    assert_eq!(info.firmware_version, 0x0520);

    let bad = enumerator.open(DEFAULT_VID, DEFAULT_PID, Some("/dev/hidraw-missing"));
    assert!(matches!(bad, Err(CoreError::DevicePathNotMatched(_))));
}

#[test]
fn end_to_end_failure_paths_and_mid_transfer_cancellation() {
    let image = FirmwareImage::from_bytes(synthetic_firmware(1024, 0x0630)).expect("image");

    // 1. Downgrade error in StartUpdate
    let mut sim_downgrade = SimDualSenseController::new(0x0641);
    sim_downgrade.inject_start_error = Some(StartUpdateStatusCode::HeaderVersionCheckError);
    let updater = DualSenseUpdater::with_sleeper_and_config(
        sim_downgrade,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let err = updater
        .run_full_update(&image, &CancellationToken::new(), |_| {})
        .unwrap_err();
    assert_eq!(
        err,
        CoreError::UpdateFailed(UpdateFailure::StartUpdate(
            StartUpdateError::HeaderVersionCheckError
        ))
    );
    assert_eq!(
        err.user_message(),
        "Firmware image is not an upgrade; downgrades are not allowed."
    );

    // 2. Flash write failure mid-write
    let mut sim_write_err = SimDualSenseController::new(0x0520);
    sim_write_err.inject_write_error_at_chunk =
        Some((3, WriteUpdateStatusCode::WriteImageFlashWriteError));
    let updater_write_err = DualSenseUpdater::with_sleeper_and_config(
        sim_write_err,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let err_write = updater_write_err
        .run_full_update(&image, &CancellationToken::new(), |_| {})
        .unwrap_err();
    assert_eq!(
        err_write,
        CoreError::UpdateFailed(UpdateFailure::WriteUpdateImage(
            WriteUpdateImageError::WriteImageFlashWriteError
        ))
    );

    // 3. Body CMAC failure in VerifyUpdateImage
    let mut sim_verify_err = SimDualSenseController::new(0x0520);
    sim_verify_err.inject_verify_error = Some(VerifyUpdateStatusCode::VerifyFwBodyCmacCheckError);
    let updater_verify_err = DualSenseUpdater::with_sleeper_and_config(
        sim_verify_err,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let err_verify = updater_verify_err
        .run_full_update(&image, &CancellationToken::new(), |_| {})
        .unwrap_err();
    assert_eq!(
        err_verify,
        CoreError::UpdateFailed(UpdateFailure::VerifyUpdateImage(
            VerifyUpdateImageError::VerifyFwBodyCmacCheckError
        ))
    );
    assert!(!*updater_verify_err.device().transport().finalized.borrow());

    // 4. Mid-write cancellation triggered by progress callback
    let sim_cancel = SimDualSenseController::new(0x0520);
    let updater_cancel = DualSenseUpdater::with_sleeper_and_config(
        sim_cancel,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let err_cancel = updater_cancel
        .run_full_update(&image, &cancel, |p| {
            if p.phase == UpdatePhase::Writing && p.current_bytes > 0 {
                cancel_clone.cancel();
            }
        })
        .unwrap_err();
    assert_eq!(err_cancel, CoreError::Cancelled);
    assert!(!*updater_cancel.device().transport().finalized.borrow());
}

#[test]
fn end_to_end_catalog_download_rejects_corrupt_html_undersized_and_mismatched_images() {
    let cancel = CancellationToken::new();

    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "small HTML error page",
            b"<html><title>Error 502</title><body>Gateway error</body></html>".to_vec(),
        ),
        (
            "undersized Sony binary",
            synthetic_official_firmware(64 * 1024, DEFAULT_PID, 0x000B, 0x0630),
        ),
        (
            "random 256 KiB payload without Sony signature",
            synthetic_firmware(FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES, 0x0630),
        ),
        (
            "mismatched target ID",
            synthetic_official_firmware(
                FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
                DEFAULT_PID,
                0x0004,
                0x0630,
            ),
        ),
        (
            "mismatched USB PID",
            synthetic_official_firmware(
                FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
                0x0df2,
                0x000B,
                0x0630,
            ),
        ),
        (
            "mismatched firmware version",
            synthetic_official_firmware(
                FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
                DEFAULT_PID,
                0x000B,
                0x0641,
            ),
        ),
    ];

    for (label, bad_bytes) in cases {
        let client = FirmwareCatalogClient::new(MockSonyHttp {
            image_bytes: bad_bytes,
        });
        let catalog = client.fetch_catalog(&cancel).expect("fetch catalog");
        let release = catalog.release_for_target("000B").expect("target 000B");
        let err = client
            .download_firmware(release, &cancel, &mut |_| {})
            .expect_err(&format!("expected failure for {label}"));
        assert!(
            matches!(
                err,
                CoreError::InvalidDownloadedFirmware(_)
                    | CoreError::FirmwareTargetMismatch { .. }
                    | CoreError::FirmwareVersionMismatch { .. }
            ),
            "unexpected error for {label}: {err:?}"
        );
    }
}
