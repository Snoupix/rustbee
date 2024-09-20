use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, io::Error};

use futures::stream::StreamExt as _;
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
use rustbee_common::constants::{
    MaskT, OutputCode, ADDR_LEN, BUFFER_LEN, OUTPUT_LEN, SET, SOCKET_PATH,
};

const TIMEOUT_SECS: u64 = 60 * 2;
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
    SearchName,
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

    let devices: Arc<Mutex<HashMap<[u8; ADDR_LEN], HueDevice<Server>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => break,
            timeout = time::timeout(Duration::from_secs(TIMEOUT_SECS), listener.accept()) => {
                let Ok(conn) = timeout else {
                    // Timed out
                    break;
                };

                tokio::spawn(process_conn(conn, Arc::clone(&devices)));
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
    devices: Arc<Mutex<HashMap<[u8; ADDR_LEN], HueDevice<Server>>>>,
) {
    match conn {
        Ok(mut stream) => {
            let mut buf = [0; BUFFER_LEN];
            if let Err(error) = stream.read_exact(&mut buf).await {
                eprintln!("Unexpected error on reading chunks: {error}");
                return;
            }
            let mut addr = [0; ADDR_LEN];
            for (i, byte) in buf[..addr.len()].iter().enumerate() {
                addr[i] = *byte;
            }
            let flags = ((buf[7] as u16) << 8) | buf[6] as u16;
            let set = buf[8] == SET;
            let data = &buf[9..];

            let mut output_buf = [0; OUTPUT_LEN];
            output_buf[0] = u8::MAX;

            let mut commands = get_commands_from_flags(flags);

            // println!("{buf:?}");
            // println!(
            //     "addr: {:?} flags: {} set {} data: {:?}",
            //     addr, flags, set, data
            // );
            // println!("{addr:?} {commands:?}");

            // Commands that are executed alone and only alone without the need to fetch the device
            if commands.contains(&Command::SearchName) {
                let name =
                    String::from_utf8(data.iter().copied().filter(|c| *c != b'\0').collect())
                        .unwrap();
                let mut stream_iter = search_devices_by_name::<Server>(&name, 10).await.unwrap();
                let mut device_sent = 0;

                while let Some(device) = stream_iter.next().await {
                    let mut buf = [0; OUTPUT_LEN];
                    buf[0] = OutputCode::Streaming.into();

                    let addr = *device.addr;
                    for (i, byte) in addr.iter().enumerate() {
                        buf[i + 1] = *byte;
                    }

                    for (i, byte) in device
                        .name()
                        .await
                        .map_err(|_| Some(String::new()))
                        .unwrap()
                        .or_else(|| Some(String::new()))
                        .unwrap()
                        .as_bytes()
                        .iter()
                        .enumerate()
                    {
                        let offset = addr.len() + 1 + i;
                        if offset >= buf.len() {
                            break;
                        }

                        buf[offset] = *byte;
                    }

                    send_to_stream(&mut stream, buf).await;
                    device_sent += 1;
                }

                if device_sent == 0 {
                    send_output_code(&mut stream, OutputCode::DeviceNotFound).await;
                    return;
                }

                send_output_code(&mut stream, OutputCode::StreamEOF).await;
                return;
            }

            let mut devices = devices.lock().await;
            if devices.get(&addr).is_none() {
                match time::timeout(
                    Duration::from_secs(FOUND_DEVICE_TIMEOUT_SECS),
                    get_device(addr),
                )
                .await
                {
                    Err(elapsed) => {
                        // Timed out
                        eprintln!(
                            "[WARN] Timeout: {elapsed} during device discovery, address: {addr:?}"
                        );
                        send_output_code(&mut stream, OutputCode::DeviceNotFound).await;
                        return;
                    }
                    Ok(value) => {
                        let myb_device = match value {
                            Ok(myb_device) => myb_device,
                            Err(err) => {
                                eprintln!("[ERROR] Cannot get device, address: {addr:?} {err:?}");
                                send_output_code(&mut stream, OutputCode::Failure).await;
                                return;
                            }
                        };

                        let Some(device) = myb_device else {
                            eprintln!("[WARN] Device not found or not in range, address: {addr:?}");
                            send_output_code(&mut stream, OutputCode::DeviceNotFound).await;
                            return;
                        };

                        devices.insert(addr, device);
                    }
                }
            }

            let hue_device = devices.get_mut(&addr).unwrap();

            // If we only need to get connect status, avoid connecting to set services
            if commands.len() == 1 && commands[0] == Command::Connect && !set {
                if let Ok(state) = hue_device.is_device_connected().await {
                    output_buf[0] = OutputCode::Success.into();
                    output_buf[1] = state as _;
                } else {
                    output_buf[0] = OutputCode::Failure.into();
                }

                send_to_stream(&mut stream, output_buf).await;
                return;
            }

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

            // Since we're not mutating the device internally, only the hashmap, we can clone the
            // device and free the lock
            let hue_device = hue_device.clone();
            drop(devices);

            // Priority command
            if commands.contains(&Command::Connect) {
                let value = res_to_u8!(hue_device.try_connect().await);
                output_buf[0] = u8::min(output_buf[0], value);
                commands.retain(|cmd| *cmd != Command::Connect);
            }

            for command in commands {
                let value = match command {
                    Command::Connect | Command::SearchName => continue,
                    Command::PairAndTrust => res_to_u8!(hue_device.try_pair().await),
                    Command::Disconnect => res_to_u8!(hue_device.try_disconnect().await),
                    Command::Power { .. } => {
                        if set {
                            res_to_u8!(hue_device.set_power(data[0]).await)
                        } else if let Ok(state) = hue_device.get_power().await {
                            output_buf[1] = state as _;
                            OutputCode::Success.into()
                        } else {
                            OutputCode::Failure.into()
                        }
                    }
                    Command::Brightness { .. } => {
                        if set {
                            res_to_u8!(hue_device.set_brightness(data[0]).await)
                        } else if let Ok(v) = hue_device.get_brightness().await {
                            output_buf[1] = v as _;
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
                                output_buf[i + 1] = *byte;
                            }

                            OutputCode::Success.into()
                        } else {
                            OutputCode::Failure.into()
                        }
                    }
                    Command::Name => {
                        let res = hue_device.get_name().await;

                        if let Ok(Some(ref name_str)) = res {
                            let len = name_str.len();
                            for (i, byte) in name_str.bytes().take(OUTPUT_LEN - 1).enumerate() {
                                output_buf[i + 1] = byte;
                            }
                            if len > (OUTPUT_LEN - 1) {
                                output_buf[OUTPUT_LEN - 3] = b'.';
                                output_buf[OUTPUT_LEN - 2] = b'.';
                                output_buf[OUTPUT_LEN - 1] = b'.';
                            }
                        }

                        res_to_u8!(res)
                    }
                };
                output_buf[0] = u8::min(output_buf[0], value);

                // https://developers.meethue.com/develop/get-started-2/core-concepts/#limitations
                sleep(Duration::from_millis(100)).await;
            }

            if output_buf[0] != u8::MAX {
                send_to_stream(&mut stream, output_buf).await;
            }
        }
        Err(error) => eprintln!("Error on connection: {error}"),
    }
}

async fn send_to_stream(stream: &mut Stream, buf: [u8; OUTPUT_LEN]) {
    stream.write_all(&buf).await.unwrap();
    stream.flush().await.unwrap();
}

async fn send_output_code(stream: &mut Stream, output_code: OutputCode) {
    let mut buf = [0; OUTPUT_LEN];
    buf[0] = output_code.into();
    send_to_stream(stream, buf).await;
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
    if (flags >> (SEARCH_NAME - 1)) & 1 == 1 {
        v.push(Command::SearchName)
    }

    v
}
