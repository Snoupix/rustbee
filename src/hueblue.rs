use std::collections::HashMap;
use std::ops::Deref;
use std::pin::Pin;
use std::time::Duration;

use bluer::{
    gatt::remote::{Characteristic as BlueCharacteristic, Service as BlueService},
    AdapterEvent, Address, Device, Session,
};
use futures::StreamExt;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct HueBar {
    pub device: Option<Device>,
    pub addr: Address,
    pub services: Option<Vec<Service>>,
}

#[derive(Debug)]
pub struct Service {
    pub uuid: Uuid,
    pub id: u16,
    pub characteristics: Vec<Characteristic>,
    pub inner: BlueService,
}

#[derive(Debug)]
pub struct Characteristic {
    pub uuid: Uuid,
    pub id: u16,
    pub inner: BlueCharacteristic,
}

impl HueBar {
    fn new(addr: Address) -> Self {
        // TODO: On init, load every services onto the struct so it
        // avoids to iterate over them all since bluer only indexes
        // services and characteristics by ID and not UUID
        Self {
            addr,
            ..Default::default()
        }
    }

    fn set_device(&mut self, device: Device) {
        self.device = Some(device);
    }

    fn unset_device(&mut self) {
        self.device = None;
    }

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
        &mut self,
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

    pub async fn ensure_pairing(&mut self) -> bluer::Result<()> {
        let mut retries = 3;
        let mut error = None;
        while !self.is_paired().await? {
            if retries <= 0 {
                eprintln!(
                    "[ERROR] Failed to pair device {} after 3 attempts {:?}",
                    self.addr, error
                );
                return Err(bluer::Error {
                    kind: bluer::ErrorKind::Failed,
                    message: "Faileed to disconnect after 3 attempts".into(),
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
                    message: "Faileed to disconnect after 3 attempts".into(),
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

impl Deref for HueBar {
    type Target = Device;

    /// Be sure to use it wisely since it NEEDS to have the device set
    fn deref(&self) -> &Self::Target {
        self.device.as_ref().unwrap()
    }
}

pub async fn get_devices(addrs: &[[u8; 6]]) -> bluer::Result<Vec<HueBar>> {
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    if !adapter.is_powered().await? {
        adapter.set_powered(true).await?;
    }

    let mut discovery = adapter.discover_devices().await?;
    let mut pinned_disco = unsafe { Pin::new_unchecked(&mut discovery) };
    let mut addresses: HashMap<[u8; 6], HueBar> = HashMap::with_capacity(addrs.len());

    addrs.iter().for_each(|addr| {
        addresses.insert(*addr, HueBar::new(Address::new(*addr)));
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