mod address;
mod cli;

use std::process;

use clap::Parser;
use rustbee_common::bluetooth::*;
use rustbee_common::logger::*;
use rustbee_common::storage::Storage;
use rustbee_common::utils::{launch_daemon, shutdown_daemon};

use address::*;
use cli::Command;

static LOGGER: Logger = Logger::new("Rustbee-CLI", true);

#[tokio::main]
async fn main() -> btleplug::Result<()> {
    let args = cli::Args::parse();
    let command: &mut Command = Box::leak(Box::new(args.command));
    let mut tasks = Vec::new();
    let mut storage = Storage::try_default()
        .unwrap_or_else(|_| Storage::new(unimplemented!("Fallback path unimplemented")));

    LOGGER.init();

    match *command {
        Command::Gui => {
            if let Err(err) = process::Command::new("rustbee-gui").spawn() {
                error!("ERROR: Couldn't launch rustbee-gui ({err})");
            }

            return Ok(());
        }
        Command::Shutdown { force } => {
            if let Err(err) = shutdown_daemon(force) {
                error!("{err}");
                std::process::exit(1);
            }

            return Ok(());
        }
        Command::Logs {
            follow,
            tail,
            purge,
        } => {
            if purge {
                LOGGER.purge();

                return Ok(());
            }

            if follow {
                LOGGER.follow().await;

                return Ok(());
            }

            LOGGER.print(tail);

            return Ok(());
        }
        _ => (),
    }

    let addresses = match &args.hex_mac_addresses {
        Some(values) => values
            .iter()
            .map(|s| parse_hex_address(s))
            .collect::<Vec<_>>(),
        None => storage.get_devices().keys().copied().collect(),
    };

    if addresses.is_empty() {
        error!("No device MAC address(es) specified nor found on local storage");
        return Ok(());
    }

    if let Err(err) = launch_daemon().await {
        error!("{err}");
        std::process::exit(1);
    }

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
