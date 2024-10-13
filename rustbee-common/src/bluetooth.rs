use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use btleplug::api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures::{future, stream, StreamExt};
use interprocess::{
    local_socket::{tokio::Stream as TokioStream, traits::tokio::Stream as _, ToFsName as _},
    os::unix::local_socket::FilesystemUdSocket,
};
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    time::{self, sleep},
};
use uuid::Uuid;

#[cfg(feature = "ffi")]
use interprocess::local_socket::{traits::Stream as _, Stream as SyncStream};

use crate::constants::{masks::*, *};

pub const EMPTY_BUFFER: [u8; DATA_LEN + 1] = [0; DATA_LEN + 1];
const ATTEMPTS: u8 = 3;

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Default, Hash)]
pub struct FoundDevice {
    pub address: [u8; ADDR_LEN],
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct Client;
#[derive(Clone, Debug, Default)]
pub struct Server;
#[cfg(feature = "ffi")]
#[derive(Clone, Debug, Default)]
pub struct FFI;

#[derive(Clone, Debug)]
pub struct HueDevice<Type> {
    pub addr: BDAddr,
    pub device: Option<Peripheral>,
    _type: PhantomData<Type>,
}

impl Default for HueDevice<Server> {
    fn default() -> Self {
        Self {
            addr: Default::default(),
            device: Default::default(),
            _type: Default::default(),
        }
    }
}
impl Default for HueDevice<Client> {
    fn default() -> Self {
        Self {
            addr: Default::default(),
            device: Default::default(),
            _type: Default::default(),
        }
    }
}
#[cfg(feature = "ffi")]
impl Default for HueDevice<FFI> {
    fn default() -> Self {
        Self {
            addr: Default::default(),
            device: Default::default(),
            _type: Default::default(),
        }
    }
}

// The client doesn't use the bluetooth struct so only the server needs to deref since the client
// device field should always be None
impl Deref for HueDevice<Server> {
    type Target = Peripheral;

    /// Be sure to use it wisely since it NEEDS to have the device set
    fn deref(&self) -> &Self::Target {
        self.device.as_ref().unwrap()
    }
}

impl<T> HueDevice<T>
where
    HueDevice<T>: Default + std::fmt::Debug,
{
    pub fn new(addr: BDAddr) -> Self {
        Self {
            addr,
            ..Default::default()
        }
    }

    pub fn new_with_device(addr: BDAddr, device: Peripheral) -> Self {
        Self {
            addr,
            device: Some(device),
            ..Default::default()
        }
    }

    pub fn set_device(&mut self, device: Peripheral) {
        self.device = Some(device);
    }

    pub fn unset_device(&mut self) {
        self.device = None;
    }
}

impl HueDevice<Server>
where
    HueDevice<Server>: Default + Deref<Target = Peripheral> + std::fmt::Debug,
{
    pub async fn read_gatt_char(
        &self,
        service: &Uuid,
        charac: &Uuid,
    ) -> btleplug::Result<Option<Vec<u8>>> {
        if let Some(service) = self.services().iter().find(|&s| &s.uuid == service) {
            if let Some(charac) = service.characteristics.iter().find(|&c| &c.uuid == charac) {
                return Ok(Some(self.read(charac).await?));
            }
        }

        Ok(None)
    }

    pub async fn write_gatt_char(
        &self,
        service: &Uuid,
        charac: &Uuid,
        bytes: &[u8],
    ) -> btleplug::Result<bool> {
        if let Some(service) = self.services().iter().find(|&s| &s.uuid == service) {
            if let Some(charac) = service.characteristics.iter().find(|&c| &c.uuid == charac) {
                self.write(charac, bytes, WriteType::WithoutResponse)
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn try_connect(&self) -> btleplug::Result<()> {
        let mut retries = ATTEMPTS;
        loop {
            if self.is_connected().await? {
                break;
            }

            if retries == 0 {
                eprintln!(
                    "[ERROR] Failed to connect to {} after {ATTEMPTS} attempts",
                    self.addr
                );
                return Err(btleplug::Error::Other(Box::new(Error(format!(
                    "Failed to connect after {ATTEMPTS} attempts"
                )))));
            }

            if let Err(error) = self.connect().await {
                eprintln!("[WARN] Connecting to device {} failed: {error}", self.addr);
            }

            retries -= 1;
        }
        sleep(Duration::from_millis(150)).await;

        Ok(())
    }

    pub async fn try_disconnect(&self) -> btleplug::Result<()> {
        let mut retries = ATTEMPTS;
        loop {
            if !self.is_connected().await? {
                break;
            }

            if retries == 0 {
                eprintln!(
                    "[ERROR] Failed to disconnect from {} after {ATTEMPTS} attempts",
                    self.addr
                );
                return Err(btleplug::Error::Other(Box::new(Error(format!(
                    "Failed to disconnect after {ATTEMPTS} attempts"
                )))));
            }

            if let Err(error) = self.disconnect().await {
                eprintln!(
                    "[WARN] Disconnecting from device {} failed: {error}",
                    self.addr
                );
            }

            retries -= 1;
        }

        Ok(())
    }

    pub async fn is_device_connected(&self) -> btleplug::Result<bool> {
        (*self).is_connected().await
    }

    pub async fn get_power(&self) -> btleplug::Result<bool> {
        let read = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &POWER_UUID)
            .await?;
        if let Some(bytes) = read {
            Ok(*bytes.first().unwrap() == true as u8)
        } else {
            Err(btleplug::Error::Other(Box::new(Error (
                format!("[ERROR] Service or Characteristic \"{POWER_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
            ))))
        }
    }

    pub async fn set_power(&self, value: u8) -> btleplug::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &POWER_UUID, &[value])
            .await?;

        Ok(())
    }

    pub async fn get_brightness(&self) -> btleplug::Result<f32> {
        let read = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &BRIGHTNESS_UUID)
            .await?;
        if let Some(bytes) = read {
            Ok(*bytes.first().unwrap() as f32)
        } else {
            Err(btleplug::Error::Other(Box::new(Error(
                format!("[ERROR] Service or Characteristic \"{BRIGHTNESS_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
            ))))
        }
    }

    pub async fn set_brightness(&self, value: u8) -> btleplug::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &BRIGHTNESS_UUID, &[value])
            .await?;

        Ok(())
    }

