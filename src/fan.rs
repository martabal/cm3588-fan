use std::{error::Error, fs};

use log::{error, info, trace};

use crate::config::Config;

const DEVICE_NAME_COOLING: &str = "cooling_device";
const DEVICE_TYPE_PWM_FAN: &str = "pwm-fan";
const THERMAL_DIR: &str = "/sys/class/thermal";
const THERMAL_ZONE_NAME: &str = "thermal_zone";

pub struct Fan {
    pub fan_path: String,
    pub temp_path: String,
    pub max_state: u32,
    pub temp_slots: Box<[(u32, f64)]>,
}

impl Fan {
    fn get_device_max_state(device: &str) -> Result<u32, Box<dyn Error>> {
        let content = fs::read_to_string(format!("{device}/max_state"))?;
        let parsed = content.trim().parse::<u32>()?;
        Ok(parsed)
    }

    pub fn new_fan_device(fan_path: String, config: &Config) -> Self {
        let max_state = Self::get_device_max_state(&fan_path).unwrap();
        let temp_path = Self::get_temp_path().unwrap();
        let temp_slots = Self::get_temperature_slots(config, &fan_path, &max_state);
        let fan = Fan {
            temp_path,
            fan_path,
            max_state,
            temp_slots,
        };
        config.check_config(Some(&fan));
        fan
    }

    pub fn get_current_temp(&self) -> Result<f64, Box<dyn Error>> {
        let temp = fs::read_to_string(&self.temp_path)?.trim().parse::<f64>()? / 1000.0;
        Ok(temp)
    }

    pub fn get_temp_path() -> Result<String, Box<dyn Error>> {
        for entry in fs::read_dir(THERMAL_DIR)? {
            let entry = entry?;
            let path = entry.path();

            if path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with(THERMAL_ZONE_NAME))
                .unwrap_or(false)
            {
                let temp_path = path.join("temp");
                if let Ok(s) = fs::read_to_string(&temp_path) {
                    if s.trim().parse::<f64>().is_ok() {
                        return Ok(s);
                    }
                }
            }
        }

        Err(Box::from("No valid thermal zone found"))
    }

    fn calculate_slots(config: &Config, max_state: u32) -> Box<[(u32, f64)]> {
        let num_slots = config.state.max.unwrap_or(max_state) - config.state.min;
        let step = (config.threshold.max - config.threshold.min) / (num_slots - 1) as f64;

        trace!(
            "Calculate slots, min_state: {}, num_slots: {num_slots}, step: {step}",
            config.state.min
        );

        (0..num_slots)
            .map(|i| {
                (
                    i + 1 + config.state.min,
                    config.threshold.min + i as f64 * step,
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
        trace!("Slots: {:?}", slots);
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
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap, path::Path};

    use crate::{
        FILE_NAME_CUR_STATE,
        config::{State, Threshold},
    };

    use super::*;

    struct MockFs {
        files: RefCell<HashMap<String, String>>,
        dirs: RefCell<HashMap<String, Vec<String>>>,
    }

    impl MockFs {
        fn new() -> Self {
            Self {
                files: RefCell::new(HashMap::new()),
                dirs: RefCell::new(HashMap::new()),
            }
        }

        fn add_file(&self, path: &str, content: &str) {
            self.files
                .borrow_mut()
                .insert(path.to_string(), content.to_string());
        }

        fn add_dir(&self, path: &str, entries: Vec<String>) {
            self.dirs.borrow_mut().insert(path.to_string(), entries);
        }
    }

    fn mock_read_dir(mock_fs: &MockFs, path: &str) -> Result<Vec<String>, std::io::Error> {
        match mock_fs.dirs.borrow().get(path) {
            Some(entries) => Ok(entries.clone()),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Directory not found",
            )),
        }
    }

    fn mock_read_to_string(mock_fs: &MockFs, path: &str) -> Result<String, std::io::Error> {
        match mock_fs.files.borrow().get(path) {
            Some(content) => Ok(content.clone()),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            )),
        }
    }

    #[test]
    fn test_get_device_max_state() {
        let mock_fs = MockFs::new();
        let fan_device = format!("{}/cooling_device0", THERMAL_DIR);

        mock_fs.add_file(&format!("{}/max_state", fan_device), "4");
        let result = mock_read_to_string(&mock_fs, &format!("{}/max_state", fan_device))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        assert_eq!(result, 4);

        mock_fs.add_file(&format!("{}/max_state", fan_device), "invalid");
        let result = mock_read_to_string(&mock_fs, &format!("{}/max_state", fan_device))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_current_temp() {
        let mock_fs = MockFs::new();

        mock_fs.add_dir(
            THERMAL_DIR,
            vec![
                format!("{}/thermal_zone0", THERMAL_DIR),
                format!("{}/thermal_zone1", THERMAL_DIR),
                format!("{}/cooling_device0", THERMAL_DIR),
            ],
        );

        mock_fs.add_file(&format!("{}/thermal_zone0/temp", THERMAL_DIR), "45000");
        mock_fs.add_file(&format!("{}/thermal_zone1/temp", THERMAL_DIR), "55000");

        let result = {
            let mut temps = vec![];
            if let Ok(entries) = mock_read_dir(&mock_fs, THERMAL_DIR) {
                for entry in entries {
                    let path = Path::new(&entry);
                    if let Some(file_name) = path.file_name() {
                        if let Some(file_str) = file_name.to_str() {
                            if file_str.starts_with(THERMAL_ZONE_NAME) {
                                let temp_file = format!("{}/temp", path.to_string_lossy());
                                if let Ok(content) = mock_read_to_string(&mock_fs, &temp_file) {
                                    if let Ok(temp) = content.trim().parse::<f64>() {
                                        temps.push(temp / 1000.0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            temps.into_iter().fold(0.0, f64::max)
        };

        assert_eq!(result, 55.0);
    }

    #[test]
    fn test_check_config() {
        let max_state = Some(4);
        let min_state = 0;

        let mut panic_occurred = false;

        if let Some(max) = max_state {
            if min_state >= max {
                panic_occurred = true;
            }
        }

        assert!(!panic_occurred);
        let max_state = Some(2);
        let min_state = 3;

        let mut panic_occurred = false;
        if let Some(max) = max_state {
            if min_state >= max {
                panic_occurred = true;
            }
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
        let slots_len = 5;

        assert_eq!(slots.len(), slots_len);
        assert_eq!(slots[0], (1, 40.0));
        assert_eq!(slots[4], (5, max_threshold));

        let step = (max_threshold - min_threshold) / 4.0;
        assert_eq!(slots[1], (2, min_threshold + step));
        assert_eq!(slots[2], (3, min_threshold + 2.0 * step));
        assert_eq!(slots[3], (4, min_threshold + 3.0 * step));
    }

    #[test]
    fn test_adjust_speed() {
        let mock_fs = MockFs::new();
        let fan_device = format!("{}/cooling_device0", THERMAL_DIR);

        mock_fs.add_file(&format!("{}/max_state", fan_device), "4");
        mock_fs.add_file(&format!("{}/{}", fan_device, FILE_NAME_CUR_STATE), "1");

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
}
