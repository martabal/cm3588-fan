//! End-to-end integration tests that exercise `Fan` and `Temp` against a
//! minimal mock sysfs tree written to a temporary directory.

use std::{
    fs,
    path::{Path, PathBuf},
};

use cm3588_fan::{fan::Fan, temp::Temp};

// ── helpers ──────────────────────────────────────────────────────────────────

struct MockSysfs {
    root: PathBuf,
}

impl MockSysfs {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(name);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn add_cooling_device(&self, name: &str, device_type: &str, max_state: u8) -> PathBuf {
        let dev = self.root.join(name);
        fs::create_dir_all(&dev).unwrap();
        fs::write(dev.join("type"), device_type).unwrap();
        fs::write(dev.join("max_state"), max_state.to_string()).unwrap();
        fs::write(dev.join("cur_state"), "0").unwrap();
        dev
    }

    fn add_thermal_zone(&self, name: &str, temp_millideg: i64) -> PathBuf {
        let zone = self.root.join(name);
        fs::create_dir_all(&zone).unwrap();
        fs::write(zone.join("temp"), format!("{temp_millideg}\n")).unwrap();
        zone
    }
}

impl Drop for MockSysfs {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn read_file(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap().trim().to_owned()
}

// ── Fan::get_fan_device_from ──────────────────────────────────────────────────

#[test]
fn e2e_get_fan_device_from_finds_pwm_fan() {
    let sysfs = MockSysfs::new("e2e_fan_device_finds");
    sysfs.add_cooling_device("cooling_device0", "pwm-fan", 5);

    let result = Fan::get_fan_device_from(&sysfs.root);

    assert!(result.is_some());
    let (state, path) = result.unwrap();
    assert_eq!(path.file_name().unwrap(), "cooling_device0");
    assert!(state.ends_with("cur_state"));
}

#[test]
fn e2e_get_fan_device_from_ignores_non_pwm_fan() {
    let sysfs = MockSysfs::new("e2e_fan_device_ignores");
    sysfs.add_cooling_device("cooling_device0", "thermal-fan", 5);

    let result = Fan::get_fan_device_from(&sysfs.root);

    assert!(result.is_none());
}

#[test]
fn e2e_get_fan_device_from_empty_dir_returns_none() {
    let sysfs = MockSysfs::new("e2e_fan_device_empty");
    // no entries
    let result = Fan::get_fan_device_from(&sysfs.root);
    assert!(result.is_none());
}

// ── Temp::get_temp_path_from ──────────────────────────────────────────────────

#[test]
fn e2e_get_temp_path_from_finds_valid_zone() {
    let sysfs = MockSysfs::new("e2e_temp_path_finds");
    sysfs.add_thermal_zone("thermal_zone0", 45_000);

    let result = Temp::get_temp_path_from(&sysfs.root);

    assert!(result.is_ok());
    assert!(result.unwrap().ends_with("temp"));
}

#[test]
fn e2e_get_temp_path_from_no_valid_zone() {
    let sysfs = MockSysfs::new("e2e_temp_path_no_valid");
    let zone = sysfs.root.join("thermal_zone0");
    fs::create_dir_all(&zone).unwrap();
    fs::write(zone.join("temp"), "not_a_number").unwrap();

    let result = Temp::get_temp_path_from(&sysfs.root);

    assert!(result.is_err());
}

// ── full temperature-to-fan-speed mapping ────────────────────────────────────

/// Verify that for a range of temperatures the computed fan speed written to
/// `cur_state` matches the expected value.
#[test]
fn e2e_full_temperature_to_speed_mapping() {
    use cm3588_fan::config::{Config, State, Threshold};

    let sysfs = MockSysfs::new("e2e_full_mapping");
    let dev_dir = sysfs.add_cooling_device("cooling_device0", "pwm-fan", 5);
    sysfs.add_thermal_zone("thermal_zone0", 0); // will be overridden per case

    let config = Config {
        threshold: Threshold {
            min: 40.0,
            max: 80.0,
        },
        state: State {
            min: 0,
            max: Some(5),
        },
        sleep_time: 5,
    };

    let fan = Fan::new_fan_device(dev_dir.join("cur_state"), dev_dir.clone(), &config);

    let cases: &[(f32, u8)] = &[
        (20.0, 0), // below min threshold → min state
        (40.0, 1), // at min threshold → slot (1, 40.0); not min-state branch since `t < threshold.min` is false at exactly 40.0
        (50.0, 2), // mid-range
        (80.0, 5), // at max threshold → max state
        (90.0, 5), // above max threshold → max state
    ];

    for &(temp, expected_state) in cases {
        let speed = fan.choose_speed(temp, &config);
        assert_eq!(
            speed, expected_state,
            "at {temp}°C expected state {expected_state}, got {speed}"
        );
    }
}

/// Write the fan speed to `cur_state` and confirm the file is updated.
#[test]
fn e2e_fan_speed_written_to_cur_state() {
    use cm3588_fan::config::{Config, State, Threshold};

    let sysfs = MockSysfs::new("e2e_fan_write");
    let dev_dir = sysfs.add_cooling_device("cooling_device0", "pwm-fan", 5);
    sysfs.add_thermal_zone("thermal_zone0", 60_000);

    let config = Config {
        threshold: Threshold {
            min: 40.0,
            max: 80.0,
        },
        state: State {
            min: 0,
            max: Some(5),
        },
        sleep_time: 5,
    };

    let fan = Fan::new_fan_device(dev_dir.join("cur_state"), dev_dir.clone(), &config);
    let desired = fan.choose_speed(60.0, &config);

    // Write the desired speed (simulating what Checker::adjust_speed does).
    fs::write(&fan.state, desired.to_string()).unwrap();

    let written: u8 = read_file(&fan.state).parse().unwrap();
    assert_eq!(written, desired);
}
