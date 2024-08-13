use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, color_picker, CentralPanel, TopBottomPanel};
use eframe::{CreationContext, NativeOptions};
use futures::StreamExt as _;
use tokio::runtime::{self, Runtime};
use tokio::sync::{
    watch::{channel, Receiver},
    RwLock,
};
use tokio::time::{self, Instant};

use rustbee_common::bluetooth::{get_devices, Client, FoundDevice, HueDevice};
use rustbee_common::color_space::Rgb;
use rustbee_common::colors::Xy;
use rustbee_common::constants::{
    flags::COLOR_RGB, OutputCode, ADDR_LEN, DATA_LEN, HUE_BAR_1_ADDR, HUE_BAR_2_ADDR,
};
use rustbee_common::BluetoothAddr;

const SEARCH_MAX_CHARS: usize = DATA_LEN;

#[derive(Clone)]
struct HueDeviceWrapper {
    // Since most of the time, fields are already initiated, using Option<T> would just make everything
    // more verbose
    is_initiated: bool,
    is_paired: bool,
    is_found: bool,
    last_update: Instant,
    is_connected: bool,
    power_state: bool,
    brightness: u8,
    current_color: [u8; 3],
    name: String,
    inner: HueDevice<Client>,
}

impl Default for HueDeviceWrapper {
    /// Do not use default when there's no inner HueDevice defined
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            power_state: Default::default(),
            brightness: Default::default(),
            current_color: Default::default(),
            name: Default::default(),
            is_found: false,
            is_connected: false,
            is_paired: false,
            is_initiated: false,
            inner: Default::default(),
        }
    }
}

