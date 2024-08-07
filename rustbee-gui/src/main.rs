use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, color_picker, CentralPanel};
use eframe::{CreationContext, NativeOptions};
use tokio::runtime::{self, Runtime};
use tokio::sync::{
    watch::{channel, Receiver},
    RwLock,
};
use tokio::time::{self, Instant};

use rustbee_common::bluetooth::{get_devices, Client, HueDevice};
use rustbee_common::color_space::Rgb;
use rustbee_common::colors::Xy;
use rustbee_common::constants::{flags::COLOR_RGB, HUE_BAR_1_ADDR, HUE_BAR_2_ADDR};

struct HueDeviceWrapper {
    // Since most of the time, fields are already initiated, using Option<T> would just make everything
    // more verbose
    is_initiated: bool,
    last_update: Instant,
    is_connected: bool,
    power_state: bool,
    brightness: u8,
    current_color: [u8; 3],
    inner: HueDevice<Client>,
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
            is_connected: false,
            power_state: false,
            brightness: 0,
            current_color: [0; 3],
            is_initiated: false,
            last_update: Instant::now(),
        }
    }
}

type AppDevices = HashMap<[u8; 6], HueDeviceWrapper>;

struct App {
    tokio_rt: Runtime,
    devices: Arc<RwLock<AppDevices>>,
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
            channel: None,
        })
    }
}

macro_rules! run_promise {
    ($rt:expr, $f:expr, $bloc:expr) => {{
        let (tx, rx) = channel(false);

        $rt.spawn(async move {
            let res = $f.await;
            tx.send(res).unwrap();

            if res {
                $bloc
            }
        });

        rx
    }};
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
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

            let devices = Arc::clone(&self.devices);
            let mut devices_mut = self.tokio_rt.block_on(self.devices.write());

            if ui.button("new connect").clicked() {
                self.channel = Some(run_promise!(
                    &self.tokio_rt,
                    async {
                        let devices = devices.read().await;
                        let futures = devices
                            .iter()
                            .map(|(_, device)| device.connect_device())
                            .collect::<Vec<_>>();
                        futures::future::join_all(futures).await;
                        true
                    },
                    {
                        let mut lock = devices.write().await;
                        for (_, device) in lock.iter_mut() {
                            update_device_state(device).await;
                        }
                    }
                ));
                return;
            }

            if ui.button("Power OFF all devices").clicked() {
                self.channel = Some(run_promise!(
                    &self.tokio_rt,
                    async {
                        let devices = devices.read().await;
                        let futures = devices
                            .iter()
                            .map(|(_, device)| device.set_power(false))
                            .collect::<Vec<_>>();
                        futures::future::join_all(futures).await;
                        true
                    },
                    {
                        let mut lock = devices.write().await;
                        lock.iter_mut().for_each(|(_, device)| {
                            device.power_state = false;
                        });
                    }
                ));
                return;
            }

            if ui.button("Power ON all devices").clicked() {
                self.channel = Some(run_promise!(
                    &self.tokio_rt,
                    async {
                        let devices = devices.read().await;
                        let futures = devices
                            .iter()
                            .map(|(_, device)| device.set_power(true))
                            .collect::<Vec<_>>();
                        futures::future::join_all(futures).await;
                        true
                    },
                    {
                        let mut lock = devices.write().await;
                        lock.iter_mut().for_each(|(_, device)| {
                            device.power_state = true;
                        });
                    }
                ));
                return;
            }

            if ui.button("Disconnect from all devices").clicked() {
                self.channel = Some(run_promise!(
                    &self.tokio_rt,
                    async {
                        let devices = devices.read().await;
                        let futures = devices
                            .iter()
                            .map(|(_, device)| device.disconnect_device())
                            .collect::<Vec<_>>();
                        futures::future::join_all(futures).await;
                        true
                    },
                    {
                        let mut lock = devices.write().await;
                        for (_, device) in lock.iter_mut() {
                            update_device_state(device).await;
                        }
                    }
                ));
                return;
            }

            if ui.button("Connect to all devices").clicked() {
                self.channel = Some(run_promise!(
                    &self.tokio_rt,
                    async {
                        let devices = devices.read().await;
                        let futures = devices
                            .iter()
                            .map(|(_, device)| device.connect_device())
                            .collect::<Vec<_>>();
                        futures::future::join_all(futures).await;
                        true
                    },
                    {
                        let mut lock = devices.write().await;
                        for (_, device) in lock.iter_mut() {
                            update_device_state(device).await;
                        }
                    }
                ));
                return;
            }

            for (addr, device) in devices_mut.iter_mut() {
                let addr = *addr;
                ui.label("Device:");
                ui.label(format!("Hex UUID: {:?}", addr));
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
                        self.channel = Some(run_promise!(
                            &self.tokio_rt,
                            device.set_colors(x as _, y as _, COLOR_RGB),
                            {}
                        ));
                    }
                    ui.label(format!("Current color is {:?}", device.current_color));

                    if device.power_state {
                        if ui.button("Power OFF").clicked() {
                            let device = device.clone();

                            self.channel =
                                Some(run_promise!(&self.tokio_rt, device.set_power(false), {
                                    let mut lock = devices.write().await;
                                    let device = lock.get_mut(&addr).unwrap();
                                    device.power_state = false;
                                }));
                            return;
                        }
                    } else if ui.button("Power ON").clicked() {
                        let device = device.clone();

                        self.channel =
                            Some(run_promise!(&self.tokio_rt, device.set_power(true), {
                                let mut lock = devices.write().await;
                                let device = lock.get_mut(&addr).unwrap();
                                device.power_state = true;
                            }));
                        return;
                    }

                    if ui.button("Disconnect from device").clicked() {
                        let device = device.clone();

                        self.channel =
                            Some(run_promise!(&self.tokio_rt, device.disconnect_device(), {
                                let mut lock = devices.write().await;
                                let device = lock.get_mut(&addr).unwrap();
                                update_device_state(device).await;
                            }));
                        return;
                    }
                } else if ui.button("Connect to device").clicked() {
                    let device = device.clone();

                    self.channel = Some(run_promise!(&self.tokio_rt, device.connect_device(), {
                        let mut lock = devices.write().await;
                        let device = lock.get_mut(&addr).unwrap();
                        update_device_state(device).await;
                    }));
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

async fn update_device_state(device: &mut HueDeviceWrapper) {
    if let Ok(v) = device.is_connected().await {
        device.is_connected = v;
    }

    if device.is_connected {
        let ((succ_color, buf_color), (succ_bright, buf_bright), (succ_power, buf_power)) = tokio::join!(
            device.get_colors(COLOR_RGB),
            device.get_brightness(),
            device.get_power(),
        );
        if succ_color && succ_bright && succ_power {
            let x = u16::from_le_bytes([buf_color[0], buf_color[1]]) as f64 / 0xFFFF as f64;
            let y = u16::from_le_bytes([buf_color[2], buf_color[3]]) as f64 / 0xFFFF as f64;
            let xy = Xy::new(x, y);
            let rgb = xy.to_rgb(buf_bright[0] as f64 / 255.);

            device.current_color = [rgb.r as _, rgb.g as _, rgb.b as _];
            device.brightness = ((buf_bright[0] as f64 / 255.) * 100.) as _;
            device.power_state = *buf_power.first().unwrap() == 1;
        }
    }
    device.is_initiated = true;
    device.last_update = Instant::now();
}
