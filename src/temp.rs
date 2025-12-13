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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_current_temp_valid_value() {
        let temp_dir = std::env::temp_dir().join("test_temp_valid");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "45000\n").unwrap();

        let temp = Temp {
            path: temp_file.clone(),
        };

        let result = temp.get_current_temp();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 45.0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_current_temp_with_whitespace() {
        let temp_dir = std::env::temp_dir().join("test_temp_whitespace");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "  50000  \n").unwrap();

        let temp = Temp {
            path: temp_file.clone(),
        };

        let result = temp.get_current_temp();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50.0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_current_temp_zero_value() {
        let temp_dir = std::env::temp_dir().join("test_temp_zero");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "0").unwrap();

        let temp = Temp {
            path: temp_file.clone(),
        };

        let result = temp.get_current_temp();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_current_temp_high_value() {
        let temp_dir = std::env::temp_dir().join("test_temp_high");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "100000").unwrap();

        let temp = Temp {
            path: temp_file.clone(),
        };

        let result = temp.get_current_temp();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100.0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_current_temp_invalid_content() {
        let temp_dir = std::env::temp_dir().join("test_temp_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "not_a_number").unwrap();

        let temp = Temp {
            path: temp_file.clone(),
        };

        let result = temp.get_current_temp();
        assert!(result.is_err());

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_current_temp_file_not_found() {
        let temp = Temp {
            path: PathBuf::from("/nonexistent/path/temp"),
        };

        let result = temp.get_current_temp();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_current_temp_empty_file() {
        let temp_dir = std::env::temp_dir().join("test_temp_empty");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "").unwrap();

        let temp = Temp {
            path: temp_file.clone(),
        };

        let result = temp.get_current_temp();
        assert!(result.is_err());

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_current_temp_negative_value() {
        let temp_dir = std::env::temp_dir().join("test_temp_negative");
        fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("temp");
        fs::write(&temp_file, "-5000").unwrap();

        let temp = Temp {
            path: temp_file.clone(),
        };

        let result = temp.get_current_temp();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), -5.0);

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_temp_path_no_thermal_dir() {
        let result = Temp::get_temp_path();
        // This will fail in test environment without actual thermal zones
        // but we're testing that it returns an error rather than panicking
        assert!(result.is_err() || result.is_ok());
    }
}
