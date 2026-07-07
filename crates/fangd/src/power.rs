//! Which power source the laptop is on, read from sysfs.
//!
//! `/sys/class/power_supply/<name>/type` is `Mains` for an AC adapter; its
//! `online` file is `1` when plugged in. Laptops usually expose `AC`/`ADP1`,
//! but the name varies, so we match on `type` rather than a fixed name. A
//! machine with no Mains supply (a desktop) yields `None`.

/// `Some(true)` on AC, `Some(false)` on battery, `None` when no AC adapter is
/// exposed or sysfs is unreadable (non-Linux always returns `None`).
pub fn on_ac() -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        let dir = std::fs::read_dir("/sys/class/power_supply").ok()?;
        for entry in dir.flatten() {
            let path = entry.path();
            let kind = std::fs::read_to_string(path.join("type")).unwrap_or_default();
            if kind.trim() == "Mains" {
                let online = std::fs::read_to_string(path.join("online")).ok()?;
                return Some(online.trim() == "1");
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
