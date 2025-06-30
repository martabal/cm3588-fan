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
                error!("Still no fan device available");
                return;
            }
        }

        let fan = self.fan_device.as_mut().unwrap();
        let current_speed: u32 = match fs::read_to_string(&fan.path) {
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
            match Temp::new() {
                Ok(device) => {
                    trace!("New temp device detected");
                    self.temp_device = Some(device);
                }
                Err(_) => {
                    error!("Still no temp device available");
                    return;
                }
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
            if fs::write(&fan.path, desired_speed.to_string()).is_ok() {
                fan.last_state = Some(desired_speed);
            } else {
                error!("Can't set speed on device {}", fan.path);
                self.fan_device = None;
            }
        } else {
            debug!("Temp: {current_temp:.2}°C, no speed change needed");
        }
    }
}
