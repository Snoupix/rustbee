use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::*;
use eframe::{CreationContext, NativeOptions};
use egui_extras::{Size, StripBuilder};
use futures::{FutureExt, StreamExt as _};
use serde_json::json;
use tokio::runtime::{self, Runtime};
use tokio::sync::{
    watch::{channel, Receiver},
    RwLock,
};
use tokio::time::{self, Instant};

use rustbee_common::bluetooth::{Client, FoundDevice, HueDevice};
use rustbee_common::color_space::Rgb;
use rustbee_common::colors::Xy;
use rustbee_common::constants::{masks, OutputCode, ADDR_LEN, DATA_LEN, GUI_SAVE_INTERVAL_SECS};
use rustbee_common::utils::launch_daemon;
use rustbee_common::{BluetoothAddr, BluetoothPeripheral as _};

const APP_ID: &str = "Rustbee";
const FONT_NAME: &str = "monaspace";
// When adding a SVG, add `fill="#FFFFFF"` to the path tag because egui expect svgs to be white by
// default so it can "tint" => multiply base values to a color and if it's black, so #000000, it's
// always gonna be black
const LIGHT_BULB_SVG: ImageSource = include_image!("../assets/lightbulb.svg");
const BLUETOOTH_SVG: ImageSource = include_image!("../assets/bluetooth.svg");
const WHITE: Color32 = Color32::from_rgb(0xE7, 0xE7, 0xE4);
const BACKGROUND: Color32 = Color32::from_rgb(0x0F, 0x0F, 0x10);
const SEARCH_MAX_CHARS: usize = DATA_LEN;
const DEVICE_STATE_UPDATE_SECS: u64 = 60;
const DEBOUNCE_SECS: u64 = 5;

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
    /// Don't forget to call .update() after updating the inner value
    current_color: Debounce<[u8; 3]>,
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
            name: Default::default(),
            current_color: Debounce::new([0; 3], Duration::from_secs(DEBOUNCE_SECS)),
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

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedDevice {
    address: [u8; ADDR_LEN],
    name: String,
    current_color: [u8; 3],
    brightness: u8,
}

