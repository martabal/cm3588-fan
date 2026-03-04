use std::{
    fs::{self, File},
    io::Read,
};

use log::{debug, error, info, trace};

use crate::{config::Config, fan::Fan, temp::Temp};

pub struct Checker {
    is_init: bool,
    pub config: Config,
    fan_device: Option<Fan>,
    temp_device: Option<Temp>,
    buf: String,
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker {
    #[must_use]
    pub fn new() -> Self {
        let temp_device = match Temp::new() {
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
        }
    }

    pub fn adjust_speed(&mut self) {
        if self.fan_device.is_none() {
            if let Some((fan_path, path)) = Fan::get_fan_device() {
                trace!("New fan device detected");
                self.fan_device = Some(Fan::new_fan_device(fan_path, path, &self.config));
            } else {
                error!("Still no fan device available");
                return;
            }
        }

        if self.temp_device.is_none() {
            if let Ok(device) = Temp::new() {
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

        // Only read the current fan state when we may need to write a new value,
        // avoiding a syscall in the common steady-state case.
        self.buf.clear();
        let current_speed: u32 =
            match File::open(&fan.state).and_then(|mut f| f.read_to_string(&mut self.buf)) {
                Ok(_) => match self.buf.trim().parse::<u32>() {
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

        fn create_fan(&self, state_content: &str, last_state: Option<u32>) -> Fan {
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
        };

        checker.adjust_speed();
    }
}
