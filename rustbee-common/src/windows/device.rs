use std::ops::Deref;

use log::*;
use uuid::Uuid;

use crate::constants::*;
use crate::device::*;
use crate::InnerDevice;

impl HueDevice<Server>
where
    HueDevice<Server>: Default + Deref<Target = InnerDevice> + std::fmt::Debug,
{
    pub async fn read_gatt_char(
        &self,
        service: &Uuid,
        charac: &Uuid,
    ) -> bluest::Result<Option<Vec<u8>>> {
        let services = self.services().await.map_err(|err| {
            error!("Failed to get services {err}");
            bluest::error::ErrorKind::NotFound
        })?;

        if let Some(service) = services.iter().find(|&s| &s.uuid() == service) {
            let characteristics = service.characteristics().await.map_err(|err| {
                error!("Failed to get characteristics {err} for service {service:?}");
                bluest::error::ErrorKind::NotFound
            })?;

            if let Some(charac) = characteristics.iter().find(|&c| &c.uuid() == charac) {
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
    ) -> bluest::Result<bool> {
        let services = self.services().await.map_err(|err| {
            error!("Failed to get services {err}");
            bluest::error::ErrorKind::NotFound
        })?;

        if let Some(service) = services.iter().find(|&s| &s.uuid() == service) {
            let characteristics = service.characteristics().await.map_err(|err| {
                error!("Failed to get characteristics {err} for service {service:?}");
                bluest::error::ErrorKind::NotFound
            })?;

            if let Some(charac) = characteristics.iter().find(|&c| &c.uuid() == charac) {
                charac.write(bytes).await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// This is no-op, Windows connects automatically when needed
    /// https://docs.rs/bluest/latest/bluest/struct.Adapter.html#method.connect_device
    pub async fn try_connect(&self) -> bluest::Result<()> {
        Ok(())
    }

    /// This is no-op, Windows disconnects automatically
    /// https://docs.rs/bluest/latest/bluest/struct.Adapter.html#method.disconnect_device
    pub async fn try_disconnect(&self) -> bluest::Result<()> {
        Ok(())
    }

    pub async fn is_device_connected(&self) -> bluest::Result<bool> {
        Ok((*self).is_connected().await)
    }

    pub async fn get_power(&self) -> bluest::Result<bool> {
        let read = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &POWER_UUID)
            .await?;
        if let Some(bytes) = read {
            Ok(*bytes.first().unwrap() == true as u8)
        } else {
            error!("Service or Characteristic \"{POWER_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {:?}", self.addr);
            Err(bluest::error::ErrorKind::Other.into())
        }
    }

    pub async fn set_power(&self, value: u8) -> bluest::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &POWER_UUID, &[value])
            .await?;

        Ok(())
    }

    pub async fn get_brightness(&self) -> bluest::Result<f32> {
        let read = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &BRIGHTNESS_UUID)
            .await?;
        if let Some(bytes) = read {
            Ok(*bytes.first().unwrap() as f32)
        } else {
            error!("Service or Characteristic \"{BRIGHTNESS_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {:?}", self.addr);
            Err(bluest::error::ErrorKind::Other.into())
        }
    }

    pub async fn set_brightness(&self, value: u8) -> bluest::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &BRIGHTNESS_UUID, &[value])
            .await?;

        Ok(())
    }

    pub async fn get_color(&self) -> bluest::Result<[u8; 4]> {
        let mut buf = [0u8; 4];
        if let Some(bytes) = self
            .read_gatt_char(&LIGHT_SERVICES_UUID, &COLOR_UUID)
            .await?
        {
            let len = buf.len();
            buf.copy_from_slice(&bytes[..len]);

            Ok(buf)
        } else {
            error!("[ERROR] Service or Characteristic \"{COLOR_UUID}\" for \"{LIGHT_SERVICES_UUID}\" not found for device {:?}", self.addr);
            Err(bluest::error::ErrorKind::Other.into())
        }
    }

    pub async fn set_color(&self, buf: [u8; 4]) -> bluest::Result<()> {
        self.write_gatt_char(&LIGHT_SERVICES_UUID, &COLOR_UUID, &buf)
            .await?;

        Ok(())
    }

    pub async fn get_name(&self) -> bluest::Result<Option<String>> {
        self.name_async().await.map(Some)
    }
}
