use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, color_picker, CentralPanel};
use eframe::{CreationContext, NativeOptions};
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

use rustbee_common::bluetooth::{get_devices, HueDevice};
use rustbee_common::constants::{HUE_BAR_1_ADDR, HUE_BAR_2_ADDR};
use tokio::time;

struct HueDeviceWrapper {
    last_update: Instant,
    is_connected: bool,
    brightness: u8,
    current_color: [u8; 3],
    inner: HueDevice,
}

impl Deref for HueDeviceWrapper {
    type Target = HueDevice;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<HueDevice> for HueDeviceWrapper {
    fn from(inner: HueDevice) -> Self {
        Self {
            inner,
            is_connected: false,
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
        Box::new(Self { devices, tokio_rt })
    }

    async fn update(&mut self) {
        for (_, device) in self.devices.write().await.iter_mut() {
            if device.last_update.elapsed() < Duration::from_secs(5) {
                continue;
            }

            device.is_connected = device.is_connected().await.unwrap();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            let mut devices = self.tokio_rt.block_on(self.devices.write());

            for (addr, device) in devices.iter_mut() {
                color_picker::color_edit_button_srgb(ui, &mut device.current_color);
                ui.heading(format!("Current color is {:?}", device.current_color));

                ui.label("Device:");
                ui.label(format!("Hex UUID: {:?}", addr));
                ui.label(format!("Is connected: {}", device.is_connected));
                ui.label(format!("Brightness: {}%", device.brightness));
            }
        });
    }
}

fn main() -> eframe::Result {
    let rt = Runtime::new().unwrap();
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
