use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use serde::ser::SerializeStruct as _;
use tokio::time::Instant;

use rustbee_common::constants::ADDR_LEN;
use rustbee_common::device::{Client, FoundDevice, HueDevice};
use rustbee_common::storage::{SavedDevice, Storage};

use crate::DEBOUNCE_SECS;

pub type AppDevices = HashMap<[u8; ADDR_LEN], HueDeviceWrapper>;
pub type ParsedAppDevices = HashMap<String, HueDeviceWrapper>;

#[derive(Clone)]
pub struct GlobalState {
    pub storage: Storage,
    // TODO: Async functions have their own runtime on Tauri; this may be useless
    // pub tokio_rt: tokio::runtime::Runtime,
    pub devices_color: Debounce<[u8; 3]>,
    pub devices_brightness: Debounce<u8>,
    pub device_error: Option<String>,
    pub device_name_search: String,
    pub devices_found: Vec<FoundDevice>,
    pub new_device_addr: String,
    pub is_new_device_addr_error: bool,
}

impl GlobalState {
    pub fn new(
        /* tokio_rt: tokio::runtime::Runtime, */ lowest_brightness: u8,
        storage: Storage,
    ) -> Self {
        Self {
            storage,
            // tokio_rt,
            devices_color: Debounce::new([0; 3], Duration::from_secs(DEBOUNCE_SECS)),
            devices_brightness: Debounce::new(lowest_brightness, Duration::from_secs(1)),
            device_error: None,
            device_name_search: String::new(),
            devices_found: Vec::new(),
            new_device_addr: String::new(),
            is_new_device_addr_error: false,
        }
    }
}

impl serde::Serialize for GlobalState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("GlobalState", 3)?;
        state.serialize_field("color", self.devices_color.deref())?;
        state.serialize_field("brightness", self.devices_brightness.deref())?;
        state.serialize_field("devices_found", &self.devices_found)?;
        state.end()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HueDeviceWrapper {
    // Since most of the time, fields are already initiated, using Option<T> would just make everything
    // more verbose
    #[serde(skip)]
    pub is_initiated: bool,
    pub is_found: bool,
    #[serde(skip)]
    pub last_update: Instant,
    pub is_connected: bool, // TODO: Watch for Windows, maybe get name to check if connected or erase the field
    pub power_state: bool,
    pub brightness: u8,
    /// Don't forget to call .update() after updating the inner value
    pub current_color: Debounce<[u8; 3]>,
    pub name: String,
    #[serde(skip)]
    inner: HueDevice<Client>,
}

impl Default for HueDeviceWrapper {
    /// Do not use default when there's no inner HueDevice defined
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            power_state: Default::default(),
            brightness: Default::default(),
            name: Default::default(),
            current_color: Debounce::new([0; 3], Duration::from_secs(DEBOUNCE_SECS)),
            is_found: false,
            is_connected: false,
            is_initiated: false,
            inner: Default::default(),
        }
    }
}

impl HueDeviceWrapper {
    pub fn from_address(addr: [u8; ADDR_LEN]) -> Self {
        Self {
            inner: HueDevice::new(addr),
            ..Default::default()
        }
    }
}

impl Deref for HueDeviceWrapper {
    type Target = HueDevice<Client>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<HueDevice<Client>> for HueDeviceWrapper {
    fn from(inner: HueDevice<Client>) -> Self {
        Self {
            inner,
            ..Default::default()
        }
    }
}

impl From<&HueDeviceWrapper> for SavedDevice {
    fn from(device: &HueDeviceWrapper) -> Self {
        Self {
            name: device.name.clone(),
            current_color: *device.current_color,
            brightness: device.brightness,
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Debounce<T> {
    #[serde(skip)]
    instant: Instant,
    #[serde(skip)]
    duration: Duration,
    #[serde(skip)]
    value: T,
    actual_value: T,
}

impl<T: Copy> Debounce<T> {
    pub fn new(value: T, duration: Duration) -> Self {
        Self {
            instant: Instant::now(),
            duration,
            value,
            actual_value: value,
        }
    }

    /// Method to call when mutating the debounce value and returns wheter or not the value has
    /// changed
    pub fn update(&mut self) -> bool {
        if self.instant.elapsed() > self.duration {
            self.instant = Instant::now();
            self.actual_value = self.value;

            return true;
        }

        false
    }
}

impl<T> Deref for Debounce<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.actual_value
    }
}

impl<T> DerefMut for Debounce<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: fmt::Display> fmt::Display for Debounce<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.actual_value)
    }
}
