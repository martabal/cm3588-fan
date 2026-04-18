use log::info;
use std::{
    error::Error,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use crate::THERMAL_DIR;

pub struct Temp {
    pub path: PathBuf,
}

const THERMAL_ZONE_NAME: &str = "thermal_zone";

impl Temp {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Self::new_from(THERMAL_DIR)
    }

    pub fn new_from(dir: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let path = Self::get_temp_path_from(dir)?;
        Ok(Self { path })
    }

    pub fn get_current_temp(&self, buf: &mut String) -> Result<f32, Box<dyn Error>> {
        buf.clear();
        File::open(&self.path)?.read_to_string(buf)?;
        Ok(buf.trim().parse::<f32>()? / 1000.0)
    }

    pub fn get_temp_path() -> Result<PathBuf, Box<dyn Error>> {
        Self::get_temp_path_from(THERMAL_DIR)
    }

    pub fn get_temp_path_from(dir: impl AsRef<Path>) -> Result<PathBuf, Box<dyn Error>> {
        for entry in fs::read_dir(dir.as_ref())? {
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

    struct TempTestDir {
        path: PathBuf,
    }

    impl TempTestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(name);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn create_temp_file(&self, content: &str) -> PathBuf {
            let temp_file = self.path.join("temp");
            fs::write(&temp_file, content).unwrap();
            temp_file
        }
    }

    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_get_current_temp_valid_value() {
        let test_dir = TempTestDir::new("test_temp_valid");
        let temp_file = test_dir.create_temp_file("45000\n");

        let temp = Temp { path: temp_file };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 45.0);
    }

    #[test]
    fn test_get_current_temp_with_whitespace() {
        let test_dir = TempTestDir::new("test_temp_whitespace");
        let temp_file = test_dir.create_temp_file("  50000  \n");

        let temp = Temp { path: temp_file };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50.0);
    }

    #[test]
    fn test_get_current_temp_zero_value() {
        let test_dir = TempTestDir::new("test_temp_zero");
        let temp_file = test_dir.create_temp_file("0");

        let temp = Temp { path: temp_file };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.0);
    }

    #[test]
    fn test_get_current_temp_high_value() {
        let test_dir = TempTestDir::new("test_temp_high");
        let temp_file = test_dir.create_temp_file("100000");

        let temp = Temp { path: temp_file };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100.0);
    }

    #[test]
    fn test_get_current_temp_invalid_content() {
        let test_dir = TempTestDir::new("test_temp_invalid");
        let temp_file = test_dir.create_temp_file("not_a_number");

        let temp = Temp { path: temp_file };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_current_temp_file_not_found() {
        let temp = Temp {
            path: PathBuf::from("/nonexistent/path/temp"),
        };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_current_temp_empty_file() {
        let test_dir = TempTestDir::new("test_temp_empty");
        let temp_file = test_dir.create_temp_file("");

        let temp = Temp { path: temp_file };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_current_temp_negative_value() {
        let test_dir = TempTestDir::new("test_temp_negative");
        let temp_file = test_dir.create_temp_file("-5000");

        let temp = Temp { path: temp_file };

        let mut buf = String::new();
        let result = temp.get_current_temp(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), -5.0);
    }

    // ── get_temp_path_from tests ─────────────────────────────────────────────

    fn create_thermal_zone(root: &PathBuf, zone: &str, temp_content: &str) {
        let zone_dir = root.join(zone);
        fs::create_dir_all(&zone_dir).unwrap();
        fs::write(zone_dir.join("temp"), temp_content).unwrap();
    }

    #[test]
    fn test_get_temp_path_from_valid_zone() {
        let root = TempTestDir::new("test_temp_path_valid_zone");
        create_thermal_zone(&root.path, "thermal_zone0", "45000\n");

        let result = Temp::get_temp_path_from(&root.path);
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("temp"));
    }

    #[test]
    fn test_get_temp_path_from_no_valid_zone() {
        let root = TempTestDir::new("test_temp_path_no_valid");
        // Zone exists but temp file contains non-numeric content.
        let zone_dir = root.path.join("thermal_zone0");
        fs::create_dir_all(&zone_dir).unwrap();
        fs::write(zone_dir.join("temp"), "not_a_number").unwrap();

        let result = Temp::get_temp_path_from(&root.path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_temp_path_from_ignores_non_zone_dirs() {
        let root = TempTestDir::new("test_temp_path_ignores_non_zone");
        // Only a directory that does NOT start with "thermal_zone".
        let other_dir = root.path.join("cooling_device0");
        fs::create_dir_all(&other_dir).unwrap();
        fs::write(other_dir.join("temp"), "45000").unwrap();

        let result = Temp::get_temp_path_from(&root.path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_temp_path_from_nonexistent_dir() {
        let result = Temp::get_temp_path_from("/nonexistent/thermal/dir");
        assert!(result.is_err());
    }

    #[test]
    fn test_new_from_valid_dir() {
        let root = TempTestDir::new("test_new_from_valid");
        create_thermal_zone(&root.path, "thermal_zone0", "55000\n");

        let result = Temp::new_from(&root.path);
        assert!(result.is_ok());
    }
}
