use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, color_picker, CentralPanel};
use eframe::{CreationContext, NativeOptions};
use poll_promise::Promise;
use tokio::runtime::{self, Runtime};
use tokio::sync::RwLock;
use tokio::time;

use rustbee_common::bluetooth::{get_devices, Client, HueDevice};
use rustbee_common::color_space::Rgb;
use rustbee_common::colors::Xy;
use rustbee_common::constants::{flags::COLOR_RGB, HUE_BAR_1_ADDR, HUE_BAR_2_ADDR};

struct HueDeviceWrapper {
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
            last_update: Instant::now(),
        }
    }
}

type AppDevices = HashMap<[u8; 6], HueDeviceWrapper>;

struct App {
    tokio_rt: Runtime,
    devices: Arc<RwLock<AppDevices>>,
    promise: Option<Promise<bool>>,
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
            promise: None,
        })
    }
}

macro_rules! run_promise {
    ($rt:expr, $f:expr) => {{
        let (sender, promise) = Promise::new();

        $rt.spawn(async move {
            sender.send($f.await);
        });

        promise
    }};
}

/* fn run_promise<F, R>(rt: &Runtime, f: F) -> Promise<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Copy + Send,
{
    let (sender, promise) = Promise::new();

    rt.spawn(async move {
        sender.send(f.await);
    });

    promise
} */

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            if let Some(promise) = &self.promise {
                if let Some(result) = promise.ready() {
                    match *result {
                        true => {
                            ui.label("success");
                        }
                        false => {
                            ui.colored_label(ui.visuals().error_fg_color, "Error");
                        }
                    }
                    self.promise = None;
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                    return;
                }
            }

            let mut devices = self.tokio_rt.block_on(self.devices.write());

            for (addr, device) in devices.iter_mut() {
                ui.label("Device:");
                ui.label(format!("Hex UUID: {:?}", addr));
                ui.label(format!("Is connected: {}", device.is_connected));
                if device.is_connected {
                    ui.label(format!("Brightness: {}%", device.brightness));
                    if color_picker::color_edit_button_srgb(ui, &mut device.current_color).changed()
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
                        self.promise = Some(run_promise!(
                            &self.tokio_rt,
                            device.set_colors(x as _, y as _, COLOR_RGB)
                        ));
                    }
                    ui.label(format!("Current color is {:?}", device.current_color));

                    if device.power_state {
                        if ui.button("Power OFF").clicked() {
                            let device = device.clone();
                            self.promise =
                                Some(run_promise!(&self.tokio_rt, device.set_power(false)));
                        }
                    } else if ui.button("Power ON").clicked() {
                        let device = device.clone();
                        self.promise = Some(run_promise!(&self.tokio_rt, device.set_power(true)));
                    }

                    if ui.button("Disconnect from device").clicked() {
                        let device = device.clone();
                        self.promise =
                            Some(run_promise!(&self.tokio_rt, device.disconnect_device()));
                    }
                } else if ui.button("Connect to device").clicked() {
                    let device = device.clone();
                    self.promise = Some(run_promise!(&self.tokio_rt, device.connect_device()));
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

    // Async runtime to sync current device data on state
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

        loop {
            for (_, device) in state_async.write().await.iter_mut() {
                if device.last_update.elapsed() < Duration::from_secs(5) {
                    continue;
                }

                if let Ok(v) = device.is_connected().await {
                    device.is_connected = v;
                }

                if device.is_connected {
                    let ((success, buf), (success_b, buf_b)) =
                        tokio::join!(device.get_colors(COLOR_RGB), device.get_brightness());
                    if success && success_b {
                        let x = u16::from_le_bytes([buf[0], buf[1]]) as f64 / 0xFFFF as f64;
                        let y = u16::from_le_bytes([buf[2], buf[3]]) as f64 / 0xFFFF as f64;
                        let xy = Xy::new(x, y);
                        let rgb = xy.to_rgb(buf_b[0] as f64 / 255.);

                        device.current_color = [rgb.r as _, rgb.g as _, rgb.b as _];
                    }
                }
                device.last_update = Instant::now();
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
