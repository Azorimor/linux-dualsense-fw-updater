use std::sync::Arc;
use std::time::Duration;

use dualsense_core::{
    CancellationToken, CoreError, DownloadProgress, DualSenseUpdater, FirmwareCatalogClient,
    FirmwareImage, FirmwareRelease, HidApiBackend, HidDeviceEnumerator, HidDeviceInfo, HttpClient,
    Sleeper, StdSleeper, UpdateConfig, decode_ds5_color, resolve_controller_target,
};
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::sync::{mpsc, watch};

use crate::http::UreqHttpClient;
use crate::viewmodel::{
    AppCommand, AppEvent, AppState, DeviceInspection, DeviceTarget, LocalFirmwareInfo, UiSnapshot,
};

/// Default interval between background USB HID device polls in desktop usage.
pub const DEFAULT_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(800);

/// Default concrete controller using live `hidapi`, `ureq` HTTPS, and `std::thread::sleep`.
pub type DefaultAppController = AppController<HidApiBackend, UreqHttpClient, StdSleeper>;

/// Default concrete runtime owning a background Tokio runtime and [`DefaultAppController`].
pub type DefaultAppRuntime = AppRuntime<HidApiBackend, UreqHttpClient, StdSleeper>;

#[derive(Clone)]
struct SharedHttpClient<H>(Arc<H>);

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

/// Async application controller running HID, network, and file I/O off the UI thread.
pub struct AppController<E, H, S> {
    event_tx: mpsc::UnboundedSender<AppEvent>,
    snapshot_rx: watch::Receiver<UiSnapshot>,
    _marker: std::marker::PhantomData<(E, H, S)>,
}

impl<E, H, S> Clone for AppController<E, H, S> {
    fn clone(&self) -> Self {
        Self {
            event_tx: self.event_tx.clone(),
            snapshot_rx: self.snapshot_rx.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<E, H, S> std::fmt::Debug for AppController<E, H, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppController").finish_non_exhaustive()
    }
}

impl DefaultAppController {
    /// Creates a production [`DefaultAppController`] attached to an existing Tokio `handle`.
    pub fn with_default_backends<F>(handle: &Handle, on_snapshot: F) -> Self
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_poll_interval(
            handle,
            HidApiBackend,
            UreqHttpClient::new(),
            StdSleeper,
            UpdateConfig::default(),
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            on_snapshot,
        )
    }
}

impl<E, H, S> AppController<E, H, S>
where
    E: HidDeviceEnumerator + Send + Sync + 'static,
    H: HttpClient + Send + Sync + 'static,
    S: Sleeper + Clone + Send + Sync + 'static,
{
    /// Spawns the controller actor on `handle` using the default 800 ms device poll interval.
    pub fn new<F>(
        handle: &Handle,
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        on_snapshot: F,
    ) -> Self
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_poll_interval(
            handle,
            enumerator,
            http,
            sleeper,
            update_config,
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            on_snapshot,
        )
    }

    /// Spawns the controller actor on `handle` with an explicit background `device_poll_interval`.
    pub fn with_poll_interval<F>(
        handle: &Handle,
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        device_poll_interval: Option<Duration>,
        on_snapshot: F,
    ) -> Self
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        let initial_state = AppState::new();
        let initial_snapshot = initial_state.snapshot();
        let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot.clone());
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let actor_tx = event_tx.clone();
        let on_snapshot: Arc<dyn Fn(UiSnapshot) + Send + Sync> = Arc::new(on_snapshot);
        on_snapshot(initial_snapshot);

        let ctx = ActorContext {
            handle: handle.clone(),
            enumerator: Arc::new(enumerator),
            http: Arc::new(http),
            sleeper,
            update_config,
            event_tx: actor_tx,
            snapshot_tx,
            on_snapshot,
        };

        handle.spawn(run_actor_loop(
            initial_state,
            event_rx,
            device_poll_interval,
            ctx,
        ));

        Self {
            event_tx,
            snapshot_rx,
            _marker: std::marker::PhantomData,
        }
    }

    /// Enqueues a user or system [`AppEvent`] without blocking the calling UI thread.
    pub fn dispatch(&self, event: AppEvent) {
        if self.event_tx.send(event).is_err() {
            log::debug!("AppController event dropped because actor loop stopped");
        }
    }

    /// Returns the latest [`UiSnapshot`] immediately.
    pub fn snapshot(&self) -> UiSnapshot {
        self.snapshot_rx.borrow().clone()
    }

    /// Subscribes to [`UiSnapshot`] changes using a Tokio `watch` receiver.
    pub fn subscribe(&self) -> watch::Receiver<UiSnapshot> {
        self.snapshot_rx.clone()
    }
}

