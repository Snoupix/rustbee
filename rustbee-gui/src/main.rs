use std::collections::HashMap;

use eframe::egui::{self, color_picker, CentralPanel};
use eframe::{CreationContext, NativeOptions};

use rustbee_common::bluetooth::HueDevice;

#[derive(Default)]
struct App {
    current_color: [u8; 3],
    devices: HashMap<[u8; 6], HueDevice>,
}

impl App {
    fn new(_cc: &CreationContext<'_>) -> Box<Self> {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Box::new(Self::default())
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            color_picker::color_edit_button_srgb(ui, &mut self.current_color);
            ui.heading(format!("Current color is {:?}", self.current_color));

            for (addr, device) in self.devices.iter() {
                ui.label("Device:");
                ui.label(format!("Hex UUID: {:?}", addr));
            }
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Rustbee",
        NativeOptions::default(),
        Box::new(|cc| Ok(App::new(cc))),
    )?;

    Ok(())
}
