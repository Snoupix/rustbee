pub mod colors;
pub mod constants;
pub mod device;
pub mod logger;
pub mod storage;
pub mod utils;

#[cfg(not(target_os = "windows"))]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(test)]
mod tests;

#[cfg(feature = "ffi")]
mod ffi;

#[cfg(not(target_os = "windows"))]
pub use linux::*;

#[cfg(target_os = "windows")]
pub use windows::*;

// Re-exports
pub use color_space;