    pub async fn get_color(&self) -> btleplug::Result<[u8; 4]> {
        let mut buf = [0u8; 4];
        if let Some(bytes) = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &COLOR_UUID)
            .await?
        {
            let len = buf.len();
            buf.copy_from_slice(&bytes[..len]);

            Ok(buf)
        } else {
            Err(btleplug::Error::Other(Box::new(Error(
                format!("[ERROR] Service or Characteristic \"{COLOR_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
            ))))
        }
    }

    pub async fn set_color(&self, buf: [u8; 4]) -> btleplug::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &COLOR_UUID, &buf)
            .await?;

        Ok(())
    }

    pub async fn get_name(&self) -> btleplug::Result<Option<String>> {
        Ok(self
            .properties()
            .await?
            .map(|properties| properties.local_name)
            .unwrap_or(None))
    }
}

pub type CmdOutput = (OutputCode, [u8; OUTPUT_LEN - 1]);

impl HueDevice<Client>
where
    HueDevice<Client>: Default + std::fmt::Debug,
{
    pub async fn set_power(&self, state: bool) -> OutputCode {
        let mut buf = EMPTY_BUFFER;
        buf[0] = SET;
        buf[1] = state as _;

        self.send_packet_to_daemon(CONNECT | POWER, buf).await.0
    }

    pub async fn get_power(&self) -> CmdOutput {
        self.send_packet_to_daemon(CONNECT | POWER, EMPTY_BUFFER)
            .await
    }

    pub async fn set_brightness(&self, value: u8) -> OutputCode {
        let mut buf = EMPTY_BUFFER;
        buf[0] = SET;
        buf[1] = (((value as f32) / 100.) * 0xff as f32) as _;

        self.send_packet_to_daemon(CONNECT | BRIGHTNESS, buf)
            .await
            .0
    }

    pub async fn get_brightness(&self) -> CmdOutput {
        self.send_packet_to_daemon(CONNECT | BRIGHTNESS, EMPTY_BUFFER)
            .await
    }

    pub async fn get_colors(&self, color_mask: MaskT) -> CmdOutput {
        assert!([COLOR_XY, COLOR_RGB, COLOR_HEX].contains(&color_mask));

        self.send_packet_to_daemon(CONNECT | color_mask, EMPTY_BUFFER)
            .await
    }

    pub async fn set_colors(&self, scaled_x: u16, scaled_y: u16, color_mask: MaskT) -> OutputCode {
        assert!([COLOR_XY, COLOR_RGB, COLOR_HEX].contains(&color_mask));

        let mut buf = EMPTY_BUFFER;
        buf[0] = SET;
        buf[1] = (scaled_x & 0xFF) as _;
        buf[2] = (scaled_x >> 8) as _;
        buf[3] = (scaled_y & 0xFF) as _;
        buf[4] = (scaled_y >> 8) as _;

        println!("{scaled_x} {scaled_y} {buf:?}");

        self.send_packet_to_daemon(CONNECT | color_mask, buf)
            .await
            .0
    }

    pub async fn get_name(&self) -> CmdOutput {
        self.send_packet_to_daemon(NAME, EMPTY_BUFFER).await
    }

    pub async fn is_connected(&self) -> CmdOutput {
        self.send_packet_to_daemon(CONNECT, EMPTY_BUFFER).await
    }

    pub async fn search_by_name(
        name: &String,
    ) -> Pin<Box<dyn stream::Stream<Item = FoundDevice> + Send>> {
        let mut buf = EMPTY_BUFFER;
        let bytes = name.as_bytes();
        let len = usize::min(bytes.len(), buf.len());

        // 1 for set/get byte offset
        buf[1..len + 1].copy_from_slice(&bytes[..len]);

        let get_found_device = |device_buf: [u8; OUTPUT_LEN - 1]| {
            let mut address = [0; ADDR_LEN];
            let len = address.len();
            address.copy_from_slice(&device_buf[..len]);

            let idx = device_buf[len..]
                .iter()
                .position(|b| *b == b'\0')
                .unwrap_or(device_buf[len..].len())
                + len; // since I'm getting the index of the sub_slice [len..] I need to add the
                       // offset len to have the exact index of the slice

            FoundDevice {
                address,
                name: String::from_utf8(device_buf[len..idx].to_vec()).unwrap(),
            }
        };

        let stream = Arc::new(Mutex::new(Self::get_file_socket().await));

        let stream_iter = stream::unfold(
            Some((Arc::clone(&stream), false)),
            move |state| async move {
                let (stream_guard_ref, is_stream_initiated) = state?;
                let mut stream_guard = stream_guard_ref.lock().await;

                if !is_stream_initiated {
                    let (code, device_buf) =
                        Self::_send_packet_to_daemon(&mut stream_guard, None, SEARCH_NAME, buf)
                            .await;

                    if code != OutputCode::Streaming {
                        return None;
                    }

                    drop(stream_guard);

                    return Some((get_found_device(device_buf), Some((stream_guard_ref, true))));
                }

                let (code, device_buf) = Self::receive_packet_from_daemon(&mut stream_guard).await;

                // Failure is already handled by the receive_packet fn above
                if matches!(code, OutputCode::Failure | OutputCode::StreamEOF) {
                    return None;
                }

                drop(stream_guard);

                Some((get_found_device(device_buf), Some((stream_guard_ref, true))))
            },
        );

        Box::pin(stream_iter.filter(|device| future::ready(device.address != [0; ADDR_LEN])))
    }

    pub async fn disconnect_device(&self) -> OutputCode {
        self.send_packet_to_daemon(DISCONNECT, EMPTY_BUFFER).await.0
    }

    pub async fn connect_device(&self) -> OutputCode {
        let mut buf = EMPTY_BUFFER;
        buf[0] = SET;
        self.send_packet_to_daemon(CONNECT, buf).await.0
    }

    async fn get_file_socket() -> TokioStream {
        let fs_name = SOCKET_PATH
            .to_fs_name::<FilesystemUdSocket>()
            .unwrap_or_else(|error| {
                eprintln!("Error cannot create filesystem path name: {error}");
                std::process::exit(2);
            });
        TokioStream::connect(fs_name).await.unwrap_or_else(|error| {
            eprintln!("Error cannot connect to file socket name: {SOCKET_PATH} => {error}");
            std::process::exit(2);
        })
    }

    async fn send_packet_to_daemon(&self, flags: MaskT, data: [u8; DATA_LEN + 1]) -> CmdOutput {
        Self::_send_packet_to_daemon(
            &mut Self::get_file_socket().await,
            Some(self.addr.into_inner()),
            flags,
            data,
        )
        .await
    }

    /// Data is DATA_LEN + 1 for set/get flag
    async fn _send_packet_to_daemon(
        stream: &mut TokioStream,
        address: Option<[u8; ADDR_LEN]>,
        flags: MaskT,
        data: [u8; DATA_LEN + 1],
    ) -> CmdOutput {
        #[allow(unused_assignments)]
        let mut offset = 0;
        let mut chunks = [0; BUFFER_LEN];
        if let Some(addr) = address {
            for (i, byte) in addr.iter().enumerate() {
                chunks[i] = *byte;
            }
        }
        offset = ADDR_LEN;
        chunks[offset] = (flags & 0xff) as _;
        offset += 1;
        chunks[offset] = (flags >> 8) as _;
        offset += 1;
        for (i, byte) in data.iter().enumerate() {
            chunks[i + offset] = *byte;
        }

        stream.write_all(&chunks[..]).await.unwrap();
        stream.flush().await.unwrap();

        Self::receive_packet_from_daemon(stream).await
    }

    async fn receive_packet_from_daemon(stream: &mut TokioStream) -> CmdOutput {
        // - 1 since the first byte is the output code
        let mut output = [0; OUTPUT_LEN - 1];

        let mut buf = [0; OUTPUT_LEN];
        if let Err(error) = stream.read_exact(&mut buf).await {
            eprintln!("Error cannot read daemon output, please check /var/log/rustbee-daemon.log file ({error}) buffer: {buf:?}");
            return (OutputCode::Failure, output);
        }

        for (i, byte) in buf[1..].iter().enumerate() {
            output[i] = *byte;
        }

        (OutputCode::from(buf[0]), output)
    }
}

