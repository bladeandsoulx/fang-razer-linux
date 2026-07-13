//! Supported laptop table, keyed by USB product id.
//!
//! Imported from Razer-Control's `laptops.json` (GPL-2.0,
//! <https://github.com/Rintastic247/Razer-Control>), the maintained
//! continuation of razer-laptop-control. Feature flags cover CPU overclock,
//! the battery charge limiter, and the lid logo LED.
//!
//! Listed PIDs are recognized models with `verified: true`; unknown PIDs get
//! [`FALLBACK`] limits and `verified: false`.

pub struct LaptopModel {
    pub pid: u16,
    pub name: &'static str,
    pub fan_rpm_min: u16,
    pub fan_rpm_max: u16,
    /// Supports CPU overclock boost level (feature "boost").
    pub has_cpu_boost_oc: bool,
    /// Supports the Battery Health Optimizer charge limiter (feature "bho").
    pub has_bho: bool,
    /// Has a lid logo LED (feature "logo").
    pub has_logo: bool,
}

pub const MODELS: &[LaptopModel] = &[
    LaptopModel {
        pid: 0x0205,
        name: "Razer Blade Stealth 2015",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x020F,
        name: "Razer Blade QHD",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0210,
        name: "Razer Blade Pro 2017 v2",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0220,
        name: "Razer Blade Stealth Late 2016",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0224,
        name: "Razer Blade 15 2016",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0225,
        name: "Razer Blade Pro 2017",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x022D,
        name: "Razer Blade Stealth 2017",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x022F,
        name: "Razer Blade Pro 2018 FHD",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0232,
        name: "Razer Blade Stealth Late 2017",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0233,
        name: "Razer Blade 15 2018 Advanced",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0234,
        name: "Razer Blade Pro 2019",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0239,
        name: "Razer Blade Stealth 2019",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x023A,
        name: "Razer Blade 15 2019 Advanced",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x023B,
        name: "Razer Blade 15 2018 Base",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0240,
        name: "Razer Blade 15 2018 Mercury",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0245,
        name: "Razer Blade 15 2019 Mercury",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0246,
        name: "Razer Blade 15 2019 Base",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x024A,
        name: "Razer Blade Stealth 2019 GTX",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x024B,
        name: "Razer Blade 15 Late 2019 Advanced",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x024C,
        name: "Razer Blade Pro Late 2019",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x024D,
        name: "Razer Blade 15 Studio Edition 2019",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0252,
        name: "Razer Blade Stealth 2020",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0253,
        name: "Razer Blade 15 2020 Advanced",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: true,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0255,
        name: "Razer Blade 15 2020 Base",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0256,
        name: "Razer Blade Pro 2020",
        fan_rpm_min: 3500,
        fan_rpm_max: 5300,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0259,
        name: "Razer Blade Stealth Late 2020",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0268,
        name: "Razer Blade 15 Late 2020 Base",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x026A,
        name: "Razer Book 13 2020",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: false,
    },
    LaptopModel {
        pid: 0x026D,
        name: "Razer Blade 15 Late 2021 Advanced",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x026E,
        name: "Razer Blade Pro 17 Early 2021",
        fan_rpm_min: 2300,
        fan_rpm_max: 4300,
        has_cpu_boost_oc: true,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x026F,
        name: "Razer Blade 15 2021 Base",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0270,
        name: "Razer Blade 14 2021",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0276,
        name: "Razer Blade 15 2021 Advanced",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x0279,
        name: "Razer Blade Pro 17 Mid 2021",
        fan_rpm_min: 2300,
        fan_rpm_max: 4300,
        has_cpu_boost_oc: true,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x027A,
        name: "Razer Blade 15 Late 2021 Base",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: false,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x028A,
        name: "Razer Blade 15 Early 2022 Advanced",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x028B,
        name: "Razer Blade 17 2022",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: false,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x028C,
        name: "Razer Blade 14 2022",
        fan_rpm_min: 3500,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x029D,
        name: "Razer Blade 14 2023",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x029E,
        name: "Razer Blade 15 2023",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x029F,
        name: "Razer Blade 16 2023",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x02A0,
        name: "Razer Blade 18 2023",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x02B6,
        name: "Razer Blade 14 2024",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x02B7,
        name: "Razer Blade 16 2024",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x02B8,
        name: "Razer Blade 18 2024",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x02C5,
        name: "Razer Blade 14 2025",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x02C6,
        name: "Razer Blade 16 2025",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
    LaptopModel {
        pid: 0x02C7,
        name: "Razer Blade 18 2025",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
        has_bho: true,
        has_logo: true,
    },
];

/// Conservative limits for Razer laptops not (yet) in [`MODELS`].
pub const FALLBACK: LaptopModel = LaptopModel {
    pid: 0x0000,
    name: "Unknown Razer laptop",
    fan_rpm_min: 2200,
    fan_rpm_max: 5000,
    has_cpu_boost_oc: false,
    has_bho: false,
    has_logo: false,
};

pub fn by_pid(pid: u16) -> Option<&'static LaptopModel> {
    MODELS.iter().find(|m| m.pid == pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sane() {
        let mut seen = std::collections::HashSet::new();
        for m in MODELS {
            assert!(seen.insert(m.pid), "duplicate pid {:#06x}", m.pid);
            assert!(m.name.starts_with("Razer "), "{}", m.name);
            assert!(
                m.fan_rpm_min >= 2000 && m.fan_rpm_max <= 6000 && m.fan_rpm_min < m.fan_rpm_max,
                "implausible fan range on {}",
                m.name
            );
        }
        assert_eq!(MODELS.len(), 48);
    }

    #[test]
    fn known_models_have_expected_flags() {
        let b18_2024 = by_pid(0x02B8).expect("Blade 18 2024");
        assert!(b18_2024.has_cpu_boost_oc && b18_2024.has_bho);
        assert!(b18_2024.has_logo);
        assert!(by_pid(0x02A0).is_some());
        // the only logo-less model in the table
        assert!(!by_pid(0x026A).expect("Razer Book 13").has_logo);
    }
}