impl From<&HueDeviceWrapper> for SavedDevice {
    fn from(device: &HueDeviceWrapper) -> Self {
        Self {
            address: device.addr.into_inner(),
            name: device.name.clone(),
            current_color: *device.current_color,
            brightness: device.brightness,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Debounce<T> {
    instant: Instant,
    duration: Duration,
    value: T,
    actual_value: T,
}

impl<T: Copy> Debounce<T> {
    fn new(value: T, duration: Duration) -> Self {
        Self {
            instant: Instant::now(),
            duration,
            value,
            actual_value: value,
        }
    }

    /// Method to call when mutating the debounce value and returns wheter or not the value has
    /// changed
    fn update(&mut self) -> bool {
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

trait Text {
    fn text(&mut self, s: impl Into<String>) -> Response;
    fn header(&mut self, s: impl Into<String>) -> Response;
}

impl Text for Ui {
    fn text(&mut self, s: impl Into<String>) -> Response {
        self.label(RichText::new(s).size(16.))
    }

    fn header(&mut self, s: impl Into<String>) -> Response {
        self.label(RichText::new(s).heading().size(22.).strong())
    }
}

type AppDevices = HashMap<[u8; ADDR_LEN], HueDeviceWrapper>;

struct App {
    devices: Arc<RwLock<AppDevices>>,
    tokio_rt: Runtime,
    devices_color: Debounce<[u8; 3]>,
    devices_brightness: Debounce<u8>,
    device_error: Option<String>,
    device_name_search: String,
    devices_found: Arc<RwLock<Vec<FoundDevice>>>,
    new_device_addr: String,
    is_new_device_addr_error: bool,
    channel: Option<Receiver<bool>>,
}

impl App {
    fn new(
        cc: &CreationContext<'_>,
        devices: Arc<RwLock<AppDevices>>,
        tokio_rt: Runtime,
    ) -> Box<Self> {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            FONT_NAME.to_owned(),
            FontData::from_static(include_bytes!("../assets/MonaspaceKrypton-Regular.otf")),
        );
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, FONT_NAME.to_owned());
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.style_mut(|style| {
            style.debug.debug_on_hover = true;
            style.debug.show_expand_width = true;
            style.debug.show_expand_height = true;
            style.debug.show_resize = true;

            style.spacing.button_padding = vec2(10., 5.);
            style.override_font_id = Some(FontId {
                size: 14.,
                family: FontFamily::Monospace,
            });

            style.visuals.window_fill = BACKGROUND;
            style.visuals.panel_fill = BACKGROUND;

            style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(0x18, 0x18, 0x1B);
            style.visuals.widgets.noninteractive.bg_stroke =
                Stroke::new(2., Color32::from_rgb(0x30, 0x30, 0x36));
            style.visuals.widgets.noninteractive.fg_stroke =
                Stroke::new(1., Color32::from_rgb(0xE7, 0xE7, 0xE4));
            style.visuals.selection.bg_fill = WHITE;
        });

        let mut devices_guard = tokio_rt.block_on(devices.write());

        if let Some(state) = cc.storage {
            for device in state
                .get_string("devices")
                .map(|devices_str| {
                    serde_json::Value::from_str(&devices_str)
                        .unwrap_or(json!([]))
                        .as_array()
                        .cloned()
                        .unwrap_or(Vec::new())
                        .into_iter()
                        .map(|val| serde_json::from_value::<SavedDevice>(val).unwrap())
                        .collect()
                })
                .unwrap_or(Vec::new())
            {
                let mut hue_device =
                    HueDeviceWrapper::from_address(BluetoothAddr::from(device.address));
                hue_device.name = device.name;
                hue_device.current_color =
                    Debounce::new(device.current_color, Duration::from_secs(DEBOUNCE_SECS));

                devices_guard.insert(device.address, hue_device);
            }
        }

        let lower_brightness = devices_guard.iter().fold(100u8, |v, (_, device)| {
            if device.brightness < v {
                device.brightness
            } else {
                v
            }
        });

        drop(devices_guard);

        Box::new(Self {
            devices,
            tokio_rt,
            devices_color: Debounce::new([0; 3], Duration::from_secs(DEBOUNCE_SECS)),
            devices_brightness: Debounce::new(lower_brightness, Duration::from_secs(1)),
            device_error: None,
            device_name_search: String::new(),
            devices_found: Arc::new(RwLock::new(Vec::new())),
            new_device_addr: String::new(),
            is_new_device_addr_error: false,
            channel: None,
        })
    }

    fn add_light_bulb_icon(&self, ui: &mut Ui, scale: f32, color: Option<Color32>) -> Response {
        ui.add_sized(
            vec2(14. * scale, 14. * scale),
            Image::new(LIGHT_BULB_SVG).tint(color.unwrap_or(WHITE)),
        )
    }

    fn add_bluetooth_icon(&self, ui: &mut Ui, scale: f32, color: Option<Color32>) -> Response {
        ui.add_sized(
            vec2(14. * scale, 14. * scale),
            Image::new(BLUETOOTH_SVG).tint(color.unwrap_or(WHITE)),
        )
    }

    fn show_status_circle(&self, ui: &mut Ui, radius: f32, is_on: bool, offset: Option<Vec2>) {
        // let (Response { rect, .. }, painter) =
        //     ui.allocate_painter(Vec2::splat(radius * 1.5), Sense::hover());
        let (_, rect) = ui.allocate_space(Vec2::splat(radius));
        let pos = if let Some(offset) = offset {
            rect.center() - offset
        } else {
            rect.center()
        };

        ui.painter().circle(
            pos,
            radius,
            if is_on {
                Color32::from_rgb(0, 255, 0)
            } else {
                Color32::from_rgb(255, 0, 0)
            },
            Stroke::NONE,
        );
    }

    // fn display_device(
    //     &mut self,
    //     ui: &mut Ui,
    //     addr: [u8; 6],
    //     device: &mut HueDeviceWrapper,
    // ) -> bool {
    //     let devices = Arc::clone(&self.devices);
    //
    //     if device.name.is_empty() {
    //         ui.label("Device:");
    //     } else {
    //         if device.is_connected {
    //             ui.painter().circle(
    //                 ui.next_widget_position(),
    //                 3.,
    //                 Color32::from_rgb(0, 255, 0),
    //                 Stroke::NONE,
    //             );
    //         } else {
    //             ui.painter().circle(
    //                 ui.next_widget_position(),
    //                 3.,
    //                 Color32::from_rgb(255, 0, 0),
    //                 Stroke::NONE,
    //             );
    //         }
    //         ui.label(format!("Device {}:", device.name));
    //     }
    //     ui.label(format!("Hex UUID: {:?}", addr));
    //
    //     if ui.button("Remove device").clicked() {
    //         let devices = Arc::clone(&devices);
    //
    //         self.tokio_rt.spawn(async move {
    //             devices.write().await.remove(&addr);
    //         });
    //
    //         return true;
    //     }
    //
    //     if !device.is_found {
    //         ui.label("Device not found");
    //
    //         if ui.button("Pair device").clicked() {
    //             run_async!(self, async move {
    //                 let mut lock = devices.write().await;
    //                 let device = lock.get_mut(&addr).unwrap();
    //                 match device.pair().await {
    //                     OutputCode::Success => {
    //                         device.is_paired = true;
    //                         device.is_found = true;
    //                         true
    //                     }
    //                     _ => {
    //                         device.is_paired = false;
    //                         device.is_found = false;
    //                         false
    //                     }
    //                 }
    //             });
    //
    //             return true;
    //         }
    //         return false;
    //     }
    //
    //     ui.label(format!("Is paired: {}", device.is_paired));
    //     ui.label(format!("Is connected: {}", device.is_connected));
    //     if device.is_connected {
    //         ui.add(Slider::new(&mut device.brightness, 0..=100).text("Brightness percentage"));
    //         if ui.button("Set brightness value").clicked() {
    //             let device = device.clone();
    //
    //             run_async!(
    //                 self,
    //                 device
    //                     .set_brightness(device.brightness)
    //                     .map(|output| output.is_success())
    //             );
    //
    //             return true;
    //         }
    //
    //         if self.channel.is_none()
    //             && color_picker::color_edit_button_srgb(ui, &mut device.current_color).changed()
    //             && device.current_color.update()
    //         {
    //             let (r, g, b) = (
    //                 device.current_color[0],
    //                 device.current_color[1],
    //                 device.current_color[2],
    //             );
    //             let Xy {
    //                 x,
    //                 y,
    //                 brightness: _,
    //             } = Xy::from(Rgb::new(r as _, g as _, b as _));
    //             let device = device.clone();
    //             run_async!(self, async move {
    //                 device
    //                     .set_colors(x as _, y as _, masks::COLOR_RGB)
    //                     .await
    //                     .is_success()
    //             });
    //         }
    //         ui.label(format!("Current color is {:?}", device.current_color));
    //
    //         if device.power_state {
    //             ui.painter().circle(
    //                 ui.next_widget_position(),
    //                 3.,
    //                 Color32::from_rgb(0, 255, 0),
    //                 Stroke::NONE,
    //             );
    //         } else {
    //             ui.painter().circle(
    //                 ui.next_widget_position(),
    //                 3.,
    //                 Color32::from_rgb(255, 0, 0),
    //                 Stroke::NONE,
    //             );
    //         }
    //         if device.power_state {
    //             if ui.button("Power OFF").clicked() {
    //                 let device = device.clone();
    //
    //                 run_async!(self, async move {
    //                     let res = device.set_power(false).await.is_success();
    //                     if res {
    //                         let mut lock = devices.write().await;
    //                         let device = lock.get_mut(&addr).unwrap();
    //                         device.power_state = false;
    //                     }
    //                     res
    //                 });
    //
    //                 return true;
    //             }
    //         } else if ui.button("Power ON").clicked() {
    //             let device = device.clone();
    //
    //             run_async!(self, async move {
    //                 let res = device.set_power(true).await.is_success();
    //                 if res {
    //                     let mut lock = devices.write().await;
    //                     let device = lock.get_mut(&addr).unwrap();
    //                     device.power_state = true;
    //                 }
    //                 res
    //             });
    //
    //             return true;
    //         }
    //
    //         if ui.button("Disconnect from device").clicked() {
    //             let device = device.clone();
    //
    //             run_async!(self, async move {
    //                 let res = device.disconnect_device().await.is_success();
    //                 if res {
    //                     let mut lock = devices.write().await;
    //                     let device = lock.get_mut(&addr).unwrap();
    //                     update_device_state(device).await;
    //                 }
    //                 res
    //             });
    //
    //             return true;
    //         }
    //     } else if ui.button("Connect to device").clicked() {
    //         let device = device.clone();
    //
    //         run_async!(self, async move {
    //             let res = device.connect_device().await.is_success();
    //             if res {
    //                 let mut lock = devices.write().await;
    //                 let device = lock.get_mut(&addr).unwrap();
    //                 update_device_state(device).await;
    //             }
    //             res
    //         });
    //
    //         return true;
    //     }
    //
    //     false
    // }

    fn display_device(
        &mut self,
        ui: &mut Ui,
        addr: [u8; 6],
        device: &mut HueDeviceWrapper,
    ) -> bool {
        let mut reset_frame = false;
        let devices = Arc::clone(&self.devices);
        let size = ui.available_size();

        Frame::none()
            .fill(Color32::from_rgb(0x18, 0x18, 0x1B))
            .rounding(Rounding::same(10.))
            .inner_margin(Margin::symmetric(25., 10.))
            .show(ui, |ui| {
                StripBuilder::new(ui)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .clip(true)
                    .sizes(Size::exact(25.), 8)
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.label(format!("{size}"));
                        });

                        strip.strip(|builder| {
                            builder.sizes(Size::remainder(), 3).horizontal(|mut strip| {
                                strip.empty();
                                strip.cell(|ui| {
                                    if device.name.is_empty() {
                                        ui.header("Unknown name");
                                    } else {
                                        self.add_light_bulb_icon(ui, 2., None);
                                        ui.header(&device.name);
                                    }
                                });
                                strip.empty();
                            });
                        });

                        if device.is_connected {
                            strip.empty();
                        }

                        strip.strip(|builder| {
                            builder.sizes(Size::remainder(), 3).horizontal(|mut strip| {
                                strip.empty();
                                strip.cell(|ui| {
                                    self.show_status_circle(ui, 6., device.is_connected, None);
                                    ui.text(if device.is_connected {
                                        "Connected"
                                    } else {
                                        "Disconnected"
                                    });
                                });
                                strip.empty();
                            });
                        });

                        // if !device.is_connected {
                        //     return;
                        // }

                        strip.strip(|builder| {
                            builder.sizes(Size::remainder(), 3).horizontal(|mut strip| {
                                strip.cell(|ui| {
                                    ui.text("Power");
                                });
                                strip.empty();
                                strip.strip(|builder| {
                                    builder
                                        .size(Size::relative(1.))
                                        .cell_layout(Layout::right_to_left(Align::Min))
                                        .horizontal(|mut strip| {
                                            strip.cell(|ui| {
                                                ui.text(if device.power_state {
                                                    "On"
                                                } else {
                                                    "Off"
                                                });
                                                self.show_status_circle(
                                                    ui,
                                                    6.,
                                                    device.power_state,
                                                    Some(vec2(-5., -3.)),
                                                );
                                            });
                                        });
                                });
                            });
                        });

                        strip.cell(|ui| {
                            ui.text("Brightness");
                            ui.add(
                                Slider::new(&mut device.brightness, 0..=100)
                                    .suffix("%")
                                    .trailing_fill(true),
                            );
                            if ui.button("set").clicked() {
                                let device = device.clone();

                                run_async!(
                                    self,
                                    device
                                        .set_brightness(device.brightness)
                                        .map(|output| output.is_success())
                                );

                                reset_frame = true;
                            }
                        });

                        strip.strip(|builder| {
                            builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                                let (r, g, b) = (
                                    device.current_color[0],
                                    device.current_color[1],
                                    device.current_color[2],
                                );

                                strip.cell(|ui| {
                                    ui.text("Color");
                                    ui.label(format!("{:?}", *device.current_color));
                                    ui.painter().circle_filled(
                                        ui.next_widget_position(),
                                        7.,
                                        Color32::from_rgb(r, g, b),
                                    );
                                });

                                strip.cell(|ui| {
                                    let picker = color_picker::color_edit_button_srgb(
                                        ui,
                                        &mut device.current_color,
                                    );
                                    if (picker.changed() || picker.clicked_elsewhere())
                                        && device.current_color.update()
                                    {
                                        let Xy {
                                            x,
                                            y,
                                            brightness: _,
                                        } = Xy::from(Rgb::new(r as _, g as _, b as _));
                                        let device = device.clone();
                                        run_async!(self, async move {
                                            device
                                                .set_colors(x as _, y as _, masks::COLOR_RGB)
                                                .await
                                                .is_success()
                                        });
                                    }
                                });
                            });
                        });

                        strip.strip(|builder| {
                            builder
                                .size(Size::remainder())
                                .cell_layout(Layout::right_to_left(Align::Min))
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        if ui
                                            .button(format!(
                                                "Turn {}",
                                                if device.power_state { "Off" } else { "On" }
                                            ))
                                            .clicked()
                                        {
                                            let device = device.clone();

                                            run_async!(self, async move {
                                                let res = device
                                                    .set_power(!device.power_state)
                                                    .await
                                                    .is_success();

                                                if res {
                                                    let mut lock = devices.write().await;
                                                    let device = lock.get_mut(&addr).unwrap();
                                                    update_device_state(device).await;
                                                }

                                                res
                                            });

                                            reset_frame = true;
                                        }
                                    });
                                });
                        });
                    });
            });

        // Frame::none()
        //     .fill(Color32::from_rgb(0x18, 0x18, 0x1B))
        //     .rounding(Rounding::same(10.))
        //     .show(ui, |ui| {
        //         ui.add_sized(size, |ui: &mut Ui| {
        //             Grid::new(format!("Device widget {:?}", addr))
        //                 .num_columns(3)
        //                 .show(ui, |ui| {
        //                     ui.label(format!("{size}"));
        //                     ui.end_row();
        //
        //                     ui.horizontal(|ui| {
        //                         self.add_light_bulb_icon(ui, 2., None);
        //                         if device.name.is_empty() {
        //                             ui.heading("Unknown name");
        //                         } else {
        //                             ui.heading(&device.name);
        //                         }
        //                     });
        //                     ui.end_row();
        //                     ui.end_row();
        //
        //                     ui.horizontal(|ui| {
        //                         ui.label(if device.is_connected {
        //                             "Connected"
        //                         } else {
        //                             "Disconnected"
        //                         });
        //                         self.show_status_circle(
        //                             ui,
        //                             ui.next_widget_position(),
        //                             device.is_connected,
        //                         );
        //                     });
        //                     ui.label("");
        //                     ui.label("");
        //                     ui.end_row();
        //
        //                     if !device.is_connected {
        //                         return;
        //                     }
        //
        //                     ui.horizontal(|ui| {
        //                         ui.label("Power");
        //                         self.show_status_circle(
        //                             ui,
        //                             ui.next_widget_position(),
        //                             device.power_state,
        //                         );
        //                     });
        //                     ui.label("");
        //                     ui.label(if device.power_state { "On" } else { "Off" });
        //                     ui.end_row();
        //
        //                     ui.horizontal(|ui| {
        //                         ui.label("Brightness");
        //                         ui.add(
        //                             Slider::new(&mut device.brightness, 0..=100)
        //                                 .suffix("%")
        //                                 .trailing_fill(true),
        //                         );
        //                     });
        //                     ui.label("");
        //                     if ui.button("set").clicked() {
        //                         let device = device.clone();
        //
        //                         run_async!(
        //                             self,
        //                             device
        //                                 .set_brightness(device.brightness)
        //                                 .map(|output| output.is_success())
        //                         );
        //
        //                         reset_frame = true;
        //                     }
        //                     ui.end_row();
        //
        //                     let (r, g, b) = (
        //                         device.current_color[0],
        //                         device.current_color[1],
        //                         device.current_color[2],
        //                     );
        //
        //                     ui.horizontal(|ui| {
        //                         ui.label("Color");
        //                         ui.label(format!("{:?}", *device.current_color));
        //                         ui.painter().circle_filled(
        //                             ui.next_widget_position(),
        //                             7.,
        //                             Color32::from_rgb(r, g, b),
        //                         );
        //                     });
        //                     ui.label("");
        //                     let picker =
        //                         color_picker::color_edit_button_srgb(ui, &mut device.current_color);
        //                     if (picker.changed() || picker.clicked_elsewhere())
        //                         && device.current_color.update()
        //                     {
        //                         let Xy {
        //                             x,
        //                             y,
        //                             brightness: _,
        //                         } = Xy::from(Rgb::new(r as _, g as _, b as _));
        //                         let device = device.clone();
        //                         run_async!(self, async move {
        //                             device
        //                                 .set_colors(x as _, y as _, masks::COLOR_RGB)
        //                                 .await
        //                                 .is_success()
        //                         });
        //                     }
        //                     ui.end_row();
        //                     // TODO: Impl missing funcionnalities and match the canvas
        //
        //                     ui.label("");
        //                     ui.label("");
        //                     if ui
        //                         .button(format!(
        //                             "Turn {}",
        //                             if device.power_state { "Off" } else { "On" }
        //                         ))
        //                         .clicked()
        //                     {}
        //                 })
        //                 .response
        //         });
        //     });

        reset_frame
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let devices = Arc::clone(&self.devices);

        TopBottomPanel::top("banner")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Top panel frame
                    Frame::none()
                        .inner_margin(Margin::symmetric(10., 20.))
                        .show(ui, |ui| {
                            // Top panel layout
                            ui.horizontal(|ui| {
                                ui.add_space(5.);
                                self.add_light_bulb_icon(ui, 2.5, None);
                                ui.heading(RichText::new("Rustbee").strong().size(24.));

                                /* if !self.new_device_addr.is_empty() && self.is_new_device_addr_error {
                                    ui.horizontal(|ui| {
                                        ui.label("Error on parsing Address, please respect the following format: ff:aa:55:ff:aa:55");
                                    });
                                }

                                ui.horizontal(|ui| {
                                    self.add_bluetooth_icon(ui, 2.);
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
                                                if let Some(storage) = frame.storage_mut() {
                                                    self.save(storage);
                                                }
                                            },
                                            Err(_) => {
                                                self.is_new_device_addr_error = true;
                                            },
                                        }
                                    }
                                }); */

                                ui.add_space(ui.available_width() - 350.);
                                Frame::none()
                                    .stroke(Stroke::new(1., Color32::WHITE))
                                    .rounding(Rounding::same(25.))
                                    .fill(Color32::from_rgb(0x21, 0x1F, 0x1D))
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing = vec2(25., 0.);
                                        ui.with_layout(
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
                                                let size = ui.available_size();
                                                ui.add_space(25.);
                                                Frame::none()
                                                    .inner_margin(Margin {
                                                        top: size.y / 5.,
                                                        ..Default::default()
                                                    })
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            RichText::new("Search:")
                                                                .size(16.)
                                                                .color(WHITE),
                                                        );
                                                    });
                                                ui.add(
                                                    TextEdit::singleline(
                                                        &mut self.device_name_search,
                                                    )
                                                    .horizontal_align(Align::Center)
                                                    .vertical_align(Align::Center)
                                                    .desired_width(110.)
                                                    .char_limit(SEARCH_MAX_CHARS)
                                                    .margin(Margin {
                                                        top: size.y / 4.,
                                                        ..Default::default()
                                                    })
                                                    .text_color(WHITE)
                                                    .font(FontId {
                                                        size: 18.,
                                                        family: FontFamily::Monospace,
                                                    })
                                                    .min_size(vec2(0., size.y))
                                                    .frame(false),
                                                );

                                                let mut btn = Frame::none()
                                                    .inner_margin(Margin::same(2.5))
                                                    .rounding(Rounding::same(50.))
                                                    .stroke(Stroke::new(1., WHITE))
                                                    .fill(WHITE)
                                                    .begin(ui);

                                                {
                                                    self.add_bluetooth_icon(
                                                        &mut btn.content_ui,
                                                        1.5,
                                                        Some(BACKGROUND),
                                                    );
                                                }

                                                let btn_response = btn
                                                    .allocate_space(ui)
                                                    .on_hover_text("Search a Philips Hue device by (partial) name")
                                                    .on_hover_cursor(CursorIcon::PointingHand);

                                                if btn_response.hovered() {
                                                    btn.frame = btn.frame.fill(Color32::TRANSPARENT);
                                                }

                                                if btn_response.interact(Sense::click()).clicked {
                                                    let name = self.device_name_search.clone();
                                                    let devices_found_ref =
                                                        Arc::clone(&self.devices_found);

                                                    run_async!(self, async move {
                                                        let name = name;
                                                        let mut stream =
                                                            HueDevice::search_by_name(&name).await;

                                                        while let Some(device) = stream.next().await
                                                        {
                                                            let mut devices_found =
                                                                devices_found_ref.write().await;
                                                            devices_found.push(device);
                                                        }

                                                        true
                                                    });
                                                }
                                                btn.paint(ui);

                                                ui.add_space(0.);
                                            },
                                        );
                                    });
                            });
                        });

                    let devices_found = self.tokio_rt.block_on(self.devices_found.read());

                    if !devices_found.is_empty() {
                        ui.horizontal(|ui| {
                            if self.channel.is_none() && ui.button("close").clicked() {
                                let devices_found_ref = Arc::clone(&self.devices_found);

                                run_async!(self, async move {
                                    // This causes a race on the read block_on above but it's fast
                                    // enough not to cause any issues
                                    let mut devices_found = devices_found_ref.write().await;
                                    devices_found.clear();

                                    true
                                });
                                return;
                            }

                            ui.vertical_centered(|ui| {
                                ui.text("Devices found:");

                                for device in devices_found.iter() {
                                    let devices = Arc::clone(&devices);
                                    let addr = device.address;
                                    let btn = ui.button(format!(
                                        "{} - {}",
                                        device.name,
                                        BluetoothAddr::from(addr)
                                    ));

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
                                            let mut device = HueDeviceWrapper::from_address(
                                                BluetoothAddr::from(addr),
                                            );

                                            match device.pair().await {
                                                OutputCode::Success => {
                                                    device.is_paired = true;
                                                    device.is_found = true;
                                                }
                                                _ => {
                                                    device.is_paired = false;
                                                    device.is_found = false;
                                                }
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
            ui.allocate_ui(ui.available_size(), |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    if let Some(ref error) = self.device_error {
                        if !error.is_empty() {
                            ui.horizontal(|ui| {
                                ui.text(format!("Error {error}"));
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
                                    ctx.request_repaint();
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.spinner();
                                    });
                                    return;
                                }
                            }
                            Err(_) => {
                                self.channel = None;
                                ctx.request_repaint();
                            }
                        }
                    }

                    let devices_ref = Arc::clone(&self.devices);
                    let mut devices_mut = self.tokio_rt.block_on(devices_ref.write());

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

                            update_all_devices_state(devices).await;

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

                            update_all_devices_state(devices).await;

                            !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                        });
                        return;
                    }

                    ui.horizontal(|ui| {
                        ui.text(format!("Devices brightness {}%", *self.devices_brightness));
                        let slider = ui.add(
                            Slider::new(&mut *self.devices_brightness, 0..=100)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        if slider.changed() && self.devices_brightness.update() {
                            let percentage = *self.devices_brightness;
                            let devices_ref = Arc::clone(&devices);

                            run_async!(self, async move {
                                let devices_read = devices_ref.read().await;
                                let futures = devices_read
                                    .iter()
                                    .map(|(_, device)| device.set_brightness(percentage))
                                    .collect::<Vec<_>>();
                                let res = futures::future::join_all(futures).await;
                                drop(devices_read);

                                update_all_devices_state(devices_ref).await;

                                !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                            });
                        }
                    });

                    if self.channel.is_none()
                        && color_picker::color_edit_button_srgb(ui, &mut self.devices_color)
                            .changed()
                        && self.devices_color.update()
                    {
                        let color = *self.devices_color;
                        let devices_ref = Arc::clone(&devices);

                        run_async!(self, async move {
                            let mut res = Vec::new();

                            for (_, device) in devices_ref.read().await.iter() {
                                let (r, g, b) = (color[0], color[1], color[2]);
                                let Xy {
                                    x,
                                    y,
                                    brightness: _,
                                } = Xy::from(Rgb::new(r as _, g as _, b as _));
                                // TODO: Fixme
                                res.push(device.set_colors(x as _, y as _, masks::COLOR_RGB).await);
                            }

                            !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                        });
                    }

                    // Commented out since every command ensures connection and disconnects on socket
                    // shutdown
                    /* if ui.button("Disconnect from all devices").clicked() {
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
                        } */

                    if ui.button("Connect to all devices").clicked() {
                        run_async!(self, async {
                            let devices_read = devices.read().await;
                            let futures = devices_read
                                .iter()
                                .map(|(_, device)| device.connect_device())
                                .collect::<Vec<_>>();
                            let res = futures::future::join_all(futures).await;
                            drop(devices_read);

                            update_all_devices_state(devices).await;

                            !res.into_iter().fold(true, |acc, v| !acc || !v.is_success())
                        });
                        return;
                    }

                    ui.separator();

                    // Grid::new("devices")
                    //     .striped(true)
                    //     .spacing(vec2(15., 15.))
                    //     .min_col_width(ui.available_width() / 4.)
                    //     .min_row_height(ui.available_height() / 2.)
                    //     .show(ui, |ui| {
                    //         for (addr, device) in devices_mut.iter_mut() {
                    //             if self.display_device(ui, *addr, device) {
                    //                 return;
                    //             }
                    //             ui.end_row();
                    //         }
                    //     });
                    // let (addr, device) = devices_mut.iter_mut().next().unwrap();
                    // self.display_device(ui, *addr, device);
                    let width = ui.available_width();
                    let height = ui.available_height();
                    static WIDGET_WIDTH: f32 = 420.;
                    static WIDGET_HEIGHT: f32 = 480.;
                    let widget_count = f32::floor(width / WIDGET_WIDTH);
                    Frame::none()
                        .inner_margin(Margin::same(20.))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(20.);
                            egui_extras::StripBuilder::new(ui)
                                .sizes(
                                    //egui_extras::Size::initial(height / 2.),
                                    egui_extras::Size::initial(WIDGET_HEIGHT),
                                    (devices_mut.len() as f32 / widget_count).ceil().max(1.) as _,
                                )
                                .cell_layout(Layout::top_down(Align::Min))
                                .vertical(|mut strip| {
                                    // TODO: Add multiple lines when devices len * widget width >=
                                    // available width
                                    strip.strip(|builder| {
                                        builder
                                            .sizes(
                                                egui_extras::Size::initial(width / widget_count),
                                                devices_mut.len(),
                                            )
                                            .cell_layout(Layout::left_to_right(Align::Min))
                                            .horizontal(|mut strip| {
                                                let mut reset_frame = false;
                                                for (addr, device) in devices_mut.iter_mut() {
                                                    strip.cell(|ui| {
                                                        reset_frame =
                                                            self.display_device(ui, *addr, device);
                                                    });

                                                    if reset_frame {
                                                        return;
                                                    }
                                                }
                                            });
                                    });
                                });
                        });
                });
            });
        });
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(GUI_SAVE_INTERVAL_SECS)
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let devices_ref = Arc::clone(&self.devices);
        let devices = self.tokio_rt.block_on(devices_ref.read());

        storage.set_string(
            "devices",
            json!(devices
                .values()
                .map(SavedDevice::from)
                .map(|device| json!({
                    "name": device.name,
                    "address": device.address,
                    "current_color": device.current_color,
                    "brightness": device.brightness,
                }))
                .collect::<Vec<_>>())
            .to_string(),
        );

        storage.flush();
    }
}