/// Owned Tokio runtime + [`AppController`] wrapper for desktop GUI entry points and tests.
pub struct AppRuntime<E, H, S> {
    runtime: Option<Runtime>,
    controller: AppController<E, H, S>,
}

impl DefaultAppRuntime {
    /// Starts a Tokio runtime and [`DefaultAppController`] for live desktop usage.
    pub fn start<F>(on_snapshot: F) -> std::io::Result<Self>
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_backends_and_poll_interval(
            HidApiBackend,
            UreqHttpClient::new(),
            StdSleeper,
            UpdateConfig::default(),
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            on_snapshot,
        )
    }
}

impl<E, H, S> AppRuntime<E, H, S>
where
    E: HidDeviceEnumerator + Send + Sync + 'static,
    H: HttpClient + Send + Sync + 'static,
    S: Sleeper + Clone + Send + Sync + 'static,
{
    /// Starts a multi-threaded Tokio runtime with injected backends and default 800 ms polling.
    pub fn with_backends<F>(
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        on_snapshot: F,
    ) -> std::io::Result<Self>
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_backends_and_poll_interval(
            enumerator,
            http,
            sleeper,
            update_config,
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            on_snapshot,
        )
    }

    /// Starts a multi-threaded Tokio runtime with injected backends and custom `device_poll_interval`.
    pub fn with_backends_and_poll_interval<F>(
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        device_poll_interval: Option<Duration>,
        on_snapshot: F,
    ) -> std::io::Result<Self>
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        let runtime = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("dualsense-ui-worker")
            .enable_all()
            .build()?;
        let controller = AppController::with_poll_interval(
            runtime.handle(),
            enumerator,
            http,
            sleeper,
            update_config,
            device_poll_interval,
            on_snapshot,
        );
        Ok(Self {
            runtime: Some(runtime),
            controller,
        })
    }

    /// Returns a cloneable [`AppController`] handle.
    pub fn controller(&self) -> AppController<E, H, S> {
        self.controller.clone()
    }

    /// Dispatches an [`AppEvent`] to the background controller.
    pub fn dispatch(&self, event: AppEvent) {
        self.controller.dispatch(event);
    }

    /// Returns the latest [`UiSnapshot`].
    pub fn snapshot(&self) -> UiSnapshot {
        self.controller.snapshot()
    }

    /// Subscribes to [`UiSnapshot`] updates.
    pub fn subscribe(&self) -> watch::Receiver<UiSnapshot> {
        self.controller.subscribe()
    }

    /// Shuts down the owned Tokio runtime cleanly.
    pub fn shutdown(mut self) {
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_background();
        }
    }
}

impl<E, H, S> Drop for AppRuntime<E, H, S> {
    fn drop(&mut self) {
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_background();
        }
    }
}

struct ActorContext<E, H, S> {
    handle: Handle,
    enumerator: Arc<E>,
    http: Arc<H>,
    sleeper: S,
    update_config: UpdateConfig,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    snapshot_tx: watch::Sender<UiSnapshot>,
    on_snapshot: Arc<dyn Fn(UiSnapshot) + Send + Sync>,
}

