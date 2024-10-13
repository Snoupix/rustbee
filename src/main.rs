mod address;
mod cli;

use std::process;

use clap::Parser;
use rustbee_common::bluetooth::*;
use rustbee_common::storage::Storage;
use rustbee_common::utils::{launch_daemon, shutdown_daemon};

use address::*;
use cli::Command;

#[tokio::main]
async fn main() -> btleplug::Result<()> {
    let args = cli::Args::parse();
    let command: &mut Command = Box::leak(Box::new(args.command));
    let mut tasks = Vec::new();
    let mut storage = Storage::try_default()
        .unwrap_or_else(|_| Storage::new(unimplemented!("Fallback path unimplemented")));

    match *command {
        Command::Gui => {
            if let Err(err) = process::Command::new("rustbee-gui").output() {
                eprintln!("ERROR: Couldn't launch rustbee-gui ({err})");
            }

            return Ok(());
        }
        Command::Shutdown { force } => {
            if let Err(err) = shutdown_daemon(force) {
                eprintln!("{err}");
                std::process::exit(1);
            }

            return Ok(());
        }
        Command::Logs => {}
        _ => (),
    }

    if let Err(err) = launch_daemon().await {
        eprintln!("{err}");
        std::process::exit(1);
    }

    let addresses = match &args.hex_mac_addresses {
        Some(values) => values
            .iter()
            .map(|s| parse_hex_address(s))
            .collect::<Vec<_>>(),
        None => storage
            .get_devices()
            .iter()
            .map(|(addr, _)| *addr)
            .collect(),
    };
    // Returns Result<Vec<HueDevice<Client>>> infered because the Command::handle fn requires a
    // Client variant so the turbofish would be useless
    let hue_devices = get_devices(&addresses).await?;

    for hue_device in hue_devices {
        tasks.push(tokio::spawn(command.handle(hue_device)));
    }

    for task in tasks {
        task.await.expect("Failed to spawn async tokio task")?;
    }

    if args.save {
        save_addresses(&mut storage, &addresses);
    }

    if args.one_shot {
        return shutdown_daemon(false).map_err(|err| btleplug::Error::Other(Box::new(err)));
    }

    Ok(())
}
