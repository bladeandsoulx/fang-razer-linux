//! Which power source the laptop is on, read from sysfs.
//!
//! `/sys/class/power_supply/<name>/type` identifies batteries and external
//! supplies, while `online` is `1` when a supply is powering the machine.
//! Names such as `AC`, `ADP1`, and `ucsi-source-psy-*` are driver-specific, so
//! detection uses the kernel's type values and aggregates every adapter.

use std::path::Path;

/// Kernel `power_supply_type_text` values that represent an external source.
/// `USB_TYPE_C` is accepted as a compatibility alias used by some out-of-tree
/// drivers; upstream kernels expose that type as `USB_C`.
fn is_external_supply(kind: &str) -> bool {
    matches!(
        kind,
        "Mains"
            | "USB"
            | "USB_DCP"
            | "USB_CDP"
            | "USB_ACA"
            | "USB_C"
            | "USB_TYPE_C"
            | "USB_PD"
            | "USB_PD_DRP"
            | "BrickID"
            | "Wireless"
    )
}

fn parse_online(value: &str) -> Option<bool> {
    match value.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn on_ac_at(root: &Path) -> Option<bool> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut saw_offline = false;

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = std::fs::read_to_string(path.join("type")) else {
            continue;
        };
        if !is_external_supply(kind.trim()) {
            continue;
        }
        let online = std::fs::read_to_string(path.join("online"))
            .ok()
            .and_then(|value| parse_online(&value));
        match online {
            // Any live adapter wins, regardless of directory ordering or an
            // earlier offline barrel/USB-C entry.
            Some(true) => return Some(true),
            Some(false) => saw_offline = true,
            None => {}
        }
    }

    saw_offline.then_some(false)
}

/// `Some(true)` on external power, `Some(false)` when every readable external
/// supply is offline, and `None` when no supported supply is exposed or sysfs
/// is unreadable (non-Linux always returns `None`).
pub fn on_ac() -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        on_ac_at(Path::new("/sys/class/power_supply"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::on_ac_at;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(entries: &[(&str, &str, Option<&str>)]) -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("fang-power-test-{}-{id}", std::process::id()));
            std::fs::create_dir(&root).unwrap();
            for (name, kind, online) in entries {
                let supply = root.join(name);
                std::fs::create_dir(&supply).unwrap();
                std::fs::write(supply.join("type"), kind).unwrap();
                if let Some(online) = online {
                    std::fs::write(supply.join("online"), online).unwrap();
                }
            }
            Fixture(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn any_online_adapter_wins_over_an_offline_one() {
        let fixture = Fixture::new(&[("AC0", "Mains", Some("0\n")), ("AC1", "Mains", Some("1\n"))]);
        assert_eq!(on_ac_at(fixture.path()), Some(true));
    }

    #[test]
    fn usb_c_and_usb_pd_are_external_power_sources() {
        for kind in ["USB_C", "USB_PD", "USB_PD_DRP"] {
            let fixture = Fixture::new(&[("ucsi-source-psy", kind, Some("1"))]);
            assert_eq!(on_ac_at(fixture.path()), Some(true), "type {kind}");
        }
    }

    #[test]
    fn all_readable_adapters_offline_means_battery_power() {
        let fixture = Fixture::new(&[
            ("barrel", "Mains", Some("0")),
            ("typec", "USB_C", Some("0")),
            ("BAT0", "Battery", None),
        ]);
        assert_eq!(on_ac_at(fixture.path()), Some(false));
    }

    #[test]
    fn absent_or_unreadable_external_supplies_are_unknown() {
        let batteries_only = Fixture::new(&[("BAT0", "Battery", None)]);
        assert_eq!(on_ac_at(batteries_only.path()), None);

        let unreadable = Fixture::new(&[("AC0", "Mains", None)]);
        assert_eq!(on_ac_at(unreadable.path()), None);
    }
}
