use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bluest::{Adapter, AdvertisingDevice};
use futures::{future, stream, StreamExt};
use log::*;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::timeout;
use windows::core::{Error as WinError, Result as WinResult, RuntimeType, HSTRING};
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Foundation::{AsyncStatus, IAsyncOperation};

use crate::constants::ADDR_LEN;
use crate::device::{HueDevice, Server};
use crate::utils::{addr_to_uint, uint_to_addr};

const NO_ADAPTER_FOUND: &str = "Failed to get Bluetooth adapter. (maybe your Bluetooth is OFF ?)";

async fn scan(adapter: Adapter, tx: Sender<AdvertisingDevice>) {
    let mut discovery = adapter.scan(&[]).await.unwrap();

    while let Some(dev) = discovery.next().await {
        // If it errors, channel is probably closed so it's alright to deny the error
        if tx.send(dev).await.is_err() {
            break;
        }
    }
}

pub async fn search_devices_by_name(
    name: &str,
    timeout_seconds: u64,
) -> bluest::Result<Pin<Box<dyn stream::Stream<Item = HueDevice<Server>> + Send>>> {
    let Some(adapter) = Adapter::default().await else {
        error!("{NO_ADAPTER_FOUND}");
        return Err(bluest::error::ErrorKind::Other.into());
    };

    adapter.wait_available().await?;

    let (disco_tx, disco_rx) = mpsc::channel(1);

    tokio::spawn(scan(adapter, disco_tx));

    let stream = stream::unfold(
        Some((disco_rx, name.to_string(), HashSet::new())),
        move |state| async move {
            let (mut discovery, name, mut seen_devices) = match state {
                Some(state) => state,
                None => return None,
            };

            match timeout(Duration::from_secs(timeout_seconds), discovery.recv()).await {
                Ok(Some(adv_device)) => {
                    match seen_devices.get(&adv_device.device.id()) {
                        Some(_) => {
                            return Some((
                                HueDevice::default(),
                                Some((discovery, name, seen_devices)),
                            ))
                        }
                        None => seen_devices.insert(adv_device.device.id()),
                    };

                    if let Ok(device_name) = adv_device.device.name() {
                        if device_name.to_lowercase().contains(&name.to_lowercase()) {
                            if let Some(Ok(address)) = get_windows_device_from_device_id(
                                adv_device.device.id().to_string(),
                            )
                            .await
                            .map(|ble_device| ble_device.BluetoothAddress())
                            {
                                let hue_device = HueDevice::new_with_device(
                                    uint_to_addr(address),
                                    adv_device.device,
                                );
                                return Some((hue_device, Some((discovery, name, seen_devices))));
                            }
                        }
                    }
                }
                Ok(None) | Err(_) => return None, // No more events or timeout reached
            }

            Some((HueDevice::default(), Some((discovery, name, seen_devices))))
        },
    );

    Ok(Box::pin(stream.filter(|hue_device| {
        future::ready(hue_device.device.is_some())
    })))
}

pub async fn get_device(address: [u8; ADDR_LEN]) -> bluest::Result<Option<HueDevice<Server>>> {
    let Some(adapter) = Adapter::default().await else {
        error!("{NO_ADAPTER_FOUND}");
        return Err(bluest::error::ErrorKind::Other.into());
    };

    adapter.wait_available().await?;

    let mut device = None;

    let mut discovery = adapter.scan(&[]).await?;
    while let Some(adv_device) = discovery.next().await {
        let addr = match get_windows_device_from_device_id(adv_device.device.id().to_string())
            .await
            .map(|dev| dev.BluetoothAddress())
        {
            Some(res) => match res {
                Ok(addr) => addr,
                Err(err) => {
                    error!("Unexpected error while getting Windows BLE Device address {err}");
                    continue;
                }
            },
            None => continue,
        };

        let addr_conv = addr_to_uint(&address);
        if addr != addr_conv {
            continue;
        }

        device = Some(HueDevice::new_with_device(address, adv_device.device));

        break;
    }

    Ok(device)
}

pub async fn get_devices(addrs: &[[u8; ADDR_LEN]]) -> bluest::Result<Vec<HueDevice<Server>>> {
    let Some(adapter) = Adapter::default().await else {
        error!("{NO_ADAPTER_FOUND}");
        return Err(bluest::error::ErrorKind::Other.into());
    };

    adapter.wait_available().await?;

    let mut addresses = HashMap::with_capacity(addrs.len());
    addrs.iter().for_each(|addr| {
        addresses.insert(*addr, HueDevice::new(*addr));
    });

    let mut discovery = adapter.scan(&[]).await?;
    while let Some(adv_device) = discovery.next().await {
        let addr = match get_windows_device_from_device_id(adv_device.device.id().to_string())
            .await
            .map(|dev| dev.BluetoothAddress())
        {
            Some(res) => match res {
                Ok(addr) => addr,
                Err(err) => {
                    error!("Unexpected error while getting Windows BLE Device address {err}");
                    continue;
                }
            },
            None => continue,
        };

        let addr_slice = uint_to_addr(addr);
        let hue_device = addresses.get_mut(&addr_slice).unwrap(); // Shouldn't panic
        hue_device.set_device(adv_device.device);

        if !addresses.iter().any(|(_, v)| v.device.is_none()) {
            // Not any None variant
            // device
            break;
        }
    }

    Ok(addresses.into_values().collect())
}

async fn get_windows_device_from_device_id(device_id: String) -> Option<BluetoothLEDevice> {
    let async_op: AsyncOp<BluetoothLEDevice> =
        BluetoothLEDevice::FromIdAsync(&HSTRING::from(device_id.clone()))
            .unwrap_or_else(|err| {
                panic!("Failed to create a windows BLE device from id: {device_id}. {err}")
            })
            .into();

    let timeout = timeout(Duration::from_millis(1000), async_op).await;
    if timeout.is_err() {
        return None;
    }

    let device = timeout.unwrap();

    Some(device.unwrap_or_else(|err| {
        panic!("Unreachable: Failed to get Bluetooth LE device from windows API: {err}")
    }))
}

struct AsyncOp<T: RuntimeType + 'static>(IAsyncOperation<T>);

impl<T> Future for AsyncOp<T>
where
    T: RuntimeType,
{
    type Output = WinResult<T>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.Status()? {
            AsyncStatus::Started => Poll::Pending,
            AsyncStatus::Completed => Poll::Ready(self.0.GetResults()),
            AsyncStatus::Error => Poll::Ready(Err(WinError::from_hresult(self.0.ErrorCode()?))),
            AsyncStatus::Canceled => Poll::Ready(Err(WinError::empty())), // unreachable
            _ => unreachable!(),
        }
    }
}

impl<T> From<IAsyncOperation<T>> for AsyncOp<T>
where
    T: RuntimeType,
{
    fn from(operation: IAsyncOperation<T>) -> Self {
        Self(operation)
    }
}
