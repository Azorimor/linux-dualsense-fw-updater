// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use dualsense_core::{
    CancellationToken, CoreError, DownloadProgress, DualSenseUpdater, FirmwareCatalogClient,
    FirmwareImage, FirmwareRelease, HidDeviceEnumerator, HttpClient, Sleeper, UpdateConfig,
    decode_ds5_color, resolve_controller_target,
};
use tokio::sync::mpsc;

use crate::viewmodel::{
    AppEvent, DeviceInspection, DeviceTarget, LocalFirmwareInfo, UpdateOutcome,
};

#[derive(Clone)]
pub(crate) struct SharedHttpClient<H>(pub(crate) Arc<H>);

impl<H: HttpClient> HttpClient for SharedHttpClient<H> {
    fn get_text(&self, url: &str, cancel: &CancellationToken) -> dualsense_core::Result<String> {
        self.0.get_text(url, cancel)
    }

    fn download_with_progress(
        &self,
        url: &str,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> dualsense_core::Result<Vec<u8>> {
        self.0.download_with_progress(url, cancel, on_progress)
    }
}

#[derive(Debug, Default)]
pub(crate) struct FirmwareCache {
    entries: HashMap<(String, u16), Arc<FirmwareImage>>,
}

impl FirmwareCache {
    pub(crate) fn get(&self, target_id: &str, version: u16) -> Option<Arc<FirmwareImage>> {
        self.entries
            .get(&(target_id.to_ascii_uppercase(), version))
            .cloned()
    }

    pub(crate) fn insert(&mut self, target_id: &str, version: u16, image: Arc<FirmwareImage>) {
        self.entries
            .insert((target_id.to_ascii_uppercase(), version), image);
    }
}

pub(crate) fn read_device_inspection<E>(
    enumerator: &E,
    device: &DeviceTarget,
    cancel: &CancellationToken,
) -> dualsense_core::Result<DeviceInspection>
where
    E: HidDeviceEnumerator + ?Sized,
{
    if cancel.is_cancelled() {
        return Err(CoreError::Cancelled);
    }
    let transport = enumerator.open(device.vendor_id, device.product_id, Some(&device.path))?;
    if cancel.is_cancelled() {
        return Err(CoreError::Cancelled);
    }
    let updater = DualSenseUpdater::new(transport);
    let firmware_info = updater.read_firmware_info()?;
    let hardware = resolve_controller_target(&firmware_info, device.product_id);
    let serial_number = updater.try_read_serial_number();
    let color_name = serial_number
        .as_deref()
        .and_then(decode_ds5_color)
        .map(ToString::to_string);
    let battery_status = updater.try_read_battery_status();
    Ok(DeviceInspection {
        firmware_info,
        hardware,
        serial_number,
        color_name,
        battery_status,
    })
}

pub(crate) fn inspect_local_file(
    path: &Path,
    cancel: &CancellationToken,
) -> dualsense_core::Result<LocalFirmwareInfo> {
    if cancel.is_cancelled() {
        return Err(CoreError::Cancelled);
    }
    let image = FirmwareImage::from_file(path)?;
    if cancel.is_cancelled() {
        return Err(CoreError::Cancelled);
    }
    Ok(LocalFirmwareInfo::from_image(&image))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_download_and_install<E, H, S>(
    enumerator: &E,
    http: Arc<H>,
    firmware_cache: &Mutex<FirmwareCache>,
    sleeper: S,
    config: UpdateConfig,
    device: &DeviceTarget,
    release: &FirmwareRelease,
    cancel: &CancellationToken,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> dualsense_core::Result<UpdateOutcome>
where
    E: HidDeviceEnumerator + ?Sized,
    H: HttpClient,
    S: Sleeper,
{
    let cached = firmware_cache
        .lock()
        .expect("lock firmware cache")
        .get(&release.target_id, release.version);

    let image = if let Some(cached_image) = cached {
        let len = cached_image.len();
        let _ = tx.send(AppEvent::DownloadProgressUpdated(DownloadProgress {
            downloaded_bytes: len,
            total_bytes: Some(len),
        }));
        let _ = tx.send(AppEvent::OfficialFirmwareCached {
            target_id: release.target_id.to_ascii_uppercase(),
            version: release.version,
        });
        cached_image
    } else {
        let client = FirmwareCatalogClient::new(SharedHttpClient(http));
        let downloaded = client.download_firmware(release, cancel, &mut |progress| {
            let _ = tx.send(AppEvent::DownloadProgressUpdated(progress));
        })?;
        let arc_image = Arc::new(downloaded);
        firmware_cache.lock().expect("lock firmware cache").insert(
            &release.target_id,
            release.version,
            Arc::clone(&arc_image),
        );
        let _ = tx.send(AppEvent::OfficialFirmwareCached {
            target_id: release.target_id.to_ascii_uppercase(),
            version: release.version,
        });
        arc_image
    };

    flash_firmware_image(enumerator, sleeper, config, device, &image, cancel, tx)
}

pub(crate) fn flash_firmware_image<E, S>(
    enumerator: &E,
    sleeper: S,
    config: UpdateConfig,
    device: &DeviceTarget,
    image: &FirmwareImage,
    cancel: &CancellationToken,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> dualsense_core::Result<UpdateOutcome>
where
    E: HidDeviceEnumerator + ?Sized,
    S: Sleeper,
{
    let transport = enumerator.open(device.vendor_id, device.product_id, Some(&device.path))?;
    let updater = DualSenseUpdater::with_sleeper_and_config(transport, sleeper, config);
    updater.run_full_update(image, cancel, |progress| {
        let _ = tx.send(AppEvent::UpdateProgressUpdated(progress));
    })?;
    let verification = updater.verify_post_flash_firmware(image.version());
    Ok(UpdateOutcome::new(image.version(), verification))
}

pub(crate) fn save_cached_firmware_to_path(
    firmware_cache: &Mutex<FirmwareCache>,
    target_id: &str,
    version: u16,
    path: &Path,
) -> dualsense_core::Result<()> {
    let image = firmware_cache
        .lock()
        .expect("lock firmware cache")
        .get(target_id, version)
        .ok_or_else(|| {
            CoreError::Io("Cached official firmware image is not available in memory.".to_string())
        })?;
    std::fs::write(path, image.as_bytes())?;
    Ok(())
}
