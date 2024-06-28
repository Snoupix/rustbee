use std::io::Error;
use std::path::Path;
use std::time::Duration;

use bluer::{Adapter, Address, Device, Session};
use interprocess::{
    local_socket::{
        tokio::Stream, traits::tokio::Listener as _, ListenerNonblockingMode, ListenerOptions,
        ToFsName,
    },
    os::unix::local_socket::FilesystemUdSocket,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    signal,
    time::{self, sleep},
};

include!("../../shared.rs");

const TIMEOUT_SECS: u64 = 60 * 5;

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

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => break,
            timeout = time::timeout(Duration::from_secs(TIMEOUT_SECS), listener.accept()) => {
                if timeout.is_err() {
                    break;
                }

                process_conn(timeout.unwrap()).await;
            }
        }
    }

    std::fs::remove_file(path).unwrap();
}

async fn process_conn(conn: Result<Stream, Error>) {
    match conn {
        Ok(mut stream) => {
            let mut buf = [0; 6 + 1]; // 6bytes BLE id length + 1 for the flags
            if let Err(error) = stream.read_exact(&mut buf).await {
                eprintln!("Unexpected error on reading chunks: {error}");
                return;
            }
            let mut addr = [0; 6];
            for (i, byte) in buf[..buf.len() - 1].iter().enumerate() {
                addr[i] = *byte;
            }
            let flags = buf[buf.len() - 1];

            let adapter = get_adapter().await.unwrap();
            let device = adapter.device(Address::new(addr)).unwrap();

            if flags & CONNECT == 1 {
                let success = [match connect(device).await {
                    Ok(_) => SUCCESS,
                    Err(_) => 0,
                }];
                stream.write_all(&success).await.unwrap();
                stream.flush().await.unwrap();
            } else if (flags >> (DISCONNECT - 1)) == 1 {
                let success = [match disconnect(device).await {
                    Ok(_) => SUCCESS,
                    Err(_) => 0,
                }];
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

async fn connect(device: Device) -> bluer::Result<()> {
    let mut retries = 3;
    loop {
        if device.is_connected().await? {
            break;
        }

        if retries <= 0 {
            eprintln!(
                "[ERROR] Failed to connect to {} after 3 attempts",
                device.address()
            );
            return Err(bluer::Error {
                kind: bluer::ErrorKind::Failed,
                message: "Faileed to disconnect after 3 attempts".into(),
            });
        }

        if let Err(error) = device.connect().await {
            eprintln!(
                "[ERROR] Connecting to device {:?} failed: {error}",
                device.address().0
            );
        }

        retries -= 1;
    }
    sleep(Duration::from_millis(150)).await;

    Ok(())
}

async fn disconnect(device: Device) -> bluer::Result<()> {
    let mut retries = 3;
    loop {
        if !device.is_connected().await? {
            break;
        }

        if retries <= 0 {
            eprintln!(
                "[ERROR] Failed to disconnect from {} after 3 attempts",
                device.address()
            );
            return Err(bluer::Error {
                kind: bluer::ErrorKind::Failed,
                message: "Faileed to disconnect after 3 attempts".into(),
            });
        }

        if let Err(error) = device.disconnect().await {
            eprintln!(
                "[ERROR] Disconnecting from device {:?} failed: {error}",
                device.address().0
            );
        }

        retries -= 1;
    }
    sleep(Duration::from_millis(150)).await;

    Ok(())
}
