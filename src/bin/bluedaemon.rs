use std::path::Path;
use std::time::Duration;
use std::{collections::HashMap, io::Error};

use bluer::{Adapter, Address, Session};
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{
    traits::tokio::Listener as _, ListenerNonblockingMode, ListenerOptions, ToFsName,
};
use interprocess::os::unix::local_socket::FilesystemUdSocket;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    signal, time,
};

use rustbee::cli::Command;
use rustbee::constants::{flags::*, get_path, BUFFER_LEN, FAILURE, OUTPUT_LEN, SET, SUCCESS};
use rustbee::hueblue::*;
use rustbee::mask::get_commands_from_flags;

const TIMEOUT_SECS: u64 = 60 * 5;

// converts Result<T, E> into SUCCESS or FAILURE (1 or 0)
macro_rules! u {
    ($r:expr) => {
        $r.map_or_else(|_| FAILURE, |_| SUCCESS)
    };
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let path = get_path().await;

    if Path::new(path).exists() {
        eprintln!("Error: socket is already in use, an instance might already be running");
        std::process::exit(2);
    }

    let fs_name = path
        .to_fs_name::<FilesystemUdSocket>()
        .unwrap_or_else(|error| {
            eprintln!("Error cannot create filesystem path name: {path} => {error}");
            std::process::exit(1);
        });

    let socket = ListenerOptions::default()
        .name(fs_name)
        .nonblocking(ListenerNonblockingMode::Neither)
        .create_tokio();

    let listener = match socket {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Error on spawning local socket: {error}");
            std::process::exit(1);
        }
    };

    let mut devices: HashMap<[u8; 6], HueDevice> = HashMap::new();

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => break,
            timeout = time::timeout(Duration::from_secs(TIMEOUT_SECS), listener.accept()) => {
                if timeout.is_err() {
                    break;
                }

                process_conn(timeout.unwrap(), &mut devices).await;
            }
        }
    }

    std::fs::remove_file(path).unwrap();
}

/*
 * It works as follows:
 * - When setting up a new device, Pair & Trust it, connect and retrieve services to index them by UUID
 * - Respond with [SUCCESS | FAILURE, DATA if any or filled with 0u8]
 */
async fn process_conn(conn: Result<Stream, Error>, devices: &mut HashMap<[u8; 6], HueDevice>) {
    match conn {
        Ok(mut stream) => {
            let mut buf = [0; BUFFER_LEN];
            if let Err(error) = stream.read_exact(&mut buf).await {
                eprintln!("Unexpected error on reading chunks: {error}");
                return;
            }
            let mut addr = [0; 6];
            for (i, byte) in buf[..addr.len()].iter().enumerate() {
                addr[i] = *byte;
            }
            let flags = buf[6];
            let set = buf[7] == SET;
            let data = &buf[8..];

            let adapter = get_adapter().await.unwrap();
            if devices.get(&addr).is_none() {
                let device = adapter.device(Address::new(addr)).unwrap();

                devices.insert(
                    addr,
                    HueDevice {
                        addr: device.address(),
                        services: None,
                        device: Some(device),
                    },
                );
            }
            let hue_device = devices.get_mut(&addr).unwrap();
            if hue_device.services.is_none() {
                if let Err(error) = hue_device.try_pair().await {
                    eprintln!(
                        "Unexpected error trying to pair with device {}: {error}",
                        hue_device.addr
                    );
                    devices.remove(&addr).unwrap();
                    return;
                }
                if let Err(error) = hue_device.try_connect().await {
                    eprintln!(
                        "Unexpected error trying to connect with device {}: {error}",
                        hue_device.addr
                    );
                    devices.remove(&addr).unwrap();
                    return;
                }
                if let Err(error) = hue_device.set_services().await {
                    eprintln!("Unexpected error trying get GATT characteristics and services with device {}: {error}", hue_device.addr);
                    devices.remove(&addr).unwrap();
                    return;
                }
            }

            let mut success = [0; OUTPUT_LEN];
            success[0] = 3;
            let commands = get_commands_from_flags(flags);

            if (flags >> (CONNECT - 1)) & 1 == 1 {
                let value = u!(hue_device.try_connect().await);
                success[0] = u8::min(success[0], value);
            }

            for command in commands {
                let value = match command {
                    Command::PairAndTrust => u!(hue_device.try_pair().await),
                    Command::Disconnect => u!(hue_device.try_disconnect().await),
                    Command::Power { .. } => {
                        if set {
                            u!(hue_device.set_power(data[0]).await)
                        } else if let Ok(state) = hue_device.get_power().await {
                            success[1] = state as _;
                            SUCCESS
                        } else {
                            FAILURE
                        }
                    }
                    Command::Brightness { .. } => {
                        if set {
                            u!(hue_device.set_brightness(data[0]).await)
                        } else if let Ok(v) = hue_device.get_brightness().await {
                            success[1] = v as _;
                            SUCCESS
                        } else {
                            FAILURE
                        }
                    }
                    Command::ColorRgb { .. }
                    | Command::ColorHex { .. }
                    | Command::ColorXy { .. } => {
                        let buf = [data[0], data[1], data[2], data[3]];
                        if set {
                            u!(hue_device.set_color(buf).await)
                        } else if let Ok(bytes) = hue_device.get_color().await {
                            for (i, byte) in bytes.iter().enumerate() {
                                success[i + 1] = *byte;
                            }

                            SUCCESS
                        } else {
                            FAILURE
                        }
                    }
                };
                success[0] = u8::min(success[0], value);
            }

            if success[0] != 3 {
                stream.write_all(&success).await.unwrap();
                stream.flush().await.unwrap();
            }
        }
        Err(error) => eprintln!("Error on connection: {error}"),
    }
}

async fn get_adapter() -> bluer::Result<Adapter> {
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    if !adapter.is_powered().await? {
        adapter.set_powered(true).await?;
    }

    Ok(adapter)
}
