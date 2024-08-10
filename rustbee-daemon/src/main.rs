use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, io::Error};

use interprocess::local_socket::{
    tokio::Stream, traits::tokio::Listener as _, ListenerNonblockingMode, ListenerOptions, ToFsName,
};
use interprocess::os::unix::local_socket::FilesystemUdSocket;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    signal,
    time::{self, sleep},
};

use rustbee_common::bluetooth::*;

use rustbee_common::constants::{MaskT, OutputCode, BUFFER_LEN, OUTPUT_LEN, SET, SOCKET_PATH};

const TIMEOUT_SECS: u64 = 60 * 5;
const FOUND_DEVICE_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, PartialEq)]
enum Command {
    Connect,
    PairAndTrust,
    Power,
    ColorRgb,
    ColorHex,
    ColorXy,
    Brightness,
    Disconnect,
    Name,
}

/// converts Result<T, E> into SUCCESS or FAILURE (0 or 1)
macro_rules! res_to_u8 {
    ($r:expr) => {
        u8::from($r.map_or_else(|_| OutputCode::Failure, |_| OutputCode::Success))
    };
}

#[tokio::main]
async fn main() {
    check_if_path_is_writable().await;

    if Path::new(SOCKET_PATH).exists() {
        eprintln!("Error: socket is already in use, an instance might already be running");
        std::process::exit(2);
    }

    let fs_name = SOCKET_PATH
        .to_fs_name::<FilesystemUdSocket>()
        .unwrap_or_else(|error| {
            eprintln!("Error cannot create filesystem path name: {SOCKET_PATH} => {error}");
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

    let devices: Arc<Mutex<HashMap<[u8; 6], HueDevice<Server>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => break,
            timeout = time::timeout(Duration::from_secs(TIMEOUT_SECS), listener.accept()) => {
                if timeout.is_err() {
                    break;
                }

                let devices = Arc::clone(&devices);

                tokio::spawn(async move {
                    process_conn(timeout.unwrap(), devices).await;
                });
            }
        }
    }

    for (_, device) in devices.lock().await.iter() {
        let _ = device.try_disconnect().await;
    }
    std::fs::remove_file(SOCKET_PATH).unwrap();
}

/*
 * It works as follows:
 * - When setting up a new device, Pair & Trust it, connect and retrieve services to index them by UUID
 * - Respond with [SUCCESS | FAILURE, DATA if any or filled with 0u8]
 * - Multiple commands can be used at the same time like PAIR | CONNECT | POWER for example but do
 * not use multiple commands that returns data, the output could be corrupted
 */