#[cfg(feature = "ffi")]
impl HueDevice<FFI>
where
    HueDevice<FFI>: Default + std::fmt::Debug,
{
    pub fn get_file_socket() -> interprocess::local_socket::Stream {
        let fs_name = SOCKET_PATH
            .to_fs_name::<FilesystemUdSocket>()
            .unwrap_or_else(|error| {
                eprintln!("Error cannot create filesystem path name: {error}");
                std::process::exit(2);
            });
        SyncStream::connect(fs_name).unwrap_or_else(|error| {
            eprintln!("Error cannot connect to file socket name: {SOCKET_PATH} => {error}");
            std::process::exit(2);
        })
    }

    pub fn send_packet_to_daemon(
        stream: &mut SyncStream,
        address: Option<[u8; ADDR_LEN]>,
        flags: MaskT,
        data: [u8; DATA_LEN + 1],
    ) -> CmdOutput {
        use std::io::Write as _;

        #[allow(unused_assignments)]
        let mut offset = 0;
        let mut chunks = [0; BUFFER_LEN];
        if let Some(addr) = address {
            for (i, byte) in addr.iter().enumerate() {
                chunks[i] = *byte;
            }
        }
        offset = ADDR_LEN;
        chunks[offset] = (flags & 0xff) as _;
        offset += 1;
        chunks[offset] = (flags >> 8) as _;
        offset += 1;
        for (i, byte) in data.iter().enumerate() {
            chunks[i + offset] = *byte;
        }

        stream.write_all(&chunks[..]).unwrap();
        stream.flush().unwrap();

        Self::receive_packet_from_daemon(stream)
    }

    fn receive_packet_from_daemon(stream: &mut SyncStream) -> CmdOutput {
        use std::io::Read as _;

        let mut output = [0; OUTPUT_LEN - 1];

        let mut buf = [0; OUTPUT_LEN];
        if let Err(error) = stream.read_exact(&mut buf) {
            eprintln!("Error cannot read daemon output, please check /var/log/rustbee-daemon.log file ({error}) buffer: {buf:?}");
            return (OutputCode::Failure, output);
        }

        for (i, byte) in buf[1..].iter().enumerate() {
            output[i] = *byte;
        }

        (OutputCode::from(buf[0]), output)
    }
}

