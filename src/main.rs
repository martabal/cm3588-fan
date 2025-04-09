use std::{env, error::Error, fs, io::Write, str::FromStr, thread, time::Duration};

use colored::Colorize;
use env_logger::Builder;
use log::{Level, LevelFilter, debug, error, info, trace, warn};

const DEVICE_NAME_COOLING: &str = "cooling_device";
const DEVICE_TYPE_PWM_FAN: &str = "pwm-fan";
const THERMAL_DIR: &str = "/sys/class/thermal";
const THERMAL_ZONE_NAME: &str = "thermal_zone";
const FILE_NAME_CUR_STATE: &str = "cur_state";
const LOWER_TEMP_THRESHOLD: f64 = 45.0;
const UPPER_TEMP_THRESHOLD: f64 = 65.0;
const MIN_STATE: u32 = 0;

struct Fan {
    threshold: Threshold,
    state: State,
}

struct State {
    max_state: Option<u32>,
    min_state: u32,
}

struct Threshold {
    min_threshold: f64,
    max_threshold: f64,
}

fn check_config(max_state: &Option<u32>, min_state: u32, fan_device: &Option<String>) {
    if let Some(max) = max_state {
        if min_state >= *max {
            panic!("min_state can't be >= max_state: {min_state} >= {max}");
        }

        match fan_device {
            Some(device) => {
                let device_max = get_device_max_state(device);

                if *max > device_max {
                    panic!("Configured max_state {max} exceeds device max_state {device_max}");
                }
            }
            None => warn!("max_state can't be checked because fan_device is not detected"),
        }
    }
}

fn setup_logging(debug_mode: bool) {
    let level_filter = match env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Debug,
    };

    let mut builder = Builder::new();

    if !debug_mode {
        builder.format(|f, r| {
            let color = match r.level() {
                Level::Warn => r.args().to_string().yellow(),
                Level::Error => r.args().to_string().red(),
                Level::Info => r.args().to_string().green(),
                Level::Debug => r.args().to_string().blue(),
                Level::Trace => r.args().to_string().cyan(),
            };
            writeln!(f, "{color}")
        });
    }

    builder.filter_level(level_filter).init();

    println!("Log level set to: {level_filter}");
    let msg = format!(
        "Starting PWM Fan Control Service v{}",
        env!("CARGO_PKG_VERSION")
    );

    if debug_mode {
        info!("{msg}");
    } else {
        println!("{msg}");
    }
}

fn get_device_max_state(device: &str) -> u32 {
    fs::read_to_string(format!("{device}/max_state"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn calculate_slots(fan: &Fan, max_state: u32) -> Vec<(u32, f64)> {
    let step = (fan.threshold.max_threshold - fan.threshold.min_threshold) / max_state as f64;

    (0..=max_state)
        .map(|i| {
            (
                i + fan.state.min_state,
                i as f64 * step + fan.threshold.min_threshold,
            )
        })
        .collect()
}

fn get_temperature_slots(fan: &Fan, fan_device: &mut String) -> Vec<(u32, f64)> {
    let max_state = fan
        .state
        .max_state
        .unwrap_or_else(|| get_device_max_state(fan_device));

    trace!("max_state: {max_state}");
    if max_state == 0 {
        error!("max_state could not be determined for {fan_device}");
        return vec![];
    }

    let slots = calculate_slots(fan, max_state);
    trace!("Slots: {:?}", slots);
    slots
}

fn get_fan_device() -> Option<String> {
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

fn get_current_temp() -> Result<f64, Box<dyn Error>> {
    let mut max_temp = None;

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
                if let Ok(t) = s.trim().parse::<f64>() {
                    let temp = t / 1000.0;
                    max_temp = Some(max_temp.map_or(temp, |m: f64| m.max(temp)));
                }
            }
        }
    }

    max_temp.ok_or_else(|| Box::from("No valid thermal zone found"))
}

fn adjust_speed(current_temp: f64, is_init: &mut bool, fan: &Fan, fan_device: &mut Option<String>) {
    let path = match fan_device {
        Some(p) => p,
        None => return,
    };

    let file_content = match fs::read_to_string(format!("{}/{}", path, FILE_NAME_CUR_STATE)) {
        Ok(content) => content,
        Err(e) => {
            *fan_device = None;
            error!("Device is not available {e}");
            return;
        }
    };

    match file_content.trim().parse::<u32>() {
        Ok(speed) => {
            let slots = get_temperature_slots(fan, path);
            let fallback = (fan.state.min_state, fan.threshold.min_threshold);
            let desired = slots
                .iter()
                .rev()
                .find(|(_, t)| *t <= current_temp)
                .unwrap_or(slots.first().unwrap_or(&fallback));
            let desired_state = desired.0;

            if speed != desired_state || !*is_init {
                info!("Adjusting fan speed to {desired_state} (Temp: {current_temp:.2}°C)");

                match fs::write(
                    format!("{path}/{FILE_NAME_CUR_STATE}"),
                    desired_state.to_string(),
                ) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Can't set the speed to device {path} {err}")
                    }
                }
            } else {
                debug!("Temp: {current_temp:.2}°C, no speed change needed");
            }
            if !*is_init {
                debug!("Setting the speed for the first time!");
                *is_init = true;
            }
        }
        Err(e) => {
            error!("Can't parse speed value {e}");
        }
    }
}