impl HueDeviceWrapper {
    fn from_address(addr: BluetoothAddr) -> Self {
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

type AppDevices = HashMap<[u8; ADDR_LEN], HueDeviceWrapper>;

struct App {
    devices: Arc<RwLock<AppDevices>>,
    tokio_rt: Runtime,
    device_error: Option<String>,
    device_name_search: String,
    devices_found: Arc<RwLock<Vec<FoundDevice>>>,
    new_device_addr: String,
    is_new_device_addr_error: bool,
    channel: Option<Receiver<bool>>,
}

impl App {
    fn new(
        _cc: &CreationContext<'_>,
        devices: Arc<RwLock<AppDevices>>,
        tokio_rt: Runtime,
    ) -> Box<Self> {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Box::new(Self {
            devices,
            tokio_rt,
            device_error: None,
            device_name_search: String::new(),
            devices_found: Arc::new(RwLock::new(Vec::new())),
            new_device_addr: String::new(),
            is_new_device_addr_error: false,
            channel: None,
        })
    }
}

/// Keep in mind that this overwrites the current receiver channel,
/// making the previous future unable to be read (but not cancelled)
macro_rules! run_async {
    ($self:expr, $f:expr) => {{
        let (tx, rx) = channel(false);

        $self.tokio_rt.spawn(async move {
            // Intentionally not handling the error since the receiver channel can be overwritten
            // so the previous one is dropped
            let _ = tx.send($f.await);
        });

        $self.channel = Some(rx);
    }};
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let devices = Arc::clone(&self.devices);

        TopBottomPanel::top("banner")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    if !self.new_device_addr.is_empty() && self.is_new_device_addr_error {
                        ui.horizontal(|ui| {
                                    ui.label("Error on parsing Address, please respect the following format: ff:aa:55:ff:aa:55");
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("Add a Philips Hue device by uuid");
                        let input = ui.text_edit_singleline(&mut self.new_device_addr);
                        if input.has_focus() {
                            input.show_tooltip_ui(|ui| {
                                ui.label(
                                    "Enter the Hexadecimal UUID and press enter or click elsewhere",
                                );
                            });
                        }
                        if input.lost_focus() {
                            match parse_address(&self.new_device_addr) {
                                Ok(addr) => {
                                    let devices = Arc::clone(&devices);

                                    run_async!(self, async move {
                                        let devices_read = devices.read().await;
                                        if devices_read.get(&*addr).is_some() {
                                            return false;
                                        }
                                        drop(devices_read);

                                        let mut devices = devices.write().await;
                                        let mut device = HueDeviceWrapper::from_address(addr);

                                        match device.pair().await {
                                            OutputCode::Success => {
                                                device.is_paired = true;
                                                device.is_found = true;
                                            },
                                            _ => {
                                                device.is_paired = false;
                                                device.is_found = false;
                                            },
                                        }

                                        devices.insert(*addr, device);
                                        true
                                    });

                                    self.is_new_device_addr_error = false;
                                },
                                Err(_) => {
                                    self.is_new_device_addr_error = true;
                                },
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Add a Philips Hue device by (partial) name");
                        ui.text_edit_singleline(&mut self.device_name_search);
                        // TODO: Make it less ugly if possible
                        while self.device_name_search.as_bytes().len() > SEARCH_MAX_CHARS {
                            self.device_name_search.pop();
                        }
                        if ui.button("Search").clicked() {
                            let name = self.device_name_search.clone();
                            let devices_found_ref = Arc::clone(&self.devices_found);

                            run_async!(self, async move {
                                let name = name;
                                let mut stream = HueDevice::search_by_name(&name).await;

                                while let Some(device) = stream.next().await {
                                    let mut devices_found = devices_found_ref.write().await;
                                    devices_found.push(device);
                                }

                                true
                            });
                        }
                    });

                    let devices_found = self.tokio_rt.block_on(self.devices_found.read());

                    if !devices_found.is_empty() {
                        ui.horizontal(|ui| {
                            if self.channel.is_none() && ui.button("close").clicked() {
                                let devices_found_ref = Arc::clone(&self.devices_found);

                                run_async!(self, async move {
                                    // This causes a race on the read block_on above but it's fast
                                    // enought not to cause any issues
                                    let mut devices_found = devices_found_ref.write().await;
                                    devices_found.clear();

                                    true
                                });
                                return;
                            }

                            ui.vertical_centered(|ui| {
                                ui.label("Devices found:");

                                for device in devices_found.iter() {
                                    let devices = Arc::clone(&devices);
                                    let addr = device.address;
                                    let btn = ui.button(format!("{} - {}", device.name, BluetoothAddr::new(addr)));

                                    if btn.hovered() {
                                        btn.show_tooltip_text("Pair to this device");
                                    }

                                    if btn.clicked() {
                                        run_async!(self, async move {
                                            let devices_read = devices.read().await;
                                            if devices_read.get(&addr).is_some() {
                                                return false;
                                            }
                                            drop(devices_read);

                                            let mut devices = devices.write().await;
                                            let mut device = HueDeviceWrapper::from_address(BluetoothAddr::new(addr));

                                            match device.pair().await {
                                                OutputCode::Success => {
                                                    device.is_paired = true;
                                                    device.is_found = true;
                                                },
                                                _ => {
                                                    device.is_paired = false;
                                                    device.is_found = false;
                                                },
                                            }

                                            devices.insert(addr, device);
                                            true
                                        });
                                    }
                                }
                            });
                        });
                    }
                });
            });

        CentralPanel::default().show(ctx, |ui| {
            if let Some(ref error) = self.device_error {
                if !error.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Error {error}"));
                    });
                }
            }

