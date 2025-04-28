use std::{error::Error, fs};

use log::info;

use crate::{THERMAL_DIR, THERMAL_ZONE_NAME};

pub struct Temp {
    pub path: String,
}

impl Temp {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        return match Self::get_temp_path() {
            Ok(path) => Ok(Temp { path }),
            Err(err) => Err(err),
        };
    }

    pub fn get_current_temp(&self) -> Result<f64, Box<dyn Error>> {
        let temp = fs::read_to_string(&self.path)?.trim().parse::<f64>()? / 1000.0;
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
                        let path: String = temp_path.to_string_lossy().into_owned();
                        info!("Temp path: {path}");
                        return Ok(path);
                    }
                }
            }
        }

        Err(Box::from("No valid thermal zone found"))
    }
}
