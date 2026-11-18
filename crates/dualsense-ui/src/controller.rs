// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dualsense_core::{
    CancellationToken, CoreError, FirmwareCatalogClient, FirmwareImage, HidApiBackend,
    HidDeviceEnumerator, HidDeviceInfo, HttpClient, Sleeper, StdSleeper, UpdateConfig,
};
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::sync::{mpsc, watch};

use crate::controller_tasks::{
    FirmwareCache, SharedHttpClient, flash_firmware_image, inspect_local_file,
    read_device_inspection, run_download_and_install, save_cached_firmware_to_path,
};
use crate::http::UreqHttpClient;
use crate::no_flash::{NoFlashEnumerator, NoFlashOutcome, SimulatedDeviceState};
use crate::viewmodel::{AppCommand, AppEvent, AppState, UiSnapshot};

/// Default interval between background USB HID device polls in desktop usage.
pub const DEFAULT_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(800);

/// Default concrete controller using live `hidapi`, `ureq` HTTPS, and `std::thread::sleep`.
pub type DefaultAppController = AppController<HidApiBackend, UreqHttpClient, StdSleeper>;

/// Default concrete runtime owning a background Tokio runtime and [`DefaultAppController`].
pub type DefaultAppRuntime = AppRuntime<HidApiBackend, UreqHttpClient, StdSleeper>;

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
        Self::with_no_flash(handle, None, on_snapshot)
    }

    /// Creates a production [`DefaultAppController`] with optional `--no-flash` simulation.
    pub fn with_no_flash<F>(
        handle: &Handle,
        no_flash: Option<NoFlashOutcome>,
        on_snapshot: F,
    ) -> Self
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_poll_interval_and_no_flash(
            handle,
            HidApiBackend,
            UreqHttpClient::new(),
            StdSleeper,
            UpdateConfig::default(),
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            no_flash,
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
        Self::with_poll_interval_and_no_flash(
            handle,
            enumerator,
            http,
            sleeper,
            update_config,
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            None,
            on_snapshot,
        )
    }

    /// Spawns the controller actor on `handle` with `--no-flash` simulation support and 800 ms polling.
    pub fn new_with_no_flash<F>(
        handle: &Handle,
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        no_flash: Option<NoFlashOutcome>,
        on_snapshot: F,
    ) -> Self
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_poll_interval_and_no_flash(
            handle,
            enumerator,
            http,
            sleeper,
            update_config,
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            no_flash,
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
        Self::with_poll_interval_and_no_flash(
            handle,
            enumerator,
            http,
            sleeper,
            update_config,
            device_poll_interval,
            None,
            on_snapshot,
        )
    }

    /// Spawns the controller actor on `handle` with explicit `device_poll_interval` and optional `--no-flash`.
    #[allow(clippy::too_many_arguments)]
    pub fn with_poll_interval_and_no_flash<F>(
        handle: &Handle,
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        device_poll_interval: Option<Duration>,
        no_flash: Option<NoFlashOutcome>,
        on_snapshot: F,
    ) -> Self
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        let initial_state = AppState::with_no_flash(no_flash.is_some());
        let initial_snapshot = initial_state.snapshot();
        let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot.clone());
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let actor_tx = event_tx.clone();
        let on_snapshot: Arc<dyn Fn(UiSnapshot) + Send + Sync> = Arc::new(on_snapshot);
        on_snapshot(initial_snapshot);

        let enumerator = match no_flash {
            Some(_) => Arc::new(RuntimeEnumerator::NoFlash(Arc::new(
                NoFlashEnumerator::new(enumerator),
            ))),
            None => Arc::new(RuntimeEnumerator::Live(Arc::new(enumerator))),
        };

        let ctx = ActorContext {
            handle: handle.clone(),
            enumerator,
            http: Arc::new(http),
            firmware_cache: Arc::new(Mutex::new(FirmwareCache::default())),
            sleeper,
            update_config,
            no_flash,
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
        Self::start_with_no_flash(None, on_snapshot)
    }

    /// Starts a Tokio runtime and [`DefaultAppController`] with optional `--no-flash` simulation.
    pub fn start_with_no_flash<F>(
        no_flash: Option<NoFlashOutcome>,
        on_snapshot: F,
    ) -> std::io::Result<Self>
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_backends_poll_interval_and_no_flash(
            HidApiBackend,
            UreqHttpClient::new(),
            StdSleeper,
            UpdateConfig::default(),
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            no_flash,
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
        Self::with_backends_poll_interval_and_no_flash(
            enumerator,
            http,
            sleeper,
            update_config,
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            None,
            on_snapshot,
        )
    }

    /// Starts a multi-threaded Tokio runtime with injected backends, optional `--no-flash`, and 800 ms polling.
    pub fn with_backends_and_no_flash<F>(
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        no_flash: Option<NoFlashOutcome>,
        on_snapshot: F,
    ) -> std::io::Result<Self>
    where
        F: Fn(UiSnapshot) + Send + Sync + 'static,
    {
        Self::with_backends_poll_interval_and_no_flash(
            enumerator,
            http,
            sleeper,
            update_config,
            Some(DEFAULT_DEVICE_POLL_INTERVAL),
            no_flash,
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
        Self::with_backends_poll_interval_and_no_flash(
            enumerator,
            http,
            sleeper,
            update_config,
            device_poll_interval,
            None,
            on_snapshot,
        )
    }

    /// Starts a multi-threaded Tokio runtime with injected backends, custom `device_poll_interval`, and optional `--no-flash`.
    pub fn with_backends_poll_interval_and_no_flash<F>(
        enumerator: E,
        http: H,
        sleeper: S,
        update_config: UpdateConfig,
        device_poll_interval: Option<Duration>,
        no_flash: Option<NoFlashOutcome>,
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
        let controller = AppController::with_poll_interval_and_no_flash(
            runtime.handle(),
            enumerator,
            http,
            sleeper,
            update_config,
            device_poll_interval,
            no_flash,
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

enum RuntimeEnumerator<E> {
    Live(Arc<E>),
    NoFlash(Arc<NoFlashEnumerator<E>>),
}

impl<E: HidDeviceEnumerator> RuntimeEnumerator<E> {
    fn simulated_state(&self) -> Option<Arc<SimulatedDeviceState>> {
        match self {
            Self::Live(_) => None,
            Self::NoFlash(sim) => Some(sim.simulated_state()),
        }
    }
}

impl<E: HidDeviceEnumerator> HidDeviceEnumerator for RuntimeEnumerator<E> {
    fn list_devices(&self, vid: u16, pid: u16) -> dualsense_core::Result<Vec<HidDeviceInfo>> {
        match self {
            Self::Live(inner) => inner.list_devices(vid, pid),
            Self::NoFlash(sim) => sim.list_devices(vid, pid),
        }
    }

    fn open(
        &self,
        vid: u16,
        pid: u16,
        path: Option<&str>,
    ) -> dualsense_core::Result<Box<dyn dualsense_core::HidTransport>> {
        match self {
            Self::Live(inner) => inner.open(vid, pid, path),
            Self::NoFlash(sim) => sim.open(vid, pid, path),
        }
    }

    fn list_dualsense_devices(&self) -> dualsense_core::Result<Vec<HidDeviceInfo>> {
        match self {
            Self::Live(inner) => inner.list_dualsense_devices(),
            Self::NoFlash(sim) => sim.list_dualsense_devices(),
        }
    }
}

struct ActorContext<E, H, S> {
    handle: Handle,
    enumerator: Arc<RuntimeEnumerator<E>>,
    http: Arc<H>,
    firmware_cache: Arc<Mutex<FirmwareCache>>,
    sleeper: S,
    update_config: UpdateConfig,
    no_flash: Option<NoFlashOutcome>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    snapshot_tx: watch::Sender<UiSnapshot>,
    on_snapshot: Arc<dyn Fn(UiSnapshot) + Send + Sync>,
}

#[derive(Default)]
struct CancelTokens {
    operation: CancellationToken,
    catalog: CancellationToken,
    inspections: HashMap<String, CancellationToken>,
}

impl CancelTokens {
    fn cancel_all(&mut self) {
        self.operation.cancel();
        self.catalog.cancel();
        for token in self.inspections.values() {
            token.cancel();
        }
        self.inspections.clear();
    }
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
    let mut tokens = CancelTokens::default();
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
                apply_event_and_commands(event, &mut state, &mut tokens, &ctx);
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
                        &mut tokens,
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
    tokens: &mut CancelTokens,
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
        execute_command(command, tokens, ctx);
    }
}

fn execute_command<E, H, S>(
    command: AppCommand,
    tokens: &mut CancelTokens,
    ctx: &ActorContext<E, H, S>,
) where
    E: HidDeviceEnumerator + Send + Sync + 'static,
    H: HttpClient + Send + Sync + 'static,
    S: Sleeper + Clone + Send + Sync + 'static,
{
    match command {
        AppCommand::CancelActiveOperation => {
            tokens.cancel_all();
        }
        AppCommand::ScanDevices => {
            let cancel = fresh_token(&mut tokens.operation);
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
            let cancel = CancellationToken::new();
            tokens
                .inspections
                .insert(device.path.clone(), cancel.clone());
            let enumerator = Arc::clone(&ctx.enumerator);
            let tx = ctx.event_tx.clone();
            let path = device.path.clone();
            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    read_device_inspection(&*enumerator, &device, &cancel)
                })
                .await
                .unwrap_or_else(|_| {
                    Err(CoreError::Io("Firmware info read task failed.".to_string()))
                });
                let _ = tx.send(AppEvent::DeviceFirmwareInfoLoaded { path, result: res });
            });
        }
        AppCommand::FetchCatalog => {
            let cancel = fresh_token(&mut tokens.catalog);
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
            let cancel = fresh_token(&mut tokens.operation);
            let tx = ctx.event_tx.clone();
            let inspected_path = path.clone();
            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || inspect_local_file(&path, &cancel))
                    .await
                    .unwrap_or_else(|_| {
                        Err(CoreError::Io("File inspection task failed.".to_string()))
                    });
                let _ = tx.send(AppEvent::LocalFileInspected {
                    path: inspected_path,
                    result: res,
                });
            });
        }
        AppCommand::DownloadAndInstall { device, release } => {
            let cancel = fresh_token(&mut tokens.operation);
            let enumerator = Arc::clone(&ctx.enumerator);
            let http = Arc::clone(&ctx.http);
            let firmware_cache = Arc::clone(&ctx.firmware_cache);
            let sleeper = ctx.sleeper.clone();
            let config = ctx.update_config.clone();
            let no_flash = ctx.no_flash;
            let simulated_state = enumerator.simulated_state();
            let tx = ctx.event_tx.clone();
            let progress_tx = tx.clone();

            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    run_download_and_install(
                        &*enumerator,
                        http,
                        &firmware_cache,
                        sleeper,
                        config,
                        &device,
                        &release,
                        &cancel,
                        &progress_tx,
                        no_flash,
                        simulated_state.as_deref(),
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
            let cancel = fresh_token(&mut tokens.operation);
            let enumerator = Arc::clone(&ctx.enumerator);
            let sleeper = ctx.sleeper.clone();
            let config = ctx.update_config.clone();
            let no_flash = ctx.no_flash;
            let simulated_state = enumerator.simulated_state();
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
                        no_flash,
                        simulated_state.as_deref(),
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
        AppCommand::SaveCachedFirmware {
            target_id,
            version,
            path,
        } => {
            let firmware_cache = Arc::clone(&ctx.firmware_cache);
            let tx = ctx.event_tx.clone();
            let save_path = path.clone();
            ctx.handle.spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    save_cached_firmware_to_path(&firmware_cache, &target_id, version, &path)
                })
                .await
                .unwrap_or_else(|_| {
                    Err(CoreError::Io(
                        "Save firmware task stopped unexpectedly.".to_string(),
                    ))
                });
                let _ = tx.send(AppEvent::CachedFirmwareSaved {
                    path: save_path,
                    result: res,
                });
            });
        }
    }
}

fn fresh_token(slot: &mut CancellationToken) -> CancellationToken {
    let token = CancellationToken::new();
    *slot = token.clone();
    token
}
