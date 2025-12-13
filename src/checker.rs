use std::fs;

use log::{debug, error, info, trace};

use crate::{config::Config, fan::Fan, temp::Temp};

pub struct Checker {
    is_init: bool,
    pub config: Config,
    fan_device: Option<Fan>,
    temp_device: Option<Temp>,
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

        let fan = self.fan_device.as_mut().unwrap();
        let current_speed: u32 = match fs::read_to_string(&fan.state) {
            Ok(content) => match content.trim().parse::<u32>() {
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
        let current_temp = match temp.get_current_temp() {
            Ok(temp) => temp,
            Err(err) => {
                error!("Can't read temperature: {err}");
                self.temp_device = None;
                return;
            }
        };
        debug!("Current temp {current_temp}");

        let desired_speed = fan.choose_speed(current_temp, &self.config);
        debug!("Desired speed {desired_speed}");

        if fan.last_state == Some(desired_speed) {
            debug!("State unchanged");
            return;
        }

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

    #[test]
    fn test_checker_structure() {
        let checker = Checker {
            is_init: false,
            config: Config {
                threshold: Threshold {
                    min: 45.0,
                    max: 70.0,
                },
                state: State {
                    min: 0,
                    max: Some(5),
                },
                sleep_time: 5,
            },
            fan_device: None,
            temp_device: None,
        };
        assert!(!checker.is_init);
        assert!(checker.fan_device.is_none());
        assert!(checker.temp_device.is_none());
    }

    #[test]
    fn test_adjust_speed_without_fan_device() {
        let mut checker = Checker {
            is_init: false,
            config: Config {
                threshold: Threshold {
                    min: 45.0,
                    max: 70.0,
                },
                state: State {
                    min: 0,
                    max: Some(5),
                },
                sleep_time: 5,
            },
            fan_device: None,
            temp_device: None,
        };

        checker.adjust_speed();
    }

    #[test]
    fn test_checker_with_mock_fan() {
        let temp_dir = std::env::temp_dir().join("test_checker_mock");
        fs::create_dir_all(&temp_dir).unwrap();
        let state_file = temp_dir.join("cur_state");
        fs::write(&state_file, "2").unwrap();

        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "55000").unwrap();

        let fan = Fan {
            path: temp_dir.clone(),
            state: state_file.clone(),
            max_state: 5,
            temp_slots: vec![
                (1, 45.0),
                (2, 50.0),
                (3, 55.0),
                (4, 60.0),
                (5, 65.0),
            ],
            last_state: None,
        };

        let temp = Temp {
            path: temp_file.clone(),
        };

        let mut checker = Checker {
            is_init: false,
            config: Config {
                threshold: Threshold {
                    min: 45.0,
                    max: 70.0,
                },
                state: State {
                    min: 0,
                    max: Some(5),
                },
                sleep_time: 5,
            },
            fan_device: Some(fan),
            temp_device: Some(temp),
        };

        checker.adjust_speed();
        assert!(checker.is_init);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_adjust_speed_with_same_desired_speed() {
        let temp_dir = std::env::temp_dir().join("test_checker_same_speed");
        fs::create_dir_all(&temp_dir).unwrap();
        let state_file = temp_dir.join("cur_state");
        fs::write(&state_file, "3").unwrap();

        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "55000").unwrap();

        let fan = Fan {
            path: temp_dir.clone(),
            state: state_file.clone(),
            max_state: 5,
            temp_slots: vec![
                (1, 45.0),
                (2, 50.0),
                (3, 55.0),
                (4, 60.0),
                (5, 65.0),
            ],
            last_state: Some(3),
        };

        let temp = Temp {
            path: temp_file.clone(),
        };

        let mut checker = Checker {
            is_init: true,
            config: Config {
                threshold: Threshold {
                    min: 45.0,
                    max: 70.0,
                },
                state: State {
                    min: 0,
                    max: Some(5),
                },
                sleep_time: 5,
            },
            fan_device: Some(fan),
            temp_device: Some(temp),
        };

        checker.adjust_speed();

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_adjust_speed_with_invalid_temp_file() {
        let temp_dir = std::env::temp_dir().join("test_checker_invalid_temp");
        fs::create_dir_all(&temp_dir).unwrap();
        let state_file = temp_dir.join("cur_state");
        fs::write(&state_file, "2").unwrap();

        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "invalid").unwrap();

        let fan = Fan {
            path: temp_dir.clone(),
            state: state_file.clone(),
            max_state: 5,
            temp_slots: vec![(1, 45.0), (2, 50.0), (3, 55.0)],
            last_state: None,
        };

        let temp = Temp {
            path: temp_file.clone(),
        };

        let mut checker = Checker {
            is_init: false,
            config: Config {
                threshold: Threshold {
                    min: 45.0,
                    max: 70.0,
                },
                state: State {
                    min: 0,
                    max: Some(5),
                },
                sleep_time: 5,
            },
            fan_device: Some(fan),
            temp_device: Some(temp),
        };

        checker.adjust_speed();
        assert!(checker.temp_device.is_none());

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_adjust_speed_with_invalid_speed_file() {
        let temp_dir = std::env::temp_dir().join("test_checker_invalid_speed");
        fs::create_dir_all(&temp_dir).unwrap();
        let state_file = temp_dir.join("cur_state");
        fs::write(&state_file, "invalid_speed").unwrap();

        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "50000").unwrap();

        let fan = Fan {
            path: temp_dir.clone(),
            state: state_file.clone(),
            max_state: 5,
            temp_slots: vec![(1, 45.0), (2, 50.0), (3, 55.0)],
            last_state: None,
        };

        let temp = Temp {
            path: temp_file.clone(),
        };

        let mut checker = Checker {
            is_init: false,
            config: Config {
                threshold: Threshold {
                    min: 45.0,
                    max: 70.0,
                },
                state: State {
                    min: 0,
                    max: Some(5),
                },
                sleep_time: 5,
            },
            fan_device: Some(fan),
            temp_device: Some(temp),
        };

        checker.adjust_speed();

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_adjust_speed_without_temp_device() {
        let temp_dir = std::env::temp_dir().join("test_checker_no_temp");
        fs::create_dir_all(&temp_dir).unwrap();
        let state_file = temp_dir.join("cur_state");
        fs::write(&state_file, "2").unwrap();

        let fan = Fan {
            path: temp_dir.clone(),
            state: state_file.clone(),
            max_state: 5,
            temp_slots: vec![(1, 45.0), (2, 50.0), (3, 55.0)],
            last_state: None,
        };

        let mut checker = Checker {
            is_init: false,
            config: Config {
                threshold: Threshold {
                    min: 45.0,
                    max: 70.0,
                },
                state: State {
                    min: 0,
                    max: Some(5),
                },
                sleep_time: 5,
            },
            fan_device: Some(fan),
            temp_device: None,
        };

        checker.adjust_speed();

        fs::remove_dir_all(&temp_dir).ok();
    }
}
