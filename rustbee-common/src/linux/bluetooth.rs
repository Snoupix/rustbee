use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _};
use btleplug::platform::Manager;
use futures::{future, stream, StreamExt};
use tokio::time;

use crate::device::*;

use crate::constants::ADDR_LEN;

const NO_ADAPTER_FOUND: &str = "Failed to get Bluetooth adapter. (maybe your Bluetooth is OFF ?)";

pub async fn search_devices_by_name(
    name: &str,
    timeout_seconds: u64,
) -> btleplug::Result<Pin<Box<dyn stream::Stream<Item = HueDevice<Server>> + Send>>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = match adapters.into_iter().next() {
        Some(adapter) => adapter,
        None => {
            return Err(btleplug::Error::Other(Box::new(Error(
                NO_ADAPTER_FOUND.into(),
            ))));
        }
    };

    let discovery = adapter.events().await?;

    let stream = stream::unfold(
        Some((discovery, adapter, name.to_string(), HashSet::new())),
        move |state| async move {
            let (mut discovery, adapter, name, mut seen_devices) = match state {
                Some(state) => state,
                None => return None,
            };

            match time::timeout(Duration::from_secs(timeout_seconds), discovery.next()).await {
                Ok(Some(CentralEvent::DeviceDiscovered(id))) => {
                    match seen_devices.get(&id) {
                        Some(_) => {
                            return Some((
                                HueDevice::default(),
                                Some((discovery, adapter, name, seen_devices)),
                            ))
                        }
                        None => seen_devices.insert(id.clone()),
                    };

                    if let Ok(bt_device) = adapter.peripheral(&id).await {
                        if let Some(device_name) = bt_device
                            .properties()
                            .await
                            .unwrap_or(None)
                            .map(|properties| properties.local_name)
                            .unwrap_or(None)
                        {
                            if device_name.to_lowercase().contains(&name.to_lowercase()) {
                                let mut hue_device =
                                    HueDevice::new(bt_device.address().into_inner());
                                hue_device.set_device(bt_device);
                                return Some((
                                    hue_device,
                                    Some((discovery, adapter, name, seen_devices)),
                                ));
                            }
                        }
                    }
                }
                Ok(None) | Err(_) => return None, // No more events or timeout reached
                _ => (),
            }

            Some((
                HueDevice::default(),
                Some((discovery, adapter, name, seen_devices)),
            ))
        },
    );

    Ok(Box::pin(stream.filter(|hue_device| {
        future::ready(hue_device.device.is_some())
    })))
}

pub async fn get_device(address: [u8; ADDR_LEN]) -> btleplug::Result<Option<HueDevice<Server>>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = match adapters.into_iter().next() {
        Some(adapter) => adapter,
        None => {
            return Err(btleplug::Error::Other(Box::new(Error(
                NO_ADAPTER_FOUND.into(),
            ))));
        }
    };

    let mut discovery = adapter.events().await?;
    let mut device = None;

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

            device = Some(HueDevice::new_with_device(addr.into_inner(), bt_device));
            break;
        }
    }

    Ok(device)
}

pub async fn get_devices(addrs: &[[u8; ADDR_LEN]]) -> btleplug::Result<Vec<HueDevice<Server>>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = match adapters.into_iter().next() {
        Some(adapter) => adapter,
        None => {
            return Err(btleplug::Error::Other(Box::new(Error(
                NO_ADAPTER_FOUND.into(),
            ))));
        }
    };

    let mut discovery = adapter.events().await?;
    let mut addresses = HashMap::with_capacity(addrs.len());

    addrs.iter().for_each(|addr| {
        addresses.insert(*addr, HueDevice::new(*addr));
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
