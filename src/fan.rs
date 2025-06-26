use std::{error::Error, fs};

use log::{error, info, trace};

use crate::{THERMAL_DIR, config::Config};

const DEVICE_NAME_COOLING: &str = "cooling_device";
const DEVICE_TYPE_PWM_FAN: &str = "pwm-fan";

pub struct Fan {
    pub path: String,
    pub max_state: u32,
    pub temp_slots: Box<[(u32, f64)]>,
    pub last_state: Option<u32>,
}

impl Fan {
    fn get_device_max_state(device: &str) -> Result<u32, Box<dyn Error>> {
        let content = fs::read_to_string(format!("{device}/max_state"))?;
        let parsed = content.trim().parse::<u32>()?;
        Ok(parsed)
    }

    pub fn new_fan_device(path: String, config: &Config) -> Self {
        let max_state = Self::get_device_max_state(&path).unwrap();

        config.check_config(max_state);

        let temp_slots = Self::get_temperature_slots(config, &path, &max_state);
        Fan {
            path,
            max_state,
            temp_slots,
            last_state: None,
        }
    }

    fn calculate_slots(config: &Config, max_state: u32) -> Box<[(u32, f64)]> {
        let num_slots = config.state.max.unwrap_or(max_state) - config.state.min;

        let step = if num_slots <= 1 {
            0.0
        } else {
            (config.threshold.max - config.threshold.min) / (num_slots - 1) as f64
        };

        trace!(
            "Calculate slots, min_state: {}, num_slots: {num_slots}, step: {step}",
            config.state.min
        );

        (0..num_slots)
            .map(|i| {
                (
                    i + 1 + config.state.min,
                    if num_slots == 1 {
                        config.threshold.min
                    } else {
                        config.threshold.min + i as f64 * step
                    },
                )
            })
            .collect()
    }

    pub fn get_fan_device() -> Option<String> {
        fs::read_dir(THERMAL_DIR).ok()?.flatten().find_map(|entry| {
            let path = entry.path();
            if path.file_name()?.to_str()?.starts_with(DEVICE_NAME_COOLING) {
                let content = fs::read_to_string(path.join("type")).ok()?;
                if content.trim() == DEVICE_TYPE_PWM_FAN {
                    return Some(path.to_string_lossy().into_owned());
                }
            }
            None
        })
    }

    fn get_temperature_slots(
        config: &Config,
        fan_device: &String,
        max_state: &u32,
    ) -> Box<[(u32, f64)]> {
        let max_state = config.state.max.unwrap_or(*max_state);

        trace!("max_state: {max_state}");
        if max_state == 0 {
            error!("max_state could not be determined for {fan_device}");
            return Box::new([]);
        }

        let slots = Self::calculate_slots(config, max_state);
        trace!("Slots: {slots:?}");
        slots
    }

    pub fn new(config: &Config) -> Option<Self> {
        match Self::get_fan_device() {
            Some(device) => {
                info!("Config device: {device}");
                Some(Self::new_fan_device(device, config))
            }
            None => {
                error!("No PWM fan device found");
                None
            }
        }
    }

