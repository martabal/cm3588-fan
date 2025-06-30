pub mod cheker;
pub mod config;
pub mod fan;
pub mod temp;

pub const THERMAL_DIR: &str = "/sys/class/thermal";
pub const THERMAL_ZONE_NAME: &str = "thermal_zone";
