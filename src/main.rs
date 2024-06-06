use std::{ops::Deref, pin::Pin};

use bluer::{AdapterEvent, Address, Device, Session};
use futures::StreamExt;

const HUE_BAR_1_ADDR: [u8; 6] = [0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C];
const HUE_BAR_2_ADDR: [u8; 6] = [0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00];

struct HueBar {
    device: Option<Device>,
    addr: Address,
}

impl HueBar {
    fn new(addr: Address) -> Self {
        Self { device: None, addr }
    }

    fn set_device(&mut self, device: Device) {
        self.device = Some(device);
    }

    fn unset_device(&mut self) {
        self.device = None;
    }
}

impl Deref for HueBar {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        self.device.as_ref().unwrap()
    }
}

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    if !adapter.is_powered().await? {
        adapter.set_powered(true).await?;
    }

    let mut discovery = adapter.discover_devices().await?;
    let mut pinned_disco = unsafe { Pin::new_unchecked(&mut discovery) };

    let mut bar_one = HueBar::new(Address::new(HUE_BAR_1_ADDR));
    let mut bar_two = HueBar::new(Address::new(HUE_BAR_2_ADDR));

    while let Some(event) = pinned_disco.next().await {
        match event {
            AdapterEvent::DeviceAdded(addr) => {
                if addr != bar_one.addr && addr != bar_two.addr {
                    continue;
                }

                if addr == bar_one.addr {
                    bar_one.set_device(adapter.device(addr)?);
                    println!("{:?}", bar_one.device.as_ref().unwrap().name().await?);
                } else if addr == bar_two.addr {
                    bar_two.set_device(adapter.device(addr)?);
                    println!("{:?}", bar_two.device.as_ref().unwrap().name().await?);
                }

                if bar_one.device.is_some() && bar_two.device.is_some() {
                    break;
                }
            }
            AdapterEvent::DeviceRemoved(addr) => {
                if addr == bar_one.addr {
                    bar_one.unset_device();
                } else if addr == bar_two.addr {
                    bar_two.unset_device();
                }
            }
            _ => (),
        }
    }

    adapter.set_powered(false).await?;

    Ok(())
}