    pub fn choose_speed(&self, current_temp: f64, config: &Config) -> u32 {
        match current_temp {
            t if t < config.threshold.min => {
                trace!("min state desired");
                config.state.min
            }
            t if t <= config.threshold.max => {
                trace!("desired state in slots");
                self.temp_slots
                    .iter()
                    .rev()
                    .find(|(_, temp)| *temp <= current_temp)
                    .map(|(state, _)| *state)
                    .unwrap_or(config.state.min)
            }
            _ => {
                trace!("max state desired {}", self.max_state);
                config.state.max.unwrap_or(self.max_state)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::config::{State, Threshold};

    use super::*;

    #[test]
    fn test_check_config() {
        let max_state = Some(4);
        let min_state = 0;

        let mut panic_occurred = false;

        if let Some(max) = max_state
            && min_state >= max
        {
            panic_occurred = true;
        }

        assert!(!panic_occurred);
        let max_state = Some(2);
        let min_state = 3;

        let mut panic_occurred = false;
        if let Some(max) = max_state
            && min_state >= max
        {
            panic_occurred = true;
        }

        assert!(panic_occurred);
    }

    #[test]
    fn test_get_temperature_slots() {
        let max_state = 5;
        let min_threshold = 40.0;
        let max_threshold = 80.0;
        let fan = Config {
            sleep_time: 5,
            threshold: Threshold {
                max: max_threshold,
                min: min_threshold,
            },
            state: State {
                max: Some(max_state),
                min: 0,
            },
        };

        let slots = Fan::calculate_slots(&fan, max_state);

        assert_eq!(slots.len(), 5);
        assert_eq!(slots[0], (1, 40.0));
        assert_eq!(slots[4], (5, max_threshold));

        let step = (max_threshold - min_threshold) / 4.0;
        assert_eq!(slots[1], (2, min_threshold + step));
        assert_eq!(slots[2], (3, min_threshold + 2.0 * step));
        assert_eq!(slots[3], (4, min_threshold + 3.0 * step));
    }

    #[test]
    fn test_get_temperature_one_slot() {
        let min_state = 4;
        let max_state = 5;
        let min_threshold = 40.0;
        let max_threshold = 80.0;
        let fan = Config {
            sleep_time: 5,
            threshold: Threshold {
                max: max_threshold,
                min: min_threshold,
            },
            state: State {
                max: Some(max_state),
                min: min_state,
            },
        };

        let slots = Fan::calculate_slots(&fan, max_state);

        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0], (max_state, min_threshold));
    }

    #[test]
    fn test_get_temperature_no_slots() {
        let min_state = 5;
        let max_state = 5;
        let min_threshold = 40.0;
        let max_threshold = 80.0;
        let fan = Config {
            sleep_time: 5,
            threshold: Threshold {
                max: max_threshold,
                min: min_threshold,
            },
            state: State {
                max: Some(max_state),
                min: min_state,
            },
        };

        let slots = Fan::calculate_slots(&fan, max_state);

        assert_eq!(slots.len(), 0);
    }

    #[test]
    fn test_adjust_speed() {
        let max_state = 5;

        let fan = Config {
            sleep_time: 5,
            threshold: Threshold {
                max: 80.0,
                min: 40.0,
            },
            state: State {
                max: Some(max_state),
                min: 0,
            },
        };

        let current_temp = 60.0;

        let slots = Fan::calculate_slots(&fan, max_state);

        let fallback = (fan.state.min, fan.threshold.min);
        let desired_slot = slots
            .iter()
            .filter(|(_, temp)| *temp <= current_temp)
            .next_back()
            .unwrap_or(&fallback);
        let desired_state = desired_slot.0;

        assert_eq!(desired_state, 3);
    }

    fn setup_test_config() -> Config {
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

    fn setup_test_fan() -> Fan {
        let temp_slots = vec![
            (0, 45.0),
            (1, 50.0),
            (2, 55.0),
            (3, 60.0),
            (4, 65.0),
            (5, 70.0),
        ]
        .into_boxed_slice();

        Fan {
            temp_slots,
            max_state: 5,
            path: "temp".to_owned(),
            last_state: None,
        }
    }

    #[test]
    fn test_temp_below_min_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(25.0, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_temp_at_min_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(45.0, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_temp_in_between_slots() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(52.0, &config);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_temp_above_max_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(80.0, &config);
        assert_eq!(result, config.state.max.unwrap());
    }

    #[test]
    fn test_temp_at_max_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(75.0, &config);
        assert_eq!(result, config.state.max.unwrap());
    }

    #[test]
    fn test_with_empty_slots() {
        let config = Config {
            threshold: Threshold {
                min: 45.0,
                max: 70.0,
            },
            state: State {
                min: 2,
                max: Some(2),
            },
            sleep_time: 5,
        };

        let fan = Fan {
            temp_slots: Vec::new().into_boxed_slice(),
            max_state: 5,
            path: "temp".to_string(),
            last_state: None,
        };

        let result = fan.choose_speed(80.0, &config);
        assert_eq!(result, config.state.min);
    }
}
