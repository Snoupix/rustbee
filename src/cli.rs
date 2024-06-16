use crate::*;

use clap::{Parser, Subcommand};

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
        x: Option<i16>,
        y: Option<i16>,
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

        match this {
            Self::PairAndTrust => {
                hue_bar.pair().await?;
                hue_bar.set_trusted(true).await?;
                println!("Done !");
            }
            Self::Power { ref state } => {
                if let Some(state) = state {
                    if !hue_bar
                        .set_power_state(LIGHT_SERVICE, POWER, matches!(*state, State::On))
                        .await?
                    {
                        println!(
                            "[ERROR] Failed to write power state to hue bar address: {}",
                            hue_bar.addr
                        );
                    }
                } else {
                    println!(
                        "Device is {}",
                        if hue_bar
                            .get_power_state(POWER)
                            .await?
                            .expect("Cannot get power state")
                        {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                }
            }
            Self::ColorRgb { r, g, b } => todo!(),
            Self::ColorHex { hex } => todo!(),
            Self::ColorXy { x, y } => {
                if x.is_none() || y.is_none() {
                    let buf = hue_bar
                        .read_gatt_char(LIGHT_SERVICE, COLOR)
                        .await?
                        .expect("Cannot get xy colors");

                    println!(
                        "Device color is x: {}, y: {}",
                        u16::from_le_bytes([buf[0], buf[1]]) / 0xFFFF,
                        u16::from_le_bytes([buf[2], buf[3]]) / 0xFFFF,
                    );
                }
            }
            Self::Brightness { value } => match value {
                Some(value) => {
                    if !hue_bar
                        .write_gatt_char(LIGHT_SERVICE, BRIGHTNESS, &[value]) // TODO: Fix, it must
                        // be converted to a
                        // 2 bytes value
                        .await?
                    {
                        panic!("[ERROR] Cannot get characteristic for brightness setter")
                    }

                    println!("Done !",);
                }
                None => {
                    let brightness = (*hue_bar
                        .read_gatt_char(LIGHT_SERVICE, BRIGHTNESS)
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
        }

        if !matches!(this, Self::PairAndTrust) {
            hue_bar.disconnect().await?;
        }

        Ok(())
    }
}
