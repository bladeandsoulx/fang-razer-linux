//! Slow, non-thermal peripherals isolated from the EC control core.
//!
//! DDC/CI and GPU switching call external programs. They run on blocking
//! workers behind their own locks, while the 1 Hz thermal task owns only the
//! EC core. A cached snapshot keeps status assembly non-blocking.

use crate::ddc::{self, Ddc};
use crate::gpu::{self, GpuSwitch};
use fang_protocol::api::{ColorPreset, GpuMode};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeripheralSnapshot {
    pub color_ddc: bool,
    pub color_presets: Vec<ColorPreset>,
    pub color_current: Option<u8>,
    pub monitor_brightness: Option<u8>,
    pub gpu_mode: Option<GpuMode>,
    pub gpu_mode_pending: bool,
}

pub type SnapshotStore = Arc<RwLock<PeripheralSnapshot>>;

pub fn snapshot_store() -> SnapshotStore {
    Arc::new(RwLock::new(PeripheralSnapshot::default()))
}

pub fn read_snapshot(store: &SnapshotStore) -> PeripheralSnapshot {
    store
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn write_snapshot(store: &SnapshotStore, next: PeripheralSnapshot) {
    *store
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = next;
}

fn apply_ddc_snapshot(next: &mut PeripheralSnapshot, ddc: &Ddc) {
    next.color_ddc = ddc.available();
    next.color_presets = ddc.presets();
    next.color_current = ddc.current();
    next.monitor_brightness = ddc.brightness();
}

fn sync_ddc_snapshot(store: &SnapshotStore, ddc: &Ddc) -> bool {
    let mut current = store
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let before = current.clone();
    apply_ddc_snapshot(&mut current, ddc);
    *current != before
}

fn capture(ddc: &Ddc, gpu: &dyn GpuSwitch) -> PeripheralSnapshot {
    let mut snapshot = PeripheralSnapshot {
        gpu_mode: gpu.current(),
        gpu_mode_pending: gpu.pending(),
        ..PeripheralSnapshot::default()
    };
    apply_ddc_snapshot(&mut snapshot, ddc);
    snapshot
}

#[derive(Clone)]
pub struct Peripherals {
    ddc: Arc<Mutex<Ddc>>,
    gpu: Arc<Mutex<Box<dyn GpuSwitch>>>,
    snapshot: SnapshotStore,
}

impl Peripherals {
    /// Discover slow peripherals after the thermal loop is already running.
    pub async fn open(mock: bool, snapshot: SnapshotStore) -> Peripherals {
        let (ddc, gpu) = tokio::task::spawn_blocking(move || (ddc::open(mock), gpu::open(mock)))
            .await
            .expect("peripheral discovery worker panicked");
        write_snapshot(&snapshot, capture(&ddc, gpu.as_ref()));
        Peripherals {
            ddc: Arc::new(Mutex::new(ddc)),
            gpu: Arc::new(Mutex::new(gpu)),
            snapshot,
        }
    }

    pub fn snapshot(&self) -> PeripheralSnapshot {
        read_snapshot(&self.snapshot)
    }

    pub fn ddc_available(&self) -> bool {
        self.snapshot().color_ddc
    }

    async fn rescan_ddc_inner(&self, only_if_unavailable: bool) -> Result<bool, String> {
        let ddc = Arc::clone(&self.ddc);
        let snapshot = Arc::clone(&self.snapshot);
        tokio::task::spawn_blocking(move || {
            let mut ddc = ddc
                .lock()
                .map_err(|_| "DDC worker lock poisoned".to_string())?;
            if only_if_unavailable {
                ddc.rediscover_if_unavailable();
            } else {
                ddc.rediscover();
            }
            Ok(sync_ddc_snapshot(&snapshot, &ddc))
        })
        .await
        .map_err(|e| format!("DDC rescan worker failed: {e}"))?
    }

    /// Re-run DDC/CI detection even when a monitor is already cached.
    pub async fn rescan_ddc(&self) -> Result<bool, String> {
        self.rescan_ddc_inner(false).await
    }

    /// Cheap no-op while a monitor is available; otherwise retry discovery.
    pub async fn rescan_ddc_if_missing(&self) -> Result<bool, String> {
        self.rescan_ddc_inner(true).await
    }

    pub async fn set_gpu_mode(&self, mode: GpuMode) -> Result<(), String> {
        let gpu = Arc::clone(&self.gpu);
        let snapshot = Arc::clone(&self.snapshot);
        tokio::task::spawn_blocking(move || {
            let mut gpu = gpu
                .lock()
                .map_err(|_| "GPU-switch worker lock poisoned".to_string())?;
            gpu.set(mode)?;
            let mut next = read_snapshot(&snapshot);
            next.gpu_mode = gpu.current();
            next.gpu_mode_pending = gpu.pending();
            write_snapshot(&snapshot, next);
            Ok(())
        })
        .await
        .map_err(|e| format!("GPU-switch worker failed: {e}"))?
    }

    pub async fn set_color_preset(&self, value: u8) -> Result<(), String> {
        let ddc = Arc::clone(&self.ddc);
        let snapshot = Arc::clone(&self.snapshot);
        tokio::task::spawn_blocking(move || {
            let mut ddc = ddc
                .lock()
                .map_err(|_| "DDC worker lock poisoned".to_string())?;
            let result = ddc.set(value);
            sync_ddc_snapshot(&snapshot, &ddc);
            result
        })
        .await
        .map_err(|e| format!("DDC worker failed: {e}"))?
    }

    pub async fn set_monitor_brightness(&self, value: u8) -> Result<(), String> {
        let ddc = Arc::clone(&self.ddc);
        let snapshot = Arc::clone(&self.snapshot);
        tokio::task::spawn_blocking(move || {
            let mut ddc = ddc
                .lock()
                .map_err(|_| "DDC worker lock poisoned".to_string())?;
            let result = ddc.set_brightness(value);
            sync_ddc_snapshot(&snapshot, &ddc);
            result
        })
        .await
        .map_err(|e| format!("DDC worker failed: {e}"))?
    }
}
