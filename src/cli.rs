use std::f64;

use clap::{Parser, Subcommand};
use color_space::{FromRgb, Rgb, Xyz};

use rustbee_common::bluetooth::{Client, HueDevice};
use rustbee_common::colors::Xy;
use rustbee_common::constants::{masks::*, MaskT, ADDR_LEN};

#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
    #[arg(short = 'a', long = "addresses")]
    pub hex_mac_addresses: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Subcommand, Clone)]
pub enum Command {
    PairAndTrust,
    Power {
        #[command(subcommand)]
        state: Option<State>,
    },
    ColorRgb {
        r: Option<u8>,
        g: Option<u8>,
        b: Option<u8>,
    },
    ColorHex {
        hex: Option<String>,
    },
    ColorXy {
        x: Option<f64>,
        y: Option<f64>,
    },
    Brightness {
        value: Option<u8>,
    },
    Disconnect,
    Shutdown {
        #[arg(short = 'f', long)]
        force: bool,
    },
    Gui,
    Logs, // Only here for the CLI help
}

#[derive(Clone, Debug, PartialEq, Subcommand)]
pub enum State {
    On,
    Off,
}

impl From<&Command> for MaskT {
    fn from(value: &Command) -> Self {
        match value {
            Command::PairAndTrust => PAIR,
            Command::Power { .. } => POWER,
            Command::ColorRgb { .. } => COLOR_RGB,
            Command::ColorHex { .. } => COLOR_HEX,
            Command::ColorXy { .. } => COLOR_XY,
            Command::Brightness { .. } => BRIGHTNESS,
            Command::Disconnect => DISCONNECT,
            command @ Command::Gui
            | command @ Command::Logs
            | command @ Command::Shutdown { .. } => {
                unreachable!("This command {command:?} shouldn't communicate with the daemon")
            }
        }
    }
}

