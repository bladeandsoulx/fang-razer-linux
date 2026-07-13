//! Slow, non-thermal peripherals isolated from the EC control core.
//!
//! DDC/CI and GPU switching call external programs. They run on blocking
//! workers behind their own locks, while the 1 Hz thermal task owns only the
//! EC core. A cached snapshot keeps status assembly non-blocking.

use crate::ddc::{self, Ddc};
use crate::gpu::{self, GpuSwitch};
use fang_protocol::api::{ColorPreset, GpuMode};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone, Debug, Default)]
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

fn capture(ddc: &Ddc, gpu: &dyn GpuSwitch) -> PeripheralSnapshot {
    PeripheralSnapshot {
        color_ddc: ddc.available(),
        color_presets: ddc.presets(),
        color_current: ddc.current(),
        monitor_brightness: ddc.brightness(),
        gpu_mode: gpu.current(),
        gpu_mode_pending: gpu.pending(),
    }
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
            ddc.set(value)?;
            let mut next = read_snapshot(&snapshot);
            next.color_current = ddc.current();
            write_snapshot(&snapshot, next);
            Ok(())
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
            ddc.set_brightness(value)?;
            let mut next = read_snapshot(&snapshot);
            next.monitor_brightness = ddc.brightness();
            write_snapshot(&snapshot, next);
            Ok(())
        })
        .await
        .map_err(|e| format!("DDC worker failed: {e}"))?
    }
}
