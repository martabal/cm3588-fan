use std::{fs, thread, time::Duration};

use log::{debug, error, info, trace};

use cm3588_fan::{FILE_NAME_CUR_STATE, config::Config, fan::Fan};

fn adjust_speed(
    current_temp: f64,
    is_init: &mut bool,
    config: &Config,
    fan_device: &mut Option<Fan>,
) {
    if fan_device.is_none() {
        if let Some(path) = Fan::get_fan_device() {
            *fan_device = Some(Fan::new_fan_device(path, config));
        } else {
            return;
        }
    }

    let fan = fan_device.as_ref().unwrap();

    let file_content = match fs::read_to_string(format!("{}/{}", fan.path, FILE_NAME_CUR_STATE)) {
        Ok(content) => content,
        Err(e) => {
            error!("Device is not available {e}");
            *fan_device = None;
            return;
        }
    };

    match file_content.trim().parse::<u32>() {
        Ok(speed) => {
            let desired_state: u32 = match current_temp {
                t if t <= config.threshold.min_threshold => {
                    trace!("min state desired");
                    config.state.min_state
                }
                t if t <= config.threshold.max_threshold => {
                    trace!("desired state in slots");
                    fan.temp_slots
                        .iter()
                        .rev()
                        .find(|(_, temp)| *temp <= current_temp)
                        .map(|(state, _)| *state)
                        .unwrap_or(config.state.min_state)
                }
                _ => {
                    trace!("max state desired {}", fan.max_state);
                    config.state.max_state.unwrap_or(fan.max_state)
                }
            };

            if speed != desired_state || !*is_init {
                info!("Adjusting fan speed to {desired_state} (Temp: {current_temp:.2}°C)");
                match fs::write(
                    format!("{}/{FILE_NAME_CUR_STATE}", fan.path),
                    desired_state.to_string(),
                ) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Can't set the speed to device {} {err}", fan.path);
                        *fan_device = None;
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

fn main() {
    let config = Config::new();

    let mut fan_device = Fan::new(&config);

    let mut is_init = false;

    loop {
        match Fan::get_current_temp() {
            Ok(temp) => {
                adjust_speed(temp, &mut is_init, &config, &mut fan_device);
            }
            Err(err) => {
                error!("Can't read temperature {err}")
            }
        }
        debug!("Sleeping for {} seconds", config.sleep_time);

        thread::sleep(Duration::from_secs(config.sleep_time));
    }
}
