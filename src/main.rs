use env_logger::Builder;
use log::{LevelFilter, debug, error, info, trace};
use std::env;
use std::{fs, thread, time::Duration};

const THERMAL_DIR: &str = "/sys/class/thermal";
const DEVICE_TYPE_PWM_FAN: &str = "pwm-fan";
const THERMAL_ZONE_NAME: &str = "thermal_zone";
const DEVICE_NAME_COOLING: &str = "cooling_device";
const FILE_NAME_CUR_STATE: &str = "cur_state";
const LOWER_TEMP_THRESHOLD: f64 = 45.0;
const UPPER_TEMP_THRESHOLD: f64 = 65.0;
const MIN_STATE: u32 = 0;

struct Config {
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

fn setup_logging() {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    let level_filter = match log_level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Debug, // Default to Debug if invalid or missing
    };

    Builder::new().filter_level(level_filter).init();
}

fn get_temperature_slots(config: &Config) -> Vec<(u32, f64)> {
    let fan_device = match get_fan_device() {
        Some(device) => device,
        None => {
            error!("No PWM fan device found");
            return vec![];
        }
    };

    let max_state: u32 = if let Some(max) = config.state.max_state {
        max
    } else {
        fs::read_to_string(format!("{}/max_state", fan_device))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    };

    trace!("max_state: {max_state}");

    if max_state == 0 {
        error!("max_state could not be determined for {}", fan_device);
        return vec![];
    }

    let step = (config.threshold.max_threshold - config.threshold.min_threshold) / max_state as f64;

    let slots = (0..=max_state)
        .map(|x| {
            (
                x + config.state.min_state,
                x as f64 * step + config.threshold.min_threshold,
            )
        })
        .collect();

    trace!("Slots: {:?}", slots);
    slots
}

fn get_fan_device() -> Option<String> {
    let entries = fs::read_dir(THERMAL_DIR).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name()?.to_str()?.starts_with(DEVICE_NAME_COOLING) {
            let type_file = path.join("type");
            if let Ok(content) = fs::read_to_string(type_file) {
                if content.trim() == DEVICE_TYPE_PWM_FAN {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn get_fan_speed(device: &str) -> u32 {
    let cur_state_file = format!("{}/{}", device, FILE_NAME_CUR_STATE);
    fs::read_to_string(cur_state_file)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn set_fan_speed(device: &str, speed: &str) {
    let cur_state_file = format!("{}/{}", device, FILE_NAME_CUR_STATE);
    fs::write(cur_state_file, speed).unwrap()
}

fn get_current_temp() -> f64 {
    let mut temps = vec![];
    if let Ok(entries) = fs::read_dir(THERMAL_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(THERMAL_ZONE_NAME)
            {
                let temp_file = path.join("temp");
                if let Ok(content) = fs::read_to_string(temp_file) {
                    if let Ok(temp) = content.trim().parse::<f64>() {
                        temps.push(temp / 1000.0);
                    }
                }
            }
        }
    }
    temps.into_iter().fold(0.0, f64::max)
}

fn adjust_speed(current_temp: f64, is_init: &mut bool, state: &Config) {
    let slots = get_temperature_slots(state);
    let fallback = (state.state.min_state, state.threshold.min_threshold);
    let desired_slot = slots
        .iter()
        .filter(|(_, temp)| *temp <= current_temp)
        .last()
        .unwrap_or_else(|| slots.first().unwrap_or(&fallback));
    let desired_state = desired_slot.0;

    let fan_device = match get_fan_device() {
        Some(device) => device,
        None => return,
    };

    if get_fan_speed(&fan_device) != desired_state || !(*is_init) {
        info!("Adjusting fan speed to {desired_state} (Temp: {current_temp:.2}°C)",);
        set_fan_speed(&fan_device, &desired_state.to_string());
    } else {
        debug!("Temp: {current_temp:.2}°C, not changing the speed");
    }
    if !(*is_init) {
        debug!("Setting the speed for the first time!");
        *is_init = true;
    }
}

fn main() {
    setup_logging();
    info!("Starting PWM Fan Control Service");
    let fan_device = match get_fan_device() {
        Some(device) => device,
        None => {
            error!("No PWM fan device found");
            return;
        }
    };
    info!("Fan device: {fan_device}");
    let sleep_time = env::var("SLEEP_TIME")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    let max_threshold = env::var("MAX_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(UPPER_TEMP_THRESHOLD);
    let min_threshold = env::var("MIN_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(LOWER_TEMP_THRESHOLD);
    let min_state = env::var("MIN_STATE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(MIN_STATE);
    let max_state: Option<u32> = env::var("MAX_STATE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&s| (1..=4).contains(&s));

    if let Some(max_state) = max_state {
        if min_state > max_state {
            panic!(
                "min_state can't be superior to max_state. min_state: {min_state}, max_state: {max_state}"
            );
        }
    }
    let mut is_init = false;
    let config = Config {
        threshold: Threshold {
            min_threshold,
            max_threshold,
        },
        state: State {
            max_state,
            min_state,
        },
    };

    loop {
        let current_temp = get_current_temp();
        adjust_speed(current_temp, &mut is_init, &config);
        debug!("Sleeping for {sleep_time} seconds");
        thread::sleep(Duration::from_secs(sleep_time));
    }
}
