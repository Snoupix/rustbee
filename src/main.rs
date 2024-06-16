mod cli;
mod hueblue;

use clap::Parser;

use cli::Command;
use hueblue::*;

use uuid::{uuid, Uuid};

const HUE_BAR_1_ADDR: [u8; 6] = [0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00];
const HUE_BAR_2_ADDR: [u8; 6] = [0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C];

// Thanks to https://gist.github.com/shinyquagsire23/f7907fdf6b470200702e75a30135caf3 for the UUIDs
const LIGHT_SERVICE: Uuid = uuid!("932c32bd-0000-47a2-835a-a8d455b859dd");
const POWER: Uuid = uuid!("932c32bd-0002-47a2-835a-a8d455b859dd");
const BRIGHTNESS: Uuid = uuid!("932c32bd-0003-47a2-835a-a8d455b859dd");
const COLOR: Uuid = uuid!("932c32bd-0005-47a2-835a-a8d455b859dd");

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let args = cli::Args::parse();
    let mut tasks = Vec::new();

    let hue_bars = get_devices(&match &args.hex_mac_addresses {
        Some(values) => values
            .clone()
            .into_iter()
            .map(parse_hex_address)
            .collect::<Vec<_>>(),
        None => Vec::from([HUE_BAR_1_ADDR, HUE_BAR_2_ADDR]),
    })
    .await?;

    for hue_bar in hue_bars {
        tasks.push(tokio::spawn(Command::job(args.command.clone(), hue_bar)));
    }

    for task in tasks {
        task.await??;
    }

    Ok(())
}

fn parse_hex_address(address: String) -> [u8; 6] {
    let mut addr = [0; 6];
    let chars = address.chars().filter(|c| *c != ':');
    let bytes = chars
        .clone()
        .step_by(2)
        .zip(chars.skip(1).step_by(2))
        .map(|(a, b)| {
            u8::from_str_radix(&format!("{a}{b}"), 16)
                .map_err(|e| {
                    panic!("[ERROR] Cannot parse {address} to hex value, try xx:xx:xx... {e}")
                })
                .unwrap()
        })
        .collect::<Vec<_>>();
    if bytes.len() != 6 {
        panic!("[ERROR] Hex address {address} is not right. It must be of length 6 => xx:xx:xx:xx:xx:xx");
    }

    for (i, byte) in bytes.into_iter().enumerate() {
        addr[i] = byte;
    }

    addr
}