async fn run_actor_loop<E, H, S>(
    mut state: AppState,
    mut event_rx: mpsc::UnboundedReceiver<AppEvent>,
    device_poll_interval: Option<Duration>,
    ctx: ActorContext<E, H, S>,
) where
    E: HidDeviceEnumerator + Send + Sync + 'static,
    H: HttpClient + Send + Sync + 'static,
    S: Sleeper + Clone + Send + Sync + 'static,
{
    let mut active_cancel = CancellationToken::new();
    let (poll_tx, mut poll_rx) = mpsc::unbounded_channel::<Option<Vec<HidDeviceInfo>>>();
    let mut poll_in_flight = false;
    let mut poll_timer = device_poll_interval.filter(|d| !d.is_zero()).map(|period| {
        let mut interval = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval
    });

    loop {
        tokio::select! {
            maybe_event = event_rx.recv() => {
                let Some(event) = maybe_event else {
                    break;
                };
                apply_event_and_commands(event, &mut state, &mut active_cancel, &ctx);
            }
            Some(poll_result) = poll_rx.recv() => {
                poll_in_flight = false;
                if let Some(polled_devices) = poll_result
                    && !state.is_flashing()
                    && state.poll_devices_changed(&polled_devices)
                {
                    apply_event_and_commands(
                        AppEvent::DevicesPollUpdated(polled_devices),
                        &mut state,
                        &mut active_cancel,
                        &ctx,
                    );
                }
            }
            _ = wait_poll_tick(poll_timer.as_mut()) => {
                if !poll_in_flight && !state.is_flashing() {
                    poll_in_flight = true;
                    let enumerator = Arc::clone(&ctx.enumerator);
                    let tx = poll_tx.clone();
                    ctx.handle.spawn(async move {
                        let res = tokio::task::spawn_blocking(move || {
                            enumerator.list_dualsense_devices()
                        })
                        .await;
                        let devices = match res {
                            Ok(Ok(devices)) => Some(devices),
                            Ok(Err(err)) => {
                                log::debug!("Background USB device poll failed: {err}");
                                None
                            }
                            Err(_) => {
                                log::debug!("Background USB device poll task failed");
                                None
                            }
                        };
                        let _ = tx.send(devices);
                    });
                }
            }
        }
    }
}

async fn wait_poll_tick(timer: Option<&mut tokio::time::Interval>) {
    match timer {
        Some(interval) => {
            interval.tick().await;
        }
        None => std::future::pending::<()>().await,
    }
}

fn apply_event_and_commands<E, H, S>(
    event: AppEvent,
    state: &mut AppState,
    active_cancel: &mut CancellationToken,
    ctx: &ActorContext<E, H, S>,
) where
    E: HidDeviceEnumerator + Send + Sync + 'static,
    H: HttpClient + Send + Sync + 'static,
    S: Sleeper + Clone + Send + Sync + 'static,
{
    let commands = state.update(event);
    let snapshot = state.snapshot();
    let _ = ctx.snapshot_tx.send(snapshot.clone());
    (ctx.on_snapshot)(snapshot);

    for command in commands {
        execute_command(command, active_cancel, ctx);
    }
}

