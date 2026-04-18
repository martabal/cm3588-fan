use std::{
    fs::{self, File},
    io::Read,
};

use log::{debug, error, info, trace};

use crate::{THERMAL_DIR, config::Config, fan::Fan, temp::Temp};

pub struct Checker {
    is_init: bool,
    pub config: Config,
    fan_device: Option<Fan>,
    temp_device: Option<Temp>,
    buf: String,
    thermal_dir: String,
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker {
    #[must_use]
    pub fn new() -> Self {
        let thermal_dir = THERMAL_DIR.to_owned();
        let temp_device = match Temp::new_from(&thermal_dir) {
            Ok(temp) => Some(temp),
            Err(err) => {
                error!("Can't read temperature: {err}");
                None
            }
        };

        let config = Config::new();
        let fan_device = Fan::new(&config);

        Self {
            is_init: false,
            config,
            fan_device,
            temp_device,
            buf: String::new(),
            thermal_dir,
        }
    }

    pub fn adjust_speed(&mut self) {
        if self.fan_device.is_none() {
            if let Some((fan_path, path)) = Fan::get_fan_device_from(&self.thermal_dir) {
                trace!("New fan device detected");
                self.fan_device = Some(Fan::new_fan_device(fan_path, path, &self.config));
            } else {
                error!("Still no fan device available");
                return;
            }
        }

        if self.temp_device.is_none() {
            if let Ok(device) = Temp::new_from(&self.thermal_dir) {
                trace!("New temp device detected");
                self.temp_device = Some(device);
            } else {
                error!("Still no temp device available");
                return;
            }
        }

        let temp = self.temp_device.as_ref().unwrap();
        let current_temp = match temp.get_current_temp(&mut self.buf) {
            Ok(temp) => temp,
            Err(err) => {
                error!("Can't read temperature: {err}");
                self.temp_device = None;
                return;
            }
        };
        debug!("Current temp {current_temp}");

        let fan = self.fan_device.as_mut().unwrap();
        let desired_speed = fan.choose_speed(current_temp, &self.config);
        debug!("Desired speed {desired_speed}");

        if fan.last_state == Some(desired_speed) {
            debug!("State unchanged");
            return;
        }

        self.buf.clear();
        let current_speed: u8 =
            match File::open(&fan.state).and_then(|mut f| f.read_to_string(&mut self.buf)) {
                Ok(_) => match self.buf.trim().parse::<u8>() {
                    Ok(speed) => speed,
                    Err(e) => {
                        error!("Can't parse speed value: {e}");
                        return;
                    }
                },
                Err(e) => {
                    error!("Device is not available: {e}");
                    self.fan_device = None;
                    return;
                }
            };

        if current_speed != desired_speed || !self.is_init {
            if !self.is_init {
                debug!("Setting the speed for the first time!");
                self.is_init = true;
            }
            info!("Adjusting fan speed to {desired_speed} (Temp: {current_temp:.2}°C)");
            if fs::write(&fan.state, desired_speed.to_string()).is_ok() {
                fan.last_state = Some(desired_speed);
            } else {
                error!("Can't set speed on device {}", fan.state.display());
                self.fan_device = None;
            }
        } else {
            debug!("Temp: {current_temp:.2}°C, no speed change needed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{State, Threshold};
    use std::path::PathBuf;

    fn create_test_config() -> Config {
        Config {
            threshold: Threshold {
                min: 45.0,
                max: 70.0,
            },
            state: State {
                min: 0,
                max: Some(5),
            },
            sleep_time: 5,
        }
    }

    struct TestEnv {
        path: PathBuf,
    }

    impl TestEnv {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(name);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn create_fan(&self, state_content: &str, last_state: Option<u8>) -> Fan {
            let state_file = self.path.join("cur_state");
            fs::write(&state_file, state_content).unwrap();

            Fan {
                path: self.path.clone(),
                state: state_file,
                max_state: 5,
                temp_slots: vec![(1, 45.0), (2, 50.0), (3, 55.0), (4, 60.0), (5, 65.0)],
                last_state,
            }
        }

        fn create_temp(&self, content: &str) -> Temp {
            let temp_file = self.path.join("temp");
            fs::write(&temp_file, content).unwrap();

            Temp { path: temp_file }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_checker_structure() {
        let checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: None,
            temp_device: None,
            buf: String::new(),
            thermal_dir: String::new(),
        };
        assert!(!checker.is_init);
        assert!(checker.fan_device.is_none());
        assert!(checker.temp_device.is_none());
    }

    #[test]
    fn test_adjust_speed_without_fan_device() {
        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: None,
            temp_device: None,
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();
    }

    #[test]
    fn test_checker_with_mock_fan() {
        let env = TestEnv::new("test_checker_mock");
        let fan = env.create_fan("2", None);
        let temp = env.create_temp("55000");

        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: Some(temp),
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();
        assert!(checker.is_init);
    }

    #[test]
    fn test_adjust_speed_with_same_desired_speed() {
        let env = TestEnv::new("test_checker_same_speed");
        let fan = env.create_fan("3", Some(3));
        let temp = env.create_temp("55000");

        let mut checker = Checker {
            is_init: true,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: Some(temp),
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();
    }

    #[test]
    fn test_adjust_speed_with_invalid_temp_file() {
        let env = TestEnv::new("test_checker_invalid_temp");
        let fan = env.create_fan("2", None);
        let temp = env.create_temp("invalid");

        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: Some(temp),
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();
        assert!(checker.temp_device.is_none());
    }

    #[test]
    fn test_adjust_speed_with_invalid_speed_file() {
        let env = TestEnv::new("test_checker_invalid_speed");
        let fan = env.create_fan("invalid_speed", None);
        let temp = env.create_temp("50000");

        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: Some(temp),
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();
    }

    #[test]
    fn test_adjust_speed_without_temp_device() {
        let env = TestEnv::new("test_checker_no_temp");
        let fan = env.create_fan("2", None);

        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: None,
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();
    }

    // ── new tests ─────────────────────────────────────────────────────────────

    impl TestEnv {
        /// Create a minimal mock sysfs suitable for both fan and temp recovery
        /// tests. The directory layout is:
        ///   <root>/cooling_device0/{type, max_state, cur_state}
        ///   <root>/thermal_zone0/temp
        fn create_mock_sysfs(&self, temp_content: &str) {
            // cooling device
            let dev = self.path.join("cooling_device0");
            fs::create_dir_all(&dev).unwrap();
            fs::write(dev.join("type"), "pwm-fan").unwrap();
            fs::write(dev.join("max_state"), "5").unwrap();
            fs::write(dev.join("cur_state"), "0").unwrap();

            // thermal zone
            let zone = self.path.join("thermal_zone0");
            fs::create_dir_all(&zone).unwrap();
            fs::write(zone.join("temp"), temp_content).unwrap();
        }
    }

    #[test]
    fn test_adjust_speed_fan_recovers() {
        let env = TestEnv::new("test_checker_fan_recovers");
        env.create_mock_sysfs("55000");
        let temp = env.create_temp("55000");

        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: None,
            temp_device: Some(temp),
            buf: String::new(),
            thermal_dir: env.path.to_string_lossy().into_owned(),
        };

        checker.adjust_speed();

        assert!(
            checker.fan_device.is_some(),
            "fan should be detected and assigned"
        );
        assert!(checker.is_init);
    }

    #[test]
    fn test_adjust_speed_temp_recovers() {
        let env = TestEnv::new("test_checker_temp_recovers");
        env.create_mock_sysfs("55000");
        let fan = env.create_fan("0", None);

        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: None,
            buf: String::new(),
            thermal_dir: env.path.to_string_lossy().into_owned(),
        };

        checker.adjust_speed();

        assert!(
            checker.temp_device.is_some(),
            "temp device should be detected and assigned"
        );
        assert!(checker.is_init);
    }

    #[test]
    fn test_adjust_speed_no_speed_change_when_already_correct() {
        let env = TestEnv::new("test_checker_no_change");
        // Temp 55 °C with the test config slots maps to state 2.
        // Pre-write "2" to the state file and set last_state = None so the
        // checker reads the file and finds current_speed == desired_speed.
        let fan = env.create_fan("2", None);
        let temp = env.create_temp("55000");

        // We need is_init = true so the `|| !self.is_init` branch does not
        // force a write.
        let mut checker = Checker {
            is_init: true,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: Some(temp),
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();

        // The fan device must still be present (no error path triggered).
        assert!(checker.fan_device.is_some());
    }

    #[cfg(unix)]
    #[test]
    fn test_adjust_speed_write_failure_clears_fan_device() {
        use std::os::unix::fs::PermissionsExt;

        let env = TestEnv::new("test_checker_write_fail");
        let fan = env.create_fan("0", None);
        let temp = env.create_temp("80000"); // high temp → max speed → definitely different from 0

        // Make the state file read-only so the write fails.
        let state_path = env.path.join("cur_state");
        fs::set_permissions(&state_path, fs::Permissions::from_mode(0o444)).unwrap();

        let mut checker = Checker {
            is_init: false,
            config: create_test_config(),
            fan_device: Some(fan),
            temp_device: Some(temp),
            buf: String::new(),
            thermal_dir: String::new(),
        };

        checker.adjust_speed();

        // The fan device must have been cleared after the write error.
        assert!(checker.fan_device.is_none());
    }
}
