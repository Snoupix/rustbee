use std::ops::Deref;
use std::time::Duration;

use btleplug::api::WriteType;
use log::*;
use tokio::time::sleep;
use uuid::Uuid;

use crate::constants::*;
use crate::device::*;
use crate::BluetoothPeripheralImpl as _;
use crate::InnerDevice;

const ATTEMPTS: u8 = 3;

impl HueDevice<Server>
where
    HueDevice<Server>: Default + Deref<Target = InnerDevice> + std::fmt::Debug,
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
                error!(
                    "Failed to connect to {:?} after {ATTEMPTS} attempts",
                    self.addr
                );
                return Err(btleplug::Error::Other(Box::new(Error(format!(
                    "Failed to connect after {ATTEMPTS} attempts"
                )))));
            }

            if let Err(error) = self.connect().await {
                warn!("Connecting to device {:?} failed: {error}", self.addr);
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
                error!(
                    "Failed to disconnect from {:?} after {ATTEMPTS} attempts",
                    self.addr
                );
                return Err(btleplug::Error::Other(Box::new(Error(format!(
                    "Failed to disconnect after {ATTEMPTS} attempts"
                )))));
            }

            if let Err(error) = self.disconnect().await {
                warn!("Disconnecting from device {:?} failed: {error}", self.addr);
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
                format!("[ERROR] Service or Characteristic \"{POWER_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {:?}", self.addr)
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
                format!("[ERROR] Service or Characteristic \"{BRIGHTNESS_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {:?}", self.addr)
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
                format!("[ERROR] Service or Characteristic \"{COLOR_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {:?}", self.addr)
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