fn execute_command<E, H, S>(
    command: AppCommand,
    active_cancel: &mut CancellationToken,
    ctx: &ActorContext<E, H, S>,
) where
    E: HidDeviceEnumerator + Send + Sync + 'static,
    H: HttpClient + Send + Sync + 'static,
    S: Sleeper + Clone + Send + Sync + 'static,
{
    match command {
        AppCommand::CancelActiveOperation => {
            active_cancel.cancel();
        }
        AppCommand::ScanDevices => {
            let cancel = fresh_token(active_cancel);
            let enumerator = Arc::clone(&ctx.enumerator);
            let tx = ctx.event_tx.clone();
            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    if cancel.is_cancelled() {
                        return Err(CoreError::Cancelled);
                    }
                    let devices = enumerator.list_dualsense_devices()?;
                    if cancel.is_cancelled() {
                        return Err(CoreError::Cancelled);
                    }
                    Ok(devices)
                })
                .await
                .unwrap_or_else(|_| Err(CoreError::Io("Device scan task failed.".to_string())));
                let _ = tx.send(AppEvent::DevicesScanned(res));
            });
        }
        AppCommand::ReadDeviceFirmwareInfo { device } => {
            let cancel = fresh_token(active_cancel);
            let enumerator = Arc::clone(&ctx.enumerator);
            let tx = ctx.event_tx.clone();
            let path = device.path.clone();
            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    if cancel.is_cancelled() {
                        return Err(CoreError::Cancelled);
                    }
                    let transport =
                        enumerator.open(device.vendor_id, device.product_id, Some(&device.path))?;
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
                    Ok(DeviceInspection {
                        firmware_info,
                        hardware,
                        serial_number,
                        color_name,
                    })
                })
                .await
                .unwrap_or_else(|_| {
                    Err(CoreError::Io("Firmware info read task failed.".to_string()))
                });
                let _ = tx.send(AppEvent::DeviceFirmwareInfoLoaded { path, result: res });
            });
        }
        AppCommand::FetchCatalog => {
            let cancel = fresh_token(active_cancel);
            let http = Arc::clone(&ctx.http);
            let tx = ctx.event_tx.clone();
            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let client = FirmwareCatalogClient::new(SharedHttpClient(http));
                    client.fetch_catalog(&cancel)
                })
                .await
                .unwrap_or_else(|_| {
                    Err(CoreError::CatalogHttp(
                        "Catalog fetch task stopped unexpectedly.".to_string(),
                    ))
                });
                let _ = tx.send(AppEvent::CatalogLoaded(res));
            });
        }
        AppCommand::InspectLocalFile { path } => {
            let cancel = fresh_token(active_cancel);
            let tx = ctx.event_tx.clone();
            let inspected_path = path.clone();
            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    if cancel.is_cancelled() {
                        return Err(CoreError::Cancelled);
                    }
                    let image = FirmwareImage::from_file(&path)?;
                    if cancel.is_cancelled() {
                        return Err(CoreError::Cancelled);
                    }
                    Ok(LocalFirmwareInfo {
                        version: image.version(),
                        size_bytes: image.len(),
                    })
                })
                .await
                .unwrap_or_else(|_| Err(CoreError::Io("File inspection task failed.".to_string())));
                let _ = tx.send(AppEvent::LocalFileInspected {
                    path: inspected_path,
                    result: res,
                });
            });
        }
        AppCommand::DownloadAndInstall { device, release } => {
            let cancel = fresh_token(active_cancel);
            let enumerator = Arc::clone(&ctx.enumerator);
            let http = Arc::clone(&ctx.http);
            let sleeper = ctx.sleeper.clone();
            let config = ctx.update_config.clone();
            let tx = ctx.event_tx.clone();
            let progress_tx = tx.clone();

            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    run_download_and_install(
                        &*enumerator,
                        http,
                        sleeper,
                        config,
                        &device,
                        &release,
                        &cancel,
                        &progress_tx,
                    )
                })
                .await
                .unwrap_or_else(|_| {
                    Err(CoreError::Io(
                        "Firmware update task stopped unexpectedly.".to_string(),
                    ))
                });
                let _ = tx.send(AppEvent::UpdateFinished(res));
            });
        }
        AppCommand::InstallLocalFirmware { device, path } => {
            let cancel = fresh_token(active_cancel);
            let enumerator = Arc::clone(&ctx.enumerator);
            let sleeper = ctx.sleeper.clone();
            let config = ctx.update_config.clone();
            let tx = ctx.event_tx.clone();
            let progress_tx = tx.clone();

            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    let image = FirmwareImage::from_file(&path)?;
                    flash_firmware_image(
                        &*enumerator,
                        sleeper,
                        config,
                        &device,
                        &image,
                        &cancel,
                        &progress_tx,
                    )
                })
                .await
                .unwrap_or_else(|_| {
                    Err(CoreError::Io(
                        "Firmware update task stopped unexpectedly.".to_string(),
                    ))
                });
                let _ = tx.send(AppEvent::UpdateFinished(res));
            });
        }
    }
}

fn fresh_token(slot: &mut CancellationToken) -> CancellationToken {
    let token = CancellationToken::new();
    *slot = token.clone();
    token
}

#[allow(clippy::too_many_arguments)]
fn run_download_and_install<E, H, S>(
    enumerator: &E,
    http: Arc<H>,
    sleeper: S,
    config: UpdateConfig,
    device: &DeviceTarget,
    release: &FirmwareRelease,
    cancel: &CancellationToken,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> dualsense_core::Result<u16>
where
    E: HidDeviceEnumerator + ?Sized,
    H: HttpClient,
    S: Sleeper,
{
    let client = FirmwareCatalogClient::new(SharedHttpClient(http));
    let image = client.download_firmware(release, cancel, &mut |progress| {
        let _ = tx.send(AppEvent::DownloadProgressUpdated(progress));
    })?;
    flash_firmware_image(enumerator, sleeper, config, device, &image, cancel, tx)
}

fn flash_firmware_image<E, S>(
    enumerator: &E,
    sleeper: S,
    config: UpdateConfig,
    device: &DeviceTarget,
    image: &FirmwareImage,
    cancel: &CancellationToken,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> dualsense_core::Result<u16>
where
    E: HidDeviceEnumerator + ?Sized,
    S: Sleeper,
{
    let transport = enumerator.open(device.vendor_id, device.product_id, Some(&device.path))?;
    let updater = DualSenseUpdater::with_sleeper_and_config(transport, sleeper, config);
    updater.run_full_update(image, cancel, |progress| {
        let _ = tx.send(AppEvent::UpdateProgressUpdated(progress));
    })?;
    Ok(image.version())
}
