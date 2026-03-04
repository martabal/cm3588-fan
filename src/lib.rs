#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod checker;
pub mod config;
#[cfg(feature = "std")]
pub mod fan;
#[cfg(feature = "std")]
pub mod temp;

#[cfg(feature = "std")]
pub const THERMAL_DIR: &str = "/sys/class/thermal";
