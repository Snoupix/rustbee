use std::collections::HashMap;
use std::ops::Deref;
use std::pin::Pin;

use bluer::{
    gatt::remote::{Characteristic, Service},
    AdapterEvent, Address, Device, Session,
};
use futures::StreamExt;
use uuid::Uuid;

#[derive(Debug)]
pub struct HueBar {
    pub device: Option<Device>,
    pub addr: Address,
}

impl HueBar {
    fn new(addr: Address) -> Self {
        // TODO: On init, load every services onto the struct so it
        // avoids to iterate over them all since bluer only indexes
        // services and characteristics by ID and not UUID
        Self { device: None, addr }
    }

    fn set_device(&mut self, device: Device) {
        self.device = Some(device);
    }

    fn unset_device(&mut self) {
        self.device = None;
    }

    pub async fn get_power_state(&self, power: Uuid) -> bluer::Result<Option<bool>> {
        let characteristic = find_charac(self, power).await?;
        if let Some(charac) = characteristic {
            let res = charac.read().await?;
            return Ok(Some(*res.first().unwrap() == 1));
        }

        Ok(None)
    }

    pub async fn set_power_state(&self, power: Uuid, state: bool) -> bluer::Result<bool> {
        let characteristic = find_charac(self, power).await?;
        if let Some(charac) = characteristic {
            charac.write(&[state as u8]).await?;
            return Ok(true);
        }

        Ok(false)
    }
}

impl Deref for HueBar {
    type Target = Device;

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

                let hue_bar = addresses.get_mut(addr_slice).unwrap(); // Shouldn't panic because of
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

                let hue_bar = addresses.get_mut(addr_slice).unwrap(); // Shouldn't panic because of
                hue_bar.unset_device();
            }
            _ => (),
        }
    }

    Ok(addresses.into_values().collect())
}

pub async fn find_service(device: &Device, uuid: Uuid) -> bluer::Result<Option<Service>> {
    for service in device.services().await.unwrap().into_iter() {
        if service.uuid().await.unwrap() == uuid {
            return Ok(Some(service));
        }
    }

    Ok(None)
}

pub async fn find_charac(device: &Device, uuid: Uuid) -> bluer::Result<Option<Characteristic>> {
    for service in device.services().await?.into_iter() {
        for charac in service.characteristics().await? {
            if charac.uuid().await? == uuid {
                return Ok(Some(charac));
            }
        }
    }

    Ok(None)
}

pub async fn get_charac(
    device: &Device,
    service: Uuid,
    uuid: Uuid,
) -> bluer::Result<Option<Characteristic>> {
    for service in device.services().await?.into_iter() {
        for charac in service.characteristics().await? {
            if charac.uuid().await? == uuid {
                return Ok(Some(charac));
            }
        }
    }

    Ok(None)
}
