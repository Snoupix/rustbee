// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::{FutureExt, StreamExt as _};
use tauri::{Manager, Theme};
use tokio::runtime::{self, Runtime};
use tokio::sync::{
    watch::{channel, Receiver},
    RwLock,
};
use tokio::time::{self, Instant};

use rustbee_common::color_space::Rgb;
use rustbee_common::colors::Xy;
use rustbee_common::constants::{
    masks, OutputCode, ADDR_LEN, APP_ID, DATA_LEN, GUI_SAVE_INTERVAL_SECS,
};
use rustbee_common::logger::Logger;
use rustbee_common::storage::Storage;
use rustbee_common::utils::launch_daemon;

use state::*;

const SEARCH_MAX_CHARS: usize = DATA_LEN;
const DEVICE_STATE_UPDATE_SECS: u64 = 60;
const DEBOUNCE_SECS: u64 = 5;

static LOGGER: Logger = Logger::new("Rustbee-GUI", false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    LOGGER.init();

    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    // TODO: Handle a fallback path
    let mut storage = Storage::try_default().unwrap();
    let devices_state: Arc<RwLock<AppDevices>> = Arc::new(RwLock::new(HashMap::new()));
    let mut lowest_brightness = 100u8;

    rt.block_on(launch_daemon()).unwrap();

    // Init devices state from storage
    let mut devices_guard = rt.block_on(devices_state.write());

    for (addr, device) in storage.get_devices() {
        let mut hue_device = HueDeviceWrapper::from_address(*addr);
        hue_device.name = device.name.clone();
        hue_device.current_color =
            Debounce::new(device.current_color, Duration::from_secs(DEBOUNCE_SECS));

        lowest_brightness = u8::min(lowest_brightness, device.brightness);
        devices_guard.insert(*addr, hue_device);
    }

    drop(devices_guard);

    let _devices_state = Arc::clone(&devices_state);

    // Thread used to sync devices state on a loop every x ms
    // There must be a loop to update state in case devices' state gets updated by a thrird party app
    rt.spawn(async move {
        loop {
            for (_, device) in _devices_state.write().await.iter_mut() {
                if device.last_update.elapsed() < Duration::from_secs(DEVICE_STATE_UPDATE_SECS) {
                    continue;
                }

                update_device_state(device).await;
            }

            time::sleep(Duration::from_millis(1000)).await;
        }
    });

    let global_state = Arc::new(RwLock::new(GlobalState::new(
        // rt,
        lowest_brightness,
        storage,
    )));

    tauri::Builder::default()
        .setup(|app| {
            app.manage(global_state);
            app.manage(devices_state);
            app.set_theme(Some(Theme::Dark));

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::set_brightness,
            commands::set_brightness_all,
            commands::set_power,
            commands::set_power_all,
            commands::get_devices,
            commands::log,
            commands::get_global_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn update_all_devices_state(devices: Arc<RwLock<AppDevices>>) {
    for (_, device) in devices.write().await.iter_mut() {
        update_device_state(device).await;
    }
}

async fn update_device_state(device: &mut HueDeviceWrapper) {
    if cfg!(target_os = "windows") {
        let (res_conn, buf_conn) = device.get_name().await;

        // TODO: Move this trickery server side, the client shouldn't manage this
        // On Windows, it's the OS that manages the connection so getting a non-empty name will do
        // the trick
        device.is_connected = res_conn.is_success() && buf_conn.iter().any(|byte| *byte != 0);
    } else {
        let (res_conn, buf_conn) = device.is_connected().await;

        device.is_connected = res_conn.is_success() && buf_conn[0] == true as u8;
    }

    if device.is_connected {
        let (
            (res_color, buf_color),
            (res_bright, buf_bright),
            (res_power, buf_power),
            (res_name, buf_name),
        ) = tokio::join!(
            device.get_colors(masks::COLOR_RGB),
            device.get_brightness(),
            device.get_power(),
            device.get_name()
        );

        if matches!(res_color, OutputCode::DeviceNotFound)
            || matches!(res_bright, OutputCode::DeviceNotFound)
            || matches!(res_power, OutputCode::DeviceNotFound)
            || matches!(res_name, OutputCode::DeviceNotFound)
        {
            device.is_found = false;
            return;
        }
        if res_color.is_success()
            && res_bright.is_success()
            && res_power.is_success()
            && res_name.is_success()
        {
            let x = u16::from_le_bytes([buf_color[0], buf_color[1]]) as f64 / 0xFFFF as f64;
            let y = u16::from_le_bytes([buf_color[2], buf_color[3]]) as f64 / 0xFFFF as f64;
            let xy = Xy::new(x, y);
            let rgb = xy.to_rgb(buf_bright[0] as f64 / 255.);

            *device.current_color = [rgb.r as _, rgb.g as _, rgb.b as _];
            device.current_color.update();
            device.brightness = ((buf_bright[0] as f64 / 255.) * 100.) as _;
            device.power_state = *buf_power.first().unwrap() == 1;
            device.name = (*String::from_utf8_lossy(&buf_name)).to_owned();
            device.is_found = true;
        }
    }

    device.is_initiated = true;
    device.last_update = Instant::now();
}