async fn process_conn(
    conn: Result<Stream, Error>,
    devices: Arc<Mutex<HashMap<[u8; 6], HueDevice<Server>>>>,
) {
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
            let flags = ((buf[7] as u16) << 8) | buf[6] as u16;
            let set = buf[8] == SET;
            let data = &buf[9..];

            let mut success = [0; OUTPUT_LEN];
            success[0] = u8::MAX;

            let mut devices = devices.lock().await;
            if devices.get(&addr).is_none() {
                match time::timeout(
                    Duration::from_secs(FOUND_DEVICE_TIMEOUT_SECS),
                    get_device(addr),
                )
                .await
                {
                    Err(_) => {
                        // Timed out
                        eprintln!("[WARN] Timeout during device discovery, address: {addr:?}");
                        return_error(&mut stream, OutputCode::DeviceNotFound).await;
                        return;
                    }
                    Ok(value) => {
                        let myb_device = match value {
                            Ok(myb_device) => myb_device,
                            Err(err) => {
                                eprintln!("[ERROR] Cannot get device, address: {addr:?} {err:?}");
                                return_error(&mut stream, OutputCode::Failure).await;
                                return;
                            }
                        };

                        println!("after get_device {myb_device:?}");

                        let Some(device) = myb_device else {
                            eprintln!("[WARN] Timeout during device discovery, address: {addr:?}");
                            return_error(&mut stream, OutputCode::DeviceNotFound).await;
                            return;
                        };

                        devices.insert(addr, device);
                    }
                }
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

            // Since we're not mutating the device internally, only the hashmap, we can clone and
            // free the lock
            let hue_device = hue_device.clone();
            drop(devices);

            let mut commands = get_commands_from_flags(flags);

            if commands.contains(&Command::Connect) {
                let value = res_to_u8!(hue_device.try_connect().await);
                success[0] = u8::min(success[0], value);
                commands.retain(|cmd| *cmd != Command::Connect);
            }

            for command in commands {
                let value = match command {
                    Command::Connect => continue,
                    Command::PairAndTrust => res_to_u8!(hue_device.try_pair().await),
                    Command::Disconnect => res_to_u8!(hue_device.try_disconnect().await),
                    Command::Power { .. } => {
                        if set {
                            res_to_u8!(hue_device.set_power(data[0]).await)
                        } else if let Ok(state) = hue_device.get_power().await {
                            success[1] = state as _;
                            OutputCode::Success.into()
                        } else {
                            OutputCode::Failure.into()
                        }
                    }
                    Command::Brightness { .. } => {
                        if set {
                            res_to_u8!(hue_device.set_brightness(data[0]).await)
                        } else if let Ok(v) = hue_device.get_brightness().await {
                            success[1] = v as _;
                            OutputCode::Success.into()
                        } else {
                            OutputCode::Failure.into()
                        }
                    }
                    Command::ColorRgb { .. }
                    | Command::ColorHex { .. }
                    | Command::ColorXy { .. } => {
                        let mut buf = [0u8; 4];
                        buf.copy_from_slice(&data[..4]);

                        if set {
                            res_to_u8!(hue_device.set_color(buf).await)
                        } else if let Ok(bytes) = hue_device.get_color().await {
                            for (i, byte) in bytes.iter().enumerate() {
                                success[i + 1] = *byte;
                            }

                            OutputCode::Success.into()
                        } else {
                            OutputCode::Failure.into()
                        }
                    }
                    Command::Name => {
                        let res = hue_device.get_name().await;

                        if let Ok(ref opt) = res {
                            if let Some(name_str) = opt.as_ref() {
                                let len = name_str.len();
                                for (i, byte) in name_str.bytes().take(OUTPUT_LEN - 1).enumerate() {
                                    success[i + 1] = byte;
                                }
                                if len > (OUTPUT_LEN - 1) {
                                    success[OUTPUT_LEN - 3] = b'.';
                                    success[OUTPUT_LEN - 2] = b'.';
                                    success[OUTPUT_LEN - 1] = b'.';
                                }
                            }
                        }

                        res_to_u8!(res)
                    }
                };
                success[0] = u8::min(success[0], value);

                // https://developers.meethue.com/develop/get-started-2/core-concepts/#limitations
                sleep(Duration::from_millis(100)).await;
            }

            if success[0] != u8::MAX {
                stream.write_all(&success).await.unwrap();
                stream.flush().await.unwrap();
            }
        }
        Err(error) => eprintln!("Error on connection: {error}"),
    }
}

async fn return_error(stream: &mut Stream, output_code: OutputCode) {
    let mut buf = [0; OUTPUT_LEN];
    buf[0] = output_code.into();
    stream.write_all(&buf).await.unwrap();
    stream.flush().await.unwrap();
}

async fn check_if_path_is_writable() {
    if fs::read_dir("/var/run").await.is_err() {
        eprintln!("Cannot find /var/run directory or lacking permissions to read it");
        std::process::exit(2);
    }

    if fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open("/var/run/x")
        .await
        .is_err()
    {
        eprintln!("Lacking permissions to write to /var/run directory");
        std::process::exit(2);
    }

    let _ = fs::remove_file("/var/run/x").await;
}

fn get_commands_from_flags(flags: MaskT) -> Vec<Command> {
    use rustbee_common::constants::flags::*;

    let mut v = Vec::new();

    // Could also do flags & CONNECT == CONNECT where connect is the mask
    if (flags >> (CONNECT - 1)) & 1 == 1 {
        v.push(Command::Connect);
    }
    if (flags >> (PAIR - 1)) & 1 == 1 {
        v.push(Command::PairAndTrust);
    }
    if (flags >> (POWER - 1)) & 1 == 1 {
        v.push(Command::Power)
    }
    if (flags >> (COLOR_RGB - 1)) & 1 == 1 {
        v.push(Command::ColorRgb)
    }
    if (flags >> (COLOR_HEX - 1)) & 1 == 1 {
        v.push(Command::ColorHex)
    }
    if (flags >> (COLOR_XY - 1)) & 1 == 1 {
        v.push(Command::ColorXy)
    }
    if (flags >> (BRIGHTNESS - 1)) & 1 == 1 {
        v.push(Command::Brightness)
    }
    if (flags >> (DISCONNECT - 1)) & 1 == 1 {
        v.push(Command::Disconnect)
    }
    if (flags >> (NAME - 1)) & 1 == 1 {
        v.push(Command::Name)
    }

    v
}