pub async fn search_devices_by_name<T>(
    name: &str,
    timeout_seconds: u64,
) -> btleplug::Result<Pin<Box<dyn stream::Stream<Item = HueDevice<T>> + Send>>>
where
    T: std::fmt::Debug + Send + 'static,
    HueDevice<T>: Default,
{
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.into_iter().next().unwrap();

    let discovery = adapter.events().await?;

    let stream = stream::unfold(
        Some((discovery, adapter, name.to_string())),
        move |state| async move {
            let (mut discovery, adapter, name) = match state {
                Some(state) => state,
                None => return None,
            };

            match time::timeout(Duration::from_secs(timeout_seconds), discovery.next()).await {
                Ok(Some(CentralEvent::DeviceDiscovered(id))) => {
                    if let Ok(bt_device) = adapter.peripheral(&id).await {
                        if let Some(device_name) = bt_device
                            .properties()
                            .await
                            .unwrap_or(None)
                            .map(|properties| properties.local_name)
                            .unwrap_or(None)
                        {
                            if device_name.to_lowercase().contains(&name.to_lowercase()) {
                                let mut hue_device = HueDevice::new(bt_device.address());
                                hue_device.set_device(bt_device);
                                return Some((hue_device, Some((discovery, adapter, name))));
                            }
                        }
                    }
                }
                Ok(None) | Err(_) => return None, // No more events or timeout reached
                _ => (),
            }

            Some((HueDevice::default(), Some((discovery, adapter, name))))
        },
    );

    // TODO: Try to remove duplicates
    Ok(Box::pin(stream.filter(|hue_device| {
        future::ready(hue_device.device.is_some())
    })))
}

