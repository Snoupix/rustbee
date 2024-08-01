mod cli;
mod colors;
mod constants;
mod hueblue;
mod mask;

use clap::Parser;

use cli::Command;
use constants::*;
use hueblue::*;

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let args = cli::Args::parse();
    let command: &mut Command = Box::leak(Box::new(args.command));
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
        tasks.push(tokio::spawn(command.handle(hue_bar)));
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

    assert!(
        bytes.len() == 6,
        "[ERROR] Hex address {address} is not right. It must be of length 6 => xx:xx:xx:xx:xx:xx"
    );

    for (i, byte) in bytes.into_iter().enumerate() {
        addr[i] = byte;
    }

    addr
}
