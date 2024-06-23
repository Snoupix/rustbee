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
                let mut retries = 2;
                let mut error = None;
                while !hue_bar.is_paired().await? {
                    if retries <= 0 {
                        panic!(
                            "[ERROR] Failed to pair device {} after 2 attempts {:?}",
                            hue_bar.addr, error
                        );
                    }
                    error = match hue_bar.pair().await {
                        Ok(_) => break,
                        Err(err) => Some(err),
                    };
                    retries -= 1;
                }

                retries = 2;
                error = None;
                while !hue_bar.is_trusted().await? {
                    if retries <= 0 {
                        panic!(
                            "[ERROR] Failed to \"trust\" device {} after 2 attempts {:?}",
                            hue_bar.addr, error
                        );
                    }
                    error = match hue_bar.set_trusted(true).await {
                        Ok(_) => break,
                        Err(err) => Some(err),
                    };
                    retries -= 1;
                }
            }
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
                            "[ERROR] Characteristic \"{POWER}\" for \"{LIGHT_SERVICE}\" not found for device {}", hue_bar.addr
                        );
                    }
                }
            }
            Self::ColorRgb { r, g, b } => todo!(),
            Self::ColorHex { hex } => todo!(),
            Self::ColorXy { x, y } => {
                if x.is_none() || y.is_none() {
                    let buf = hue_bar
                        .read_gatt_char(&LIGHT_SERVICE, &COLOR)
                        .await?
                        .expect("Cannot get xy colors");

                    eprintln!(
                        "Device color is x: {}, y: {}",
                        u16::from_le_bytes([buf[0], buf[1]]) / 0xFFFF,
                        u16::from_le_bytes([buf[2], buf[3]]) / 0xFFFF,
                    );
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
                            "[ERROR] Characteristic \"{POWER}\" for \"{LIGHT_SERVICE}\" not found for device {}", hue_bar.addr
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
        }

        if !matches!(this, Self::PairAndTrust) {
            hue_bar.disconnect().await?;
        }

        Ok(())
    }
}
