use std::f64;

use crate::*;

use clap::{Parser, Subcommand};
use color_space::{FromRgb, Rgb, Xyz};

#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
    #[arg(short = 'a', long = "addresses")]
    pub hex_mac_addresses: Option<Vec<String>>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    PairAndTrust,
    Power {
        // #[arg(short = 's', long)]
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
}

#[derive(Clone, Debug, Subcommand)]
pub enum State {
    On,
    Off,
}

impl Command {
    pub async fn job(this: Self, mut hue_bar: HueBar) -> bluer::Result<()> {
        if !matches!(this, Self::PairAndTrust) {
            hue_bar.init_connection().await?;
        }

        hue_bar.ensure_pairing().await?;

        match this {
            Self::PairAndTrust => (),
            Self::Power { ref state } => {
                if let Some(state) = state {
                    if !hue_bar
                        .write_gatt_char(
                            &LIGHT_SERVICE,
                            &POWER,
                            &[matches!(*state, State::On) as _],
                        )
                        .await?
                    {
                        eprintln!(
                            "[ERROR] Failed to write power state to hue bar address: {}",
                            hue_bar.addr
                        );
                    }
                } else {
                    let read = hue_bar.read_gatt_char(&LIGHT_SERVICE, &POWER).await?;

                    if let Some(bytes) = read {
                        println!(
                            "Device is {}",
                            if *bytes.first().unwrap() == true as _ {
                                "ON"
                            } else {
                                "OFF"
                            }
                        );
                    } else {
                        eprintln!(
                            "[ERROR] Service or Characteristic \"{POWER}\" for \"{LIGHT_SERVICE}\" not found for device {}", hue_bar.addr
                        );
                    }
                }
            }
            Self::Brightness { value } => match value {
                Some(value) => {
                    assert!(
                        (0..=100).contains(&value),
                        "[ERROR] Brightness value must be between 0 and 100 inclusive"
                    );

                    if !hue_bar
                        .write_gatt_char(
                            &LIGHT_SERVICE,
                            &BRIGHTNESS,
                            &[(((value as f32) / 100.) * 0xff as f32) as u8],
                        )
                        .await?
                    {
                        eprintln!(
                            "[ERROR] Service or Characteristic \"{POWER}\" for \"{LIGHT_SERVICE}\" not found for device {}", hue_bar.addr
                        );
                    }
                }
                None => {
                    let brightness = (*hue_bar
                        .read_gatt_char(&LIGHT_SERVICE, &BRIGHTNESS)
                        .await?
                        .expect("Cannot get brightness level")
                        .first()
                        .expect("Cannot get first byte, shouldn't happen"))
                        as f32;

                    println!(
                        "Device brightness level is {:.2}%",
                        (brightness / 255.) * 100.
                    );
                }
            },
            Self::ColorHex { .. } | Self::ColorXy { .. } | Self::ColorRgb { .. } => {
                let mut read = false;
                let (mut x, mut y) = (0., 0.);

                match this {
                    Self::ColorRgb {
                        ref r,
                        ref g,
                        ref b,
                    } => {
                        if r.is_none() || g.is_none() || b.is_none() {
                            read = true;
                        } else {
                            let xyz = Xyz::from_rgb(&Rgb::new(
                                r.unwrap() as _,
                                g.unwrap() as _,
                                b.unwrap() as _,
                            ));
                            (x, y) = (xyz.x / 100., xyz.y / 100.);
                        }
                    }
                    Self::ColorHex { ref hex } => {
                        if hex.is_none() {
                            read = true;
                        } else {
                            let hex = hex.clone().unwrap();
                            assert!(hex.len() == 6, "Hex lenght must be 6 like so: ffFF00");
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

                if read || x == 0. || y == 0. {
                    let buf = hue_bar
                        .read_gatt_char(&LIGHT_SERVICE, &COLOR)
                        .await?
                        .expect("Cannot get xy colors");

                    let xyz = Xyz::new(
                        u16::from_le_bytes([buf[0], buf[1]]) as f64 / 0xFFFF as f64,
                        u16::from_le_bytes([buf[2], buf[3]]) as f64 / 0xFFFF as f64,
                        0.,
                    );

                    // TODO: Fix colors display
                    match this {
                        Self::ColorRgb { .. } => {
                            let rgb = Rgb::from(xyz);
                            println!("Device color is ({:.0}, {:.0}, {:.0})", rgb.r, rgb.g, rgb.b);
                        }
                        Self::ColorHex { .. } => {
                            let rgb = Rgb::from(xyz);
                            let hex = [rgb.b as u8, rgb.g as u8, rgb.r as u8]
                                .into_iter()
                                .fold(String::new(), |_, v| format!("{v:x}"));
                            println!("Device color is #{hex}");
                        }
                        Self::ColorXy { .. } => {
                            println!("Device color is x: {:.2}, y: {:.2}", xyz.x, xyz.y);
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let mut buf = [0u8; 4];

                    let scaled_x = (x * 0xFFFF as f64) as u16;
                    let scaled_y = (y * 0xFFFF as f64) as u16;

                    buf[0] = (scaled_x & 0xFF) as _;
                    buf[1] = (scaled_x >> 8) as _;
                    buf[2] = (scaled_y & 0xFF) as _;
                    buf[3] = (scaled_y >> 8) as _;

                    if !hue_bar
                        .write_gatt_char(&LIGHT_SERVICE, &COLOR, &buf)
                        .await?
                    {
                        eprintln!(
                            "[ERROR] Service or Characteristic \"{COLOR}\" for \"{LIGHT_SERVICE}\" not found for device {}", hue_bar.addr
                        );
                    }
                }
            }
        }

        if !matches!(this, Self::PairAndTrust) {
            hue_bar.disconnect().await?;
        }

        Ok(())
    }
}