            if let Some(ref mut rx) = self.channel {
                match rx.has_changed() {
                    Ok(changed) => {
                        if changed {
                            if !*rx.borrow_and_update() {
                                ui.colored_label(ui.visuals().error_fg_color, "Error");
                            }
                            self.channel = None;
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.spinner();
                            });
                            return;
                        }
                    }
                    Err(_) => {
                        self.channel = None;
                    }
                }
            }

            let mut devices_mut = self.tokio_rt.block_on(self.devices.write());

            if devices_mut.is_empty() {
                return;
            }

            if ui.button("Power OFF all devices").clicked() {
                run_async!(self, async {
                    let devices_read = devices.read().await;
                    let futures = devices_read
                        .iter()
                        .map(|(_, device)| device.set_power(false))
                        .collect::<Vec<_>>();
                    let res = futures::future::join_all(futures).await;
                    drop(devices_read);

                    let mut lock = devices.write().await;
                    lock.iter_mut().for_each(|(_, device)| {
                        device.power_state = false;
                    });

                    !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                });
                return;
            }

            if ui.button("Power ON all devices").clicked() {
                run_async!(self, async {
                    let devices_read = devices.read().await;
                    let futures = devices_read
                        .iter()
                        .map(|(_, device)| device.set_power(true))
                        .collect::<Vec<_>>();
                    let res = futures::future::join_all(futures).await;
                    drop(devices_read);

                    let mut lock = devices.write().await;
                    lock.iter_mut().for_each(|(_, device)| {
                        device.power_state = true;
                    });

                    !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                });
                return;
            }

            if ui.button("Disconnect from all devices").clicked() {
                run_async!(self, async {
                    let devices_read = devices.read().await;
                    let futures = devices_read
                        .iter()
                        .map(|(_, device)| device.disconnect_device())
                        .collect::<Vec<_>>();
                    let res = futures::future::join_all(futures).await;
                    drop(devices_read);

                    let mut lock = devices.write().await;
                    for (_, device) in lock.iter_mut() {
                        update_device_state(device).await;
                    }

                    !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                });
                return;
            }

            if ui.button("Connect to all devices").clicked() {
                run_async!(self, async {
                    let devices_read = devices.read().await;
                    let futures = devices_read
                        .iter()
                        .map(|(_, device)| device.connect_device())
                        .collect::<Vec<_>>();
                    let res = futures::future::join_all(futures).await;
                    drop(devices_read);

                    let mut lock = devices.write().await;
                    for (_, device) in lock.iter_mut() {
                        update_device_state(device).await;
                    }

                    !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                });
                return;
            }

            for (addr, device) in devices_mut.iter_mut() {
                let addr = *addr;
                if device.name.is_empty() {
                    ui.label("Device:");
                } else {
                    ui.label(format!("Device {}:", device.name));
                }
                ui.label(format!("Hex UUID: {:?}", addr));

                if ui.button("Remove device").clicked() {
                    let devices = Arc::clone(&devices);

                    self.tokio_rt.spawn(async move {
                        devices.write().await.remove(&addr);
                    });
                    return;
                }

                if !device.is_found {
                    ui.label("Device not found");

                    if ui.button("Pair device").clicked() {
                        run_async!(self, async move {
                            let mut lock = devices.write().await;
                            let device = lock.get_mut(&addr).unwrap();
                            match device.pair().await {
                                OutputCode::Success => {
                                    device.is_paired = true;
                                    device.is_found = true;
                                    true
                                }
                                _ => {
                                    device.is_paired = false;
                                    device.is_found = false;
                                    false
                                }
                            }
                        });
                        return;
                    }
                    continue;
                }

                ui.label(format!("Is paired: {}", device.is_paired));
                ui.label(format!("Is connected: {}", device.is_connected));
                if device.is_connected {
                    ui.label(format!("Brightness: {}%", device.brightness));
                    if self.channel.is_none()
                        && color_picker::color_edit_button_srgb(ui, &mut device.current_color)
                            .changed()
                    {
                        let (r, g, b) = (
                            device.current_color[0],
                            device.current_color[1],
                            device.current_color[2],
                        );
                        let Xy {
                            x,
                            y,
                            brightness: _,
                        } = Xy::from(Rgb::new(r as _, g as _, b as _));
                        let device = device.clone();
                        run_async!(self, async move {
                            device
                                .set_colors(x as _, y as _, COLOR_RGB)
                                .await
                                .is_success()
                        });
                    }
                    ui.label(format!("Current color is {:?}", device.current_color));

                    if device.power_state {
                        if ui.button("Power OFF").clicked() {
                            let device = device.clone();

                            run_async!(self, async move {
                                let res = device.set_power(false).await.is_success();
                                if res {
                                    let mut lock = devices.write().await;
                                    let device = lock.get_mut(&addr).unwrap();
                                    device.power_state = false;
                                }
                                res
                            });
                            return;
                        }
                    } else if ui.button("Power ON").clicked() {
                        let device = device.clone();

                        run_async!(self, async move {
                            let res = device.set_power(true).await.is_success();
                            if res {
                                let mut lock = devices.write().await;
                                let device = lock.get_mut(&addr).unwrap();
                                device.power_state = true;
                            }
                            res
                        });
                        return;
                    }

                    if ui.button("Disconnect from device").clicked() {
                        let device = device.clone();

                        run_async!(self, async move {
                            let res = device.disconnect_device().await.is_success();
                            if res {
                                let mut lock = devices.write().await;
                                let device = lock.get_mut(&addr).unwrap();
                                update_device_state(device).await;
                            }
                            res
                        });
                        return;
                    }
                } else if ui.button("Connect to device").clicked() {
                    let device = device.clone();

                    run_async!(self, async move {
                        let res = device.connect_device().await.is_success();
                        if res {
                            let mut lock = devices.write().await;
                            let device = lock.get_mut(&addr).unwrap();
                            update_device_state(device).await;
                        }
                        res
                    });
                    return;
                }
            }
        });
    }
}