pub async fn get_device<T>(address: [u8; ADDR_LEN]) -> btleplug::Result<Option<HueDevice<T>>>
where
    T: std::fmt::Debug,
    HueDevice<T>: Default,
{
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.first().unwrap();

    let mut discovery = adapter.events().await?;
    let mut device: Option<HueDevice<T>> = None;

    while let Some(event) = discovery.next().await {
        if let CentralEvent::DeviceDiscovered(id) = event {
            let bt_device = match adapter.peripheral(&id).await {
                Ok(peripheral) => peripheral,
                _ => continue,
            };
            let addr = bt_device.address();
            let addr_slice = addr.into_inner();

            if address != addr_slice {
                continue;
            }

            let mut hue_device = HueDevice::new(addr);
            hue_device.set_device(bt_device);
            device = Some(hue_device);
            break;
        }
    }

    Ok(device)
}

pub async fn get_devices<T>(addrs: &[[u8; ADDR_LEN]]) -> btleplug::Result<Vec<HueDevice<T>>>
where
    T: std::fmt::Debug,
    HueDevice<T>: Default,
{
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.first().unwrap();

    let mut discovery = adapter.events().await?;
    let mut addresses: HashMap<[u8; ADDR_LEN], HueDevice<T>> = HashMap::with_capacity(addrs.len());

    addrs.iter().for_each(|addr| {
        addresses.insert(*addr, HueDevice::new(BDAddr::from(*addr)));
    });

    while let Some(event) = discovery.next().await {
        if let CentralEvent::DeviceDiscovered(id) = event {
            let bt_device = match adapter.peripheral(&id).await {
                Ok(peripheral) => peripheral,
                _ => continue,
            };
            let addr = bt_device.address();
            let addr_slice = addr.into_inner();
            if !addresses.contains_key(&addr_slice) {
                continue;
            }

            let hue_device = addresses.get_mut(&addr_slice).unwrap(); // Shouldn't panic
            hue_device.set_device(bt_device);

            if !addresses.iter().any(|(_, v)| v.device.is_none()) {
                // Not any None variant
                // device
                break;
            }
        }
    }

    Ok(addresses.into_values().collect())
}
