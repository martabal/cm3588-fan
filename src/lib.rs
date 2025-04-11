pub mod config;
pub mod fan;

pub const DEVICE_NAME_COOLING: &str = "cooling_device";
pub const DEVICE_TYPE_PWM_FAN: &str = "pwm-fan";
pub const THERMAL_DIR: &str = "/sys/class/thermal";
pub const THERMAL_ZONE_NAME: &str = "thermal_zone";
pub const FILE_NAME_CUR_STATE: &str = "cur_state";
