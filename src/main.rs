use std::{fs, thread, time::Duration};

use log::{debug, error, info, trace};

use cm3588_fan::{FILE_NAME_CUR_STATE, config::Config, fan::Fan, temp::Temp};

fn adjust_speed(
    is_init: &mut bool,
    config: &Config,
    fan_device: &mut Option<Fan>,
    temp_device: &mut Option<Temp>,
) {
    if fan_device.is_none() {
        if let Some(path) = Fan::get_fan_device() {
            trace!("New fan device detected");
            *fan_device = Some(Fan::new_fan_device(path, config));
        } else {
            error!("Still no new fan device");
            return;
        }
    }

    let fan = fan_device.as_mut().unwrap();

    let file_content = match fs::read_to_string(format!("{}/{}", fan.path, FILE_NAME_CUR_STATE)) {
        Ok(content) => content,
        Err(e) => {
            error!("Device is not available {e}");
            *fan_device = None;
            return;
        }
    };

    if temp_device.is_none() {
        if let Ok(new_temp_device) = Temp::new() {
            trace!("New temp device detected");
            *temp_device = Some(new_temp_device);
        } else {
            error!("Still no new fan device");
            return;
        }
    }

    let temp = temp_device.as_ref().unwrap();

    let current_temp = match temp.get_current_temp() {
        Ok(temp) => temp,
        Err(err) => {
            error!("Can't read temperature {err}, trying to re-read again");
            match Temp::get_temp_path() {
                Ok(path) => {
                    if let Some(device) = temp_device {
                        device.path = path;
                    }
                }
                Err(err) => {
                    error!("Can't get temp path: {err}");
                }
            }

            return;
        }
    };

    match file_content.trim().parse::<u32>() {
        Ok(speed) => {
            let desired_state: u32 = fan.choose_speed(current_temp, config);
            if let Some(last_state) = fan.last_state {
                if last_state == desired_state {
                    trace!("state didn't change compared to the last time");
                    return;
                }
            }
            if speed != desired_state || !*is_init {
                info!("Adjusting fan speed to {desired_state} (Temp: {current_temp:.2}°C)");
                match fs::write(
                    format!("{}/{FILE_NAME_CUR_STATE}", fan.path),
                    desired_state.to_string(),
                ) {
                    Ok(_) => fan.last_state = Some(desired_state),
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
    let mut temp = match Temp::new() {
        Ok(temp) => Some(temp),
        Err(err) => {
            error!("Can't read temperature: {err}");
            None
        }
    };

    let mut is_init = false;

    loop {
        adjust_speed(&mut is_init, &config, &mut fan_device, &mut temp);
        debug!("Sleeping for {} seconds", config.sleep_time);

        thread::sleep(Duration::from_secs(config.sleep_time));
    }
}
