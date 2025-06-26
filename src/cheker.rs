use std::fs;

use log::{debug, error, info, trace};

use crate::{FILE_NAME_CUR_STATE, config::Config, fan::Fan, temp::Temp};

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
            if let Some(path) = Fan::get_fan_device() {
                trace!("New fan device detected");
                self.fan_device = Some(Fan::new_fan_device(path, &self.config));
            } else {
                error!("Still no new fan device");
                return;
            }
        }

        let fan = self.fan_device.as_mut().unwrap();

        let file_content = match fs::read_to_string(format!("{}/{}", fan.path, FILE_NAME_CUR_STATE))
        {
            Ok(content) => content,
            Err(e) => {
                error!("Device is not available {e}");
                self.fan_device = None;
                return;
            }
        };

        if self.temp_device.is_none() {
            if let Ok(new_temp_device) = Temp::new() {
                trace!("New temp device detected");
                self.temp_device = Some(new_temp_device);
            } else {
                error!("Still no new fan device");
                return;
            }
        }

        let temp = self.temp_device.as_ref().unwrap();

        let current_temp: f64 = match temp.get_current_temp() {
            Ok(temp) => temp,
            Err(err) => {
                error!("Can't read temperature {err}, trying to re-read again");

                match Temp::get_temp_path() {
                    Ok(path) => {
                        if let Some(device) = &mut self.temp_device {
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
                let desired_state: u32 = fan.choose_speed(current_temp, &self.config);
                if let Some(last_state) = fan.last_state
                    && last_state == desired_state
                {
                    trace!("state didn't change compared to the last time");
                    return;
                }
                if speed != desired_state || !self.is_init {
                    info!("Adjusting fan speed to {desired_state} (Temp: {current_temp:.2}°C)");
                    match fs::write(
                        format!("{}/{FILE_NAME_CUR_STATE}", fan.path),
                        desired_state.to_string(),
                    ) {
                        Ok(_) => fan.last_state = Some(desired_state),
                        Err(err) => {
                            error!("Can't set the speed to device {} {err}", fan.path);
                            self.fan_device = None;
                        }
                    }
                } else {
                    debug!("Temp: {current_temp:.2}°C, no speed change needed");
                }

                if !self.is_init {
                    debug!("Setting the speed for the first time!");
                    self.is_init = true;
                }
            }
            Err(e) => {
                error!("Can't parse speed value {e}");
            }
        }
    }
}