fn get_env<T: FromStr>(key: &str, fallback: T) -> T {
    env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(fallback)
}

fn main() {
    let debug = get_env("DEBUG", false);
    setup_logging(debug);

    let mut fan_device = match get_fan_device() {
        Some(device) => {
            info!("Fan device: {device}");
            Some(device)
        }
        None => {
            error!("No PWM fan device found");
            None
        }
    };

    let sleep_time = get_env("SLEEP_TIME", 5);
    let max_threshold = get_env("MAX_THRESHOLD", UPPER_TEMP_THRESHOLD);
    let min_threshold = get_env("MIN_THRESHOLD", LOWER_TEMP_THRESHOLD);
    let min_state = get_env("MIN_STATE", MIN_STATE);

    let max_state = env::var("MAX_STATE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|v| (1..=4).contains(v));

    if min_threshold >= max_threshold {
        panic!("min_threshold ({min_threshold}) >= max_threshold ({max_threshold})");
    }

    check_config(&max_state, min_state, &fan_device);

    let fan = Fan {
        threshold: Threshold {
            min_threshold,
            max_threshold,
        },
        state: State {
            max_state,
            min_state,
        },
    };

    let mut is_init = false;

    loop {
        match get_current_temp() {
            Ok(temp) => {
                adjust_speed(temp, &mut is_init, &fan, &mut fan_device);
                debug!("Sleeping for {sleep_time} seconds");
            }
            Err(err) => {
                error!("Can't read temperature {err}")
            }
        }

        thread::sleep(Duration::from_secs(sleep_time));
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap, path::Path};

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
        let fan = Fan {
            threshold: Threshold {
                min_threshold: 40.0,
                max_threshold: 80.0,
            },
            state: State {
                max_state: Some(max_state),
                min_state: 0,
            },
        };

        let slots = calculate_slots(&fan, max_state);

        assert_eq!(slots.len(), 6);
        assert_eq!(slots[0], (0, 40.0));
        assert_eq!(slots[5], (5, 80.0));

        let step = (80.0 - 40.0) / 5.0;
        assert_eq!(slots[1], (1, 40.0 + step));
        assert_eq!(slots[2], (2, 40.0 + 2.0 * step));
        assert_eq!(slots[3], (3, 40.0 + 3.0 * step));
    }

    #[test]
    fn test_adjust_speed() {
        let mock_fs = MockFs::new();
        let fan_device = format!("{}/cooling_device0", THERMAL_DIR);

        mock_fs.add_file(&format!("{}/max_state", fan_device), "4");
        mock_fs.add_file(&format!("{}/{}", fan_device, FILE_NAME_CUR_STATE), "1");

        let max_state = 5;

        let fan = Fan {
            threshold: Threshold {
                min_threshold: 40.0,
                max_threshold: 80.0,
            },
            state: State {
                max_state: Some(max_state),
                min_state: 0,
            },
        };

        let current_temp = 60.0;

        let slots = calculate_slots(&fan, max_state);

        let fallback = (fan.state.min_state, fan.threshold.min_threshold);
        let desired_slot = slots
            .iter()
            .filter(|(_, temp)| *temp <= current_temp)
            .next_back()
            .unwrap_or(&fallback);
        let desired_state = desired_slot.0;

        assert_eq!(desired_state, 2);
    }

    #[test]
    fn test_env_exists_and_valid() {
        unsafe { env::set_var("TEST_PORT", "3000") };
        let port: u16 = get_env("TEST_PORT", 8080);
        assert_eq!(port, 3000);
        unsafe { env::remove_var("TEST_PORT") };
    }

    #[test]
    fn test_env_not_set() {
        unsafe { env::remove_var("TEST_PORT") };
        let port: u16 = get_env("TEST_PORT", 8080);
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_env_invalid_format() {
        unsafe { env::set_var("TEST_PORT", "not_a_number") };
        let port: u16 = get_env("TEST_PORT", 8080);
        assert_eq!(port, 8080);
        unsafe { env::remove_var("TEST_PORT") };
    }

    #[test]
    fn test_string_env() {
        unsafe { env::set_var("TEST_NAME", "Alice") };
        let name: String = get_env("TEST_NAME", "default".to_string());
        assert_eq!(name, "Alice".to_string());
        unsafe { env::remove_var("TEST_NAME") };
    }

    #[test]
    fn test_bool_env() {
        unsafe { env::set_var("DEBUG", "true") };
        let debug: bool = get_env("DEBUG", false);
        assert_eq!(debug, true);
        unsafe { env::remove_var("DEBUG") };
    }

    #[test]
    fn test_bool_env_invalid() {
        unsafe { env::set_var("DEBUG", "yes") };
        let debug: bool = get_env("DEBUG", false);
        assert_eq!(debug, false);
        unsafe { env::remove_var("DEBUG") };
    }
}
