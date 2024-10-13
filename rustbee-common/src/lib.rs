pub mod bluetooth;
pub mod colors;
pub mod constants;
pub mod storage;

mod daemon;
mod logs;

#[cfg(test)]
mod tests;

pub use btleplug::api::{BDAddr as BluetoothAddr, Peripheral as BluetoothPeripheral};
pub use color_space;

pub mod utils {
    pub use super::daemon::*;
    pub use super::logs::*;
}

#[cfg(feature = "ffi")]
mod ffi;
