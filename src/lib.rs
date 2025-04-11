pub mod config;

pub type FanDevice = Option<(String, u32, Vec<(u32, f64)>)>;
