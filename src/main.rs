mod cli;

use clap::Parser;
use rustbee_common::bluetooth::*;
use rustbee_common::constants::*;

use cli::Command;

#[tokio::main]
async fn main() -> btleplug::Result<()> {
    let args = cli::Args::parse();
    let command: &mut Command = Box::leak(Box::new(args.command));
    let mut tasks = Vec::new();

    // Returns Result<Vec<HueDevice<Client>>> infered because the Command::handle fn requires a
    // Client variant so the turbofish would be useless
    let hue_devices = get_devices(&match &args.hex_mac_addresses {
        Some(values) => values
            .clone()
            .into_iter()
            .map(parse_hex_address)
            .collect::<Vec<_>>(),
        None => Vec::from([HUE_BAR_1_ADDR, HUE_BAR_2_ADDR]),
    })
    .await?;

    for hue_device in hue_devices {
        tasks.push(tokio::spawn(command.handle(hue_device)));
    }

    for task in tasks {
        task.await.expect("Failed to spawn async tokio task")?;
    }

    Ok(())
}

fn parse_hex_address(address: String) -> [u8; ADDR_LEN] {
    let mut addr = [0; ADDR_LEN];
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
        bytes.len() == ADDR_LEN,
        "[ERROR] Hex address {address} is not right. It must be of length {ADDR_LEN} => xx:xx:xx:xx:xx:xx"
    );

    for (i, byte) in bytes.into_iter().enumerate() {
        addr[i] = byte;
    }

    addr
}
