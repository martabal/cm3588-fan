use std::{env, io::Write, str::FromStr};

use env_logger::Builder;
use log::{Level, LevelFilter, info};

const LOWER_TEMP_THRESHOLD: f64 = 45.0;
const UPPER_TEMP_THRESHOLD: f64 = 65.0;
const MIN_STATE: u32 = 0;

pub struct Config {
    pub threshold: Threshold,
    pub state: State,
    pub sleep_time: u64,
}
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

#[derive(Debug)]
pub struct State {
    pub max: Option<u32>,
    pub min: u32,
}
#[derive(Debug)]
pub struct Threshold {
    pub max: f64,
    pub min: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    fn get_level_color<T: Into<Option<Level>>>(level: T) -> &'static str {
        match level.into() {
            Some(Level::Error) => RED,
            Some(Level::Warn) => YELLOW,
            Some(Level::Info) => GREEN,
            Some(Level::Debug) => BLUE,
            Some(Level::Trace) => CYAN,
            None => RESET,
        }
    }
    fn setup_logging(debug_mode: bool) {
        let level_filter = match env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "trace" => LevelFilter::Trace,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Debug,
        };

        let mut builder = Builder::new();

        if !debug_mode {
            builder.format(|f, r| {
                let color = Self::get_level_color(r.level());
                writeln!(f, "{}{}{}", color, r.args(), RESET)
            });
        }

        builder.filter_level(level_filter).init();

        println!(
            "Log level set to: {}{}{RESET}",
            Self::get_level_color(level_filter.to_level()),
            level_filter
        );

        let msg = format!(
            "Starting PWM Config Control Service v{}",
            env!("CARGO_PKG_VERSION")
        );

        if debug_mode {
            info!("{msg}");
        } else {
            println!("{msg}");
        }
    }
    fn get_env<T: FromStr>(key: &str, fallback: T) -> T {
        env::var(key)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(fallback)
    }
    #[must_use]
    pub fn new() -> Self {
        let debug = Self::get_env("DEBUG", false);
        Self::setup_logging(debug);
        let sleep_time = Self::get_env("SLEEP_TIME", 5);
        let max_threshold = Self::get_env("MAX_THRESHOLD", UPPER_TEMP_THRESHOLD);
        let min_threshold = Self::get_env("MIN_THRESHOLD", LOWER_TEMP_THRESHOLD);
        let min_state = Self::get_env("MIN_STATE", MIN_STATE);

        let max_state = env::var("MAX_STATE")
            .ok()
            .and_then(|s| s.parse::<u32>().ok());
        Self {
            sleep_time,
            threshold: Threshold {
                max: max_threshold,
                min: min_threshold,
            },
            state: State {
                max: max_state,
                min: min_state,
            },
        }
    }

    pub fn check_config(&self, fan_max_state: u32) {
        assert!(
            self.threshold.min < self.threshold.max,
            "min threshold can't be >= max threshold: {} >= {}",
            self.threshold.min,
            self.threshold.max
        );
        if let Some(max) = self.state.max {
            assert!(
                (self.state.min < max),
                "min state can't be >= max state: {} >= {max}",
                self.state.min
            );

            assert!(
                (max <= fan_max_state),
                "Configured max state {max} exceeds device max state {fan_max_state}"
            );
        }

        assert!(
            (self.state.min <= fan_max_state),
            "Configured min state {} exceeds device max state {}",
            self.state.min,
            fan_max_state
        );
    }
}

#[cfg(test)]
mod tests {
    use std::panic;

    use crate::config::Config;

    use super::{State, Threshold};

    fn assert_panics<F: FnOnce() + panic::UnwindSafe>(f: F, msg_contains: &str) {
        let result = panic::catch_unwind(f);
        assert!(
            result.is_err(),
            "Expected panic, but function did not panic"
        );
        let err = result.unwrap_err();
        let err_msg = err
            .downcast_ref::<String>()
            .map(std::string::String::as_str)
            .or_else(|| err.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic>");
        assert!(
            err_msg.contains(msg_contains),
            "Panic message did not contain '{msg_contains}': got '{err_msg}'"
        );
    }

    #[test]
    fn test_valid_config_with_fan_device() {
        let max_state = Some(5);
        let min_state = 3;

        let config: Config = Config {
            threshold: Threshold {
                max: 60.0,
                min: 40.0,
            },
            state: State {
                max: max_state,
                min: min_state,
            },
            sleep_time: 5,
        };
        config.check_config(5);
    }

    #[test]
    fn test_min_state_superior_to_device_max_state() {
        let max_state = 5;
        let min_state = 6;
        let config: Config = Config {
            threshold: Threshold {
                max: 60.0,
                min: 40.0,
            },
            state: State {
                max: None,
                min: min_state,
            },
            sleep_time: 5,
        };
        let msg_contains = format!(
            "Configured min state {min_state} exceeds device max state {max_state}"
        );
        assert_panics(|| config.check_config(5), &msg_contains);
    }

    #[test]
    fn test_min_state_greater_than_or_equal_to_max_panics() {
        let max_state = Some(3);
        let min_state = 3;

        let config: Config = Config {
            threshold: Threshold {
                max: 60.0,
                min: 40.0,
            },
            state: State {
                max: max_state,
                min: min_state,
            },
            sleep_time: 5,
        };

        assert_panics(|| config.check_config(5), "min state can't be >=");
    }

    #[test]
    fn test_max_state_exceeds_device_max_panics() {
        let max_state = Some(6);
        let min_state = 3;

        let config: Config = Config {
            threshold: Threshold {
                max: 60.0,
                min: 40.0,
            },
            state: State {
                max: max_state,
                min: min_state,
            },
            sleep_time: 5,
        };

        assert_panics(|| config.check_config(5), "exceeds device max state");
    }

    #[test]
    fn threshold_min_exceeds_threshold_max_panics() {
        let max_state = Some(5);
        let min_state = 0;

        let config: Config = Config {
            threshold: Threshold {
                max: 60.0,
                min: 80.0,
            },
            state: State {
                max: max_state,
                min: min_state,
            },
            sleep_time: 5,
        };

        assert_panics(|| config.check_config(5), "min threshold can't be >=");
    }

    #[test]
    fn test_no_max_state_does_nothing() {
        let max_state = None;
        let min_state = 0;

        let config: Config = Config {
            threshold: Threshold {
                max: 60.0,
                min: 40.0,
            },
            state: State {
                max: max_state,
                min: min_state,
            },
            sleep_time: 5,
        };

        config.check_config(5);
    }
}