fn main() -> eframe::Result {
    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let state: &'static Arc<RwLock<AppDevices>> =
        Box::leak(Box::new(Arc::new(RwLock::new(HashMap::new()))));
    let state_async = Arc::clone(state);
    let app_options = NativeOptions {
        persistence_path: eframe::storage_dir(APP_ID),
        ..Default::default()
    };

    rt.block_on(launch_daemon()).unwrap();

    // Thread used to init devices state and sync devices state on a timout
    rt.spawn(async move {
        // let mut hue_devices = get_devices(&[HUE_BAR_1_ADDR, HUE_BAR_2_ADDR])
        //     .await
        //     .unwrap()
        //     .into_iter();
        // let default_devices = HashMap::from([
        //     (HUE_BAR_1_ADDR, hue_devices.next().unwrap()),
        //     (HUE_BAR_2_ADDR, hue_devices.next().unwrap()),
        // ]);

        // Implicit drop by using the lock right away
        // *state_async.write().await = default_devices
        //     .into_iter()
        //     .map(|(addr, device)| (addr, device.into()))
        //     .collect::<AppDevices>();

        // There must be a loop to update state in case devices state gets updated by a thrird party app
        loop {
            for (_, device) in state_async.write().await.iter_mut() {
                // if device.is_initiated || device.last_update.elapsed() < Duration::from_secs(DEVICE_STATE_UPDATE_SECS)
                if device.last_update.elapsed() < Duration::from_secs(DEVICE_STATE_UPDATE_SECS) {
                    continue;
                }

                update_device_state(device).await;
            }

            time::sleep(Duration::from_millis(1000)).await;
        }
    });

    eframe::run_native(
        APP_ID,
        app_options,
        Box::new(|cc| Ok(App::new(cc, Arc::clone(state), rt))),
    )?;

    Ok(())
}

// fn parse_address(str: &str) -> Result<BluetoothAddr, String> {
//     BluetoothAddr::from_str(str).map_err(|e| e.0)
// }

async fn update_all_devices_state(devices: Arc<RwLock<AppDevices>>) {
    for (_, device) in devices.write().await.iter_mut() {
        update_device_state(device).await;
    }
}

async fn update_device_state(device: &mut HueDeviceWrapper) {
    let (res_conn, buf_conn) = device.is_connected().await;
    if res_conn.is_success() {
        device.is_connected = buf_conn[0] == true as u8;
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
            device.is_paired = true;
            device.is_found = true;
        }
    }
    device.is_initiated = true;
    device.last_update = Instant::now();
}
