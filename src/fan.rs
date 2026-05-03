use crate::{THERMAL_DIR, config::Config, temp::MAX_LEVEL};
use log::{error, info, trace};
use std::{
    fmt, fs,
    io::{self, Read},
    num::ParseIntError,
    path::{Path, PathBuf},
};

const FILE_NAME_CUR_STATE: &str = "cur_state";
const DEVICE_NAME_COOLING: &str = "cooling_device";
const DEVICE_TYPE_PWM_FAN: &str = "pwm-fan";

pub struct Fan {
    pub path: PathBuf,
    pub state: PathBuf,
    pub max_state: u8,
    pub temp_slots: [Option<(u8, f32)>; MAX_LEVEL],
    pub last_state: Option<u8>,
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Parse(ParseIntError),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
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

impl Fan {
    fn get_device_max_state(device: impl AsRef<Path>) -> Result<u8, Error> {
        let path = device.as_ref().to_path_buf().join("max_state");

        let mut file = fs::File::open(&path)?;
        let mut buf = [0u8; 3]; // u8 max is "255" — 3 bytes
        let n = file.read(&mut buf)?;

        let s = std::str::from_utf8(&buf[..n])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .trim();

        s.parse::<u8>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e).into())
    }

    #[must_use]
    pub fn new_fan_device(state: PathBuf, path: PathBuf, config: &Config) -> Self {
        let max_state = Self::get_device_max_state(&path).unwrap();
        config.check_config(max_state);

        let temp_slots = Self::get_temperature_slots(config, max_state);
        Self {
            path,
            state,
            max_state,
            temp_slots,
            last_state: None,
        }
    }

    fn calculate_slots(config: &Config, max_state: u8) -> [Option<(u8, f32)>; MAX_LEVEL] {
        let num_slots: usize = (config.state.max.unwrap_or(max_state) - config.state.min).into();

        let step = if num_slots <= 1 {
            0.0
        } else {
            (config.threshold.max - config.threshold.min) / (num_slots - 1) as f32
        };

        trace!(
            "Calculate slots, min_state: {}, num_slots: {}, step: {}",
            config.state.min, num_slots, step
        );

        let mut results = [None; MAX_LEVEL];

        for (i, result) in results
            .iter_mut()
            .enumerate()
            .take(num_slots.min(MAX_LEVEL))
        {
            let state = config
                .state
                .min
                .saturating_add(u8::try_from(i).unwrap() + 1);

            let value = if num_slots <= 1 {
                config.threshold.min
            } else {
                (i as f32).mul_add(step, config.threshold.min)
            };

            *result = Some((state, value));
        }

        results
    }

    #[must_use]
    pub fn get_fan_device() -> Option<(PathBuf, PathBuf)> {
        fs::read_dir(THERMAL_DIR).ok()?.flatten().find_map(|entry| {
            let entry_path = entry.path();
            if !entry_path
                .file_name()?
                .to_str()?
                .starts_with(DEVICE_NAME_COOLING)
            {
                return None;
            }

            let mut file = fs::File::open(entry_path.join("type")).ok()?;
            let mut buf = [0u8; 32]; // enough for any thermal device type name
            let n = file.read(&mut buf).ok()?;
            let content = std::str::from_utf8(&buf[..n]).ok()?.trim();

            if content == DEVICE_TYPE_PWM_FAN {
                Some((entry_path.join(FILE_NAME_CUR_STATE), entry_path))
            } else {
                None
            }
        })
    }

    fn get_temperature_slots(config: &Config, max_state: u8) -> [Option<(u8, f32)>; MAX_LEVEL] {
        let max_state = config.state.max.unwrap_or(max_state);
        trace!("max_state: {max_state}");
        if max_state == 0 {
            error!("max_state could not be determined");
            return [None; MAX_LEVEL];
        }
        let slots = Self::calculate_slots(config, max_state);
        trace!("Slots: {slots:?}");
        slots
    }

    #[must_use]
    pub fn new(config: &Config) -> Option<Self> {
        if let Some((state, path)) = Self::get_fan_device() {
            info!("Fan device: {}", path.display());
            Some(Self::new_fan_device(state, path, config))
        } else {
            error!("No PWM fan device found");
            None
        }
    }

    #[must_use]
    pub fn choose_speed(&self, current_temp: f32, config: &Config) -> u8 {
        match current_temp {
            t if t < config.threshold.min => {
                trace!("Min state desired");
                config.state.min
            }
            t if t <= config.threshold.max => {
                trace!("Desired state in slots");
                self.temp_slots
                    .iter()
                    .flatten()
                    .rev()
                    .find(|(_, temp)| *temp <= current_temp)
                    .map_or(config.state.min, |(state, _)| *state)
            }
            _ => {
                trace!("Max state desired {}", self.max_state);
                config.state.max.unwrap_or(self.max_state)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::config::{DEFAULT_MAX_STATE, DEFAULT_SLEEP_TIME, State, Threshold};

    use super::*;

    fn rest_is_none(slots: [Option<(u8, f32)>; MAX_LEVEL], index: usize) {
        assert!(
            slots
                .get(index..)
                .map_or(true, |rest| rest.iter().all(|x| x.is_none()))
        );
    }

    #[test]
    fn test_check_config() {
        let max_state = Some(4);
        let min_state = 0;

        let panic_occurred = max_state.is_some_and(|max| min_state >= max);

        assert!(!panic_occurred);
        let max_state = Some(2);
        let min_state = 3;

        let mut panic_occurred = false;
        if let Some(max) = max_state
            && min_state >= max
        {
            panic_occurred = true;
        }

        assert!(panic_occurred);
    }

    #[test]
    fn test_get_temperature_slots() {
        let min_threshold = 40.0;
        let max_threshold = 80.0;
        let fan = Config {
            sleep_time: DEFAULT_SLEEP_TIME,
            threshold: Threshold {
                max: max_threshold,
                min: min_threshold,
            },
            state: State {
                max: Some(DEFAULT_MAX_STATE),
                min: 0,
            },
        };

        let slots = Fan::calculate_slots(&fan, DEFAULT_MAX_STATE);

        assert_eq!(slots[0].unwrap(), (1, 40.0));
        assert_eq!(slots[4].unwrap(), (5, max_threshold));

        let step = (max_threshold - min_threshold) / 4.0;
        assert_eq!(slots[1].unwrap(), (2, min_threshold + step));
        assert_eq!(slots[2].unwrap(), (3, 2.0f32.mul_add(step, min_threshold)));
        assert_eq!(slots[3].unwrap(), (4, 3.0f32.mul_add(step, min_threshold)));
    }

    #[test]
    fn test_get_temperature_one_slot() {
        let min_state = 4;

        let min_threshold = 40.0;
        let max_threshold = 80.0;
        let fan = Config {
            sleep_time: DEFAULT_SLEEP_TIME,
            threshold: Threshold {
                max: max_threshold,
                min: min_threshold,
            },
            state: State {
                max: Some(DEFAULT_MAX_STATE),
                min: min_state,
            },
        };

        let slots = Fan::calculate_slots(&fan, DEFAULT_MAX_STATE);

        rest_is_none(slots, 1);
        assert_eq!(slots[0].unwrap(), (DEFAULT_MAX_STATE, min_threshold));
    }

    #[test]
    fn test_get_temperature_no_slots() {
        let min_state = 5;
        let min_threshold = 40.0;
        let max_threshold = 80.0;
        let fan = Config {
            sleep_time: DEFAULT_SLEEP_TIME,
            threshold: Threshold {
                max: max_threshold,
                min: min_threshold,
            },
            state: State {
                max: Some(DEFAULT_MAX_STATE),
                min: min_state,
            },
        };

        let slots = Fan::calculate_slots(&fan, DEFAULT_MAX_STATE);

        rest_is_none(slots, 0);
    }

    #[test]
    fn test_adjust_speed() {
        let config = Config {
            sleep_time: DEFAULT_SLEEP_TIME,
            threshold: Threshold {
                max: 80.0,
                min: 40.0,
            },
            state: State {
                max: Some(DEFAULT_MAX_STATE),
                min: 0,
            },
        };

        let current_temp = 60.0;

        let slots = Fan::calculate_slots(&config, DEFAULT_MAX_STATE);

        let fan = Fan {
            temp_slots: slots,
            max_state: DEFAULT_MAX_STATE,
            path: "cooling_device".into(),
            state: "cooling_device/cur_state".into(),
            last_state: None,
        };
        let desired_state = fan.choose_speed(current_temp, &config);

        assert_eq!(desired_state, 3);
    }

    fn setup_test_config() -> Config {
        Config {
            threshold: Threshold {
                min: 45.0,
                max: 70.0,
            },
            state: State {
                min: 0,
                max: Some(DEFAULT_MAX_STATE),
            },
            sleep_time: DEFAULT_SLEEP_TIME,
        }
    }

    fn setup_test_fan() -> Fan {
        let temp_slots = [
            Some((0, 45.0)),
            Some((1, 50.0)),
            Some((2, 55.0)),
            Some((3, 60.0)),
            Some((4, 65.0)),
            Some((5, 70.0)),
        ];

        Fan {
            temp_slots,
            max_state: DEFAULT_MAX_STATE,
            path: "cooling_device".into(),
            state: "cooling_device/cur_state".into(),
            last_state: None,
        }
    }

    #[test]
    fn test_temp_below_min_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(25.0, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_temp_at_min_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(45.0, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_temp_in_between_slots() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(52.0, &config);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_temp_above_max_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(80.0, &config);
        assert_eq!(result, config.state.max.unwrap());
    }

    #[test]
    fn test_temp_at_max_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(75.0, &config);
        assert_eq!(result, config.state.max.unwrap());
    }

    #[test]
    fn test_with_empty_slots() {
        let config = Config {
            threshold: Threshold {
                min: 45.0,
                max: 70.0,
            },
            state: State {
                min: 2,
                max: Some(2),
            },
            sleep_time: DEFAULT_SLEEP_TIME,
        };

        let fan = Fan {
            temp_slots: [None; MAX_LEVEL],
            max_state: DEFAULT_MAX_STATE,
            path: "cooling_device".into(),
            state: "cooling_device/cur_state".into(),
            last_state: None,
        };

        let result = fan.choose_speed(80.0, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_temp_exactly_at_slot_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(60.0, &config);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_temp_slightly_above_slot_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(60.1, &config);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_temp_slightly_below_min_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(44.9, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_temp_just_above_min_threshold() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(45.1, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_calculate_slots_with_two_slots() {
        let config = Config {
            threshold: Threshold {
                min: 40.0,
                max: 60.0,
            },
            state: State {
                max: Some(2),
                min: 0,
            },
            sleep_time: DEFAULT_SLEEP_TIME,
        };

        let slots = Fan::calculate_slots(&config, 5);

        rest_is_none(slots, 2);
        assert_eq!(slots[0].unwrap(), (1, 40.0));
        assert_eq!(slots[1].unwrap(), (2, 60.0));
    }

    #[test]
    fn test_calculate_slots_with_min_state_non_zero() {
        let config = Config {
            threshold: Threshold {
                min: 50.0,
                max: 70.0,
            },
            state: State {
                max: Some(DEFAULT_MAX_STATE),
                min: 2,
            },
            sleep_time: DEFAULT_SLEEP_TIME,
        };

        let slots = Fan::calculate_slots(&config, 5);

        rest_is_none(slots, 3);
        assert_eq!(slots[0].unwrap().0, 3);
        assert_eq!(slots[1].unwrap().0, 4);
        assert_eq!(slots[2].unwrap().0, 5);
    }

    #[test]
    fn test_choose_speed_no_max_state_config() {
        let config = Config {
            threshold: Threshold {
                min: 45.0,
                max: 70.0,
            },
            state: State { min: 0, max: None },
            sleep_time: DEFAULT_SLEEP_TIME,
        };

        let fan = Fan {
            temp_slots: [
                Some((1, 50.0)),
                Some((2, 55.0)),
                Some((3, 60.0)),
                Some((4, 65.0)),
                Some((5, 70.0)),
                None,
            ],
            max_state: DEFAULT_MAX_STATE,
            path: "cooling_device".into(),
            state: "cooling_device/cur_state".into(),
            last_state: None,
        };

        let result = fan.choose_speed(80.0, &config);
        assert_eq!(result, DEFAULT_MAX_STATE);
    }

    #[test]
    fn test_choose_speed_with_very_low_temp() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(0.0, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_choose_speed_with_negative_temp() {
        let config = setup_test_config();
        let fan = setup_test_fan();

        let result = fan.choose_speed(-10.0, &config);
        assert_eq!(result, config.state.min);
    }

    #[test]
    fn test_choose_speed_boundary_between_slots() {
        let config = Config {
            threshold: Threshold {
                min: 40.0,
                max: 60.0,
            },
            state: State {
                min: 0,
                max: Some(3),
            },
            sleep_time: DEFAULT_SLEEP_TIME,
        };

        let fan = Fan {
            temp_slots: [
                Some((1, 40.0)),
                Some((2, 50.0)),
                Some((3, 60.0)),
                None,
                None,
                None,
            ],
            max_state: DEFAULT_MAX_STATE,
            path: "cooling_device".into(),
            state: "cooling_device/cur_state".into(),
            last_state: None,
        };

        let result = fan.choose_speed(49.0, &config);
        assert_eq!(result, 1);

        let result = fan.choose_speed(50.0, &config);
        assert_eq!(result, 2);

        let result = fan.choose_speed(51.0, &config);
        assert_eq!(result, 2);
    }
}
