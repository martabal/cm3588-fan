use std::{fs, thread, time::Duration};

use log::{debug, error, info, trace};

use cm3588_fan::{FILE_NAME_CUR_STATE, config::Config, fan::Fan};

fn adjust_speed(is_init: &mut bool, config: &Config, fan_device: &mut Option<Fan>) {
    if fan_device.is_none() {
        if let Some(path) = Fan::get_fan_device() {
            *fan_device = Some(Fan::new_fan_device(path, config));
        } else {
            return;
        }
    }

    let fan = fan_device.as_ref().unwrap();

    let file_content = match fs::read_to_string(format!("{}/{}", fan.fan_path, FILE_NAME_CUR_STATE))
    {
        Ok(content) => content,
        Err(e) => {
            error!("Device is not available {e}");
            *fan_device = None;
            return;
        }
    };

    let current_temp = match fan.get_current_temp() {
        Ok(temp) => temp,
        Err(err) => {
            error!("Can't read temperature {err}, trying to re-read again");
            match Fan::get_temp_path() {
                Ok(path) => {
                    if let Some(device) = fan_device {
                        device.temp_path = path;
                    }
                }
                Err(path_err) => {
                    error!("Can't get temp path: {path_err}");
                }
            }

            return;
        }
    };

    match file_content.trim().parse::<u32>() {
        Ok(speed) => {
            let desired_state: u32 = match current_temp {
                t if t <= config.threshold.min => {
                    trace!("min state desired");
                    config.state.min
                }
                t if t <= config.threshold.max => {
                    trace!("desired state in slots");
                    fan.temp_slots
                        .iter()
                        .rev()
                        .find(|(_, temp)| *temp <= current_temp)
                        .map(|(state, _)| *state)
                        .unwrap_or(config.state.min)
                }
                _ => {
                    trace!("max state desired {}", fan.max_state);
                    config.state.max.unwrap_or(fan.max_state)
                }
            };

            if speed != desired_state || !*is_init {
                info!("Adjusting fan speed to {desired_state} (Temp: {current_temp:.2}°C)");
                match fs::write(
                    format!("{}/{FILE_NAME_CUR_STATE}", fan.fan_path),
                    desired_state.to_string(),
                ) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Can't set the speed to device {} {err}", fan.fan_path);
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
        adjust_speed(&mut is_init, &config, &mut fan_device);
        debug!("Sleeping for {} seconds", config.sleep_time);

        thread::sleep(Duration::from_secs(config.sleep_time));
    }
}
