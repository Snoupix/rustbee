pub mod bluetooth;
pub mod device;

pub(crate) mod daemon;
pub(crate) use btleplug::platform::Peripheral as InnerDevice;

// Re-exports
pub use btleplug::api::Peripheral as BluetoothPeripheralImpl;
