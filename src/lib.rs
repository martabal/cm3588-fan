pub mod config;
pub mod fan;
pub mod temp;

pub const FILE_NAME_CUR_STATE: &str = "cur_state";
pub const THERMAL_DIR: &str = "/sys/class/thermal";
pub const THERMAL_ZONE_NAME: &str = "thermal_zone";
