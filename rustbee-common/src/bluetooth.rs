use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bluer::{
    gatt::remote::{Characteristic as BlueCharacteristic, Service as BlueService},
    AdapterEvent, Address, Device, Session,
};
use futures::{future, stream, StreamExt};
use interprocess::{
    local_socket::{tokio::Stream, traits::tokio::Stream as _, ToFsName as _},
    os::unix::local_socket::FilesystemUdSocket,
};
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    time::{self, sleep},
};
use uuid::Uuid;

use crate::constants::{
    flags::{COLOR_HEX, COLOR_RGB},
    masks::*,
    *,
};

const EMPTY_BUFFER: [u8; DATA_LEN + 1] = [0; DATA_LEN + 1];

#[derive(Debug, Default)]
pub struct FoundDevice {
    pub address: [u8; ADDR_LEN],
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct Client;
#[derive(Clone, Debug, Default)]
pub struct Server;

#[derive(Clone, Debug)]
pub struct HueDevice<Type> {
    pub addr: Address,
    pub device: Option<Device>,
    pub services: Option<Vec<Service>>,
    _type: PhantomData<Type>,
}

impl Default for HueDevice<Server> {
    fn default() -> Self {
        Self {
            addr: Default::default(),
            device: Default::default(),
            services: Default::default(),
            _type: Default::default(),
        }
    }
}
impl Default for HueDevice<Client> {
    fn default() -> Self {
        Self {
            addr: Default::default(),
            device: Default::default(),
            services: Default::default(),
            _type: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Service {
    pub uuid: Uuid,
    pub id: u16,
    pub characteristics: Vec<Characteristic>,
    pub inner: BlueService,
}

#[derive(Clone, Debug)]
pub struct Characteristic {
    pub uuid: Uuid,
    pub id: u16,
    pub inner: BlueCharacteristic,
}

impl Deref for Service {
    type Target = BlueService;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Deref for Characteristic {
    type Target = BlueCharacteristic;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// The client doesn't use the bluetooth struct so only the server needs to deref since the client
// device field should always be None
impl Deref for HueDevice<Server> {
    type Target = Device;

    /// Be sure to use it wisely since it NEEDS to have the device set
    fn deref(&self) -> &Self::Target {
        self.device.as_ref().unwrap()
    }
}

impl<T> HueDevice<T>
where
    HueDevice<T>: Default + std::fmt::Debug,
{
    pub fn new(addr: Address) -> Self {
        Self {
            addr,
            ..Default::default()
        }
    }

    pub fn new_with_device(addr: Address, device: Device) -> Self {
        Self {
            addr,
            device: Some(device),
            ..Default::default()
        }
    }

    pub fn set_device(&mut self, device: Device) {
        self.device = Some(device);
    }

    pub fn unset_device(&mut self) {
        self.device = None;
    }
}

impl HueDevice<Server>
where
    HueDevice<Server>: Default + Deref<Target = Device> + std::fmt::Debug,
{
    pub async fn set_services(&mut self) -> bluer::Result<()> {
        let mut services = Vec::new();

        for service in self.services().await? {
            let mut characs = Vec::new();
            for charac in service.characteristics().await? {
                characs.push(Characteristic {
                    uuid: charac.uuid().await?,
                    id: charac.id(),
                    inner: charac,
                });
            }

            services.push(Service {
                uuid: service.uuid().await?,
                id: service.id(),
                characteristics: characs,
                inner: service,
            });

            sleep(Duration::from_millis(150)).await;
        }

        self.services = Some(services);
        Ok(())
    }

    pub async fn read_gatt_char(
        &self,
        service: &Uuid,
        charac: &Uuid,
    ) -> bluer::Result<Option<Vec<u8>>> {
        if let Some(service) = self
            .services
            .as_ref()
            .unwrap()
            .iter()
            .find(|&s| &s.uuid == service)
        {
            if let Some(charac) = service.characteristics.iter().find(|&c| &c.uuid == charac) {
                return Ok(Some(charac.read().await?));
            }
        }

        Ok(None)
    }

    pub async fn write_gatt_char(
        &self,
        service: &Uuid,
        charac: &Uuid,
        bytes: &[u8],
    ) -> bluer::Result<bool> {
        if let Some(service) = self
            .services
            .as_ref()
            .unwrap()
            .iter()
            .find(|&s| &s.uuid == service)
        {
            if let Some(charac) = service.characteristics.iter().find(|&c| &c.uuid == charac) {
                charac.write(bytes).await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn try_connect(&self) -> bluer::Result<()> {
        let mut retries = 3;
        loop {
            if self.is_connected().await? {
                break;
            }

            if retries <= 0 {
                eprintln!(
                    "[ERROR] Failed to connect to {} after 3 attempts",
                    self.addr
                );
                return Err(bluer::Error {
                    kind: bluer::ErrorKind::Failed,
                    message: "Failed to disconnect after 3 attempts".into(),
                });
            }

            if let Err(error) = self.connect().await {
                eprintln!("[WARN] Connecting to device {} failed: {error}", self.addr);
            }

            retries -= 1;
        }
        sleep(Duration::from_millis(150)).await;

        Ok(())
    }

    pub async fn try_disconnect(&self) -> bluer::Result<()> {
        let mut retries = 3;
        loop {
            if !self.is_connected().await? {
                break;
            }

            if retries <= 0 {
                eprintln!(
                    "[ERROR] Failed to disconnect from {} after 3 attempts",
                    self.addr
                );
                return Err(bluer::Error {
                    kind: bluer::ErrorKind::Failed,
                    message: "Failed to disconnect after 3 attempts".into(),
                });
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

    pub async fn try_pair(&self) -> bluer::Result<()> {
        let mut retries = 3;
        let mut error = None;

        if self.is_connected().await? {
            return Ok(());
        }

        while !self.is_paired().await? {
            if retries <= 0 {
                eprintln!(
                    "[ERROR] Failed to pair device {} after 3 attempts {:?}",
                    self.addr, error
                );
                return Err(bluer::Error {
                    kind: bluer::ErrorKind::Failed,
                    message: "Failed to pair device after 3 attempts".into(),
                });
            }
            error = match self.pair().await {
                Ok(_) => break,
                Err(err) => Some(err),
            };
            retries -= 1;
        }

        retries = 2;
        error = None;
        while !self.is_trusted().await? {
            if retries <= 0 {
                eprintln!(
                    "[ERROR] Failed to \"trust\" device {} after 3 attempts {:?}",
                    self.addr, error
                );
                return Err(bluer::Error {
                    kind: bluer::ErrorKind::Failed,
                    message: "Failed to trust device after 3 attempts".into(),
                });
            }
            error = match self.set_trusted(true).await {
                Ok(_) => break,
                Err(err) => Some(err),
            };
            retries -= 1;
        }

        Ok(())
    }

    pub async fn is_device_connected(&self) -> bluer::Result<bool> {
        (*self).is_connected().await
    }

    pub async fn get_power(&self) -> bluer::Result<bool> {
        let read = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &POWER_UUID)
            .await?;
        if let Some(bytes) = read {
            Ok(*bytes.first().unwrap() == true as _)
        } else {
            Err(bluer::Error {
                kind: bluer::ErrorKind::InvalidArguments,
                message: format!("[ERROR] Service or Characteristic \"{POWER_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
            })
        }
    }

    pub async fn set_power(&self, value: u8) -> bluer::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &POWER_UUID, &[value])
            .await?;

        Ok(())
    }

    pub async fn get_brightness(&self) -> bluer::Result<f32> {
        let read = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &BRIGHTNESS_UUID)
            .await?;
        if let Some(bytes) = read {
            Ok(*bytes.first().unwrap() as f32)
        } else {
            Err(bluer::Error {
                kind: bluer::ErrorKind::InvalidArguments,
                message: format!("[ERROR] Service or Characteristic \"{BRIGHTNESS_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
            })
        }
    }

    pub async fn set_brightness(&self, value: u8) -> bluer::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &BRIGHTNESS_UUID, &[value])
            .await?;

        Ok(())
    }

    pub async fn get_color(&self) -> bluer::Result<[u8; 4]> {
        let mut buf = [0u8; 4];
        if let Some(bytes) = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &COLOR_UUID)
            .await?
        {
            let len = buf.len();
            buf.copy_from_slice(&bytes[..len]);

            Ok(buf)
        } else {
            Err(bluer::Error {
                kind: bluer::ErrorKind::InvalidArguments,
                message: format!("[ERROR] Service or Characteristic \"{COLOR_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
            })
        }
    }

    pub async fn set_color(&self, buf: [u8; 4]) -> bluer::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &COLOR_UUID, &buf)
            .await?;

        Ok(())
    }

    pub async fn get_name(&self) -> bluer::Result<Option<String>> {
        self.name().await
    }
}

type CmdOutput = (OutputCode, [u8; OUTPUT_LEN - 1]);

impl HueDevice<Client>
where
    HueDevice<Client>: Default + std::fmt::Debug,
{
    pub async fn pair(&self) -> OutputCode {
        self.send_packet_to_daemon(PAIR, EMPTY_BUFFER).await.0
    }

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

        self.send_packet_to_daemon(CONNECT | color_mask, buf)
            .await
            .0
    }

    pub async fn get_name(&self) -> CmdOutput {
        let mut buf = EMPTY_BUFFER;
        buf[0] = GET;

        self.send_packet_to_daemon(CONNECT, buf).await
    }

    pub async fn is_connected(&self) -> CmdOutput {
        let mut buf = EMPTY_BUFFER;
        buf[0] = GET;

        self.send_packet_to_daemon(CONNECT, buf).await
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

                match code {
                    // Failure is already handled by the receive_packet fn above
                    OutputCode::Failure | OutputCode::StreamEOF => return None,
                    _ => (),
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
        self.send_packet_to_daemon(CONNECT, EMPTY_BUFFER).await.0
    }

    async fn get_file_socket() -> Stream {
        let fs_name = SOCKET_PATH
            .to_fs_name::<FilesystemUdSocket>()
            .unwrap_or_else(|error| {
                eprintln!("Error cannot create filesystem path name: {error}");
                std::process::exit(2);
            });
        Stream::connect(fs_name).await.unwrap_or_else(|error| {
            eprintln!("Error cannot connect to file socket name: {SOCKET_PATH} => {error}");
            std::process::exit(2);
        })
    }

    async fn send_packet_to_daemon(&self, flags: MaskT, data: [u8; DATA_LEN + 1]) -> CmdOutput {
        Self::_send_packet_to_daemon(
            &mut Self::get_file_socket().await,
            Some(*self.addr),
            flags,
            data,
        )
        .await
    }

    /// Data is DATA_LEN + 1 for set/get flag
    async fn _send_packet_to_daemon(
        stream: &mut Stream,
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

    async fn receive_packet_from_daemon(stream: &mut Stream) -> CmdOutput {
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

pub async fn search_devices_by_name<T>(
    name: &str,
    timeout_seconds: u64,
) -> bluer::Result<Pin<Box<dyn stream::Stream<Item = HueDevice<T>> + Send>>>
where
    T: std::fmt::Debug + Send + 'static,
    HueDevice<T>: Default,
{
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    if !adapter.is_powered().await? {
        adapter.set_powered(true).await?;
    }

    let discovery = adapter.discover_devices().await?;

    let stream = stream::unfold(
        Some((discovery, adapter, name.to_string())),
        move |state| async move {
            let (mut discovery, adapter, name) = match state {
                Some(state) => state,
                None => return None,
            };

            match time::timeout(Duration::from_secs(timeout_seconds), discovery.next()).await {
                Ok(Some(AdapterEvent::DeviceAdded(addr))) => {
                    if let Ok(ble_device) = adapter.device(addr) {
                        if let Ok(Some(device_name)) = ble_device.name().await {
                            if device_name.to_lowercase().contains(&name.to_lowercase()) {
                                let mut hue_device = HueDevice::new(addr);
                                hue_device.set_device(ble_device);
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

    Ok(Box::pin(stream.filter(|hue_device| {
        future::ready(hue_device.device.is_some())
    })))
}

pub async fn get_device<T>(address: [u8; ADDR_LEN]) -> bluer::Result<Option<HueDevice<T>>>
where
    T: std::fmt::Debug,
    HueDevice<T>: Default,
{
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    if !adapter.is_powered().await? {
        adapter.set_powered(true).await?;
    }

    let mut discovery = adapter.discover_devices().await?;
    let mut pinned_disco = unsafe { Pin::new_unchecked(&mut discovery) };
    let mut device: Option<HueDevice<T>> = None;

    while let Some(event) = pinned_disco.next().await {
        if let AdapterEvent::DeviceAdded(addr) = event {
            let addr_slice = *addr;
            if address != addr_slice {
                continue;
            }
            let ble_device = adapter.device(addr)?;

            // Device known but not in range
            if ble_device.rssi().await?.is_none() {
                // TODO: Handle me (it, somehow returns None even if in range)
            }

            let mut hue_device = HueDevice::new(addr);
            hue_device.set_device(ble_device);
            device = Some(hue_device);
            break;
        }
    }

    Ok(device)
}

pub async fn get_devices<T>(addrs: &[[u8; ADDR_LEN]]) -> bluer::Result<Vec<HueDevice<T>>>
where
    T: std::fmt::Debug,
    HueDevice<T>: Default,
{
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    if !adapter.is_powered().await? {
        adapter.set_powered(true).await?;
    }

    let mut discovery = adapter.discover_devices().await?;
    let mut pinned_disco = unsafe { Pin::new_unchecked(&mut discovery) };
    let mut addresses: HashMap<[u8; ADDR_LEN], HueDevice<T>> = HashMap::with_capacity(addrs.len());

    addrs.iter().for_each(|addr| {
        addresses.insert(*addr, HueDevice::new(Address::new(*addr)));
    });

    while let Some(event) = pinned_disco.next().await {
        match event {
            AdapterEvent::DeviceAdded(addr) => {
                let addr_slice = *addr;
                if !addresses.contains_key(&addr_slice) {
                    continue;
                }
                let ble_device = adapter.device(addr)?;

                // Device known but not in range
                if ble_device.rssi().await?.is_none() {
                    // TODO: Handle me
                }

                let hue_device = addresses.get_mut(&addr_slice).unwrap(); // Shouldn't panic
                hue_device.set_device(ble_device);

                if !addresses.iter().any(|(_, v)| v.device.is_none()) {
                    // Not any None variant
                    // device
                    break;
                }
            }
            AdapterEvent::DeviceRemoved(addr) => {
                let addr_slice = *addr;
                if !addresses.contains_key(&addr_slice) {
                    continue;
                }

                let hue_device = addresses.get_mut(&addr_slice).unwrap(); // Shouldn't panic
                hue_device.unset_device();
            }
            _ => (),
        }
    }

    Ok(addresses.into_values().collect())
}
