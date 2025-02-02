use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt as _;
use serde_json::json;
use tauri::Emitter as _;
use tauri::{AppHandle, State};
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::time;

use rustbee_common::color_space::Rgb;
use rustbee_common::colors::Xy;
use rustbee_common::constants::{masks, ADDR_LEN};
use rustbee_common::device::HueDevice;
use rustbee_common::logger::*;

use crate::{
    update_all_devices_state, update_device_state, AppDevices, GlobalState as Global,
    ParsedAppDevices, DEVICE_STATE_UPDATE_SECS, HAS_SYNC_LOOP_STARTED, NAME_THREAD_ID,
};

type GlobalState<'a> = State<'a, Arc<RwLock<Global>>>;
type DevicesState<'a> = State<'a, Arc<RwLock<AppDevices>>>;
type RuntimeState<'a> = State<'a, Runtime>;

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
pub async fn init(
    handle: AppHandle,
    runtime: RuntimeState<'_>,
    devices_state: DevicesState<'_>,
    global_state: GlobalState<'_>,
) -> Result<Global, Error> {
    if HAS_SYNC_LOOP_STARTED.load(Ordering::Relaxed) {
        return Ok(global_state.read().await.clone());
    }

    HAS_SYNC_LOOP_STARTED.store(true, Ordering::SeqCst);

    let _devices_state = Arc::clone(&devices_state);

    // Thread used to sync devices state on a loop every x ms
    // There must be a loop to update state in case devices' state gets updated by a thrird party app
    runtime.spawn(async move {
        loop {
            for (addr, device) in _devices_state.write().await.iter_mut() {
                if device.last_update.elapsed() < Duration::from_secs(DEVICE_STATE_UPDATE_SECS) {
                    continue;
                }

                update_device_state(device).await;

                // TODO: Maybe figure out a way to get active clients and turn
                // HAS_SYNC_LOOP_STARTED to false when it goes to 0 + break
                if let Err(err) = handle.emit(
                    "device_sync",
                    json!({
                        format!("{addr:?}"): device.clone()
                    }),
                ) {
                    error!("Failed to send \"device_sync\" event to all targets: {err}");
                }
            }

            time::sleep(Duration::from_millis(1000)).await;
        }
    });

    Ok(global_state.read().await.clone())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_bt_devices(
    handle: AppHandle,
    runtime: RuntimeState<'_>,
    global_state: GlobalState<'_>,
    name: String,
) -> Result<u8, Error> {
    let id = NAME_THREAD_ID.load(Ordering::Relaxed);
    NAME_THREAD_ID.store(id + 1, Ordering::SeqCst);
    let state = Arc::clone(&global_state);

    runtime.spawn(async move {
        let mut stream = HueDevice::search_by_name(&name).await;

        while let Some(device) = stream.next().await {
            if let Err(err) = handle.emit(&format!("bt_stream_{id}_data"), json!(device)) {
                error!("Failed to send \"bt_stream_{id}_data\" event to all targets: {err}");
            }

            state.write().await.devices_found.push(device);
        }

        NAME_THREAD_ID.store(NAME_THREAD_ID.load(Ordering::Relaxed) - 1, Ordering::SeqCst);

        if let Err(err) = handle.emit(&format!("bt_stream_{id}_end"), ()) {
            error!("Failed to send \"bt_stream_{id}_end\" event to all targets: {err}");
        }
    });

    Ok(id)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn clear_devices_found(global_state: GlobalState<'_>) -> Result<(), ()> {
    global_state.write().await.devices_found.clear();

    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_device_colors(
    devices_state: DevicesState<'_>,
    address: [u8; ADDR_LEN],
    r: u8,
    g: u8,
    b: u8,
) -> Result<bool, Error> {
    let rgb = Rgb::new(r as _, g as _, b as _);
    let xy = Xy::from(rgb);
    let mut devices = devices_state.write().await;

    if let Some(device) = devices.get_mut(&address) {
        return Ok(device
            .set_colors(xy.x, xy.y, masks::COLOR_XY)
            .await
            .is_success());
    }

    Err(Error::NotFound(address))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_devices_colors(
    devices_state: DevicesState<'_>,
    r: u8,
    g: u8,
    b: u8,
) -> Result<bool, Error> {
    let rgb = Rgb::new(r as _, g as _, b as _);
    let xy = Xy::from(rgb);
    let mut devices = devices_state.write().await;

    for (_, device) in devices.iter_mut() {
        if !device
            .set_colors(xy.x, xy.y, masks::COLOR_XY)
            .await
            .is_success()
        {
            return Ok(false);
        }
    }

    drop(devices);

    update_all_devices_state(Arc::clone(devices_state.inner())).await;

    Ok(true)
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
pub async fn update_devices(devices_state: DevicesState<'_>) -> Result<ParsedAppDevices, Error> {
    update_all_devices_state(Arc::clone(devices_state.inner())).await;

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