impl Command {
    pub async fn handle(&self, hue_device: HueDevice<Client>) -> btleplug::Result<()> {
        if matches!(self, Self::Gui | Self::Logs | Self::Shutdown { .. }) {
            // Should never occur since it's handled before
            return Ok(());
        }

        if !hue_device.pair().await.is_success() {
            eprintln!("Error: failed to pair and trust device {}", hue_device.addr);
            return Ok(());
        }

        match self {
            Self::Gui | Self::Logs | Self::Shutdown { .. } | Self::PairAndTrust => (),
            Self::Power { state } => match state {
                Some(state) => {
                    if !hue_device
                        .set_power(matches!(*state, State::On))
                        .await
                        .is_success()
                    {
                        eprintln!(
                            "[ERROR] Failed to write power state to hue device address: {}",
                            hue_device.addr
                        );
                    }
                }
                None => {
                    let (res, state) = hue_device.get_power().await;
                    let success = res.is_success();

                    if !success {
                        eprintln!(
                            "[ERROR] Failed to read power state to hue device address: {}",
                            hue_device.addr
                        );
                    } else {
                        let (code, buf) = hue_device.get_name().await;
                        let name = if !code.is_success() {
                            eprintln!(
                                "[ERROR] Failed to read device name from hue device address: {}",
                                hue_device.addr
                            );
                            String::new()
                        } else {
                            String::from_utf8(buf.to_vec()).unwrap()
                        };

                        println!(
                            "Device{} {} is {}",
                            if name.is_empty() {
                                name
                            } else {
                                format!(" {name}")
                            },
                            hue_device.addr,
                            if state[0] == 1 { "ON" } else { "OFF" }
                        );
                    }
                }
            },
            Self::Brightness { value } => match value {
                Some(value) => {
                    assert!(
                        (0..=100).contains(value),
                        "[ERROR] Brightness value must be between 0 and 100 inclusive"
                    );

                    if !hue_device.set_brightness(*value).await.is_success() {
                        eprintln!(
                            "[ERROR] Failed to write brightness state to hue device address: {}",
                            hue_device.addr
                        );
                    }
                }
                None => {
                    let (res, brightness) = hue_device.get_brightness().await;
                    let success = res.is_success();

                    if !success {
                        eprintln!(
                            "[ERROR] Failed to get brightness level from hue device address: {}",
                            hue_device.addr
                        );
                    } else {
                        let (code, buf) = hue_device.get_name().await;
                        let name = if !code.is_success() {
                            eprintln!(
                                "[ERROR] Failed to read device name from hue device address: {}",
                                hue_device.addr
                            );
                            String::new()
                        } else {
                            String::from_utf8(buf.to_vec()).unwrap()
                        };

                        println!(
                            "Device{} {} brightness level is {}%",
                            if name.is_empty() {
                                name
                            } else {
                                format!(" {name}")
                            },
                            hue_device.addr,
                            (brightness[0] as f32 / 255.) * 100.
                        );
                    }
                }
            },
            Self::ColorHex { .. } | Self::ColorXy { .. } | Self::ColorRgb { .. } => {
                let mut read = false;
                let (mut x, mut y) = (0., 0.);

                match self {
                    Self::ColorRgb {
                        ref r,
                        ref g,
                        ref b,
                    } => {
                        if r.is_none() || g.is_none() || b.is_none() {
                            read = true;
                        } else {
                            // let xyz = Xyz::from_rgb(&Rgb::new(
                            //     r.unwrap() as _,
                            //     g.unwrap() as _,
                            //     b.unwrap() as _,
                            // ));
                            // (x, y) = (xyz.x / 100., xyz.y / 100.);
                            let xy = Xy::from(Rgb::new(
                                r.unwrap() as _,
                                g.unwrap() as _,
                                b.unwrap() as _,
                            ));
                            x = xy.x;
                            y = xy.y;
                        }
                    }
                    Self::ColorHex { hex } => {
                        if hex.is_none() {
                            read = true;
                        } else {
                            let hex = hex.clone().unwrap();
                            assert!(
                                hex.len() == ADDR_LEN,
                                "Hex lenght must be {ADDR_LEN} like so: ffFF00"
                            );
                            let odd_it = hex.chars().skip(1).step_by(2);
                            let [r, g, b] = hex
                                .chars()
                                .step_by(2)
                                .zip(odd_it)
                                .map(|(bit1, bit2)| {
                                    i32::from_str_radix(&format!("{bit1}{bit2}"), 16).unwrap()
                                        as f64
                                })
                                .collect::<Vec<_>>()[..]
                            else {
                                panic!("Unexpected error: cannot get RGB values from HEX {hex}")
                            };
                            let xyz = Xyz::from_rgb(&Rgb::new(r, g, b));
                            (x, y) = (xyz.x / 100., xyz.y / 100.);
                        }
                    }
                    Self::ColorXy {
                        x: ref _x,
                        y: ref _y,
                    } => {
                        if _x.is_none() || _y.is_none() {
                            read = true;
                        } else {
                            (x, y) = (_x.unwrap(), _y.unwrap());
                        }
                    }
                    _ => unreachable!(),
                };

                if read {
                    let (res, data) = hue_device.get_colors(MaskT::from(self)).await;
                    let success = res.is_success();

                    if !success {
                        eprintln!(
                            "[ERROR] Failed to get color data from hue device address: {}",
                            hue_device.addr
                        );
                    } else {
                        let x = u16::from_le_bytes([data[0], data[1]]) as f64 / 0xFFFF as f64;
                        let y = u16::from_le_bytes([data[2], data[3]]) as f64 / 0xFFFF as f64;
                        let xy = Xy::new(x, y);
                        let xyz = Xyz::new(x, y, 1. - x - y);

                        // TODO: Fix colors display / color processing
                        match self {
                            Self::ColorRgb { .. } => {
                                let (res, brightness) = hue_device.get_brightness().await;
                                let success = res.is_success();

                                if !success {
                                    eprintln!(
                                        "[ERROR] Failed to get brightness to calculate XYZ color"
                                    );
                                    return Ok(());
                                }

                                let rgb = xy.to_rgb(brightness[0] as f64 / 255.);
                                assert!(rgb.r * 100. <= 255.);
                                assert!(rgb.g * 100. <= 255.);
                                assert!(rgb.b * 100. <= 255.);
                                println!(
                                    "Device color is ({:.0}, {:.0}, {:.0}) ({:?})",
                                    rgb.r * 100.,
                                    rgb.g * 100.,
                                    rgb.b * 100.,
                                    Rgb::from(xyz)
                                );
                            }
                            Self::ColorHex { .. } => {
                                let rgb = Rgb::from(xyz);
                                let hex = [rgb.b as u8, rgb.g as u8, rgb.r as u8]
                                    .into_iter()
                                    .fold(String::new(), |_, v| format!("{v:06x}"));
                                println!("Device color is #{hex}");
                            }
                            Self::ColorXy { .. } => {
                                println!("Device color is x: {:.3}, y: {:.3}", xyz.x, xyz.y);
                            }
                            _ => unreachable!(),
                        }
                    }
                } else {
                    let scaled_x = (x * 0xFFFF as f64) as u16;
                    let scaled_y = (y * 0xFFFF as f64) as u16;

                    if !hue_device
                        .set_colors(scaled_x, scaled_y, MaskT::from(self))
                        .await
                        .is_success()
                    {
                        eprintln!(
                            "Error: daemon failed to disconnect from device {}",
                            hue_device.addr
                        );
                    }
                }
            }
            Self::Disconnect => {
                if !hue_device.disconnect_device().await.is_success() {
                    eprintln!(
                        "Error: daemon failed to disconnect from device {}",
                        hue_device.addr
                    );
                }
            }
        }

        Ok(())
    }
}
