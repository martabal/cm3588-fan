use core::fmt;
use log::info;
use std::{
    fs::{self, File},
    io::{self, Read},
    num::ParseFloatError,
    path::PathBuf,
};

use crate::{THERMAL_DIR, config::MAX_STATE};

pub struct Temp {
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Parse(ParseFloatError),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Self::Parse(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Parse(e) => write!(f, "Parse error: {e}"),
        }
    }
}

pub const MAX_LEVEL: usize = MAX_STATE as usize + 1;

const THERMAL_ZONE_NAME: &str = "thermal_zone";

impl Temp {
    pub fn new() -> io::Result<Self> {
        let path = Self::get_temp_path()?;
        Ok(Self { path })
    }

    pub fn get_current_temp(&self, buf: &mut String) -> Result<f32, Error> {
        buf.clear();
        File::open(&self.path)?.read_to_string(buf)?;
        Ok(buf.trim().parse::<f32>()? / 1000.0)
    }

    pub fn get_temp_path() -> io::Result<PathBuf> {
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

        Err(io::Error::other("No valid thermal zone found"))
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
}
