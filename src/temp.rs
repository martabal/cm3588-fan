use log::info;
use std::{error::Error, fs, path::PathBuf};

use crate::THERMAL_DIR;

pub struct Temp {
    pub path: PathBuf,
}

const THERMAL_ZONE_NAME: &str = "thermal_zone";

impl Temp {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let path = Self::get_temp_path()?;
        Ok(Self { path })
    }

    pub fn get_current_temp(&self) -> Result<f64, Box<dyn Error>> {
        let temp = fs::read_to_string(&self.path)?.trim().parse::<f64>()? / 1000.0;
        Ok(temp)
    }

    pub fn get_temp_path() -> Result<PathBuf, Box<dyn Error>> {
        for entry in fs::read_dir(THERMAL_DIR)? {
            let entry = entry?;
            let path = entry.path();

            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with(THERMAL_ZONE_NAME))
            {
                let temp_path = path.join("temp");

                if let Ok(content) = fs::read_to_string(&temp_path)
                    && content.trim().parse::<f64>().is_ok()
                {
                    info!("Temp path: {}", temp_path.display());
                    return Ok(temp_path);
                }
            }
        }

        Err("No valid thermal zone found".into())
    }
}