fn main() -> eframe::Result {
    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let state = Box::leak(Box::new(Arc::new(RwLock::new(HashMap::new()))));
    let state_async = Arc::clone(state);
    let app_options = NativeOptions {
        persistence_path: Some("./data".into()),
        ..Default::default()
    };

    // Thread used to init devices state and sync devices state on a timout
    rt.spawn(async move {
        // TODO: Use the save method and load devices for users to use
        let mut hue_devices = get_devices(&[HUE_BAR_1_ADDR, HUE_BAR_2_ADDR])
            .await
            .unwrap()
            .into_iter();
        let default_devices = HashMap::from([
            (HUE_BAR_1_ADDR, hue_devices.next().unwrap()),
            (HUE_BAR_2_ADDR, hue_devices.next().unwrap()),
        ]);

        // Implicit drop by using the lock right away
        *state_async.write().await = default_devices
            .into_iter()
            .map(|(addr, device)| (addr, device.into()))
            .collect::<AppDevices>();

        // There must be a loop to update state in case devices state gets updated by a thrird party app
        loop {
            for (_, device) in state_async.write().await.iter_mut() {
                if device.is_initiated || device.last_update.elapsed() < Duration::from_secs(60 * 2)
                {
                    continue;
                }

                update_device_state(device).await;
            }

            time::sleep(Duration::from_millis(500)).await;
        }
    });

    eframe::run_native(
        "Rustbee",
        app_options,
        Box::new(|cc| Ok(App::new(cc, Arc::clone(state), rt))),
    )?;

    Ok(())
}

fn parse_address(str: &str) -> Result<BluetoothAddr, String> {
    BluetoothAddr::from_str(str).map_err(|e| e.0)
}

async fn update_device_state(device: &mut HueDeviceWrapper) {
    let (res_conn, buf_conn) = device.is_connected().await;
    if res_conn.is_success() {
        device.is_connected = buf_conn[0] == 1;
    }

    if device.is_connected {
        let (
            (res_color, buf_color),
            (res_bright, buf_bright),
            (res_power, buf_power),
            (res_name, buf_name),
        ) = tokio::join!(
            device.get_colors(COLOR_RGB),
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

            device.current_color = [rgb.r as _, rgb.g as _, rgb.b as _];
            device.brightness = ((buf_bright[0] as f64 / 255.) * 100.) as _;
            device.power_state = *buf_power.first().unwrap() == 1;
            device.name = (*String::from_utf8_lossy(&buf_name)).to_owned();
            device.is_paired = true;
            device.is_found = true;
        }
    }
    device.is_initiated = true;
    device.last_update = Instant::now();
}
