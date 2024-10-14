pub mod bluetooth;
pub mod colors;
pub mod constants;
pub mod logger;
pub mod storage;

mod daemon;

#[cfg(test)]
mod tests;

pub use btleplug::api::{BDAddr as BluetoothAddr, Peripheral as BluetoothPeripheral};
pub use color_space;

pub mod utils {
    pub use super::daemon::*;
}

#[cfg(feature = "ffi")]
mod ffi;
