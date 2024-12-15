use std::sync::Arc;

use rustbee_common::constants::ADDR_LEN;
use tauri::State;
use tokio::sync::RwLock;

use rustbee_common::logger::*;

use crate::{update_all_devices_state, AppDevices, GlobalState as Global, ParsedAppDevices};

type GlobalState<'a> = State<'a, Arc<RwLock<Global>>>;
type DevicesState<'a> = State<'a, Arc<RwLock<AppDevices>>>;

#[derive(Debug, serde::Serialize)]
pub enum Error {
    NotFound([u8; ADDR_LEN]),
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warn,
    Debug,
    Error,
    Trace,
}

// Put rename_all = "snake_case" on every command so client side keeps snake case for parameters

#[tauri::command(rename_all = "snake_case")]
pub fn log(data: String, log_level: LogLevel) {
    match log_level {
        LogLevel::Info => info!("{data}"),
        LogLevel::Warn => warn!("{data}"),
        LogLevel::Debug => debug!("{data}"),
        LogLevel::Error => error!("{data}"),
        LogLevel::Trace => trace!("{data}"),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_global_state(global_state: GlobalState<'_>) -> Result<Global, Error> {
    Ok(global_state.read().await.clone())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_devices(devices_state: DevicesState<'_>) -> Result<ParsedAppDevices, Error> {
    Ok(devices_state
        .read()
        .await
        .clone()
        .into_iter()
        .map(|(k, v)| (format!("{k:?}"), v))
        .collect())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_power_all(devices_state: DevicesState<'_>, power_state: bool) -> Result<bool, ()> {
    let mut devices = devices_state.write().await;

    for (_, device) in devices.iter_mut() {
        if !device.set_power(power_state).await.is_success() {
            return Ok(false);
        }
    }

    drop(devices);

    update_all_devices_state(Arc::clone(devices_state.inner())).await;

    Ok(true)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_brightness_all(
    global_state: GlobalState<'_>,
    devices_state: DevicesState<'_>,
    brightness: u8,
) -> Result<bool, ()> {
    let mut devices = devices_state.write().await;

    for (_, device) in devices.iter_mut() {
        if !device.set_brightness(brightness).await.is_success() {
            return Ok(false);
        }
    }

    drop(devices);

    update_all_devices_state(Arc::clone(devices_state.inner())).await;

    Ok(true)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_power(
    devices_state: DevicesState<'_>,
    addr: [u8; ADDR_LEN],
    power_state: bool,
) -> Result<bool, Error> {
    let mut devices = devices_state.write().await;
    let Some(device) = devices.get_mut(&addr) else {
        return Err(Error::NotFound(addr));
    };

    if !device.set_power(power_state).await.is_success() {
        return Ok(false);
    }

    Ok(true)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_brightness(
    devices_state: DevicesState<'_>,
    addr: [u8; ADDR_LEN],
    brightness: u8,
) -> Result<bool, Error> {
    let mut devices = devices_state.write().await;
    let Some(device) = devices.get_mut(&addr) else {
        return Err(Error::NotFound(addr));
    };

    if !device.set_brightness(brightness).await.is_success() {
        return Ok(false);
    }

    Ok(true)
}
