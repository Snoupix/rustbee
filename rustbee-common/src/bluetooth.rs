use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::time::Duration;

use bluer::{
    gatt::remote::{Characteristic as BlueCharacteristic, Service as BlueService},
    AdapterEvent, Address, Device, Session,
};
use flags::{COLOR_HEX, COLOR_RGB};
use futures::StreamExt;
use interprocess::{
    local_socket::{tokio::Stream, traits::tokio::Stream as _, ToFsName as _},
    os::unix::local_socket::FilesystemUdSocket,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    time::sleep,
};
use uuid::Uuid;

use crate::constants::{masks::*, *};

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

impl<T> Deref for HueDevice<T> {
    type Target = Device;

    /// Be sure to use it wisely since it NEEDS to have the device set
    fn deref(&self) -> &Self::Target {
        self.device.as_ref().unwrap()
    }
}

impl<T> HueDevice<T>
where
    HueDevice<T>: Default + Deref<Target = Device> + std::fmt::Debug,
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

    pub async fn get_power(&self) -> bluer::Result<bool> {
        let read = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &POWER_UUID)
            .await?;
        if let Some(bytes) = read {
            Ok(*bytes.first().unwrap() == true as _)
        } else {
            Err(bluer::Error {
                kind: bluer::ErrorKind::InvalidArguments,
                message: format!("[ERROR] Service or Characteristic \"{POWER}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
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
                message: format!("[ERROR] Service or Characteristic \"{BRIGHTNESS}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {}", self.addr)
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
}

type CmdOutput = (bool, [u8; OUTPUT_LEN - 1]);

impl HueDevice<Client>
where
    HueDevice<Client>: Default + Deref<Target = Device> + std::fmt::Debug,
{
    pub async fn pair(&self) -> bool {
        self.send_packet_to_daemon(PAIR, [0; 6]).await.0
    }

    pub async fn set_power(&self, state: bool) -> bool {
        let mut buf = [0u8; DATA_LEN];
        buf[0] = SET;
        buf[1] = state as _;

        self.send_packet_to_daemon(CONNECT | POWER, buf).await.0
    }

    pub async fn get_power(&self) -> CmdOutput {
        let mut buf = [0u8; DATA_LEN];
        buf[0] = GET;

        self.send_packet_to_daemon(CONNECT | POWER, buf).await
    }

    pub async fn set_brightness(&self, value: u8) -> bool {
        let mut buf = [0u8; DATA_LEN];
        buf[0] = SET;
        buf[1] = (((value as f32) / 100.) * 0xff as f32) as _;

        self.send_packet_to_daemon(CONNECT | BRIGHTNESS, buf)
            .await
            .0
    }

    pub async fn get_brightness(&self) -> CmdOutput {
        let mut buf = [0u8; DATA_LEN];
        buf[0] = GET;

        self.send_packet_to_daemon(CONNECT | BRIGHTNESS, buf).await
    }

    pub async fn get_colors(&self, color_mask: u8) -> CmdOutput {
        assert!([COLOR_XY, COLOR_RGB, COLOR_HEX].contains(&color_mask));

        let mut buf = [0u8; DATA_LEN];
        buf[0] = GET;

        self.send_packet_to_daemon(CONNECT | color_mask, buf).await
    }

    pub async fn set_colors(&self, scaled_x: u16, scaled_y: u16, color_mask: u8) -> bool {
        assert!([COLOR_XY, COLOR_RGB, COLOR_HEX].contains(&color_mask));

        let mut buf = [0u8; DATA_LEN];
        buf[0] = SET;
        buf[1] = (scaled_x & 0xFF) as _;
        buf[2] = (scaled_x >> 8) as _;
        buf[3] = (scaled_y & 0xFF) as _;
        buf[4] = (scaled_y >> 8) as _;

        self.send_packet_to_daemon(CONNECT | color_mask, buf)
            .await
            .0
    }

    pub async fn disconnect_device(&self) -> bool {
        let mut buf = [0u8; DATA_LEN];
        buf[0] = GET;

        self.send_packet_to_daemon(DISCONNECT, buf).await.0
    }

    pub async fn connect_device(&self) -> bool {
        let mut buf = [0u8; DATA_LEN];
        buf[0] = GET;

        self.send_packet_to_daemon(CONNECT, buf).await.0
    }

    async fn send_packet_to_daemon(&self, flags: MaskT, data: [u8; DATA_LEN]) -> CmdOutput {
        let mut output = [0; OUTPUT_LEN - 1];

        let fs_name = SOCKET_PATH
            .to_fs_name::<FilesystemUdSocket>()
            .unwrap_or_else(|error| {
                eprintln!("Error cannot create filesystem path name: {error}");
                std::process::exit(2);
            });
        let mut stream = Stream::connect(fs_name).await.unwrap_or_else(|error| {
            eprintln!("Error cannot connect to file socket name: {SOCKET_PATH} => {error}");
            std::process::exit(2);
        });

        let mut chunks = [0; BUFFER_LEN];
        for (i, byte) in self.addr.0.iter().enumerate() {
            chunks[i] = *byte;
        }
        chunks[DATA_LEN] = flags;
        for (i, byte) in data.iter().enumerate() {
            chunks[i + DATA_LEN + 1] = *byte;
        }

        stream.write_all(&chunks[..]).await.unwrap();
        stream.flush().await.unwrap();

        let mut buf = [0; OUTPUT_LEN];
        if let Err(error) = stream.read_exact(&mut buf).await {
            eprintln!("Error cannot read daemon output, please check /var/log/rustbee-daemon.log file ({error})");
            return (false, output);
        }

        for (i, byte) in buf[1..].iter().enumerate() {
            output[i] = *byte;
        }

        (buf[0] & SUCCESS == 1, output)
    }
}

pub async fn get_devices<T>(addrs: &[[u8; 6]]) -> bluer::Result<Vec<HueDevice<T>>>
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
    let mut addresses: HashMap<[u8; 6], HueDevice<T>> = HashMap::with_capacity(addrs.len());

    addrs.iter().for_each(|addr| {
        addresses.insert(*addr, HueDevice::new(Address::new(*addr)));
    });

    while let Some(event) = pinned_disco.next().await {
        match event {
            AdapterEvent::DeviceAdded(addr) => {
                let addr_vec = addr.to_vec();
                let addr_slice = addr_vec.as_slice();
                if !addresses.contains_key(addr_slice) {
                    continue;
                }

                let hue_bar = addresses.get_mut(addr_slice).unwrap(); // Shouldn't panic
                hue_bar.set_device(adapter.device(addr)?);

                if !addresses.iter().any(|(_, v)| v.device.is_none()) {
                    // Not any None variant
                    // device
                    break;
                }
            }
            AdapterEvent::DeviceRemoved(addr) => {
                let addr_vec = addr.to_vec();
                let addr_slice = addr_vec.as_slice();
                if !addresses.contains_key(addr_slice) {
                    continue;
                }

                let hue_bar = addresses.get_mut(addr_slice).unwrap(); // Shouldn't panic
                hue_bar.unset_device();
            }
            _ => (),
        }
    }

    Ok(addresses.into_values().collect())
}
